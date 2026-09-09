use async_nats::jetstream::consumer::PullConsumer;
use anyhow::{Result, Context as AnyhowContext};
use futures::StreamExt;
use tracing::{info, error};
use crate::event::Event;
use crate::config::Config;
use crate::http_server::AppState;
use crate::message_handler;

// OpenTelemetry imports for metrics
use opentelemetry::{
    global,
    KeyValue,
    metrics::{Histogram, Meter},
};
use std::time::Instant;
use once_cell::sync::Lazy;
use std::env;

// Lazy-initialized service name for OpenTelemetry meter
static SERVICE_NAME: Lazy<String> = Lazy::new(|| {
    env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "nats-agent".to_string())
});

// Struct to hold all our metrics
struct ProcessorMetrics {
    processing_latency: Histogram<f64>,
}

impl ProcessorMetrics {
    fn new(meter: &Meter) -> Self {
        ProcessorMetrics {
            processing_latency: meter
                .f64_histogram("events.processed.duration")
                .with_description("Message processing latency in seconds (labeled by status: success/failed)")
                .with_unit("s")
                .build(),
        }
    }
}

pub async fn process_messages(consumer: PullConsumer, config: Config, http_state: AppState) -> Result<()> {
    info!("Starting message processor");

    // Initialize OpenTelemetry metrics
    let meter = global::meter(&SERVICE_NAME);
    let metrics = ProcessorMetrics::new(&meter);

    info!("Waiting for messages...");

    // Continuously fetch and process messages in batches
    loop {
        // Create a new batch request for each iteration
        // The expires() timeout ensures we don't wait forever if no messages arrive
        let mut messages = consumer
            .batch()
            .max_messages(config.batch_size)
            .expires(std::time::Duration::from_secs(30))
            .messages()
            .await
            .with_context(|| format!("Failed to read messages with batch size {} from stream", config.batch_size))?;

        let mut batch_count = 0;

        // Process all messages in this batch
        while let Some(message) = messages.next().await {
            batch_count += 1;
            match message {
                Ok(msg) => {
                    // Start timing for latency measurement
                    let start = Instant::now();

                    // Deserialize the message payload into an Event (generic JSON)
                    match serde_json::from_slice::<Event>(&msg.payload) {
                        Ok(event) => {
                            // The subject is the source of truth for the event
                            // kind; the body carries only the payload.
                            let event_id = event.get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let subject = msg.subject.to_string();

                            info!(
                                "Received event - ID: {}, Subject: {}",
                                event_id, subject
                            );

                            // Handle the message using the message handler
                            let handle_result =
                                message_handler::handle_message(&event, &subject).await;

                            // Check if message handling was successful
                            if let Err(e) = handle_result {
                                error!("Failed to handle message: {}", e);
                                let duration = start.elapsed().as_secs_f64();
                                metrics.processing_latency.record(duration, &[
                                    KeyValue::new("subject", subject.clone()),
                                    KeyValue::new("status", "failed"),
                                    KeyValue::new("error", "handler_failed"),
                                ]);
                                // Increment HTTP state error counter
                                http_state.increment_errors().await;
                                // Still acknowledge to avoid reprocessing
                                if let Err(ack_err) = msg.ack().await {
                                    error!("Failed to acknowledge message after handler error: {}", ack_err);
                                }
                                continue;
                            }

                            // Acknowledge the message after successful handling
                            if let Err(e) = msg.ack().await {
                                error!("Failed to acknowledge message: {}", e);
                                let duration = start.elapsed().as_secs_f64();
                                metrics.processing_latency.record(duration, &[
                                    KeyValue::new("subject", subject.clone()),
                                    KeyValue::new("status", "failed"),
                                    KeyValue::new("error", "ack_failed"),
                                ]);
                                // Increment HTTP state error counter
                                http_state.increment_errors().await;
                            } else {
                                info!("Event processed and acknowledged");

                                // Record successful processing
                                let duration = start.elapsed().as_secs_f64();
                                metrics.processing_latency.record(duration, &[
                                    KeyValue::new("subject", subject.clone()),
                                    KeyValue::new("status", "success"),
                                ]);
                                // Increment HTTP state message counter (for /status endpoint)
                                http_state.increment_messages().await;
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to deserialize event from subject {}: {}. Payload: {}",
                                msg.subject,
                                e,
                                String::from_utf8_lossy(&msg.payload)
                            );

                            // Record failure for deserialization errors
                            let duration = start.elapsed().as_secs_f64();
                            metrics.processing_latency.record(duration, &[
                                KeyValue::new("subject", msg.subject.to_string()),
                                KeyValue::new("status", "failed"),
                                KeyValue::new("error", "deserialization_failed"),
                            ]);
                            // Increment HTTP state error counter
                            http_state.increment_errors().await;

                            // Still acknowledge to avoid reprocessing bad messages
                            if let Err(ack_err) = msg.ack().await {
                                error!("Failed to acknowledge bad message: {}", ack_err);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error receiving message: {}", e);
                    // Record failure for message receive errors (duration = 0 since we never started processing)
                    metrics.processing_latency.record(0.0, &[
                        KeyValue::new("status", "failed"),
                        KeyValue::new("error", "receive_failed"),
                    ]);
                    // Increment HTTP state error counter
                    http_state.increment_errors().await;
                }
            }
        }

        if batch_count == 0 {
            info!("No messages available in batch, waiting for next batch...");
        } else {
            info!("Processed {} messages in batch", batch_count);
        }
    }
}
