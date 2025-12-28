//! BOINC Runner - Main execution loop for BOINC work processing
//!
//! This module provides the high-level BOINC work runner that:
//! - Fetches work from the oracle
//! - Manages the BOINC client daemon
//! - Processes work units and submits results
//! - Handles automatic project attachment and task management

use anyhow::{Context, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::BoincAutomation;
use crate::config::MinerConfig;
use crate::oracle::{fetch_job, submit_result};

/// Statistics for BOINC work processing
#[derive(Debug, Default)]
pub struct BoincStats {
    /// Total work units fetched
    pub work_units_fetched: AtomicU64,
    /// Work units completed successfully
    pub work_units_completed: AtomicU64,
    /// Work units failed
    pub work_units_failed: AtomicU64,
    /// Total compute time in seconds
    pub total_compute_time_secs: AtomicU64,
    /// Current project name
    pub current_project: RwLock<Option<String>>,
    /// Current task ID
    pub current_task: RwLock<Option<String>>,
}

impl BoincStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_fetch(&self) {
        self.work_units_fetched.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_completion(&self, compute_time_secs: u64) {
        self.work_units_completed.fetch_add(1, Ordering::Relaxed);
        self.total_compute_time_secs
            .fetch_add(compute_time_secs, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.work_units_failed.fetch_add(1, Ordering::Relaxed);
    }
}

/// BOINC Runner - manages the BOINC work processing loop
pub struct BoincRunner {
    config: MinerConfig,
    boinc: BoincAutomation,
    stats: Arc<BoincStats>,
    running: Arc<AtomicBool>,
    /// Current work unit being processed
    current_work: RwLock<Option<WorkUnit>>,
}

/// Represents an active work unit being processed
#[derive(Debug, Clone)]
pub struct WorkUnit {
    pub task_id: String,
    pub project_name: String,
    pub project_url: String,
    pub started_at: std::time::Instant,
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
}

impl BoincRunner {
    /// Create a new BOINC runner
    pub fn new(config: MinerConfig) -> Self {
        let boinc = BoincAutomation::new(&config.boinc_install_dir);
        Self {
            config,
            boinc,
            stats: Arc::new(BoincStats::new()),
            running: Arc::new(AtomicBool::new(false)),
            current_work: RwLock::new(None),
        }
    }

    /// Get statistics
    pub fn stats(&self) -> Arc<BoincStats> {
        Arc::clone(&self.stats)
    }

    /// Check if runner is active
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Initialize the BOINC runner
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing BOINC runner...");

        // Ensure directories exist
        self.boinc.ensure_dirs()?;

        // Check if BOINC is installed
        if !self.boinc.is_boinc_installed() {
            info!("BOINC not installed, attempting auto-install...");
            self.boinc
                .auto_install_boinc()
                .await
                .context("Failed to auto-install BOINC")?;
            info!("BOINC installed successfully");
        } else {
            info!(
                "BOINC already installed at: {}",
                self.boinc.get_boinc_path().display()
            );
        }

        // Create optimized client configuration
        self.boinc.create_client_config()?;
        info!("BOINC configuration created");

        Ok(())
    }

    /// Start the BOINC runner main loop
    pub async fn start(&mut self) -> Result<()> {
        if self.running.swap(true, Ordering::SeqCst) {
            warn!("BOINC runner already started");
            return Ok(());
        }

        info!("Starting BOINC runner...");

        // Start BOINC daemon
        self.start_boinc_daemon().await?;

        // Run main work loop
        self.run_work_loop().await
    }

    /// Stop the BOINC runner
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping BOINC runner...");
        self.running.store(false, Ordering::SeqCst);

        // Stop BOINC daemon
        self.boinc.stop_daemon().await?;

        info!("BOINC runner stopped");
        Ok(())
    }

    /// Start the BOINC daemon
    async fn start_boinc_daemon(&mut self) -> Result<()> {
        // Check if already running
        if self.boinc.is_daemon_running().await {
            info!("BOINC daemon already running");
            return Ok(());
        }

        // Get project URL from oracle or config
        let project_url = self.get_project_url().await?;
        let authenticator = &self.config.user_id;

        info!("Starting BOINC daemon with project: {}", project_url);

        self.boinc
            .start_daemon(Some(&project_url), Some(authenticator))
            .await
            .context("Failed to start BOINC daemon")?;

        // Wait for daemon to initialize
        tokio::time::sleep(Duration::from_secs(5)).await;

        info!("BOINC daemon started successfully");
        Ok(())
    }

    /// Main work processing loop
    async fn run_work_loop(&mut self) -> Result<()> {
        info!("Entering BOINC work loop");

        let poll_interval = Duration::from_secs(30);
        let work_check_interval = Duration::from_secs(10);

        while self.running.load(Ordering::Relaxed) {
            // Check if we have active work
            let has_work = self.current_work.read().await.is_some();

            if has_work {
                // Monitor current work progress
                self.check_work_progress().await?;
                tokio::time::sleep(work_check_interval).await;
            } else {
                // Try to fetch new work
                match self.fetch_and_start_work().await {
                    Ok(true) => {
                        info!("Started new work unit");
                    }
                    Ok(false) => {
                        debug!("No work available, waiting...");
                        tokio::time::sleep(poll_interval).await;
                    }
                    Err(e) => {
                        warn!("Error fetching work: {}", e);
                        tokio::time::sleep(poll_interval).await;
                    }
                }
            }
        }

        Ok(())
    }

    /// Fetch work from oracle and start processing
    async fn fetch_and_start_work(&mut self) -> Result<bool> {
        // Fetch work from oracle
        let work = match fetch_job(&self.config.oracle_url, &self.config.user_id).await {
            Ok(work) => work,
            Err(e) => {
                debug!("No work available: {}", e);
                return Ok(false);
            }
        };

        self.stats.record_fetch();

        info!(
            "Received work unit: {} from project {}",
            work.task_id, work.project_name
        );

        // Create work unit tracking - use project name as URL placeholder
        let project_url = self
            .project_name_to_url(&work.project_name)
            .unwrap_or_else(|| format!("{}/boinc/{}", self.config.oracle_url, work.project_name));

        let work_unit = WorkUnit {
            task_id: work.task_id.clone(),
            project_name: work.project_name.clone(),
            project_url,
            started_at: std::time::Instant::now(),
            deadline: None, // BoincWork doesn't have deadline field, use completion_time as estimate
        };

        // Store current work
        *self.current_work.write().await = Some(work_unit.clone());
        *self.stats.current_project.write().await = Some(work.project_name.clone());
        *self.stats.current_task.write().await = Some(work.task_id.clone());

        // Ensure BOINC daemon is running with the correct project
        if !self.boinc.is_daemon_running().await {
            self.start_boinc_daemon().await?;
        }

        info!("Work unit {} queued for processing", work_unit.task_id);
        Ok(true)
    }

    /// Check progress of current work
    async fn check_work_progress(&mut self) -> Result<()> {
        let work = match self.current_work.read().await.clone() {
            Some(w) => w,
            None => return Ok(()),
        };

        // Check if work has been running too long (deadline check)
        if let Some(deadline) = work.deadline {
            if chrono::Utc::now() > deadline {
                warn!(
                    "Work unit {} exceeded deadline, marking as failed",
                    work.task_id
                );
                self.stats.record_failure();
                *self.current_work.write().await = None;
                return Ok(());
            }
        }

        // Check BOINC status for task completion
        // In a real implementation, we would parse BOINC's state file or use RPC
        let result_file = self
            .boinc
            .data_dir
            .join(format!("result_{}.dat", work.task_id));

        if result_file.exists() {
            // Work completed - submit result
            let result_data = tokio::fs::read_to_string(&result_file)
                .await
                .unwrap_or_else(|_| String::new());

            let compute_time = work.started_at.elapsed().as_secs();

            // Create a BoincWork for submission (using the actual model fields)
            let boinc_work = silica_models::boinc::BoincWork {
                task_id: work.task_id.clone(),
                project_name: work.project_name.clone(),
                user_id: self.config.user_id.clone(),
                cpu_time: compute_time as f64,
                credit_granted: 0.0, // Will be set by oracle
                completion_time: chrono::Utc::now(),
                validation_state: None,
            };

            match submit_result(&self.config.oracle_url, &boinc_work, &result_data).await {
                Ok(()) => {
                    self.stats.record_completion(compute_time);
                    info!("Work unit {} completed in {}s", work.task_id, compute_time);

                    // Clean up result file
                    let _ = tokio::fs::remove_file(&result_file).await;
                }
                Err(e) => {
                    error!("Failed to submit result for {}: {}", work.task_id, e);
                    self.stats.record_failure();
                }
            }

            // Clear current work
            *self.current_work.write().await = None;
            *self.stats.current_task.write().await = None;
        } else {
            // Log progress periodically
            let elapsed = work.started_at.elapsed().as_secs();
            if elapsed % 60 == 0 && elapsed > 0 {
                debug!("Work unit {} running for {}s", work.task_id, elapsed);
            }
        }

        Ok(())
    }

    /// Get project URL from config or oracle
    async fn get_project_url(&self) -> Result<String> {
        // Check config for preferred projects
        if let Some(first) = self.config.preferences.preferred_projects.first() {
            if let Some(url) = self.project_name_to_url(first) {
                return Ok(url);
            }
        }

        // Default to oracle's BOINC proxy
        Ok(format!("{}/boinc/milkyway", self.config.oracle_url))
    }

    /// Convert project name to URL
    fn project_name_to_url(&self, name: &str) -> Option<String> {
        let base = &self.config.oracle_url;
        let path = match name.to_lowercase().as_str() {
            "milkyway@home" | "milkyway" => "boinc/milkyway",
            "rosetta@home" | "rosetta" => "boinc/rosetta",
            "einstein@home" | "einstein" => "boinc/einstein",
            "seti@home" | "seti" => "boinc/seti",
            "world community grid" | "wcg" => "boinc/wcg",
            "gpugrid" => "boinc/gpugrid",
            "asteroids@home" => "boinc/asteroids",
            "lhc@home" => "boinc/lhc",
            "climateprediction.net" => "boinc/climate",
            _ => return None,
        };
        Some(format!("{}/{}", base, path))
    }
}

/// Standalone function to run BOINC work loop
pub async fn run_boinc_worker(config: MinerConfig, running: Arc<AtomicBool>) -> Result<()> {
    let mut runner = BoincRunner::new(config);

    // Override the running flag
    runner.running = running;

    runner.initialize().await?;
    runner.start().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boinc_stats_default() {
        let stats = BoincStats::new();
        assert_eq!(stats.work_units_fetched.load(Ordering::Relaxed), 0);
        assert_eq!(stats.work_units_completed.load(Ordering::Relaxed), 0);
        assert_eq!(stats.work_units_failed.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_boinc_stats_recording() {
        let stats = BoincStats::new();
        stats.record_fetch();
        stats.record_fetch();
        stats.record_completion(100);
        stats.record_failure();

        assert_eq!(stats.work_units_fetched.load(Ordering::Relaxed), 2);
        assert_eq!(stats.work_units_completed.load(Ordering::Relaxed), 1);
        assert_eq!(stats.work_units_failed.load(Ordering::Relaxed), 1);
        assert_eq!(stats.total_compute_time_secs.load(Ordering::Relaxed), 100);
    }
}
