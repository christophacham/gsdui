use std::sync::Arc;

use axum::{Router, routing::get};

use crate::state::AppState;

/// Build the health sub-router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(health_check))
}

async fn health_check() -> &'static str {
    r#"{"status":"ok"}"#
}
