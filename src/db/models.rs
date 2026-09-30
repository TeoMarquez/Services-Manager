#[derive(Debug, serde::Serialize)]
pub struct Service {
    pub id: i64,
    pub unit_name: String,
    pub alias: Option<String>,
    pub description: Option<String>,
    pub origin: String,
    pub visible: bool,
    pub system_service: bool,
    pub last_seen: Option<String>,
    pub present: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

#[derive(Debug)]
pub struct ServiceTag {
    pub service_id: i64,
    pub tag_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DiscoveryCheckpoint {
    pub active: bool,
    pub cursor: Option<String>,
    pub discovered: i64,
    pub added: i64,
    pub refreshed: i64,
    pub marked_missing: i64,
}
