use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
    routing::put,
};
use serde::{Deserialize, Serialize};

use crate::db;
use crate::state::AppState;

/// Build the config API sub-router.
/// Mounted at /api/v1/projects/:id/...
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/config", put(update_config))
}

#[derive(Debug, Deserialize)]
struct UpdateConfigRequest {
    config_json: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct UpdateConfigResponse {
    config_json: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

/// PUT /api/v1/projects/:id/config -- Update project config
///
/// Writes the config JSON to {project_path}/.planning/config.json on disk.
/// The file watcher will detect the change, parse it, update the DB,
/// and broadcast a ConfigUpdated delta to WebSocket subscribers.
async fn update_config(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<UpdateConfigRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Look up the project to get its filesystem path
    let project = db::schema::get_project(&state.db, &id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Database error: {}", e),
            }),
        )
    })?;

    let project = project.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Project not found: {}", id),
            }),
        )
    })?;

    // Build the config file path
    let config_path = std::path::PathBuf::from(&project.path)
        .join(".planning")
        .join("config.json");

    // Serialize the config JSON with pretty printing
    let config_str = serde_json::to_string_pretty(&body.config_json).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to serialize config: {}", e),
            }),
        )
    })?;

    // Ensure the .planning directory exists
    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to create directory: {}", e),
                }),
            )
        })?;
    }

    // Write the config file -- the file watcher will pick up the change
    tokio::fs::write(&config_path, &config_str)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to write config file: {}", e),
                }),
            )
        })?;

    Ok(Json(UpdateConfigResponse {
        config_json: body.config_json,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_update_config_request() {
        let json = r#"{"config_json": {"workflow": {"planning": {"model": "claude"}}}}"#;
        let req: UpdateConfigRequest = serde_json::from_str(json).unwrap();
        assert!(req.config_json.is_object());
        assert!(
            req.config_json["workflow"]["planning"]["model"]
                .as_str()
                .unwrap()
                == "claude"
        );
    }

    #[test]
    fn test_serialize_update_config_response() {
        let resp = UpdateConfigResponse {
            config_json: serde_json::json!({"test": true}),
        };
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains("test"));
    }
}
