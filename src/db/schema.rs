use sqlx::SqlitePool;

use super::models::*;

// --- Project CRUD ---

pub async fn create_project(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    path: &str,
) -> Result<Project, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        "INSERT INTO projects (id, name, path) VALUES (?, ?, ?) RETURNING *",
    )
    .bind(id)
    .bind(name)
    .bind(path)
    .fetch_one(pool)
    .await
}

pub async fn get_project(pool: &SqlitePool, id: &str) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_all_projects(pool: &SqlitePool) -> Result<Vec<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>("SELECT * FROM projects ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
}

pub async fn update_project(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    retention_days: Option<i64>,
) -> Result<Option<Project>, sqlx::Error> {
    // Build dynamic update
    let existing = get_project(pool, id).await?;
    if existing.is_none() {
        return Ok(None);
    }
    let existing = existing.unwrap();
    let new_name = name.unwrap_or(&existing.name);
    let new_retention = retention_days.unwrap_or(existing.retention_days.unwrap_or(180));

    let project = sqlx::query_as::<_, Project>(
        "UPDATE projects SET name = ?, retention_days = ?, last_seen_at = datetime('now') WHERE id = ? RETURNING *",
    )
    .bind(new_name)
    .bind(new_retention)
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(project)
}

pub async fn delete_project(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// --- Phase State ---

pub async fn upsert_phase_state(
    pool: &SqlitePool,
    project_id: &str,
    phase_number: &str,
    phase_name: &str,
    goal: Option<&str>,
    depends_on: Option<&str>,
    stage: &str,
    status: Option<&str>,
    requirements: Option<&str>,
    plan_count: i64,
    completed_plan_count: i64,
) -> Result<PhaseState, sqlx::Error> {
    sqlx::query_as::<_, PhaseState>(
        "INSERT INTO phase_state (project_id, phase_number, phase_name, goal, depends_on, stage, status, requirements, plan_count, completed_plan_count)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(project_id, phase_number) DO UPDATE SET
           phase_name = excluded.phase_name,
           goal = excluded.goal,
           depends_on = excluded.depends_on,
           stage = excluded.stage,
           status = excluded.status,
           requirements = excluded.requirements,
           plan_count = excluded.plan_count,
           completed_plan_count = excluded.completed_plan_count,
           updated_at = datetime('now')
         RETURNING *",
    )
    .bind(project_id)
    .bind(phase_number)
    .bind(phase_name)
    .bind(goal)
    .bind(depends_on)
    .bind(stage)
    .bind(status)
    .bind(requirements)
    .bind(plan_count)
    .bind(completed_plan_count)
    .fetch_one(pool)
    .await
}

pub async fn get_phase_states_for_project(
    pool: &SqlitePool,
    project_id: &str,
) -> Result<Vec<PhaseState>, sqlx::Error> {
    sqlx::query_as::<_, PhaseState>(
        "SELECT * FROM phase_state WHERE project_id = ? ORDER BY phase_number",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
}

// --- Plan State ---

pub async fn upsert_plan_state(
    pool: &SqlitePool,
    project_id: &str,
    phase_number: &str,
    plan_number: &str,
    plan_name: Option<&str>,
    wave: Option<i64>,
    depends_on: Option<&str>,
    plan_type: Option<&str>,
    status: &str,
    requirements: Option<&str>,
    files_modified: Option<&str>,
) -> Result<PlanState, sqlx::Error> {
    sqlx::query_as::<_, PlanState>(
        "INSERT INTO plan_state (project_id, phase_number, plan_number, plan_name, wave, depends_on, plan_type, status, requirements, files_modified)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(project_id, phase_number, plan_number) DO UPDATE SET
           plan_name = excluded.plan_name,
           wave = excluded.wave,
           depends_on = excluded.depends_on,
           plan_type = excluded.plan_type,
           status = excluded.status,
           requirements = excluded.requirements,
           files_modified = excluded.files_modified,
           updated_at = datetime('now')
         RETURNING *",
    )
    .bind(project_id)
    .bind(phase_number)
    .bind(plan_number)
    .bind(plan_name)
    .bind(wave)
    .bind(depends_on)
    .bind(plan_type)
    .bind(status)
    .bind(requirements)
    .bind(files_modified)
    .fetch_one(pool)
    .await
}

pub async fn get_plan_states_for_phase(
    pool: &SqlitePool,
    project_id: &str,
    phase_number: &str,
) -> Result<Vec<PlanState>, sqlx::Error> {
    sqlx::query_as::<_, PlanState>(
        "SELECT * FROM plan_state WHERE project_id = ? AND phase_number = ? ORDER BY plan_number",
    )
    .bind(project_id)
    .bind(phase_number)
    .fetch_all(pool)
    .await
}

// --- Execution Runs ---

pub async fn insert_execution_run(
    pool: &SqlitePool,
    project_id: &str,
    phase_number: &str,
    plan_number: &str,
    run_number: i64,
    started_at: Option<&str>,
    status: Option<&str>,
) -> Result<ExecutionRun, sqlx::Error> {
    sqlx::query_as::<_, ExecutionRun>(
        "INSERT INTO execution_runs (project_id, phase_number, plan_number, run_number, started_at, status)
         VALUES (?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(project_id)
    .bind(phase_number)
    .bind(plan_number)
    .bind(run_number)
    .bind(started_at)
    .bind(status)
    .fetch_one(pool)
    .await
}

pub async fn update_execution_run(
    pool: &SqlitePool,
    id: i64,
    completed_at: Option<&str>,
    duration_minutes: Option<f64>,
    status: Option<&str>,
    superseded: Option<i64>,
    key_files_created: Option<&str>,
    key_files_modified: Option<&str>,
    requirements_completed: Option<&str>,
) -> Result<Option<ExecutionRun>, sqlx::Error> {
    sqlx::query_as::<_, ExecutionRun>(
        "UPDATE execution_runs SET
           completed_at = COALESCE(?, completed_at),
           duration_minutes = COALESCE(?, duration_minutes),
           status = COALESCE(?, status),
           superseded = COALESCE(?, superseded),
           key_files_created = COALESCE(?, key_files_created),
           key_files_modified = COALESCE(?, key_files_modified),
           requirements_completed = COALESCE(?, requirements_completed)
         WHERE id = ?
         RETURNING *",
    )
    .bind(completed_at)
    .bind(duration_minutes)
    .bind(status)
    .bind(superseded)
    .bind(key_files_created)
    .bind(key_files_modified)
    .bind(requirements_completed)
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn get_runs_for_plan(
    pool: &SqlitePool,
    project_id: &str,
    phase_number: &str,
    plan_number: &str,
) -> Result<Vec<ExecutionRun>, sqlx::Error> {
    sqlx::query_as::<_, ExecutionRun>(
        "SELECT * FROM execution_runs WHERE project_id = ? AND phase_number = ? AND plan_number = ? ORDER BY run_number",
    )
    .bind(project_id)
    .bind(phase_number)
    .bind(plan_number)
    .fetch_all(pool)
    .await
}

// --- Commits ---

pub async fn insert_commit(
    pool: &SqlitePool,
    execution_run_id: i64,
    task_number: i64,
    task_name: Option<&str>,
    commit_hash: Option<&str>,
    commit_type: Option<&str>,
) -> Result<Commit, sqlx::Error> {
    sqlx::query_as::<_, Commit>(
        "INSERT INTO commits (execution_run_id, task_number, task_name, commit_hash, commit_type)
         VALUES (?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(execution_run_id)
    .bind(task_number)
    .bind(task_name)
    .bind(commit_hash)
    .bind(commit_type)
    .fetch_one(pool)
    .await
}

pub async fn get_commits_for_run(
    pool: &SqlitePool,
    execution_run_id: i64,
) -> Result<Vec<Commit>, sqlx::Error> {
    sqlx::query_as::<_, Commit>(
        "SELECT * FROM commits WHERE execution_run_id = ? ORDER BY task_number",
    )
    .bind(execution_run_id)
    .fetch_all(pool)
    .await
}

// --- Agent Sessions ---

pub async fn insert_agent_session(
    pool: &SqlitePool,
    project_id: &str,
    agent_id: Option<&str>,
    agent_type: Option<&str>,
    phase_number: Option<&str>,
    plan_number: Option<&str>,
    started_at: Option<&str>,
    ended_at: Option<&str>,
) -> Result<AgentSession, sqlx::Error> {
    sqlx::query_as::<_, AgentSession>(
        "INSERT INTO agent_sessions (project_id, agent_id, agent_type, phase_number, plan_number, started_at, ended_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(project_id)
    .bind(agent_id)
    .bind(agent_type)
    .bind(phase_number)
    .bind(plan_number)
    .bind(started_at)
    .bind(ended_at)
    .fetch_one(pool)
    .await
}

pub async fn get_sessions_for_project(
    pool: &SqlitePool,
    project_id: &str,
) -> Result<Vec<AgentSession>, sqlx::Error> {
    sqlx::query_as::<_, AgentSession>(
        "SELECT * FROM agent_sessions WHERE project_id = ? ORDER BY started_at",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
}

// --- Verification Results ---

pub async fn upsert_verification(
    pool: &SqlitePool,
    project_id: &str,
    phase_number: &str,
    status: &str,
    score: Option<&str>,
    verified_at: Option<&str>,
) -> Result<VerificationResult, sqlx::Error> {
    sqlx::query_as::<_, VerificationResult>(
        "INSERT INTO verification_results (project_id, phase_number, status, score, verified_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(project_id, phase_number) DO UPDATE SET
           status = excluded.status,
           score = excluded.score,
           verified_at = excluded.verified_at
         RETURNING *",
    )
    .bind(project_id)
    .bind(phase_number)
    .bind(status)
    .bind(score)
    .bind(verified_at)
    .fetch_one(pool)
    .await
}

pub async fn get_verification_for_phase(
    pool: &SqlitePool,
    project_id: &str,
    phase_number: &str,
) -> Result<Option<VerificationResult>, sqlx::Error> {
    sqlx::query_as::<_, VerificationResult>(
        "SELECT * FROM verification_results WHERE project_id = ? AND phase_number = ?",
    )
    .bind(project_id)
    .bind(phase_number)
    .fetch_optional(pool)
    .await
}

// --- Parse Errors ---

pub async fn insert_parse_error(
    pool: &SqlitePool,
    project_id: &str,
    file_path: &str,
    error_message: &str,
    severity: &str,
) -> Result<ParseError, sqlx::Error> {
    sqlx::query_as::<_, ParseError>(
        "INSERT INTO parse_errors (project_id, file_path, error_message, severity)
         VALUES (?, ?, ?, ?)
         RETURNING *",
    )
    .bind(project_id)
    .bind(file_path)
    .bind(error_message)
    .bind(severity)
    .fetch_one(pool)
    .await
}

pub async fn resolve_parse_error(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE parse_errors SET resolved_at = datetime('now') WHERE id = ? AND resolved_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn get_active_errors_for_project(
    pool: &SqlitePool,
    project_id: &str,
) -> Result<Vec<ParseError>, sqlx::Error> {
    sqlx::query_as::<_, ParseError>(
        "SELECT * FROM parse_errors WHERE project_id = ? AND resolved_at IS NULL ORDER BY occurred_at DESC",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
}

// --- Project Config ---

pub async fn upsert_config(
    pool: &SqlitePool,
    project_id: &str,
    config_json: &str,
) -> Result<ProjectConfig, sqlx::Error> {
    sqlx::query_as::<_, ProjectConfig>(
        "INSERT INTO project_config (project_id, config_json)
         VALUES (?, ?)
         ON CONFLICT(project_id) DO UPDATE SET
           config_json = excluded.config_json,
           updated_at = datetime('now')
         RETURNING *",
    )
    .bind(project_id)
    .bind(config_json)
    .fetch_one(pool)
    .await
}

pub async fn get_config_for_project(
    pool: &SqlitePool,
    project_id: &str,
) -> Result<Option<ProjectConfig>, sqlx::Error> {
    sqlx::query_as::<_, ProjectConfig>(
        "SELECT * FROM project_config WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await
}
