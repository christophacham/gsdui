use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::routing::get;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::EnvFilter;

use gsdui::api;
use gsdui::broadcast::Broadcaster;
use gsdui::config::DaemonConfig;
use gsdui::db;
use gsdui::state::AppState;
use gsdui::watcher;
use gsdui::watcher::pipeline;
use gsdui::watcher::retention;
use gsdui::ws;

#[tokio::main]
async fn main() {
    // Initialize tracing with env filter (RUST_LOG)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("gsdui=debug,tower_http=info")),
        )
        .init();

    // Load configuration from environment
    let config = DaemonConfig::from_env();
    tracing::info!(listen_addr = %config.listen_addr, "Starting GSD Pipeline UI daemon");

    // Create data directory if it doesn't exist
    if let Err(e) = tokio::fs::create_dir_all("data").await {
        tracing::error!("Failed to create data directory: {}", e);
        std::process::exit(1);
    }

    // Initialize database pool
    let pool = match db::init_pool(&config.database_url).await {
        Ok(pool) => pool,
        Err(e) => {
            tracing::error!("Failed to initialize database: {}", e);
            std::process::exit(1);
        }
    };

    // --- Channel setup ---

    // Raw file events: watcher -> debouncer (capacity 256 for burst absorption)
    let (file_event_tx, file_event_rx) = mpsc::channel(256);

    // Debounced events: debouncer -> pipeline (capacity 128)
    let (debounced_tx, debounced_rx) = mpsc::channel(128);

    // State updates: pipeline -> broadcast forwarder (capacity 64)
    let (broadcast_tx, _broadcast_rx) = broadcast::channel(64);

    // Per-project broadcaster for WebSocket clients
    let broadcaster = Broadcaster::new();

    // --- Spawn background tasks ---

    // Cancellation token for graceful shutdown
    let cancel = CancellationToken::new();

    // Spawn debouncer task (75ms delay, midpoint of 50-100ms range)
    let debouncer_handle =
        watcher::debounce::Debouncer::spawn(file_event_rx, debounced_tx, Duration::from_millis(75));

    // Spawn parse pipeline task
    let pipeline_db = pool.clone();
    let pipeline_broadcast_tx = broadcast_tx.clone();
    let pipeline_handle = tokio::spawn(async move {
        pipeline::run_pipeline(debounced_rx, pipeline_db, pipeline_broadcast_tx).await;
    });

    // Spawn retention pruning task
    let retention_db = pool.clone();
    let retention_cancel = cancel.clone();
    let retention_handle = tokio::spawn(async move {
        retention::run_retention_pruning(retention_db, retention_cancel, None).await;
    });

    // --- Startup reconciliation ---
    // Re-parse all registered projects to reconcile offline changes
    {
        let projects = match db::schema::get_all_projects(&pool).await {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to fetch registered projects: {}", e);
                Vec::new()
            }
        };

        let mut file_watcher = watcher::FileWatcher::new(file_event_tx.clone());

        for project in &projects {
            let project_path = std::path::Path::new(&project.path);

            // Check if project path still exists
            if !project_path.exists() {
                tracing::warn!(
                    project_id = %project.id,
                    path = %project.path,
                    "Project path no longer exists, marking as offline"
                );
                // Mark project as offline
                let _ = sqlx::query(
                    "UPDATE projects SET status = 'offline', last_seen_at = datetime('now') WHERE id = ?",
                )
                .bind(&project.id)
                .execute(&pool)
                .await;
                continue;
            }

            // Bootstrap: full re-parse of all .planning/ files
            let planning_path = project_path.join(".planning");
            if planning_path.exists()
                && let Err(e) =
                    pipeline::bootstrap_project(&project.id, &planning_path, &pool, &broadcast_tx)
                        .await
            {
                tracing::error!(
                    project_id = %project.id,
                    error = %e,
                    "Failed to bootstrap project during startup reconciliation"
                );
            }

            // Add file watcher for this project
            if let Err(e) = file_watcher.watch_project(&project.id, project_path) {
                tracing::error!(
                    project_id = %project.id,
                    error = %e,
                    "Failed to start file watcher for project"
                );
            }
        }

        tracing::info!(
            project_count = projects.len(),
            watched = file_watcher.watched_count(),
            "Startup reconciliation complete"
        );
    }

    // Build shared application state
    let state = Arc::new(AppState {
        db: pool,
        config: config.clone(),
        start_time: Instant::now(),
        broadcast_tx: broadcast_tx.clone(),
        file_event_tx,
        broadcaster,
    });

    // --- Spawn broadcast forwarder ---
    // Subscribes to pipeline's broadcast channel and forwards to per-project Broadcaster
    let forwarder_state = state.clone();
    let mut forwarder_rx = broadcast_tx.subscribe();
    let forwarder_cancel = cancel.clone();
    let forwarder_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                result = forwarder_rx.recv() => {
                    match result {
                        Ok(update) => {
                            let project_id = update.project_id.clone();
                            let _ = forwarder_state.broadcaster.broadcast(&project_id, update).await;
                        }
                        Err(broadcast::error::RecvError::Lagged(count)) => {
                            tracing::warn!(lagged = count, "Broadcast forwarder lagged, skipped messages");
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::info!("Broadcast forwarder shutting down (channel closed)");
                            break;
                        }
                    }
                }
                _ = forwarder_cancel.cancelled() => {
                    tracing::info!("Broadcast forwarder shutting down (cancelled)");
                    break;
                }
            }
        }
    });

    // Build router with API routes, WebSocket, and static file fallback
    let static_dir = config.static_dir.clone();
    let app = Router::new()
        .route("/api/v1/ws/state", get(ws::ws_handler))
        .nest("/api/v1", api::router())
        .fallback_service(
            ServeDir::new(&static_dir)
                .not_found_service(ServeFile::new(format!("{}/index.html", static_dir))),
        )
        .with_state(state);

    // Bind to configured address
    let listener = tokio::net::TcpListener::bind(&config.listen_addr)
        .await
        .expect("Failed to bind to address");

    tracing::info!(
        addr = %listener.local_addr().unwrap(),
        "Listening for connections"
    );

    // Serve with graceful shutdown on SIGINT/SIGTERM
    let server_cancel = cancel.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(server_cancel))
        .await
        .expect("Server error");

    // Signal all background tasks to stop
    cancel.cancel();

    // Wait briefly for tasks to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Abort tasks if they haven't stopped
    debouncer_handle.abort();
    pipeline_handle.abort();
    retention_handle.abort();
    forwarder_handle.abort();

    tracing::info!("Shutdown complete");
}

async fn shutdown_signal(cancel: CancellationToken) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("Received SIGINT, shutting down"); },
        _ = terminate => { tracing::info!("Received SIGTERM, shutting down"); },
        _ = cancel.cancelled() => { tracing::info!("Shutdown signal received from cancel token"); },
    }
}
