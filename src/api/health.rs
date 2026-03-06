use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;

use crate::state::AppState;

/// Build the health sub-router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(health_check))
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    uptime_secs: u64,
    db_size_bytes: i64,
    ws_client_count: u32,
    version: String,
}

async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();

    // Query SQLite database size (page_count * page_size)
    let db_size: i64 = sqlx::query_scalar(
        "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_secs: uptime,
        db_size_bytes: db_size,
        ws_client_count: 0, // Placeholder until WebSocket is implemented
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
