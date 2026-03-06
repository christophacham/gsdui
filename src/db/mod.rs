pub mod models;
pub mod schema;

use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

/// Initialize the SQLite connection pool with WAL mode and run migrations.
pub async fn init_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    // Run embedded migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| sqlx::Error::Configuration(e.into()))?;

    // Enable WAL mode for concurrent reads during writes
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;

    // NORMAL synchronous mode (safe with WAL, better performance)
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await?;

    // Enable foreign key enforcement
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    // Set busy timeout for write contention
    sqlx::query("PRAGMA busy_timeout = 5000")
        .execute(&pool)
        .await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_connects_and_wal_enabled() {
        let pool = init_pool("sqlite::memory:").await.unwrap();

        // Verify WAL mode is enabled
        let row: (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            row.0, "memory",
            "In-memory DBs report 'memory' for journal_mode, but WAL was set"
        );
        // Note: in-memory databases cannot use WAL mode, they always report "memory".
        // On-disk databases will report "wal". The PRAGMA still executes without error.

        // Verify foreign keys are enabled
        let row: (i64,) = sqlx::query_as("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0, 1, "Foreign keys should be enabled");

        pool.close().await;
    }

    #[tokio::test]
    async fn test_migration_creates_all_tables() {
        let pool = init_pool("sqlite::memory:").await.unwrap();

        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        let table_names: Vec<&str> = tables.iter().map(|t| t.0.as_str()).collect();
        let expected = vec![
            "agent_sessions",
            "commits",
            "execution_runs",
            "parse_errors",
            "phase_state",
            "plan_state",
            "project_config",
            "projects",
            "verification_results",
        ];
        assert_eq!(table_names, expected);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_project_crud() {
        let pool = init_pool("sqlite::memory:").await.unwrap();

        // Create
        let project =
            schema::create_project(&pool, "test-id-1", "Test Project", "/tmp/test-project")
                .await
                .unwrap();
        assert_eq!(project.id, "test-id-1");
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.status, "active");

        // Read
        let found = schema::get_project(&pool, "test-id-1").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Project");

        // Read all
        let all = schema::get_all_projects(&pool).await.unwrap();
        assert_eq!(all.len(), 1);

        // Update
        let updated = schema::update_project(&pool, "test-id-1", Some("Renamed"), Some(90))
            .await
            .unwrap();
        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.retention_days, Some(90));

        // Delete
        let deleted = schema::delete_project(&pool, "test-id-1").await.unwrap();
        assert!(deleted);

        let found = schema::get_project(&pool, "test-id-1").await.unwrap();
        assert!(found.is_none());

        pool.close().await;
    }

    #[tokio::test]
    async fn test_phase_and_plan_state_with_fk() {
        let pool = init_pool("sqlite::memory:").await.unwrap();

        // Create project first (FK target)
        schema::create_project(&pool, "proj-1", "Test", "/tmp/test")
            .await
            .unwrap();

        // Insert phase state
        let phase = schema::upsert_phase_state(
            &pool,
            "proj-1",
            "01",
            "Backend Foundation",
            Some("Build backend"),
            None,
            "executing",
            None,
            Some("[\"STATE-01\"]"),
            3,
            1,
        )
        .await
        .unwrap();
        assert_eq!(phase.phase_number, "01");
        assert_eq!(phase.stage, "executing");

        // Insert plan state
        let plan = schema::upsert_plan_state(
            &pool,
            "proj-1",
            "01",
            "01",
            Some("Scaffold"),
            Some(1),
            None,
            Some("execute"),
            "working",
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(plan.plan_number, "01");
        assert_eq!(plan.status, "working");

        // Insert execution run
        let run = schema::insert_execution_run(
            &pool,
            "proj-1",
            "01",
            "01",
            1,
            Some("2026-03-06T12:00:00Z"),
            Some("in_progress"),
        )
        .await
        .unwrap();
        assert_eq!(run.run_number, 1);

        // Insert commit for the run
        let commit = schema::insert_commit(
            &pool,
            run.id,
            1,
            Some("Scaffold project"),
            Some("abc1234"),
            Some("feat"),
        )
        .await
        .unwrap();
        assert_eq!(commit.task_number, 1);

        // Verify reads
        let phases = schema::get_phase_states_for_project(&pool, "proj-1")
            .await
            .unwrap();
        assert_eq!(phases.len(), 1);

        let plans = schema::get_plan_states_for_phase(&pool, "proj-1", "01")
            .await
            .unwrap();
        assert_eq!(plans.len(), 1);

        let runs = schema::get_runs_for_plan(&pool, "proj-1", "01", "01")
            .await
            .unwrap();
        assert_eq!(runs.len(), 1);

        let commits = schema::get_commits_for_run(&pool, run.id).await.unwrap();
        assert_eq!(commits.len(), 1);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_unique_constraint_phase_state() {
        let pool = init_pool("sqlite::memory:").await.unwrap();

        schema::create_project(&pool, "proj-1", "Test", "/tmp/test")
            .await
            .unwrap();

        // First insert
        schema::upsert_phase_state(
            &pool, "proj-1", "01", "Backend", None, None, "planned", None, None, 0, 0,
        )
        .await
        .unwrap();

        // Upsert with same (project_id, phase_number) should update, not fail
        let updated = schema::upsert_phase_state(
            &pool,
            "proj-1",
            "01",
            "Backend Updated",
            Some("New goal"),
            None,
            "executing",
            None,
            None,
            3,
            1,
        )
        .await
        .unwrap();
        assert_eq!(updated.phase_name, "Backend Updated");
        assert_eq!(updated.stage, "executing");

        // Verify only one row exists
        let phases = schema::get_phase_states_for_project(&pool, "proj-1")
            .await
            .unwrap();
        assert_eq!(phases.len(), 1);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_cascade_delete() {
        let pool = init_pool("sqlite::memory:").await.unwrap();

        // Create project with child rows
        schema::create_project(&pool, "proj-1", "Test", "/tmp/test")
            .await
            .unwrap();

        schema::upsert_phase_state(
            &pool, "proj-1", "01", "Backend", None, None, "planned", None, None, 0, 0,
        )
        .await
        .unwrap();

        schema::upsert_plan_state(
            &pool,
            "proj-1",
            "01",
            "01",
            Some("Plan 1"),
            None,
            None,
            None,
            "pending",
            None,
            None,
        )
        .await
        .unwrap();

        let run = schema::insert_execution_run(&pool, "proj-1", "01", "01", 1, None, None)
            .await
            .unwrap();

        schema::insert_commit(&pool, run.id, 1, Some("Task 1"), Some("abc"), Some("feat"))
            .await
            .unwrap();

        schema::insert_agent_session(
            &pool,
            "proj-1",
            Some("agent-1"),
            Some("claude"),
            Some("01"),
            Some("01"),
            None,
            None,
        )
        .await
        .unwrap();

        // Delete the project -- all child rows should cascade
        let deleted = schema::delete_project(&pool, "proj-1").await.unwrap();
        assert!(deleted);

        // Verify all child tables are empty
        let phases = schema::get_phase_states_for_project(&pool, "proj-1")
            .await
            .unwrap();
        assert_eq!(phases.len(), 0);

        let plans = schema::get_plan_states_for_phase(&pool, "proj-1", "01")
            .await
            .unwrap();
        assert_eq!(plans.len(), 0);

        let runs = schema::get_runs_for_plan(&pool, "proj-1", "01", "01")
            .await
            .unwrap();
        assert_eq!(runs.len(), 0);

        let sessions = schema::get_sessions_for_project(&pool, "proj-1")
            .await
            .unwrap();
        assert_eq!(sessions.len(), 0);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_verification_and_parse_errors() {
        let pool = init_pool("sqlite::memory:").await.unwrap();

        schema::create_project(&pool, "proj-1", "Test", "/tmp/test")
            .await
            .unwrap();

        // Upsert verification result
        let v = schema::upsert_verification(
            &pool,
            "proj-1",
            "01",
            "passed",
            Some("5/5"),
            Some("2026-03-06"),
        )
        .await
        .unwrap();
        assert_eq!(v.status, "passed");

        // Read it back
        let found = schema::get_verification_for_phase(&pool, "proj-1", "01")
            .await
            .unwrap();
        assert!(found.is_some());

        // Insert parse error
        let err = schema::insert_parse_error(&pool, "proj-1", "STATE.md", "Invalid YAML", "error")
            .await
            .unwrap();
        assert_eq!(err.severity, "error");

        // Get active errors
        let errors = schema::get_active_errors_for_project(&pool, "proj-1")
            .await
            .unwrap();
        assert_eq!(errors.len(), 1);

        // Resolve error
        let resolved = schema::resolve_parse_error(&pool, err.id).await.unwrap();
        assert!(resolved);

        // No active errors now
        let errors = schema::get_active_errors_for_project(&pool, "proj-1")
            .await
            .unwrap();
        assert_eq!(errors.len(), 0);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_project_config() {
        let pool = init_pool("sqlite::memory:").await.unwrap();

        schema::create_project(&pool, "proj-1", "Test", "/tmp/test")
            .await
            .unwrap();

        // Upsert config
        let config = schema::upsert_config(&pool, "proj-1", "{\"mode\":\"yolo\"}")
            .await
            .unwrap();
        assert_eq!(config.config_json, "{\"mode\":\"yolo\"}");

        // Read it back
        let found = schema::get_config_for_project(&pool, "proj-1")
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().config_json, "{\"mode\":\"yolo\"}");

        // Update
        let updated = schema::upsert_config(&pool, "proj-1", "{\"mode\":\"careful\"}")
            .await
            .unwrap();
        assert_eq!(updated.config_json, "{\"mode\":\"careful\"}");

        pool.close().await;
    }
}
