use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::EnvFilter;

use gsdui::api;
use gsdui::config::DaemonConfig;
use gsdui::db;
use gsdui::state::AppState;

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
    let db = match db::init_pool(&config.database_url).await {
        Ok(pool) => pool,
        Err(e) => {
            tracing::error!("Failed to initialize database: {}", e);
            std::process::exit(1);
        }
    };

    // Build shared application state
    let state = Arc::new(AppState {
        db,
        config: config.clone(),
        start_time: Instant::now(),
    });

    // Build router with API routes and static file fallback
    let static_dir = config.static_dir.clone();
    let app = Router::new()
        .nest("/api/v1", api::router())
        .fallback_service(
            ServeDir::new(&static_dir).not_found_service(
                ServeFile::new(format!("{}/index.html", static_dir)),
            ),
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
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");

    tracing::info!("Shutdown complete");
}

async fn shutdown_signal() {
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
    }
}
