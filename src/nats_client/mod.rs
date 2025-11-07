use anyhow::{
    Result,
    Context as AnyhowContext,
};
use async_nats::{
    client::Client,
    jetstream::{
        context::Context,
        stream::{
            Config,
            Stream,
        },
        consumer::PullConsumer,
    },
};

pub async fn connect(url: &str) -> Result<Client> {
    let client = async_nats::connect(url)
        .await
        .context(format!("Unable to connect to nats server: {}", url))?;
    Ok(client)
}

pub async fn ensure_consumer(stream: Stream, consumer_name: &str) -> Result<PullConsumer> {
    let consumer = stream.get_or_create_consumer(
        consumer_name,
        async_nats::jetstream::consumer::pull::Config {
            durable_name: Some(consumer_name.to_string()),
            name: Some(consumer_name.to_string()),
            description: Some("consumer of all workflow events".to_string()),
            ..Default::default()
        },
    )
    .await
    .context(format!("error creating durable consumer: {}", consumer_name))?;

    Ok(consumer)
}

pub async fn ensure_stream(js: Context, stream_name: &str, subjects: Vec<String>) -> anyhow::Result<Stream> {
    let s = js.get_or_create_stream(Config {
        name: stream_name.to_string(),
        subjects,
        ..Default::default()
    }).await?;
    Ok(s)
}
