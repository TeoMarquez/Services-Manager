use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupMode {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OperationalState {
    pub active: bool,
    pub startup_mode: StartupMode,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct SystemService {
    pub unit_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicePage {
    pub services: Vec<SystemService>,
    pub next_cursor: Option<String>,
    pub complete: bool,
}

pub trait SystemProvider {
    fn list_services_page(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<ServicePage, SystemProviderError>;

    fn operational_state(&self, _unit_name: &str) -> Result<OperationalState, SystemProviderError> {
        Err(SystemProviderError::UnsupportedOperation)
    }

    fn is_active(&self, unit_name: &str) -> Result<bool, SystemProviderError> {
        self.operational_state(unit_name).map(|state| state.active)
    }

    fn set_active(&self, _unit_name: &str, _active: bool) -> Result<(), SystemProviderError> {
        Err(SystemProviderError::UnsupportedOperation)
    }

    fn set_startup_mode(
        &self,
        _unit_name: &str,
        _mode: StartupMode,
    ) -> Result<(), SystemProviderError> {
        Err(SystemProviderError::UnsupportedOperation)
    }

    fn reload_units(&self) -> Result<(), SystemProviderError> {
        Err(SystemProviderError::UnsupportedOperation)
    }
}

#[derive(Debug, Error)]
pub enum SystemProviderError {
    #[error("could not read service inventory: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not parse service inventory: {0}")]
    InvalidData(#[from] serde_json::Error),
    #[error("invalid discovery cursor: {0}")]
    InvalidCursor(String),
    #[error("provider does not support service control")]
    UnsupportedOperation,
    #[error("service control failed: {0}")]
    Control(String),
}
