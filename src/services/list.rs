use crate::db::{ServiceRepository, models::Service};
use rusqlite::types::Value;
use thiserror::Error;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ServiceQuery {
    pub search: Option<String>,
    pub present: Option<bool>,
    pub visible: Option<bool>,
    pub system_service: Option<bool>,
    pub tag_id: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total_items: u32,
    pub total_pages: u32,
}

pub struct QueryFilter {
    pub condition: String,
    pub params: Vec<Value>,
}

#[derive(Debug, Error)]
pub enum ListError {
    #[error("page number must be greater than zero")]
    InvalidPage,
    #[error("page size must be greater than zero")]
    InvalidPageSize,
    #[error("tag id must be greater than zero")]
    InvalidTagId,
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
}

fn build_filter(query: ServiceQuery) -> Result<QueryFilter, ListError> {
    if query.tag_id.is_some_and(|tag_id| tag_id <= 0) {
        return Err(ListError::InvalidTagId);
    }

    let mut conditions = Vec::new();
    let mut params = Vec::new();
    if let Some(search) = query.search {
        let escaped = search
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = Value::Text(format!("%{escaped}%"));
        conditions.push(
            "(unit_name LIKE ? ESCAPE '\\'
             OR COALESCE(alias, '') LIKE ? ESCAPE '\\'
             OR COALESCE(description, '') LIKE ? ESCAPE '\\')",
        );
        params.extend([pattern.clone(), pattern.clone(), pattern]);
    }
    if let Some(present) = query.present {
        conditions.push("present = ?");
        params.push(Value::Integer(if present { 1 } else { 0 }));
    }
    if let Some(visible) = query.visible {
        conditions.push("visible = ?");
        params.push(Value::Integer(if visible { 1 } else { 0 }));
    }
    if let Some(system_service) = query.system_service {
        conditions.push("system_service = ?");
        params.push(Value::Integer(if system_service { 1 } else { 0 }));
    }
    if let Some(tag_id) = query.tag_id {
        conditions.push("id IN (SELECT service_id FROM service_tags WHERE tag_id = ?)");
        params.push(Value::Integer(tag_id));
    }

    Ok(QueryFilter {
        condition: if conditions.is_empty() {
            "1=1".into()
        } else {
            conditions.join(" AND ")
        },
        params,
    })
}

pub fn list_page(
    repository: &ServiceRepository,
    query: ServiceQuery,
    page: u32,
    per_page: u32,
) -> Result<Page<Service>, ListError> {
    if page == 0 {
        return Err(ListError::InvalidPage);
    }
    if per_page == 0 {
        return Err(ListError::InvalidPageSize);
    }

    let query_filter = build_filter(query)?;
    let (items, total_items) = repository.find_page(
        &query_filter.condition,
        &query_filter.params,
        page,
        per_page,
    )?;

    Ok(Page {
        items,
        page,
        per_page,
        total_items,
        total_pages: total_items.div_ceil(per_page),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{ServiceRepository, TagRepository};
    use rusqlite::Connection;

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        crate::db::migrations::run(&conn);
        conn
    }

    #[test]
    fn search_uses_literal_text_and_returns_pages() {
        let conn = database();
        let repo = ServiceRepository::new(&conn);
        repo.insert("api_100%.service", "DISCOVERED").unwrap();
        repo.insert("api_100x.service", "DISCOVERED").unwrap();

        let page = list_page(
            &repo,
            ServiceQuery {
                search: Some("100%".into()),
                ..ServiceQuery::default()
            },
            1,
            1,
        )
        .unwrap();

        assert_eq!(page.total_items, 1);
        assert_eq!(page.total_pages, 1);
        assert_eq!(page.items[0].unit_name, "api_100%.service");
    }

    #[test]
    fn tag_filter_is_parameterized_and_pagination_validates_bounds() {
        let conn = database();
        let repo = ServiceRepository::new(&conn);
        let tags = TagRepository::new(&conn);
        repo.insert("worker.service", "DISCOVERED").unwrap();
        repo.insert("web.service", "DISCOVERED").unwrap();
        let worker = repo.find_by_unit_name("worker.service").unwrap().unwrap();
        let tag = tags.create("background").unwrap();
        repo.add_tag(worker.id, tag.id).unwrap();
        repo.set_visible("worker.service", true).unwrap();

        let page = list_page(
            &repo,
            ServiceQuery {
                tag_id: Some(tag.id),
                present: Some(true),
                visible: Some(true),
                ..ServiceQuery::default()
            },
            1,
            25,
        )
        .unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].unit_name, "worker.service");
        assert!(matches!(
            list_page(&repo, ServiceQuery::default(), 0, 25),
            Err(ListError::InvalidPage)
        ));
        assert!(matches!(
            list_page(&repo, ServiceQuery::default(), 1, 0),
            Err(ListError::InvalidPageSize)
        ));
        assert!(matches!(
            list_page(
                &repo,
                ServiceQuery {
                    tag_id: Some(0),
                    ..ServiceQuery::default()
                },
                1,
                25
            ),
            Err(ListError::InvalidTagId)
        ));
    }
}
