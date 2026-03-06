use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    Json, Router,
    body::Body,
    extract::{Path as AxumPath, State},
    http::{StatusCode, header},
    response::Response,
    routing::get,
};
use serde::Serialize;

use crate::db;
use crate::state::AppState;

/// Build the files sub-router.
/// Mounted at /api/v1/projects/:id/files/...
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/{*path}", get(get_file_content))
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

/// GET /api/v1/projects/:id/files/*path -- Raw file content from .planning/
///
/// The path parameter is a relative path within .planning/ directory.
/// e.g., "phases/01-name/01-01-PLAN.md" or "STATE.md"
///
/// Security: validates path is within .planning/ directory to prevent path traversal.
async fn get_file_content(
    State(state): State<Arc<AppState>>,
    AxumPath((id, file_path)): AxumPath<(String, String)>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // Verify project exists and get its path
    let project = db::schema::get_project(&state.db, &id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Project not found: {}", id),
                }),
            )
        })?;

    // Decode the file path
    let decoded_path = urlencoding::decode(&file_path)
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| file_path.clone());

    // Security: check for path traversal
    if decoded_path.contains("..") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Path traversal detected: '..' not allowed".to_string(),
            }),
        ));
    }

    // Build full path: project_path/.planning/<file_path>
    let planning_dir = PathBuf::from(&project.path).join(".planning");
    let full_path = planning_dir.join(&decoded_path);

    // Security: ensure resolved path is within .planning/ directory
    let canonical_planning = match planning_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Project .planning/ directory not accessible".to_string(),
                }),
            ));
        }
    };

    let canonical_file = match full_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("File not found: {}", file_path),
                }),
            ));
        }
    };

    if !canonical_file.starts_with(&canonical_planning) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Path traversal detected: file outside .planning/ directory".to_string(),
            }),
        ));
    }

    // Read file content
    let content = tokio::fs::read_to_string(&canonical_file)
        .await
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("File not found: {}", file_path),
                }),
            )
        })?;

    // Determine content type from file extension
    let content_type = if file_path.ends_with(".md") {
        "text/markdown; charset=utf-8"
    } else if file_path.ends_with(".json") {
        "application/json; charset=utf-8"
    } else {
        "text/plain; charset=utf-8"
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(content))
        .unwrap())
}
