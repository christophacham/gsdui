use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use gsdui::api;
use gsdui::config::DaemonConfig;
use gsdui::db;
use gsdui::state::AppState;
use reqwest::StatusCode;
use tower_http::services::{ServeDir, ServeFile};

/// Spin up a test server on a random port with an in-memory SQLite database.
/// Returns the base URL (e.g., "http://127.0.0.1:12345").
async fn spawn_test_server() -> String {
    let pool = db::init_pool("sqlite::memory:").await.unwrap();

    let config = DaemonConfig {
        listen_addr: "127.0.0.1:0".to_string(),
        database_url: "sqlite::memory:".to_string(),
        static_dir: "static".to_string(),
    };

    let state = Arc::new(AppState {
        db: pool,
        config: config.clone(),
        start_time: Instant::now(),
    });

    let app = Router::new()
        .nest("/api/v1", api::router())
        .fallback_service(
            ServeDir::new(&config.static_dir).not_found_service(
                ServeFile::new(format!("{}/index.html", config.static_dir)),
            ),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    base_url
}

#[tokio::test]
async fn test_project_crud_lifecycle() {
    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Create a temp directory with .planning/ to simulate a real project
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(".planning")).unwrap();
    let project_path = tmp.path().to_str().unwrap().to_string();

    // POST /api/v1/projects -- create project
    let resp = client
        .post(format!("{}/api/v1/projects", base_url))
        .json(&serde_json::json!({
            "name": "Test Project",
            "path": project_path
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let project: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(project["name"], "Test Project");
    assert_eq!(project["path"], project_path);
    assert_eq!(project["status"], "active");
    let project_id = project["id"].as_str().unwrap().to_string();

    // GET /api/v1/projects -- list projects
    let resp = client
        .get(format!("{}/api/v1/projects", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let projects: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["name"], "Test Project");

    // GET /api/v1/projects/:id -- get single project
    let resp = client
        .get(format!("{}/api/v1/projects/{}", base_url, project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let project: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(project["name"], "Test Project");

    // PUT /api/v1/projects/:id -- update project
    let resp = client
        .put(format!("{}/api/v1/projects/{}", base_url, project_id))
        .json(&serde_json::json!({
            "name": "Renamed Project",
            "retention_days": 90
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let project: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(project["name"], "Renamed Project");
    assert_eq!(project["retention_days"], 90);

    // DELETE /api/v1/projects/:id -- delete project
    let resp = client
        .delete(format!("{}/api/v1/projects/{}", base_url, project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify it's gone
    let resp = client
        .get(format!("{}/api/v1/projects/{}", base_url, project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_duplicate_path_returns_409() {
    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(".planning")).unwrap();
    let project_path = tmp.path().to_str().unwrap().to_string();

    // Create first project
    let resp = client
        .post(format!("{}/api/v1/projects", base_url))
        .json(&serde_json::json!({
            "name": "First",
            "path": project_path
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Try to create second project with same path
    let resp = client
        .post(format!("{}/api/v1/projects", base_url))
        .json(&serde_json::json!({
            "name": "Second",
            "path": project_path
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_create_project_invalid_path_returns_400() {
    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Non-existent path
    let resp = client
        .post(format!("{}/api/v1/projects", base_url))
        .json(&serde_json::json!({
            "name": "Bad",
            "path": "/nonexistent/path/abc123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Path exists but no .planning/ directory
    let tmp = tempfile::tempdir().unwrap();
    let resp = client
        .post(format!("{}/api/v1/projects", base_url))
        .json(&serde_json::json!({
            "name": "No Planning",
            "path": tmp.path().to_str().unwrap()
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_health_endpoint() {
    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/api/v1/health", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let health: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(health["status"], "ok");
    assert!(health["uptime_secs"].is_number());
    assert!(health["db_size_bytes"].is_number());
    assert_eq!(health["ws_client_count"], 0);
    assert_eq!(health["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn test_static_file_serving() {
    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.text().await.unwrap();
    assert!(body.contains("GSD Pipeline UI"));
    assert!(body.contains("#040814"));
}

#[tokio::test]
async fn test_get_nonexistent_project_returns_404() {
    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/api/v1/projects/nonexistent-id", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_nonexistent_project_returns_404() {
    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .delete(format!("{}/api/v1/projects/nonexistent-id", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
