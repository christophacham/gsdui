pub mod messages;

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::db;
use crate::state::AppState;
use crate::watcher::pipeline::StateUpdate;
use messages::{ClientMessage, ProjectState, WsMessage};

/// WebSocket upgrade handler.
///
/// Clients connect to `/api/v1/ws/state` and are upgraded to a WebSocket connection.
/// After upgrade, they must send a Subscribe message with project IDs.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle a single WebSocket connection.
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Step 1: Wait for initial Subscribe message
    let subscribed_projects = match wait_for_subscribe(&mut ws_rx).await {
        Ok(projects) => projects,
        Err(e) => {
            warn!("WebSocket client failed to subscribe: {}", e);
            let err_msg = WsMessage::Error {
                message: e.to_string(),
                code: "subscribe_required".to_string(),
            };
            if let Ok(json) = serde_json::to_string(&err_msg) {
                let _ = ws_tx.send(Message::Text(json.into())).await;
            }
            return;
        }
    };

    info!(
        projects = ?subscribed_projects,
        "WebSocket client subscribed"
    );

    // Step 2: Send snapshot for each subscribed project
    for project_id in &subscribed_projects {
        match build_project_state(&state, project_id).await {
            Ok(project_state) => {
                let msg = WsMessage::Snapshot {
                    project: project_id.clone(),
                    data: project_state,
                };
                match serde_json::to_string(&msg) {
                    Ok(json) => {
                        if ws_tx.send(Message::Text(json.into())).await.is_err() {
                            debug!("WebSocket client disconnected during snapshot send");
                            return;
                        }
                    }
                    Err(e) => {
                        error!("Failed to serialize snapshot: {}", e);
                    }
                }
            }
            Err(e) => {
                let err_msg = WsMessage::Error {
                    message: format!("Project not found: {}", project_id),
                    code: "project_not_found".to_string(),
                };
                if let Ok(json) = serde_json::to_string(&err_msg) {
                    let _ = ws_tx.send(Message::Text(json.into())).await;
                }
                warn!(project_id, error = %e, "Failed to build snapshot");
            }
        }
    }

    // Step 3: Subscribe to broadcast channels
    let receivers = state.broadcaster.subscribe(&subscribed_projects).await;

    // Step 4: Spawn tasks for sending deltas, receiving client messages, and health heartbeats

    // Channel for the send task to receive delta updates from all project receivers
    let (delta_tx, mut delta_rx) = tokio::sync::mpsc::channel::<(String, StateUpdate)>(128);

    // Spawn receiver tasks for each project subscription
    let mut receiver_handles = Vec::new();
    for (project_id, mut rx) in receivers {
        let delta_tx = delta_tx.clone();
        let handle = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(update) => {
                        if delta_tx.send((project_id.clone(), update)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        warn!(
                            project_id = %project_id,
                            lagged = count,
                            "WebSocket receiver lagged, skipped messages"
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });
        receiver_handles.push(handle);
    }
    drop(delta_tx); // Drop our copy so the channel closes when all receiver tasks end

    // Send task: forwards deltas and health heartbeats to the WebSocket client
    let health_state = state.clone();
    let cancel = tokio_util::sync::CancellationToken::new();
    let send_cancel = cancel.clone();
    let recv_cancel = cancel.clone();

    let send_task = tokio::spawn(async move {
        let mut health_interval = tokio::time::interval(std::time::Duration::from_secs(7));
        // Skip the first immediate tick
        health_interval.tick().await;

        loop {
            tokio::select! {
                // Forward delta updates
                delta = delta_rx.recv() => {
                    match delta {
                        Some((project_id, update)) => {
                            let msg = WsMessage::Delta {
                                project: project_id,
                                changes: vec![update.change],
                            };
                            match serde_json::to_string(&msg) {
                                Ok(json) => {
                                    if ws_tx.send(Message::Text(json.into())).await.is_err() {
                                        debug!("WebSocket client disconnected during delta send");
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to serialize delta: {}", e);
                                }
                            }
                        }
                        None => {
                            // All receiver tasks ended
                            break;
                        }
                    }
                }

                // Health heartbeat every 7 seconds
                _ = health_interval.tick() => {
                    let health_msg = build_health_message(&health_state).await;
                    match serde_json::to_string(&health_msg) {
                        Ok(json) => {
                            if ws_tx.send(Message::Text(json.into())).await.is_err() {
                                debug!("WebSocket client disconnected during health send");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Failed to serialize health message: {}", e);
                        }
                    }
                }

                _ = send_cancel.cancelled() => {
                    break;
                }
            }
        }
    });

    // Receive task: handles client messages (Unsubscribe, ping/pong)
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(ClientMessage::Unsubscribe { projects: _ }) => {
                            // For now, we handle unsubscribe by logging it.
                            // Full unsubscribe would require dropping specific receivers.
                            debug!("Client sent unsubscribe (noted)");
                        }
                        Ok(ClientMessage::Subscribe { projects: _ }) => {
                            // Additional subscribe not supported after initial
                            debug!("Client sent additional subscribe (ignored)");
                        }
                        Err(e) => {
                            debug!("Unknown client message: {}", e);
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    // axum handles Pong automatically, but log for visibility
                    debug!(len = data.len(), "Received ping");
                }
                Ok(Message::Close(_)) => {
                    debug!("Client sent close frame");
                    break;
                }
                Err(e) => {
                    debug!("WebSocket receive error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        recv_cancel.cancel();
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {
            cancel.cancel();
        }
        _ = recv_task => {
            cancel.cancel();
        }
    }

    // Abort receiver handles
    for handle in receiver_handles {
        handle.abort();
    }

    // Decrement client count
    state.broadcaster.unsubscribe();

    info!("WebSocket client disconnected");
}

/// Wait for the first message to be a Subscribe command.
/// Returns the list of project IDs to subscribe to.
async fn wait_for_subscribe(
    ws_rx: &mut futures_util::stream::SplitStream<WebSocket>,
) -> Result<Vec<String>, String> {
    // Wait up to 10 seconds for the subscribe message
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), ws_rx.next()).await;

    match timeout {
        Ok(Some(Ok(Message::Text(text)))) => match serde_json::from_str::<ClientMessage>(&text) {
            Ok(ClientMessage::Subscribe { projects }) => {
                if projects.is_empty() {
                    Err("Subscribe message must include at least one project".to_string())
                } else {
                    Ok(projects)
                }
            }
            Ok(_) => Err("First message must be a subscribe command".to_string()),
            Err(e) => Err(format!("Invalid message format: {}", e)),
        },
        Ok(Some(Ok(_))) => {
            Err("First message must be a text frame with subscribe command".to_string())
        }
        Ok(Some(Err(e))) => Err(format!("WebSocket error: {}", e)),
        Ok(None) => Err("Connection closed before subscribe".to_string()),
        Err(_) => Err("Timed out waiting for subscribe message".to_string()),
    }
}

/// Build a full ProjectState by querying the database.
pub async fn build_project_state(
    state: &AppState,
    project_id: &str,
) -> Result<ProjectState, String> {
    let project = db::schema::get_project(&state.db, project_id)
        .await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    let phases = db::schema::get_phase_states_for_project(&state.db, project_id)
        .await
        .unwrap_or_default();

    // Build plans map keyed by phase_number
    let mut plans: HashMap<String, Vec<_>> = HashMap::new();
    for phase in &phases {
        let phase_plans =
            db::schema::get_plan_states_for_phase(&state.db, project_id, &phase.phase_number)
                .await
                .unwrap_or_default();
        plans.insert(phase.phase_number.clone(), phase_plans);
    }

    // Get recent execution runs (across all phases/plans, limited)
    let mut recent_runs = Vec::new();
    for phase in &phases {
        if let Some(phase_plans) = plans.get(&phase.phase_number) {
            for plan in phase_plans {
                let mut runs = db::schema::get_runs_for_plan(
                    &state.db,
                    project_id,
                    &phase.phase_number,
                    &plan.plan_number,
                )
                .await
                .unwrap_or_default();
                recent_runs.append(&mut runs);
            }
        }
    }
    // Sort by created_at descending and take most recent
    recent_runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    recent_runs.truncate(50);

    let agent_sessions = db::schema::get_sessions_for_project(&state.db, project_id)
        .await
        .unwrap_or_default();

    // Build verifications map keyed by phase_number
    let mut verifications = HashMap::new();
    for phase in &phases {
        if let Ok(Some(v)) =
            db::schema::get_verification_for_phase(&state.db, project_id, &phase.phase_number).await
        {
            verifications.insert(phase.phase_number.clone(), v);
        }
    }

    let config = db::schema::get_config_for_project(&state.db, project_id)
        .await
        .unwrap_or(None);

    let parse_errors = db::schema::get_active_errors_for_project(&state.db, project_id)
        .await
        .unwrap_or_default();

    Ok(ProjectState {
        project,
        phases,
        plans,
        recent_runs,
        agent_sessions,
        verifications,
        config,
        parse_errors,
    })
}

/// Build a health heartbeat message with daemon diagnostics.
async fn build_health_message(state: &AppState) -> WsMessage {
    let uptime_secs = state.start_time.elapsed().as_secs();

    let db_size: i64 = sqlx::query_scalar(
        "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let ws_client_count = state.broadcaster.client_count();

    // Memory usage from /proc/self/status on Linux
    let memory_usage_bytes = get_memory_usage();

    WsMessage::Health {
        uptime_secs,
        db_size_bytes: db_size,
        ws_client_count,
        watcher_queue_depth: 0, // Approximate; exact tracking would need a shared counter
        memory_usage_bytes,
        per_project_status: HashMap::new(), // Populated when watcher registry is available
    }
}

/// Read memory usage from /proc/self/status (Linux) or return 0.
fn get_memory_usage() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    // Format: "VmRSS:     12345 kB"
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2
                        && let Ok(kb) = parts[1].parse::<u64>()
                    {
                        return kb * 1024; // Convert kB to bytes
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
