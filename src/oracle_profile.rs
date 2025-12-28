//! Oracle profile registration and smart task selection integration
//!
//! This module integrates the miner's hardware detection with the oracle's
//! task selection system for intelligent work assignment based on hardware
//! capabilities and user preferences.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::config::{MinerConfig, create_secure_client};
use crate::hardware_detection::{GpuVendor, HardwareDetector, HardwareProfile, HardwareType};
use crate::project_preferences::ProjectCategory;

// ============================================================================
// API Request/Response Types
// ============================================================================

/// Request to register hardware profile with oracle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterProfileRequest {
    pub user: String,
    pub cpu: CpuProfileInput,
    pub gpus: Vec<GpuProfileInput>,
    pub ram_mb: u64,
    pub storage_gb: u64,
    pub os: String,
    pub network_speed_mbps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuProfileInput {
    pub vendor: String,
    pub model: String,
    pub cores: u32,
    pub threads: u32,
    pub base_frequency_mhz: Option<u32>,
    pub features: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuProfileInput {
    pub vendor: String,
    pub model: String,
    pub vram_mb: u32,
    pub compute_capability: Option<String>,
}

/// Response from profile registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterProfileResponse {
    pub success: bool,
    pub message: String,
    pub compatible_projects: Vec<String>,
    pub recommended_project: Option<String>,
}

/// Request to update miner preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub user: String,
    pub preferences: MinerOraclePreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerOraclePreferences {
    pub preferred_projects: Vec<String>,
    pub blocked_projects: Vec<String>,
    pub hardware_capabilities: String,
    pub auto_select_projects: bool,
    pub prefer_gpu_tasks: bool,
    pub prefer_short_tasks: bool,
    pub max_task_duration_hours: Option<u32>,
    pub project_weights: HashMap<String, f64>,
    pub preferred_science_areas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePreferencesResponse {
    pub success: bool,
    pub message: String,
    pub updated_preferences: Option<MinerOraclePreferences>,
}

/// Response from preferences endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferencesResponse {
    pub user: String,
    pub available_projects: Vec<String>,
    pub compatible_projects: Vec<String>,
    pub recommended_projects: Vec<String>,
    pub current_preferences: Option<MinerOraclePreferences>,
}

/// Response from recommendations endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationsResponse {
    pub user: String,
    pub recommendations: Vec<ProjectRecommendation>,
    pub total_compatible: usize,
}

/// Individual project recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRecommendation {
    pub rank: usize,
    pub project_name: String,
    pub score: f64,
    pub estimated_reward: f64,
    pub estimated_duration_hours: f64,
    pub science_area: String,
    pub gpu_required: bool,
    pub compatibility_notes: Vec<String>,
}

