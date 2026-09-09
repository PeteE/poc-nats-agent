use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;
use sysinfo::System;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    start_time: Instant,
    stats: Arc<Mutex<RuntimeStats>>,
}

#[derive(Default, Clone)]
pub struct RuntimeStats {
    pub messages_processed: u64,
    pub last_message_time: Option<std::time::SystemTime>,
    pub errors: u64,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Serialize)]
struct StatusResponse {
    uptime_seconds: u64,
    memory_usage_mb: u64,
    messages_processed: u64,
    last_message_time: Option<String>,
    errors: u64,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            stats: Arc::new(Mutex::new(RuntimeStats::default())),
        }
    }

    pub async fn increment_messages(&self) {
        let mut stats = self.stats.lock().await;
        stats.messages_processed += 1;
        stats.last_message_time = Some(std::time::SystemTime::now());
    }

    pub async fn increment_errors(&self) {
        let mut stats = self.stats.lock().await;
        stats.errors += 1;
    }
}

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(HealthResponse {
        status: "ok".to_string(),
    }))
}

async fn status_handler(State(state): State<AppState>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();

    // Get memory usage
    let mut sys = System::new_all();
    sys.refresh_memory();
    let memory_mb = sys.used_memory() / 1024 / 1024;

    // Get runtime stats
    let stats = state.stats.lock().await;
    let last_message = stats.last_message_time.map(|t| {
        humantime::format_rfc3339_seconds(t).to_string()
    });

    (StatusCode::OK, Json(StatusResponse {
        uptime_seconds: uptime,
        memory_usage_mb: memory_mb,
        messages_processed: stats.messages_processed,
        last_message_time: last_message,
        errors: stats.errors,
    }))
}

pub async fn run_server(port: u16, state: AppState) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/status", get(status_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("HTTP server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
