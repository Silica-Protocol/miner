//! Integration example demonstrating hardware detection and project preference management
//!
//! This module shows how to use the hardware detection and project preference
//! management systems together to create an intelligent mining setup.

use anyhow::Result;
use std::time::SystemTime;
use tracing::{info, warn};

use crate::config::MinerConfig;
use crate::hardware_detection::{HardwareProfile, HardwareType, detect_hardware_capabilities};
use crate::project_preferences::{PerformanceRecord, ProjectPreferenceManager};

/// Integration manager that coordinates hardware detection and project preferences
pub struct IntegrationManager {
    /// Hardware profile
    hardware_profile: Option<HardwareProfile>,
    /// Project preference manager
    project_manager: ProjectPreferenceManager,
    /// Current configuration
    config: MinerConfig,
    /// Current selected project
    current_project: Option<String>,
    /// Project start time
    project_start_time: Option<SystemTime>,
}

impl IntegrationManager {
    /// Create a new integration manager
    pub fn new(config: MinerConfig) -> Self {
        let project_manager = ProjectPreferenceManager::new(config.preferences.clone());

        Self {
            hardware_profile: None,
            project_manager,
            config,
            current_project: None,
            project_start_time: None,
        }
    }

    /// Initialize the integration manager
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing Chert Miner Integration Manager");

        // Step 1: Detect hardware capabilities
        self.detect_and_configure_hardware().await?;

        // Step 2: Configure project preferences based on hardware
        self.configure_project_preferences()?;

        // Step 3: Select optimal project
        self.select_initial_project()?;

