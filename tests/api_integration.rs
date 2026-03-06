use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use gsdui::api;
use gsdui::broadcast::Broadcaster;
use gsdui::config::DaemonConfig;
use gsdui::db;
use gsdui::state::AppState;
use reqwest::StatusCode;
use tokio::sync::{broadcast, mpsc};
use tower_http::services::{ServeDir, ServeFile};

/// Spin up a test server on a random port with an in-memory SQLite database.
/// Returns (base_url, state).
async fn spawn_test_server() -> (String, Arc<AppState>) {
    let pool = db::init_pool("sqlite::memory:").await.unwrap();

    let config = DaemonConfig {
        listen_addr: "127.0.0.1:0".to_string(),
        database_url: "sqlite::memory:".to_string(),
        static_dir: "static".to_string(),
    };

    let (broadcast_tx, _rx) = broadcast::channel(64);
    let (file_event_tx, _file_event_rx) = mpsc::channel(256);

    let state = Arc::new(AppState {
        db: pool,
        config: config.clone(),
        start_time: Instant::now(),
        broadcast_tx,
        file_event_tx,
        broadcaster: Broadcaster::new(),
    });

    let app_state = state.clone();
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

    (base_url, app_state)
}

// =====================================================================
// Existing tests (from Plan 01-01)
// =====================================================================

