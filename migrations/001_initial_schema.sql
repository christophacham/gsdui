-- GSD Pipeline UI - Initial Schema
-- All tables for Phase 1: project management, parsed state, execution history

-- Projects registered for monitoring
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'active',
    retention_days INTEGER DEFAULT 180,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Current parsed state per phase (cache of parsed files, replaced on each parse)
CREATE TABLE phase_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    phase_number TEXT NOT NULL,
    phase_name TEXT NOT NULL,
    goal TEXT,
    depends_on TEXT,
    stage TEXT NOT NULL DEFAULT 'planned',
    status TEXT,
    requirements TEXT,
    plan_count INTEGER DEFAULT 0,
    completed_plan_count INTEGER DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, phase_number)
);

-- Current parsed state per plan (cache of parsed PLAN.md/SUMMARY.md)
CREATE TABLE plan_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    phase_number TEXT NOT NULL,
    plan_number TEXT NOT NULL,
    plan_name TEXT,
    wave INTEGER,
    depends_on TEXT,
    plan_type TEXT DEFAULT 'execute',
    status TEXT NOT NULL DEFAULT 'pending',
    requirements TEXT,
    files_modified TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, phase_number, plan_number)
);

-- Execution history (persists across file changes, supports "keep all runs")
CREATE TABLE execution_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    phase_number TEXT NOT NULL,
    plan_number TEXT NOT NULL,
    run_number INTEGER NOT NULL DEFAULT 1,
    superseded INTEGER NOT NULL DEFAULT 0,
    started_at TEXT,
    completed_at TEXT,
    duration_minutes REAL,
    status TEXT,
    key_files_created TEXT,
    key_files_modified TEXT,
    requirements_completed TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Individual commit records from SUMMARY.md Task Commits section
CREATE TABLE commits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    execution_run_id INTEGER NOT NULL REFERENCES execution_runs(id) ON DELETE CASCADE,
    task_number INTEGER NOT NULL,
    task_name TEXT,
    commit_hash TEXT,
    commit_type TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Agent session history from agent-history.json
CREATE TABLE agent_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    agent_id TEXT,
    agent_type TEXT,
    phase_number TEXT,
    plan_number TEXT,
    started_at TEXT,
    ended_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Verification results from VERIFICATION.md
CREATE TABLE verification_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    phase_number TEXT NOT NULL,
    status TEXT NOT NULL,
    score TEXT,
    verified_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, phase_number)
);

-- Parse errors (accumulated with timestamps, queryable)
CREATE TABLE parse_errors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    error_message TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'warning',
    occurred_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT
);

-- Project configuration cache (from config.json)
CREATE TABLE project_config (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    config_json TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id)
);

-- Indexes for common query patterns
CREATE INDEX idx_phase_state_project ON phase_state(project_id);
CREATE INDEX idx_plan_state_project ON plan_state(project_id, phase_number);
CREATE INDEX idx_execution_runs_project ON execution_runs(project_id, phase_number, plan_number);
CREATE INDEX idx_agent_sessions_project ON agent_sessions(project_id);
CREATE INDEX idx_parse_errors_project ON parse_errors(project_id, resolved_at);
CREATE INDEX idx_commits_run ON commits(execution_run_id);
