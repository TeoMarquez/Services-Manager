use super::provider::{
    OperationalState, ServicePage, StartupMode, SystemProvider, SystemProviderError, SystemService,
};
use std::process::Command;

#[derive(serde::Serialize, serde::Deserialize)]
struct DiscoveryCursor {
    unit_names: Vec<String>,
    offset: usize,
}

/// Linux systemd adapter. It invokes `systemctl` directly without a shell;
/// privilege elevation is deliberately delegated to the service manager.
pub struct SystemdProvider {
    executable: String,
}

impl SystemdProvider {
    pub fn new() -> Self {
        Self {
            executable: "systemctl".to_owned(),
        }
    }

    pub fn with_executable(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    fn output(&self, args: &[&str]) -> Result<String, SystemProviderError> {
        let output = Command::new(&self.executable).args(args).output()?;
        if !output.status.success() {
            return Err(SystemProviderError::Control(format!(
                "{} exited with {}: {}",
                args.join(" "),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        String::from_utf8(output.stdout).map_err(|error| {
            SystemProviderError::Control(format!("systemctl returned invalid UTF-8: {error}"))
        })
    }

    fn run(&self, args: &[&str]) -> Result<(), SystemProviderError> {
        self.output(args).map(|_| ())
    }

    fn status_output(&self, args: &[&str]) -> Result<String, SystemProviderError> {
        let output = Command::new(&self.executable).args(args).output()?;
        let stdout = String::from_utf8(output.stdout).map_err(|error| {
            SystemProviderError::Control(format!("systemctl returned invalid UTF-8: {error}"))
        })?;
        if stdout.trim().is_empty() {
            return Err(SystemProviderError::Control(format!(
                "{} exited with {}: {}",
                args.join(" "),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(stdout)
    }

    fn validate_unit(unit_name: &str) -> Result<(), SystemProviderError> {
        if unit_name.is_empty() || unit_name.starts_with('-') || !unit_name.ends_with(".service") {
            return Err(SystemProviderError::Control(format!(
                "invalid service unit name: {unit_name}"
            )));
        }
        Ok(())
    }
}

impl Default for SystemdProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemProvider for SystemdProvider {
    fn list_services_page(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<ServicePage, SystemProviderError> {
        let snapshot = if let Some(cursor) = cursor {
            serde_json::from_str::<DiscoveryCursor>(cursor)
                .map_err(|_| SystemProviderError::InvalidCursor(cursor.to_owned()))?
        } else {
            let output = self.output(&[
                "list-unit-files",
                "--type=service",
                "--no-legend",
                "--no-pager",
            ])?;
            let mut unit_names = output
                .lines()
                .filter_map(|line| {
                    let name = line.split_whitespace().next()?;
                    name.ends_with(".service").then(|| name.to_owned())
                })
                .collect::<Vec<_>>();
            unit_names.sort();
            unit_names.dedup();
            DiscoveryCursor {
                unit_names,
                offset: 0,
            }
        };
        if snapshot.offset > snapshot.unit_names.len() {
            return Err(SystemProviderError::InvalidCursor(
                snapshot.offset.to_string(),
            ));
        }
        let end = snapshot
            .offset
            .saturating_add(limit)
            .min(snapshot.unit_names.len());
        let complete = end == snapshot.unit_names.len();
        let services = snapshot.unit_names[snapshot.offset..end]
            .iter()
            .map(|unit_name| SystemService {
                unit_name: unit_name.clone(),
            })
            .collect();
        let next_cursor = if complete {
            None
        } else {
            Some(
                serde_json::to_string(&DiscoveryCursor {
                    unit_names: snapshot.unit_names,
                    offset: end,
                })
                .expect("discovery cursor serialization cannot fail"),
            )
        };
        Ok(ServicePage {
            services,
            next_cursor,
            complete,
        })
    }

    fn operational_state(&self, unit_name: &str) -> Result<OperationalState, SystemProviderError> {
        Self::validate_unit(unit_name)?;
        let active =
            self.output(&["show", "--property=ActiveState", "--value", "--", unit_name])?;
        // systemctl returns non-zero for normal states such as `disabled`.
        let enabled = self.status_output(&["is-enabled", "--", unit_name])?;
        let active = match active.trim() {
            "active" => true,
            "inactive" => false,
            value => {
                return Err(SystemProviderError::Control(format!(
                    "unknown ActiveState '{value}' for {unit_name}"
                )));
            }
        };
        let startup_mode = match enabled.trim() {
            "enabled" => StartupMode::Enabled,
            "disabled" => StartupMode::Disabled,
            value => {
                return Err(SystemProviderError::Control(format!(
                    "unsupported UnitFileState '{value}' for {unit_name}"
                )));
            }
        };
        Ok(OperationalState {
            active,
            startup_mode,
        })
    }

    fn set_active(&self, unit_name: &str, active: bool) -> Result<(), SystemProviderError> {
        Self::validate_unit(unit_name)?;
        self.run(&[if active { "start" } else { "stop" }, "--", unit_name])
    }

    fn set_startup_mode(
        &self,
        unit_name: &str,
        mode: StartupMode,
    ) -> Result<(), SystemProviderError> {
        Self::validate_unit(unit_name)?;
        self.run(&[
            match mode {
                StartupMode::Enabled => "enable",
                StartupMode::Disabled => "disable",
            },
            "--",
            unit_name,
        ])
    }
}
