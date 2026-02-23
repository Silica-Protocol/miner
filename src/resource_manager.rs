//! Resource Manager - CPU and GPU resource management for miner
//!
//! Handles:
//! - CPU throttling (limits, low CPU mode)
//! - Process priority (nice values)
//! - BOINC CPU/GPU allocation (dynamic based on NUW activity)
//! - NUW CPU allocation
//!
//! ## Dynamic BOINC Throttling
//!
//! When NUW work is active, BOINC CPU is reduced quickly.
//! When NUW is idle, BOINC CPU builds back up slowly.
//!
//! ## Usage
//!
//! ```ignore
//! let resource_manager = ResourceManager::new(work_allocation_config);
//! resource_manager.apply().await;
//! ```

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::{info, warn, debug};

use crate::config::WorkAllocationConfig;

/// Resource manager for CPU/GPU throttling
pub struct ResourceManager {
    config: WorkAllocationConfig,
    /// Current dynamic BOINC CPU percentage (starts at configured value)
    current_boinc_cpu: AtomicU64,
    /// Last time NUW had activity
    last_nuw_activity: std::sync::Mutex<Option<Instant>>,
    /// Last NUW completed task count (for detecting new work)
    last_nuw_completed: AtomicU64,
    /// Base BOINC CPU percentage (original configured value)
    base_boinc_cpu: u8,
    /// How quickly to reduce BOINC CPU when NUW active (percentage points per check)
    boinc_reduce_rate: u8,
    /// How quickly to increase BOINC CPU when NUW idle (percentage points per check)
    boinc_recover_rate: u8,
    /// Seconds of idle before BOINC starts recovering
    boinc_recover_delay_secs: u64,
}

impl ResourceManager {
    pub fn new(config: WorkAllocationConfig) -> Self {
        let base_boinc_cpu = config.boinc_cpu_percentage;
        Self {
            config: config.clone(),
            current_boinc_cpu: AtomicU64::new(base_boinc_cpu as u64),
            last_nuw_activity: std::sync::Mutex::new(None),
            last_nuw_completed: AtomicU64::new(0),
            base_boinc_cpu,
            boinc_reduce_rate: config.throttling_reduce_rate,
            boinc_recover_rate: config.throttling_recover_rate,
            boinc_recover_delay_secs: config.throttling_recover_delay_secs,
        }
    }

