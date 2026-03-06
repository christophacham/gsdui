use std::collections::HashMap;
use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;

use crate::db;
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
    memory_usage_bytes: u64,
    per_project_watcher_status: HashMap<String, ProjectHealthStatus>,
    parse_error_counts: HashMap<String, ParseErrorCounts>,
}

#[derive(Debug, Serialize)]
struct ProjectHealthStatus {
    active: bool,
    last_seen_at: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct ParseErrorCounts {
    active: i64,
    resolved: i64,
}

async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();

    // Query SQLite database size (page_count * page_size)
    let db_size: i64 = sqlx::query_scalar(
        "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let ws_client_count = state.broadcaster.client_count();

    // Memory usage from /proc/self/status on Linux
    let memory_usage_bytes = get_memory_usage();

    // Per-project status and parse error counts
    let mut per_project_watcher_status = HashMap::new();
    let mut parse_error_counts = HashMap::new();

    if let Ok(projects) = db::schema::get_all_projects(&state.db).await {
        for project in &projects {
            per_project_watcher_status.insert(
                project.id.clone(),
                ProjectHealthStatus {
                    active: project.status == "active",
                    last_seen_at: project.last_seen_at.clone(),
                    status: project.status.clone(),
                },
            );

            if let Ok((active, resolved)) =
                db::schema::get_parse_error_counts(&state.db, &project.id).await
            {
                parse_error_counts
                    .insert(project.id.clone(), ParseErrorCounts { active, resolved });
            }
        }
    }

    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_secs: uptime,
        db_size_bytes: db_size,
        ws_client_count,
        version: env!("CARGO_PKG_VERSION").to_string(),
        memory_usage_bytes,
        per_project_watcher_status,
        parse_error_counts,
    })
}

/// Read memory usage from /proc/self/status (Linux) or return 0.
fn get_memory_usage() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2
                        && let Ok(kb) = parts[1].parse::<u64>()
                    {
                        return kb * 1024;
                    }
                }
            }
        }
        0
    }
    #[cfg(not(target_os = "linux"))]
    {
        0
    }
}