#[tokio::test]
async fn test_project_crud_lifecycle() {
    let (base_url, _state) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(".planning")).unwrap();
    let project_path = tmp.path().to_str().unwrap().to_string();

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
    let project_id = project["id"].as_str().unwrap().to_string();

    let resp = client
        .get(format!("{}/api/v1/projects", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let projects: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(projects.len(), 1);

    let resp = client
        .get(format!("{}/api/v1/projects/{}", base_url, project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

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

    let resp = client
        .delete(format!("{}/api/v1/projects/{}", base_url, project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = client
        .get(format!("{}/api/v1/projects/{}", base_url, project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_duplicate_path_returns_409() {
    let (base_url, _state) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(".planning")).unwrap();
    let project_path = tmp.path().to_str().unwrap().to_string();

    client
        .post(format!("{}/api/v1/projects", base_url))
        .json(&serde_json::json!({
            "name": "First",
            "path": project_path
        }))
        .send()
        .await
        .unwrap();

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
    let (base_url, _state) = spawn_test_server().await;
    let client = reqwest::Client::new();

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
    let (base_url, _state) = spawn_test_server().await;
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
    // New diagnostic fields
    assert!(health["memory_usage_bytes"].is_number());
    assert!(health["per_project_watcher_status"].is_object());
    assert!(health["parse_error_counts"].is_object());
}

#[tokio::test]
async fn test_static_file_serving() {
    let (base_url, _state) = spawn_test_server().await;
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
    let (base_url, _state) = spawn_test_server().await;
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
    let (base_url, _state) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .delete(format!("{}/api/v1/projects/nonexistent-id", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// =====================================================================
// New tests (Plan 01-04 Task 2)
// =====================================================================

/// Helper: register a project directly in the DB and populate it with test data.
async fn setup_project_with_data(state: &AppState) -> String {
    let project_id = "api-test-project";
    db::schema::create_project(&state.db, project_id, "API Test Project", "/tmp/api-test")
        .await
        .unwrap();

    // Insert phase state
    db::schema::upsert_phase_state(
        &state.db,
        project_id,
        "01",
        "backend-foundation",
        Some("Build the backend"),
        None,
        "executing",
        Some("active"),
        Some("STATE-01"),
        2,
        1,
    )
    .await
    .unwrap();

    // Insert plan state
    db::schema::upsert_plan_state(
        &state.db,
        project_id,
        "01",
        "01",
        Some("Project scaffold"),
        Some(1),
        None,
        Some("execute"),
        "done",
        Some("STATE-01"),
        None,
    )
    .await
    .unwrap();

    db::schema::upsert_plan_state(
        &state.db,
        project_id,
        "01",
        "02",
        Some("Parsers"),
        Some(2),
        Some("01-01"),
        Some("execute"),
        "pending",
        Some("STATE-02"),
        None,
    )
    .await
    .unwrap();

    // Insert execution run
    db::schema::insert_execution_run(
        &state.db,
        project_id,
        "01",
        "01",
        1,
        Some("2026-03-06T10:00:00Z"),
        Some("completed"),
    )
    .await
    .unwrap();

    // Insert agent session
    db::schema::insert_agent_session(
        &state.db,
        project_id,
        Some("agent-1"),
        Some("executor"),
        Some("01"),
        Some("01"),
        Some("2026-03-06T10:00:00Z"),
        Some("2026-03-06T10:09:00Z"),
    )
    .await
    .unwrap();

    project_id.to_string()
}

// ---------------------------------------------------------------------------
// State endpoint tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_state_endpoint_returns_full_state() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let project_id = setup_project_with_data(&state).await;

    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/state",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["project"]["id"], project_id);
    assert!(body["phases"].is_array());
    assert!(body["plans"].is_object());
    assert!(body["recent_runs"].is_array());
    assert!(body["agent_sessions"].is_array());
    assert!(body["parse_errors"].is_array());
}

#[tokio::test]
async fn test_state_endpoint_404_for_unknown_project() {
    let (base_url, _state) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!(
            "{}/api/v1/projects/nonexistent/state",
            base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_phases_endpoint() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let project_id = setup_project_with_data(&state).await;

    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/phases",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let phases: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(phases.len(), 1);
    assert_eq!(phases[0]["phase_number"], "01");
    assert_eq!(phases[0]["phase_name"], "backend-foundation");
}

#[tokio::test]
async fn test_phase_plans_endpoint() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let project_id = setup_project_with_data(&state).await;

    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/phases/01/plans",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let plans: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(plans.len(), 2);
}

#[tokio::test]
async fn test_errors_endpoint() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let project_id = setup_project_with_data(&state).await;

    // Insert a parse error
    db::schema::insert_parse_error(
        &state.db,
        &project_id,
        "/test/STATE.md",
        "Invalid frontmatter",
        "error",
    )
    .await
    .unwrap();

    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/errors",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let errors: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0]["error_message"], "Invalid frontmatter");
}

// ---------------------------------------------------------------------------
// History endpoint tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_history_runs_endpoint() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let project_id = setup_project_with_data(&state).await;

    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/history/runs",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["runs"].is_array());
    assert_eq!(body["runs"].as_array().unwrap().len(), 1);
    assert!(body["pagination"].is_object());
    assert_eq!(body["pagination"]["total"], 1);
}

#[tokio::test]
async fn test_history_runs_with_filters() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let project_id = setup_project_with_data(&state).await;

    // Filter by phase
    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/history/runs?phase=01",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["pagination"]["total"], 1);

    // Filter by non-existent phase
    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/history/runs?phase=99",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["pagination"]["total"], 0);
}

#[tokio::test]
async fn test_history_runs_pagination() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let project_id = setup_project_with_data(&state).await;

    // Insert additional runs
    for i in 2..=5 {
        db::schema::insert_execution_run(
            &state.db,
            &project_id,
            "01",
            "01",
            i,
            Some("2026-03-06T12:00:00Z"),
            Some("completed"),
        )
        .await
        .unwrap();
    }

    // Request with limit=2
    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/history/runs?limit=2&offset=0",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["runs"].as_array().unwrap().len(), 2);
    assert_eq!(body["pagination"]["total"], 5);
    assert_eq!(body["pagination"]["limit"], 2);
    assert_eq!(body["pagination"]["offset"], 0);
}

#[tokio::test]
async fn test_history_run_commits() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let project_id = setup_project_with_data(&state).await;

    // Get the run ID
    let runs = db::schema::get_runs_for_plan(&state.db, &project_id, "01", "01")
        .await
        .unwrap();
    let run_id = runs[0].id;

    // Insert a commit
    db::schema::insert_commit(&state.db, run_id, 1, Some("scaffold"), Some("abc1234"), Some("feat"))
        .await
        .unwrap();

    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/history/runs/{}/commits",
            base_url, project_id, run_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let commits: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0]["task_name"], "scaffold");
    assert_eq!(commits[0]["commit_hash"], "abc1234");
}

#[tokio::test]
async fn test_history_agents_endpoint() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let project_id = setup_project_with_data(&state).await;

    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/history/agents",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["agent_type"], "executor");
}

#[tokio::test]
async fn test_history_agents_with_filter() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let project_id = setup_project_with_data(&state).await;

    // Filter by agent_type
    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/history/agents?agent_type=executor",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(sessions.len(), 1);

    // Filter by non-matching type
    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/history/agents?agent_type=planner",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(sessions.len(), 0);
}

