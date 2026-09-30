use crate::{
    db::ServiceRepository,
    system::{OperationalState, StartupMode, SystemProvider, SystemProviderError},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ControlError {
    #[error("service is not present in the managed inventory: {0}")]
    ServiceNotFound(String),
    #[error(transparent)]
    Provider(#[from] SystemProviderError),
    #[error("database operation failed: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("could not persist confirmed service state: {source}; compensation: {compensation}")]
    Persistence {
        source: rusqlite::Error,
        compensation: String,
    },
    #[error(
        "requested system state was not verified; observed {observed:?}; compensation: {compensation}"
    )]
    Verification {
        observed: OperationalState,
        compensation: String,
    },
    #[error(
        "could not determine system state after control operation: {cause}; compensation: {compensation}"
    )]
    Indeterminate { cause: String, compensation: String },
}

pub fn set_active<P: SystemProvider + ?Sized>(
    provider: &P,
    repository: &ServiceRepository,
    unit_name: &str,
    active: bool,
) -> Result<OperationalState, ControlError> {
    if !repository
        .find_by_unit_name(unit_name)?
        .is_some_and(|service| service.present)
    {
        return Err(ControlError::ServiceNotFound(unit_name.to_owned()));
    }
    let mut desired = provider.operational_state(unit_name)?;
    if desired.active == active {
        repository
            .save_operational_state(unit_name, desired)
            .map_err(|source| ControlError::Persistence {
                source,
                compensation: "not required; OS already matched request".to_owned(),
            })?;
        return Ok(desired);
    }
    let previous = desired;
    desired.active = active;
    execute_and_commit(
        provider,
        repository,
        unit_name,
        previous,
        desired,
        |provider| provider.set_active(unit_name, active),
    )
}

pub fn set_startup_mode<P: SystemProvider + ?Sized>(
    provider: &P,
    repository: &ServiceRepository,
    unit_name: &str,
    mode: StartupMode,
) -> Result<OperationalState, ControlError> {
    if !repository
        .find_by_unit_name(unit_name)?
        .is_some_and(|service| service.present)
    {
        return Err(ControlError::ServiceNotFound(unit_name.to_owned()));
    }
    let mut desired = provider.operational_state(unit_name)?;
    if desired.startup_mode == mode {
        repository
            .save_operational_state(unit_name, desired)
            .map_err(|source| ControlError::Persistence {
                source,
                compensation: "not required; OS already matched request".to_owned(),
            })?;
        return Ok(desired);
    }
    let previous = desired;
    desired.startup_mode = mode;
    execute_and_commit(
        provider,
        repository,
        unit_name,
        previous,
        desired,
        |provider| provider.set_startup_mode(unit_name, mode),
    )
}

fn execute_and_commit<P, F>(
    provider: &P,
    repository: &ServiceRepository,
    unit_name: &str,
    previous: OperationalState,
    desired: OperationalState,
    operation: F,
) -> Result<OperationalState, ControlError>
where
    P: SystemProvider + ?Sized,
    F: FnOnce(&P) -> Result<(), SystemProviderError>,
{
    if let Err(error) = operation(provider) {
        let compensation = compensate(provider, unit_name, previous);
        return Err(ControlError::Provider(if compensation.is_empty() {
            error
        } else {
            SystemProviderError::Control(format!("{error}; compensation: {compensation}"))
        }));
    }

    let observed = match provider.operational_state(unit_name) {
        Ok(observed) => observed,
        Err(error) => {
            let compensation = compensate(provider, unit_name, previous);
            return Err(ControlError::Indeterminate {
                cause: error.to_string(),
                compensation: format!("state read failed ({error}); {compensation}"),
            });
        }
    };
    if observed != desired {
        let compensation = compensate(provider, unit_name, previous);
        return Err(ControlError::Verification {
            observed,
            compensation,
        });
    }

    if let Err(source) = repository.save_operational_state(unit_name, observed) {
        let compensation = compensate(provider, unit_name, previous);
        return Err(ControlError::Persistence {
            source,
            compensation,
        });
    }
    Ok(observed)
}

