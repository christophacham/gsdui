use sqlx::SqlitePool;
use std::time::Instant;

use crate::config::DaemonConfig;

/// Shared application state passed to all Axum handlers via Arc<AppState>.
pub struct AppState {
    /// Database connection pool
    pub db: SqlitePool,
    /// Daemon configuration
    pub config: DaemonConfig,
    /// Server start time for uptime calculation
    pub start_time: Instant,
    // Future plans will add:
    // pub broadcast: Broadcaster,
    // pub watcher_handle: ...
}
