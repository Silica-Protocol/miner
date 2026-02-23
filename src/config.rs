use crate::hardware_detection::HardwareType;
use anyhow::{Context, Result};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use silica_models::boinc::BoincWork;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{info, warn};

/// Configuration for the Chert miner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerConfig {
    /// Oracle URL for PoI operations
    pub oracle_url: String,
    /// Timeout for oracle requests in seconds
    pub oracle_timeout_secs: u64,
    /// BOINC installation directory
    pub boinc_install_dir: PathBuf,
    /// BOINC data directory
    pub boinc_data_dir: PathBuf,
    /// BOINC log file path
    pub boinc_log_file: PathBuf,
    /// Miner account address (for payment)
    pub account_address: String,
    /// Miner worker name (like traditional PoW miner worker names)
    pub worker_name: String,
    /// Miner user identifier (deprecated, use account_address + worker_name)
    pub user_id: String,
    /// Miner operation mode
    pub mode: MinerMode,
    /// Work allocation preferences
    pub work_allocation: WorkAllocationConfig,
    /// Project preferences (alias: project_preferences)
    #[serde(alias = "project_preferences")]
    pub preferences: ProjectPreferencesConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Debug settings
    pub debug: DebugConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MinerMode {
    Traditional,
    Tui,
}

/// Work allocation preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkAllocationConfig {
    /// Enable NUW mining on CPU
    pub nuw_on_cpu: bool,
    /// Enable BOINC processing on GPU
    pub boinc_on_gpu: bool,
    /// Percentage of CPU cores to allocate to NUW (0-100)
    pub nuw_cpu_percentage: u8,
    /// Percentage of CPU cores to allocate to BOINC (0-100)
    pub boinc_cpu_percentage: u8,
    /// Percentage of GPU resources to allocate to BOINC (0-100)
    pub boinc_gpu_percentage: u8,
    /// Low CPU mode - limits CPU usage for casual use (0-100)
    /// When enabled, caps total CPU and lowers process priority
    pub low_cpu_mode: bool,
    /// CPU limit percentage when low_cpu_mode is enabled (default 70%)
    pub low_cpu_limit: u8,
    /// Enable on-demand NUW tasks (user can choose when to run)
    pub nuw_on_demand: bool,
    /// Minimum NUW difficulty threshold to accept
    pub min_nuw_difficulty: u32,
    /// Maximum concurrent BOINC tasks
    pub max_boinc_tasks: u8,
    /// Hardware capability type (auto-detected or manual)
    pub hardware_capabilities: HardwareType,
    /// Auto-detect hardware capabilities on startup
    pub auto_detect_hardware: bool,
    /// Dynamic throttling: reduce BOINC by this % when NUW active (default 20)
    pub throttling_reduce_rate: u8,
    /// Dynamic throttling: recover BOINC by this % when NUW idle (default 5)
    pub throttling_recover_rate: u8,
    /// Dynamic throttling: seconds of NUW idle before BOINC recovers (default 30)
    pub throttling_recover_delay_secs: u64,
    /// Dynamic throttling: minimum BOINC CPU % (default 20)
    pub throttling_min_boinc_cpu: u8,
}

/// Project preferences configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPreferencesConfig {
    /// Preferred BOINC projects (in order of priority)
    pub preferred_projects: Vec<String>,
    /// Hardware capability type for project matching
    pub hardware_capabilities: HardwareType,
    /// Automatically select projects based on hardware
    pub auto_select_projects: bool,
    /// Project weights for prioritization (project_name -> weight)
    pub project_weights: HashMap<String, f64>,
    /// Minimum project priority threshold
    pub min_project_priority: u8,
    /// Maximum number of concurrent projects
    pub max_concurrent_projects: u8,
    /// Preferred science areas (astronomy, biology, medicine, etc.)
    pub preferred_science_areas: Vec<String>,
    /// Projects to block (never select)
    pub blocked_projects: Vec<String>,
    /// Project switching preferences
    pub switching: ProjectSwitchingConfig,
}

/// Project switching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSwitchingConfig {
    /// Enable automatic project switching
    pub auto_switch: bool,
    /// Minimum time before switching projects (seconds)
    pub min_run_time_seconds: u64,
    /// Switch based on performance metrics
    pub performance_based_switching: bool,
    /// Switch based on reward rates
    pub reward_based_switching: bool,
    /// Maximum switch attempts per hour
    pub max_switches_per_hour: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Require HTTPS for all external communications
    pub require_https: bool,
    /// Verify TLS certificates
    pub verify_certificates: bool,
    /// Rate limit for API requests (per minute)
    pub rate_limit_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebugConfig {
    /// Enable debug mode
    pub debug_mode: bool,
    /// Enable verbose logging
    pub verbose_logging: bool,
}

