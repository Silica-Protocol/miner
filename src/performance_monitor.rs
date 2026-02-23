//! Performance monitoring and metrics collection for the Chert miner
//!
//! This module provides comprehensive performance monitoring including:
//! - System resource usage (CPU, memory, disk)
//! - BOINC task progress and compute metrics
//! - Performance calculations (FLOPS, work rate, efficiency)
//! - Real-time logging and TUI display

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;
use tracing::info;

#[cfg(feature = "sysinfo")]
use sysinfo::{CpuExt, SystemExt};

/// System resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU usage percentage (0-100)
    pub cpu_usage: f32,
    /// Memory usage in bytes
    pub memory_used: u64,
    /// Total memory in bytes
    pub memory_total: u64,
    /// Memory usage percentage (0-100)
    pub memory_percentage: f32,
    /// Available disk space in bytes
    pub disk_available: u64,
    /// Total disk space in bytes
    pub disk_total: u64,
    /// Number of CPU cores
    pub cpu_cores: usize,
    /// System load average (1 minute)
    pub load_average: f64,
    /// Timestamp when metrics were collected
    pub timestamp: DateTime<Utc>,
}

/// BOINC task progress and compute metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoincTaskMetrics {
    /// Task/result name
    pub task_name: String,
    /// Progress fraction (0.0 to 1.0)
    pub fraction_done: f64,
    /// CPU time consumed in seconds
    pub cpu_time: f64,
    /// Elapsed wall-clock time in seconds
    pub elapsed_time: f64,
    /// Peak working set size (memory) in bytes
    pub peak_memory: u64,
    /// Peak swap usage in bytes
    pub peak_swap: u64,
    /// Peak disk usage in bytes
    pub peak_disk: u64,
    /// Estimated FLOPS (floating point operations per second)
    pub estimated_flops: f64,
    /// Current FLOPS rate
    pub current_flops_rate: f64,
    /// Timestamp when metrics were collected
    pub timestamp: DateTime<Utc>,
}

/// Performance calculation metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Current FLOPS rate (operations per second)
    pub flops_per_second: f64,
    /// Average FLOPS rate over last hour
    pub avg_flops_per_hour: f64,
    /// Work units completed per hour
    pub work_units_per_hour: f64,
    /// CPU efficiency (actual vs theoretical)
    pub cpu_efficiency: f32,
    /// Memory efficiency (used vs allocated)
    pub memory_efficiency: f32,
    /// Estimated completion time for current task
    pub estimated_completion: Option<DateTime<Utc>>,
    /// Power efficiency (FLOPS per watt estimate)
    pub power_efficiency: Option<f64>,
    /// Timestamp when metrics were calculated
    pub timestamp: DateTime<Utc>,
}

/// GPU metrics (when available)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMetrics {
    /// GPU name/model
    pub name: String,
    /// GPU utilization percentage (0-100)
    pub utilization: f32,
    /// GPU memory used in bytes
    pub memory_used: u64,
    /// GPU memory total in bytes
    pub memory_total: u64,
    /// GPU temperature in celsius
    pub temperature: f32,
    /// GPU power usage in watts
    pub power_usage: f32,
    /// Timestamp when metrics were collected
    pub timestamp: DateTime<Utc>,
}

/// Combined metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub system: SystemMetrics,
    pub boinc_task: Option<BoincTaskMetrics>,
    pub performance: PerformanceMetrics,
    pub gpu: Option<GpuMetrics>,
    pub timestamp: DateTime<Utc>,
}

/// Performance monitor with historical data
pub struct PerformanceMonitor {
    /// Historical metrics (rolling window)
    metrics_history: VecDeque<MetricsSnapshot>,
    /// Maximum history size
    max_history_size: usize,
    /// BOINC data directory path
    boinc_data_dir: String,
    /// System info collector
    #[cfg(feature = "sysinfo")]
    system: sysinfo::System,
    /// Last metrics collection time
    last_collection: Option<DateTime<Utc>>,
    /// Suppress console logging (for TUI mode)
    suppress_logging: bool,
    /// Previous BOINC metrics for rate calculations
    previous_boinc_metrics: Option<BoincTaskMetrics>,
    /// Task start time for elapsed time calculation
    task_start_time: Option<DateTime<Utc>>,
    /// Current task name to detect task changes
    current_task_name: Option<String>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(boinc_data_dir: String) -> Self {
        Self::new_with_options(boinc_data_dir, false)
    }

