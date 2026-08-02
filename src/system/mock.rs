use super::provider::{
    SystemProvider,
    SystemService
};

use std::fs;


pub struct MockSystem {


}


impl MockSystem {

    pub fn new() -> Self {
        Self {}
    }

}



impl SystemProvider for MockSystem {


    fn list_services(
        &self
    ) -> Vec<SystemService> {


        let data = fs::read_to_string(
            "mock/systemd.json"
        )
        .expect("Could not read mock systemd");


        serde_json::from_str(
            &data
        )
        .expect("Invalid mock json")

    }

}