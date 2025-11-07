use async_nats::jetstream::consumer::PullConsumer;
use anyhow::{Result, Context as AnyhowContext};
use futures::StreamExt;
use tracing::{info, error, warn};
use crate::event::Event;
use crate::config::Config;

// OpenTelemetry imports for metrics
use opentelemetry::{
    global,
    KeyValue,
    metrics::{Counter, Histogram, Meter},
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
    messages_processed: Counter<u64>,
    messages_failed: Counter<u64>,
    processing_latency: Histogram<f64>,
}

impl ProcessorMetrics {
    fn new(meter: &Meter) -> Self {
        ProcessorMetrics {
            messages_processed: meter
                .u64_counter("messages.processed")
                .with_description("Total number of messages successfully processed")
                .build(),

            messages_failed: meter
                .u64_counter("messages.failed")
                .with_description("Total number of messages that failed processing")
                .build(),

            processing_latency: meter
                .f64_histogram("message.processing.duration")
                .with_description("Message processing latency in seconds")
                .with_unit("s")
                .build(),
        }
    }
}

pub async fn process_messages(consumer: PullConsumer, config: Config) -> Result<()> {
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

                    // Deserialize the message payload into an Event
                    match serde_json::from_slice::<Event>(&msg.payload) {
                        Ok(event) => {
                            info!(
                                "Received event - ID: {}, Type: {}, Subject: {}",
                                event.id, event.event_type, msg.subject
                            );
                            info!("Event details: {:?}", event);

                            // TODO: Process the event based on event_type
                            // For now, just print and acknowledge

                            if let Err(e) = msg.ack().await {
                                error!("Failed to acknowledge message: {}", e);
                                // Increment failure counter
                                metrics.messages_failed.add(1, &[
                                    KeyValue::new("event_type", event.event_type.clone()),
                                    KeyValue::new("error", "ack_failed"),
                                ]);
                            } else {
                                info!("Event processed and acknowledged");

                                // Record successful processing
                                let duration = start.elapsed().as_secs_f64();
                                metrics.processing_latency.record(duration, &[
                                    KeyValue::new("event_type", event.event_type.clone()),
                                ]);
                                metrics.messages_processed.add(1, &[
                                    KeyValue::new("event_type", event.event_type),
                                ]);
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to deserialize event from subject {}: {}. Payload: {}",
                                msg.subject,
                                e,
                                String::from_utf8_lossy(&msg.payload)
                            );

                            // Increment failure counter for deserialization errors
                            metrics.messages_failed.add(1, &[
                                KeyValue::new("subject", msg.subject.to_string()),
                                KeyValue::new("error", "deserialization_failed"),
                            ]);

                            // Still acknowledge to avoid reprocessing bad messages
                            if let Err(ack_err) = msg.ack().await {
                                error!("Failed to acknowledge bad message: {}", ack_err);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error receiving message: {}", e);
                    // Increment failure counter for message receive errors
                    metrics.messages_failed.add(1, &[
                        KeyValue::new("error", "receive_failed"),
                    ]);
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
