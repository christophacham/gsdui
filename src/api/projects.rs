use std::path::Path;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db;
use crate::state::AppState;

/// Build the projects sub-router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_projects).post(create_project))
        .route(
            "/{id}",
            get(get_project).put(update_project).delete(delete_project),
        )
}

#[derive(Debug, Deserialize)]
struct CreateProjectRequest {
    name: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct UpdateProjectRequest {
    name: Option<String>,
    retention_days: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

async fn list_projects(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let projects = db::schema::get_all_projects(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;
    Ok(Json(projects))
}

async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateProjectRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Validate path exists on disk
    let project_path = Path::new(&payload.path);
    if !project_path.exists() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Path does not exist: {}", payload.path),
            }),
        ));
    }

    // Validate .planning/ directory exists
    let planning_dir = project_path.join(".planning");
    if !planning_dir.exists() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!(
                    "Path does not contain a .planning/ directory: {}",
                    payload.path
                ),
            }),
        ));
    }

    // Generate UUID
    let id = Uuid::new_v4().to_string();

    // Try to create -- will fail on duplicate path due to UNIQUE constraint
    let project = db::schema::create_project(&state.db, &id, &payload.name, &payload.path)
        .await
        .map_err(|e| {
            // Check if this is a unique constraint violation (duplicate path)
            let err_str = e.to_string();
            if err_str.contains("UNIQUE constraint failed") {
                (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error: format!("Project path already registered: {}", payload.path),
                    }),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Database error: {}", e),
                    }),
                )
            }
        })?;

    // Trigger initial full parse via bootstrap_project
    let planning_dir_path = planning_dir.clone();
    let project_id = project.id.clone();
    let db = state.db.clone();
    let broadcast_tx = state.broadcast_tx.clone();
    let file_event_tx = state.file_event_tx.clone();

    tokio::spawn(async move {
        // Bootstrap: full re-parse of all .planning/ files
        if let Err(e) = crate::watcher::pipeline::bootstrap_project(
            &project_id,
            &planning_dir_path,
            &db,
            &broadcast_tx,
        )
        .await
        {
            tracing::error!(
                project_id = %project_id,
                error = %e,
                "Failed to bootstrap newly registered project"
            );
        }

        // Add a file watcher (requires creating a temporary watcher)
        // Note: In the full system, the file watcher instance is managed centrally.
        // Here we send a notification event so the watcher can be added in the main loop.
        // For now, we create a dedicated watcher for this project.
        let project_path = planning_dir_path.parent().unwrap_or(&planning_dir_path);
        let mut watcher = crate::watcher::FileWatcher::new(file_event_tx);
        if let Err(e) = watcher.watch_project(&project_id, project_path) {
            tracing::error!(
                project_id = %project_id,
                error = %e,
                "Failed to start file watcher for new project"
            );
        }
        // Keep the watcher alive by leaking it (it will be managed by the runtime)
        // In production, this would be stored in a shared registry
        std::mem::forget(watcher);
    });

    Ok((StatusCode::CREATED, Json(project)))
}

async fn get_project(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
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

    match project {
        Some(p) => Ok(Json(p)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Project not found: {}", id),
            }),
        )),
    }
}

async fn update_project(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<UpdateProjectRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let project = db::schema::update_project(
        &state.db,
        &id,
        payload.name.as_deref(),
        payload.retention_days,
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Database error: {}", e),
            }),
        )
    })?;

    match project {
        Some(p) => Ok(Json(p)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Project not found: {}", id),
            }),
        )),
    }
}

async fn delete_project(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let deleted = db::schema::delete_project(&state.db, &id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Project not found: {}", id),
            }),
        ))
    }
}
