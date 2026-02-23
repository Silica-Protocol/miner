use anyhow::Result;
use serde::{Deserialize, Serialize};
use silica_models::boinc::BoincWork;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{info, warn};

use crate::config::{create_secure_client, sanitize_user_id, validate_boinc_work};
use crate::rate_limiter::RateLimiter;

// Global rate limiter for oracle requests
static ORACLE_RATE_LIMITER: OnceLock<RateLimiter> = OnceLock::new();

/// Response from the miner API for job requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetJobResponse {
    pub job: Option<BoincWork>,
    pub message: Option<String>,
}

/// Request for submitting work to the miner API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitWorkRequest {
    pub user: String,
    pub work: BoincWork,
}

/// Response from the miner API for work submissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitWorkResponse {
    pub success: bool,
    pub message: String,
    pub receipt: Option<String>,
}

/// Fetch a job from the PoI Oracle's miner API with security validation
pub async fn fetch_job(oracle_url: &str, user: &str) -> Result<BoincWork> {
    // Initialize rate limiter if not already done
    let rate_limiter = ORACLE_RATE_LIMITER.get_or_init(|| {
        // Get rate limit from environment or use default (60 requests per minute)
        let rate_limit = std::env::var("CHERT_RATE_LIMIT_PER_MINUTE")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap_or(60);
        RateLimiter::new(rate_limit, Duration::from_secs(60))
    });

    // Check rate limit before making request
    let rate_limit_key = format!("oracle_fetch:{}", oracle_url);
    if !rate_limiter.is_allowed(&rate_limit_key).await? {
        let remaining_time = rate_limiter.time_until_reset(&rate_limit_key).await?;
        if let Some(wait_time) = remaining_time {
            warn!(
                "Rate limited for oracle requests. Waiting {:?} before retry",
                wait_time
            );
            tokio::time::sleep(wait_time).await;
        }
        // Try again after waiting
        if !rate_limiter.is_allowed(&rate_limit_key).await? {
            return Err(anyhow::anyhow!("Rate limited: too many requests to oracle"));
        }
    }

    // Validate inputs
    if oracle_url.is_empty() {
        return Err(anyhow::anyhow!("Oracle URL cannot be empty"));
    }
    if user.is_empty() {
        return Err(anyhow::anyhow!("User ID cannot be empty"));
    }

    // Security validation for HTTPS
    if !oracle_url.starts_with("https://") {
        warn!("Oracle URL is not HTTPS: {}. This is insecure!", oracle_url);
        // In strict mode, we could reject HTTP entirely
        if std::env::var("CHERT_REQUIRE_HTTPS").unwrap_or_default() == "true" {
            return Err(anyhow::anyhow!(
                "HTTPS is required but oracle URL is HTTP: {}",
                oracle_url
            ));
        }
    }

    // Sanitize user input to prevent injection
    let sanitized_user = sanitize_user_id(user)?;

    let client = create_secure_client()?;
    let request_url = format!("{}/miner/job?user={}", oracle_url, sanitized_user);

    info!("Fetching job from oracle: {}", oracle_url);

    let resp = client
        .get(&request_url)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to oracle: {}", e))?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!(
            "Oracle request failed with status: {}",
            resp.status()
        ));
    }

    let text = resp
        .text()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read oracle response: {}", e))?;

    if text.len() > 1024 * 1024 {
        // 1MB limit
        return Err(anyhow::anyhow!("Oracle response too large"));
    }

    info!("Received oracle response ({} bytes)", text.len());

    let job_response: GetJobResponse = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Failed to parse oracle response: {}", e))?;

    match job_response.job {
        Some(work) => {
            validate_boinc_work(&work)?;
            Ok(work)
        }
        None => {
            let message = job_response
                .message
                .unwrap_or_else(|| "No work available".to_string());
            Err(anyhow::anyhow!("No work available: {}", message))
        }
    }
}

