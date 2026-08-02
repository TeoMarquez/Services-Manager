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
#[derive(Debug)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total_items: u32,
    pub total_pages: u32,
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

pub fn list_page(
    repository: &ServiceRepository,
    filter: ServiceFilter,
    page: u32,
    per_page: u32,
) -> Page<Service> {


    let condition = match filter {

        ServiceFilter::All => "1=1",
        ServiceFilter::Present => "present = 1",
        ServiceFilter::Missing => "present = 0",
        ServiceFilter::System => "system_service = 1",
        ServiceFilter::User => "system_service = 0",

    };


    let (items, total_items) =
        repository
            .find_page(
                condition,
                page,
                per_page
            )
            .unwrap();


    let total_pages =
        (total_items + per_page - 1)
        / per_page;


    Page {
        items,
        page,
        per_page,
        total_items,
        total_pages,
    }

}