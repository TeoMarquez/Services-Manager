mod db;
mod system;
mod services;


use db::{
    create_connection,
    ServiceRepository
};

use services::list::{
    list_page,  
    ServiceFilter
};

use system::{
    MockSystem
};

use services::sync::sync;

fn main() {

    tracing_subscriber::fmt::init();


    let db = create_connection();


    let repo = ServiceRepository::new(&db);
use services::list::list_services;

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
    let services =
    list_services(&repo, ServiceFilter::All);

let page = list_page(
    &repo,
    ServiceFilter::All,
    2,
    2
);


println!("{:?}", page);


for service in page.items {

    println!(
        "{}",
        service.unit_name
    );

}

for service in services {

    println!(
        "{} - present:{}",
        service.unit_name,
        service.present
    );

}
}