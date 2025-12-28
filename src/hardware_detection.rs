//! Hardware detection and profiling module for Chert miner
//!
//! This module provides comprehensive hardware detection capabilities including:
//! - CPU profiling (cores, threads, cache, instruction sets)
//! - GPU detection (NVIDIA, AMD, Intel)
//! - Memory analysis (RAM, VRAM)
//! - Storage assessment
//! - Network evaluation

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::process::Command;
use sysinfo::{CpuExt, DiskExt, System, SystemExt};
use tracing::{debug, info};

/// Hardware capability types for work type matching
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum HardwareType {
    /// CPU only processing
    CpuOnly,
    /// GPU only processing
    GpuOnly,
    /// Both CPU and GPU processing
    Both,
    /// Unknown/undetermined capabilities
    #[default]
    Unknown,
}

impl std::fmt::Display for HardwareType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HardwareType::CpuOnly => write!(f, "CPU Only"),
            HardwareType::GpuOnly => write!(f, "GPU Only"),
            HardwareType::Both => write!(f, "CPU + GPU"),
            HardwareType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Comprehensive CPU profile information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuProfile {
    /// CPU vendor and model
    pub vendor_model: String,
    /// Number of physical cores
    pub physical_cores: usize,
    /// Number of logical threads
    pub logical_threads: usize,
    /// Cache sizes (L1, L2, L3) in KB
    pub cache_sizes: CacheSizes,
    /// Supported instruction sets
    pub instruction_sets: Vec<String>,
    /// Base clock frequency in GHz
    pub base_frequency: f64,
    /// Maximum turbo frequency in GHz
    pub max_frequency: f64,
    /// Thermal design power in watts
    pub tdp: f64,
    /// Performance score (0-100)
    pub performance_score: f64,
    /// CPU architecture (x86_64, arm64, etc.)
    pub architecture: String,
    /// CPU features for optimization
    pub features: Vec<String>,
}

/// Cache size information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheSizes {
    /// L1 cache size in KB
    pub l1_kb: u32,
    /// L2 cache size in KB
    pub l2_kb: u32,
    /// L3 cache size in KB
    pub l3_kb: u32,
}

/// Comprehensive GPU profile information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GpuProfile {
    /// GPU vendor and model
    pub vendor_model: String,
    /// GPU vendor (NVIDIA, AMD, Intel)
    pub vendor: GpuVendor,
    /// Available VRAM in bytes
    pub total_memory: u64,
    /// Compute capability (CUDA version for NVIDIA)
    pub compute_capability: Option<(u8, u8)>,
    /// CUDA cores or stream processors
    pub processor_count: u32,
    /// Memory bandwidth in GB/s
    pub memory_bandwidth: f64,
    /// Supported features
    pub features: Vec<String>,
    /// Performance score (0-100)
    pub performance_score: f64,
    /// Driver version
    pub driver_version: String,
    /// GPU architecture name
    pub architecture: String,
}

/// GPU vendor enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    #[default]
    Unknown,
}

impl std::fmt::Display for GpuVendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuVendor::Nvidia => write!(f, "NVIDIA"),
            GpuVendor::Amd => write!(f, "AMD"),
            GpuVendor::Intel => write!(f, "Intel"),
            GpuVendor::Unknown => write!(f, "Unknown"),
        }
    }
}

/// System information for hardware profiling
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemInfo {
    /// Total system memory in bytes
    pub total_memory: u64,
    /// Available memory in bytes
    pub available_memory: u64,
    /// Total disk space in bytes
    pub total_disk_space: u64,
    /// Available disk space in bytes
    pub available_disk_space: u64,
    /// Network connectivity status
    pub network_status: NetworkStatus,
    /// Operating system information
    pub os_info: OsInfo,
}

/// Network status information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkStatus {
    /// Network connectivity available
    pub connectivity: bool,
    /// Estimated bandwidth in Mbps
    pub bandwidth_mbps: Option<f64>,
    /// Latency in milliseconds
    pub latency_ms: Option<f64>,
    /// Connection type (ethernet, wifi, etc.)
    pub connection_type: Option<String>,
}

/// Operating system information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OsInfo {
    /// OS name (Linux, Windows, macOS)
    pub name: String,
    /// OS version
    pub version: String,
    /// Architecture (x86_64, arm64, etc.)
    pub architecture: String,
    /// Kernel version
    pub kernel_version: String,
}

/// Complete hardware profile
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardwareProfile {
    /// CPU profile information
    pub cpu: CpuProfile,
    /// GPU profiles (multiple GPUs supported)
    pub gpus: Vec<GpuProfile>,
    /// System information
    pub system: SystemInfo,
    /// Overall hardware capability type
    pub hardware_type: HardwareType,
    /// Hardware compatibility score for different work types
    pub compatibility_scores: HashMap<String, f64>,
    /// Recommended configuration
    pub recommended_config: RecommendedConfig,
}

/// Recommended configuration based on hardware profile
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecommendedConfig {
    /// Recommended work allocation
    pub work_allocation: WorkAllocationRecommendation,
    /// Recommended BOINC projects
    pub recommended_projects: Vec<String>,
    /// Performance optimization settings
    pub optimization_settings: OptimizationSettings,
}