fn compensate<P: SystemProvider + ?Sized>(
    provider: &P,
    unit_name: &str,
    previous: OperationalState,
) -> String {
    let restore_active = provider.set_active(unit_name, previous.active);
    let restore_startup = provider.set_startup_mode(unit_name, previous.startup_mode);
    if let Err(error) = restore_active {
        return format!("could not restore active state: {error}");
    }
    if let Err(error) = restore_startup {
        return format!("could not restore startup mode: {error}");
    }
    match provider.operational_state(unit_name) {
        Ok(actual) if actual == previous => "previous OS state restored and verified".to_owned(),
        Ok(actual) => format!("rollback verification mismatch: {actual:?}"),
        Err(error) => format!("rollback verification failed: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::{ServicePage, SystemService};
    use rusqlite::Connection;
    use std::sync::Mutex;

    struct FakeProvider {
        state: Mutex<OperationalState>,
        fail_set: bool,
    }

    impl SystemProvider for FakeProvider {
        fn list_services_page(
            &self,
            _: Option<&str>,
            _: usize,
        ) -> Result<ServicePage, SystemProviderError> {
            Ok(ServicePage {
                services: vec![SystemService {
                    unit_name: "demo.service".into(),
                }],
                next_cursor: None,
                complete: true,
            })
        }
        fn operational_state(&self, _: &str) -> Result<OperationalState, SystemProviderError> {
            Ok(*self.state.lock().unwrap())
        }
        fn set_active(&self, _: &str, active: bool) -> Result<(), SystemProviderError> {
            if self.fail_set {
                return Err(SystemProviderError::Control("injected OS failure".into()));
            }
            self.state.lock().unwrap().active = active;
            Ok(())
        }
        fn set_startup_mode(&self, _: &str, mode: StartupMode) -> Result<(), SystemProviderError> {
            if self.fail_set {
                return Err(SystemProviderError::Control("injected OS failure".into()));
            }
            self.state.lock().unwrap().startup_mode = mode;
            Ok(())
        }
    }

    fn setup() -> (Connection, OperationalState) {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&conn);
        ServiceRepository::new(&conn)
            .insert("demo.service", "DISCOVERED")
            .unwrap();
        let previous = OperationalState {
            active: false,
            startup_mode: StartupMode::Disabled,
        };
        (conn, previous)
    }

    #[test]
    fn control_applies_os_then_persists_verified_state() {
        let (conn, previous) = setup();
        let provider = FakeProvider {
            state: Mutex::new(previous),
            fail_set: false,
        };
        let result = set_active(
            &provider,
            &ServiceRepository::new(&conn),
            "demo.service",
            true,
        )
        .unwrap();
        assert!(result.active);
        let observed: i64 = conn
            .query_row("SELECT active FROM service_operational_state", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(observed, 1);
    }

    #[test]
    fn failed_os_operation_does_not_persist_state() {
        let (conn, previous) = setup();
        let provider = FakeProvider {
            state: Mutex::new(previous),
            fail_set: true,
        };
        assert!(
            set_active(
                &provider,
                &ServiceRepository::new(&conn),
                "demo.service",
                true
            )
            .is_err()
        );
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM service_operational_state", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn database_failure_compensates_and_verifies_os_state() {
        let (conn, previous) = setup();
        conn.execute_batch("CREATE TRIGGER fail_operational_write BEFORE INSERT ON service_operational_state BEGIN SELECT RAISE(ABORT, 'injected DB failure'); END;").unwrap();
        let provider = FakeProvider {
            state: Mutex::new(previous),
            fail_set: false,
        };
        assert!(
            matches!(set_active(&provider, &ServiceRepository::new(&conn), "demo.service", true), Err(ControlError::Persistence { compensation, .. }) if compensation.contains("restored and verified"))
        );
        assert_eq!(*provider.state.lock().unwrap(), previous);
    }
}
