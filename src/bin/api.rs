use std::{env, error::Error, net::SocketAddr, path::Path};

use services_manager::{api, db::create_connection, system::SystemdProvider, token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    set_project_root_from_executable();
    let bind: SocketAddr = env::var("SERVICES_MANAGER_API_BIND")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_owned())
        .parse()?;
    let options = token::parse_args(&env::args().skip(1).collect::<Vec<_>>())?;
    let configured_token = token::configure_token(
        options,
        env::var("SERVICES_MANAGER_API_TOKEN").ok().as_deref(),
        Path::new(".env"),
    )?;
    if !configured_token.silent {
        println!("Services Manager API token: {}", configured_token.value);
    }
    let app = api::router(
        api::ApiState::new(create_connection(), SystemdProvider::new())
            .with_bearer_token(configured_token.value),
    );
    let listener = tokio::net::TcpListener::bind(bind).await?;
    eprintln!("Services Manager API listening on {bind}");
    axum::serve(listener, app).await?;
    Ok(())
}

fn set_project_root_from_executable() {
    let Ok(executable) = env::current_exe() else {
        return;
    };
    let Some(executable_dir) = executable.parent() else {
        return;
    };
    if let Some(root) = executable_dir
        .ancestors()
        .find(|candidate| candidate.join("migrations").is_dir())
    {
        let _ = env::set_current_dir(root);
    }
}
