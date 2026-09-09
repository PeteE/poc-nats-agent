use tracing::{info, Level};
use async_nats::jetstream::context::Context;
use config::Config;

mod nats_client;
mod config;
mod processor;
mod event;
mod telemetry;
mod http_server;
mod message_handler;

// configs needed:
// - NATS_URL
// - NATS_SUBJECTS
// - NATS_STREAM_NAME
// - NATS_CONSUMER_NAME

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize basic console logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let config = Config::from_env()?;

    info!("Starting NATS Agent POC");

    // Initialize OpenTelemetry metrics if endpoint is configured
    let meter_provider = if let Some(ref endpoint) = config.otel_endpoint {
        let provider = telemetry::init_metrics(&config.otel_service_name, endpoint)?;
        info!("OpenTelemetry metrics initialized, exporting to {}", endpoint);
        Some(provider)
    } else {
        tracing::warn!("OpenTelemetry metrics disabled: OTEL_EXPORTER_OTLP_ENDPOINT not set");
        None
    };

    let client = nats_client::connect(&config.nats_url).await?;
    info!("Connected to NATS successfully");

    // setup stream
    let js: Context = async_nats::jetstream::new(client);

    // create a stream
    let stream = nats_client::ensure_stream(js,
        &config.nats_stream_name,
        config.nats_subjects.clone())
        .await?;

    // create a consumer
    let consumer = nats_client::ensure_consumer(stream,
        &config.nats_consumer_name)
        .await?;

    // Create shared HTTP server state
    let http_state = http_server::AppState::new();

    // Spawn the HTTP server as a background task
    let http_port = config.http_port;
    let http_handle = tokio::spawn({
        let state = http_state.clone();
        async move {
            if let Err(e) = http_server::run_server(http_port, state).await {
                tracing::error!("HTTP server error: {}", e);
            }
        }
    });

    // Spawn the message processor as a background task
    info!("Starting message processor");
    let processor_handle = tokio::spawn({
        let state = http_state.clone();
        async move {
            if let Err(e) = processor::process_messages(consumer, config, state).await {
                tracing::error!("Message processor error: {}", e);
            }
        }
    });

    // Wait for either task to finish (or run forever)
    tokio::select! {
        res = processor_handle => {
            if let Err(e) = res {
                tracing::error!("Processor task panicked: {}", e);
            }
        }
        res = http_handle => {
            if let Err(e) = res {
                tracing::error!("HTTP server task panicked: {}", e);
            }
        }
    }

    // Shutdown metrics gracefully
    info!("Shutting down metrics");
    telemetry::shutdown_metrics(meter_provider)?;

    Ok(())
}