    /// Check for NUW activity and adjust BOINC CPU accordingly
    /// Call this periodically (e.g., every 5 seconds)
    pub fn adjust_for_nuw_activity(&self, nuw_completed: u64) -> u8 {
        let last_completed = self.last_nuw_completed.load(Ordering::Relaxed);
        let now = Instant::now();
        
        let is_active = nuw_completed > last_completed;
        
        if is_active {
            // NUW is active - reduce BOINC CPU quickly
            let current = self.current_boinc_cpu.load(Ordering::Relaxed) as u8;
            let new_cpu = current.saturating_sub(self.boinc_reduce_rate);
            let min_cpu = self.config.throttling_min_boinc_cpu;
            
            let final_cpu = new_cpu.max(min_cpu);
            
            self.current_boinc_cpu.store(final_cpu as u64, Ordering::Relaxed);
            
            // Update last activity time
            if let Ok(mut last) = self.last_nuw_activity.lock() {
                *last = Some(now);
            }
            self.last_nuw_completed.store(nuw_completed, Ordering::Relaxed);
            
            debug!("NUW active - reduced BOINC CPU to {}%", final_cpu);
            
            final_cpu
        } else {
            // NUW is idle - check if we should recover
            let should_recover = {
                if let Ok(last) = self.last_nuw_activity.lock() {
                    if let Some(last_time) = *last {
                        now.duration_since(last_time).as_secs() >= self.boinc_recover_delay_secs
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            
            if should_recover {
                // Increase BOINC CPU slowly
                let current = self.current_boinc_cpu.load(Ordering::Relaxed) as u8;
                let new_cpu = current.saturating_add(self.boinc_recover_rate).min(self.base_boinc_cpu);
                
                self.current_boinc_cpu.store(new_cpu as u64, Ordering::Relaxed);
                
                debug!("NUW idle - recovering BOINC CPU to {}%", new_cpu);
                
                new_cpu
            } else {
                // Still within delay window
                let current = self.current_boinc_cpu.load(Ordering::Relaxed) as u8;
                debug!("NUW idle but within delay - BOINC CPU at {}%", current);
                current
            }
        }
    }

    /// Get current BOINC CPU percentage
    pub fn get_current_boinc_cpu(&self) -> u8 {
        self.current_boinc_cpu.load(Ordering::Relaxed) as u8
    }

    /// Adjust BOINC CPU based on oracle demand
    /// Higher demand = less BOINC CPU
    /// This is called when oracle demand is fetched
    pub fn adjust_for_demand(&self, demand_score: u8) -> u8 {
        // demand_score is 0-100
        // 0 = no demand, 100 = very high demand
        
        let current = self.get_current_boinc_cpu();
        
        if demand_score >= 80 {
            // Very high demand - reduce BOINC to minimum
            let new_cpu = self.config.throttling_min_boinc_cpu;
            self.current_boinc_cpu.store(new_cpu as u64, Ordering::Relaxed);
            debug!("Very high demand ({}) - BOINC CPU at minimum {}%", demand_score, new_cpu);
            new_cpu
        } else if demand_score >= 50 {
            // High demand - reduce BOINC significantly
            let reduction = (self.base_boinc_cpu as f64 * 0.5) as u8;
            let new_cpu = self.config.throttling_min_boinc_cpu.max(current.saturating_sub(reduction));
            self.current_boinc_cpu.store(new_cpu as u64, Ordering::Relaxed);
            debug!("High demand ({}) - BOINC CPU reduced to {}%", demand_score, new_cpu);
            new_cpu
        } else if demand_score >= 25 {
            // Medium demand - slight reduction
            let reduction = (self.base_boinc_cpu as f64 * 0.25) as u8;
            let new_cpu = current.saturating_sub(reduction).max(self.config.throttling_min_boinc_cpu);
            self.current_boinc_cpu.store(new_cpu as u64, Ordering::Relaxed);
            debug!("Medium demand ({}) - BOINC CPU reduced to {}%", demand_score, new_cpu);
            new_cpu
        } else {
            // Low/no demand - allow BOINC to use configured amount
            // This is handled by recover logic in adjust_for_nuw_activity
            debug!("Low demand ({}) - BOINC CPU unchanged at {}%", demand_score, current);
            current
        }
    }

    /// Combined adjustment: factors in both NUW activity and oracle demand
    /// This is the main method to call in the miner loop
    pub fn adjust(&self, nuw_completed: u64, oracle_demand_score: Option<u8>) -> u8 {
        // First, adjust for NUW activity
        let cpu_after_nuw = self.adjust_for_nuw_activity(nuw_completed);
        
        // Then, apply oracle demand if available
        if let Some(demand_score) = oracle_demand_score {
            // Take the more aggressive throttling
            let cpu_after_demand = self.adjust_for_demand_with_current(demand_score, cpu_after_nuw);
            return cpu_after_demand;
        }
        
        cpu_after_nuw
    }

    /// Helper to apply demand adjustment on top of current NUW-adjusted value
    fn adjust_for_demand_with_current(&self, demand_score: u8, current_cpu: u8) -> u8 {
        if demand_score >= 80 {
            let new_cpu = self.config.throttling_min_boinc_cpu;
            self.current_boinc_cpu.store(new_cpu as u64, Ordering::Relaxed);
            new_cpu
        } else if demand_score >= 50 {
            let reduction = (self.base_boinc_cpu as f64 * 0.5) as u8;
            let new_cpu = self.config.throttling_min_boinc_cpu.max(current_cpu.saturating_sub(reduction));
            self.current_boinc_cpu.store(new_cpu as u64, Ordering::Relaxed);
            new_cpu
        } else if demand_score >= 25 {
            let reduction = (self.base_boinc_cpu as f64 * 0.25) as u8;
            let new_cpu = current_cpu.saturating_sub(reduction).max(self.config.throttling_min_boinc_cpu);
            self.current_boinc_cpu.store(new_cpu as u64, Ordering::Relaxed);
            new_cpu
        } else {
            current_cpu
        }
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
        self.limit_cpu_to_percentage(limit);
    }

    /// Apply dedicated mining mode - full power
    fn apply_dedicated_mode(&self) {
        info!("Applying DEDICATED mining mode: full power");
    }

    /// Limit CPU usage by setting process priority (nice value)
    fn apply_process_priority(&self) {
        let nice_value = if self.config.low_cpu_mode {
            10 // nice value 10 = very low priority
        } else {
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

        #[cfg(windows)]
        {
            info!("Windows priority adjustment not implemented");
        }
    }

    /// Get effective CPU percentages based on mode
    pub fn get_effective_cpu_percentage(&self) -> (u8, u8) {
        let nuw_pct = if self.config.low_cpu_mode {
            (self.config.nuw_cpu_percentage as f32 * 0.5) as u8
        } else {
            self.config.nuw_cpu_percentage
        };

        let boinc_pct = if self.config.low_cpu_mode {
            self.config.low_cpu_limit.saturating_sub(nuw_pct)
        } else {
            self.get_current_boinc_cpu()
        };

        (nuw_pct, boinc_pct)
    }

    /// Get effective GPU percentage for BOINC
    pub fn get_boinc_gpu_percentage(&self) -> u8 {
        if self.config.low_cpu_mode {
            (self.config.boinc_gpu_percentage as f32 * 0.7) as u8
        } else {
            self.config.boinc_gpu_percentage
        }
    }

    /// Calculate CPU limit percentage
    fn limit_cpu_to_percentage(&self, percentage: u8) {
        info!(
            "Would limit CPU to {}% (requires cgroups/container support)",
            percentage
        );
    }

    /// Get mode description with dynamic info
    pub fn get_mode_description(&self) -> String {
        let boinc_cpu = self.get_current_boinc_cpu();
        
        if self.config.low_cpu_mode {
            format!(
                "LOW CPU MODE ({}% limit, nice={}, BOINC={}%)",
                self.config.low_cpu_limit,
                if self.config.low_cpu_mode { 10 } else { 0 },
                boinc_cpu
            )
        } else {
            format!(
                "DEDICATED MODE (NUW={}%, BOINC={}%, GPU={}%, base BOINC={}%)",
                self.config.nuw_cpu_percentage,
                boinc_cpu,
                self.config.boinc_gpu_percentage,
                self.base_boinc_cpu
            )
        }
    }

    /// Force BOINC to run at specified percentage
    /// This communicates with BOINC client via config update
    pub fn apply_boinc_cpu_limit(&self, percentage: u8) {
        info!("Applying BOINC CPU limit: {}%", percentage);
        
        // Update our tracking
        self.current_boinc_cpu.store(percentage as u64, Ordering::Relaxed);
        
        // Note: Actual BOINC CPU limiting would be done via:
        // 1. BOINC config file update (cc_config.xml)
        // 2. BOINC RPC command (boinc_cmd --set_cpu_limits)
        // For now, this is tracked for reporting purposes
    }

    /// Check if BOINC should be suspended (NUW is very active)
    pub fn should_suspend_boinc(&self) -> bool {
        let boinc_cpu = self.get_current_boinc_cpu();
        boinc_cpu <= 20 && self.last_nuw_completed.load(Ordering::Relaxed) > 0
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
        
        // Initial should be base
        assert_eq!(manager.get_current_boinc_cpu(), 50);
        
        // After NUW activity - should reduce
        let new_cpu = manager.adjust_for_nuw_activity(1);
        assert!(new_cpu < 50);
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
    
    #[test]
    fn test_dynamic_throttling_reduces_boinc() {
        let config = WorkAllocationConfig {
            low_cpu_mode: false,
            nuw_cpu_percentage: 25,
            boinc_cpu_percentage: 50,
            boinc_gpu_percentage: 100,
            ..Default::default()
        };

        let manager = ResourceManager::new(config);
        
        // Simulate NUW becoming active
        let cpu_after_activity = manager.adjust_for_nuw_activity(10);
        
        // Should be reduced from 50
        assert!(cpu_after_activity < 50);
        assert!(cpu_after_activity >= 30); // 50 - 20 = 30
    }
}
