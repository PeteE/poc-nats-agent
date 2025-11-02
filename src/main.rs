use anyhow::Result;
use futures::StreamExt;
use tracing::{info, Level};
use tracing_subscriber;

mod nats_client;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting NATS Agent POC");

    // Connect to NATS
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
    info!("Connecting to NATS at {}", nats_url);

    let client = nats_client::connect(&nats_url).await?;
    info!("Connected to NATS successfully");

    // Subscribe to a test subject
    let subject = "test.messages";
    info!("Subscribing to subject: {}", subject);

    let mut subscriber = client.subscribe(subject).await?;
    info!("Subscribed successfully, waiting for messages...");

    // Process messages
    while let Some(message) = subscriber.next().await {
        info!(
            "Received message on {}: {:?}",
            message.subject,
            String::from_utf8_lossy(&message.payload)
        );

        // TODO: Process message with agent logic
        // TODO: Call LLM if needed
        // TODO: Publish results
    }

    Ok(())
}
