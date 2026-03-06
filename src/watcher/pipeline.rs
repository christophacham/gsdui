use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};

use super::FileEventKind;
use super::debounce::DebouncedEvent;
use crate::db;
use crate::parser;

// ---------------------------------------------------------------------------
// StateUpdate types (for downstream WebSocket broadcast in Plan 01-04)
// ---------------------------------------------------------------------------

/// A state change event broadcast to WebSocket clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateUpdate {
    /// The project this change belongs to
    pub project_id: String,
    /// The specific change that occurred
    pub change: StateChange,
}

/// Specific kinds of state changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum StateChange {
    PhaseUpdated {
        phase_number: String,
        stage: String,
    },
    PlanUpdated {
        phase_number: String,
        plan_number: String,
        status: String,
    },
    VerificationUpdated {
        phase_number: String,
        status: String,
    },
    ConfigUpdated,
    AgentHistoryUpdated {
        session_count: usize,
    },
    ProjectStateUpdated {
        status: Option<String>,
    },
    ParseError {
        file_path: String,
        error: String,
    },
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Receive batched debounced events and route them to the correct parser,
/// then persist results to the database and broadcast state updates.
pub async fn run_pipeline(
    mut event_rx: mpsc::Receiver<Vec<DebouncedEvent>>,
    db: SqlitePool,
    broadcast_tx: broadcast::Sender<StateUpdate>,
) {
    info!("Parse pipeline started");

    while let Some(batch) = event_rx.recv().await {
        for event in batch {
            debug!(
                path = %event.path.display(),
                kind = ?event.kind,
                project_id = %event.project_id,
                "Processing file event"
            );

            if event.kind == FileEventKind::Remove {
                // For removals, we don't re-parse -- we could mark data stale,
                // but per CONTEXT.md we keep last-known-good. Log and skip.
                debug!(path = %event.path.display(), "File removed, preserving last-known-good state");
                continue;
            }

            process_file_event(&event.project_id, &event.path, &db, &broadcast_tx).await;
        }
    }

    info!("Parse pipeline shutting down (event channel closed)");
}

/// Process a single file event: read, parse, persist, broadcast.
async fn process_file_event(
    project_id: &str,
    path: &Path,
    db: &SqlitePool,
    broadcast_tx: &broadcast::Sender<StateUpdate>,
) {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name.to_string(),
        None => {
            warn!(path = %path.display(), "Cannot extract filename from path");
            return;
        }
    };

    // Read file content
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(e) => {
            warn!(path = %path.display(), error = %e, "Failed to read file");
            // Record parse error for unreadable files
            let _ = db::schema::insert_parse_error(
                db,
                project_id,
                &path.to_string_lossy(),
                &format!("Failed to read file: {}", e),
                "error",
            )
            .await;
            let _ = broadcast_tx.send(StateUpdate {
                project_id: project_id.to_string(),
                change: StateChange::ParseError {
                    file_path: path.to_string_lossy().to_string(),
                    error: format!("Failed to read file: {}", e),
                },
            });
            return;
        }
    };

    // Route to the appropriate parser based on filename
    match classify_file(&file_name) {
        FileType::StateMd => {
            handle_state_md(project_id, path, &content, db, broadcast_tx).await;
        }
        FileType::RoadmapMd => {
            handle_roadmap(project_id, path, &content, db, broadcast_tx).await;
        }
        FileType::PlanMd {
            phase_number,
            plan_number,
        } => {
            handle_plan_md(
                project_id,
                path,
                &content,
                &phase_number,
                &plan_number,
                db,
                broadcast_tx,
            )
            .await;
        }
        FileType::SummaryMd {
            phase_number,
            plan_number,
        } => {
            handle_summary_md(
                project_id,
                path,
                &content,
                &phase_number,
                &plan_number,
                db,
                broadcast_tx,
            )
            .await;
        }
        FileType::VerificationMd { phase_number } => {
            handle_verification_md(project_id, path, &content, &phase_number, db, broadcast_tx)
                .await;
        }
        FileType::ContextOrResearch { phase_number } => {
            // Presence matters for stage derivation, but no content parsing needed
            update_phase_stage(project_id, &phase_number, path, db, broadcast_tx).await;
        }
        FileType::ConfigJson => {
            handle_config_json(project_id, path, &content, db, broadcast_tx).await;
        }
        FileType::AgentHistory => {
            handle_agent_history(project_id, path, &content, db, broadcast_tx).await;
        }
        FileType::Unknown => {
            debug!(file_name = %file_name, "Ignoring unrecognized file type");
        }
    }
}

