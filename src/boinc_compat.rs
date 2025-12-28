use crate::boinc_client::BoincClient;
use anyhow::Result;
use tracing::info;

/// Public API: fetch a work id from the configured endpoint.
pub async fn fetch_work(endpoint: &str) -> Result<String> {
    info!(%endpoint, "fetch_work -> client path");
    let c = BoincClient::new(endpoint.to_string());
    let w = c.fetch_work().await?;
    Ok(w.job_id)
}
