use std::sync::Arc;

use axum::{
    Router,
    routing::get,
};

use crate::state::AppState;

/// Build the projects sub-router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_projects).post(create_project))
        .route("/{id}", get(get_project).put(update_project).delete(delete_project))
}

async fn list_projects() -> &'static str {
    "[]"
}

async fn create_project() -> &'static str {
    "{}"
}

async fn get_project() -> &'static str {
    "{}"
}

async fn update_project() -> &'static str {
    "{}"
}

async fn delete_project() -> &'static str {
    ""
}
