use crate::system::SystemProvider;
use crate::db::ServiceRepository;



pub fn sync<P: SystemProvider>(
    provider: &P,
    repository: &ServiceRepository
) {

    let mut discovered = Vec::new();

    let services = provider.list_services();

    for service in services {

        discovered.push(service.unit_name.clone());

        let exists = repository
            .find_by_unit_name(
                &service.unit_name
            )
            .unwrap();


        match exists {

            None => {

                println!(
                    "New service found: {}",
                    service.unit_name
                );


                repository
                    .insert(
                        &service.unit_name,
                        "DISCOVERED"
                    )
                    .unwrap();

            },


            Some(_) => {

                repository
                    .update_last_seen(
                        &service.unit_name
                    )
                    .unwrap();

            }

        }

    }

}