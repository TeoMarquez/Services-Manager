#[derive(Debug)]
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

#[derive(Debug)]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

#[derive(Debug)]
pub struct ServiceTag {
    pub service_id: i64,
    pub tag_id: i64,
}