use crate::db::{ServiceRepository, models::DiscoveryCheckpoint};
use crate::system::{SystemProvider, SystemProviderError};
use thiserror::Error;

pub const DEFAULT_BATCH_SIZE: usize = 100;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub discovered: usize,
    pub added: usize,
    pub refreshed: usize,
    pub marked_missing: usize,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct DiscoveryProgress {
    pub complete: bool,
    pub batch_discovered: usize,
    pub checkpoint: DiscoveryCheckpoint,
}

#[derive(Debug, Error)]
pub enum SyncError {
    #[error(transparent)]
    Provider(#[from] SystemProviderError),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error("discovery batch size must be greater than zero")]
    InvalidBatchSize,
    #[error("provider returned an invalid page for cursor {cursor:?}")]
    InvalidPage { cursor: Option<String> },
}

/// Processes one bounded page and commits its observations and cursor together.
/// If the provider fails, the active checkpoint remains resumable.
pub fn sync_step<P: SystemProvider + ?Sized>(
    provider: &P,
    repository: &ServiceRepository,
    batch_size: usize,
) -> Result<DiscoveryProgress, SyncError> {
    if batch_size == 0 {
        return Err(SyncError::InvalidBatchSize);
    }

    let before = repository.begin_or_resume_discovery()?;
    let page = provider.list_services_page(before.cursor.as_deref(), batch_size)?;
    let cursor_advanced = page.next_cursor.as_deref() != before.cursor.as_deref();
    if page.services.len() > batch_size
        || (page.services.is_empty() && !page.complete)
        || (!page.complete && (page.next_cursor.is_none() || !cursor_advanced))
        || (page.complete && page.next_cursor.is_some())
    {
        return Err(SyncError::InvalidPage {
            cursor: before.cursor,
        });
    }

    let batch_discovered = page.services.len();
    let services = page
        .services
        .into_iter()
        .map(|service| service.unit_name)
        .collect::<Vec<_>>();
    let checkpoint = repository.save_discovery_page(
        before.cursor.as_deref(),
        &services,
        page.next_cursor.as_deref(),
        page.complete,
    )?;

    Ok(DiscoveryProgress {
        complete: !checkpoint.active,
        batch_discovered,
        checkpoint,
    })
}

/// Runs pages until completion. A provider error leaves the durable cursor in
/// place, so a later call resumes the same discovery cycle.
pub fn sync<P: SystemProvider + ?Sized>(
    provider: &P,
    repository: &ServiceRepository,
) -> Result<SyncReport, SyncError> {
    loop {
        let progress = sync_step(provider, repository, DEFAULT_BATCH_SIZE)?;
        if progress.complete {
            let checkpoint = progress.checkpoint;
            return Ok(SyncReport {
                discovered: checkpoint.discovered as usize,
                added: checkpoint.added as usize,
                refreshed: checkpoint.refreshed as usize,
                marked_missing: checkpoint.marked_missing as usize,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::{ServicePage, SystemService};
    use rusqlite::Connection;

    struct FakeProvider {
        services: Vec<SystemService>,
        fail_at: Option<usize>,
    }

    impl SystemProvider for FakeProvider {
        fn list_services_page(
            &self,
            cursor: Option<&str>,
            limit: usize,
        ) -> Result<ServicePage, SystemProviderError> {
            let start = cursor.unwrap_or("0").parse::<usize>().unwrap();
            if self.fail_at == Some(start) {
                return Err(SystemProviderError::Io(std::io::Error::other("offline")));
            }
            let end = start.saturating_add(limit).min(self.services.len());
            let complete = end == self.services.len();
            Ok(ServicePage {
                services: self.services[start..end].to_vec(),
                next_cursor: (!complete).then(|| end.to_string()),
                complete,
            })
        }
    }

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&conn);
        conn
    }

    fn service(name: &str) -> SystemService {
        SystemService {
            unit_name: name.to_owned(),
        }
    }

    fn provider(names: &[&str], fail_at: Option<usize>) -> FakeProvider {
        FakeProvider {
            services: names.iter().map(|name| service(name)).collect(),
            fail_at,
        }
    }

    #[test]
    fn discovery_is_applied_in_pages_and_missing_is_marked_only_on_final_page() {
        let conn = database();
        let repo = ServiceRepository::new(&conn);
        repo.insert("old.service", "DISCOVERED").unwrap();
        let provider = provider(&["a.service", "b.service"], None);

        let first = sync_step(&provider, &repo, 1).unwrap();
        assert!(!first.complete);
        assert_eq!(first.batch_discovered, 1);
        assert!(repo.find_by_unit_name("a.service").unwrap().is_some());
        assert!(
            repo.find_by_unit_name("old.service")
                .unwrap()
                .unwrap()
                .present
        );

        let second = sync_step(&provider, &repo, 1).unwrap();
        assert!(second.complete);
        assert_eq!(second.checkpoint.discovered, 2);
        assert_eq!(second.checkpoint.added, 2);
        assert_eq!(second.checkpoint.marked_missing, 1);
        assert!(
            !repo
                .find_by_unit_name("old.service")
                .unwrap()
                .unwrap()
                .present
        );
    }

    #[test]
    fn failed_page_resumes_from_persisted_cursor_without_premature_absence() {
        let conn = database();
        let repo = ServiceRepository::new(&conn);
        repo.insert("old.service", "DISCOVERED").unwrap();
        let first_provider = provider(&["a.service", "b.service"], None);
        let failing_provider = provider(&["a.service", "b.service"], Some(1));
        let resumed_provider = provider(&["a.service", "b.service"], None);

        let first = sync_step(&first_provider, &repo, 1).unwrap();
        assert_eq!(first.checkpoint.cursor.as_deref(), Some("1"));
        assert!(sync_step(&failing_provider, &repo, 1).is_err());
        assert!(
            repo.find_by_unit_name("old.service")
                .unwrap()
                .unwrap()
                .present
        );

        let report = sync(&resumed_provider, &repo).unwrap();
        assert_eq!(report.discovered, 2);
        assert_eq!(report.added, 2);
        assert_eq!(report.marked_missing, 1);
        assert!(
            !repo
                .find_by_unit_name("old.service")
                .unwrap()
                .unwrap()
                .present
        );
    }

    #[test]
    fn completed_discovery_is_idempotent_and_provider_failure_is_resumable() {
        let conn = database();
        let repo = ServiceRepository::new(&conn);
        let inventory = provider(&["web.service"], None);

        let first = sync(&inventory, &repo).unwrap();
        let second = sync(&inventory, &repo).unwrap();

        assert_eq!(first.added, 1);
        assert_eq!(second.added, 0);
        assert_eq!(second.refreshed, 1);
        assert_eq!(repo.find_all().unwrap().len(), 1);
    }

    #[test]
    fn provider_failure_before_first_page_does_not_change_service_presence() {
        let conn = database();
        let repo = ServiceRepository::new(&conn);
        repo.insert("existing.service", "DISCOVERED").unwrap();

        assert!(sync(&provider(&["a.service"], Some(0)), &repo).is_err());
        assert_eq!(repo.find_all().unwrap().len(), 1);
        assert!(
            repo.find_by_unit_name("existing.service")
                .unwrap()
                .unwrap()
                .present
        );
    }

    #[test]
    fn zero_batch_size_is_rejected() {
        let conn = database();
        let repo = ServiceRepository::new(&conn);
        assert!(matches!(
            sync_step(&provider(&[], None), &repo, 0),
            Err(SyncError::InvalidBatchSize)
        ));
    }

    #[test]
    fn failed_page_write_rolls_back_observations_and_cursor_together() {
        let conn = database();
        let repo = ServiceRepository::new(&conn);
        conn.execute_batch(
            "CREATE TRIGGER reject_bad_service
             BEFORE INSERT ON services
             WHEN NEW.unit_name = 'bad.service'
             BEGIN SELECT RAISE(ABORT, 'injected database failure'); END;",
        )
        .unwrap();

        let result = sync_step(&provider(&["good.service", "bad.service"], None), &repo, 2);

        assert!(matches!(result, Err(SyncError::Database(_))));
        assert!(repo.find_by_unit_name("good.service").unwrap().is_none());
        let checkpoint = repo.begin_or_resume_discovery().unwrap();
        assert!(checkpoint.active);
        assert_eq!(checkpoint.cursor, None);
        assert_eq!(checkpoint.discovered, 0);
    }
}