/// Submit completed work result to the PoI Oracle's miner API with security validation
pub async fn submit_result(oracle_url: &str, work: &BoincWork, _result: &str) -> Result<()> {
    // Get rate limiter instance
    let rate_limiter = ORACLE_RATE_LIMITER.get_or_init(|| {
        let rate_limit = std::env::var("CHERT_RATE_LIMIT_PER_MINUTE")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap_or(60);
        RateLimiter::new(rate_limit, Duration::from_secs(60))
    });

    // Check rate limit before making request
    let rate_limit_key = format!("oracle_submit:{}", oracle_url);
    if !rate_limiter.is_allowed(&rate_limit_key).await? {
        let remaining_time = rate_limiter.time_until_reset(&rate_limit_key).await?;
        if let Some(wait_time) = remaining_time {
            warn!(
                "Rate limited for oracle submit requests. Waiting {:?} before retry",
                wait_time
            );
            tokio::time::sleep(wait_time).await;
        }
        // Try again after waiting
        if !rate_limiter.is_allowed(&rate_limit_key).await? {
            return Err(anyhow::anyhow!(
                "Rate limited: too many submit requests to oracle"
            ));
        }
    }

    // Validate inputs
    if oracle_url.is_empty() {
        return Err(anyhow::anyhow!("Oracle URL cannot be empty"));
    }

    // Security validation for HTTPS
    if !oracle_url.starts_with("https://") {
        warn!("Oracle URL is not HTTPS: {}. This is insecure!", oracle_url);
        if std::env::var("CHERT_REQUIRE_HTTPS").unwrap_or_default() == "true" {
            return Err(anyhow::anyhow!(
                "HTTPS is required but oracle URL is HTTP: {}",
                oracle_url
            ));
        }
    }

    validate_boinc_work(work)?;

    let client = create_secure_client()?;

    let request = SubmitWorkRequest {
        user: work.user_id.clone(),
        work: work.clone(),
    };

    let request_url = format!("{}/miner/submit", oracle_url);
    info!("Submitting work result to oracle: {}", oracle_url);

    let resp = client
        .post(&request_url)
        .json(&request)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to submit result to oracle: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let error_text = resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!(
            "Result submission failed with status {}: {}",
            status,
            error_text
        ));
    }

    let submit_response: SubmitWorkResponse = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse submission response: {}", e))?;

    if submit_response.success {
        info!("Work submitted successfully: {}", submit_response.message);
        if let Some(receipt) = submit_response.receipt {
            info!("Receipt: {}", receipt);
        }
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Failed to submit work: {}",
            submit_response.message
        ))
    }
}

/// Demand level from oracle
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DemandResponse {
    #[serde(default)]
    pub current: DemandCurrent,
    #[serde(default)]
    pub saturation: f64,
    #[serde(default)]
    pub recommendation: String,
    #[serde(default)]
    pub eta_empty_seconds: f64,
    #[serde(default)]
    pub historical: Option<DemandHistorical>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DemandCurrent {
    #[serde(default)]
    pub p0: PriorityDemand,
    #[serde(default)]
    pub p1: PriorityDemand,
    #[serde(default)]
    pub p2: PriorityDemand,
    #[serde(default)]
    pub special: PriorityDemand,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PriorityDemand {
    #[serde(default)]
    pub depth: u32,
    #[serde(default)]
    pub rate_in: f64,
    #[serde(default)]
    pub rate_out: f64,
    #[serde(default)]
    pub avg_wait_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DemandHistorical {
    #[serde(default)]
    pub avg_hourly_demand: f64,
    #[serde(default)]
    pub peak_hourly_demand: f64,
    #[serde(default)]
    pub typical_peak_hour: u32,
    #[serde(default)]
    pub total_tasks_24h: f64,
    #[serde(default)]
    pub total_tasks_7d: f64,
}

impl DemandResponse {
    /// Get total pending tasks across all priorities
    pub fn total_depth(&self) -> u32 {
        self.current.p0.depth + self.current.p1.depth + self.current.p2.depth + self.current.special.depth
    }

    /// Get high priority depth (p0 + p1)
    pub fn high_priority_depth(&self) -> u32 {
        self.current.p0.depth + self.current.p1.depth
    }

    /// Get demand level as a score (0-100)
    /// Higher score = more demand = reduce BOINC more
    pub fn demand_score(&self) -> u8 {
        let depth = self.total_depth();
        let high_prio = self.high_priority_depth();
        
        // Base score from depth (0-60)
        let depth_score = ((depth.min(100) as f64) * 0.6) as u8;
        
        // High priority boost (0-30)
        let prio_score = ((high_prio.min(50) as f64) * 0.6) as u8;
        
        // Saturation factor (0-10)
        let sat_score = (self.saturation.min(100.0) * 0.1) as u8;
        
        (depth_score + prio_score + sat_score).min(100)
    }
}

/// Fetch demand from oracle
pub async fn fetch_demand(oracle_url: &str) -> anyhow::Result<DemandResponse> {
    // Validate URL
    if oracle_url.is_empty() {
        return Err(anyhow::anyhow!("Oracle URL cannot be empty"));
    }

    let client = create_secure_client()?;
    let request_url = format!("{}/demand", oracle_url);

    let resp = client
        .get(&request_url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to oracle: {}", e))?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!(
            "Oracle demand request failed with status: {}",
            resp.status()
        ));
    }

    let demand: DemandResponse = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse demand response: {}", e))?;

    Ok(demand)
}

/// Get available work types from oracle (placeholder - could be enhanced)
pub async fn get_available_work_types(oracle_url: &str) -> anyhow::Result<Vec<String>> {
    // For now, return known BOINC project types
    // This could be enhanced to query the actual oracle for supported projects
    let _ = oracle_url; // Avoid unused parameter warning
    Ok(vec![
        "MilkyWay@Home".to_string(),
        "Rosetta@Home".to_string(),
    ])
}
