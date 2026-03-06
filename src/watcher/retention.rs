use std::time::Duration;

use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::db;

/// Run the periodic data retention pruning task.
///
/// On each tick (default: every 24 hours), queries all projects and their
/// retention_days setting, then hard-deletes expired records from:
/// - execution_runs (and cascading to commits) where completed_at < cutoff
/// - agent_sessions where ended_at < cutoff
/// - parse_errors where occurred_at < cutoff AND resolved_at IS NOT NULL
///
/// Does NOT delete phase_state, plan_state (current state), or verification_results
/// (lightweight, useful for historical trends).
pub async fn run_retention_pruning(
    db: SqlitePool,
    cancel: CancellationToken,
    interval: Option<Duration>,
) {
    let interval = interval.unwrap_or(Duration::from_secs(24 * 60 * 60)); // Default: 24 hours
    info!(interval_secs = interval.as_secs(), "Retention pruning task started");

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("Retention pruning task shutting down");
                return;
            }
            _ = tokio::time::sleep(interval) => {
                prune_all_projects(&db).await;
            }
        }
    }
}

/// Prune expired data for all registered projects.
async fn prune_all_projects(db: &SqlitePool) {
    let projects = match db::schema::get_all_projects(db).await {
        Ok(p) => p,
        Err(e) => {
            error!(error = %e, "Failed to fetch projects for retention pruning");
            return;
        }
    };

    for project in &projects {
        let retention_days = project.retention_days.unwrap_or(180);
        let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

        let runs = prune_expired_runs(db, &project.id, &cutoff_str)
            .await
            .unwrap_or(0);
        let sessions = prune_expired_sessions(db, &project.id, &cutoff_str)
            .await
            .unwrap_or(0);
        let errors = prune_resolved_errors(db, &project.id, &cutoff_str)
            .await
            .unwrap_or(0);

        if runs > 0 || sessions > 0 || errors > 0 {
            info!(
                project_id = %project.id,
                project_name = %project.name,
                runs_deleted = runs,
                sessions_deleted = sessions,
                errors_deleted = errors,
                "Retention pruning: deleted {} execution runs, {} agent sessions, {} resolved parse errors",
                runs, sessions, errors
            );
        }
    }
}

/// Delete execution_runs (and cascading commits) older than the cutoff.
pub async fn prune_expired_runs(
    pool: &SqlitePool,
    project_id: &str,
    cutoff: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM execution_runs WHERE project_id = ? AND completed_at IS NOT NULL AND completed_at < ?",
    )
    .bind(project_id)
    .bind(cutoff)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Delete agent_sessions older than the cutoff.
pub async fn prune_expired_sessions(
    pool: &SqlitePool,
    project_id: &str,
    cutoff: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM agent_sessions WHERE project_id = ? AND ended_at IS NOT NULL AND ended_at < ?",
    )
    .bind(project_id)
    .bind(cutoff)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Delete resolved parse_errors older than the cutoff.
///
/// Only prunes errors that have been resolved (resolved_at IS NOT NULL).
/// Unresolved errors are kept regardless of age.
pub async fn prune_resolved_errors(
    pool: &SqlitePool,
    project_id: &str,
    cutoff: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM parse_errors WHERE project_id = ? AND resolved_at IS NOT NULL AND occurred_at < ?",
    )
    .bind(project_id)
    .bind(cutoff)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
