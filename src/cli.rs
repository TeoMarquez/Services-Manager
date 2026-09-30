use std::io::{self, Write};

use crate::{
    db::{ServiceRepository, TagRepository},
    services::{
        delete::remove_service,
        list::{ServiceQuery, list_page},
        sync::sync,
        visibility::{hide_service, show_service},
    },
    system::MockSystem,
};

fn input() -> String {
    let mut value = String::new();

    print!("> ");
    io::stdout().flush().unwrap();

    io::stdin().read_line(&mut value).unwrap();

    value.trim().to_string()
}

pub fn run(repo: &ServiceRepository, tags: &TagRepository, system: &MockSystem) {
    loop {
        println!("==== Service Manager ====");
        println!("1) Sync");
        println!("2) List");
        println!("3) Hide");
        println!("4) Show");
        println!("5) Delete");
        println!("6) List tags");
        println!("7) Create tag");
        println!("8) Delete tag");
        println!("9) Add tag to service");
        println!("10) Remove tag from service");
        println!("11) Show service tags");
        println!("12) Search services");
        println!("0) Exit");

        match input().as_str() {
            "1" => match sync(system, repo) {
                Ok(report) => println!(
                    "Discovery complete: {} found, {} added, {} refreshed, {} marked missing",
                    report.discovered, report.added, report.refreshed, report.marked_missing
                ),
                Err(error) => println!("Discovery failed: {error}"),
            },

            "2" => {
                let services = repo.find_all().unwrap();

                for service in services {
                    let tags = repo.find_tags(service.id).unwrap();

                    let tag_list = if tags.is_empty() {
                        "no tags".to_string()
                    } else {
                        tags.iter()
                            .map(|tag| tag.name.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };

                    println!(
                        "{} | visible:{} | present:{} | tags:[{}]",
                        service.unit_name, service.visible, service.present, tag_list
                    );
                }
            }

            "3" => {
                println!("service:");
                let name = input();

                match hide_service(repo, &name) {
                    Ok(_) => println!("hidden"),

                    Err(e) => println!("error: {}", e),
                }
            }

            "4" => {
                println!("service:");
                let name = input();

                match show_service(repo, &name) {
                    Ok(_) => println!("visible"),

                    Err(e) => println!("error: {}", e),
                }
            }

            "5" => {
                println!("service:");
                let name = input();

                match remove_service(repo, &name) {
                    Ok(_) => println!("deleted"),

                    Err(e) => println!("error: {}", e),
                }
            }
            "6" => {
                let all = tags.find_all().unwrap();

                for tag in all {
                    println!("{} ({})", tag.name, tag.id);
                }
            }

            "7" => {
                println!("tag name:");

                let name = input();

                match tags.create(&name) {
                    Ok(tag) => println!("created {} ({})", tag.name, tag.id),

                    Err(e) => println!("error: {}", e),
                }
            }
            "8" => {
                println!("tag name:");

                let name = input();

                match tags.delete(&name) {
                    Ok(_) => println!("deleted"),

                    Err(e) => println!("error: {}", e),
                }
            }
            "9" => {
                println!("service:");

                let service_name = input();

                println!("tag:");

                let tag_name = input();

                let service = repo.find_by_unit_name(&service_name).unwrap();

                let tag = tags.find_by_name(&tag_name).unwrap();

                if let (Some(service), Some(tag)) = (service, tag) {
                    repo.add_tag(service.id, tag.id).unwrap();

                    println!("tag added");
                } else {
                    println!("service or tag not found");
                }
            }
            "10" => {
                println!("service:");

                let service_name = input();

                println!("tag:");

                let tag_name = input();

                let service = repo.find_by_unit_name(&service_name).unwrap();

                let tag = tags.find_by_name(&tag_name).unwrap();

                if let (Some(service), Some(tag)) = (service, tag) {
                    repo.remove_tag(service.id, tag.id).unwrap();

                    println!("tag removed");
                }
            }
            "11" => {
                println!("service:");

                let name = input();

                let service = repo.find_by_unit_name(&name).unwrap();

                if let Some(service) = service {
                    let tags = repo.find_tags(service.id).unwrap();

                    for tag in tags {
                        println!("{} ({})", tag.name, tag.id);
                    }
                }
            }

            "12" => {
                println!("search text:");
                let query = input();

                match list_page(
                    repo,
                    ServiceQuery {
                        search: Some(query),
                        ..ServiceQuery::default()
                    },
                    1,
                    20,
                ) {
                    Ok(page) => {
                        println!("{} result(s)", page.total_items);
                        for service in page.items {
                            println!("{}", service.unit_name);
                        }
                    }
                    Err(error) => println!("error: {error}"),
                }
            }

            "0" => break,

            _ => println!("unknown command"),
        }
    }
}
