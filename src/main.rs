mod db;
mod system;
mod services;
mod cli;

use db::{
    create_connection,
    ServiceRepository,
    TagRepository,
};

use system::MockSystem;
use services::sync::sync;

fn main() {

    tracing_subscriber::fmt::init();

    let db = create_connection();

    let service_repo =
        ServiceRepository::new(&db);

    let tag_repo =
        TagRepository::new(&db);

    let system =
        MockSystem::new();


    sync(
        &system,
        &service_repo
    );


    cli::run(
        &service_repo,
        &tag_repo,
        &system,
    );

}