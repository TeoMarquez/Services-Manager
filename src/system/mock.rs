use super::provider::{ServicePage, SystemProvider, SystemProviderError, SystemService};

use std::fs;

pub struct MockSystem {}

impl MockSystem {
    pub fn new() -> Self {
        Self {}
    }
}

impl SystemProvider for MockSystem {
    fn list_services_page(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<ServicePage, SystemProviderError> {
        let data = fs::read_to_string("mock/systemd.json")?;
        let all_services: Vec<SystemService> = serde_json::from_str(&data)?;
        let start = match cursor {
            Some(value) => value
                .parse::<usize>()
                .map_err(|_| SystemProviderError::InvalidCursor(value.to_owned()))?,
            None => 0,
        };
        if start > all_services.len() {
            return Err(SystemProviderError::InvalidCursor(start.to_string()));
        }

        let end = start.saturating_add(limit).min(all_services.len());
        let complete = end == all_services.len();
        Ok(ServicePage {
            services: all_services[start..end].to_vec(),
            next_cursor: (!complete).then(|| end.to_string()),
            complete,
        })
    }
}
