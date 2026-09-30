use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{Path, Query, Request, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::{
    db::{ServiceRepository, TagRepository},
    services::{
        control::{self, ControlError},
        list::{ListError, ServiceQuery, list_page},
        sync::{SyncError, sync_step},
    },
    system::{OperationalState, StartupMode, SystemProvider},
};

#[derive(Clone)]
pub struct ApiState {
    connection: Arc<Mutex<Connection>>,
    provider: Arc<dyn SystemProvider + Send + Sync>,
    bearer_token: Option<Arc<str>>,
}

impl ApiState {
    pub fn new<P>(connection: Connection, provider: P) -> Self
    where
        P: SystemProvider + Send + Sync + 'static,
    {
        Self {
            connection: Arc::new(Mutex::new(connection)),
            provider: Arc::new(provider),
            bearer_token: None,
        }
    }

    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(Arc::from(token.into()));
        self
    }
}

pub fn router(state: ApiState) -> Router {
    let routes = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/discovery", post(discover))
        .route("/api/v1/services", get(list_services))
        .route(
            "/api/v1/services/{unit_name}/visibility",
            put(set_visibility),
        )
        .route("/api/v1/services/{unit_name}/start", post(start_service))
        .route("/api/v1/services/{unit_name}/stop", post(stop_service))
        .route(
            "/api/v1/services/{unit_name}/startup-mode",
            put(set_startup_mode),
        )
        .route("/api/v1/services/{unit_name}/tags", get(service_tags))
        .route(
            "/api/v1/services/{unit_name}/tags/{tag_name}",
            put(add_tag).delete(remove_tag),
        )
        .route("/api/v1/tags", get(list_tags).post(create_tag))
        .route("/api/v1/tags/{tag_name}", delete(delete_tag))
        .route("/api/v1/reset", post(reset));

    if state.bearer_token.is_some() {
        routes
            .layer(middleware::from_fn_with_state(
                state.clone(),
                require_bearer_token,
            ))
            .with_state(state)
    } else {
        routes.with_state(state)
    }
}