// ---------------------------------------------------------------------------
// File type classification
// ---------------------------------------------------------------------------

enum FileType {
    StateMd,
    RoadmapMd,
    PlanMd {
        phase_number: String,
        plan_number: String,
    },
    SummaryMd {
        phase_number: String,
        plan_number: String,
    },
    VerificationMd {
        phase_number: String,
    },
    ContextOrResearch {
        phase_number: String,
    },
    ConfigJson,
    AgentHistory,
    Unknown,
}

fn classify_file(file_name: &str) -> FileType {
    if file_name == "STATE.md" {
        return FileType::StateMd;
    }
    if file_name == "ROADMAP.md" {
        return FileType::RoadmapMd;
    }
    if file_name == "config.json" {
        return FileType::ConfigJson;
    }
    if file_name == "agent-history.json" {
        return FileType::AgentHistory;
    }

    // Pattern: {NN}-{NN}-PLAN.md  (e.g., 01-01-PLAN.md)
    let plan_re = Regex::new(r"^(\d+)-(\d+)-PLAN\.md$").unwrap();
    if let Some(caps) = plan_re.captures(file_name) {
        return FileType::PlanMd {
            phase_number: caps[1].to_string(),
            plan_number: caps[2].to_string(),
        };
    }

    // Pattern: {NN}-{NN}-SUMMARY.md
    let summary_re = Regex::new(r"^(\d+)-(\d+)-SUMMARY\.md$").unwrap();
    if let Some(caps) = summary_re.captures(file_name) {
        return FileType::SummaryMd {
            phase_number: caps[1].to_string(),
            plan_number: caps[2].to_string(),
        };
    }

    // Pattern: {NN}-VERIFICATION.md
    let verification_re = Regex::new(r"^(\d+)-VERIFICATION\.md$").unwrap();
    if let Some(caps) = verification_re.captures(file_name) {
        return FileType::VerificationMd {
            phase_number: caps[1].to_string(),
        };
    }

    // Pattern: {NN}-CONTEXT.md or {NN}-RESEARCH.md
    let context_re = Regex::new(r"^(\d+)-(?:CONTEXT|RESEARCH)\.md$").unwrap();
    if let Some(caps) = context_re.captures(file_name) {
        return FileType::ContextOrResearch {
            phase_number: caps[1].to_string(),
        };
    }

    FileType::Unknown
}

// ---------------------------------------------------------------------------
// Per-file-type handlers
// ---------------------------------------------------------------------------

async fn handle_state_md(
    project_id: &str,
    path: &Path,
    content: &str,
    db: &SqlitePool,
    broadcast_tx: &broadcast::Sender<StateUpdate>,
) {
    match parser::state_md::parse_state_md(content) {
        Ok(data) => {
            // Update project state (status from STATE.md)
            let _ = db::schema::update_project(db, project_id, None, None).await;

            let _ = broadcast_tx.send(StateUpdate {
                project_id: project_id.to_string(),
                change: StateChange::ProjectStateUpdated {
                    status: data.status.clone(),
                },
            });
            debug!(project_id, "Parsed STATE.md successfully");
        }
        Err(e) => {
            record_parse_error(project_id, path, &e.to_string(), db, broadcast_tx).await;
        }
    }
}