    /// Create a new performance monitor with options
    pub fn new_with_options(boinc_data_dir: String, suppress_logging: bool) -> Self {
        Self {
            metrics_history: VecDeque::new(),
            max_history_size: 1000, // Keep last 1000 snapshots
            boinc_data_dir,
            #[cfg(feature = "sysinfo")]
            system: sysinfo::System::new_all(),
            last_collection: None,
            suppress_logging,
            previous_boinc_metrics: None,
            task_start_time: None,
            current_task_name: None,
        }
    }

    /// Collect all metrics and create a snapshot
    pub fn collect_metrics(&mut self) -> Result<MetricsSnapshot> {
        let now = Utc::now();

        // Collect system metrics
        let system_metrics = self.collect_system_metrics()?;

        // Collect BOINC task metrics
        let boinc_metrics = self.collect_boinc_metrics().ok();

        // Calculate performance metrics
        let performance_metrics =
            self.calculate_performance_metrics(&system_metrics, &boinc_metrics)?;

        // Collect GPU metrics (if available)
        let gpu_metrics = self.collect_gpu_metrics().ok();

        let snapshot = MetricsSnapshot {
            system: system_metrics,
            boinc_task: boinc_metrics,
            performance: performance_metrics,
            gpu: gpu_metrics,
            timestamp: now,
        };

        // Add to history
        self.metrics_history.push_back(snapshot.clone());
        if self.metrics_history.len() > self.max_history_size {
            self.metrics_history.pop_front();
        }

        self.last_collection = Some(now);

        // Only log if not suppressed (traditional mode)
        if !self.suppress_logging {
            info!(
                "Collected performance metrics: CPU: {:.1}%, BOINC: {:.2}% complete",
                snapshot.system.cpu_usage,
                snapshot
                    .boinc_task
                    .as_ref()
                    .map(|t| t.fraction_done * 100.0)
                    .unwrap_or(0.0)
            );
        }

        Ok(snapshot)
    }

    /// Collect system resource metrics
    fn collect_system_metrics(&mut self) -> Result<SystemMetrics> {
        #[cfg(feature = "sysinfo")]
        {
            self.system.refresh_all();

            Ok(SystemMetrics {
                cpu_usage: self.system.global_cpu_info().cpu_usage(),
                memory_used: self.system.used_memory(),
                memory_total: self.system.total_memory(),
                memory_percentage: (self.system.used_memory() as f32
                    / self.system.total_memory() as f32)
                    * 100.0,
                disk_available: 0, // TODO: implement disk space checking
                disk_total: 0,
                cpu_cores: self.system.cpus().len(),
                load_average: self.system.load_average().one,
                timestamp: Utc::now(),
            })
        }

        #[cfg(not(feature = "sysinfo"))]
        {
            // Fallback implementation using /proc filesystem
            Ok(SystemMetrics {
                cpu_usage: self.get_cpu_usage_proc()?,
                memory_used: self.get_memory_usage_proc()?.0,
                memory_total: self.get_memory_usage_proc()?.1,
                memory_percentage: {
                    let (used, total) = self.get_memory_usage_proc()?;
                    (used as f32 / total as f32) * 100.0
                },
                disk_available: 0,
                disk_total: 0,
                cpu_cores: self.get_cpu_cores_proc()?,
                load_average: self.get_load_average_proc()?,
                timestamp: Utc::now(),
            })
        }
    }

    /// Collect BOINC task metrics from both log and state files
    fn collect_boinc_metrics(&mut self) -> Result<BoincTaskMetrics> {
        // Try to get real-time data from BOINC output log first
        if let Ok(metrics) = self.collect_boinc_metrics_from_log() {
            // Store for rate calculations
            self.update_boinc_state(&metrics);
            return Ok(metrics);
        }

        // Fallback to client state file
        if let Ok(metrics) = self.collect_boinc_metrics_from_state_file() {
            self.update_boinc_state(&metrics);
            return Ok(metrics);
        }

        // If no fresh data available, return previous metrics if available
        if let Some(ref prev) = self.previous_boinc_metrics {
            // Check if the data is still recent (within 15 seconds)
            let age = (Utc::now() - prev.timestamp).num_seconds();
            if age < 15 {
                // Create updated metrics with current timestamp but preserve the data
                return Ok(BoincTaskMetrics {
                    task_name: prev.task_name.clone(),
                    fraction_done: prev.fraction_done,
                    cpu_time: prev.cpu_time,
                    elapsed_time: if let Some(start_time) = self.task_start_time {
                        (Utc::now() - start_time).num_seconds() as f64
                    } else {
                        prev.elapsed_time
                    },
                    peak_memory: prev.peak_memory,
                    peak_swap: prev.peak_swap,
                    peak_disk: prev.peak_disk,
                    estimated_flops: prev.estimated_flops,
                    current_flops_rate: prev.current_flops_rate,
                    timestamp: Utc::now(),
                });
            }
        }

        Err(anyhow::anyhow!("No BOINC metrics available"))
    }