/// Response from profile query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResponse {
    pub user: String,
    pub profile: Option<ProfileOutput>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileOutput {
    pub cpu: CpuProfileOutput,
    pub gpus: Vec<GpuProfileOutput>,
    pub ram_mb: u64,
    pub storage_gb: u64,
    pub os: String,
    pub gpu_tier: String,
    pub has_cuda: bool,
    pub total_vram_mb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuProfileOutput {
    pub vendor: String,
    pub model: String,
    pub cores: u32,
    pub threads: u32,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuProfileOutput {
    pub vendor: String,
    pub model: String,
    pub vram_mb: u32,
    pub tier: String,
}

// ============================================================================
// Oracle Profile Client
// ============================================================================

/// Client for interacting with the oracle's task selection system
pub struct OracleProfileClient {
    oracle_url: String,
    user_id: String,
    profile_registered: bool,
    cached_recommendations: Option<Vec<ProjectRecommendation>>,
    last_recommendation_fetch: Option<std::time::Instant>,
}

impl OracleProfileClient {
    /// Create a new oracle profile client
    pub fn new(oracle_url: &str, user_id: &str) -> Self {
        Self {
            oracle_url: oracle_url.trim_end_matches('/').to_string(),
            user_id: user_id.to_string(),
            profile_registered: false,
            cached_recommendations: None,
            last_recommendation_fetch: None,
        }
    }

    /// Create from miner configuration
    pub fn from_config(config: &MinerConfig) -> Self {
        Self::new(&config.oracle_url, &config.user_id)
    }

    /// Check if profile has been registered
    pub fn is_profile_registered(&self) -> bool {
        self.profile_registered
    }

    /// Get cached recommendations
    pub fn get_cached_recommendations(&self) -> Option<&[ProjectRecommendation]> {
        // Cache valid for 5 minutes
        if let Some(last_fetch) = self.last_recommendation_fetch {
            if last_fetch.elapsed() < Duration::from_secs(300) {
                return self.cached_recommendations.as_deref();
            }
        }
        None
    }

    /// Detect hardware and register profile with oracle
    pub async fn register_hardware_profile(&mut self) -> Result<RegisterProfileResponse> {
        info!("Detecting hardware and registering with oracle...");

        // Detect hardware
        let mut detector = HardwareDetector::new();
        let profile = detector.detect_hardware()?;

        self.register_profile(&profile).await
    }

    /// Register a specific hardware profile with the oracle
    pub async fn register_profile(
        &mut self,
        profile: &HardwareProfile,
    ) -> Result<RegisterProfileResponse> {
        let request = convert_to_register_request(&self.user_id, profile);

        let client = create_secure_client()?;
        let url = format!("{}/miner/profile", self.oracle_url);

        info!("Registering hardware profile with oracle at {}", url);
        debug!(
            "Profile: {} cores, {} GPU(s), {} MB RAM",
            request.cpu.cores,
            request.gpus.len(),
            request.ram_mb
        );

        let resp = client
            .post(&url)
            .json(&request)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .context("Failed to connect to oracle for profile registration")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Profile registration failed with status {}: {}",
                status,
                error_text
            ));
        }

        let response: RegisterProfileResponse = resp
            .json()
            .await
            .context("Failed to parse profile registration response")?;

        if response.success {
            self.profile_registered = true;
            info!(
                "Profile registered successfully. {} compatible projects found.",
                response.compatible_projects.len()
            );
            if let Some(ref recommended) = response.recommended_project {
                info!("Recommended project: {}", recommended);
            }
        } else {
            warn!("Profile registration failed: {}", response.message);
        }

        Ok(response)
    }

    /// Get current profile from oracle
    pub async fn get_profile(&self) -> Result<ProfileResponse> {
        let client = create_secure_client()?;
        let url = format!("{}/miner/profile?user={}", self.oracle_url, self.user_id);

        debug!("Fetching profile from oracle");

        let resp = client
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .context("Failed to connect to oracle for profile query")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Profile query failed with status {}: {}",
                status,
                error_text
            ));
        }

        let response: ProfileResponse = resp
            .json()
            .await
            .context("Failed to parse profile response")?;

        Ok(response)
    }

    /// Get project recommendations from oracle
    pub async fn get_recommendations(
        &mut self,
        limit: Option<usize>,
    ) -> Result<Vec<ProjectRecommendation>> {
        // Check cache first
        if let Some(cached) = self.get_cached_recommendations() {
            debug!("Using cached recommendations");
            return Ok(cached.to_vec());
        }

        let client = create_secure_client()?;
        let mut url = format!(
            "{}/miner/recommendations?user={}",
            self.oracle_url, self.user_id
        );
        if let Some(l) = limit {
            url.push_str(&format!("&limit={}", l));
        }

        info!("Fetching recommendations from oracle");

        let resp = client
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .context("Failed to connect to oracle for recommendations")?;

        if !resp.status().is_success() {
            let status = resp.status();
            if status.as_u16() == 404 {
                return Err(anyhow::anyhow!(
                    "Profile not registered. Call register_hardware_profile() first."
                ));
            }
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Recommendations query failed with status {}: {}",
                status,
                error_text
            ));
        }

        let response: RecommendationsResponse = resp
            .json()
            .await
            .context("Failed to parse recommendations response")?;

        info!(
            "Received {} recommendations ({} total compatible)",
            response.recommendations.len(),
            response.total_compatible
        );

        // Cache the recommendations
        self.cached_recommendations = Some(response.recommendations.clone());
        self.last_recommendation_fetch = Some(std::time::Instant::now());

        Ok(response.recommendations)
    }

    /// Get the best recommended project
    pub async fn get_best_project(&mut self) -> Result<Option<ProjectRecommendation>> {
        let recommendations = self.get_recommendations(Some(1)).await?;
        Ok(recommendations.into_iter().next())
    }

    /// Update miner preferences on the oracle
    pub async fn update_preferences(
        &self,
        preferences: MinerOraclePreferences,
    ) -> Result<UpdatePreferencesResponse> {
        let request = UpdatePreferencesRequest {
            user: self.user_id.clone(),
            preferences,
        };

        let client = create_secure_client()?;
        let url = format!("{}/miner/preferences", self.oracle_url);

        info!("Updating preferences on oracle");

        let resp = client
            .post(&url)
            .json(&request)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .context("Failed to connect to oracle for preferences update")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Preferences update failed with status {}: {}",
                status,
                error_text
            ));
        }

        let response: UpdatePreferencesResponse = resp
            .json()
            .await
            .context("Failed to parse preferences update response")?;

        if response.success {
            info!("Preferences updated successfully");
        }

        Ok(response)
    }

    /// Get current preferences from oracle
    pub async fn get_preferences(&self) -> Result<PreferencesResponse> {
        let client = create_secure_client()?;
        let url = format!(
            "{}/miner/preferences?user={}",
            self.oracle_url, self.user_id
        );

        debug!("Fetching preferences from oracle");

        let resp = client
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .context("Failed to connect to oracle for preferences query")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Preferences query failed with status {}: {}",
                status,
                error_text
            ));
        }

        let response: PreferencesResponse = resp
            .json()
            .await
            .context("Failed to parse preferences response")?;

        Ok(response)
    }

    /// Invalidate cached recommendations (call when preferences change)
    pub fn invalidate_cache(&mut self) {
        self.cached_recommendations = None;
        self.last_recommendation_fetch = None;
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert local hardware profile to oracle registration request
fn convert_to_register_request(user_id: &str, profile: &HardwareProfile) -> RegisterProfileRequest {
    let cpu = CpuProfileInput {
        vendor: extract_cpu_vendor(&profile.cpu.vendor_model),
        model: profile.cpu.vendor_model.clone(),
        cores: profile.cpu.physical_cores as u32,
        threads: profile.cpu.logical_threads as u32,
        base_frequency_mhz: Some((profile.cpu.base_frequency * 1000.0) as u32),
        features: Some(profile.cpu.features.clone()),
    };

    let gpus: Vec<GpuProfileInput> = profile
        .gpus
        .iter()
        .map(|gpu| {
            let compute_cap = gpu
                .compute_capability
                .map(|(major, minor)| format!("{}.{}", major, minor));

            GpuProfileInput {
                vendor: convert_gpu_vendor(&gpu.vendor),
                model: gpu.vendor_model.clone(),
                vram_mb: (gpu.total_memory / (1024 * 1024)) as u32,
                compute_capability: compute_cap,
            }
        })
        .collect();

    let os = match profile.system.os_info.name.to_lowercase().as_str() {
        "linux" => "linux",
        "windows" => "windows",
        "macos" | "darwin" => "macos",
        _ => "other",
    }
    .to_string();

    RegisterProfileRequest {
        user: user_id.to_string(),
        cpu,
        gpus,
        ram_mb: profile.system.total_memory / (1024 * 1024),
        storage_gb: profile.system.available_disk_space / (1024 * 1024 * 1024),
        os,
        network_speed_mbps: profile
            .system
            .network_status
            .bandwidth_mbps
            .map(|b| b as u32),
    }
}

/// Extract CPU vendor from vendor_model string
fn extract_cpu_vendor(vendor_model: &str) -> String {
    let lower = vendor_model.to_lowercase();
    if lower.contains("intel") {
        "Intel".to_string()
    } else if lower.contains("amd") {
        "AMD".to_string()
    } else if lower.contains("arm") {
        "ARM".to_string()
    } else if lower.contains("apple") {
        "Apple".to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Convert GPU vendor enum to string
fn convert_gpu_vendor(vendor: &GpuVendor) -> String {
    match vendor {
        GpuVendor::Nvidia => "nvidia".to_string(),
        GpuVendor::Amd => "amd".to_string(),
        GpuVendor::Intel => "intel".to_string(),
        GpuVendor::Unknown => "unknown".to_string(),
    }
}

/// Convert project category to science area string
pub fn category_to_science_area(category: &ProjectCategory) -> String {
    match category {
        ProjectCategory::Astronomy => "astronomy".to_string(),
        ProjectCategory::Medical => "medicine".to_string(),
        ProjectCategory::Physics => "physics".to_string(),
        ProjectCategory::Mathematics => "mathematics".to_string(),
        ProjectCategory::Biology => "biology".to_string(),
        ProjectCategory::Climate => "climate".to_string(),
        ProjectCategory::ComputerScience => "machinelearning".to_string(),
        ProjectCategory::Other(name) => name.to_lowercase(),
    }
}

/// Convert local preferences to oracle preferences format
pub fn convert_preferences_to_oracle(
    config: &MinerConfig,
    profile: &HardwareProfile,
) -> MinerOraclePreferences {
    let hardware_capabilities = match profile.hardware_type {
        HardwareType::CpuOnly => "cpu_only".to_string(),
        HardwareType::GpuOnly => "gpu_only".to_string(),
        HardwareType::Both => "cpu_and_gpu".to_string(),
        HardwareType::Unknown => "unknown".to_string(),
    };

    let preferred_science_areas: Vec<String> = config
        .preferences
        .preferred_science_areas
        .iter()
        .map(|area| area.to_lowercase())
        .collect();

    MinerOraclePreferences {
        preferred_projects: config.preferences.preferred_projects.clone(),
        blocked_projects: Vec::new(), // Could be added to config
        hardware_capabilities,
        auto_select_projects: config.preferences.auto_select_projects,
        prefer_gpu_tasks: !profile.gpus.is_empty(),
        prefer_short_tasks: false,     // Could be added to config
        max_task_duration_hours: None, // Could be added to config
        project_weights: config.preferences.project_weights.clone(),
        preferred_science_areas,
    }
}

// ============================================================================
// Integration Manager
// ============================================================================

/// High-level manager for oracle profile integration
pub struct OracleProfileManager {
    client: OracleProfileClient,
    hardware_profile: Option<HardwareProfile>,
    auto_register: bool,
}

impl OracleProfileManager {
    /// Create a new profile manager
    pub fn new(config: &MinerConfig) -> Self {
        Self {
            client: OracleProfileClient::from_config(config),
            hardware_profile: None,
            auto_register: true,
        }
    }

    /// Initialize the manager - detect hardware and register with oracle
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing oracle profile manager...");

        // Detect hardware
        let mut detector = HardwareDetector::new();
        let profile = detector.detect_hardware()?;

        info!(
            "Hardware detected: {} cores, {} GPU(s), {:.1} GB RAM",
            profile.cpu.physical_cores,
            profile.gpus.len(),
            profile.system.total_memory as f64 / (1024.0 * 1024.0 * 1024.0)
        );

        self.hardware_profile = Some(profile.clone());

        // Auto-register with oracle if enabled
        if self.auto_register {
            match self.client.register_profile(&profile).await {
                Ok(response) => {
                    if response.success {
                        info!("Successfully registered with oracle");
                    } else {
                        warn!("Oracle registration returned failure: {}", response.message);
                    }
                }
                Err(e) => {
                    warn!("Failed to register with oracle: {}. Will retry later.", e);
                    // Don't fail initialization - oracle might be temporarily unavailable
                }
            }
        }

        Ok(())
    }

    /// Get the hardware profile
    pub fn hardware_profile(&self) -> Option<&HardwareProfile> {
        self.hardware_profile.as_ref()
    }

    /// Check if registered with oracle
    pub fn is_registered(&self) -> bool {
        self.client.is_profile_registered()
    }

    /// Get recommendations (with fallback to local selection if oracle unavailable)
    pub async fn get_recommendations(&mut self) -> Result<Vec<ProjectRecommendation>> {
        // Try oracle first
        match self.client.get_recommendations(None).await {
            Ok(recommendations) => Ok(recommendations),
            Err(e) => {
                warn!(
                    "Failed to get oracle recommendations: {}. Using local selection.",
                    e
                );
                // Return empty list - caller should fall back to local selection
                Ok(Vec::new())
            }
        }
    }

    /// Get the best project recommendation
    pub async fn get_best_project(&mut self) -> Result<Option<String>> {
        let recommendations = self.get_recommendations().await?;
        Ok(recommendations.into_iter().next().map(|r| r.project_name))
    }

    /// Update preferences on oracle
    pub async fn sync_preferences(&self, config: &MinerConfig) -> Result<()> {
        if let Some(profile) = &self.hardware_profile {
            let preferences = convert_preferences_to_oracle(config, profile);
            self.client.update_preferences(preferences).await?;
        }
        Ok(())
    }

    /// Force re-registration with oracle
    pub async fn re_register(&mut self) -> Result<RegisterProfileResponse> {
        if let Some(profile) = &self.hardware_profile {
            self.client.register_profile(profile).await
        } else {
            self.client.register_hardware_profile().await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware_detection::*;

    fn create_test_profile() -> HardwareProfile {
        HardwareProfile {
            cpu: CpuProfile {
                vendor_model: "Intel Core i7-12700K".to_string(),
                physical_cores: 12,
                logical_threads: 20,
                base_frequency: 3.6,
                performance_score: 75.0,
                features: vec!["AVX2".to_string(), "SSE4.2".to_string()],
                ..Default::default()
            },
            gpus: vec![GpuProfile {
                vendor_model: "NVIDIA GeForce RTX 3080".to_string(),
                vendor: GpuVendor::Nvidia,
                total_memory: 10 * 1024 * 1024 * 1024, // 10 GB
                compute_capability: Some((8, 6)),
                performance_score: 85.0,
                ..Default::default()
            }],
            system: SystemInfo {
                total_memory: 32 * 1024 * 1024 * 1024,          // 32 GB
                available_disk_space: 500 * 1024 * 1024 * 1024, // 500 GB
                os_info: OsInfo {
                    name: "Linux".to_string(),
                    ..Default::default()
                },
                network_status: NetworkStatus {
                    connectivity: true,
                    bandwidth_mbps: Some(100.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            hardware_type: HardwareType::Both,
            ..Default::default()
        }
    }

    #[test]
    fn test_convert_to_register_request() {
        let profile = create_test_profile();
        let request = convert_to_register_request("test_user", &profile);

        assert_eq!(request.user, "test_user");
        assert_eq!(request.cpu.vendor, "Intel");
        assert_eq!(request.cpu.cores, 12);
        assert_eq!(request.cpu.threads, 20);
        assert_eq!(request.gpus.len(), 1);
        assert_eq!(request.gpus[0].vendor, "nvidia");
        assert_eq!(request.gpus[0].vram_mb, 10240); // 10 GB in MB
        assert_eq!(request.gpus[0].compute_capability, Some("8.6".to_string()));
        assert_eq!(request.os, "linux");
    }

    #[test]
    fn test_extract_cpu_vendor() {
        assert_eq!(extract_cpu_vendor("Intel Core i7-12700K"), "Intel");
        assert_eq!(extract_cpu_vendor("AMD Ryzen 9 5900X"), "AMD");
        assert_eq!(extract_cpu_vendor("Apple M1 Max"), "Apple");
        assert_eq!(extract_cpu_vendor("ARM Cortex-A78"), "ARM");
        assert_eq!(extract_cpu_vendor("Unknown Processor"), "Unknown");
    }

    #[test]
    fn test_convert_gpu_vendor() {
        assert_eq!(convert_gpu_vendor(&GpuVendor::Nvidia), "nvidia");
        assert_eq!(convert_gpu_vendor(&GpuVendor::Amd), "amd");
        assert_eq!(convert_gpu_vendor(&GpuVendor::Intel), "intel");
        assert_eq!(convert_gpu_vendor(&GpuVendor::Unknown), "unknown");
    }

    #[test]
    fn test_category_to_science_area() {
        assert_eq!(
            category_to_science_area(&ProjectCategory::Astronomy),
            "astronomy"
        );
        assert_eq!(
            category_to_science_area(&ProjectCategory::Medical),
            "medicine"
        );
        assert_eq!(
            category_to_science_area(&ProjectCategory::Biology),
            "biology"
        );
        assert_eq!(
            category_to_science_area(&ProjectCategory::Other("Quantum".to_string())),
            "quantum"
        );
    }
}
