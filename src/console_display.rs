//! Console Display Module - Non-TUI terminal output
//!
//! Provides a simple console-based dashboard for the miner without requiring
//! the full TUI (ratatui) dependency.
//!
//! Usage:
//!   --console or --display=console
//!
//! Displays:
//!   - Miner status and mode
//!   - Current task being processed
//!   - System metrics (CPU, Memory)
//!   - NUW statistics
//!   - BOINC statistics

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::boinc::BoincStats;
use crate::nuw_worker::NuwStats;
use crate::config::MinerConfig;

/// Console display configuration
#[derive(Debug, Clone)]
pub struct ConsoleDisplayConfig {
    /// Refresh interval in seconds
    pub refresh_secs: u64,
    /// Show detailed NUW stats
    pub show_nuw_detail: bool,
    /// Show BOINC stats
    pub show_boinc: bool,
    /// Show system metrics
    pub show_system: bool,
}

impl Default for ConsoleDisplayConfig {
    fn default() -> Self {
        Self {
            refresh_secs: 5,
            show_nuw_detail: true,
            show_boinc: true,
            show_system: true,
        }
    }
}

/// Console display for miner status
pub struct ConsoleDisplay {
    running: Arc<AtomicBool>,
    config: ConsoleDisplayConfig,
}

impl ConsoleDisplay {
    pub fn new(config: ConsoleDisplayConfig) -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            config,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Start the console display loop
    pub async fn start(
        &self,
        miner_config: MinerConfig,
        nuw_stats: Option<Arc<NuwStats>>,
        boinc_stats: Option<Arc<BoincStats>>,
        account_address: String,
    ) {
        if self.running.swap(true, Ordering::SeqCst) {
            warn!("Console display already running");
            return;
        }

        let running = Arc::clone(&self.running);
        let config = self.config.clone();

        tokio::spawn(async move {
            let start_time = Instant::now();
            let mut iteration = 0u64;

            while running.load(Ordering::Relaxed) {
                iteration += 1;
                let uptime = start_time.elapsed();

                // Print header
                println!("\n{}", "=".repeat(80));
                println!(" CHERT MINER - {}", Self::format_uptime(uptime));
                println!("{}", "=".repeat(80));

                // Miner info
                println!(" Account: {}", account_address);
                println!(" Mode:    {}", format!("{:?}", miner_config.work_allocation));
                println!(" Oracle:  {}", miner_config.oracle_url);

                // System metrics
                if config.show_system {
                    Self::print_system_metrics();
                }

                // NUW stats
                if let Some(ref stats) = nuw_stats {
                    Self::print_nuw_stats(stats, config.show_nuw_detail);
                }

                // BOINC stats
                if let Some(ref stats) = boinc_stats && config.show_boinc {
                    Self::print_boinc_stats(stats);
                }

                println!("{}", "=".repeat(80));
                println!(" Press Ctrl+C to stop");
                println!();

                sleep(Duration::from_secs(config.refresh_secs)).await;
            }

            info!("Console display stopped");
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    fn print_system_metrics() {
        // Get system info without sysinfo to avoid API version issues
        // Just show basic info
        
        // CPU cores
        let cpu_count = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1);
        
        println!("\n[System]");
        println!(" CPU Cores: {}", cpu_count);
        println!(" (Enable sysinfo for detailed metrics)");
    }

    fn print_nuw_stats(stats: &NuwStats, detailed: bool) {
        let completed = stats.tasks_completed.load(Ordering::Relaxed);
        let failed = stats.tasks_failed.load(Ordering::Relaxed);
        let avg_time = stats.avg_solution_time_ms.load(Ordering::Relaxed);

        println!("\n[NUW Tasks]");
        println!(" Completed: {}", completed);
        println!(" Failed:    {}", failed);
        println!(" Avg Time:  {}ms", avg_time);

        if detailed {
            let sig = stats.sig_batch_verified.load(Ordering::Relaxed);
            let zk = stats.zk_verified.load(Ordering::Relaxed);
            let merkle = stats.merkle_verified.load(Ordering::Relaxed);
            let boinc = stats.boinc_completed.load(Ordering::Relaxed);

            println!(" Breakdown:");
            println!("  Signatures: {}", sig);
            println!("  ZK Proofs:  {}", zk);
            println!("  Merkle:      {}", merkle);
            println!("  BOINC:       {}", boinc);
        }
    }

    fn print_boinc_stats(stats: &BoincStats) {
        let completed = stats.work_units_completed.load(Ordering::Relaxed);
        let failed = stats.work_units_failed.load(Ordering::Relaxed);
        let total_time = stats.total_compute_time_secs.load(Ordering::Relaxed);
        let fetched = stats.work_units_fetched.load(Ordering::Relaxed);
        
        // Get project name - this is async so we can't easily get it here
        // Just show basic stats

        println!("\n[BOINC]");
        println!(" Fetched:   {}", fetched);
        println!(" Completed: {} completed, {} failed", completed, failed);
        println!(" Compute:   {} hours", total_time / 3600);
    }

    fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    fn format_uptime(duration: Duration) -> String {
        let secs = duration.as_secs();
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        let secs = secs % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, mins, secs)
        } else {
            format!("{:02}:{:02}", mins, secs)
        }
    }
}

/// Start console display as a background task
pub async fn start_console_display(
    config: &MinerConfig,
    nuw_stats: Option<Arc<NuwStats>>,
    boinc_stats: Option<Arc<BoincStats>>,
) -> Option<ConsoleDisplay> {
    // Check if console mode is enabled
    let use_console = std::env::var("CHERT_CONSOLE")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);

    if !use_console {
        return None;
    }

    let console_config = ConsoleDisplayConfig::default();
    let display = ConsoleDisplay::new(console_config);

    let account = config.account_address.clone();
    let miner_config = config.clone();

    display.start(
        miner_config,
        nuw_stats,
        boinc_stats,
        account,
    ).await;

    Some(display)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(ConsoleDisplay::format_bytes(500), "500 B");
        assert_eq!(ConsoleDisplay::format_bytes(1024), "1.0 KB");
        assert_eq!(ConsoleDisplay::format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(ConsoleDisplay::format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_format_uptime() {
        assert_eq!(
            ConsoleDisplay::format_uptime(Duration::from_secs(65)),
            "01:05"
        );
        assert_eq!(
            ConsoleDisplay::format_uptime(Duration::from_secs(3665)),
            "01:01:05"
        );
    }
}