/// Work allocation recommendations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkAllocationRecommendation {
    /// NUW on CPU recommended
    pub nuw_on_cpu: bool,
    /// BOINC on GPU recommended
    pub boinc_on_gpu: bool,
    /// Recommended CPU percentage for NUW
    pub nuw_cpu_percentage: u8,
    /// Recommended GPU percentage for BOINC
    pub boinc_gpu_percentage: u8,
    /// Maximum concurrent BOINC tasks
    pub max_boinc_tasks: u8,
}

/// Performance optimization settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OptimizationSettings {
    /// Recommended memory limit percentage
    pub memory_limit_percentage: u8,
    /// Maximum temperature in Celsius
    pub max_temperature_celsius: f64,
    /// Recommended checkpoint interval in seconds
    pub checkpoint_interval_seconds: u64,
    /// Thread affinity recommendations
    pub thread_affinity: Option<Vec<usize>>,
}

/// Hardware detection and profiling engine
pub struct HardwareDetector {
    system: System,
}

impl HardwareDetector {
    /// Create a new hardware detector
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
    }

    /// Perform comprehensive hardware detection
    pub fn detect_hardware(&mut self) -> Result<HardwareProfile> {
        info!("Starting comprehensive hardware detection");

        // Detect CPU information
        let cpu = self.detect_cpu_profile()?;

        // Detect GPU information
        let gpus = self.detect_gpu_profiles()?;

        // Detect system information
        let system = self.detect_system_info()?;

        // Determine hardware type
        let hardware_type = self.determine_hardware_type(&cpu, &gpus);

        // Calculate compatibility scores
        let compatibility_scores = self.calculate_compatibility_scores(&cpu, &gpus, &system);

        // Generate recommendations
        let recommended_config = self.generate_recommendations(&cpu, &gpus, &system)?;

        let profile = HardwareProfile {
            cpu,
            gpus,
            system,
            hardware_type,
            compatibility_scores,
            recommended_config,
        };

        info!("Hardware detection completed successfully");
        debug!("Hardware profile: {:?}", profile);

        Ok(profile)
    }

    /// Detect CPU profile information
    fn detect_cpu_profile(&mut self) -> Result<CpuProfile> {
        debug!("Detecting CPU profile");

        let cpus = self.system.cpus();
        if cpus.is_empty() {
            return Err(anyhow::anyhow!("No CPUs detected"));
        }

        // Get CPU information from first CPU (assuming homogeneous system)
        let cpu = &cpus[0];
        let vendor_model = format!("{} {}", cpu.vendor_id(), cpu.brand());

        // Count physical cores and logical threads
        let physical_cores = num_cpus::get_physical();
        let logical_threads = num_cpus::get();

        // Detect cache sizes
        let cache_sizes = self.detect_cache_sizes()?;

        // Detect instruction sets
        let instruction_sets = self.detect_instruction_sets()?;

        // Detect CPU frequencies
        let (base_frequency, max_frequency) = self.detect_cpu_frequencies()?;

        // Estimate TDP based on CPU type and cores
        let tdp = self.estimate_cpu_tdp(&vendor_model, physical_cores);

        // Calculate performance score
        let performance_score = self.calculate_cpu_performance_score(
            physical_cores,
            logical_threads,
            base_frequency,
            &cache_sizes,
        );

        // Detect architecture
        let architecture = std::env::consts::ARCH.to_string();

        // Detect CPU features
        let features = self.detect_cpu_features();

        Ok(CpuProfile {
            vendor_model,
            physical_cores,
            logical_threads,
            cache_sizes,
            instruction_sets,
            base_frequency,
            max_frequency,
            tdp,
            performance_score,
            architecture,
            features,
        })
    }

    /// Detect cache sizes
    fn detect_cache_sizes(&self) -> Result<CacheSizes> {
        // Try to read cache information from /sys on Linux
        if cfg!(target_os = "linux") {
            if let Ok(l1_cache) =
                self.read_cache_size("/sys/devices/system/cpu/cpu0/cache/index0/size")
            {
                if let Ok(l2_cache) =
                    self.read_cache_size("/sys/devices/system/cpu/cpu0/cache/index1/size")
                {
                    if let Ok(l3_cache) =
                        self.read_cache_size("/sys/devices/system/cpu/cpu0/cache/index2/size")
                    {
                        return Ok(CacheSizes {
                            l1_kb: l1_cache,
                            l2_kb: l2_cache,
                            l3_kb: l3_cache,
                        });
                    }
                }
            }
        }

        // Fallback to reasonable defaults
        Ok(CacheSizes {
            l1_kb: 32,   // Typical L1 cache
            l2_kb: 256,  // Typical L2 cache
            l3_kb: 8192, // Typical L3 cache
        })
    }

    /// Read cache size from sysfs
    fn read_cache_size(&self, path: &str) -> Result<u32> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read cache size from {}", path))?;

        let size_str = content.trim().trim_end_matches('K');
        let size_kb: u32 = size_str
            .parse()
            .with_context(|| format!("Failed to parse cache size: {}", size_str))?;

        Ok(size_kb)
    }

    /// Detect CPU instruction sets
    fn detect_instruction_sets(&self) -> Result<Vec<String>> {
        let mut instruction_sets = Vec::new();

        // Check for common instruction sets
        if cfg!(target_arch = "x86_64") {
            // Check /proc/cpuinfo for instruction sets
            if cfg!(target_os = "linux") {
                if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
                    for line in cpuinfo.lines() {
                        if line.starts_with("flags") {
                            let flags = line.split(':').nth(1).unwrap_or("");
                            let flag_list = flags.split_whitespace();
                            for flag in flag_list {
                                match flag {
                                    "sse" | "sse2" | "sse3" | "sse4_1" | "sse4_2" | "avx"
                                    | "avx2" | "avx512f" | "fma" | "fma3" => {
                                        instruction_sets.push(flag.to_uppercase());
                                    }
                                    _ => {}
                                }
                            }
                            break;
                        }
                    }
                }
            }

            // Add guaranteed instruction sets for x86_64
            instruction_sets.extend_from_slice(&["SSE2".to_string(), "X86_64".to_string()]);
        } else if cfg!(target_arch = "aarch64") {
            instruction_sets.extend_from_slice(&["NEON".to_string(), "ARM64".to_string()]);
        }

        Ok(instruction_sets)
    }

    /// Detect CPU frequencies
    fn detect_cpu_frequencies(&self) -> Result<(f64, f64)> {
        if cfg!(target_os = "linux") {
            // Try to read from /proc/cpuinfo
            if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
                let mut base_freq: f64 = 0.0;
                let mut max_freq: f64 = 0.0;

                for line in cpuinfo.lines() {
                    if line.starts_with("cpu MHz") {
                        if let Some(freq_str) = line.split(':').nth(1) {
                            if let Ok(mhz) = freq_str.trim().parse::<f64>() {
                                let ghz = mhz / 1000.0;
                                base_freq = base_freq.max(ghz);
                                max_freq = max_freq.max(ghz);
                            }
                        }
                    }
                }

                if base_freq > 0.0 {
                    return Ok((base_freq, max_freq));
                }
            }

            // Try to read from /sys/devices/system/cpu/cpu0/cpufreq
            if let Ok(base_freq_str) =
                fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/base_frequency")
            {
                if let Ok(base_freq_hz) = base_freq_str.trim().parse::<u64>() {
                    let base_freq_ghz = base_freq_hz as f64 / 1_000_000_000.0;

                    if let Ok(max_freq_str) =
                        fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_max_freq")
                    {
                        if let Ok(max_freq_hz) = max_freq_str.trim().parse::<u64>() {
                            let max_freq_ghz = max_freq_hz as f64 / 1_000_000_000.0;
                            return Ok((base_freq_ghz, max_freq_ghz));
                        }
                    }

                    return Ok((base_freq_ghz, base_freq_ghz));
                }
            }
        }

        // Fallback to reasonable defaults
        Ok((2.0, 3.0)) // 2.0 GHz base, 3.0 GHz turbo
    }

    /// Estimate CPU TDP based on model and cores
    fn estimate_cpu_tdp(&self, vendor_model: &str, physical_cores: usize) -> f64 {
        let model_lower = vendor_model.to_lowercase();

        // Intel CPU TDP estimation
        if model_lower.contains("intel") {
            if model_lower.contains("i9") || model_lower.contains("xeon") {
                95.0 + (physical_cores as f64 * 2.5)
            } else if model_lower.contains("i7") {
                65.0 + (physical_cores as f64 * 2.0)
            } else if model_lower.contains("i5") {
                65.0 + (physical_cores as f64 * 1.5)
            } else if model_lower.contains("i3") {
                45.0 + (physical_cores as f64 * 1.0)
            } else {
                65.0 // Default Intel
            }
        }
        // AMD CPU TDP estimation
        else if model_lower.contains("amd") || model_lower.contains("ryzen") {
            if model_lower.contains("threadripper") || model_lower.contains("epyc") {
                180.0 + (physical_cores as f64 * 3.0)
            } else if model_lower.contains("ryzen 9") {
                105.0 + (physical_cores as f64 * 2.5)
            } else if model_lower.contains("ryzen 7") {
                65.0 + (physical_cores as f64 * 2.0)
            } else if model_lower.contains("ryzen 5") {
                65.0 + (physical_cores as f64 * 1.5)
            } else {
                65.0 // Default AMD
            }
        }
        // ARM CPU TDP estimation
        else if model_lower.contains("arm") {
            15.0 + (physical_cores as f64 * 0.5)
        }
        // Default fallback
        else {
            65.0
        }
    }

    /// Calculate CPU performance score
    fn calculate_cpu_performance_score(
        &self,
        physical_cores: usize,
        logical_threads: usize,
        base_frequency: f64,
        cache_sizes: &CacheSizes,
    ) -> f64 {
        // Base score from cores and frequency
        let core_score = (physical_cores as f64 * base_frequency) * 10.0;

        // Bonus for hyperthreading
        let hyperthreading_bonus = if logical_threads > physical_cores {
            (logical_threads as f64 / physical_cores as f64 - 1.0) * 20.0
        } else {
            0.0
        };

        // Cache score (normalized)
        let cache_score = ((cache_sizes.l3_kb as f64).ln() / 10.0).min(20.0);

        // Total score (capped at 100)
        (core_score + hyperthreading_bonus + cache_score).min(100.0)
    }

    /// Detect CPU features
    fn detect_cpu_features(&self) -> Vec<String> {
        let mut features = Vec::new();

        if cfg!(target_arch = "x86_64") {
            features.push("64-bit".to_string());

            // Check for specific features
            if cfg!(target_feature = "avx2") {
                features.push("AVX2".to_string());
            }
            if cfg!(target_feature = "fma") {
                features.push("FMA".to_string());
            }
        } else if cfg!(target_arch = "aarch64") {
            features.push("64-bit".to_string());
            features.push("NEON".to_string());
        }

        features
    }

    /// Detect GPU profiles
    fn detect_gpu_profiles(&self) -> Result<Vec<GpuProfile>> {
        debug!("Detecting GPU profiles");
        let mut gpus = Vec::new();

        // Try NVIDIA GPUs first
        if let Ok(nvidia_gpus) = self.detect_nvidia_gpus() {
            gpus.extend(nvidia_gpus);
        }

        // Try AMD GPUs
        if let Ok(amd_gpus) = self.detect_amd_gpus() {
            gpus.extend(amd_gpus);
        }

        // Try Intel GPUs
        if let Ok(intel_gpus) = self.detect_intel_gpus() {
            gpus.extend(intel_gpus);
        }

        info!("Detected {} GPU(s)", gpus.len());
        Ok(gpus)
    }

    /// Detect NVIDIA GPUs
    fn detect_nvidia_gpus(&self) -> Result<Vec<GpuProfile>> {
        let mut gpus = Vec::new();

        // Try nvidia-smi
        if let Ok(output) = Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,memory.total,compute_cap,driver_version",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if let Some(gpu) = self.parse_nvidia_gpu_line(line) {
                        gpus.push(gpu);
                    }
                }
            }
        }

        Ok(gpus)
    }

    /// Parse NVIDIA GPU information from nvidia-smi output
    fn parse_nvidia_gpu_line(&self, line: &str) -> Option<GpuProfile> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 4 {
            return None;
        }

        let name = parts[0].trim().to_string();
        let memory_mb: u64 = parts[1].trim().parse().ok()?;
        let compute_cap_str = parts[2].trim();
        let driver_version = parts[3].trim().to_string();

        // Parse compute capability
        let compute_capability = if compute_cap_str.contains('.') {
            let caps: Vec<&str> = compute_cap_str.split('.').collect();
            if caps.len() == 2 {
                Some((caps[0].parse().ok()?, caps[1].parse().ok()?))
            } else {
                None
            }
        } else {
            None
        };

        // Estimate processor count and memory bandwidth based on GPU name
        let (processor_count, memory_bandwidth) = self.estimate_nvidia_specs(&name);

        // Calculate performance score
        let performance_score = self.calculate_gpu_performance_score(
            memory_mb * 1024 * 1024,
            processor_count,
            memory_bandwidth,
            compute_capability,
        );

        Some(GpuProfile {
            vendor_model: name.clone(),
            vendor: GpuVendor::Nvidia,
            total_memory: memory_mb * 1024 * 1024,
            compute_capability,
            processor_count,
            memory_bandwidth,
            features: self.get_nvidia_features(compute_capability),
            performance_score,
            driver_version,
            architecture: self.get_nvidia_architecture(compute_capability),
        })
    }

    /// Estimate NVIDIA GPU specifications based on model name
    fn estimate_nvidia_specs(&self, model: &str) -> (u32, f64) {
        let model_lower = model.to_lowercase();

        if model_lower.contains("rtx 4090") {
            (16384, 1008.0)
        } else if model_lower.contains("rtx 4080") {
            (9728, 716.8)
        } else if model_lower.contains("rtx 4070") {
            (5888, 504.2)
        } else if model_lower.contains("rtx 4060") {
            (3072, 272.0)
        } else if model_lower.contains("rtx 3090") {
            (10496, 936.0)
        } else if model_lower.contains("rtx 3080") {
            (8704, 760.0)
        } else if model_lower.contains("rtx 3070") {
            (5888, 448.0)
        } else if model_lower.contains("rtx 3060") {
            (3584, 360.0)
        } else if model_lower.contains("rtx 2080") {
            (2944, 448.0)
        } else if model_lower.contains("rtx 2070") {
            (2304, 448.0)
        } else if model_lower.contains("rtx 2060") {
            (1920, 336.0)
        } else if model_lower.contains("gtx 1080") {
            (2560, 320.0)
        } else if model_lower.contains("gtx 1070") {
            (1920, 256.0)
        } else if model_lower.contains("gtx 1060") {
            (1280, 192.0)
        } else {
            // Default estimates
            (2048, 256.0)
        }
    }

    /// Get NVIDIA GPU features based on compute capability
    fn get_nvidia_features(&self, compute_cap: Option<(u8, u8)>) -> Vec<String> {
        let mut features = vec!["CUDA".to_string()];

        if let Some((major, minor)) = compute_cap {
            features.push(format!("Compute {}.{}", major, minor));

            if major >= 7 {
                features.push("Tensor Cores".to_string());
            }
            if major >= 8 {
                features.push("RT Cores".to_string());
            }
            if major >= 6 {
                features.push("FP16".to_string());
            }
            if major >= 7 {
                features.push("TensorFloat-32".to_string());
            }
        }

        features
    }

    /// Get NVIDIA GPU architecture name
    fn get_nvidia_architecture(&self, compute_cap: Option<(u8, u8)>) -> String {
        if let Some((major, _)) = compute_cap {
            match major {
                8 => "Ampere".to_string(),
                7 => "Turing".to_string(),
                6 => "Pascal".to_string(),
                5 => "Maxwell".to_string(),
                3 => "Kepler".to_string(),
                _ => "Unknown".to_string(),
            }
        } else {
            "Unknown".to_string()
        }
    }

    /// Detect AMD GPUs
    fn detect_amd_gpus(&self) -> Result<Vec<GpuProfile>> {
        let mut gpus = Vec::new();

        // Try rocm-smi
        if let Ok(output) = Command::new("rocm-smi")
            .args(["--showproductname", "--showmem", "--csv"])
            .output()
        {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Parse ROCm output (simplified)
                for line in output_str.lines() {
                    if let Some(gpu) = self.parse_amd_gpu_line(line) {
                        gpus.push(gpu);
                    }
                }
            }
        }

        Ok(gpus)
    }

    /// Parse AMD GPU information from ROCm output
    fn parse_amd_gpu_line(&self, line: &str) -> Option<GpuProfile> {
        // Simplified AMD GPU parsing
        // In a real implementation, this would be more sophisticated
        if line.contains("AMD") || line.contains("Radeon") {
            let name = line.trim().to_string();

            // Estimate specs based on model name
            let (memory_mb, processor_count, memory_bandwidth) = self.estimate_amd_specs(&name);

            let performance_score = self.calculate_gpu_performance_score(
                memory_mb * 1024 * 1024,
                processor_count,
                memory_bandwidth,
                None,
            );

            Some(GpuProfile {
                vendor_model: name.clone(),
                vendor: GpuVendor::Amd,
                total_memory: memory_mb * 1024 * 1024,
                compute_capability: None,
                processor_count,
                memory_bandwidth,
                features: vec!["OpenCL".to_string(), "Vulkan".to_string()],
                performance_score,
                driver_version: "Unknown".to_string(),
                architecture: "RDNA".to_string(), // Simplified
            })
        } else {
            None
        }
    }

    /// Estimate AMD GPU specifications based on model name
    fn estimate_amd_specs(&self, model: &str) -> (u64, u32, f64) {
        let model_lower = model.to_lowercase();

        if model_lower.contains("rx 7900") {
            (24576, 6144, 960.0)
        } else if model_lower.contains("rx 7800") {
            (16384, 3840, 624.0)
        } else if model_lower.contains("rx 7700") {
            (12288, 3072, 576.0)
        } else if model_lower.contains("rx 6900") {
            (16384, 5120, 512.0)
        } else if model_lower.contains("rx 6800") {
            (16384, 3840, 512.0)
        } else if model_lower.contains("rx 6700") {
            (12288, 2560, 448.0)
        } else if model_lower.contains("rx 5700") {
            (8192, 2304, 448.0)
        } else if model_lower.contains("rx 5600") {
            (6144, 2048, 288.0)
        } else if model_lower.contains("rx 5500") {
            (8192, 1408, 224.0)
        } else {
            // Default estimates
            (8192, 2048, 256.0)
        }
    }

    /// Detect Intel GPUs
    fn detect_intel_gpus(&self) -> Result<Vec<GpuProfile>> {
        let mut gpus = Vec::new();

        // Try to detect integrated Intel GPUs
        if cfg!(target_os = "linux") {
            if let Ok(output) = Command::new("lspci")
                .args(["-nn", "|", "grep", "-i", "vga\\|display"])
                .output()
            {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    for line in output_str.lines() {
                        if line.contains("Intel")
                            && (line.contains("HD Graphics")
                                || line.contains("Iris")
                                || line.contains("UHD")
                                || line.contains("Arc"))
                        {
                            if let Some(gpu) = self.parse_intel_gpu_line(line) {
                                gpus.push(gpu);
                            }
                        }
                    }
                }
            }
        }

        Ok(gpus)
    }

    /// Parse Intel GPU information from lspci output
    fn parse_intel_gpu_line(&self, line: &str) -> Option<GpuProfile> {
        // Extract GPU name from lspci output
        let name_start = line.find("Intel")?;
        let name_part = &line[name_start..];
        let name_end = name_part.find('[').unwrap_or(name_part.len());
        let name = name_part[..name_end].trim().to_string();

        // Estimate specs based on Intel GPU model
        let (memory_mb, processor_count, memory_bandwidth) = self.estimate_intel_specs(&name);

        let performance_score = self.calculate_gpu_performance_score(
            memory_mb * 1024 * 1024,
            processor_count,
            memory_bandwidth,
            None,
        );

        Some(GpuProfile {
            vendor_model: name.clone(),
            vendor: GpuVendor::Intel,
            total_memory: memory_mb * 1024 * 1024,
            compute_capability: None,
            processor_count,
            memory_bandwidth,
            features: vec![
                "OpenCL".to_string(),
                "Vulkan".to_string(),
                "Quick Sync".to_string(),
            ],
            performance_score,
            driver_version: "Unknown".to_string(),
            architecture: "Xe".to_string(), // Modern Intel GPUs
        })
    }

    /// Estimate Intel GPU specifications based on model name
    fn estimate_intel_specs(&self, model: &str) -> (u64, u32, f64) {
        let model_lower = model.to_lowercase();

        if model_lower.contains("arc a770") {
            (16384, 4096, 560.0)
        } else if model_lower.contains("arc a750") {
            (8192, 3072, 448.0)
        } else if model_lower.contains("arc a380") {
            (6144, 1024, 192.0)
        } else if model_lower.contains("iris xe") || model_lower.contains("uhd 770") {
            (4096, 96, 68.0)
        } else if model_lower.contains("uhd 630") || model_lower.contains("hd 630") {
            (4096, 24, 42.0)
        } else {
            // Default integrated GPU estimates
            (2048, 24, 34.0)
        }
    }

    /// Calculate GPU performance score
    fn calculate_gpu_performance_score(
        &self,
        memory_bytes: u64,
        processor_count: u32,
        memory_bandwidth: f64,
        compute_capability: Option<(u8, u8)>,
    ) -> f64 {
        // Base score from memory and processors
        let memory_score = (memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0)) * 5.0; // GB * 5
        let processor_score = (processor_count as f64 / 1000.0) * 30.0; // Thousands of cores * 30
        let bandwidth_score = (memory_bandwidth / 100.0) * 20.0; // GB/s / 100 * 20

        // Compute capability bonus for NVIDIA
        let compute_bonus = if let Some((major, minor)) = compute_capability {
            match major {
                8 => 25.0 + (minor as f64 * 2.5),
                7 => 20.0 + (minor as f64 * 2.0),
                6 => 15.0 + (minor as f64 * 1.5),
                _ => 10.0,
            }
        } else {
            10.0 // Non-NVIDIA GPUs get base bonus
        };

        // Total score (capped at 100)
        (memory_score + processor_score + bandwidth_score + compute_bonus).min(100.0)
    }

    /// Detect system information
    fn detect_system_info(&mut self) -> Result<SystemInfo> {
        debug!("Detecting system information");

        // Memory information
        let total_memory = self.system.total_memory();
        let available_memory = self.system.available_memory();

        // Disk information
        let (total_disk_space, available_disk_space) = self.detect_disk_info()?;

        // Network status
        let network_status = self.detect_network_status()?;

        // OS information
        let os_info = self.detect_os_info()?;

        Ok(SystemInfo {
            total_memory,
            available_memory,
            total_disk_space,
            available_disk_space,
            network_status,
            os_info,
        })
    }

    /// Detect disk information
    fn detect_disk_info(&self) -> Result<(u64, u64)> {
        let mut total_space = 0u64;
        let mut available_space = 0u64;

        for disk in self.system.disks() {
            total_space += disk.total_space();
            available_space += disk.available_space();
        }

        Ok((total_space, available_space))
    }

    /// Detect network status
    fn detect_network_status(&self) -> Result<NetworkStatus> {
        // Basic connectivity check
        let connectivity = self.check_network_connectivity();

        let (bandwidth_mbps, latency_ms) = if connectivity {
            self.measure_network_performance()
        } else {
            (None, None)
        };

        Ok(NetworkStatus {
            connectivity,
            bandwidth_mbps,
            latency_ms,
            connection_type: None, // Could be detected with more complex logic
        })
    }

    /// Check basic network connectivity
    fn check_network_connectivity(&self) -> bool {
        // Try to ping a reliable host
        Command::new("ping")
            .args(["-c", "1", "-W", "5", "8.8.8.8"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Measure network performance
    fn measure_network_performance(&self) -> (Option<f64>, Option<f64>) {
        // This is a simplified implementation
        // In practice, you'd want more sophisticated network testing

        // Measure latency with ping
        let latency_ms =
            if let Ok(output) = Command::new("ping").args(["-c", "3", "8.8.8.8"]).output() {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    self.extract_ping_latency(&output_str)
                } else {
                    None
                }
            } else {
                None
            };

        // Bandwidth measurement would require more complex implementation
        // For now, return None
        (None, latency_ms)
    }

    /// Extract latency from ping output
    fn extract_ping_latency(&self, ping_output: &str) -> Option<f64> {
        for line in ping_output.lines() {
            if line.contains("time=") {
                if let Some(time_part) = line.split("time=").nth(1) {
                    if let Some(time_str) = time_part.split(' ').next() {
                        return time_str.parse::<f64>().ok();
                    }
                }
            }
        }
        None
    }

    /// Detect OS information
    fn detect_os_info(&self) -> Result<OsInfo> {
        let name = std::env::consts::OS.to_string();
        let architecture = std::env::consts::ARCH.to_string();

        let (version, kernel_version) = if cfg!(target_os = "linux") {
            let version = self.get_linux_version();
            let kernel = self.get_kernel_version();
            (version, kernel)
        } else {
            ("Unknown".to_string(), "Unknown".to_string())
        };

        Ok(OsInfo {
            name,
            version,
            architecture,
            kernel_version,
        })
    }

    /// Get Linux distribution version
    fn get_linux_version(&self) -> String {
        if let Ok(content) = fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    return line
                        .split('=')
                        .nth(1)
                        .unwrap_or("Unknown")
                        .trim_matches('"')
                        .to_string();
                }
            }
        }
        "Unknown".to_string()
    }

    /// Get kernel version
    fn get_kernel_version(&self) -> String {
        if let Ok(output) = Command::new("uname").args(["-r"]).output() {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }
        "Unknown".to_string()
    }

    /// Determine overall hardware type
    fn determine_hardware_type(&self, cpu: &CpuProfile, gpus: &[GpuProfile]) -> HardwareType {
        if gpus.is_empty() {
            HardwareType::CpuOnly
        } else if cpu.performance_score < 30.0 {
            HardwareType::GpuOnly
        } else {
            HardwareType::Both
        }
    }

    /// Calculate compatibility scores for different work types
    fn calculate_compatibility_scores(
        &self,
        cpu: &CpuProfile,
        gpus: &[GpuProfile],
        system: &SystemInfo,
    ) -> HashMap<String, f64> {
        let mut scores = HashMap::new();

        // NUW CPU compatibility
        let nuw_cpu_score = self.calculate_nuw_cpu_score(cpu, system);
        scores.insert("nuw_cpu".to_string(), nuw_cpu_score);

        // BOINC CPU compatibility
        let boinc_cpu_score = self.calculate_boinc_cpu_score(cpu, system);
        scores.insert("boinc_cpu".to_string(), boinc_cpu_score);

        // BOINC GPU compatibility
        let boinc_gpu_score = self.calculate_boinc_gpu_score(gpus, system);
        scores.insert("boinc_gpu".to_string(), boinc_gpu_score);

        scores
    }

    /// Calculate NUW CPU compatibility score
    fn calculate_nuw_cpu_score(&self, cpu: &CpuProfile, system: &SystemInfo) -> f64 {
        let cpu_score = cpu.performance_score;
        let memory_score =
            (system.total_memory as f64 / (1024.0 * 1024.0 * 1024.0)).min(16.0) / 16.0 * 20.0;
        let instruction_bonus = if cpu.instruction_sets.contains(&"AVX2".to_string()) {
            10.0
        } else {
            0.0
        };

        (cpu_score + memory_score + instruction_bonus).min(100.0)
    }

    /// Calculate BOINC CPU compatibility score
    fn calculate_boinc_cpu_score(&self, cpu: &CpuProfile, system: &SystemInfo) -> f64 {
        let cpu_score = cpu.performance_score;
        let memory_score =
            (system.total_memory as f64 / (1024.0 * 1024.0 * 1024.0)).min(32.0) / 32.0 * 20.0;
        let cache_score = ((cpu.cache_sizes.l3_kb as f64).ln() / 10.0).min(15.0);

        (cpu_score + memory_score + cache_score).min(100.0)
    }

    /// Calculate BOINC GPU compatibility score
    fn calculate_boinc_gpu_score(&self, gpus: &[GpuProfile], _system: &SystemInfo) -> f64 {
        if gpus.is_empty() {
            return 0.0;
        }

        let best_gpu = gpus
            .iter()
            .max_by(|a, b| {
                a.performance_score
                    .partial_cmp(&b.performance_score)
                    .unwrap()
            })
            .unwrap();
        let gpu_score = best_gpu.performance_score;
        let memory_score =
            (best_gpu.total_memory as f64 / (1024.0 * 1024.0 * 1024.0)).min(24.0) / 24.0 * 20.0;

        (gpu_score + memory_score).min(100.0)
    }

    /// Generate configuration recommendations
    fn generate_recommendations(
        &self,
        cpu: &CpuProfile,
        gpus: &[GpuProfile],
        system: &SystemInfo,
    ) -> Result<RecommendedConfig> {
        let work_allocation = self.recommend_work_allocation(cpu, gpus);
        let recommended_projects = self.recommend_projects(cpu, gpus);
        let optimization_settings = self.recommend_optimization_settings(cpu, gpus, system);

        Ok(RecommendedConfig {
            work_allocation,
            recommended_projects,
            optimization_settings,
        })
    }

    /// Recommend work allocation settings
    fn recommend_work_allocation(
        &self,
        cpu: &CpuProfile,
        gpus: &[GpuProfile],
    ) -> WorkAllocationRecommendation {
        if gpus.is_empty() {
            // CPU only system
            WorkAllocationRecommendation {
                nuw_on_cpu: false,
                boinc_on_gpu: false,
                nuw_cpu_percentage: 0,
                boinc_gpu_percentage: 0,
                max_boinc_tasks: (cpu.physical_cores / 2).max(1) as u8,
            }
        } else {
            // System with GPU
            let best_gpu = gpus
                .iter()
                .max_by(|a, b| {
                    a.performance_score
                        .partial_cmp(&b.performance_score)
                        .unwrap()
                })
                .unwrap();

            if best_gpu.performance_score > cpu.performance_score * 1.5 {
                // GPU is significantly more powerful
                WorkAllocationRecommendation {
                    nuw_on_cpu: true,
                    boinc_on_gpu: true,
                    nuw_cpu_percentage: 25,
                    boinc_gpu_percentage: 80,
                    max_boinc_tasks: 2,
                }
            } else {
                // CPU is competitive
                WorkAllocationRecommendation {
                    nuw_on_cpu: true,
                    boinc_on_gpu: true,
                    nuw_cpu_percentage: 40,
                    boinc_gpu_percentage: 60,
                    max_boinc_tasks: 2,
                }
            }
        }
    }

    /// Recommend BOINC projects based on hardware
    fn recommend_projects(&self, cpu: &CpuProfile, gpus: &[GpuProfile]) -> Vec<String> {
        let mut projects = Vec::new();

        // Always recommend MilkyWay@Home for GPU systems
        if !gpus.is_empty() {
            projects.push("MilkyWay@Home".to_string());
        }

        // Recommend CPU-intensive projects for strong CPUs
        if cpu.performance_score > 60.0 {
            projects.push("Rosetta@Home".to_string());
        }

        // Recommend additional projects based on specific hardware
        for gpu in gpus {
            if gpu.vendor == GpuVendor::Nvidia && gpu.compute_capability.unwrap_or((0, 0)).0 >= 6 {
                projects.push("GPUGRID".to_string());
            }
        }

        // Add World Community Grid as a general recommendation
        projects.push("World Community Grid".to_string());

        projects
    }

    /// Recommend optimization settings
    fn recommend_optimization_settings(
        &self,
        _cpu: &CpuProfile,
        gpus: &[GpuProfile],
        system: &SystemInfo,
    ) -> OptimizationSettings {
        let memory_limit_percentage = if system.total_memory > 16 * 1024 * 1024 * 1024 {
            80
        } else {
            70
        };

        let max_temperature_celsius = if !gpus.is_empty() {
            85.0 // GPU systems can handle higher temps
        } else {
            75.0 // CPU-only systems should be more conservative
        };

        let checkpoint_interval_seconds = if !gpus.is_empty() {
            300 // 5 minutes for GPU work
        } else {
            600 // 10 minutes for CPU work
        };

        OptimizationSettings {
            memory_limit_percentage,
            max_temperature_celsius,
            checkpoint_interval_seconds,
            thread_affinity: None, // Could be implemented based on CPU topology
        }
    }
}