    /// Update internal state for task tracking
    fn update_boinc_state(&mut self, metrics: &BoincTaskMetrics) {
        // Check if this is a new task
        if self.current_task_name.as_ref() != Some(&metrics.task_name) {
            self.task_start_time = Some(Utc::now());
            self.current_task_name = Some(metrics.task_name.clone());
        }

        // Store for next iteration's rate calculation
        self.previous_boinc_metrics = Some(metrics.clone());
    }

    /// Collect BOINC metrics from real-time app_msg_receive log output
    fn collect_boinc_metrics_from_log(&self) -> Result<BoincTaskMetrics> {
        let log_path = format!("{}/boinc_output.log", self.boinc_data_dir);

        if !Path::new(&log_path).exists() {
            return Err(anyhow::anyhow!("BOINC output log not found"));
        }

        // Read the last 50 lines to get recent app messages
        let output = std::process::Command::new("tail")
            .arg("-50")
            .arg(log_path)
            .output()?;

        let log_content = String::from_utf8_lossy(&output.stdout);

        // Find the most recent app_msg_receive block
        let lines: Vec<&str> = log_content.lines().collect();
        let mut current_cpu_time = None;
        let mut checkpoint_cpu_time = None;
        let mut fraction_done = None;
        let mut task_name = String::new();

        // Search backwards through lines to find the most recent complete app message
        for i in (0..lines.len()).rev() {
            let line = lines[i];

            // Look for the start of an app_msg_receive block
            if line.contains("[app_msg_receive] got msg from slot") {
                if let Some(project_match) = line.find('[')
                    && let Some(project_end) = line[project_match + 1..].find(']')
                {
                    task_name =
                        line[project_match + 1..project_match + 1 + project_end].to_string();
                }

                // Look for the XML data in the following lines
                for &xml_line in lines.iter().skip(i + 1).take(10) {
                    if xml_line.contains("<current_cpu_time>")
                        && let Some(value) =
                            self.extract_scientific_notation(xml_line, "current_cpu_time")
                    {
                        current_cpu_time = Some(value);
                    } else if xml_line.contains("<checkpoint_cpu_time>")
                        && let Some(value) =
                            self.extract_scientific_notation(xml_line, "checkpoint_cpu_time")
                    {
                        checkpoint_cpu_time = Some(value);
                    } else if xml_line.contains("<fraction_done>")
                        && let Some(value) =
                            self.extract_scientific_notation(xml_line, "fraction_done")
                    {
                        fraction_done = Some(value);
                    }

                    // If we have all the data we need, break
                    if current_cpu_time.is_some() && fraction_done.is_some() {
                        break;
                    }
                }

                // If we found data, use it
                if current_cpu_time.is_some() && fraction_done.is_some() {
                    break;
                }
            }
        }

        let cpu_time =
            current_cpu_time.ok_or_else(|| anyhow::anyhow!("No current_cpu_time found in log"))?;
        let fraction =
            fraction_done.ok_or_else(|| anyhow::anyhow!("No fraction_done found in log"))?;
        let _checkpoint_cpu = checkpoint_cpu_time.unwrap_or(cpu_time);

        // Calculate elapsed wall-clock time since this measurement started
        let elapsed_time = if let Some(start_time) = self.task_start_time {
            (Utc::now() - start_time).num_seconds() as f64
        } else {
            // First measurement: estimate based on CPU time and progress
            // Assuming roughly real-time CPU usage (could be higher with optimization)
            if fraction > 0.001 {
                // Estimate total task time and calculate elapsed portion.
                // For now, rely on CPU time as elapsed estimate because BOINC reports
                // accumulated compute time.
                cpu_time
            } else {
                cpu_time
            }
        };

        // Get memory usage from the log if available
        let (peak_memory, peak_swap) = self.extract_memory_usage_from_log(&log_content);

        // Calculate FLOPS rate based on delta from previous measurement
        let (current_flops_rate, estimated_flops) =
            if let Some(ref prev) = self.previous_boinc_metrics {
                let time_delta = (Utc::now() - prev.timestamp).num_seconds() as f64;
                let cpu_time_delta = cpu_time - prev.cpu_time;

                if time_delta > 0.8 && time_delta < 5.0 && cpu_time_delta > 0.0 {
                    // MilkyWay@Home N-body simulations: approximately 2-5 billion FLOPS per CPU second
                    // The high CPU time increase (10+ seconds per wall-clock second) suggests
                    // the simulation is running faster than real-time
                    let flops_per_cpu_second = 3.5e9; // Conservative estimate

                    // Rate is FLOPS processed in this time period / wall-clock time
                    let flops_in_period = cpu_time_delta * flops_per_cpu_second;
                    let rate = flops_in_period / time_delta;

                    let total_flops = cpu_time * flops_per_cpu_second;
                    (rate, total_flops)
                } else {
                    // Keep previous rate if time delta is out of range or cpu delta is invalid
                    (prev.current_flops_rate, cpu_time * 3.5e9)
                }
            } else {
                // First measurement: estimate based on current CPU time
                let flops_per_cpu_second = 3.5e9;
                let total_flops = cpu_time * flops_per_cpu_second;
                // For first measurement, estimate rate based on CPU time progression
                let rate = if elapsed_time > 1.0 {
                    total_flops / elapsed_time
                } else {
                    total_flops // Assume 1 second for initial estimate
                };
                (rate, total_flops)
            };

        Ok(BoincTaskMetrics {
            task_name: if task_name.is_empty() {
                "MilkyWay@home".to_string()
            } else {
                task_name
            },
            fraction_done: fraction,
            cpu_time,
            elapsed_time,
            peak_memory,
            peak_swap,
            peak_disk: 0,
            estimated_flops,
            current_flops_rate,
            timestamp: Utc::now(),
        })
    }

