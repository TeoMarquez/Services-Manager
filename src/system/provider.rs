use serde::Deserialize;


#[derive(Debug, Deserialize)]
pub struct SystemService {

    pub unit_name: String,

}


pub trait SystemProvider {

    fn list_services(&self) -> Vec<SystemService>;

}