/**
 * WebSocket message types matching Rust serde serialization.
 *
 * Server messages use #[serde(tag = "type")] on WsMessage enum.
 * Client messages use #[serde(tag = "type")] on ClientMessage enum.
 * StateChange uses #[serde(tag = "type", content = "data")].
 *
 * Source: src/ws/messages.rs, src/watcher/pipeline.rs
 */

import type { ProjectState, ProjectWatcherStatus } from './api.js';

// ---------------------------------------------------------------------------
// Server-to-client messages (tagged union via "type" field)
// ---------------------------------------------------------------------------

export type WsMessage =
	| { type: 'snapshot'; project: string; data: ProjectState }
	| { type: 'delta'; project: string; changes: StateChange[] }
	| {
			type: 'health';
			uptime_secs: number;
			db_size_bytes: number;
			ws_client_count: number;
			watcher_queue_depth: number;
			memory_usage_bytes: number;
			per_project_status: Record<string, ProjectWatcherStatus>;
		}
	| { type: 'error'; message: string; code: string };

// ---------------------------------------------------------------------------
// State change types (tagged union with "type" + "data")
// ---------------------------------------------------------------------------

export type StateChange =
	| { type: 'PhaseUpdated'; data: { phase_number: string; stage: string } }
	| { type: 'PlanUpdated'; data: { phase_number: string; plan_number: string; status: string } }
	| { type: 'VerificationUpdated'; data: { phase_number: string; status: string } }
	| { type: 'ConfigUpdated' }
	| { type: 'AgentHistoryUpdated'; data: { session_count: number } }
	| { type: 'ProjectStateUpdated'; data: { status: string | null } }
	| { type: 'ParseError'; data: { file_path: string; error: string } };

// ---------------------------------------------------------------------------
// Client-to-server messages
// ---------------------------------------------------------------------------

export type ClientMessage =
	| { type: 'subscribe'; projects: string[] }
	| { type: 'unsubscribe'; projects: string[] };