    /// Collect BOINC task metrics from client state file (fallback)
    fn collect_boinc_metrics_from_state_file(&self) -> Result<BoincTaskMetrics> {
        let task_state_path = format!("{}/slots/0/boinc_task_state.xml", self.boinc_data_dir);

        if !Path::new(&task_state_path).exists() {
            return Err(anyhow::anyhow!("BOINC task state file not found"));
        }

        let xml_content = std::fs::read_to_string(&task_state_path)?;

        // Parse XML to extract metrics
        let task_name = self.extract_xml_value(&xml_content, "result_name")?;
        let fraction_done: f64 = self
            .extract_xml_value(&xml_content, "fraction_done")?
            .parse()?;
        let cpu_time: f64 = self
            .extract_xml_value(&xml_content, "checkpoint_cpu_time")?
            .parse()?;
        let elapsed_time: f64 = self
            .extract_xml_value(&xml_content, "checkpoint_elapsed_time")?
            .parse()?;
        let peak_memory: u64 = self
            .extract_xml_value(&xml_content, "peak_working_set_size")?
            .parse()?;
        let peak_swap: u64 = self
            .extract_xml_value(&xml_content, "peak_swap_size")?
            .parse()?;
        let peak_disk: u64 = self
            .extract_xml_value(&xml_content, "peak_disk_usage")?
            .parse()?;

        // Calculate FLOPS estimates
        let estimated_flops = if cpu_time > 0.0 {
            // Rough estimate: MilkyWay@Home typically does ~10^9 FLOPS per CPU second
            cpu_time * 1e9
        } else {
            0.0
        };

        let current_flops_rate = if elapsed_time > 0.0 {
            estimated_flops / elapsed_time
        } else {
            0.0
        };

        Ok(BoincTaskMetrics {
            task_name,
            fraction_done,
            cpu_time,
            elapsed_time,
            peak_memory,
            peak_swap,
            peak_disk,
            estimated_flops,
            current_flops_rate,
            timestamp: Utc::now(),
        })
    }