impl Default for MinerConfig {
    fn default() -> Self {
        // Default to local directory relative to executable
        let local_boinc = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            oracle_url: "https://oracle.silicaprotocol.network".to_string(),
            oracle_timeout_secs: 30,
            // Use local directory by default (same as miner binary)
            boinc_install_dir: local_boinc.join("boinc"),
            boinc_data_dir: local_boinc.join("boinc_data"),
            boinc_log_file: local_boinc.join("boinc.log"),
            account_address: "".to_string(),
            worker_name: "default".to_string(),
            user_id: "".to_string(),
            mode: MinerMode::Traditional,
            work_allocation: WorkAllocationConfig::default(),
            preferences: ProjectPreferencesConfig::default(),
            security: SecurityConfig::default(),
            debug: DebugConfig::default(),
        }
    }
}

impl Default for WorkAllocationConfig {
    fn default() -> Self {
        Self {
            nuw_on_cpu: false,                            // Default to BOINC on CPU
            boinc_on_gpu: true,                           // Default to BOINC on GPU
            nuw_cpu_percentage: 25,                       // Use 25% of CPU for NUW when enabled
            boinc_cpu_percentage: 50,                     // Use 50% of CPU for BOINC
            boinc_gpu_percentage: 75,                     // Use 75% of GPU for BOINC
            low_cpu_mode: false,                          // Default to full power (dedicated mode)
            low_cpu_limit: 70,                            // 70% CPU limit when low_cpu_mode enabled
            nuw_on_demand: true,                          // NUW tasks on-demand by default
            min_nuw_difficulty: 1000,                     // Minimum difficulty threshold
            max_boinc_tasks: 2,                           // Maximum concurrent BOINC tasks
            hardware_capabilities: HardwareType::Unknown, // Will be auto-detected
            auto_detect_hardware: true,                   // Auto-detect on startup
            throttling_reduce_rate: 20,                   // Reduce BOINC by 20% when NUW active
            throttling_recover_rate: 5,                   // Recover BOINC by 5% when NUW idle
            throttling_recover_delay_secs: 30,            // Wait 30s before recovering
            throttling_min_boinc_cpu: 20,                 // Minimum 20% BOINC CPU
        }
    }
}

impl Default for ProjectSwitchingConfig {
    fn default() -> Self {
        Self {
            auto_switch: true,
            min_run_time_seconds: 3600, // 1 hour
            performance_based_switching: true,
            reward_based_switching: false,
            max_switches_per_hour: 2,
        }
    }
}

impl Default for ProjectPreferencesConfig {
    fn default() -> Self {
        Self {
            preferred_projects: vec!["MilkyWay@Home".to_string(), "Rosetta@Home".to_string()],
            hardware_capabilities: HardwareType::Unknown,
            auto_select_projects: true,
            project_weights: HashMap::new(),
            min_project_priority: 1,
            max_concurrent_projects: 2,
            preferred_science_areas: Vec::new(),
            blocked_projects: Vec::new(),
            switching: ProjectSwitchingConfig::default(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_https: true,
            verify_certificates: true,
            rate_limit_per_minute: 60,
        }
    }
}

impl MinerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        // Oracle configuration
        if let Ok(url) = env::var("CHERT_ORACLE_URL") {
            // Validate URL scheme for security
            if config.security.require_https && !url.starts_with("https://") {
                return Err(anyhow::anyhow!(
                    "HTTPS is required but oracle URL is not HTTPS: {}",
                    url
                ));
            }
            config.oracle_url = url;
        }

        if let Ok(timeout) = env::var("CHERT_ORACLE_TIMEOUT_SECS") {
            config.oracle_timeout_secs = timeout.parse()?;
        }

        // BOINC configuration
        if let Ok(install_dir) = env::var("CHERT_BOINC_INSTALL_DIR") {
            config.boinc_install_dir = PathBuf::from(install_dir);
        }

        if let Ok(data_dir) = env::var("CHERT_BOINC_DATA_DIR") {
            config.boinc_data_dir = PathBuf::from(data_dir);
        }