async fn handle_roadmap(
    project_id: &str,
    path: &Path,
    content: &str,
    db: &SqlitePool,
    broadcast_tx: &broadcast::Sender<StateUpdate>,
) {
    match parser::roadmap::parse_roadmap(content) {
        Ok(data) => {
            for phase in &data.phases {
                let phase_num = normalize_phase_number(&phase.number);
                let depends_on = if phase.depends_on.is_empty() {
                    None
                } else {
                    Some(phase.depends_on.join(", "))
                };
                let requirements = if phase.requirements.is_empty() {
                    None
                } else {
                    Some(phase.requirements.join(", "))
                };

                let completed_plans = phase.plans.iter().filter(|p| p.completed).count() as i64;

                if let Err(e) = db::schema::upsert_phase_state(
                    db,
                    project_id,
                    &phase_num,
                    &phase.name,
                    phase.goal.as_deref(),
                    depends_on.as_deref(),
                    "planned", // Stage will be re-derived from files
                    None,
                    requirements.as_deref(),
                    phase.plan_count.unwrap_or(0),
                    completed_plans,
                )
                .await
                {
                    error!(error = %e, phase = %phase.number, "Failed to upsert phase state from ROADMAP");
                }

                let _ = broadcast_tx.send(StateUpdate {
                    project_id: project_id.to_string(),
                    change: StateChange::PhaseUpdated {
                        phase_number: phase_num.clone(),
                        stage: "planned".to_string(),
                    },
                });
            }
            debug!(
                project_id,
                phases = data.phases.len(),
                "Parsed ROADMAP.md successfully"
            );
        }
        Err(e) => {
            record_parse_error(project_id, path, &e.to_string(), db, broadcast_tx).await;
        }
    }
}

async fn handle_plan_md(
    project_id: &str,
    path: &Path,
    content: &str,
    phase_number: &str,
    plan_number: &str,
    db: &SqlitePool,
    broadcast_tx: &broadcast::Sender<StateUpdate>,
) {
    match parser::plan::parse_plan_md(content) {
        Ok(data) => {
            let depends_on = if data.depends_on.is_empty() {
                None
            } else {
                Some(data.depends_on.join(", "))
            };
            let requirements = if data.requirements.is_empty() {
                None
            } else {
                Some(data.requirements.join(", "))
            };
            let files_modified = if data.files_modified.is_empty() {
                None
            } else {
                Some(data.files_modified.join(", "))
            };

            if let Err(e) = db::schema::upsert_plan_state(
                db,
                project_id,
                phase_number,
                plan_number,
                None, // plan_name not in PLAN.md frontmatter in a simple way
                data.wave,
                depends_on.as_deref(),
                data.plan_type.as_deref(),
                "pending",
                requirements.as_deref(),
                files_modified.as_deref(),
            )
            .await
            {
                error!(error = %e, "Failed to upsert plan state");
            }

            let _ = broadcast_tx.send(StateUpdate {
                project_id: project_id.to_string(),
                change: StateChange::PlanUpdated {
                    phase_number: phase_number.to_string(),
                    plan_number: plan_number.to_string(),
                    status: "pending".to_string(),
                },
            });

            // Trigger stage re-derivation for the phase
            update_phase_stage(project_id, phase_number, path, db, broadcast_tx).await;

            debug!(
                project_id,
                phase_number, plan_number, "Parsed PLAN.md successfully"
            );
        }
        Err(e) => {
            record_parse_error(project_id, path, &e.to_string(), db, broadcast_tx).await;
        }
    }
}

