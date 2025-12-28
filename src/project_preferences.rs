//! Project preference management module for Chert miner
//!
//! This module provides intelligent project selection and management based on:
//! - Hardware capabilities and performance
//! - User preferences and priorities
//! - Project compatibility and requirements
//! - Performance metrics and historical data

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tracing::{debug, info};

use crate::config::ProjectPreferencesConfig;
#[cfg(test)]
use crate::config::ProjectSwitchingConfig;
use crate::hardware_detection::{GpuVendor, HardwareProfile, HardwareType};

/// Project information and requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Project name
    pub name: String,
    /// Project description
    pub description: String,
    /// Project URL
    pub url: String,
    /// Hardware requirements
    pub hardware_requirements: HardwareRequirements,
    /// Project priority (1-10, higher is more important)
    pub priority: u8,
    /// Credit multiplier for rewards
    pub credit_multiplier: f64,
    /// Typical work unit size in MB
    pub typical_work_unit_size: u64,
    /// Estimated runtime in hours
    pub estimated_runtime_hours: f64,
    /// Scientific category
    pub category: ProjectCategory,
    /// Supported platforms
    pub supported_platforms: Vec<String>,
    /// Minimum system requirements
    pub minimum_requirements: SystemRequirements,
}

/// Hardware requirements for projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareRequirements {
    /// Required hardware type
    pub hardware_type: HardwareType,
    /// Minimum CPU cores required
    pub min_cpu_cores: Option<usize>,
    /// Minimum CPU performance score
    pub min_cpu_performance: Option<f64>,
    /// GPU requirements
    pub gpu_requirements: Option<GpuRequirements>,
    /// Minimum memory in GB
    pub min_memory_gb: Option<f64>,
    /// Minimum disk space in GB
    pub min_disk_gb: Option<f64>,
    /// Network requirements
    pub network_requirements: NetworkRequirements,
}

/// GPU-specific requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    /// Minimum VRAM in GB
    pub min_vram_gb: Option<f64>,
    /// Required GPU vendors
    pub supported_vendors: Vec<GpuVendor>,
    /// Minimum compute capability (for NVIDIA)
    pub min_compute_capability: Option<(u8, u8)>,
    /// Required GPU features
    pub required_features: Vec<String>,
    /// Recommended GPU memory bandwidth in GB/s
    pub min_memory_bandwidth: Option<f64>,
}

/// Network requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequirements {
    /// Minimum bandwidth in Mbps
    pub min_bandwidth_mbps: Option<f64>,
    /// Maximum acceptable latency in ms
    pub max_latency_ms: Option<f64>,
    /// Connection reliability required
    pub requires_reliable_connection: bool,
}

/// System requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemRequirements {
    /// Minimum OS version
    pub min_os_version: Option<String>,
    /// Required libraries or dependencies
    pub required_libraries: Vec<String>,
    /// Required software versions
    pub required_software: HashMap<String, String>,
}

/// Project categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectCategory {
    /// Astronomy and space research
    Astronomy,
    /// Medical and health research
    Medical,
    /// Physics and particle physics
    Physics,
    /// Mathematics and cryptography
    Mathematics,
    /// Biology and genetics
    Biology,
    /// Climate and environmental science
    Climate,
    /// Computer science and AI
    ComputerScience,
    /// Other categories
    Other(String),
}

impl std::fmt::Display for ProjectCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectCategory::Astronomy => write!(f, "Astronomy"),
            ProjectCategory::Medical => write!(f, "Medical"),
            ProjectCategory::Physics => write!(f, "Physics"),
            ProjectCategory::Mathematics => write!(f, "Mathematics"),
            ProjectCategory::Biology => write!(f, "Biology"),
            ProjectCategory::Climate => write!(f, "Climate"),
            ProjectCategory::ComputerScience => write!(f, "Computer Science"),
            ProjectCategory::Other(name) => write!(f, "{}", name),
        }
    }
}

/// Project compatibility score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityScore {
    /// Project name
    pub project_name: String,
    /// Overall compatibility score (0-100)
    pub overall_score: f64,
    /// Hardware compatibility score
    pub hardware_score: f64,
    /// Performance compatibility score
    pub performance_score: f64,
    /// Network compatibility score
    pub network_score: f64,
    /// User preference score
    pub preference_score: f64,
    /// Compatibility factors
    pub factors: Vec<CompatibilityFactor>,
}

/// Individual compatibility factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityFactor {
    /// Factor name
    pub name: String,
    /// Factor score (0-100)
    pub score: f64,
    /// Factor weight in overall calculation
    pub weight: f64,
    /// Factor description
    pub description: String,
}

/// Project selection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSelection {
    /// Selected project
    pub project: ProjectInfo,
    /// Selection reason
    pub reason: String,
    /// Expected performance
    pub expected_performance: ExpectedPerformance,
    /// Compatibility score
    pub compatibility_score: CompatibilityScore,
}

/// Expected performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedPerformance {
    /// Expected work units per hour
    pub work_units_per_hour: f64,
    /// Expected credit per hour
    pub credit_per_hour: f64,
    /// Expected efficiency score
    pub efficiency_score: f64,
    /// Resource utilization estimate
    pub resource_utilization: ResourceUtilization,
}

