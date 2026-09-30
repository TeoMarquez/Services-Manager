use std::{env, error::Error, net::SocketAddr};

use services_manager::{api, db::create_connection, system::SystemdProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    let bind: SocketAddr = env::var("SERVICES_MANAGER_API_BIND")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_owned())
        .parse()?;
    let token = env::var("SERVICES_MANAGER_API_TOKEN")
        .map_err(|_| "SERVICES_MANAGER_API_TOKEN must be set")?;
    if token.len() < 32 {
        return Err("SERVICES_MANAGER_API_TOKEN must contain at least 32 characters".into());
    }
    let app = api::router(
        api::ApiState::new(create_connection(), SystemdProvider::new()).with_bearer_token(token),
    );
    let listener = tokio::net::TcpListener::bind(bind).await?;
    eprintln!("Services Manager API listening on {bind}");
    axum::serve(listener, app).await?;
    Ok(())
}
