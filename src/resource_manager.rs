//! Resource Manager - CPU and GPU resource management for miner
//!
//! Handles:
//! - CPU throttling (limits, low CPU mode)
//! - Process priority (nice values)
//! - BOINC CPU/GPU allocation
//! - NUW CPU allocation
//!
//! ## Usage
//!
//! ```ignore
//! let resource_manager = ResourceManager::new(work_allocation_config);
//! resource_manager.apply().await;
//! ```

use std::process::Command;
use tracing::{info, warn};

use crate::config::WorkAllocationConfig;

/// Resource manager for CPU/GPU throttling
pub struct ResourceManager {
    config: WorkAllocationConfig,
}

impl ResourceManager {
    pub fn new(config: WorkAllocationConfig) -> Self {
        Self { config }
    }

    /// Apply resource limits based on configuration
    pub async fn apply(&self) {
        if self.config.low_cpu_mode {
            self.apply_low_cpu_mode();
        } else {
            self.apply_dedicated_mode();
        }

        // Apply CPU limits to this process
        self.apply_process_priority();
    }

    /// Apply low CPU mode - limits usage for casual users
    fn apply_low_cpu_mode(&self) {
        let limit = self.config.low_cpu_limit;
        info!("Applying LOW CPU mode: {}% limit", limit);

        // Set CPU affinity to use only a subset of cores
        // This is a simplified approach - in production you'd use cgroups
        self.limit_cpu_to_percentage(limit);

        // Note: BOINC itself has built-in CPU limiting via its config
        // We'll communicate this through environment or config files
    }

    /// Apply dedicated mining mode - full power
    fn apply_dedicated_mode(&self) {
        info!("Applying DEDICATED mining mode: full power");

        // Use all available cores
        // BOINC will use configured percentage
    }

    /// Limit CPU usage by setting process priority (nice value)
    fn apply_process_priority(&self) {
        let nice_value = if self.config.low_cpu_mode {
            // Low priority for casual use
            10 // nice value 10 = very low priority
        } else {
            // Normal priority for dedicated mining
            0 // nice value 0 = normal priority
        };

        // Set nice value for current process
        #[cfg(unix)]
        {
            match Command::new("renice")
                .args(["-n", &nice_value.to_string(), "-p", &std::process::id().to_string()])
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        info!("Set process nice value to {}", nice_value);
                    } else {
                        warn!("Failed to set nice value: {:?}", output.stderr);
                    }
                }
                Err(e) => {
                    warn!("Could not execute renice: {}", e);
                }
            }
        }

        // For Windows, we'd use setpriority or similar
        #[cfg(windows)]
        {
            info!("Windows priority adjustment not implemented");
        }
    }

    /// Get effective CPU percentages based on mode
    pub fn get_effective_cpu_percentage(&self) -> (u8, u8) {
        let nuw_pct = if self.config.low_cpu_mode {
            // In low CPU mode, reduce NUW usage too
            (self.config.nuw_cpu_percentage as f32 * 0.5) as u8
        } else {
            self.config.nuw_cpu_percentage
        };

        let boinc_pct = if self.config.low_cpu_mode {
            // In low CPU mode, limit BOINC to remaining
            self.config.low_cpu_limit.saturating_sub(nuw_pct)
        } else {
            self.config.boinc_cpu_percentage
        };

        (nuw_pct, boinc_pct)
    }

    /// Get effective GPU percentage for BOINC
    pub fn get_boinc_gpu_percentage(&self) -> u8 {
        if self.config.low_cpu_mode {
            // In low CPU mode, also limit GPU to avoid heating
            (self.config.boinc_gpu_percentage as f32 * 0.7) as u8
        } else {
            self.config.boinc_gpu_percentage
        }
    }

    /// Calculate CPU limit percentage
    fn limit_cpu_to_percentage(&self, percentage: u8) {
        // This would ideally use cgroups or similar
        // For now, we log what we'd do
        info!(
            "Would limit CPU to {}% (requires cgroups/container support)",
            percentage
        );
    }

    /// Get mode description
    pub fn get_mode_description(&self) -> String {
        if self.config.low_cpu_mode {
            format!(
                "LOW CPU MODE ({}% limit, nice={})",
                self.config.low_cpu_limit,
                if self.config.low_cpu_mode { 10 } else { 0 }
            )
        } else {
            format!(
                "DEDICATED MODE (NUW={}%, BOINC CPU={}%, GPU={}%)",
                self.config.nuw_cpu_percentage,
                self.config.boinc_cpu_percentage,
                self.config.boinc_gpu_percentage
            )
        }
    }
}

/// Check if system supports resource management features
pub fn check_resource_management_support() -> &'static str {
    #[cfg(unix)]
    {
        "Unix: renice available, cgroups recommended"
    }

    #[cfg(windows)]
    {
        "Windows: limited support"
    }

    #[cfg(not(any(unix, windows)))]
    {
        "Unknown platform"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_percentages_low_mode() {
        let config = WorkAllocationConfig {
            low_cpu_mode: true,
            low_cpu_limit: 70,
            nuw_cpu_percentage: 25,
            boinc_cpu_percentage: 50,
            boinc_gpu_percentage: 100,
            ..Default::default()
        };

        let manager = ResourceManager::new(config);
        let (nuw, boinc) = manager.get_effective_cpu_percentage();

        // In low mode, NUW should be halved
        assert_eq!(nuw, 12); // 25 * 0.5 = 12
        // BOINC gets remaining
        assert_eq!(boinc, 70 - 12);
    }

    #[test]
    fn test_effective_percentages_dedicated_mode() {
        let config = WorkAllocationConfig {
            low_cpu_mode: false,
            nuw_cpu_percentage: 25,
            boinc_cpu_percentage: 50,
            boinc_gpu_percentage: 100,
            ..Default::default()
        };

        let manager = ResourceManager::new(config);
        let (nuw, boinc) = manager.get_effective_cpu_percentage();

        assert_eq!(nuw, 25);
        assert_eq!(boinc, 50);
    }

    #[test]
    fn test_mode_description() {
        let config = WorkAllocationConfig {
            low_cpu_mode: true,
            low_cpu_limit: 70,
            ..Default::default()
        };

        let manager = ResourceManager::new(config);
        let desc = manager.get_mode_description();

        assert!(desc.contains("LOW CPU MODE"));
    }
}