    /// Calculate performance metrics based on system and BOINC data
    fn calculate_performance_metrics(
        &self,
        system: &SystemMetrics,
        boinc: &Option<BoincTaskMetrics>,
    ) -> Result<PerformanceMetrics> {
        let flops_per_second = boinc.as_ref().map(|b| b.current_flops_rate).unwrap_or(0.0);

        // Calculate averages from history
        let avg_flops_per_hour = self.calculate_average_flops_rate();
        let work_units_per_hour = self.calculate_work_units_per_hour();

        // CPU efficiency: actual usage vs number of cores
        let cpu_efficiency = system.cpu_usage / (system.cpu_cores as f32 * 100.0) * 100.0;

        // Memory efficiency: used vs peak
        let memory_efficiency = if let Some(boinc) = boinc {
            if boinc.peak_memory > 0 {
                (system.memory_used as f32 / boinc.peak_memory as f32) * 100.0
            } else {
                system.memory_percentage
            }
        } else {
            system.memory_percentage
        };

        // Estimated completion time
        let estimated_completion = if let Some(boinc) = boinc {
            if boinc.fraction_done > 0.0 && boinc.elapsed_time > 0.0 {
                let total_time_estimate = boinc.elapsed_time / boinc.fraction_done;
                let remaining_time = total_time_estimate - boinc.elapsed_time;
                Some(Utc::now() + Duration::seconds(remaining_time as i64))
            } else {
                None
            }
        } else {
            None
        };

        Ok(PerformanceMetrics {
            flops_per_second,
            avg_flops_per_hour,
            work_units_per_hour,
            cpu_efficiency,
            memory_efficiency,
            estimated_completion,
            power_efficiency: None, // TODO: implement power monitoring
            timestamp: Utc::now(),
        })
    }

    /// Collect GPU metrics (placeholder for future implementation)
    fn collect_gpu_metrics(&self) -> Result<GpuMetrics> {
        // TODO: Implement NVIDIA/AMD GPU monitoring
        Err(anyhow::anyhow!("GPU monitoring not yet implemented"))
    }

