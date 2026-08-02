use crate::db::ServiceRepository;
use rusqlite::Result;


pub fn remove_service(
    repository: &ServiceRepository,
    unit_name: &str
) -> Result<()> {

    repository.delete(
        unit_name
    )

}