async fn require_bearer_token(
    State(state): State<ApiState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let supplied = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    let expected = state
        .bearer_token
        .as_deref()
        .map(|token| format!("Bearer {token}"));
    if supplied == expected.as_deref() {
        next.run(request).await
    } else {
        ApiError::new(StatusCode::UNAUTHORIZED, "missing or invalid bearer token").into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn internal(error: impl std::fmt::Display) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
    }

    fn not_found(resource: &str) -> Self {
        Self::new(StatusCode::NOT_FOUND, format!("{resource} was not found"))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

impl From<ListError> for ApiError {
    fn from(error: ListError) -> Self {
        match error {
            ListError::Database(error) => Self::internal(error),
            invalid => Self::new(StatusCode::BAD_REQUEST, invalid.to_string()),
        }
    }
}

impl From<SyncError> for ApiError {
    fn from(error: SyncError) -> Self {
        let status = match &error {
            SyncError::InvalidBatchSize => StatusCode::BAD_REQUEST,
            SyncError::InvalidPage { .. } | SyncError::Provider(_) => StatusCode::BAD_GATEWAY,
            SyncError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self::new(status, error.to_string())
    }
}

impl From<ControlError> for ApiError {
    fn from(error: ControlError) -> Self {
        let status = match error {
            ControlError::ServiceNotFound(_) => StatusCode::NOT_FOUND,
            ControlError::Persistence { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ControlError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ControlError::Provider(_)
            | ControlError::Verification { .. }
            | ControlError::Indeterminate { .. } => StatusCode::BAD_GATEWAY,
        };
        Self::new(status, error.to_string())
    }
}

fn lock_database(state: &ApiState) -> Result<std::sync::MutexGuard<'_, Connection>, ApiError> {
    state
        .connection
        .lock()
        .map_err(|_| ApiError::internal("database lock is poisoned"))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": 1 }))
}

#[derive(Debug, Deserialize)]
struct ListParams {
    search: Option<String>,
    present: Option<bool>,
    visible: Option<bool>,
    system_service: Option<bool>,
    tag_id: Option<i64>,
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn list_services(
    State(state): State<ApiState>,
    Query(params): Query<ListParams>,
) -> Result<Json<impl Serialize>, ApiError> {
    let conn = lock_database(&state)?;
    let repository = ServiceRepository::new(&conn);
    let page = list_page(
        &repository,
        ServiceQuery {
            search: params.search,
            present: params.present,
            visible: params.visible,
            system_service: params.system_service,
            tag_id: params.tag_id,
        },
        params.page.unwrap_or(1),
        params.per_page.unwrap_or(25),
    )?;
    Ok(Json(page))
}

#[derive(Debug, Deserialize)]
struct DiscoveryRequest {
    batch_size: Option<usize>,
}

async fn discover(
    State(state): State<ApiState>,
    Json(request): Json<DiscoveryRequest>,
) -> Result<Json<impl Serialize>, ApiError> {
    let connection = Arc::clone(&state.connection);
    let provider = Arc::clone(&state.provider);
    let batch_size = request.batch_size.unwrap_or(100);
    let progress = tokio::task::spawn_blocking(move || {
        let conn = connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let repository = ServiceRepository::new(&conn);
        sync_step(provider.as_ref(), &repository, batch_size)
    })
    .await
    .map_err(ApiError::internal)??;
    Ok(Json(progress))
}

#[derive(Debug, Deserialize)]
struct VisibilityRequest {
    visible: bool,
}

async fn set_visibility(
    State(state): State<ApiState>,
    Path(unit_name): Path<String>,
    Json(request): Json<VisibilityRequest>,
) -> Result<StatusCode, ApiError> {
    let conn = lock_database(&state)?;
    let repository = ServiceRepository::new(&conn);
    if repository
        .find_by_unit_name(&unit_name)
        .map_err(ApiError::internal)?
        .is_none()
    {
        return Err(ApiError::not_found("service"));
    }
    repository
        .set_visible(&unit_name, request.visible)
        .map_err(ApiError::internal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn change_active(
    state: ApiState,
    unit_name: String,
    active: bool,
) -> Result<Json<OperationalState>, ApiError> {
    let connection = Arc::clone(&state.connection);
    let provider = Arc::clone(&state.provider);
    let result = tokio::task::spawn_blocking(move || {
        let conn = connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let repository = ServiceRepository::new(&conn);
        if repository.find_by_unit_name(&unit_name)?.is_none() {
            return Err(ControlError::ServiceNotFound(unit_name));
        }
        control::set_active(provider.as_ref(), &repository, &unit_name, active)
    })
    .await
    .map_err(ApiError::internal)??;
    Ok(Json(result))
}

async fn start_service(
    State(state): State<ApiState>,
    Path(unit_name): Path<String>,
) -> Result<Json<OperationalState>, ApiError> {
    change_active(state, unit_name, true).await
}

async fn stop_service(
    State(state): State<ApiState>,
    Path(unit_name): Path<String>,
) -> Result<Json<OperationalState>, ApiError> {
    change_active(state, unit_name, false).await
}

#[derive(Debug, Deserialize)]
struct StartupModeRequest {
    mode: StartupMode,
}

async fn set_startup_mode(
    State(state): State<ApiState>,
    Path(unit_name): Path<String>,
    Json(request): Json<StartupModeRequest>,
) -> Result<Json<OperationalState>, ApiError> {
    let connection = Arc::clone(&state.connection);
    let provider = Arc::clone(&state.provider);
    let result = tokio::task::spawn_blocking(move || {
        let conn = connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let repository = ServiceRepository::new(&conn);
        if repository.find_by_unit_name(&unit_name)?.is_none() {
            return Err(ControlError::ServiceNotFound(unit_name));
        }
        control::set_startup_mode(provider.as_ref(), &repository, &unit_name, request.mode)
    })
    .await
    .map_err(ApiError::internal)??;
    Ok(Json(result))
}

async fn list_tags(State(state): State<ApiState>) -> Result<Json<impl Serialize>, ApiError> {
    let conn = lock_database(&state)?;
    let tags = TagRepository::new(&conn)
        .find_all()
        .map_err(ApiError::internal)?;
    Ok(Json(tags))
}

#[derive(Debug, Deserialize)]
struct CreateTagRequest {
    name: String,
}

async fn create_tag(
    State(state): State<ApiState>,
    Json(request): Json<CreateTagRequest>,
) -> Result<(StatusCode, Json<impl Serialize>), ApiError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "tag name must not be empty",
        ));
    }
    let conn = lock_database(&state)?;
    let tag = TagRepository::new(&conn)
        .create(name)
        .map_err(ApiError::internal)?;
    Ok((StatusCode::CREATED, Json(tag)))
}

async fn delete_tag(
    State(state): State<ApiState>,
    Path(tag_name): Path<String>,
) -> Result<StatusCode, ApiError> {
    let conn = lock_database(&state)?;
    let tags = TagRepository::new(&conn);
    if tags
        .find_by_name(&tag_name)
        .map_err(ApiError::internal)?
        .is_none()
    {
        return Err(ApiError::not_found("tag"));
    }
    tags.delete(&tag_name).map_err(ApiError::internal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn service_tags(
    State(state): State<ApiState>,
    Path(unit_name): Path<String>,
) -> Result<Json<impl Serialize>, ApiError> {
    let conn = lock_database(&state)?;
    let repository = ServiceRepository::new(&conn);
    let service = repository
        .find_by_unit_name(&unit_name)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("service"))?;
    let tags = repository
        .find_tags(service.id)
        .map_err(ApiError::internal)?;
    Ok(Json(tags))
}

async fn add_tag(
    State(state): State<ApiState>,
    Path((unit_name, tag_name)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let conn = lock_database(&state)?;
    let repository = ServiceRepository::new(&conn);
    let service = repository
        .find_by_unit_name(&unit_name)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("service"))?;
    let tag = TagRepository::new(&conn)
        .find_by_name(&tag_name)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("tag"))?;
    repository
        .add_tag(service.id, tag.id)
        .map_err(ApiError::internal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_tag(
    State(state): State<ApiState>,
    Path((unit_name, tag_name)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let conn = lock_database(&state)?;
    let repository = ServiceRepository::new(&conn);
    let service = repository
        .find_by_unit_name(&unit_name)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("service"))?;
    let tag = TagRepository::new(&conn)
        .find_by_name(&tag_name)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("tag"))?;
    repository
        .remove_tag(service.id, tag.id)
        .map_err(ApiError::internal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reset(State(state): State<ApiState>) -> Result<StatusCode, ApiError> {
    let conn = lock_database(&state)?;
    ServiceRepository::new(&conn)
        .reset_managed_data()
        .map_err(ApiError::internal)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use rusqlite::Connection;
    use std::sync::Mutex;
    use tower::ServiceExt;

    struct ControlProvider(Mutex<OperationalState>);

    impl SystemProvider for ControlProvider {
        fn list_services_page(
            &self,
            _: Option<&str>,
            _: usize,
        ) -> Result<crate::system::ServicePage, crate::system::SystemProviderError> {
            Ok(crate::system::ServicePage {
                services: vec![],
                next_cursor: None,
                complete: true,
            })
        }
        fn operational_state(
            &self,
            _: &str,
        ) -> Result<OperationalState, crate::system::SystemProviderError> {
            Ok(*self.0.lock().unwrap())
        }
        fn set_active(
            &self,
            _: &str,
            active: bool,
        ) -> Result<(), crate::system::SystemProviderError> {
            self.0.lock().unwrap().active = active;
            Ok(())
        }
        fn set_startup_mode(
            &self,
            _: &str,
            mode: StartupMode,
        ) -> Result<(), crate::system::SystemProviderError> {
            self.0.lock().unwrap().startup_mode = mode;
            Ok(())
        }
    }

    fn test_app() -> Router {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        crate::db::migrations::run(&conn);
        router(ApiState::new(conn, crate::system::MockSystem::new()))
    }

    #[tokio::test]
    async fn health_endpoint_returns_versioned_status() {
        let response = test_app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn configured_bearer_token_is_required() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&conn);
        let app = router(
            ApiState::new(conn, crate::system::MockSystem::new())
                .with_bearer_token("0123456789abcdef0123456789abcdef"),
        );
        let rejected = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let accepted = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .header(AUTHORIZATION, "Bearer 0123456789abcdef0123456789abcdef")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(accepted.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn service_endpoint_combines_search_visibility_and_tag_filters() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        crate::db::migrations::run(&conn);
        let repo = ServiceRepository::new(&conn);
        repo.insert("worker.service", "DISCOVERED").unwrap();
        repo.insert("web.service", "DISCOVERED").unwrap();
        repo.set_visible("worker.service", true).unwrap();
        let worker = repo.find_by_unit_name("worker.service").unwrap().unwrap();
        let tag = TagRepository::new(&conn).create("background").unwrap();
        repo.add_tag(worker.id, tag.id).unwrap();
        let app = router(ApiState::new(conn, crate::system::MockSystem::new()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/services?search=worker&visible=true&tag_id=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["total_items"], 1);
        assert_eq!(result["items"][0]["unit_name"], "worker.service");
    }

    #[tokio::test]
    async fn discovery_endpoint_processes_one_page_per_request() {
        let response = test_app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/discovery")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"batch_size":1}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let progress: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(progress["complete"], false);
        assert_eq!(progress["batch_discovered"], 1);
    }

    #[tokio::test]
    async fn start_endpoint_verifies_os_state_then_persists_it() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&conn);
        ServiceRepository::new(&conn)
            .insert("demo.service", "DISCOVERED")
            .unwrap();
        let initial = OperationalState {
            active: false,
            startup_mode: StartupMode::Disabled,
        };
        let state = ApiState::new(conn, ControlProvider(Mutex::new(initial)));
        let connection = Arc::clone(&state.connection);
        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/services/demo.service/start")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let connection = connection.lock().unwrap();
        let active: i64 = connection
            .query_row("SELECT active FROM service_operational_state", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(active, 1);
    }

    #[tokio::test]
    async fn reset_clears_managed_records_without_touching_provider() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        crate::db::migrations::run(&conn);
        let repo = ServiceRepository::new(&conn);
        repo.insert("kept.service", "DISCOVERED").unwrap();
        TagRepository::new(&conn).create("kept-tag").unwrap();
        let state = ApiState::new(conn, crate::system::MockSystem::new());
        let connection = Arc::clone(&state.connection);
        let app = router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/reset")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let connection = connection.lock().unwrap();
        assert!(
            ServiceRepository::new(&connection)
                .find_all()
                .unwrap()
                .is_empty()
        );
        assert!(
            TagRepository::new(&connection)
                .find_all()
                .unwrap()
                .is_empty()
        );
        let checkpoint = ServiceRepository::new(&connection)
            .begin_or_resume_discovery()
            .unwrap();
        assert_eq!(checkpoint.discovered, 0);
    }
}