async fn handle_summary_md(
    project_id: &str,
    path: &Path,
    content: &str,
    phase_number: &str,
    plan_number: &str,
    db: &SqlitePool,
    broadcast_tx: &broadcast::Sender<StateUpdate>,
) {
    match parser::summary::parse_summary_md(content) {
        Ok(data) => {
            // Update plan status to "done" since SUMMARY exists
            let _ = db::schema::upsert_plan_state(
                db,
                project_id,
                phase_number,
                plan_number,
                None,
                None,
                None,
                None,
                "done",
                None,
                None,
            )
            .await;

            // Create/update execution run
            let run_number = 1i64; // Default to run 1; subsequent re-parses will find existing
            if let Ok(run) = db::schema::insert_execution_run(
                db,
                project_id,
                phase_number,
                plan_number,
                run_number,
                None, // started_at could be extracted from SUMMARY
                Some("completed"),
            )
            .await
            {
                // Insert commit records
                for commit_rec in &data.commits {
                    let _ = db::schema::insert_commit(
                        db,
                        run.id,
                        commit_rec.task_number,
                        Some(&commit_rec.task_name),
                        commit_rec.commit_hash.as_deref(),
                        commit_rec.commit_type.as_deref(),
                    )
                    .await;
                }

                // Update run with duration and key files
                let key_files = data.key_files.join(", ");
                let reqs = data.requirements_completed.join(", ");
                let _ = db::schema::update_execution_run(
                    db,
                    run.id,
                    data.completed.as_deref(),
                    None, // duration in minutes (could parse from "Xmin")
                    Some("completed"),
                    None,
                    None,
                    if key_files.is_empty() {
                        None
                    } else {
                        Some(&key_files)
                    },
                    if reqs.is_empty() { None } else { Some(&reqs) },
                )
                .await;
            }

            let _ = broadcast_tx.send(StateUpdate {
                project_id: project_id.to_string(),
                change: StateChange::PlanUpdated {
                    phase_number: phase_number.to_string(),
                    plan_number: plan_number.to_string(),
                    status: "done".to_string(),
                },
            });

            // Trigger stage re-derivation
            update_phase_stage(project_id, phase_number, path, db, broadcast_tx).await;

            debug!(
                project_id,
                phase_number, plan_number, "Parsed SUMMARY.md successfully"
            );
        }
        Err(e) => {
            record_parse_error(project_id, path, &e.to_string(), db, broadcast_tx).await;
        }
    }
}

async fn handle_verification_md(
    project_id: &str,
    path: &Path,
    content: &str,
    phase_number: &str,
    db: &SqlitePool,
    broadcast_tx: &broadcast::Sender<StateUpdate>,
) {
    match parser::verification::parse_verification_md(content) {
        Ok(data) => {
            let status_str = data.status.to_string();
            if let Err(e) = db::schema::upsert_verification(
                db,
                project_id,
                phase_number,
                &status_str,
                data.score.as_deref(),
                data.verified_at.as_deref(),
            )
            .await
            {
                error!(error = %e, "Failed to upsert verification");
            }

            let _ = broadcast_tx.send(StateUpdate {
                project_id: project_id.to_string(),
                change: StateChange::VerificationUpdated {
                    phase_number: phase_number.to_string(),
                    status: status_str,
                },
            });

            // Trigger stage re-derivation
            update_phase_stage(project_id, phase_number, path, db, broadcast_tx).await;

            debug!(
                project_id,
                phase_number, "Parsed VERIFICATION.md successfully"
            );
        }
        Err(e) => {
            record_parse_error(project_id, path, &e.to_string(), db, broadcast_tx).await;
        }
    }
}

async fn handle_config_json(
    project_id: &str,
    path: &Path,
    content: &str,
    db: &SqlitePool,
    broadcast_tx: &broadcast::Sender<StateUpdate>,
) {
    match parser::config_json::parse_config_json(content) {
        Ok(config) => {
            // Store the raw config JSON
            if let Err(e) = db::schema::upsert_config(db, project_id, content).await {
                error!(error = %e, "Failed to upsert config");
            }

            let _ = broadcast_tx.send(StateUpdate {
                project_id: project_id.to_string(),
                change: StateChange::ConfigUpdated,
            });
            debug!(project_id, mode = ?config.mode, "Parsed config.json successfully");
        }
        Err(e) => {
            record_parse_error(project_id, path, &e.to_string(), db, broadcast_tx).await;
        }
    }
}

