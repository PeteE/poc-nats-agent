use anyhow::{Context, Result};
use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::{
    metrics::SdkMeterProvider,
    Resource,
};
use opentelemetry_otlp::{MetricExporter, WithExportConfig};
use std::time::Duration;

pub fn init_metrics(service_name: &str, otlp_endpoint: &str) -> Result<SdkMeterProvider> {
    // Create resource with service name
    let resource = Resource::new(vec![
        KeyValue::new("service.name", service_name.to_string()),
    ]);

    // Create OTLP metrics exporter
    let exporter = MetricExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint)
        .with_timeout(Duration::from_secs(3))
        .build()
        .context("Failed to create OTLP metrics exporter")?;

    // Create meter provider
    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(
            opentelemetry_sdk::metrics::PeriodicReader::builder(
                exporter,
                opentelemetry_sdk::runtime::Tokio
            )
            .with_interval(Duration::from_secs(30))
            .build()
        )
        .build();

    // Set as global meter provider
    global::set_meter_provider(meter_provider.clone());

    Ok(meter_provider)
}

pub fn shutdown_metrics(provider: Option<SdkMeterProvider>) -> Result<()> {
    if let Some(provider) = provider {
        provider.shutdown().context("Failed to shutdown meter provider")?;
    }
    Ok(())
}
