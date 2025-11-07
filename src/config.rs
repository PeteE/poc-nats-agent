use std::env;
use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub nats_url: String,
    pub nats_stream_name: String,
    pub nats_subjects: Vec<String>,
    pub nats_consumer_name: String,
    pub batch_size: usize,
    pub otel_service_name: String,
    pub otel_endpoint: Option<String>,
}

// env vars needed:
// - NATS_URL
// - NATS_SUBJECTS
// - NATS_STREAM_NAME
// - NATS_CONSUMER_NAME
// - BATCH_SIZE
// - OTEL_SERVICE_NAME
// - OTEL_EXPORTER_OTLP_ENDPOINT
//

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Config {
            nats_url: env::var("NATS_URL")
                .unwrap_or_else(|_| "nats://localhost:4222".to_string()),

            nats_stream_name: env::var("NATS_STREAM_NAME")
                .unwrap_or_else(|_| "events".to_string()),

            nats_subjects: env::var("NATS_SUBJECTS")
                .unwrap_or_else(|_| "events.>".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),

            nats_consumer_name: env::var("NATS_CONSUMER_NAME")
                .unwrap_or_else(|_| "all-events2".to_string()),

            batch_size: env::var("BATCH_SIZE")
              .unwrap_or_else(|_| "10".to_string())
              .parse()
              .context("BATCH_SIZE must be a valid integer")?,

            otel_service_name: env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "nats-agent".to_string()),

            otel_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
        })
    }
}