async fn handle_agent_history(
    project_id: &str,
    path: &Path,
    content: &str,
    db: &SqlitePool,
    broadcast_tx: &broadcast::Sender<StateUpdate>,
) {
    match parser::agent_history::parse_agent_history(content) {
        Ok(sessions) => {
            let session_count = sessions.len();
            for session in &sessions {
                let _ = db::schema::insert_agent_session(
                    db,
                    project_id,
                    session.agent_id.as_deref(),
                    session.agent_type.as_deref(),
                    session.phase.as_deref(),
                    session.plan.as_deref(),
                    session.started_at.as_deref(),
                    session.ended_at.as_deref(),
                )
                .await;
            }

            let _ = broadcast_tx.send(StateUpdate {
                project_id: project_id.to_string(),
                change: StateChange::AgentHistoryUpdated { session_count },
            });
            debug!(
                project_id,
                session_count, "Parsed agent-history.json successfully"
            );
        }
        Err(e) => {
            record_parse_error(project_id, path, &e.to_string(), db, broadcast_tx).await;
        }
    }
}

// ---------------------------------------------------------------------------
// Stage derivation
// ---------------------------------------------------------------------------

/// Re-derive the phase stage from the files in the phase directory and update DB.
async fn update_phase_stage(
    project_id: &str,
    phase_number: &str,
    changed_file_path: &Path,
    db: &SqlitePool,
    broadcast_tx: &broadcast::Sender<StateUpdate>,
) {
    // Find the phase directory from the changed file's path
    let phase_dir = match find_phase_dir(changed_file_path) {
        Some(dir) => dir,
        None => {
            debug!(path = %changed_file_path.display(), "Cannot determine phase directory");
            return;
        }
    };

    // List files in the phase directory
    let files = match list_phase_files(&phase_dir).await {
        Ok(f) => f,
        Err(e) => {
            warn!(error = %e, dir = %phase_dir.display(), "Failed to list phase directory");
            return;
        }
    };

    // Get plan count from DB
    let plan_count = match db::schema::get_plan_states_for_phase(db, project_id, phase_number).await
    {
        Ok(plans) => plans.len(),
        Err(_) => 0,
    };

    let stage = parser::stage::derive_stage(&files, plan_count);
    let stage_str = stage.to_string();

    // Update phase_state stage in DB
    // We use a direct query since upsert_phase_state requires all fields
    if let Err(e) = sqlx::query(
        "UPDATE phase_state SET stage = ?, updated_at = datetime('now') WHERE project_id = ? AND phase_number = ?",
    )
    .bind(&stage_str)
    .bind(project_id)
    .bind(phase_number)
    .execute(db)
    .await
    {
        // If no row exists yet, this is fine -- it will be created on ROADMAP parse
        debug!(error = %e, "Failed to update phase stage (may not exist yet)");
    }

    let _ = broadcast_tx.send(StateUpdate {
        project_id: project_id.to_string(),
        change: StateChange::PhaseUpdated {
            phase_number: phase_number.to_string(),
            stage: stage_str,
        },
    });
}