        if let Ok(log_file) = env::var("CHERT_BOINC_LOG_FILE") {
            config.boinc_log_file = PathBuf::from(log_file);
        }

        // Miner configuration
        if let Ok(user_id) = env::var("CHERT_MINER_USER_ID") {
            // Validate user ID format
            if user_id.is_empty() || user_id.len() > 64 {
                return Err(anyhow::anyhow!("Invalid user ID: must be 1-64 characters"));
            }
            // Basic sanitization - only allow alphanumeric and underscores
            if !user_id.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Err(anyhow::anyhow!(
                    "Invalid user ID: only alphanumeric characters and underscores allowed"
                ));
            }
            config.user_id = user_id;
        } else {
            return Err(anyhow::anyhow!(
                "CHERT_MINER_USER_ID environment variable is required"
            ));
        }

        // Account address for payments (required for NUW)
        if let Ok(account) = env::var("CHERT_MINER_ACCOUNT") {
            if account.is_empty() {
                return Err(anyhow::anyhow!("Account address cannot be empty"));
            }
            config.account_address = account;
        } else {
            return Err(anyhow::anyhow!(
                "CHERT_MINER_ACCOUNT is required for NUW mining"
            ));
        }

        // Worker name (like traditional PoW miners)
        if let Ok(worker) = env::var("CHERT_MINER_WORKER") {
            if worker.is_empty() || worker.len() > 32 {
                return Err(anyhow::anyhow!("Worker name must be 1-32 characters"));
            }
            config.worker_name = worker;
        } else {
            config.worker_name = "default".to_string();
        }

        // Work allocation configuration
        if let Ok(nuw_on_cpu) = env::var("CHERT_NUW_ON_CPU") {
            config.work_allocation.nuw_on_cpu = nuw_on_cpu.parse()?;
        }

        if let Ok(boinc_on_gpu) = env::var("CHERT_BOINC_ON_GPU") {
            config.work_allocation.boinc_on_gpu = boinc_on_gpu.parse()?;
        }

        if let Ok(nuw_cpu_percentage) = env::var("CHERT_NUW_CPU_PERCENTAGE") {
            let percentage = nuw_cpu_percentage.parse::<u8>()?;
            if percentage > 100 {
                return Err(anyhow::anyhow!("NUW CPU percentage cannot exceed 100%"));
            }
            config.work_allocation.nuw_cpu_percentage = percentage;
        }

        if let Ok(boinc_gpu_percentage) = env::var("CHERT_BOINC_GPU_PERCENTAGE") {
            let percentage = boinc_gpu_percentage.parse::<u8>()?;
            if percentage > 100 {
                return Err(anyhow::anyhow!("BOINC GPU percentage cannot exceed 100%"));
            }
            config.work_allocation.boinc_gpu_percentage = percentage;
        }

        // BOINC CPU percentage
        if let Ok(boinc_cpu_percentage) = env::var("CHERT_BOINC_CPU_PERCENTAGE") {
            let percentage = boinc_cpu_percentage.parse::<u8>()?;
            if percentage > 100 {
                return Err(anyhow::anyhow!("BOINC CPU percentage cannot exceed 100%"));
            }
            config.work_allocation.boinc_cpu_percentage = percentage;
        }

        // Low CPU mode
        if let Ok(low_cpu_mode) = env::var("CHERT_LOW_CPU_MODE") {
            config.work_allocation.low_cpu_mode = low_cpu_mode == "1" || low_cpu_mode == "true";
        }

        // Low CPU limit
        if let Ok(low_cpu_limit) = env::var("CHERT_LOW_CPU_LIMIT") {
            let limit = low_cpu_limit.parse::<u8>()?;
            if limit > 100 {
                return Err(anyhow::anyhow!("Low CPU limit cannot exceed 100%"));
            }
            config.work_allocation.low_cpu_limit = limit;
        }

        if let Ok(nuw_on_demand) = env::var("CHERT_NUW_ON_DEMAND") {
            config.work_allocation.nuw_on_demand = nuw_on_demand.parse()?;
        }

        if let Ok(min_nuw_difficulty) = env::var("CHERT_MIN_NUW_DIFFICULTY") {
            config.work_allocation.min_nuw_difficulty = min_nuw_difficulty.parse()?;
        }

        if let Ok(max_boinc_tasks) = env::var("CHERT_MAX_BOINC_TASKS") {
            let tasks = max_boinc_tasks.parse::<u8>()?;
            if tasks == 0 {
                return Err(anyhow::anyhow!("Maximum BOINC tasks must be at least 1"));
            }
            config.work_allocation.max_boinc_tasks = tasks;
        }

        // Hardware capabilities configuration
        if let Ok(hardware_caps) = env::var("CHERT_HARDWARE_CAPABILITIES") {
            config.work_allocation.hardware_capabilities =
                match hardware_caps.to_lowercase().as_str() {
                    "cpu_only" => HardwareType::CpuOnly,
                    "gpu_only" => HardwareType::GpuOnly,
                    "both" => HardwareType::Both,
                    _ => HardwareType::Unknown,
                };
        }

        if let Ok(auto_detect) = env::var("CHERT_AUTO_DETECT_HARDWARE") {
            config.work_allocation.auto_detect_hardware = auto_detect.parse()?;
        }

        // Project preferences configuration
        if let Ok(preferred_projects) = env::var("CHERT_PREFERRED_PROJECTS") {
            config.preferences.preferred_projects = preferred_projects
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        if let Ok(auto_select) = env::var("CHERT_AUTO_SELECT_PROJECTS") {
            config.preferences.auto_select_projects = auto_select.parse()?;
        }

        if let Ok(min_priority) = env::var("CHERT_MIN_PROJECT_PRIORITY") {
            config.preferences.min_project_priority = min_priority.parse()?;
        }

        if let Ok(max_projects) = env::var("CHERT_MAX_CONCURRENT_PROJECTS") {
            config.preferences.max_concurrent_projects = max_projects.parse()?;
        }

        // Project switching configuration
        if let Ok(auto_switch) = env::var("CHERT_AUTO_SWITCH_PROJECTS") {
            config.preferences.switching.auto_switch = auto_switch.parse()?;
        }

        if let Ok(min_run_time) = env::var("CHERT_MIN_PROJECT_RUN_TIME") {
            config.preferences.switching.min_run_time_seconds = min_run_time.parse()?;
        }

        if let Ok(perf_switching) = env::var("CHERT_PERFORMANCE_BASED_SWITCHING") {
            config.preferences.switching.performance_based_switching = perf_switching.parse()?;
        }

        if let Ok(reward_switching) = env::var("CHERT_REWARD_BASED_SWITCHING") {
            config.preferences.switching.reward_based_switching = reward_switching.parse()?;
        }

        if let Ok(max_switches) = env::var("CHERT_MAX_SWITCHES_PER_HOUR") {
            config.preferences.switching.max_switches_per_hour = max_switches.parse()?;
        }

        if let Ok(mode) = env::var("CHERT_MINER_MODE") {
            config.mode = match mode.to_lowercase().as_str() {
                "traditional" => MinerMode::Traditional,
                "tui" => MinerMode::Tui,
                _ => return Err(anyhow::anyhow!("Invalid miner mode: {}", mode)),
            };
        }

        // Security configuration
        if let Ok(require_https) = env::var("CHERT_REQUIRE_HTTPS") {
            config.security.require_https = require_https.parse()?;
        }

        if let Ok(verify_certs) = env::var("CHERT_VERIFY_CERTIFICATES") {
            config.security.verify_certificates = verify_certs.parse()?;
        }

        if let Ok(rate_limit) = env::var("CHERT_RATE_LIMIT_REQUESTS_PER_MINUTE") {
            let rate = rate_limit.parse::<u32>()?;
            if rate > 1000 {
                warn!(
                    "Rate limit {} is very high, consider lowering for security",
                    rate
                );
            }
            config.security.rate_limit_per_minute = rate;
        }

        // Debug configuration
        if let Ok(debug_mode) = env::var("CHERT_DEBUG_MODE") {
            config.debug.debug_mode = debug_mode.parse()?;
        }

        if let Ok(verbose) = env::var("CHERT_VERBOSE_LOGGING") {
            config.debug.verbose_logging = verbose.parse()?;
        }

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration for security and correctness
    pub fn validate(&self) -> Result<()> {
        // Basic validation
        self.validate_basic_configuration()?;

        // Security validation
        self.validate_security_configuration()?;

        // Work allocation validation
        self.validate_work_allocation()?;

        // Project preferences validation
        self.validate_project_preferences()?;

        // Hardware compatibility validation
        self.validate_hardware_compatibility()?;

        // Path validation
        self.validate_paths()?;

        // Performance validation
        self.validate_performance_settings()?;

        info!("Miner configuration validated successfully");
        Ok(())
    }

    /// Validate basic configuration requirements
    fn validate_basic_configuration(&self) -> Result<()> {
        // Validate oracle URL
        if self.oracle_url.is_empty() {
            return Err(anyhow::anyhow!("Oracle URL cannot be empty"));
        }

        // Validate user ID
        if self.user_id.is_empty() {
            return Err(anyhow::anyhow!("User ID cannot be empty"));
        }

        if self.user_id.len() > 64 {
            return Err(anyhow::anyhow!("User ID too long (max 64 characters)"));
        }

        // Validate user ID format
        if !self
            .user_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(anyhow::anyhow!(
                "User ID contains invalid characters (only alphanumeric, underscore, and dash allowed)"
            ));
        }

        Ok(())
    }

    /// Validate security configuration
    fn validate_security_configuration(&self) -> Result<()> {
        // HTTPS validation
        if self.security.require_https && !self.oracle_url.starts_with("https://") {
            return Err(anyhow::anyhow!(
                "HTTPS required but oracle URL is not HTTPS: {}",
                self.oracle_url
            ));
        }

        // Rate limit validation
        if self.security.rate_limit_per_minute == 0 {
            return Err(anyhow::anyhow!(
                "Rate limit must be at least 1 request per minute"
            ));
        }

        if self.security.rate_limit_per_minute > 1000 {
            warn!(
                "Rate limit {} is very high, consider lowering for security",
                self.security.rate_limit_per_minute
            );
        }

        // Certificate validation warning
        if !self.security.verify_certificates {
            warn!("Certificate verification is disabled - this is insecure for production");
        }

        Ok(())
    }

    /// Validate work allocation configuration
    fn validate_work_allocation(&self) -> Result<()> {
        let work_alloc = &self.work_allocation;

        // Percentage validation
        if work_alloc.nuw_cpu_percentage > 100 {
            return Err(anyhow::anyhow!("NUW CPU percentage cannot exceed 100%"));
        }

        if work_alloc.boinc_gpu_percentage > 100 {
            return Err(anyhow::anyhow!("BOINC GPU percentage cannot exceed 100%"));
        }

        // Resource allocation validation
        if work_alloc.nuw_cpu_percentage + work_alloc.boinc_gpu_percentage > 100 {
            return Err(anyhow::anyhow!(
                "Total resource allocation cannot exceed 100% (NUW: {}% + BOINC: {}% = {}%)",
                work_alloc.nuw_cpu_percentage,
                work_alloc.boinc_gpu_percentage,
                work_alloc.nuw_cpu_percentage + work_alloc.boinc_gpu_percentage
            ));
        }

        // BOINC tasks validation
        if work_alloc.max_boinc_tasks == 0 {
            return Err(anyhow::anyhow!("Maximum BOINC tasks must be at least 1"));
        }

        if work_alloc.max_boinc_tasks > 20 {
            warn!(
                "Maximum BOINC tasks {} is very high, may cause system instability",
                work_alloc.max_boinc_tasks
            );
        }

        // Difficulty validation
        if work_alloc.min_nuw_difficulty == 0 {
            return Err(anyhow::anyhow!("Minimum NUW difficulty must be at least 1"));
        }

        if work_alloc.min_nuw_difficulty > 1000000 {
            warn!(
                "Minimum NUW difficulty {} is very high, may result in no work",
                work_alloc.min_nuw_difficulty
            );
        }

        // Hardware type validation
        self.validate_hardware_type_consistency()?;

        // Resource conflict warnings
        if work_alloc.nuw_on_cpu && work_alloc.boinc_on_gpu {
            warn!("Both NUW on CPU and BOINC on GPU enabled - monitor for resource conflicts");
        }

        if work_alloc.nuw_on_cpu && work_alloc.nuw_cpu_percentage > 75 {
            warn!(
                "NUW CPU percentage {}% is high, may impact system responsiveness",
                work_alloc.nuw_cpu_percentage
            );
        }

        if work_alloc.boinc_on_gpu && work_alloc.boinc_gpu_percentage > 90 {
            warn!(
                "BOINC GPU percentage {}% is high, may cause thermal issues",
                work_alloc.boinc_gpu_percentage
            );
        }

        Ok(())
    }

    /// Validate project preferences configuration
    fn validate_project_preferences(&self) -> Result<()> {
        let prefs = &self.preferences;

        // Preferred projects validation
        if prefs.preferred_projects.is_empty() && !prefs.auto_select_projects {
            warn!(
                "No preferred projects specified and auto-selection disabled - may result in no work"
            );
        }

        // Validate project names
        for project in &prefs.preferred_projects {
            if project.is_empty() {
                return Err(anyhow::anyhow!("Project name cannot be empty"));
            }
            if project.len() > 128 {
                return Err(anyhow::anyhow!(
                    "Project name too long (max 128 characters): {}",
                    project
                ));
            }
        }

        // Validate project weights
        for (project, weight) in &prefs.project_weights {
            if *weight < 0.0 {
                return Err(anyhow::anyhow!(
                    "Project weight cannot be negative: {}",
                    project
                ));
            }
            if *weight > 10.0 {
                warn!(
                    "Project weight {} for {} is very high, may skew selection",
                    weight, project
                );
            }
        }

        // Validate concurrent projects
        if prefs.max_concurrent_projects == 0 {
            return Err(anyhow::anyhow!(
                "Maximum concurrent projects must be at least 1"
            ));
        }

        if prefs.max_concurrent_projects > 10 {
            warn!(
                "Maximum concurrent projects {} is very high, may cause resource contention",
                prefs.max_concurrent_projects
            );
        }

        // Validate switching configuration
        self.validate_switching_configuration(&prefs.switching)?;

        Ok(())
    }

    /// Validate project switching configuration
    fn validate_switching_configuration(&self, switching: &ProjectSwitchingConfig) -> Result<()> {
        // Minimum run time validation
        if switching.min_run_time_seconds == 0 {
            return Err(anyhow::anyhow!(
                "Minimum project run time must be at least 1 second"
            ));
        }

        if switching.min_run_time_seconds > 86400 {
            warn!(
                "Minimum project run time {} seconds (24 hours) is very long",
                switching.min_run_time_seconds
            );
        }

        // Maximum switches per hour validation
        if switching.max_switches_per_hour == 0 {
            return Err(anyhow::anyhow!(
                "Maximum switches per hour must be at least 1"
            ));
        }

        if switching.max_switches_per_hour > 60 {
            warn!(
                "Maximum switches per hour {} is excessive, may cause instability",
                switching.max_switches_per_hour
            );
        }

        // Switching logic validation
        let switching_methods =
            switching.performance_based_switching as u8 + switching.reward_based_switching as u8;
        if switching_methods == 0 && switching.auto_switch {
            warn!("Automatic switching enabled but no switching criteria specified");
        }

        Ok(())
    }

    /// Validate hardware compatibility
    fn validate_hardware_compatibility(&self) -> Result<()> {
        let work_alloc = &self.work_allocation;
        let prefs = &self.preferences;

        // Check for logical inconsistencies
        match (
            work_alloc.hardware_capabilities.clone(),
            prefs.hardware_capabilities.clone(),
        ) {
            (HardwareType::CpuOnly, HardwareType::GpuOnly) => {
                return Err(anyhow::anyhow!(
                    "Inconsistent hardware types: work allocation CPU-only, project preferences GPU-only"
                ));
            }
            (HardwareType::GpuOnly, HardwareType::CpuOnly) => {
                return Err(anyhow::anyhow!(
                    "Inconsistent hardware types: work allocation GPU-only, project preferences CPU-only"
                ));
            }
            _ => {} // Consistent
        }

        // Validate auto-detection consistency
        if work_alloc.auto_detect_hardware
            && work_alloc.hardware_capabilities == HardwareType::Unknown
        {
            warn!("Hardware auto-detection enabled but hardware type is Unknown");
        }

        Ok(())
    }

    /// Validate hardware type consistency
    fn validate_hardware_type_consistency(&self) -> Result<()> {
        let work_alloc = &self.work_allocation;

        // Check for logical inconsistencies in work allocation
        if work_alloc.hardware_capabilities == HardwareType::CpuOnly && work_alloc.boinc_on_gpu {
            return Err(anyhow::anyhow!(
                "Cannot enable BOINC on GPU with CPU-only hardware type"
            ));
        }

        if work_alloc.hardware_capabilities == HardwareType::GpuOnly && work_alloc.nuw_on_cpu {
            return Err(anyhow::anyhow!(
                "Cannot enable NUW on CPU with GPU-only hardware type"
            ));
        }

        Ok(())
    }

    /// Validate file paths
    fn validate_paths(&self) -> Result<()> {
        // Validate BOINC install directory
        if let Some(parent) = self.boinc_install_dir.parent() {
            if !parent.exists() {
                return Err(anyhow::anyhow!(
                    "Parent directory for BOINC install does not exist: {:?}",
                    parent
                ));
            }
        }

        // Validate BOINC data directory
        if let Some(parent) = self.boinc_data_dir.parent() {
            if !parent.exists() {
                return Err(anyhow::anyhow!(
                    "Parent directory for BOINC data does not exist: {:?}",
                    parent
                ));
            }
        }

        // Validate BOINC log file directory
        if let Some(parent) = self.boinc_log_file.parent() {
            if !parent.exists() {
                return Err(anyhow::anyhow!(
                    "Parent directory for BOINC log does not exist: {:?}",
                    parent
                ));
            }
        }

        // Check for path conflicts
        if self.boinc_install_dir == self.boinc_data_dir {
            return Err(anyhow::anyhow!(
                "BOINC install and data directories cannot be the same"
            ));
        }

        Ok(())
    }

    /// Validate performance settings
    fn validate_performance_settings(&self) -> Result<()> {
        // Debug mode warnings
        if self.debug.debug_mode {
            warn!("Debug mode is enabled - disable for production");
        }

        if self.debug.verbose_logging {
            warn!("Verbose logging enabled - may impact performance");
        }

        // Oracle timeout validation
        if self.oracle_timeout_secs == 0 {
            return Err(anyhow::anyhow!("Oracle timeout must be at least 1 second"));
        }

        if self.oracle_timeout_secs > 300 {
            warn!(
                "Oracle timeout {} seconds is very long, may cause delays",
                self.oracle_timeout_secs
            );
        }

        Ok(())
    }

    /// Validate configuration against hardware profile
    pub fn validate_with_hardware(
        &self,
        hardware: &crate::hardware_detection::HardwareProfile,
    ) -> Result<()> {
        let work_alloc = &self.work_allocation;

        // Validate resource allocation against available hardware
        if work_alloc.hardware_capabilities == HardwareType::CpuOnly && !hardware.gpus.is_empty() {
            warn!(
                "Configuration specifies CPU-only but GPU is available - consider enabling GPU work"
            );
        }

        if work_alloc.hardware_capabilities == HardwareType::GpuOnly && hardware.gpus.is_empty() {
            return Err(anyhow::anyhow!(
                "Configuration requires GPU but no GPU detected in system"
            ));
        }

        // Validate BOINC tasks against CPU cores
        if work_alloc.max_boinc_tasks as usize > hardware.cpu.physical_cores {
            warn!(
                "Maximum BOINC tasks ({}) exceeds available CPU cores ({})",
                work_alloc.max_boinc_tasks, hardware.cpu.physical_cores
            );
        }

        // Validate memory requirements
        let total_memory_gb = hardware.system.total_memory as f64 / (1024.0 * 1024.0 * 1024.0);
        if total_memory_gb < 4.0 {
            warn!(
                "Low system memory ({:.1} GB) - may impact performance",
                total_memory_gb
            );
        }

        // Validate disk space
        let available_disk_gb =
            hardware.system.available_disk_space as f64 / (1024.0 * 1024.0 * 1024.0);
        if available_disk_gb < 10.0 {
            warn!(
                "Low disk space ({:.1} GB) - may impact BOINC operations",
                available_disk_gb
            );
        }

        Ok(())
    }

    /// Get configuration recommendations based on hardware
    pub fn get_recommendations(
        &self,
        hardware: &crate::hardware_detection::HardwareProfile,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        let work_alloc = &self.work_allocation;

        // Resource allocation recommendations
        if work_alloc.nuw_cpu_percentage + work_alloc.boinc_gpu_percentage < 80 {
            recommendations
                .push("Consider increasing resource allocation (currently at {}%)".to_string());
        }

        // Hardware utilization recommendations
        if work_alloc.hardware_capabilities == HardwareType::Unknown {
            recommendations
                .push("Run hardware detection to determine optimal configuration".to_string());
        }

        // Performance recommendations
        if hardware.cpu.performance_score < 30.0 {
            recommendations.push("CPU performance is low - consider CPU-only projects".to_string());
        }

        if hardware.gpus.is_empty() && work_alloc.boinc_on_gpu {
            recommendations
                .push("GPU work enabled but no GPU detected - disable GPU work".to_string());
        }

        // Project recommendations
        if self.preferences.preferred_projects.is_empty() {
            recommendations
                .push("Consider setting preferred projects for better control".to_string());
        }

        recommendations
    }

    /// Get HTTP client with security settings applied
    pub fn create_http_client(&self) -> Result<reqwest::Client> {
        let mut client_builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.oracle_timeout_secs))
            .user_agent("silica-miner/1.0");

        // Apply security settings
        if !self.security.verify_certificates {
            warn!("Building HTTP client with certificate verification DISABLED");
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        Ok(client_builder.build()?)
    }
}