// ---------------------------------------------------------------------------
// File content endpoint tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_files_endpoint_returns_markdown() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Create a temp directory with .planning/ and test files
    let tmp = tempfile::tempdir().unwrap();
    let planning_dir = tmp.path().join(".planning");
    std::fs::create_dir_all(&planning_dir).unwrap();
    std::fs::write(
        planning_dir.join("STATE.md"),
        "---\nstatus: executing\n---\n# State\n",
    )
    .unwrap();

    let project_id = "file-test-project";
    db::schema::create_project(
        &state.db,
        project_id,
        "File Test",
        tmp.path().to_str().unwrap(),
    )
    .await
    .unwrap();

    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/files/STATE.md",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let content_type = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(
        content_type.contains("text/markdown"),
        "Should return text/markdown, got {}",
        content_type
    );

    let body = resp.text().await.unwrap();
    assert!(body.contains("status: executing"));
}

#[tokio::test]
async fn test_files_endpoint_returns_json() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let tmp = tempfile::tempdir().unwrap();
    let planning_dir = tmp.path().join(".planning");
    std::fs::create_dir_all(&planning_dir).unwrap();
    std::fs::write(
        planning_dir.join("config.json"),
        r#"{"mode": "yolo"}"#,
    )
    .unwrap();

    let project_id = "json-test-project";
    db::schema::create_project(
        &state.db,
        project_id,
        "JSON Test",
        tmp.path().to_str().unwrap(),
    )
    .await
    .unwrap();

    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/files/config.json",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let content_type = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(
        content_type.contains("application/json"),
        "Should return application/json, got {}",
        content_type
    );
}

#[tokio::test]
async fn test_files_endpoint_nested_path() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let tmp = tempfile::tempdir().unwrap();
    let phase_dir = tmp
        .path()
        .join(".planning")
        .join("phases")
        .join("01-test");
    std::fs::create_dir_all(&phase_dir).unwrap();
    std::fs::write(
        phase_dir.join("01-01-PLAN.md"),
        "---\nphase: 01-test\nplan: 1\n---\n# Plan\n",
    )
    .unwrap();

    let project_id = "nested-file-project";
    db::schema::create_project(
        &state.db,
        project_id,
        "Nested Test",
        tmp.path().to_str().unwrap(),
    )
    .await
    .unwrap();

    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/files/phases/01-test/01-01-PLAN.md",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text().await.unwrap();
    assert!(body.contains("phase: 01-test"));
}

#[tokio::test]
async fn test_files_endpoint_path_traversal_blocked() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let tmp = tempfile::tempdir().unwrap();
    let planning_dir = tmp.path().join(".planning");
    std::fs::create_dir_all(&planning_dir).unwrap();

    let project_id = "traversal-test-project";
    db::schema::create_project(
        &state.db,
        project_id,
        "Traversal Test",
        tmp.path().to_str().unwrap(),
    )
    .await
    .unwrap();

    // Attempt path traversal with URL-encoded ..
    // The raw ".." in URLs gets resolved by the HTTP client/router.
    // To test our server-side validation, we URL-encode the dots.
    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/files/phases%2F..%2F..%2Fetc%2Fpasswd",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_files_endpoint_not_found() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let tmp = tempfile::tempdir().unwrap();
    let planning_dir = tmp.path().join(".planning");
    std::fs::create_dir_all(&planning_dir).unwrap();

    let project_id = "notfound-file-project";
    db::schema::create_project(
        &state.db,
        project_id,
        "NotFound Test",
        tmp.path().to_str().unwrap(),
    )
    .await
    .unwrap();

    let resp = client
        .get(format!(
            "{}/api/v1/projects/{}/files/nonexistent.md",
            base_url, project_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Health endpoint comprehensive diagnostics test
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_health_endpoint_with_project_diagnostics() {
    let (base_url, state) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Create a project so diagnostics have data
    let project_id = "health-diag-project";
    db::schema::create_project(&state.db, project_id, "Health Diag", "/tmp/health-diag")
        .await
        .unwrap();

    // Insert a parse error
    db::schema::insert_parse_error(
        &state.db,
        project_id,
        "/test/bad.md",
        "Parse error",
        "error",
    )
    .await
    .unwrap();

    let resp = client
        .get(format!("{}/api/v1/health", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let health: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(health["status"], "ok");

    // Per-project watcher status should include our project
    let watcher_status = &health["per_project_watcher_status"];
    assert!(
        watcher_status[project_id].is_object(),
        "Should have per-project status"
    );
    assert_eq!(watcher_status[project_id]["active"], true);

    // Parse error counts should include our project
    let error_counts = &health["parse_error_counts"];
    assert!(
        error_counts[project_id].is_object(),
        "Should have per-project error counts"
    );
    assert_eq!(error_counts[project_id]["active"], 1);
    assert_eq!(error_counts[project_id]["resolved"], 0);
}
