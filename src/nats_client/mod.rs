use anyhow::Result;
use async_nats::Client;

pub async fn connect(url: &str) -> Result<Client> {
    let client = async_nats::connect(url).await?;
    Ok(client)
}
