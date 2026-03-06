use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::db::models::{
    AgentSession, ExecutionRun, ParseError, PhaseState, PlanState, Project, ProjectConfig,
    VerificationResult,
};
use crate::watcher::pipeline::StateChange;

// ---------------------------------------------------------------------------
// Server-to-client messages
// ---------------------------------------------------------------------------

/// Messages sent from server to WebSocket clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Full state snapshot for one project (sent on subscribe)
    #[serde(rename = "snapshot")]
    Snapshot {
        project: String,
        data: ProjectState,
    },

    /// Incremental delta updates for a project
    #[serde(rename = "delta")]
    Delta {
        project: String,
        changes: Vec<StateChange>,
    },

    /// Health heartbeat with daemon diagnostics
    #[serde(rename = "health")]
    Health {
        uptime_secs: u64,
        db_size_bytes: i64,
        ws_client_count: u32,
        watcher_queue_depth: u32,
        memory_usage_bytes: u64,
        per_project_status: HashMap<String, ProjectWatcherStatus>,
    },

    /// Error message
    #[serde(rename = "error")]
    Error { message: String, code: String },
}

// ---------------------------------------------------------------------------
// Client-to-server messages
// ---------------------------------------------------------------------------

/// Messages sent from WebSocket clients to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Subscribe to receive updates for specific projects
    #[serde(rename = "subscribe")]
    Subscribe { projects: Vec<String> },

    /// Unsubscribe from projects
    #[serde(rename = "unsubscribe")]
    Unsubscribe { projects: Vec<String> },
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Full project state sent as a WebSocket snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    pub project: Project,
    pub phases: Vec<PhaseState>,
    pub plans: HashMap<String, Vec<PlanState>>,
    pub recent_runs: Vec<ExecutionRun>,
    pub agent_sessions: Vec<AgentSession>,
    pub verifications: HashMap<String, VerificationResult>,
    pub config: Option<ProjectConfig>,
    pub parse_errors: Vec<ParseError>,
}

/// Status of a file watcher for a specific project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectWatcherStatus {
    pub active: bool,
    pub watched_paths: u32,
    pub last_event_at: Option<String>,
    pub error_count: u32,
}
