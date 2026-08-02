mod db;
mod system;
mod services;


use db::{
    create_connection,
    ServiceRepository,
};

use system::MockSystem;

use services::{
    sync::sync,
    list::{
        list_page,
        ServiceFilter
    },
    visibility::{
        hide_service,
        show_service
    },
    remove::remove_service,
};


fn print_services(
    title: &str,
    page: Vec<db::models::Service>
) {
    println!("\n=== {} ===", title);

    for service in page {
        println!(
            "{} | visible:{} | present:{}",
            service.unit_name,
            service.visible,
            service.present
        );
    }
}


fn main() {

    tracing_subscriber::fmt::init();


    let db = create_connection();


    let repo =
        ServiceRepository::new(&db);


    let system =
        MockSystem::new();


    //
    // SYNC INICIAL
    //
    sync(
        &system,
        &repo
    );


    //
    // LISTA INICIAL
    //
    let visible = repo
        .find_visible()
        .unwrap();

    print_services(
        "VISIBLE INICIAL",
        visible
    );


    //
    // HIDE
    //
    hide_service(
        &repo,
        "hermes.service"
    )
    .unwrap();


    let visible = repo
        .find_visible()
        .unwrap();

    let hidden = repo
        .find_hidden()
        .unwrap();


    print_services(
        "DESPUES DE HIDE - VISIBLE",
        visible
    );

    print_services(
        "DESPUES DE HIDE - HIDDEN",
        hidden
    );


    //
    // SHOW
    //
    show_service(
        &repo,
        "hermes.service"
    )
    .unwrap();


    let visible = repo
        .find_visible()
        .unwrap();


    print_services(
        "DESPUES DE SHOW",
        visible
    );


    //
    // REMOVE
    //
    remove_service(
        &repo,
        "hermes.service"
    )
    .unwrap();


    let all = repo
        .find_all()
        .unwrap();


    print_services(
        "DESPUES DE REMOVE",
        all
    );

}