    /// Extract scientific notation values from BOINC log lines
    fn extract_scientific_notation(&self, line: &str, tag: &str) -> Option<f64> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        if let Some(start_pos) = line.find(&start_tag) {
            let content_start = start_pos + start_tag.len();
            if let Some(end_pos) = line[content_start..].find(&end_tag) {
                let value_str = &line[content_start..content_start + end_pos];
                return value_str.parse::<f64>().ok();
            }
        }
        None
    }

    /// Extract memory usage from BOINC log content
    fn extract_memory_usage_from_log(&self, log_content: &str) -> (u64, u64) {
        let mut peak_memory = 0u64;
        let mut peak_swap = 0u64;

        // Look for memory usage lines like: [mem_usage] ... WS 19.75MB, ... swap 116.91MB
        for line in log_content.lines().rev() {
            if line.contains("[mem_usage]") && line.contains("WS") {
                // Extract working set size
                if let Some(ws_pos) = line.find("WS ") {
                    let ws_part = &line[ws_pos + 3..];
                    if let Some(mb_pos) = ws_part.find("MB")
                        && let Ok(mb_value) = ws_part[..mb_pos].parse::<f64>()
                    {
                        peak_memory = (mb_value * 1024.0 * 1024.0) as u64;
                    }
                }

                // Extract swap size
                if let Some(swap_pos) = line.find("swap ") {
                    let swap_part = &line[swap_pos + 5..];
                    if let Some(mb_pos) = swap_part.find("MB")
                        && let Ok(mb_value) = swap_part[..mb_pos].parse::<f64>()
                    {
                        peak_swap = (mb_value * 1024.0 * 1024.0) as u64;
                    }
                }

                // Use the first (most recent) memory usage found
                if peak_memory > 0 {
                    break;
                }
            }
        }

        (peak_memory, peak_swap)
    }

    /// Extract value from XML content
    fn extract_xml_value(&self, xml: &str, tag: &str) -> Result<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        if let Some(start_pos) = xml.find(&start_tag) {
            let content_start = start_pos + start_tag.len();
            if let Some(end_pos) = xml[content_start..].find(&end_tag) {
                return Ok(xml[content_start..content_start + end_pos].to_string());
            }
        }

        Err(anyhow::anyhow!("XML tag '{}' not found", tag))
    }

    /// Calculate average FLOPS rate from history
    fn calculate_average_flops_rate(&self) -> f64 {
        if self.metrics_history.is_empty() {
            return 0.0;
        }

        let total: f64 = self
            .metrics_history
            .iter()
            .map(|m| m.performance.flops_per_second)
            .sum();

        total / self.metrics_history.len() as f64
    }

    /// Calculate work units completed per hour
    fn calculate_work_units_per_hour(&self) -> f64 {
        // TODO: Track completed work units over time
        0.0
    }

    /// Get recent metrics history
    pub fn get_recent_metrics(&self, count: usize) -> Vec<MetricsSnapshot> {
        self.metrics_history
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get current metrics (latest snapshot)
    pub fn get_current_metrics(&self) -> Option<&MetricsSnapshot> {
        self.metrics_history.back()
    }

    #[cfg(not(feature = "sysinfo"))]
    fn get_cpu_usage_proc(&self) -> Result<f32> {
        // Read /proc/stat for CPU usage
        let stat = std::fs::read_to_string("/proc/stat")?;
        let cpu_line = stat
            .lines()
            .find(|line| line.starts_with("cpu "))
            .ok_or_else(|| anyhow::anyhow!("/proc/stat missing cpu summary"))?;

        let mut values = cpu_line
            .split_whitespace()
            .skip(1)
            .filter_map(|value| value.parse::<f64>().ok());

        let user = values.next().unwrap_or(0.0);
        let nice = values.next().unwrap_or(0.0);
        let system = values.next().unwrap_or(0.0);
        let idle = values.next().unwrap_or(0.0);
        let iowait = values.next().unwrap_or(0.0);
        let irq = values.next().unwrap_or(0.0);
        let softirq = values.next().unwrap_or(0.0);
        let steal = values.next().unwrap_or(0.0);

        let busy = user + nice + system + irq + softirq + steal;
        let total = busy + idle + iowait;

        if total == 0.0 {
            return Ok(0.0);
        }

        Ok((busy / total * 100.0) as f32)
    }

    #[cfg(not(feature = "sysinfo"))]
    fn get_memory_usage_proc(&self) -> Result<(u64, u64)> {
        // Read /proc/meminfo for memory usage
        let meminfo = std::fs::read_to_string("/proc/meminfo")?;
        let mut total = None;
        let mut available = None;

        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                total = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|value| value.parse::<u64>().ok())
                    .map(|value| value * 1024);
            } else if line.starts_with("MemAvailable:") {
                available = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|value| value.parse::<u64>().ok())
                    .map(|value| value * 1024);
            }

            if total.is_some() && available.is_some() {
                break;
            }
        }

        let total = total.ok_or_else(|| anyhow::anyhow!("MemTotal missing from /proc/meminfo"))?;
        let available = available.unwrap_or(0);
        let used = total.saturating_sub(available);

        Ok((used, total))
    }

    #[cfg(not(feature = "sysinfo"))]
    fn get_cpu_cores_proc(&self) -> Result<usize> {
        // Read /proc/cpuinfo for CPU core count
        let cpuinfo = std::fs::read_to_string("/proc/cpuinfo")?;
        let cores = cpuinfo
            .lines()
            .filter(|line| line.starts_with("processor"))
            .count();
        Ok(cores)
    }

    #[cfg(not(feature = "sysinfo"))]
    fn get_load_average_proc(&self) -> Result<f64> {
        // Read /proc/loadavg for load average
        let loadavg = std::fs::read_to_string("/proc/loadavg")?;
        let first_value = loadavg
            .split_whitespace()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid loadavg format"))?;
        Ok(first_value.parse()?)
    }
}

/// Format bytes to human readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}

/// Format FLOPS to human readable string
pub fn format_flops(flops: f64) -> String {
    const UNITS: &[&str] = &["FLOPS", "KFLOPS", "MFLOPS", "GFLOPS", "TFLOPS"];
    let mut rate = flops;
    let mut unit_index = 0;

    while rate >= 1000.0 && unit_index < UNITS.len() - 1 {
        rate /= 1000.0;
        unit_index += 1;
    }

    format!("{:.2} {}", rate, UNITS[unit_index])
}

/// Format duration to human readable string
pub fn format_duration(seconds: f64) -> String {
    if seconds < 60.0 {
        format!("{:.1}s", seconds)
    } else if seconds < 3600.0 {
        format!("{:.1}m", seconds / 60.0)
    } else if seconds < 86400.0 {
        format!("{:.1}h", seconds / 3600.0)
    } else {
        format!("{:.1}d", seconds / 86400.0)
    }
}
