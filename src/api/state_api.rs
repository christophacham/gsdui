use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::Serialize;

use crate::db;
use crate::state::AppState;
use crate::ws;

/// Build the state API sub-router.
/// Mounted at /api/v1/projects/:id/...
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/state", get(get_project_state))
        .route("/phases", get(get_project_phases))
        .route("/phases/{phase}/plans", get(get_phase_plans))
        .route("/errors", get(get_project_errors))
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

/// GET /api/v1/projects/:id/state -- Full project state
async fn get_project_state(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let project_state = ws::build_project_state(&state, &id)
        .await
        .map_err(|e| {
            if e.contains("not found") {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse { error: e }),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse { error: e }),
                )
            }
        })?;

    Ok(Json(project_state))
}

/// GET /api/v1/projects/:id/phases -- Phase list for project
async fn get_project_phases(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify project exists
    let project = db::schema::get_project(&state.db, &id)
        .await
        .map_err(|e| {
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

    let phases = db::schema::get_phase_states_for_project(&state.db, &id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    Ok(Json(phases))
}

/// GET /api/v1/projects/:id/phases/:phase/plans -- Plan list for phase
async fn get_phase_plans(
    State(state): State<Arc<AppState>>,
    AxumPath((id, phase)): AxumPath<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify project exists
    let project = db::schema::get_project(&state.db, &id)
        .await
        .map_err(|e| {
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

    let plans = db::schema::get_plan_states_for_phase(&state.db, &id, &phase)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    Ok(Json(plans))
}

/// GET /api/v1/projects/:id/errors -- Active parse errors
async fn get_project_errors(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify project exists
    let project = db::schema::get_project(&state.db, &id)
        .await
        .map_err(|e| {
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

    let errors = db::schema::get_active_errors_for_project(&state.db, &id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    Ok(Json(errors))
}