impl Default for HardwareDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to detect hardware profile
pub async fn detect_hardware_capabilities() -> Result<HardwareProfile> {
    let mut detector = HardwareDetector::new();
    detector.detect_hardware()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_type_determination() {
        let detector = HardwareDetector::new();

        // Test CPU only
        let cpu = CpuProfile {
            performance_score: 50.0,
            ..Default::default()
        };
        let gpus = vec![];
        let hw_type = detector.determine_hardware_type(&cpu, &gpus);
        assert_eq!(hw_type, HardwareType::CpuOnly);

        // Test GPU only
        let cpu = CpuProfile {
            performance_score: 20.0,
            ..Default::default()
        };
        let gpu = GpuProfile {
            performance_score: 80.0,
            ..Default::default()
        };
        let gpus = vec![gpu];
        let hw_type = detector.determine_hardware_type(&cpu, &gpus);
        assert_eq!(hw_type, HardwareType::GpuOnly);

        // Test both
        let cpu = CpuProfile {
            performance_score: 60.0,
            ..Default::default()
        };
        let gpu = GpuProfile {
            performance_score: 70.0,
            ..Default::default()
        };
        let gpus = vec![gpu];
        let hw_type = detector.determine_hardware_type(&cpu, &gpus);
        assert_eq!(hw_type, HardwareType::Both);
    }

    #[test]
    fn test_nvidia_spec_estimation() {
        let detector = HardwareDetector::new();

        let (cores, bandwidth) = detector.estimate_nvidia_specs("NVIDIA GeForce RTX 3080");
        assert_eq!(cores, 8704);
        assert_eq!(bandwidth, 760.0);

        let (cores, bandwidth) = detector.estimate_nvidia_specs("NVIDIA GeForce GTX 1060");
        assert_eq!(cores, 1280);
        assert_eq!(bandwidth, 192.0);
    }

    #[test]
    fn test_amd_spec_estimation() {
        let detector = HardwareDetector::new();

        let (memory, cores, bandwidth) = detector.estimate_amd_specs("AMD Radeon RX 6800");
        assert_eq!(memory, 16384);
        assert_eq!(cores, 3840);
        assert_eq!(bandwidth, 512.0);
    }

    #[test]
    fn test_intel_spec_estimation() {
        let detector = HardwareDetector::new();

        let (memory, cores, bandwidth) = detector.estimate_intel_specs("Intel Arc A770");
        assert_eq!(memory, 16384);
        assert_eq!(cores, 4096);
        assert_eq!(bandwidth, 560.0);
    }

    #[test]
    fn test_gpu_performance_score() {
        let detector = HardwareDetector::new();

        // Test NVIDIA GPU with compute capability
        let score = detector.calculate_gpu_performance_score(
            10 * 1024 * 1024 * 1024, // 10GB
            3072,                    // 3072 cores
            448.0,                   // 448 GB/s bandwidth
            Some((7, 5)),            // Compute capability 7.5
        );
        assert!(score > 50.0);
        assert!(score <= 100.0);

        // Test AMD GPU without compute capability
        let score = detector.calculate_gpu_performance_score(
            8 * 1024 * 1024 * 1024, // 8GB
            2048,                   // 2048 cores
            256.0,                  // 256 GB/s bandwidth
            None,                   // No compute capability
        );
        assert!(score > 30.0);
        assert!(score <= 100.0);
    }
}