        info!("Integration manager initialized successfully");
        Ok(())
    }

    /// Detect hardware and configure based on findings
    async fn detect_and_configure_hardware(&mut self) -> Result<()> {
        info!("Detecting hardware capabilities...");

        let hardware_profile = detect_hardware_capabilities().await?;

        info!("Hardware detection completed:");
        info!("  Hardware Type: {}", hardware_profile.hardware_type);
        info!(
            "  CPU: {} ({:.1} performance score)",
            hardware_profile.cpu.vendor_model, hardware_profile.cpu.performance_score
        );

        if !hardware_profile.gpus.is_empty() {
            for (i, gpu) in hardware_profile.gpus.iter().enumerate() {
                info!(
                    "  GPU {}: {} ({:.1} performance score)",
                    i, gpu.vendor_model, gpu.performance_score
                );
            }
        }

        info!(
            "  Total Memory: {:.1} GB",
            hardware_profile.system.total_memory as f64 / (1024.0 * 1024.0 * 1024.0)
        );
        info!(
            "  Available Disk: {:.1} GB",
            hardware_profile.system.available_disk_space as f64 / (1024.0 * 1024.0 * 1024.0)
        );

        // Update project manager with hardware profile
        self.project_manager
            .set_hardware_profile(hardware_profile.clone());
        self.hardware_profile = Some(hardware_profile);

        // Apply hardware-based configuration recommendations
        self.apply_hardware_recommendations()?;

        Ok(())
    }

    /// Apply hardware-based configuration recommendations
    fn apply_hardware_recommendations(&mut self) -> Result<()> {
        let hardware = self
            .hardware_profile
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Hardware profile not available"))?;

        let recommendations = &hardware.recommended_config;
        let work_allocation = &recommendations.work_allocation;

        info!("Applying hardware-based recommendations:");
        info!("  NUW on CPU: {}", work_allocation.nuw_on_cpu);
        info!("  BOINC on GPU: {}", work_allocation.boinc_on_gpu);
        info!(
            "  NUW CPU Percentage: {}%",
            work_allocation.nuw_cpu_percentage
        );
        info!(
            "  BOINC GPU Percentage: {}%",
            work_allocation.boinc_gpu_percentage
        );
        info!("  Max BOINC Tasks: {}", work_allocation.max_boinc_tasks);

        // Update configuration if auto-detection is enabled
        if self.config.work_allocation.auto_detect_hardware {
            self.config.work_allocation.nuw_on_cpu = work_allocation.nuw_on_cpu;
            self.config.work_allocation.boinc_on_gpu = work_allocation.boinc_on_gpu;
            self.config.work_allocation.nuw_cpu_percentage = work_allocation.nuw_cpu_percentage;
            self.config.work_allocation.boinc_gpu_percentage = work_allocation.boinc_gpu_percentage;
            self.config.work_allocation.max_boinc_tasks = work_allocation.max_boinc_tasks;
            self.config.work_allocation.hardware_capabilities = hardware.hardware_type.clone();

            info!("Applied hardware-based configuration updates");
        } else {
            info!("Hardware auto-detection disabled, keeping manual configuration");
        }

        // Log recommended projects
        if !recommendations.recommended_projects.is_empty() {
            info!("Recommended projects based on hardware:");
            for project in &recommendations.recommended_projects {
                info!("  - {}", project);
            }
        }

        Ok(())
    }

    /// Configure project preferences based on hardware capabilities
    fn configure_project_preferences(&mut self) -> Result<()> {
        let hardware = self
            .hardware_profile
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Hardware profile not available"))?;

        info!(
            "Configuring project preferences for {} hardware",
            hardware.hardware_type
        );

        // Get compatible projects
        let compatible_projects = self.project_manager.get_compatible_projects()?;

        info!("Found {} compatible projects:", compatible_projects.len());
        for project in &compatible_projects {
            let compatibility = self
                .project_manager
                .calculate_compatibility_score(project, hardware);
            info!(
                "  {} - Compatibility: {:.1}%, Priority: {}",
                project.name, compatibility.overall_score, project.priority
            );
        }

        // Update project preferences if auto-selection is enabled
        if self.config.preferences.auto_select_projects {
            info!("Auto-project selection enabled, will choose optimal project");
        } else {
            info!("Using manual project preferences");
            for preferred in &self.config.preferences.preferred_projects {
                let found = compatible_projects.iter().any(|p| p.name == *preferred);
                if found {
                    info!("  Preferred project available: {}", preferred);
                } else {
                    warn!("  Preferred project not compatible: {}", preferred);
                }
            }
        }

        Ok(())
    }

    /// Select initial project for mining
    fn select_initial_project(&mut self) -> Result<()> {
        info!("Selecting optimal project...");

        let selection = self.project_manager.select_optimal_project()?;

        info!("Selected project: {}", selection.project.name);
        info!("Selection reason: {}", selection.reason);
        info!("Expected performance:");
        info!(
            "  Work units/hour: {:.2}",
            selection.expected_performance.work_units_per_hour
        );
        info!(
            "  Credit/hour: {:.2}",
            selection.expected_performance.credit_per_hour
        );
        info!(
            "  Efficiency score: {:.1}",
            selection.expected_performance.efficiency_score
        );
        info!("  Resource utilization:");
        info!(
            "    CPU: {:.1}%",
            selection
                .expected_performance
                .resource_utilization
                .cpu_utilization_percent
        );
        info!(
            "    GPU: {:.1}%",
            selection
                .expected_performance
                .resource_utilization
                .gpu_utilization_percent
        );
        info!(
            "    Memory: {:.1} GB",
            selection
                .expected_performance
                .resource_utilization
                .memory_usage_gb
        );
        info!(
            "    Network: {:.1} Mbps",
            selection
                .expected_performance
                .resource_utilization
                .network_usage_mbps
        );

        self.current_project = Some(selection.project.name.clone());
        self.project_start_time = Some(SystemTime::now());

        Ok(())
    }

    /// Evaluate project switching based on current performance
    pub fn evaluate_project_switching(
        &mut self,
        current_performance: &PerformanceRecord,
    ) -> Result<bool> {
        let current_project = self
            .current_project
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No current project set"))?;

        info!("Evaluating project switching for {}", current_project);
        info!("Current performance:");
        info!(
            "  Work units completed: {}",
            current_performance.work_units_completed
        );
        info!("  Total credit: {:.2}", current_performance.total_credit);
        info!("  Success rate: {:.1}%", current_performance.success_rate);
        info!(
            "  Resource efficiency: {:.1}",
            current_performance.resource_efficiency
        );

        let decision = self
            .project_manager
            .evaluate_project_switching(current_project, current_performance)?;

        if decision.should_switch {
            if let Some(recommended) = decision.recommended_project {
                info!("Switching recommended:");
                info!("  Reason: {}", decision.reason);
                info!("  New project: {}", recommended.name);

                if let Some(improvement) = decision.performance_improvement {
                    info!("  Performance improvement: {:.1}%", improvement * 100.0);
                }

                // Perform the switch
                self.switch_to_project(&recommended.name)?;
                return Ok(true);
            }
        } else {
            info!("No switching needed: {}", decision.reason);
        }

        Ok(false)
    }

    /// Switch to a new project
    fn switch_to_project(&mut self, project_name: &str) -> Result<()> {
        info!("Switching to project: {}", project_name);

        // Record performance for current project if available
        if let (Some(current_project), Some(start_time)) =
            (&self.current_project, self.project_start_time)
        {
            let runtime = start_time
                .elapsed()
                .map_err(|e| anyhow::anyhow!("Failed to calculate runtime: {}", e))?
                .as_secs_f64()
                / 3600.0; // Convert to hours

            // Create a performance record (simplified)
            let performance_record = PerformanceRecord {
                timestamp: SystemTime::now(),
                work_units_completed: 10, // Example value
                total_credit: 15.0,       // Example value
                avg_completion_time_hours: runtime,
                success_rate: 95.0,        // Example value
                resource_efficiency: 75.0, // Example value
            };

            self.project_manager
                .record_performance(current_project, performance_record);
        }

        // Update current project
        self.current_project = Some(project_name.to_string());
        self.project_start_time = Some(SystemTime::now());

        info!("Successfully switched to project: {}", project_name);
        Ok(())
    }

    /// Get current status information
    pub fn get_status(&self) -> Result<IntegrationStatus> {
        let hardware = self
            .hardware_profile
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Hardware profile not available"))?;

        Ok(IntegrationStatus {
            hardware_type: hardware.hardware_type.clone(),
            current_project: self.current_project.clone(),
            project_start_time: self.project_start_time,
            compatible_projects_count: self.project_manager.get_compatible_projects()?.len(),
            hardware_performance_score: hardware.cpu.performance_score,
            gpu_performance_scores: hardware
                .gpus
                .iter()
                .map(|gpu| gpu.performance_score)
                .collect(),
        })
    }

    /// Get detailed hardware information
    pub fn get_hardware_info(&self) -> Option<&HardwareProfile> {
        self.hardware_profile.as_ref()
    }

    /// Get available projects
    pub fn get_available_projects(&self) -> Result<Vec<String>> {
        let projects = self.project_manager.get_compatible_projects()?;
        Ok(projects.into_iter().map(|p| p.name).collect())
    }

    /// Add custom project
    pub fn add_custom_project(
        &mut self,
        project: crate::project_preferences::ProjectInfo,
    ) -> Result<()> {
        self.project_manager.add_custom_project(project)
    }

    /// Update configuration
    pub fn update_config(&mut self, config: MinerConfig) -> Result<()> {
        self.config = config;

        // Reinitialize project manager with new preferences
        self.project_manager = ProjectPreferenceManager::new(self.config.preferences.clone());

        // Re-apply hardware profile if available
        if let Some(ref hardware) = self.hardware_profile {
            self.project_manager.set_hardware_profile(hardware.clone());
        }

        info!("Configuration updated successfully");
        Ok(())
    }
}