/// Walk up from a file path to find the phase directory.
/// Phase directories are children of the `phases/` directory.
fn find_phase_dir(file_path: &Path) -> Option<PathBuf> {
    let mut current = file_path.parent()?;
    loop {
        if let Some(parent) = current.parent()
            && parent.file_name().and_then(|n| n.to_str()) == Some("phases")
        {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

/// List all filenames in a phase directory (non-recursive).
async fn list_phase_files(dir: &Path) -> Result<Vec<String>, std::io::Error> {
    let mut files = Vec::new();
    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file()
            && let Some(name) = entry.file_name().to_str()
        {
            files.push(name.to_string());
        }
    }
    Ok(files)
}

// ---------------------------------------------------------------------------
// Bootstrap (full re-parse of all .planning/ files for a project)
// ---------------------------------------------------------------------------

/// Perform a full parse of all recognized files in a project's .planning/ directory.
///
/// Used for:
/// - Initial project registration
/// - Daemon startup reconciliation (reconcile offline changes)
pub async fn bootstrap_project(
    project_id: &str,
    planning_path: &Path,
    db: &SqlitePool,
    broadcast_tx: &broadcast::Sender<StateUpdate>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(project_id, path = %planning_path.display(), "Bootstrapping project");

    // Clear previously resolved parse errors for this project (stale errors)
    let _ =
        sqlx::query("DELETE FROM parse_errors WHERE project_id = ? AND resolved_at IS NOT NULL")
            .bind(project_id)
            .execute(db)
            .await;

    let mut file_count = 0u32;
    let mut error_count = 0u32;

    // Walk the .planning/ directory tree recursively
    let mut to_visit = vec![planning_path.to_path_buf()];
    while let Some(dir) = to_visit.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(e) => {
                warn!(dir = %dir.display(), error = %e, "Failed to read directory");
                continue;
            }
        };

        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;

            if file_type.is_dir() {
                to_visit.push(entry.path());
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            let path = entry.path();
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Only process recognized file types
            let file_type = classify_file(&file_name);
            if matches!(file_type, FileType::Unknown) {
                continue;
            }

            file_count += 1;

            // Read and process
            match tokio::fs::read_to_string(&path).await {
                Ok(_content) => {
                    process_file_event(project_id, &path, db, broadcast_tx).await;
                }
                Err(e) => {
                    error_count += 1;
                    let _ = db::schema::insert_parse_error(
                        db,
                        project_id,
                        &path.to_string_lossy(),
                        &format!("Failed to read file: {}", e),
                        "error",
                    )
                    .await;
                }
            }
        }
    }

    // Derive stage for each phase directory
    let phases_dir = planning_path.join("phases");
    if phases_dir.exists()
        && let Ok(mut entries) = tokio::fs::read_dir(&phases_dir).await
    {
        while let Some(entry) = entries.next_entry().await.ok().flatten() {
            if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                let phase_dir = entry.path();
                let dir_name = entry.file_name().to_str().unwrap_or("").to_string();

                // Extract phase number from directory name (e.g., "01-backend-foundation" -> "01")
                if let Some(phase_num) = dir_name.split('-').next() {
                    let files = list_phase_files(&phase_dir).await.unwrap_or_default();
                    let plan_count = files.iter().filter(|f| f.contains("-PLAN.md")).count();
                    let stage = parser::stage::derive_stage(&files, plan_count);

                    let _ = sqlx::query(
                            "UPDATE phase_state SET stage = ?, updated_at = datetime('now') WHERE project_id = ? AND phase_number = ?",
                        )
                        .bind(stage.to_string())
                        .bind(project_id)
                        .bind(phase_num)
                        .execute(db)
                        .await;
                }
            }
        }
    }

    info!(
        project_id,
        file_count,
        error_count,
        "{} files parsed for project, {} parse errors",
        file_count,
        error_count
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Normalize a phase number to zero-padded format for consistency.
/// "1" -> "01", "2" -> "02", "2.1" -> "2.1" (decimals unchanged), "01" -> "01"
fn normalize_phase_number(num: &str) -> String {
    // If it contains a dot (decimal), don't pad
    if num.contains('.') {
        return num.to_string();
    }
    // If it's a single digit, zero-pad
    if let Ok(n) = num.parse::<u32>() {
        format!("{:02}", n)
    } else {
        num.to_string()
    }
}

/// Record a parse error in the database and broadcast it.
async fn record_parse_error(
    project_id: &str,
    path: &Path,
    error_msg: &str,
    db: &SqlitePool,
    broadcast_tx: &broadcast::Sender<StateUpdate>,
) {
    warn!(
        project_id,
        path = %path.display(),
        error = %error_msg,
        "Parse error (last-known-good state preserved)"
    );

    let _ =
        db::schema::insert_parse_error(db, project_id, &path.to_string_lossy(), error_msg, "error")
            .await;

    let _ = broadcast_tx.send(StateUpdate {
        project_id: project_id.to_string(),
        change: StateChange::ParseError {
            file_path: path.to_string_lossy().to_string(),
            error: error_msg.to_string(),
        },
    });
}
