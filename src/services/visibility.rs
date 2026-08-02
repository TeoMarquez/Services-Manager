use crate::db::ServiceRepository;
use rusqlite::Result;


pub fn hide_service(
    repository: &ServiceRepository,
    unit_name: &str
) -> Result<()> {

    repository.set_visible(
        unit_name,
        false
    )

}


pub fn show_service(
    repository: &ServiceRepository,
    unit_name: &str
) -> Result<()> {

    repository.set_visible(
        unit_name,
        true
    )

}