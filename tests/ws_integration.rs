//! Integration tests for WebSocket endpoint with snapshot+delta protocol.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use gsdui::api;
use gsdui::broadcast::Broadcaster;
use gsdui::config::DaemonConfig;
use gsdui::db;
use gsdui::state::AppState;
use gsdui::watcher::pipeline::{StateChange, StateUpdate};
use gsdui::ws;
use gsdui::ws::messages::{ClientMessage, WsMessage};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite::Message;
use tower_http::services::{ServeDir, ServeFile};

/// Start a test server with in-memory DB and return (base_url, state).
async fn spawn_test_server() -> (String, Arc<AppState>) {
    let pool = db::init_pool("sqlite::memory:").await.unwrap();

    let config = DaemonConfig {
        listen_addr: "127.0.0.1:0".to_string(),
        database_url: "sqlite::memory:".to_string(),
        static_dir: "static".to_string(),
    };

    let (broadcast_tx, _rx) = broadcast::channel(64);
    let (file_event_tx, _file_event_rx) = mpsc::channel(256);
    let broadcaster = Broadcaster::new();

    let state = Arc::new(AppState {
        db: pool,
        config: config.clone(),
        start_time: Instant::now(),
        broadcast_tx,
        file_event_tx,
        broadcaster,
    });

    let app_state = state.clone();
    let app = Router::new()
        .route("/api/v1/ws/state", get(ws::ws_handler))
        .nest("/api/v1", api::router())
        .fallback_service(
            ServeDir::new(&config.static_dir)
                .not_found_service(ServeFile::new(format!("{}/index.html", config.static_dir))),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("ws://{}", addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (base_url, app_state)
}

/// Register a test project in the database and return the project ID.
async fn setup_test_project(state: &AppState) -> String {
    let project_id = "test-ws-project";
    db::schema::create_project(&state.db, project_id, "WS Test Project", "/tmp/ws-test")
        .await
        .unwrap();

    // Insert some phase state for snapshot data
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

    project_id.to_string()
}

/// Connect a WebSocket client and subscribe to given projects.
/// Returns the (write, read) split streams.
async fn connect_and_subscribe(
    base_url: &str,
    projects: Vec<String>,
) -> (
    futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) {
    let ws_url = format!("{}/api/v1/ws/state", base_url);
    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("Failed to connect WebSocket");

    let (mut write, read) = ws_stream.split();

    // Send subscribe message
    let subscribe_msg = ClientMessage::Subscribe { projects };
    let json = serde_json::to_string(&subscribe_msg).unwrap();
    write.send(Message::Text(json.into())).await.unwrap();

    (write, read)
}

// ---------------------------------------------------------------------------
// Test: Connect, subscribe, receive snapshot
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_subscribe_receives_snapshot() {
    let (base_url, state) = spawn_test_server().await;
    let project_id = setup_test_project(&state).await;

    let (_write, mut read) = connect_and_subscribe(&base_url, vec![project_id.clone()]).await;

    // Should receive a snapshot message
    let msg = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timed out waiting for snapshot")
        .expect("Stream ended unexpectedly")
        .expect("WebSocket error");

    if let Message::Text(text) = msg {
        let ws_msg: WsMessage = serde_json::from_str(&text).expect("Failed to parse WsMessage");
        match ws_msg {
            WsMessage::Snapshot { project, data } => {
                assert_eq!(project, project_id);
                assert_eq!(data.project.id, project_id);
                assert!(!data.phases.is_empty(), "Should include phase data");
                assert!(
                    data.plans.contains_key("01"),
                    "Should include plans for phase 01"
                );
            }
            other => panic!("Expected Snapshot, got {:?}", other),
        }
    } else {
        panic!("Expected text message, got {:?}", msg);
    }
}

// ---------------------------------------------------------------------------
// Test: Subscribe to multiple projects, receive snapshots for each
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_subscribe_multiple_projects() {
    let (base_url, state) = spawn_test_server().await;

    // Create two projects
    let project_id1 = "multi-project-1";
    db::schema::create_project(&state.db, project_id1, "Project 1", "/tmp/ws-test-1")
        .await
        .unwrap();
    let project_id2 = "multi-project-2";
    db::schema::create_project(&state.db, project_id2, "Project 2", "/tmp/ws-test-2")
        .await
        .unwrap();

    let (_write, mut read) = connect_and_subscribe(
        &base_url,
        vec![project_id1.to_string(), project_id2.to_string()],
    )
    .await;

    // Should receive two snapshot messages (one per project)
    let mut received_projects = Vec::new();
    for _ in 0..2 {
        let msg = tokio::time::timeout(Duration::from_secs(5), read.next())
            .await
            .expect("Timed out")
            .expect("Stream ended")
            .expect("Error");

        if let Message::Text(text) = msg {
            let ws_msg: WsMessage = serde_json::from_str(&text).unwrap();
            if let WsMessage::Snapshot { project, .. } = ws_msg {
                received_projects.push(project);
            }
        }
    }

    received_projects.sort();
    assert_eq!(received_projects.len(), 2);
    assert!(received_projects.contains(&project_id1.to_string()));
    assert!(received_projects.contains(&project_id2.to_string()));
}

// ---------------------------------------------------------------------------
// Test: Delta updates forwarded from broadcast
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_delta_from_broadcast() {
    let (base_url, state) = spawn_test_server().await;
    let project_id = setup_test_project(&state).await;

    let (_write, mut read) = connect_and_subscribe(&base_url, vec![project_id.clone()]).await;

    // Skip the snapshot message
    let _snapshot = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    // Broadcast a state update to the project's per-project channel
    let update = StateUpdate {
        project_id: project_id.clone(),
        change: StateChange::ConfigUpdated,
    };
    state
        .broadcaster
        .broadcast(&project_id, update)
        .await
        .unwrap();

    // Should receive a delta message
    let msg = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timed out waiting for delta")
        .expect("Stream ended")
        .expect("WebSocket error");

    if let Message::Text(text) = msg {
        let ws_msg: WsMessage = serde_json::from_str(&text).unwrap();
        match ws_msg {
            WsMessage::Delta { project, changes } => {
                assert_eq!(project, project_id);
                assert_eq!(changes.len(), 1);
            }
            WsMessage::Health { .. } => {
                // Health may arrive before delta; try once more
                let msg2 = tokio::time::timeout(Duration::from_secs(5), read.next())
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap();
                if let Message::Text(text2) = msg2 {
                    let ws_msg2: WsMessage = serde_json::from_str(&text2).unwrap();
                    if let WsMessage::Delta { project, changes } = ws_msg2 {
                        assert_eq!(project, project_id);
                        assert_eq!(changes.len(), 1);
                    } else {
                        panic!("Expected Delta, got {:?}", ws_msg2);
                    }
                }
            }
            other => panic!("Expected Delta or Health, got {:?}", other),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Reconnection receives fresh snapshot
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_reconnect_receives_fresh_snapshot() {
    let (base_url, state) = spawn_test_server().await;
    let project_id = setup_test_project(&state).await;

    // First connection
    {
        let (mut write, mut read) =
            connect_and_subscribe(&base_url, vec![project_id.clone()]).await;

        // Receive snapshot
        let _snapshot = tokio::time::timeout(Duration::from_secs(5), read.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        // Close connection
        write.close().await.ok();
    }

    // Wait briefly for disconnect to be processed
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Second connection (reconnect)
    {
        let (_write, mut read) = connect_and_subscribe(&base_url, vec![project_id.clone()]).await;

        // Should receive a fresh snapshot
        let msg = tokio::time::timeout(Duration::from_secs(5), read.next())
            .await
            .expect("Timed out on reconnect snapshot")
            .expect("Stream ended")
            .expect("WebSocket error");

        if let Message::Text(text) = msg {
            let ws_msg: WsMessage = serde_json::from_str(&text).unwrap();
            match ws_msg {
                WsMessage::Snapshot { project, data } => {
                    assert_eq!(project, project_id);
                    assert_eq!(data.project.id, project_id);
                }
                other => panic!("Expected fresh Snapshot on reconnect, got {:?}", other),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Unsubscribe message handled
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_unsubscribe() {
    let (base_url, state) = spawn_test_server().await;
    let project_id = setup_test_project(&state).await;

    let (mut write, mut read) = connect_and_subscribe(&base_url, vec![project_id.clone()]).await;

    // Skip the snapshot
    let _snapshot = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    // Send unsubscribe
    let unsub_msg = ClientMessage::Unsubscribe {
        projects: vec![project_id.clone()],
    };
    let json = serde_json::to_string(&unsub_msg).unwrap();
    write.send(Message::Text(json.into())).await.unwrap();

    // Connection should still be alive (unsubscribe doesn't close)
    // Wait briefly to ensure no error
    tokio::time::sleep(Duration::from_millis(200)).await;
}

// ---------------------------------------------------------------------------
// Test: Health heartbeat arrives within timeout
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_health_heartbeat() {
    let (base_url, state) = spawn_test_server().await;
    let project_id = setup_test_project(&state).await;

    let (_write, mut read) = connect_and_subscribe(&base_url, vec![project_id.clone()]).await;

    // Skip the snapshot
    let _snapshot = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    // Wait for health heartbeat (interval is 7 seconds, use 10 second timeout)
    let mut received_health = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);

    while tokio::time::Instant::now() < deadline {
        let msg = tokio::time::timeout(Duration::from_secs(10), read.next()).await;

        match msg {
            Ok(Some(Ok(Message::Text(text)))) => {
                let ws_msg: WsMessage = serde_json::from_str(&text).unwrap();
                if let WsMessage::Health {
                    uptime_secs,
                    db_size_bytes,
                    ws_client_count,
                    ..
                } = ws_msg
                {
                    assert!(uptime_secs < 60, "Uptime should be recent");
                    assert!(db_size_bytes >= 0, "DB size should be non-negative");
                    assert!(ws_client_count >= 1, "Should count at least this client");
                    received_health = true;
                    break;
                }
            }
            _ => break,
        }
    }

    assert!(
        received_health,
        "Should receive a health heartbeat within 10 seconds"
    );
}
