use anyhow::{Context, Result};
use clap::Parser;
use serde_json::{json, Value};
use uuid::Uuid;

// Reuse modules from the main crate
use nats_agent::event::Event;
use nats_agent::nats_client;

#[derive(Parser, Debug)]
#[command(name = "producer")]
#[command(about = "NATS event producer - publishes test events to a NATS stream", long_about = None)]
struct Args {
    /// NATS server URL
    #[arg(short, long, env = "NATS_URL", default_value = "nats://localhost:4222")]
    url: String,

    /// Subject to publish to
    #[arg(short, long, default_value = "events.workflow.created")]
    subject: String,

    /// Event type
    #[arg(short = 't', long, default_value = "workflow.created")]
    event_type: String,

    /// Number of events to generate
    #[arg(short, long, default_value = "1")]
    count: usize,

    /// Custom payload as JSON string
    #[arg(short, long)]
    payload: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    tracing::info!("Connecting to NATS at {}", args.url);
    let client = nats_client::connect(&args.url)
        .await
        .context("Failed to connect to NATS")?;

    tracing::info!("Publishing {} event(s) to subject: {}", args.count, args.subject);

    for i in 1..=args.count {
        let event = generate_event(&args.event_type, args.payload.as_deref())?;

        tracing::info!(
            "Publishing event {}/{} - ID: {}, Type: {}",
            i,
            args.count,
            event.id,
            event.event_type
        );

        let payload = serde_json::to_vec(&event)
            .context("Failed to serialize event")?;

        client
            .publish(args.subject.clone(), payload.into())
            .await
            .context("Failed to publish event")?;

        tracing::info!("Event published successfully");
    }

    tracing::info!("All events published successfully");
    Ok(())
}

fn generate_event(event_type: &str, custom_payload: Option<&str>) -> Result<Event> {
    let id = Uuid::new_v4().to_string();

    let payload: Value = if let Some(json_str) = custom_payload {
        serde_json::from_str(json_str)
            .context("Failed to parse custom payload JSON")?
    } else {
        // Generate sample payload based on event type
        match event_type {
            "workflow.created" => json!({
                "workflow_id": Uuid::new_v4().to_string(),
                "name": "Sample Workflow",
                "status": "created",
                "created_at": chrono::Utc::now().to_rfc3339(),
            }),
            "workflow.completed" => json!({
                "workflow_id": Uuid::new_v4().to_string(),
                "status": "completed",
                "duration_ms": 1234,
                "completed_at": chrono::Utc::now().to_rfc3339(),
            }),
            _ => json!({
                "message": "Generic event payload",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        }
    };

    Ok(Event {
        id,
        event_type: event_type.to_string(),
        payload,
    })
}
