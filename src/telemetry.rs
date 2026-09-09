use anyhow::{Context, Result};
use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::{
    metrics::{new_view, Aggregation, Instrument, SdkMeterProvider, Stream},
    Resource,
};
use opentelemetry_otlp::{MetricExporter, WithExportConfig};
use std::time::Duration;

/// Bucket boundaries, in SECONDS, for the `*.duration` histograms.
///
/// The OpenTelemetry SDK's default boundaries are
/// `[0, 5, 10, 25, 50, 75, 100, 250, 500, 750, 1000, 2500, 5000, 7500, 10000]`,
/// which are sized for MILLISECONDS. Our histograms record seconds
/// (`Instant::elapsed().as_secs_f64()`, `.with_unit("s")`), so a typical
/// ~0.016s observation landed in the very first real bucket (`le=5`) along
/// with everything else. With all mass in one bucket `histogram_quantile`
/// could only interpolate across [0, 5] and reported ~4.75s for p95 --
/// nonsense for a handler that runs in milliseconds.
///
/// These boundaries put useful resolution where the data actually is
/// (single-digit milliseconds) while still capturing slow outliers.
const DURATION_BOUNDARIES_SECONDS: &[f64] = &[
    0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

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

    // Override the default (millisecond-sized) histogram buckets for every
    // `*.duration` instrument. The mask deliberately sets no name -- the SDK
    // rejects a view whose criteria uses a wildcard while the mask renames.
    let duration_view = new_view(
        Instrument::new().name("*.duration"),
        Stream::new().aggregation(Aggregation::ExplicitBucketHistogram {
            boundaries: DURATION_BOUNDARIES_SECONDS.to_vec(),
            record_min_max: true,
        }),
    )
    .context("Failed to build duration histogram view")?;

    // Create meter provider
    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_view(duration_view)
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
