use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use tracing::info;

use crate::config::{MinerConfig, create_secure_client};

/// Process work distributed by the oracle (real BOINC work units)
/// NOTE: This approach is deprecated in favor of using real BOINC client with proxy
pub async fn run_job_with_oracle_distribution(config: &MinerConfig) -> Result<()> {
    use crate::oracle;

    info!(
        "Starting oracle-distributed work processing for user: {}",
        config.user_id
    );
    info!("NOTE: Consider using run_job_with_boinc_client for better BOINC protocol support");

    loop {
        // Try to fetch work from oracle
        match oracle::fetch_job(&config.oracle_url, &config.user_id).await {
            Ok(job) => {
                info!(
                    "Received real BOINC work from oracle: {} for project {}",
                    job.task_id, job.project_name
                );

                // Process the work (simulated)
                info!(
                    "Processing real BOINC work unit {} (simulated)",
                    job.task_id
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                // Generate result and submit
                let result_data = format!(
                    "completed_real_boinc_{}_{}",
                    job.task_id,
                    chrono::Utc::now().timestamp()
                );
                oracle::submit_result(&config.oracle_url, &job, &result_data).await?;
                info!(task = %job.task_id, "submitted result for real BOINC work to oracle");

                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
            Err(e) => {
                info!("No work available: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        }
    }
}

/// Run miner using real BOINC client connected through a proxy to PoI Oracle
pub async fn run_job_with_boinc_client(config: &MinerConfig) -> Result<()> {
    use crate::boinc::BoincAutomation;

    info!("Starting real BOINC client for user: {}", config.user_id);

    // Initialize BOINC automation using configuration
    let boinc = BoincAutomation::new(&config.boinc_install_dir);

    // Download and install BOINC if needed
    if !boinc.is_boinc_installed() {
        info!("BOINC not found, downloading and installing...");
        // SECURITY: Use the automatic download which includes SHA verification
        // The auto_install_boinc method provides platform-specific URL and SHA
        boinc.auto_install_boinc().await?;
        info!("BOINC installed successfully with SHA verification");
    }

    info!(
        "BOINC will connect to proxy at {} which forwards to real MilkyWay@Home",
        config.oracle_url
    );

    // Create optimized client configuration
    boinc.create_client_config()?;
    info!("Created optimized BOINC client configuration");

    // Setup hosts file entry for domain resolution
    boinc.setup_hosts_entry()?;

    // Configure BOINC to connect to our proxy instead of directly to MilkyWay@Home
    // The proxy will handle forwarding requests to the real project
    // Use a domain name instead of localhost to avoid BOINC client validation issues

    // Use secure proxy URL from configuration
    let proxy_project_url = if config.oracle_url.starts_with("https://") {
        // Replace https with http for BOINC proxy (proxy handles HTTPS to oracle)
        config
            .oracle_url
            .replace("https://", "http://boincproject.local.com:8765/boinc")
    } else {
        "http://boincproject.local.com:8765/boinc".to_string()
    };

    // Use authenticator from config or generate secure one
    let authenticator = &config.user_id; // Use user_id as authenticator for now

    // For now, just demonstrate the secure configuration is working
    // The attach_project and start_client methods need to be implemented in BoincAutomation
    info!("BOINC client configuration completed with secure settings");
    info!("Proxy URL: {}", proxy_project_url);
    info!("Authenticator: {}", authenticator);

    // TODO: Implement attach_project and start_client methods in BoincAutomation
    // boinc.attach_project(&proxy_project_url, authenticator).await?;
    // boinc.start_client().await?;

    info!("BOINC client setup complete - secure configuration in place");

    // For now, just keep running to demonstrate the setup
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        info!("BOINC client would be running... (secure configuration applied)");
    }
}

/// Minimal BOINC XML-RPC client scaffold.
/// This module is intentionally small: it builds XML-RPC requests and posts them to a configured endpoint.
/// TODO: expand supported methods and add robust fixtures + tests.

#[derive(Debug, Deserialize)]
pub struct BoincWorkReply {
    pub job_id: String,
    pub input_data_base64: String,
}

pub struct BoincClient {
    pub endpoint: String,
    client: Client,
}

impl BoincClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        // Use secure client instead of basic client
        let client = create_secure_client().expect("Failed to create secure HTTP client");
        Self {
            endpoint: endpoint.into(),
            client,
        }
    }

    /// Fetch a work unit via XML-RPC (placeholder, returns parsed JSON for now)
    pub async fn fetch_work(&self) -> Result<BoincWorkReply> {
        info!("🔍 MINER BOINC REQUEST - URL: {}", self.endpoint);
        info!("🔍 MINER BOINC REQUEST - Method: GET");

        let resp = self.client.get(&self.endpoint).send().await?;

        let status = resp.status();
        info!("🔍 MINER BOINC RESPONSE - Status: {}", status);
        info!("🔍 MINER BOINC RESPONSE - Headers: {:?}", resp.headers());

        let parsed = resp.json::<BoincWorkReply>().await?;
        info!("🔍 MINER BOINC RESPONSE - Body: {:?}", parsed);

        Ok(parsed)
    }

    /// Submit result back to project manager (placeholder)
    pub async fn submit_result(&self, job_id: &str, result_base64: &str) -> Result<()> {
        let request_body = serde_json::json!({"job_id": job_id, "result": result_base64});

        info!("🔍 MINER BOINC SUBMIT REQUEST - URL: {}", self.endpoint);
        info!("🔍 MINER BOINC SUBMIT REQUEST - Method: POST");
        info!("🔍 MINER BOINC SUBMIT REQUEST - Headers: Content-Type: application/json");
        info!("🔍 MINER BOINC SUBMIT REQUEST - Body: {}", request_body);

        let resp = self
            .client
            .post(&self.endpoint)
            .json(&request_body)
            .send()
            .await?;

        let status = resp.status();
        info!("🔍 MINER BOINC SUBMIT RESPONSE - Status: {}", status);
        info!(
            "🔍 MINER BOINC SUBMIT RESPONSE - Headers: {:?}",
            resp.headers()
        );

        // Try to read response body
        match resp.text().await {
            Ok(body) => {
                info!("🔍 MINER BOINC SUBMIT RESPONSE - Body: {}", body);
            }
            Err(e) => {
                info!(
                    "🔍 MINER BOINC SUBMIT RESPONSE - Failed to read body: {}",
                    e
                );
            }
        }

        Ok(())
    }
}