/// Create a secure HTTP client with appropriate timeouts and security settings
pub fn create_secure_client() -> Result<Client> {
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .user_agent("ChertMiner/1.0")
        .danger_accept_invalid_certs(false) // Always verify certificates
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .context("Failed to create HTTP client")?;

    Ok(client)
}

/// Sanitize user ID to prevent injection attacks
pub fn sanitize_user_id(user_id: &str) -> Result<String> {
    // Basic validation for user ID
    if user_id.is_empty() {
        return Err(anyhow::anyhow!("User ID cannot be empty"));
    }

    if user_id.len() > 64 {
        return Err(anyhow::anyhow!("User ID too long (max 64 characters)"));
    }

    // Only allow alphanumeric characters, dashes, and underscores
    if !user_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow::anyhow!("User ID contains invalid characters"));
    }

    Ok(user_id.to_string())
}

/// Validate BOINC work structure for security
pub fn validate_boinc_work(work: &BoincWork) -> Result<()> {
    // Validate required fields exist and are reasonable
    if work.project_name.is_empty() {
        return Err(anyhow::anyhow!("BOINC work missing project name"));
    }

    if work.project_name.len() > 128 {
        return Err(anyhow::anyhow!("BOINC work project name too long"));
    }

    if work.user_id.is_empty() {
        return Err(anyhow::anyhow!("BOINC work missing user ID"));
    }

    // Validate user ID format
    sanitize_user_id(&work.user_id)?;

    // Additional validation can be added here for specific fields
    // based on the BoincWork structure

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::{Mutex, OnceLock};

    // Serialize environment mutations across tests to avoid cross-test races.
    fn env_lock() -> &'static Mutex<()> {
        static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_MUTEX.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_config_validation() {
        let config = MinerConfig::default();
        // Default config should fail validation due to empty user_id
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_work_allocation_defaults() {
        let config = MinerConfig::default();
        assert!(!config.work_allocation.nuw_on_cpu);
        assert!(config.work_allocation.boinc_on_gpu);
        assert_eq!(config.work_allocation.nuw_cpu_percentage, 25);
        assert_eq!(config.work_allocation.boinc_gpu_percentage, 75);
        assert!(config.work_allocation.nuw_on_demand);
        assert_eq!(config.work_allocation.min_nuw_difficulty, 1000);
        assert_eq!(config.work_allocation.max_boinc_tasks, 2);
    }

    #[test]
    fn test_work_allocation_validation() {
        let mut config = MinerConfig::default();

        // Test invalid CPU percentage
        config.work_allocation.nuw_cpu_percentage = 101;
        assert!(config.validate().is_err());

        // Test invalid GPU percentage
        config.work_allocation.nuw_cpu_percentage = 50;
        config.work_allocation.boinc_gpu_percentage = 60;
        assert!(config.validate().is_err());

        // Test zero max tasks
        config.work_allocation.max_boinc_tasks = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_https_validation() {
        let _guard = env_lock().lock().expect("env lock poisoned");

        unsafe {
            env::set_var("CHERT_ORACLE_URL", "http://insecure.example.com");
            env::set_var("CHERT_MINER_USER_ID", "test_user");
            env::set_var("CHERT_REQUIRE_HTTPS", "true");
        }

        let result = MinerConfig::from_env();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTPS"));

        // Cleanup
        unsafe {
            env::remove_var("CHERT_ORACLE_URL");
            env::remove_var("CHERT_MINER_USER_ID");
            env::remove_var("CHERT_REQUIRE_HTTPS");
        }
    }

    #[test]
    fn test_user_id_validation() {
        let _guard = env_lock().lock().expect("env lock poisoned");

        unsafe {
            env::set_var("CHERT_ORACLE_URL", "https://secure.example.com");
            env::set_var("CHERT_MINER_USER_ID", "invalid user id!");
        }

        let result = MinerConfig::from_env();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid user ID"));

        // Cleanup
        unsafe {
            env::remove_var("CHERT_ORACLE_URL");
            env::remove_var("CHERT_MINER_USER_ID");
        }
    }
}
