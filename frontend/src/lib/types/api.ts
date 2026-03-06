/**
 * TypeScript interfaces matching Rust serde types exactly.
 *
 * Source: src/db/models.rs, src/ws/messages.rs, src/watcher/pipeline.rs
 *
 * Field names are snake_case to match Rust serde serialization.
 * Option<T> in Rust maps to T | null in TypeScript.
 * HashMap<String, T> maps to Record<string, T>.
 */

// ---------------------------------------------------------------------------
// Database row types (from src/db/models.rs)
// ---------------------------------------------------------------------------

export interface Project {
	id: string;
	name: string;
	path: string;
	status: string; // "active" | "offline"
	retention_days: number | null;
	created_at: string;
	last_seen_at: string;
}

export interface PhaseState {
	id: number;
	project_id: string;
	phase_number: string;
	phase_name: string;
	goal: string | null;
	depends_on: string | null;
	stage: string; // "planned" | "discussed" | "researched" | "planned_ready" | "executing" | "executed" | "verified"
	status: string | null;
	requirements: string | null;
	plan_count: number | null;
	completed_plan_count: number | null;
	updated_at: string;
}

export interface PlanState {
	id: number;
	project_id: string;
	phase_number: string;
	plan_number: string;
	plan_name: string | null;
	wave: number | null;
	depends_on: string | null;
	plan_type: string | null;
	status: string; // "pending" | "working" | "done" | "failed"
	requirements: string | null;
	files_modified: string | null;
	updated_at: string;
}

export interface ExecutionRun {
	id: number;
	project_id: string;
	phase_number: string;
	plan_number: string;
	run_number: number;
	superseded: number;
	started_at: string | null;
	completed_at: string | null;
	duration_minutes: number | null;
	status: string | null;
	key_files_created: string | null;
	key_files_modified: string | null;
	requirements_completed: string | null;
	created_at: string;
}

export interface AgentSession {
	id: number;
	project_id: string;
	agent_id: string | null;
	agent_type: string | null;
	phase_number: string | null;
	plan_number: string | null;
	started_at: string | null;
	ended_at: string | null;
	created_at: string;
}

export interface VerificationResult {
	id: number;
	project_id: string;
	phase_number: string;
	status: string; // "passed" | "gaps_found" | "human_needed"
	score: string | null;
	verified_at: string | null;
	created_at: string;
}

export interface ProjectConfig {
	id: number;
	project_id: string;
	config_json: string;
	updated_at: string;
}

export interface ParseError {
	id: number;
	project_id: string;
	file_path: string;
	error_message: string;
	severity: string; // "warning" | "error"
	occurred_at: string;
	resolved_at: string | null;
}

// ---------------------------------------------------------------------------
// Composite state (from src/ws/messages.rs -> ProjectState)
// ---------------------------------------------------------------------------

export interface ProjectState {
	project: Project;
	phases: PhaseState[];
	plans: Record<string, PlanState[]>; // keyed by phase_number
	recent_runs: ExecutionRun[];
	agent_sessions: AgentSession[];
	verifications: Record<string, VerificationResult>; // keyed by phase_number
	config: ProjectConfig | null;
	parse_errors: ParseError[];
}

export interface ProjectWatcherStatus {
	active: boolean;
	watched_paths: number;
	last_event_at: string | null;
	error_count: number;
}
