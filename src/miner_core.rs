//! Miner Core - Main mining orchestrator
//!
//! This module coordinates the different mining activities:
//! - BOINC scientific computing work
//! - NUW (Network Utility Work) challenges
//! - Resource allocation between work types
//!
//! ## Work Modes
//!
//! | Mode | CPU | GPU | Description |
//! |------|-----|-----|-------------|
//! | `Mixed` | NUW + BOINC | BOINC | Default balanced mode |
//! | `BoincOnly` | BOINC | BOINC | All resources to BOINC |
//! | `NuwOnly` | NUW | NUW | All resources to NUW |
//! | `GpuOnly` | Idle | BOINC | GPU-only BOINC work |

use anyhow::{Context, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::boinc::{BoincAutomation, BoincRunner, BoincStats};
use crate::config::MinerConfig;
use crate::hardware_detection::{HardwareDetector, HardwareProfile, HardwareType};
use crate::nuw_worker::NuwWorker;
use crate::oracle_profile::OracleProfileManager;
use crate::performance_monitor::{MetricsSnapshot, PerformanceMonitor};

/// Mining work mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkMode {
    /// Mixed: NUW on CPU, BOINC on GPU (default)
    #[default]
    Mixed,
    /// BOINC only on all hardware
    BoincOnly,
    /// NUW only on all hardware
    NuwOnly,
    /// GPU-only BOINC work (CPU idle or NUW)
    GpuOnly,
}

/// Mining status
#[derive(Debug, Clone)]
pub struct MinerStatus {
    /// Current work mode
    pub work_mode: WorkMode,
    /// Is BOINC running
    pub boinc_running: bool,
    /// Is NUW worker running
    pub nuw_running: bool,
    /// Hardware profile
    pub hardware_type: HardwareType,
    /// NUW challenges solved
    pub nuw_challenges_solved: u64,
    /// BOINC work units completed
    pub boinc_work_units: u64,
    /// Current BOINC project
    pub current_boinc_project: Option<String>,
    /// Oracle registered
    pub oracle_registered: bool,
}

/// Main miner orchestrator
pub struct MinerCore {
    config: MinerConfig,
    work_mode: WorkMode,
    running: Arc<AtomicBool>,

    // Hardware
    hardware_profile: Option<HardwareProfile>,

    // Workers
    boinc: Option<BoincAutomation>,
    boinc_runner: Option<BoincRunner>,
    boinc_stats: Option<Arc<BoincStats>>,
    nuw_worker: Option<Arc<NuwWorker>>,

    // Oracle integration
    oracle_manager: Option<OracleProfileManager>,

    // Performance monitoring
    perf_monitor: Option<PerformanceMonitor>,

    // State
    current_project: Arc<RwLock<Option<String>>>,
    boinc_work_units: Arc<RwLock<u64>>,
}

impl MinerCore {
    /// Create a new miner core
    pub fn new(config: MinerConfig) -> Self {
        // Determine work mode from config
        let work_mode = if config.work_allocation.nuw_on_cpu && config.work_allocation.boinc_on_gpu
        {
            WorkMode::Mixed
        } else if config.work_allocation.nuw_on_cpu && !config.work_allocation.boinc_on_gpu {
            WorkMode::NuwOnly
        } else if !config.work_allocation.nuw_on_cpu && config.work_allocation.boinc_on_gpu {
            WorkMode::BoincOnly
        } else {
            WorkMode::GpuOnly
        };

        Self {
            config,
            work_mode,
            running: Arc::new(AtomicBool::new(false)),
            hardware_profile: None,
            boinc: None,
            boinc_runner: None,
            boinc_stats: None,
            nuw_worker: None,
            oracle_manager: None,
            perf_monitor: None,
            current_project: Arc::new(RwLock::new(None)),
            boinc_work_units: Arc::new(RwLock::new(0)),
        }
    }

    /// Initialize the miner
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing Chert miner...");

        // 1. Detect hardware
        info!("Detecting hardware capabilities...");
        let mut detector = HardwareDetector::new();
        let profile = detector
            .detect_hardware()
            .context("Failed to detect hardware")?;

