use anyhow::{Context, Result};
use clap::Parser;
use serde_json::json;
use uuid::Uuid;
use std::time::Instant;

// Reuse modules from the main crate
use nats_agent::event::Event;
use nats_agent::nats_client;

// OpenTelemetry imports
use opentelemetry::{
    global,
    KeyValue,
    metrics::{Histogram, Meter},
};

#[derive(Parser, Debug)]
#[command(name = "producer")]
#[command(about = "NATS event producer - publishes test events to a NATS stream", long_about = None)]
struct Args {
    /// NATS server URL
    #[arg(short, long, env = "NATS_URL", default_value = "nats://localhost:4222")]
    url: String,

    /// Subject to publish to. This is the only identifier of what kind of
    /// event this is -- the body carries payload only, and the consumer
    /// reads the kind from the subject it arrived on.
    #[arg(short, long, default_value = "events.workflow.executed")]
    subject: String,

    /// Number of events to generate
    #[arg(short, long, default_value = "10")]
    count: usize,

    /// Custom payload as JSON string
    #[arg(short, long)]
    payload: Option<String>,

    /// OpenTelemetry OTLP endpoint
    #[arg(long, env = "OTEL_EXPORTER_OTLP_ENDPOINT")]
    otel_endpoint: Option<String>,
}

// Metrics structure
struct ProducerMetrics {
    events_published_duration: Histogram<f64>,
}

impl ProducerMetrics {
    fn new(meter: &Meter) -> Self {
        ProducerMetrics {
            events_published_duration: meter
                .f64_histogram("events.published.duration")
                .with_description("Event publishing duration in seconds (provides count and distribution via histogram)")
                .with_unit("s")
                .build(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    // Initialize OpenTelemetry metrics if endpoint is configured
    let meter_provider = if let Some(ref endpoint) = args.otel_endpoint {
        use nats_agent::telemetry;
        let provider = telemetry::init_metrics("nats-producer", endpoint)?;
        tracing::info!("OpenTelemetry metrics initialized, exporting to {}", endpoint);
        Some(provider)
    } else {
        tracing::info!("OpenTelemetry metrics disabled: OTEL_EXPORTER_OTLP_ENDPOINT not set");
        None
    };

    // Initialize metrics
    let meter = global::meter("nats-producer");
    let metrics = ProducerMetrics::new(&meter);

    tracing::info!("Connecting to NATS at {}", args.url);
    let client = nats_client::connect(&args.url)
        .await
        .context("Failed to connect to NATS")?;

    tracing::info!("Publishing {} event(s) to subject: {}", args.count, args.subject);

    for i in 1..=args.count {
        let start = Instant::now();
        let event = generate_event(&args.subject, args.payload.as_deref())?;

        // Extract id for logging if present
        let event_id = event.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        tracing::info!(
            "Publishing event {}/{} - ID: {}, Subject: {}",
            i,
            args.count,
            event_id,
            args.subject
        );

        let payload = serde_json::to_vec(&event)
            .context("Failed to serialize event")?;

        match client
            .publish(args.subject.clone(), payload.into())
            .await
        {
            Ok(_) => {
                let duration = start.elapsed().as_secs_f64();

                // Record successful publish
                metrics.events_published_duration.record(duration, &[
                    KeyValue::new("status", "success"),
                    KeyValue::new("subject", args.subject.clone()),
                ]);

                tracing::info!("Event published successfully");
            }
            Err(e) => {
                let duration = start.elapsed().as_secs_f64();

                // Record failed publish
                metrics.events_published_duration.record(duration, &[
                    KeyValue::new("status", "failed"),
                    KeyValue::new("subject", args.subject.clone()),
                    KeyValue::new("error", e.to_string()),
                ]);

                return Err(e).context("Failed to publish event");
            }
        }
    }

    tracing::info!("All events published successfully");

    // Shutdown metrics gracefully
    if let Some(provider) = meter_provider {
        use nats_agent::telemetry;
        telemetry::shutdown_metrics(Some(provider))?;
        tracing::info!("Metrics exported");
    }

    Ok(())
}

/// Build a sample event body for the given subject.
///
/// The body deliberately does NOT carry the subject or an `event_type`
/// field: the subject a message is published to is the single source of
/// truth for its kind, and duplicating it in the body just allows the two
/// to drift. Consumers read the kind from the subject (the agent forwards
/// it to handlers as `NATS_SUBJECT`).
fn generate_event(subject: &str, custom_payload: Option<&str>) -> Result<Event> {
    let event: Event = if let Some(json_str) = custom_payload {
        // Use custom payload directly (can be any JSON structure)
        serde_json::from_str(json_str)
            .context("Failed to parse custom payload JSON")?
    } else {
        // Shape the sample payload to the subject's trailing token, so
        // `events.workflow.executed` and `events.workflow.created` produce
        // plausibly different bodies.
        let id = Uuid::new_v4().to_string();
        let kind = subject.rsplit('.').next().unwrap_or(subject);
        match kind {
            "created" => json!({
                "id": id,
                "workflow_id": Uuid::new_v4().to_string(),
                "name": "Sample Workflow",
                "status": "created",
                "created_at": chrono::Utc::now().to_rfc3339(),
            }),
            "executed" | "completed" => json!({
                "id": id,
                "workflow_id": Uuid::new_v4().to_string(),
                "status": "completed",
                "duration_ms": 1234,
                "completed_at": chrono::Utc::now().to_rfc3339(),
            }),
            _ => json!({
                "id": id,
                "message": "Generic event payload",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        }
    };

    Ok(event)
}