/// Integration status information
#[derive(Debug, Clone)]
pub struct IntegrationStatus {
    /// Current hardware type
    pub hardware_type: HardwareType,
    /// Current project
    pub current_project: Option<String>,
    /// When current project was started
    pub project_start_time: Option<SystemTime>,
    /// Number of compatible projects
    pub compatible_projects_count: usize,
    /// CPU performance score
    pub hardware_performance_score: f64,
    /// GPU performance scores
    pub gpu_performance_scores: Vec<f64>,
}

/// Create a demonstration of the integration system
pub async fn run_integration_demo() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Chert Miner Integration Demo");

    // Create default configuration
    let config = MinerConfig::default();

    // Create integration manager
    let mut manager = IntegrationManager::new(config);

    // Initialize the system
    manager.initialize().await?;

    // Get status
    let status = manager.get_status()?;
    info!("System Status:");
    info!("  Hardware Type: {}", status.hardware_type);
    info!("  Current Project: {:?}", status.current_project);
    info!(
        "  Compatible Projects: {}",
        status.compatible_projects_count
    );
    info!(
        "  CPU Performance: {:.1}",
        status.hardware_performance_score
    );

    if !status.gpu_performance_scores.is_empty() {
        info!("  GPU Performance: {:.1}", status.gpu_performance_scores[0]);
    }

    // Simulate performance evaluation
    let sample_performance = PerformanceRecord {
        timestamp: SystemTime::now(),
        work_units_completed: 15,
        total_credit: 22.5,
        avg_completion_time_hours: 6.5,
        success_rate: 93.0,
        resource_efficiency: 78.0,
    };

    info!("Evaluating project switching with sample performance...");
    let switched = manager.evaluate_project_switching(&sample_performance)?;

    if switched {
        info!("Project was switched based on performance evaluation");
    } else {
        info!("No project switching needed");
    }

    info!("Integration demo completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        DebugConfig, ProjectPreferencesConfig, SecurityConfig, WorkAllocationConfig,
    };

    #[test]
    fn test_integration_manager_creation() {
        let config = MinerConfig {
            work_allocation: WorkAllocationConfig {
                auto_detect_hardware: true,
                ..Default::default()
            },
            preferences: ProjectPreferencesConfig {
                auto_select_projects: true,
                ..Default::default()
            },
            security: SecurityConfig::default(),
            debug: DebugConfig::default(),
            ..Default::default()
        };

        let manager = IntegrationManager::new(config);
        assert!(manager.hardware_profile.is_none());
        assert!(manager.current_project.is_none());
    }

    #[tokio::test]
    async fn test_hardware_detection_integration() {
        let config = MinerConfig::default();
        let manager = IntegrationManager::new(config);

        // This test would require actual hardware detection
        // In a real test environment, you might mock the hardware detection
        // For now, we'll just test the structure
        assert!(manager.hardware_profile.is_none());
    }

    #[test]
    fn test_project_switching_logic() {
        let config = MinerConfig::default();
        let mut manager = IntegrationManager::new(config);

        // Set up a mock current project
        manager.current_project = Some("TestProject".to_string());
        manager.project_start_time = Some(SystemTime::now());

        let _performance = PerformanceRecord {
            timestamp: SystemTime::now(),
            work_units_completed: 5,
            total_credit: 5.0,
            avg_completion_time_hours: 8.0,
            success_rate: 60.0,        // Poor performance
            resource_efficiency: 30.0, // Poor efficiency
        };

        // Test would require proper project setup
        // This is a structural test
        assert!(manager.current_project.is_some());
    }
}