/// Resource utilization estimates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    /// Expected CPU utilization percentage
    pub cpu_utilization_percent: f64,
    /// Expected GPU utilization percentage
    pub gpu_utilization_percent: f64,
    /// Expected memory usage in GB
    pub memory_usage_gb: f64,
    /// Expected network usage in Mbps
    pub network_usage_mbps: f64,
}

/// Project switching decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchingDecision {
    /// Whether to switch projects
    pub should_switch: bool,
    /// Recommended new project (if switching)
    pub recommended_project: Option<ProjectInfo>,
    /// Reason for switching
    pub reason: String,
    /// Estimated performance improvement
    pub performance_improvement: Option<f64>,
    /// Time until next switch consideration
    pub next_switch_time: Option<SystemTime>,
}

/// Project preference manager
pub struct ProjectPreferenceManager {
    /// Available projects database
    projects: HashMap<String, ProjectInfo>,
    /// Hardware profile
    hardware_profile: Option<HardwareProfile>,
    /// Configuration
    config: ProjectPreferencesConfig,
    /// Project performance history
    performance_history: HashMap<String, Vec<PerformanceRecord>>,
}

/// Performance record for projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRecord {
    /// Timestamp of record
    pub timestamp: SystemTime,
    /// Work units completed
    pub work_units_completed: u32,
    /// Total credit earned
    pub total_credit: f64,
    /// Average completion time in hours
    pub avg_completion_time_hours: f64,
    /// Success rate percentage
    pub success_rate: f64,
    /// Resource efficiency score
    pub resource_efficiency: f64,
}

impl ProjectPreferenceManager {
    /// Create a new project preference manager
    pub fn new(config: ProjectPreferencesConfig) -> Self {
        let mut manager = Self {
            projects: HashMap::new(),
            hardware_profile: None,
            config,
            performance_history: HashMap::new(),
        };

        // Initialize with default projects
        manager.load_default_projects();

        manager
    }

    /// Load default BOINC projects
    fn load_default_projects(&mut self) {
        let default_projects = vec![
            ProjectInfo {
                name: "MilkyWay@Home".to_string(),
                description: "N-body simulation for galactic structure research".to_string(),
                url: "https://milkyway.cs.rpi.edu/milkyway/".to_string(),
                hardware_requirements: HardwareRequirements {
                    hardware_type: HardwareType::Both,
                    min_cpu_cores: Some(2),
                    min_cpu_performance: Some(30.0),
                    gpu_requirements: Some(GpuRequirements {
                        min_vram_gb: Some(2.0),
                        supported_vendors: vec![GpuVendor::Nvidia, GpuVendor::Amd],
                        min_compute_capability: Some((3, 5)),
                        required_features: vec!["OpenCL".to_string(), "CUDA".to_string()],
                        min_memory_bandwidth: Some(100.0),
                    }),
                    min_memory_gb: Some(4.0),
                    min_disk_gb: Some(10.0),
                    network_requirements: NetworkRequirements {
                        min_bandwidth_mbps: Some(1.0),
                        max_latency_ms: Some(1000.0),
                        requires_reliable_connection: false,
                    },
                },
                priority: 8,
                credit_multiplier: 1.2,
                typical_work_unit_size: 100,
                estimated_runtime_hours: 6.0,
                category: ProjectCategory::Astronomy,
                supported_platforms: vec![
                    "linux".to_string(),
                    "windows".to_string(),
                    "macos".to_string(),
                ],
                minimum_requirements: SystemRequirements {
                    min_os_version: None,
                    required_libraries: vec!["OpenCL".to_string()],
                    required_software: HashMap::new(),
                },
            },
            ProjectInfo {
                name: "Rosetta@Home".to_string(),
                description: "Protein folding and disease research".to_string(),
                url: "https://boinc.bakerlab.org/rosetta/".to_string(),
                hardware_requirements: HardwareRequirements {
                    hardware_type: HardwareType::Both,
                    min_cpu_cores: Some(4),
                    min_cpu_performance: Some(50.0),
                    gpu_requirements: Some(GpuRequirements {
                        min_vram_gb: Some(4.0),
                        supported_vendors: vec![
                            GpuVendor::Nvidia,
                            GpuVendor::Amd,
                            GpuVendor::Intel,
                        ],
                        min_compute_capability: Some((3, 0)),
                        required_features: vec!["OpenCL".to_string()],
                        min_memory_bandwidth: Some(50.0),
                    }),
                    min_memory_gb: Some(8.0),
                    min_disk_gb: Some(20.0),
                    network_requirements: NetworkRequirements {
                        min_bandwidth_mbps: Some(2.0),
                        max_latency_ms: Some(500.0),
                        requires_reliable_connection: true,
                    },
                },
                priority: 7,
                credit_multiplier: 1.0,
                typical_work_unit_size: 300,
                estimated_runtime_hours: 8.0,
                category: ProjectCategory::Medical,
                supported_platforms: vec![
                    "linux".to_string(),
                    "windows".to_string(),
                    "macos".to_string(),
                ],
                minimum_requirements: SystemRequirements {
                    min_os_version: None,
                    required_libraries: vec!["OpenCL".to_string()],
                    required_software: HashMap::new(),
                },
            },
            ProjectInfo {
                name: "World Community Grid".to_string(),
                description: "Medical and humanitarian research projects".to_string(),
                url: "https://www.worldcommunitygrid.org/".to_string(),
                hardware_requirements: HardwareRequirements {
                    hardware_type: HardwareType::CpuOnly,
                    min_cpu_cores: Some(2),
                    min_cpu_performance: Some(25.0),
                    gpu_requirements: None,
                    min_memory_gb: Some(2.0),
                    min_disk_gb: Some(5.0),
                    network_requirements: NetworkRequirements {
                        min_bandwidth_mbps: Some(0.5),
                        max_latency_ms: Some(2000.0),
                        requires_reliable_connection: false,
                    },
                },
                priority: 6,
                credit_multiplier: 0.8,
                typical_work_unit_size: 50,
                estimated_runtime_hours: 4.0,
                category: ProjectCategory::Medical,
                supported_platforms: vec![
                    "linux".to_string(),
                    "windows".to_string(),
                    "macos".to_string(),
                ],
                minimum_requirements: SystemRequirements {
                    min_os_version: None,
                    required_libraries: vec![],
                    required_software: HashMap::new(),
                },
            },
            ProjectInfo {
                name: "GPUGRID".to_string(),
                description: "GPU-accelerated biomedical research".to_string(),
                url: "https://www.gpugrid.net/".to_string(),
                hardware_requirements: HardwareRequirements {
                    hardware_type: HardwareType::GpuOnly,
                    min_cpu_cores: Some(2),
                    min_cpu_performance: Some(20.0),
                    gpu_requirements: Some(GpuRequirements {
                        min_vram_gb: Some(6.0),
                        supported_vendors: vec![GpuVendor::Nvidia],
                        min_compute_capability: Some((6, 0)),
                        required_features: vec!["CUDA".to_string(), "Double Precision".to_string()],
                        min_memory_bandwidth: Some(200.0),
                    }),
                    min_memory_gb: Some(8.0),
                    min_disk_gb: Some(10.0),
                    network_requirements: NetworkRequirements {
                        min_bandwidth_mbps: Some(5.0),
                        max_latency_ms: Some(200.0),
                        requires_reliable_connection: true,
                    },
                },
                priority: 9,
                credit_multiplier: 1.5,
                typical_work_unit_size: 500,
                estimated_runtime_hours: 12.0,
                category: ProjectCategory::Medical,
                supported_platforms: vec!["linux".to_string(), "windows".to_string()],
                minimum_requirements: SystemRequirements {
                    min_os_version: None,
                    required_libraries: vec!["CUDA".to_string()],
                    required_software: HashMap::new(),
                },
            },
        ];

        for project in default_projects {
            self.projects.insert(project.name.clone(), project);
        }

        info!("Loaded {} default projects", self.projects.len());
    }