        info!(
            "Hardware: {} cores, {} GPU(s), {:.1} GB RAM",
            profile.cpu.physical_cores,
            profile.gpus.len(),
            profile.system.total_memory as f64 / (1024.0 * 1024.0 * 1024.0)
        );

        self.hardware_profile = Some(profile);

        // 2. Initialize Oracle profile manager
        info!("Connecting to oracle...");
        let mut oracle_manager = OracleProfileManager::new(&self.config);
        match oracle_manager.initialize().await {
            Ok(()) => {
                if oracle_manager.is_registered() {
                    info!("Successfully registered with oracle");

                    // Get recommendations
                    match oracle_manager.get_recommendations().await {
                        Ok(recs) if !recs.is_empty() => {
                            let best = &recs[0];
                            info!(
                                "Recommended project: {} (score: {:.1}, reward: {:.2}x)",
                                best.project_name, best.score, best.estimated_reward
                            );
                            *self.current_project.write().await = Some(best.project_name.clone());
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                warn!("Oracle connection failed: {}. Continuing offline.", e);
            }
        }
        self.oracle_manager = Some(oracle_manager);

        // 3. Initialize BOINC if needed
        if matches!(
            self.work_mode,
            WorkMode::Mixed | WorkMode::BoincOnly | WorkMode::GpuOnly
        ) {
            info!("Initializing BOINC client...");
            let boinc = BoincAutomation::new(&self.config.boinc_install_dir);

            // Ensure directories exist
            boinc.ensure_dirs()?;

            // Check if BOINC is installed
            if !boinc.is_boinc_installed() {
                info!("BOINC not found. Attempting auto-install...");
                match boinc.auto_install_boinc().await {
                    Ok(()) => {
                        info!("BOINC installed successfully");
                        boinc.create_client_config()?;
                    }
                    Err(e) => {
                        warn!(
                            "BOINC auto-install failed: {}. BOINC work will be skipped.",
                            e
                        );
                    }
                }
            } else {
                info!("BOINC client found: {}", boinc.get_boinc_path().display());
                boinc.create_client_config()?;
            }

            // Create the BOINC runner for work processing
            let mut boinc_runner = BoincRunner::new(self.config.clone());
            match boinc_runner.initialize().await {
                Ok(()) => {
                    info!("BOINC runner initialized");
                    self.boinc_stats = Some(boinc_runner.stats());
                    self.boinc_runner = Some(boinc_runner);
                }
                Err(e) => {
                    warn!("BOINC runner initialization failed: {}", e);
                }
            }

            self.boinc = Some(boinc);
        }

        // 4. Initialize NUW worker if needed
        if matches!(self.work_mode, WorkMode::Mixed | WorkMode::NuwOnly) {
            info!("Initializing NUW worker...");
            let nuw_worker = Arc::new(NuwWorker::new(&self.config));
            self.nuw_worker = Some(nuw_worker);
        }

        // 5. Initialize performance monitor
        let boinc_data_dir = self.config.boinc_data_dir.to_string_lossy().to_string();
        self.perf_monitor = Some(PerformanceMonitor::new(boinc_data_dir));

        info!("Miner initialization complete");
        info!("Work mode: {:?}", self.work_mode);

        Ok(())
    }

    /// Start mining
    pub async fn start(&mut self) -> Result<()> {
        if self.running.swap(true, Ordering::SeqCst) {
            warn!("Miner already running");
            return Ok(());
        }

        info!("Starting Chert miner in {:?} mode", self.work_mode);

        // Start NUW worker if enabled
        let nuw_handle = if let Some(nuw_worker) = &self.nuw_worker {
            let worker = Arc::clone(nuw_worker);
            let running = Arc::clone(&self.running);
            Some(tokio::spawn(async move {
                while running.load(Ordering::Relaxed) {
                    if let Err(e) = worker.start().await {
                        error!("NUW worker error: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }))
        } else {
            None
        };

        // Start BOINC runner if available
        let boinc_handle = if let Some(mut runner) = self.boinc_runner.take() {
            let _running = Arc::clone(&self.running);
            info!("Starting BOINC work runner...");
            Some(tokio::spawn(async move {
                // Use the runner's start which runs the work loop
                if let Err(e) = runner.start().await {
                    error!("BOINC runner error: {}", e);
                }
            }))
        } else {
            // Fallback: Start BOINC daemon directly (legacy mode)
            // Get project URL before borrowing boinc
            let project_url = self.get_boinc_project_url().await;
            let auth = self.config.user_id.clone();

            if let Some(ref mut boinc) = self.boinc {
                if boinc.is_boinc_installed() {
                    match boinc
                        .start_daemon(project_url.as_deref(), Some(&auth))
                        .await
                    {
                        Ok(()) => {
                            info!("BOINC daemon started (legacy mode)");
                        }
                        Err(e) => {
                            error!("Failed to start BOINC: {}", e);
                        }
                    }
                } else {
                    warn!("BOINC not installed, skipping BOINC work");
                }
            }
            None
        };

        // Main monitoring loop
        let running = Arc::clone(&self.running);
        let perf_monitor = self.perf_monitor.take();
        let boinc_stats = self.boinc_stats.clone();
        let boinc_work_units = Arc::clone(&self.boinc_work_units);
        let nuw_worker = self.nuw_worker.clone();

        let monitor_handle = tokio::spawn(async move {
            let mut monitor = perf_monitor;
            let mut last_boinc_completed = 0u64;

            while running.load(Ordering::Relaxed) {
                // Collect metrics
                if let Some(ref mut mon) = monitor {
                    if let Ok(_snapshot) = mon.collect_metrics() {
                        // Metrics collected successfully
                    }
                }

                // Log BOINC stats from runner
                if let Some(ref stats) = boinc_stats {
                    let completed = stats.work_units_completed.load(Ordering::Relaxed);
                    if completed > last_boinc_completed {
                        let mut units = boinc_work_units.write().await;
                        *units = completed;
                        info!(
                            "BOINC: {} completed, {} failed, {} total compute time",
                            completed,
                            stats.work_units_failed.load(Ordering::Relaxed),
                            stats.total_compute_time_secs.load(Ordering::Relaxed)
                        );
                        last_boinc_completed = completed;
                    }
                }

                // Log NUW stats periodically
                if let Some(ref worker) = nuw_worker {
                    let stats = worker.stats();
                    let solved = stats.challenges_solved.load(Ordering::Relaxed);
                    if solved > 0 && solved % 10 == 0 {
                        info!(
                            "NUW: {} solved, {} failed, avg time: {}ms",
                            solved,
                            stats.challenges_failed.load(Ordering::Relaxed),
                            stats.avg_solution_time_ms.load(Ordering::Relaxed)
                        );
                    }
                }

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        info!("Miner running. Press Ctrl+C to stop.");

        // Wait for shutdown signal
        tokio::signal::ctrl_c().await?;
        info!("Shutdown signal received");

        // Stop workers
        self.running.store(false, Ordering::SeqCst);

        if let Some(nuw_worker) = &self.nuw_worker {
            nuw_worker.stop();
        }

        if let Some(ref mut boinc) = self.boinc {
            let _ = boinc.stop_daemon().await;
        }

        // Wait for tasks
        if let Some(handle) = nuw_handle {
            let _ = handle.await;
        }
        if let Some(handle) = boinc_handle {
            let _ = handle.await;
        }
        let _ = monitor_handle.await;

        info!("Miner stopped");
        Ok(())
    }

    /// Stop mining
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping miner...");
        self.running.store(false, Ordering::SeqCst);

        if let Some(nuw_worker) = &self.nuw_worker {
            nuw_worker.stop();
        }

        if let Some(ref mut boinc) = self.boinc {
            boinc.stop_daemon().await?;
        }

        Ok(())
    }

    /// Get current status
    pub async fn status(&self) -> MinerStatus {
        let nuw_stats = self.nuw_worker.as_ref().map(|w| w.stats());

        // Get BOINC work units from stats if available
        let boinc_work_units = if let Some(ref stats) = self.boinc_stats {
            stats.work_units_completed.load(Ordering::Relaxed)
        } else {
            *self.boinc_work_units.blocking_read()
        };

        // Get current project from BOINC stats if available
        let current_project = if let Some(ref stats) = self.boinc_stats {
            stats.current_project.blocking_read().clone()
        } else {
            self.current_project.blocking_read().clone()
        };

        MinerStatus {
            work_mode: self.work_mode,
            boinc_running: self
                .boinc
                .as_ref()
                .map(|b| b.daemon_process.is_some())
                .unwrap_or(false)
                || self.boinc_runner.is_some(),
            nuw_running: self
                .nuw_worker
                .as_ref()
                .map(|w| w.is_running())
                .unwrap_or(false),
            hardware_type: self
                .hardware_profile
                .as_ref()
                .map(|p| p.hardware_type.clone())
                .unwrap_or(HardwareType::Unknown),
            nuw_challenges_solved: nuw_stats
                .map(|s| s.challenges_solved.load(Ordering::Relaxed))
                .unwrap_or(0),
            boinc_work_units,
            current_boinc_project: current_project,
            oracle_registered: self
                .oracle_manager
                .as_ref()
                .map(|o| o.is_registered())
                .unwrap_or(false),
        }
    }

    /// Get latest performance metrics
    pub fn latest_metrics(&mut self) -> Option<MetricsSnapshot> {
        self.perf_monitor
            .as_mut()
            .and_then(|m| m.get_current_metrics().cloned())
    }

    /// Set work mode
    pub fn set_work_mode(&mut self, mode: WorkMode) {
        info!("Changing work mode from {:?} to {:?}", self.work_mode, mode);
        self.work_mode = mode;
    }

    /// Get BOINC project URL based on recommendations or config
    async fn get_boinc_project_url(&self) -> Option<String> {
        // First check current project from oracle
        if let Some(project) = self.current_project.read().await.as_ref() {
            return self.project_name_to_url(project);
        }

        // Fall back to preferred projects from config
        if let Some(first_preferred) = self.config.preferences.preferred_projects.first() {
            return self.project_name_to_url(first_preferred);
        }

        // Default to MilkyWay@Home (through our proxy)
        Some(format!("{}/boinc/milkyway", self.config.oracle_url))
    }

    /// Convert project name to URL
    fn project_name_to_url(&self, name: &str) -> Option<String> {
        // Map common project names to URLs (through our oracle proxy)
        let base = &self.config.oracle_url;
        let project_path = match name.to_lowercase().as_str() {
            "milkyway@home" | "milkyway" => "boinc/milkyway",
            "rosetta@home" | "rosetta" => "boinc/rosetta",
            "einstein@home" | "einstein" => "boinc/einstein",
            "seti@home" | "seti" => "boinc/seti",
            "folding@home" | "fah" => "boinc/fah",
            "world community grid" | "wcg" => "boinc/wcg",
            "gpugrid" => "boinc/gpugrid",
            "asteroids@home" => "boinc/asteroids",
            "lhc@home" => "boinc/lhc",
            "climateprediction.net" => "boinc/climate",
            _ => return None,
        };

        Some(format!("{}/{}", base, project_path))
    }
}

/// Run the miner with configuration
pub async fn run_miner(config: MinerConfig) -> Result<()> {
    let mut miner = MinerCore::new(config);

    miner.initialize().await?;
    miner.start().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_mode_default() {
        assert_eq!(WorkMode::default(), WorkMode::Mixed);
    }

    #[test]
    fn test_miner_core_creation() {
        let config = MinerConfig::default();
        let miner = MinerCore::new(config);

        // Default config has nuw_on_cpu=false, boinc_on_gpu=true
        // So it should be BoincOnly mode
        assert!(!miner.running.load(Ordering::Relaxed));
    }
}
