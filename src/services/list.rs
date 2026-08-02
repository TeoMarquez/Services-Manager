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
    Visible,
    Hidden,
    Tagged(i64)
}

#[derive(Debug)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total_items: u32,
    pub total_pages: u32,
}

pub struct QueryFilter {
    pub condition: String,
    pub params: Vec<i64>,
}

fn build_filter(filter: ServiceFilter) -> QueryFilter {

    match filter {

        ServiceFilter::All => QueryFilter {
            condition: "1=1".into(),
            params: vec![],
        },

        ServiceFilter::Present => QueryFilter {
            condition: "present = 1".into(),
            params: vec![],
        },

        ServiceFilter::Missing => QueryFilter {
            condition: "present = 0".into(),
            params: vec![],
        },

        ServiceFilter::System => QueryFilter {
            condition: "system_service = 1".into(),
            params: vec![],
        },

        ServiceFilter::User => QueryFilter {
            condition: "system_service = 0".into(),
            params: vec![],
        },

        ServiceFilter::Visible => QueryFilter {
            condition: "visible = 1".into(),
            params: vec![],
        },

        ServiceFilter::Hidden => QueryFilter {
            condition: "visible = 0".into(),
            params: vec![],
        },

       ServiceFilter::Tagged(tag_id) => QueryFilter {
            condition: format!(
                "
                id IN (
                    SELECT service_id
                    FROM service_tags
                    WHERE tag_id = {}
                )
                ",
                tag_id
            ),
            params: vec![],
        },
    }

}

pub fn list_page(
    repository: &ServiceRepository,
    filter: ServiceFilter,
    page: u32,
    per_page: u32,
) -> Page<Service> {


    let query_filter = build_filter(filter);


    let (items, total_items) =
        repository
            .find_page(
                &query_filter.condition,
                &query_filter.params,
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