use crate::{
    db::ServiceRepository,
    services::sync::{SyncError, SyncReport, sync},
    system::{SystemProvider, SystemProviderError},
};
use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, serde::Serialize)]
pub struct CreatedService {
    pub unit_name: String,
    pub service_file: PathBuf,
    pub discovery: SyncReport,
}

#[derive(Debug, Error)]
pub enum CreateServiceError {
    #[error("invalid service name: expected a simple name without .service or path separators")]
    InvalidName,
    #[error(
        "invalid description: newlines, control characters, percent and backslash are not allowed"
    )]
    InvalidDescription,
    #[error("service file already exists: {0}")]
    AlreadyExists(PathBuf),
    #[error("service unit is already present in the managed inventory: {0}")]
    AlreadyPresent(String),
    #[error("could not create service file: {0}")]
    Io(#[from] std::io::Error),
    #[error("new service could not be reloaded by systemd ({reload}); compensation: {rollback}")]
    Reload { reload: String, rollback: String },
    #[error("service file {service_file} was created and loaded, but discovery failed: {source}")]
    Discovery {
        service_file: PathBuf,
        source: SyncError,
    },
    #[error(
        "service file {service_file} was created and systemd reloaded it, but discovery did not find {unit_name}"
    )]
    NotDiscovered {
        service_file: PathBuf,
        unit_name: String,
    },
    #[error(transparent)]
    Provider(#[from] SystemProviderError),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
}

pub fn validate_service_name(name: &str) -> Result<(), CreateServiceError> {
    if name.is_empty()
        || name.len() > 180
        || name.starts_with('.')
        || name.starts_with('-')
        || name.ends_with(".service")
        || name.contains("..")
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'@'))
    {
        return Err(CreateServiceError::InvalidName);
    }
    Ok(())
}

fn validate_description(description: &str) -> Result<(), CreateServiceError> {
    if description.trim().is_empty()
        || description
            .chars()
            .any(|character| character == '\\' || character == '%' || character.is_control())
    {
        return Err(CreateServiceError::InvalidDescription);
    }
    Ok(())
}

pub fn create_service_and_discover<P: SystemProvider + ?Sized>(
    provider: &P,
    repository: &ServiceRepository,
    service_directory: &Path,
    name: &str,
    description: &str,
) -> Result<CreatedService, CreateServiceError> {
    validate_service_name(name)?;
    validate_description(description)?;
    if !service_directory.is_absolute() {
        return Err(CreateServiceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "service directory must be absolute",
        )));
    }
    fs::create_dir_all(service_directory)?;

    let unit_name = format!("{name}.service");
    if repository
        .find_by_unit_name(&unit_name)?
        .is_some_and(|service| service.present)
    {
        return Err(CreateServiceError::AlreadyPresent(unit_name));
    }
    let service_file = service_directory.join(&unit_name);
    let content = format!(
        "[Unit]\nDescription={}\n\n# Template only: define ExecStart before starting this unit.\n[Service]\nType=oneshot\nRemainAfterExit=yes\n# ExecStart=/path/to/program\n\n[Install]\nWantedBy=multi-user.target\n",
        description.trim()
    );
    let mut file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&service_file)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(CreateServiceError::AlreadyExists(service_file));
        }
        Err(error) => return Err(CreateServiceError::Io(error)),
    };
    let write_result = file
        .write_all(content.as_bytes())
        .and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&service_file);
        return Err(CreateServiceError::Io(error));
    }

    if let Err(reload_error) = provider.reload_units() {
        let remove_result = fs::remove_file(&service_file);
        let reload_rollback = provider.reload_units();
        let rollback = match (remove_result, reload_rollback) {
            (Ok(()), Ok(())) => "service file removed and systemd reload succeeded".to_owned(),
            (remove, reload) => format!("file removal: {remove:?}; systemd reload: {reload:?}"),
        };
        return Err(CreateServiceError::Reload {
            reload: reload_error.to_string(),
            rollback,
        });
    }

    let discovery = sync(provider, repository).map_err(|source| CreateServiceError::Discovery {
        service_file: service_file.clone(),
        source,
    })?;
    if !repository
        .find_by_unit_name(&unit_name)?
        .is_some_and(|service| service.present)
    {
        return Err(CreateServiceError::NotDiscovered {
            service_file,
            unit_name,
        });
    }
    Ok(CreatedService {
        unit_name,
        service_file,
        discovery,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::{ServicePage, SystemService};
    use rusqlite::Connection;

    struct FileProvider {
        reload_ok: bool,
    }

    impl SystemProvider for FileProvider {
        fn list_services_page(
            &self,
            _: Option<&str>,
            _: usize,
        ) -> Result<ServicePage, SystemProviderError> {
            Ok(ServicePage {
                services: vec![SystemService {
                    unit_name: "demo.service".to_owned(),
                }],
                next_cursor: None,
                complete: true,
            })
        }
        fn reload_units(&self) -> Result<(), SystemProviderError> {
            if self.reload_ok {
                Ok(())
            } else {
                Err(SystemProviderError::Control("reload failed".to_owned()))
            }
        }
    }

    fn temporary_directory() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "services-manager-unit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn writes_a_scaffold_then_runs_discovery() {
        let directory = temporary_directory();
        let connection = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&connection);
        let result = create_service_and_discover(
            &FileProvider { reload_ok: true },
            &ServiceRepository::new(&connection),
            &directory,
            "demo",
            "Demo description",
        )
        .unwrap();
        assert_eq!(result.unit_name, "demo.service");
        assert_eq!(result.discovery.added, 1);
        let content = fs::read_to_string(&result.service_file).unwrap();
        assert!(content.contains("Description=Demo description"));
        assert!(content.contains("# ExecStart=/path/to/program"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn failed_reload_removes_the_created_file() {
        let directory = temporary_directory();
        let connection = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&connection);
        assert!(matches!(
            create_service_and_discover(
                &FileProvider { reload_ok: false },
                &ServiceRepository::new(&connection),
                &directory,
                "demo",
                "Demo"
            ),
            Err(CreateServiceError::Reload { .. })
        ));
        assert!(!directory.join("demo.service").exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_unit_path_injection_and_multiline_description() {
        assert!(matches!(
            validate_service_name("../bad"),
            Err(CreateServiceError::InvalidName)
        ));
        assert!(matches!(
            validate_description("bad\ndescription"),
            Err(CreateServiceError::InvalidDescription)
        ));
    }
}
