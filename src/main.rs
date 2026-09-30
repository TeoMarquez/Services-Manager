use services_manager::{
    cli,
    db::{ServiceRepository, TagRepository, create_connection},
    services::sync::sync,
    system::MockSystem,
};

fn main() {
    tracing_subscriber::fmt::init();

    let db = create_connection();

    let service_repo = ServiceRepository::new(&db);

    let tag_repo = TagRepository::new(&db);

    let system = MockSystem::new();

    if let Err(error) = sync(&system, &service_repo) {
        eprintln!("Discovery failed: {error}");
    }

    cli::run(&service_repo, &tag_repo, &system);
}
