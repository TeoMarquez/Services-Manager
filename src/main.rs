mod db;
mod system;
mod services;


use db::{
    create_connection,
    ServiceRepository
};

use system::{
    MockSystem
};

use services::sync::sync;

fn main() {

    tracing_subscriber::fmt::init();


    let db = create_connection();


    let repo = ServiceRepository::new(&db);


    let system =
        MockSystem::new();


    sync(
        &system,
        &repo
    );

    let service = repo
        .find_by_unit_name("test.service")
        .unwrap();


    println!("{:?}", service);
}