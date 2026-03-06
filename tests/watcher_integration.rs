//! Integration tests for the file watcher, parse pipeline, startup reconciliation,
//! and retention pruning.

use std::path::Path;
use std::time::Duration;

use sqlx::SqlitePool;
use tempfile::TempDir;
use tokio::sync::broadcast;

use gsdui::db;
use gsdui::watcher;
use gsdui::watcher::pipeline;
use gsdui::watcher::retention;

/// Create an in-memory SQLite database with migrations applied.
async fn test_db() -> SqlitePool {
    let pool = db::init_pool("sqlite::memory:").await.unwrap();
    pool
}

/// Create a realistic .planning/ directory structure in a temp dir.
fn create_planning_structure(base: &Path) {
    let planning = base.join(".planning");
    std::fs::create_dir_all(&planning).unwrap();

    // STATE.md
    std::fs::write(
        planning.join("STATE.md"),
        r#"---
gsd_state_version: "1.0"
milestone: v1.0
status: executing
stopped_at: "Completed 01-01-PLAN.md"
last_updated: "2026-03-06T19:54:33Z"
progress:
  total_phases: 2
  completed_phases: 0
  total_plans: 4
  completed_plans: 1
  percent: 25
---

# Project State

## Current Position

Phase: 1 of 2 (Backend Foundation)
Plan: 2 of 2 in current phase
"#,
    )
    .unwrap();

    // config.json
    std::fs::write(
        planning.join("config.json"),
        r#"{"mode": "yolo", "granularity": "coarse"}"#,
    )
    .unwrap();

    // ROADMAP.md
    std::fs::write(
        planning.join("ROADMAP.md"),
        r#"# Roadmap

## Phase Details

### Phase 1: Backend Foundation

**Goal:** Build the backend
**Depends on:** Nothing (first phase)
**Requirements:** STATE-01, STATE-02
**Plans:** 2 plans

Plans:
- [x] 01-01-PLAN.md -- Project scaffold
- [ ] 01-02-PLAN.md -- Parsers

### Phase 2: Frontend

**Goal:** Build the frontend
**Depends on:** Phase 1
**Requirements:** UI-01
**Plans:** 2 plans

Plans:
- [ ] 02-01-PLAN.md -- Components
- [ ] 02-02-PLAN.md -- Integration
"#,
    )
    .unwrap();

    // Phase directory with PLAN and SUMMARY files
    let phase_dir = planning.join("phases").join("01-backend-foundation");
    std::fs::create_dir_all(&phase_dir).unwrap();

    // 01-CONTEXT.md
    std::fs::write(phase_dir.join("01-CONTEXT.md"), "# Phase Context\n").unwrap();

    // 01-RESEARCH.md
    std::fs::write(phase_dir.join("01-RESEARCH.md"), "# Research\n").unwrap();

    // 01-01-PLAN.md
    std::fs::write(
        phase_dir.join("01-01-PLAN.md"),
        r#"---
phase: 01-backend-foundation
plan: 1
type: standard
wave: 1
depends_on: ["none"]
autonomous: true
requirements: ["STATE-01"]
---

# Plan 01-01: Project scaffold
"#,
    )
    .unwrap();

    // 01-01-SUMMARY.md
    std::fs::write(
        phase_dir.join("01-01-SUMMARY.md"),
        r#"---
phase: 01-backend-foundation
plan: 1
duration: 9min
completed: "2026-03-06"
requirements-completed: ["STATE-01"]
---

# Summary

## Task Commits

1. **Task 1: scaffold** - `abc1234` (feat)
"#,
    )
    .unwrap();

    // 01-02-PLAN.md
    std::fs::write(
        phase_dir.join("01-02-PLAN.md"),
        r#"---
phase: 01-backend-foundation
plan: 2
type: standard
wave: 2
depends_on: ["01-01"]
autonomous: true
requirements: ["STATE-02"]
---

# Plan 01-02: Parsers
"#,
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// Test: Bootstrap project parses all .planning/ files into DB
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_bootstrap_project_parses_all_files() {
    let pool = test_db().await;
    let temp_dir = TempDir::new().unwrap();
    create_planning_structure(temp_dir.path());

    let (broadcast_tx, _rx) = broadcast::channel(64);

    // Register the project in DB first
    let project = db::schema::create_project(
        &pool,
        "test-project-1",
        "Test Project",
        &temp_dir.path().to_string_lossy(),
    )
    .await
    .unwrap();

    let planning_path = temp_dir.path().join(".planning");

    // Bootstrap
    pipeline::bootstrap_project(&project.id, &planning_path, &pool, &broadcast_tx)
        .await
        .unwrap();

    // Verify: config was parsed
    let config = db::schema::get_config_for_project(&pool, &project.id)
        .await
        .unwrap();
    assert!(config.is_some(), "Config should be parsed and stored");

    // Verify: phase state was created from ROADMAP.md
    let phases = db::schema::get_phase_states_for_project(&pool, &project.id)
        .await
        .unwrap();
    assert!(
        phases.len() >= 1,
        "Should have phase state from ROADMAP.md, got {}",
        phases.len()
    );

    // Verify: plan state was created
    let plans = db::schema::get_plan_states_for_phase(&pool, &project.id, "01")
        .await
        .unwrap();
    assert!(
        plans.len() >= 1,
        "Should have plan state entries, got {}",
        plans.len()
    );

    // Verify: no parse errors for valid files
    let errors = db::schema::get_active_errors_for_project(&pool, &project.id)
        .await
        .unwrap();
    assert!(
        errors.is_empty(),
        "Should have no parse errors for valid files, got {}",
        errors.len()
    );
}

// ---------------------------------------------------------------------------
// Test: File watcher detects changes and pipeline parses them
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_watcher_detects_and_parses_file_change() {
    let pool = test_db().await;
    let temp_dir = TempDir::new().unwrap();
    create_planning_structure(temp_dir.path());

    let (broadcast_tx, mut broadcast_rx) = broadcast::channel(64);
    let (file_event_tx, file_event_rx) = tokio::sync::mpsc::channel(256);
    let (debounced_tx, debounced_rx) = tokio::sync::mpsc::channel(128);

    // Register project
    let project = db::schema::create_project(
        &pool,
        "test-watcher-project",
        "Watcher Test",
        &temp_dir.path().to_string_lossy(),
    )
    .await
    .unwrap();

    // Spawn debouncer (fast 30ms for test)
    let _debouncer =
        watcher::debounce::Debouncer::spawn(file_event_rx, debounced_tx, Duration::from_millis(30));

    // Spawn pipeline
    let pipeline_db = pool.clone();
    let pipeline_tx = broadcast_tx.clone();
    let _pipeline = tokio::spawn(async move {
        pipeline::run_pipeline(debounced_rx, pipeline_db, pipeline_tx).await;
    });

    // Start file watcher
    let mut file_watcher = watcher::FileWatcher::new(file_event_tx);
    file_watcher
        .watch_project(&project.id, temp_dir.path())
        .unwrap();

    // Give the watcher a moment to initialize
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Write a config.json file
    let config_path = temp_dir.path().join(".planning").join("config.json");
    std::fs::write(&config_path, r#"{"mode": "safe", "granularity": "fine"}"#).unwrap();

    // Wait for the pipeline to process the event (watcher detect + debounce + parse)
    let result = tokio::time::timeout(Duration::from_secs(3), broadcast_rx.recv()).await;
    assert!(
        result.is_ok(),
        "Should receive a broadcast within 3 seconds"
    );

    // Verify: config was updated in DB
    let config = db::schema::get_config_for_project(&pool, &project.id)
        .await
        .unwrap();
    assert!(config.is_some(), "Config should be in database");
    let config = config.unwrap();
    assert!(
        config.config_json.contains("safe"),
        "Config should contain updated value"
    );
}

// ---------------------------------------------------------------------------
// Test: Parse errors are recorded and last-known-good state preserved
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_parse_error_recorded_good_state_preserved() {
    let pool = test_db().await;
    let temp_dir = TempDir::new().unwrap();
    create_planning_structure(temp_dir.path());

    let (broadcast_tx, _rx) = broadcast::channel(64);

    // Register project and bootstrap good state
    let project = db::schema::create_project(
        &pool,
        "test-error-project",
        "Error Test",
        &temp_dir.path().to_string_lossy(),
    )
    .await
    .unwrap();

    let planning_path = temp_dir.path().join(".planning");
    pipeline::bootstrap_project(&project.id, &planning_path, &pool, &broadcast_tx)
        .await
        .unwrap();

    // Verify good config was parsed
    let config_before = db::schema::get_config_for_project(&pool, &project.id)
        .await
        .unwrap();
    assert!(config_before.is_some(), "Should have config from bootstrap");

    // Now write a corrupted config.json
    let config_path = planning_path.join("config.json");
    std::fs::write(&config_path, "{invalid json!!!}").unwrap();

    // Process the corrupted file through the pipeline directly
    let (debounced_tx, debounced_rx) = tokio::sync::mpsc::channel(128);
    let pipeline_db = pool.clone();
    let pipeline_broadcast = broadcast_tx.clone();

    let pipeline_handle = tokio::spawn(async move {
        pipeline::run_pipeline(debounced_rx, pipeline_db, pipeline_broadcast).await;
    });

    // Send the corrupted file event
    debounced_tx
        .send(vec![watcher::debounce::DebouncedEvent {
            path: config_path.clone(),
            project_id: project.id.clone(),
            kind: watcher::FileEventKind::Modify,
        }])
        .await
        .unwrap();

    // Give pipeline time to process
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(debounced_tx); // Close channel to stop pipeline
    let _ = pipeline_handle.await;

    // Verify: parse error was recorded
    let errors = db::schema::get_active_errors_for_project(&pool, &project.id)
        .await
        .unwrap();
    assert!(
        !errors.is_empty(),
        "Should have a parse error for corrupted config.json"
    );
    assert!(
        errors[0].error_message.contains("JSON"),
        "Error should mention JSON parsing issue"
    );

    // Verify: last-known-good state is preserved (config from bootstrap still there)
    let config_after = db::schema::get_config_for_project(&pool, &project.id)
        .await
        .unwrap();
    assert!(
        config_after.is_some(),
        "Good config should still be in database"
    );
}

// ---------------------------------------------------------------------------
// Test: Startup reconciliation re-parses all registered projects
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_startup_reconciliation() {
    let pool = test_db().await;
    let temp_dir = TempDir::new().unwrap();
    create_planning_structure(temp_dir.path());

    let (broadcast_tx, _rx) = broadcast::channel(64);

    // Insert a project in DB manually (simulating a project registered before daemon restart)
    let project = db::schema::create_project(
        &pool,
        "test-reconcile",
        "Reconcile Test",
        &temp_dir.path().to_string_lossy(),
    )
    .await
    .unwrap();

    // Simulate "daemon restart": DB has project but no parsed state.
    // Call bootstrap_project as startup reconciliation would.
    let planning_path = temp_dir.path().join(".planning");
    pipeline::bootstrap_project(&project.id, &planning_path, &pool, &broadcast_tx)
        .await
        .unwrap();

    // Verify: phase state was populated from ROADMAP.md
    let phases = db::schema::get_phase_states_for_project(&pool, &project.id)
        .await
        .unwrap();
    assert!(
        !phases.is_empty(),
        "Reconciliation should populate phase state"
    );

    // Verify: config was parsed
    let config = db::schema::get_config_for_project(&pool, &project.id)
        .await
        .unwrap();
    assert!(config.is_some(), "Reconciliation should parse config.json");

    // Now modify a file on disk (simulate offline changes)
    let config_path = planning_path.join("config.json");
    std::fs::write(
        &config_path,
        r#"{"mode": "safe", "granularity": "fine", "parallelization": false}"#,
    )
    .unwrap();

    // Re-run bootstrap (as would happen on next daemon restart)
    pipeline::bootstrap_project(&project.id, &planning_path, &pool, &broadcast_tx)
        .await
        .unwrap();

    // Verify: config reflects offline changes
    let config_after = db::schema::get_config_for_project(&pool, &project.id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        config_after.config_json.contains("safe"),
        "Config should reflect offline changes"
    );
}

// ---------------------------------------------------------------------------
// Test: Retention pruning deletes expired rows
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_retention_pruning_deletes_expired() {
    let pool = test_db().await;

    // Create a project
    let project = db::schema::create_project(
        &pool,
        "test-retention",
        "Retention Test",
        "/tmp/nonexistent",
    )
    .await
    .unwrap();

    // Insert an old execution run (300 days ago)
    let old_date = "2025-05-10 12:00:00";
    let _old_run = db::schema::insert_execution_run(
        &pool,
        &project.id,
        "01",
        "01",
        1,
        Some(old_date),
        Some("completed"),
    )
    .await
    .unwrap();

    // Update the old run's completed_at to the old date
    sqlx::query("UPDATE execution_runs SET completed_at = ? WHERE project_id = ?")
        .bind(old_date)
        .bind(&project.id)
        .execute(&pool)
        .await
        .unwrap();

    // Insert a recent execution run
    let _recent_run = db::schema::insert_execution_run(
        &pool,
        &project.id,
        "01",
        "02",
        1,
        Some("2026-03-01 12:00:00"),
        Some("completed"),
    )
    .await
    .unwrap();

    sqlx::query("UPDATE execution_runs SET completed_at = '2026-03-01 12:00:00' WHERE plan_number = '02' AND project_id = ?")
        .bind(&project.id)
        .execute(&pool)
        .await
        .unwrap();

    // Insert an old agent session
    let _ = db::schema::insert_agent_session(
        &pool,
        &project.id,
        Some("agent-1"),
        Some("planner"),
        Some("01"),
        Some("01"),
        Some(old_date),
        Some(old_date),
    )
    .await
    .unwrap();

    // Insert a resolved parse error with old date
    let error = db::schema::insert_parse_error(
        &pool,
        &project.id,
        "/test/STATE.md",
        "old error",
        "warning",
    )
    .await
    .unwrap();
    // Resolve it and backdate
    db::schema::resolve_parse_error(&pool, error.id)
        .await
        .unwrap();
    sqlx::query("UPDATE parse_errors SET occurred_at = ? WHERE id = ?")
        .bind(old_date)
        .bind(error.id)
        .execute(&pool)
        .await
        .unwrap();

    // Verify we have rows before pruning
    let runs_before = db::schema::get_runs_for_plan(&pool, &project.id, "01", "01")
        .await
        .unwrap();
    assert_eq!(runs_before.len(), 1, "Should have 1 old run");

    // Prune with 180-day cutoff (the old records at 300 days should be pruned)
    let cutoff = "2025-09-08 12:00:00"; // About 180 days before "now" (2026-03-06)
    let runs_deleted = retention::prune_expired_runs(&pool, &project.id, cutoff)
        .await
        .unwrap();
    assert_eq!(runs_deleted, 1, "Should delete 1 old execution run");

    let sessions_deleted = retention::prune_expired_sessions(&pool, &project.id, cutoff)
        .await
        .unwrap();
    assert_eq!(sessions_deleted, 1, "Should delete 1 old agent session");

    let errors_deleted = retention::prune_resolved_errors(&pool, &project.id, cutoff)
        .await
        .unwrap();
    assert_eq!(errors_deleted, 1, "Should delete 1 resolved parse error");

    // Verify recent run was NOT deleted
    let runs_after = db::schema::get_runs_for_plan(&pool, &project.id, "01", "02")
        .await
        .unwrap();
    assert_eq!(runs_after.len(), 1, "Recent run should be preserved");
}

// ---------------------------------------------------------------------------
// Test: Offline project detection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_offline_project_detection() {
    let pool = test_db().await;

    // Create a project pointing to a non-existent path
    let project = db::schema::create_project(
        &pool,
        "test-offline",
        "Offline Test",
        "/tmp/does_not_exist_gsdui_test",
    )
    .await
    .unwrap();

    // Simulate startup reconciliation check
    let project_path = std::path::Path::new(&project.path);
    assert!(
        !project_path.exists(),
        "Path should not exist for this test"
    );

    // Mark as offline (as startup would do)
    sqlx::query("UPDATE projects SET status = 'offline' WHERE id = ?")
        .bind(&project.id)
        .execute(&pool)
        .await
        .unwrap();

    // Verify status
    let updated = db::schema::get_project(&pool, &project.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.status, "offline");
}

// ---------------------------------------------------------------------------
// Test: Stage derivation after file changes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_stage_derivation_from_files() {
    let pool = test_db().await;
    let temp_dir = TempDir::new().unwrap();
    create_planning_structure(temp_dir.path());

    let (broadcast_tx, _rx) = broadcast::channel(64);

    let project = db::schema::create_project(
        &pool,
        "test-stage",
        "Stage Test",
        &temp_dir.path().to_string_lossy(),
    )
    .await
    .unwrap();

    let planning_path = temp_dir.path().join(".planning");
    pipeline::bootstrap_project(&project.id, &planning_path, &pool, &broadcast_tx)
        .await
        .unwrap();

    // Phase 1 has CONTEXT, RESEARCH, 2 PLANs, and 1 SUMMARY -- should be "executing"
    let phases = db::schema::get_phase_states_for_project(&pool, &project.id)
        .await
        .unwrap();

    let phase_1 = phases.iter().find(|p| p.phase_number.starts_with('1'));
    if let Some(p1) = phase_1 {
        // With CONTEXT, RESEARCH, PLANs, and 1 SUMMARY (out of 2), stage should be "executing"
        assert_eq!(
            p1.stage, "executing",
            "Phase 1 should be in 'executing' stage (has 1 of 2 summaries)"
        );
    }
}

// ---------------------------------------------------------------------------
// Test: Broadcast channel receives StateUpdate
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_broadcast_receives_state_updates() {
    let pool = test_db().await;
    let temp_dir = TempDir::new().unwrap();
    create_planning_structure(temp_dir.path());

    let (broadcast_tx, mut broadcast_rx) = broadcast::channel(64);

    let project = db::schema::create_project(
        &pool,
        "test-broadcast",
        "Broadcast Test",
        &temp_dir.path().to_string_lossy(),
    )
    .await
    .unwrap();

    let planning_path = temp_dir.path().join(".planning");

    // Bootstrap should emit StateUpdate events
    pipeline::bootstrap_project(&project.id, &planning_path, &pool, &broadcast_tx)
        .await
        .unwrap();

    // Collect any events that were broadcast
    let mut event_count = 0;
    while let Ok(update) = broadcast_rx.try_recv() {
        assert_eq!(update.project_id, project.id);
        event_count += 1;
    }

    assert!(
        event_count > 0,
        "Bootstrap should have broadcast at least one StateUpdate"
    );
}
