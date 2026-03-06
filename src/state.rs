use sqlx::SqlitePool;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};

use crate::broadcast::Broadcaster;
use crate::config::DaemonConfig;
use crate::watcher::FileEvent;
use crate::watcher::pipeline::StateUpdate;

/// Shared application state passed to all Axum handlers via Arc<AppState>.
pub struct AppState {
    /// Database connection pool
    pub db: SqlitePool,
    /// Daemon configuration
    pub config: DaemonConfig,
    /// Server start time for uptime calculation
    pub start_time: Instant,
    /// Broadcast channel for real-time state updates (pipeline -> WebSocket clients)
    pub broadcast_tx: broadcast::Sender<StateUpdate>,
    /// Channel for sending raw file events to the watcher (for dynamic watch management)
    pub file_event_tx: mpsc::Sender<FileEvent>,
    /// Per-project broadcaster for WebSocket subscriptions
    pub broadcaster: Broadcaster,
}
