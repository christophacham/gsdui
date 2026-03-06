use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::db;
use crate::db::schema::{PaginationMeta, RunFilters, SessionFilters};
use crate::state::AppState;

/// Build the history sub-router.
/// Mounted at /api/v1/projects/:id/history/...
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/runs", get(get_runs))
        .route("/runs/{run_id}/commits", get(get_run_commits))
        .route("/agents", get(get_agent_sessions))
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

/// Query parameters for execution runs listing.
#[derive(Debug, Deserialize)]
struct RunQueryParams {
    phase: Option<String>,
    plan: Option<String>,
    status: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Response for paginated execution runs.
#[derive(Debug, Serialize)]
struct RunsResponse {
    runs: Vec<db::models::ExecutionRun>,
    pagination: PaginationMeta,
}

/// GET /api/v1/projects/:id/history/runs -- Execution runs with filtering
async fn get_runs(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
    Query(params): Query<RunQueryParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify project exists
    let project = db::schema::get_project(&state.db, &id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Database error: {}", e),
            }),
        )
    })?;

    if project.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Project not found: {}", id),
            }),
        ));
    }

    let filters = RunFilters {
        phase: params.phase,
        plan: params.plan,
        status: params.status,
        from: params.from,
        to: params.to,
        limit: params.limit.unwrap_or(50).min(200),
        offset: params.offset.unwrap_or(0),
    };

    let (runs, pagination) = db::schema::get_runs_filtered(&state.db, &id, &filters)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    Ok(Json(RunsResponse { runs, pagination }))
}

/// GET /api/v1/projects/:id/history/runs/:run_id/commits -- Commits for a run
async fn get_run_commits(
    State(state): State<Arc<AppState>>,
    AxumPath((id, run_id)): AxumPath<(String, i64)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify project exists
    let project = db::schema::get_project(&state.db, &id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Database error: {}", e),
            }),
        )
    })?;

    if project.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Project not found: {}", id),
            }),
        ));
    }

    // Verify run exists and belongs to project
    let run = db::schema::get_run_by_id(&state.db, run_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    match run {
        Some(r) if r.project_id == id => {
            let commits = db::schema::get_commits_for_run(&state.db, r.id)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: format!("Database error: {}", e),
                        }),
                    )
                })?;
            Ok(Json(commits))
        }
        Some(_) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Run {} not found for project {}", run_id, id),
            }),
        )),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Run {} not found", run_id),
            }),
        )),
    }
}

/// Query parameters for agent sessions listing.
#[derive(Debug, Deserialize)]
struct SessionQueryParams {
    agent_type: Option<String>,
    phase: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

/// GET /api/v1/projects/:id/history/agents -- Agent sessions with filtering
async fn get_agent_sessions(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
    Query(params): Query<SessionQueryParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify project exists
    let project = db::schema::get_project(&state.db, &id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Database error: {}", e),
            }),
        )
    })?;

    if project.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Project not found: {}", id),
            }),
        ));
    }

    let filters = SessionFilters {
        agent_type: params.agent_type,
        phase: params.phase,
        from: params.from,
        to: params.to,
    };

    let sessions = db::schema::get_sessions_filtered(&state.db, &id, &filters)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    Ok(Json(sessions))
}