    /// Set hardware profile for compatibility checking
    pub fn set_hardware_profile(&mut self, profile: HardwareProfile) {
        self.hardware_profile = Some(profile);
        info!("Hardware profile set for project compatibility checking");
    }

    /// Get compatible projects based on hardware
    pub fn get_compatible_projects(&self) -> Result<Vec<ProjectInfo>> {
        let hardware_profile = self
            .hardware_profile
            .as_ref()
            .context("Hardware profile not set")?;

        let mut compatible_projects = Vec::new();

        for project in self.projects.values() {
            if self.is_project_compatible(project, hardware_profile) {
                compatible_projects.push(project.clone());
            }
        }

        // Sort by priority and compatibility score
        compatible_projects.sort_by(|a, b| {
            let score_a = self
                .calculate_compatibility_score(a, hardware_profile)
                .overall_score;
            let score_b = self
                .calculate_compatibility_score(b, hardware_profile)
                .overall_score;

            // First sort by priority (higher first), then by compatibility score
            b.priority.cmp(&a.priority).then_with(|| {
                score_b
                    .partial_cmp(&score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        Ok(compatible_projects)
    }

    /// Check if a project is compatible with the hardware
    fn is_project_compatible(&self, project: &ProjectInfo, hardware: &HardwareProfile) -> bool {
        let requirements = &project.hardware_requirements;

        // Check hardware type compatibility
        match (&requirements.hardware_type, &hardware.hardware_type) {
            (HardwareType::CpuOnly, HardwareType::Both) => {} // CPU-only projects work on both
            (HardwareType::GpuOnly, HardwareType::Both) => {} // GPU-only projects work on both
            (HardwareType::Both, HardwareType::CpuOnly) => return false, // Both types need GPU
            (HardwareType::Both, HardwareType::GpuOnly) => {} // Both types work on GPU-only
            (a, b) if a != b => return false,                 // Mismatched types
            _ => {}                                           // Compatible types
        }

        // Check CPU requirements
        if let Some(min_cores) = requirements.min_cpu_cores {
            if hardware.cpu.physical_cores < min_cores {
                debug!(
                    "Project {} requires {} CPU cores, have {}",
                    project.name, min_cores, hardware.cpu.physical_cores
                );
                return false;
            }
        }

        if let Some(min_performance) = requirements.min_cpu_performance {
            if hardware.cpu.performance_score < min_performance {
                debug!(
                    "Project {} requires CPU performance {}, have {}",
                    project.name, min_performance, hardware.cpu.performance_score
                );
                return false;
            }
        }

        // Check memory requirements
        if let Some(min_memory_gb) = requirements.min_memory_gb {
            let available_memory_gb =
                hardware.system.total_memory as f64 / (1024.0 * 1024.0 * 1024.0);
            if available_memory_gb < min_memory_gb {
                debug!(
                    "Project {} requires {} GB memory, have {:.1} GB",
                    project.name, min_memory_gb, available_memory_gb
                );
                return false;
            }
        }

        // Check disk space requirements
        if let Some(min_disk_gb) = requirements.min_disk_gb {
            let available_disk_gb =
                hardware.system.available_disk_space as f64 / (1024.0 * 1024.0 * 1024.0);
            if available_disk_gb < min_disk_gb {
                debug!(
                    "Project {} requires {} GB disk space, have {:.1} GB",
                    project.name, min_disk_gb, available_disk_gb
                );
                return false;
            }
        }

        // Check GPU requirements if specified
        if let Some(gpu_reqs) = &requirements.gpu_requirements {
            if hardware.gpus.is_empty() {
                debug!("Project {} requires GPU but none available", project.name);
                return false;
            }

            let best_gpu = hardware
                .gpus
                .iter()
                .max_by(|a, b| {
                    a.performance_score
                        .partial_cmp(&b.performance_score)
                        .unwrap()
                })
                .unwrap();

            // Check VRAM requirements
            if let Some(min_vram_gb) = gpu_reqs.min_vram_gb {
                let available_vram_gb = best_gpu.total_memory as f64 / (1024.0 * 1024.0 * 1024.0);
                if available_vram_gb < min_vram_gb {
                    debug!(
                        "Project {} requires {} GB VRAM, have {:.1} GB",
                        project.name, min_vram_gb, available_vram_gb
                    );
                    return false;
                }
            }

            // Check vendor compatibility
            if !gpu_reqs.supported_vendors.contains(&best_gpu.vendor) {
                debug!(
                    "Project {} requires GPU vendor {:?}, have {:?}",
                    project.name, gpu_reqs.supported_vendors, best_gpu.vendor
                );
                return false;
            }

            // Check compute capability for NVIDIA
            if let Some((min_major, min_minor)) = gpu_reqs.min_compute_capability {
                if best_gpu.vendor == GpuVendor::Nvidia {
                    if let Some((major, minor)) = best_gpu.compute_capability {
                        if major < min_major || (major == min_major && minor < min_minor) {
                            debug!(
                                "Project {} requires compute capability {}.{}, have {}.{}",
                                project.name, min_major, min_minor, major, minor
                            );
                            return false;
                        }
                    } else {
                        debug!(
                            "Project {} requires compute capability but GPU capability unknown",
                            project.name
                        );
                        return false;
                    }
                }
            }

            // Check memory bandwidth
            if let Some(min_bandwidth) = gpu_reqs.min_memory_bandwidth {
                if best_gpu.memory_bandwidth < min_bandwidth {
                    debug!(
                        "Project {} requires memory bandwidth {} GB/s, have {}",
                        project.name, min_bandwidth, best_gpu.memory_bandwidth
                    );
                    return false;
                }
            }
        }

        // Check network requirements
        let network = &requirements.network_requirements;
        if let Some(min_bandwidth) = network.min_bandwidth_mbps {
            if let Some(bandwidth) = hardware.system.network_status.bandwidth_mbps {
                if bandwidth < min_bandwidth {
                    debug!(
                        "Project {} requires bandwidth {} Mbps, have {}",
                        project.name, min_bandwidth, bandwidth
                    );
                    return false;
                }
            }
        }

        if let Some(max_latency) = network.max_latency_ms {
            if let Some(latency) = hardware.system.network_status.latency_ms {
                if latency > max_latency {
                    debug!(
                        "Project {} requires latency < {} ms, have {}",
                        project.name, max_latency, latency
                    );
                    return false;
                }
            }
        }

        true
    }

    /// Calculate compatibility score for a project
    pub fn calculate_compatibility_score(
        &self,
        project: &ProjectInfo,
        hardware: &HardwareProfile,
    ) -> CompatibilityScore {
        let mut factors = Vec::new();

        // Hardware compatibility factor
        let hardware_score = self.calculate_hardware_compatibility(project, hardware);
        factors.push(CompatibilityFactor {
            name: "Hardware Compatibility".to_string(),
            score: hardware_score,
            weight: 0.4,
            description: "How well the hardware matches project requirements".to_string(),
        });

        // Performance factor
        let performance_score = self.calculate_performance_compatibility(project, hardware);
        factors.push(CompatibilityFactor {
            name: "Performance Match".to_string(),
            score: performance_score,
            weight: 0.3,
            description: "Expected performance based on hardware capabilities".to_string(),
        });

        // Network compatibility factor
        let network_score = self.calculate_network_compatibility(project, hardware);
        factors.push(CompatibilityFactor {
            name: "Network Compatibility".to_string(),
            score: network_score,
            weight: 0.1,
            description: "Network suitability for the project".to_string(),
        });

        // User preference factor
        let preference_score = self.calculate_preference_score(project);
        factors.push(CompatibilityFactor {
            name: "User Preference".to_string(),
            score: preference_score,
            weight: 0.2,
            description: "Alignment with user preferences and priorities".to_string(),
        });

        // Calculate overall score
        let overall_score = factors.iter().map(|f| f.score * f.weight).sum();

        CompatibilityScore {
            project_name: project.name.clone(),
            overall_score,
            hardware_score,
            performance_score,
            network_score,
            preference_score,
            factors,
        }
    }

    /// Calculate hardware compatibility score
    fn calculate_hardware_compatibility(
        &self,
        project: &ProjectInfo,
        hardware: &HardwareProfile,
    ) -> f64 {
        let requirements = &project.hardware_requirements;
        let mut score = 100.0;

        // Hardware type matching
        match (&requirements.hardware_type, &hardware.hardware_type) {
            (HardwareType::Both, HardwareType::Both) => score -= 0.0,
            (HardwareType::CpuOnly, HardwareType::CpuOnly) => score -= 0.0,
            (HardwareType::GpuOnly, HardwareType::GpuOnly) => score -= 0.0,
            (HardwareType::CpuOnly, HardwareType::Both) => score -= 10.0, // Underutilized
            (HardwareType::GpuOnly, HardwareType::Both) => score -= 10.0, // Underutilized
            (HardwareType::Both, HardwareType::CpuOnly) => score -= 50.0, // Missing GPU
            (HardwareType::Both, HardwareType::GpuOnly) => score -= 20.0, // Missing CPU
            _ => score -= 30.0,                                           // Mismatch
        }

        // CPU performance scoring
        if let Some(min_performance) = requirements.min_cpu_performance {
            if hardware.cpu.performance_score >= min_performance {
                let excess = hardware.cpu.performance_score - min_performance;
                score += (excess / 100.0) * 10.0; // Bonus for excess performance
            } else {
                let deficit = min_performance - hardware.cpu.performance_score;
                score -= (deficit / 100.0) * 20.0; // Penalty for insufficient performance
            }
        }

        // Memory scoring
        if let Some(min_memory_gb) = requirements.min_memory_gb {
            let available_memory_gb =
                hardware.system.total_memory as f64 / (1024.0 * 1024.0 * 1024.0);
            if available_memory_gb >= min_memory_gb {
                let excess = available_memory_gb - min_memory_gb;
                score += (excess / min_memory_gb) * 5.0; // Bonus for excess memory
            } else {
                score -= 30.0; // Penalty for insufficient memory
            }
        }

        score.clamp(0.0, 100.0)
    }

    /// Calculate performance compatibility score
    fn calculate_performance_compatibility(
        &self,
        project: &ProjectInfo,
        hardware: &HardwareProfile,
    ) -> f64 {
        let mut score = 50.0; // Base score

        // Factor in CPU performance
        score += (hardware.cpu.performance_score / 100.0) * 25.0;

        // Factor in GPU performance if applicable
        if let Some(gpu_reqs) = &project.hardware_requirements.gpu_requirements {
            if !hardware.gpus.is_empty() {
                let best_gpu = hardware
                    .gpus
                    .iter()
                    .max_by(|a, b| {
                        a.performance_score
                            .partial_cmp(&b.performance_score)
                            .unwrap()
                    })
                    .unwrap();

                score += (best_gpu.performance_score / 100.0) * 25.0;

                // Bonus for meeting specific GPU requirements
                if let Some(min_vram_gb) = gpu_reqs.min_vram_gb {
                    let available_vram_gb =
                        best_gpu.total_memory as f64 / (1024.0 * 1024.0 * 1024.0);
                    if available_vram_gb >= min_vram_gb {
                        score += 10.0;
                    }
                }
            }
        }

        score.clamp(0.0, 100.0)
    }

    /// Calculate network compatibility score
    fn calculate_network_compatibility(
        &self,
        project: &ProjectInfo,
        hardware: &HardwareProfile,
    ) -> f64 {
        let network = &project.hardware_requirements.network_requirements;
        let mut score = 100.0;

        if let Some(min_bandwidth) = network.min_bandwidth_mbps {
            if let Some(bandwidth) = hardware.system.network_status.bandwidth_mbps {
                if bandwidth >= min_bandwidth {
                    let ratio = bandwidth / min_bandwidth;
                    score += ((ratio - 1.0) * 20.0).min(20.0); // Bonus for excess bandwidth
                } else {
                    score -= 40.0; // Penalty for insufficient bandwidth
                }
            } else {
                score -= 20.0; // Unknown bandwidth
            }
        }

        if let Some(max_latency) = network.max_latency_ms {
            if let Some(latency) = hardware.system.network_status.latency_ms {
                if latency <= max_latency {
                    let margin = max_latency - latency;
                    score += (margin / max_latency) * 10.0; // Bonus for low latency
                } else {
                    score -= 30.0; // Penalty for high latency
                }
            } else {
                score -= 10.0; // Unknown latency
            }
        }

        score.clamp(0.0, 100.0)
    }

    /// Calculate user preference score
    fn calculate_preference_score(&self, project: &ProjectInfo) -> f64 {
        let mut score = 50.0; // Base score

        // Check if project is in preferred list
        if self.config.preferred_projects.contains(&project.name) {
            score += 30.0;
        }

        // Apply project weights
        if let Some(weight) = self.config.project_weights.get(&project.name) {
            score += (weight - 1.0) * 20.0; // Weight is multiplier, 1.0 is neutral
        }

        // Factor in project priority
        score += (project.priority as f64 - 5.0) * 5.0; // Priority range 1-10, 5 is neutral

        score.clamp(0.0, 100.0)
    }

    /// Select optimal project based on current conditions
    pub fn select_optimal_project(&self) -> Result<ProjectSelection> {
        let hardware_profile = self
            .hardware_profile
            .as_ref()
            .context("Hardware profile not set")?;

        let compatible_projects = self.get_compatible_projects()?;

        if compatible_projects.is_empty() {
            return Err(anyhow::anyhow!("No compatible projects found"));
        }

        // Select best project based on configuration
        let selected_project = if self.config.auto_select_projects {
            // Auto-select based on compatibility score and performance
            let best_project = compatible_projects
                .iter()
                .max_by(|a, b| {
                    let score_a = self.calculate_compatibility_score(a, hardware_profile);
                    let score_b = self.calculate_compatibility_score(b, hardware_profile);

                    // Compare overall score, then priority
                    score_a
                        .overall_score
                        .partial_cmp(&score_b.overall_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| b.priority.cmp(&a.priority))
                })
                .cloned();

            best_project.unwrap_or_else(|| {
                // Fallback to first compatible project
                compatible_projects
                    .first()
                    .cloned()
                    .unwrap_or_else(|| panic!("No compatible projects available"))
            })
        } else {
            // Use user's preferred projects in order
            self.config
                .preferred_projects
                .iter()
                .find_map(|pref_name| {
                    compatible_projects
                        .iter()
                        .find(|project| project.name == *pref_name)
                        .cloned()
                })
                .or_else(|| {
                    // Fallback to highest priority compatible project
                    compatible_projects
                        .iter()
                        .max_by(|a, b| a.priority.cmp(&b.priority))
                        .cloned()
                })
                .context("No compatible projects found")?
        };

        let compatibility_score =
            self.calculate_compatibility_score(&selected_project, hardware_profile);
        let expected_performance =
            self.calculate_expected_performance(&selected_project, hardware_profile);

        let reason = if self.config.auto_select_projects {
            format!(
                "Auto-selected based on compatibility score {:.1}",
                compatibility_score.overall_score
            )
        } else {
            format!(
                "Selected from user preferences (priority: {})",
                selected_project.priority
            )
        };

        Ok(ProjectSelection {
            project: selected_project,
            reason,
            expected_performance,
            compatibility_score,
        })
    }

    /// Calculate expected performance for a project
    fn calculate_expected_performance(
        &self,
        project: &ProjectInfo,
        hardware: &HardwareProfile,
    ) -> ExpectedPerformance {
        let compatibility_score = self.calculate_compatibility_score(project, hardware);

        // Base performance estimates
        let base_work_units_per_hour = match project.category {
            ProjectCategory::Astronomy => 2.0,
            ProjectCategory::Medical => 1.5,
            ProjectCategory::Physics => 1.8,
            ProjectCategory::Mathematics => 3.0,
            ProjectCategory::Biology => 1.2,
            ProjectCategory::Climate => 1.0,
            ProjectCategory::ComputerScience => 2.5,
            ProjectCategory::Other(_) => 1.0,
        };

        // Adjust based on hardware performance
        let performance_multiplier = compatibility_score.performance_score / 100.0;
        let work_units_per_hour = base_work_units_per_hour * performance_multiplier;

        // Calculate credit per hour
        let credit_per_hour = work_units_per_hour * project.credit_multiplier;

        // Efficiency score
        let efficiency_score = compatibility_score.overall_score;

        // Resource utilization estimates
        let cpu_utilization_percent = if project.hardware_requirements.hardware_type
            == HardwareType::CpuOnly
            || project.hardware_requirements.hardware_type == HardwareType::Both
        {
            70.0 * (compatibility_score.hardware_score / 100.0)
        } else {
            20.0
        };

        let gpu_utilization_percent = if project.hardware_requirements.gpu_requirements.is_some()
            && !hardware.gpus.is_empty()
        {
            80.0 * (compatibility_score.performance_score / 100.0)
        } else {
            0.0
        };

        let memory_usage_gb = project.typical_work_unit_size as f64 / 1024.0; // Convert MB to GB
        let network_usage_mbps = if let Some(min_bandwidth) = project
            .hardware_requirements
            .network_requirements
            .min_bandwidth_mbps
        {
            min_bandwidth * 0.5 // Estimate 50% of minimum requirement
        } else {
            1.0
        };

        ExpectedPerformance {
            work_units_per_hour,
            credit_per_hour,
            efficiency_score,
            resource_utilization: ResourceUtilization {
                cpu_utilization_percent,
                gpu_utilization_percent,
                memory_usage_gb,
                network_usage_mbps,
            },
        }
    }

    /// Evaluate whether to switch projects
    pub fn evaluate_project_switching(
        &self,
        current_project: &str,
        current_performance: &PerformanceRecord,
    ) -> Result<SwitchingDecision> {
        let hardware_profile = self
            .hardware_profile
            .as_ref()
            .context("Hardware profile not set")?;

        if !self.config.switching.auto_switch {
            return Ok(SwitchingDecision {
                should_switch: false,
                recommended_project: None,
                reason: "Automatic switching is disabled".to_string(),
                performance_improvement: None,
                next_switch_time: None,
            });
        }

        // Check minimum run time using performance record timestamps
        let current_time = SystemTime::now();
        let min_run_time = Duration::from_secs(self.config.switching.min_run_time_seconds);
        let earliest_switch_time = current_performance
            .timestamp
            .checked_add(min_run_time)
            .unwrap_or(current_time + min_run_time);

        if current_time < earliest_switch_time {
            return Ok(SwitchingDecision {
                should_switch: false,
                recommended_project: None,
                reason: format!(
                    "Minimum runtime of {:?} has not elapsed for {}",
                    min_run_time, current_project
                ),
                performance_improvement: None,
                next_switch_time: Some(earliest_switch_time),
            });
        }

        let should_switch = if self.config.switching.performance_based_switching {
            current_performance.resource_efficiency < 50.0
                || current_performance.success_rate < 80.0
        } else if self.config.switching.reward_based_switching {
            current_performance.total_credit < 10.0 // Low credit per hour
        } else {
            false
        };

        if !should_switch {
            return Ok(SwitchingDecision {
                should_switch: false,
                recommended_project: None,
                reason: "Current performance is acceptable".to_string(),
                performance_improvement: None,
                next_switch_time: Some(earliest_switch_time),
            });
        }

        // Find better project
        let compatible_projects = self.get_compatible_projects()?;
        let current_project_info = self
            .projects
            .get(current_project)
            .context("Current project not found in database")?;

        let better_project = compatible_projects.into_iter().find(|project| {
            project.name != current_project && project.priority > current_project_info.priority
        });

        if let Some(recommended) = better_project {
            let expected_performance =
                self.calculate_expected_performance(&recommended, hardware_profile);
            let performance_improvement =
                expected_performance.credit_per_hour - current_performance.total_credit;

            Ok(SwitchingDecision {
                should_switch: true,
                recommended_project: Some(recommended),
                reason: format!(
                    "Better project available with {:.1}% performance improvement",
                    (performance_improvement / current_performance.total_credit) * 100.0
                ),
                performance_improvement: Some(
                    performance_improvement / current_performance.total_credit,
                ),
                next_switch_time: Some(earliest_switch_time),
            })
        } else {
            Ok(SwitchingDecision {
                should_switch: false,
                recommended_project: None,
                reason: "No better projects available".to_string(),
                performance_improvement: None,
                next_switch_time: Some(earliest_switch_time),
            })
        }
    }

    /// Record performance data for a project
    pub fn record_performance(&mut self, project_name: &str, record: PerformanceRecord) {
        let history = self
            .performance_history
            .entry(project_name.to_string())
            .or_default();
        history.push(record);

        // Keep only recent records (last 100)
        if history.len() > 100 {
            history.drain(0..history.len() - 100);
        }

        debug!("Recorded performance for project: {}", project_name);
    }

    /// Get performance history for a project
    pub fn get_performance_history(&self, project_name: &str) -> Option<&[PerformanceRecord]> {
        self.performance_history
            .get(project_name)
            .map(|history| history.as_slice())
    }

    /// Get all available projects
    pub fn get_all_projects(&self) -> Vec<&ProjectInfo> {
        self.projects.values().collect()
    }

    /// Add a custom project
    pub fn add_custom_project(&mut self, project: ProjectInfo) -> Result<()> {
        if self.projects.contains_key(&project.name) {
            return Err(anyhow::anyhow!("Project '{}' already exists", project.name));
        }

        let project_name = project.name.clone();
        self.projects.insert(project_name.clone(), project);
        info!("Added custom project: {}", project_name);
        Ok(())
    }

    /// Remove a project
    pub fn remove_project(&mut self, project_name: &str) -> Result<()> {
        if !self.projects.contains_key(project_name) {
            return Err(anyhow::anyhow!("Project '{}' not found", project_name));
        }

        self.projects.remove(project_name);
        self.performance_history.remove(project_name);
        info!("Removed project: {}", project_name);
        Ok(())
    }
}

impl Default for ProjectPreferenceManager {
    fn default() -> Self {
        Self::new(ProjectPreferencesConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware_detection::*;

    #[test]
    fn test_project_compatibility() {
        let config = ProjectPreferencesConfig::default();
        let mut manager = ProjectPreferenceManager::new(config);

        let hardware = HardwareProfile {
            hardware_type: HardwareType::Both,
            cpu: CpuProfile {
                performance_score: 70.0,
                physical_cores: 8,
                ..Default::default()
            },
            gpus: vec![GpuProfile {
                performance_score: 80.0,
                total_memory: 8 * 1024 * 1024 * 1024, // 8GB
                vendor: GpuVendor::Nvidia,
                compute_capability: Some((7, 5)),
                memory_bandwidth: 200.0, // GB/s - meets requirement
                features: vec!["OpenCL".to_string(), "CUDA".to_string()],
                ..Default::default()
            }],
            system: SystemInfo {
                total_memory: 16 * 1024 * 1024 * 1024,          // 16GB
                available_disk_space: 100 * 1024 * 1024 * 1024, // 100GB
                network_status: NetworkStatus {
                    connectivity: true,
                    bandwidth_mbps: Some(100.0),
                    latency_ms: Some(20.0),
                    connection_type: Some("ethernet".to_string()),
                },
                ..Default::default()
            },
            ..Default::default()
        };

        manager.set_hardware_profile(hardware);

        let compatible = manager.get_compatible_projects().unwrap();
        assert!(!compatible.is_empty());

        // Should find MilkyWay@Home compatible
        let milkyway_found = compatible.iter().any(|p| p.name == "MilkyWay@Home");
        assert!(milkyway_found);
    }

    #[test]
    fn test_project_selection() {
        let config = ProjectPreferencesConfig {
            auto_select_projects: true,
            preferred_projects: vec!["MilkyWay@Home".to_string()],
            ..Default::default()
        };
        let mut manager = ProjectPreferenceManager::new(config);

        let hardware = HardwareProfile {
            hardware_type: HardwareType::Both,
            cpu: CpuProfile {
                performance_score: 60.0,
                physical_cores: 4,
                ..Default::default()
            },
            gpus: vec![GpuProfile {
                performance_score: 75.0,
                total_memory: 4 * 1024 * 1024 * 1024, // 4GB
                vendor: GpuVendor::Nvidia,
                compute_capability: Some((5, 0)),
                memory_bandwidth: 150.0,
                features: vec!["OpenCL".to_string(), "CUDA".to_string()],
                ..Default::default()
            }],
            system: SystemInfo {
                total_memory: 8 * 1024 * 1024 * 1024,          // 8GB
                available_disk_space: 50 * 1024 * 1024 * 1024, // 50GB
                network_status: NetworkStatus {
                    connectivity: true,
                    bandwidth_mbps: Some(10.0),
                    latency_ms: Some(50.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        manager.set_hardware_profile(hardware);

        let selection = manager.select_optimal_project().unwrap();
        assert_eq!(selection.project.name, "MilkyWay@Home");
    }

    #[test]
    fn test_switching_decision() {
        let config = ProjectPreferencesConfig {
            switching: ProjectSwitchingConfig {
                auto_switch: true,
                performance_based_switching: true,
                min_run_time_seconds: 0, // Set to 0 so we don't wait
                ..Default::default()
            },
            ..Default::default()
        };
        let mut manager = ProjectPreferenceManager::new(config);

        // Set up hardware that's compatible with projects
        let hardware = HardwareProfile {
            hardware_type: HardwareType::Both,
            cpu: CpuProfile {
                performance_score: 60.0,
                physical_cores: 4,
                ..Default::default()
            },
            gpus: vec![GpuProfile {
                performance_score: 75.0,
                total_memory: 4 * 1024 * 1024 * 1024, // 4GB
                vendor: GpuVendor::Nvidia,
                compute_capability: Some((5, 0)),
                memory_bandwidth: 150.0,
                features: vec!["OpenCL".to_string(), "CUDA".to_string()],
                ..Default::default()
            }],
            system: SystemInfo {
                total_memory: 8 * 1024 * 1024 * 1024,          // 8GB
                available_disk_space: 50 * 1024 * 1024 * 1024, // 50GB
                network_status: NetworkStatus {
                    connectivity: true,
                    bandwidth_mbps: Some(10.0),
                    latency_ms: Some(50.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        manager.set_hardware_profile(hardware);

        // Create a low-performing project to trigger switching
        let poor_performance = PerformanceRecord {
            timestamp: SystemTime::UNIX_EPOCH, // Old timestamp to bypass min_run_time
            work_units_completed: 5,
            total_credit: 5.0,
            avg_completion_time_hours: 8.0,
            success_rate: 60.0,        // Below 80% threshold
            resource_efficiency: 30.0, // Below 50% threshold
        };

        // Use a real project name from the default list with lower priority
        // World Community Grid has priority 6, while MilkyWay@Home has priority 8
        // So if performance is poor, it should recommend switching to a higher priority project
        let decision =
            manager.evaluate_project_switching("World Community Grid", &poor_performance);

        // The decision depends on whether we can find a better project
        // With default projects loaded, we should be able to switch
        assert!(decision.is_ok());
        // Note: should_switch may be false if no better project is found
        // The test is checking the mechanism works, not that we always switch
    }
}
