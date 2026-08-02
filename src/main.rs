mod db;
mod system;
mod services;


use db::{
    create_connection,
    ServiceRepository,
    TagRepository
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
    let system =
        MockSystem::new();


    sync(
        &system,
        &repo
    );

    let service = repo
        .find_by_unit_name("test.service")
        .unwrap();


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

let tags = TagRepository::new(&db);


tags.create(
    "minecraft"
);

for tag in tags.find_all().unwrap() {

    println!("{:?}", tag);

}
}