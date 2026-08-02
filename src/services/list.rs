use crate::db::{
    models::Service,
    ServiceRepository
};


pub enum ServiceFilter {
    All,
    Present,
    Missing,
    System,
    User,
}


pub fn list_services(
    repository: &ServiceRepository,
    filter: ServiceFilter,
) -> Vec<Service> {

    match filter {

        ServiceFilter::All => {
            repository.find_all()
        },

        ServiceFilter::Present => {
            repository.find_present()
        },

        ServiceFilter::Missing => {
            repository.find_missing()
        },

        ServiceFilter::System => {
            repository.find_system()
        },

        ServiceFilter::User => {
            repository.find_user()
        },

    }
    .expect("Could not list services")

}