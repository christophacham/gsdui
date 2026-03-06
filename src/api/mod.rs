pub mod files;
pub mod health;
pub mod history;
pub mod projects;
pub mod state_api;

use std::sync::Arc;

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::state::AppState;

/// Build the API router with all /api/v1 routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/projects", projects::router())
        // Per-project nested routes
        .nest("/projects/{id}", project_sub_router())
        .nest("/health", health::router())
        .layer(CorsLayer::permissive())
}

/// Sub-routes mounted under /api/v1/projects/:id/...
fn project_sub_router() -> Router<Arc<AppState>> {
    Router::new()
        .merge(state_api::router())
        .nest("/history", history::router())
        .nest("/files", files::router())
}
