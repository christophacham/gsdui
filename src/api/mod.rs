pub mod health;
pub mod projects;

use std::sync::Arc;

use axum::Router;

use crate::state::AppState;

/// Build the API router with all /api/v1 routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/projects", projects::router())
        .nest("/health", health::router())
}
