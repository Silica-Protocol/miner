/// BOINC automation module
/// Modular BOINC client management system with focused responsibilities
///
/// This module has been refactored from a single 857+ line file into focused sub-modules:
/// - installation: Download, verify, and install BOINC binaries
/// - configuration: Client configuration and system setup  
/// - process_management: Start, stop, and monitor BOINC processes
/// - work_processing: Submit work units and process results
/// - runner: Main BOINC work execution loop
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

// Sub-modules for focused responsibilities
mod configuration;
mod installation;
mod process_management;
pub mod runner;
mod work_processing;

// Re-export runner components
pub use runner::{BoincRunner, BoincStats, WorkUnit, run_boinc_worker};

/// Main BOINC automation structure
/// Handles full BOINC client lifecycle: detection, installation, configuration, and job management
pub struct BoincAutomation {
    pub install_dir: PathBuf,
    pub binary_path: PathBuf,
    pub data_dir: PathBuf,
    pub log_file_path: PathBuf,
    pub daemon_process: Option<tokio::process::Child>,
}

impl BoincAutomation {
    /// Create a new BOINC automation instance
    pub fn new(install_dir: impl Into<PathBuf>) -> Self {
        let install_dir = install_dir.into();
        let binary_path = install_dir.join("boinc");
        let data_dir = install_dir.join("data");
        let log_file_path = install_dir.join("boinc_output.log");
        Self {
            install_dir,
            binary_path,
            data_dir,
            log_file_path,
            daemon_process: None,
        }
    }

    /// Check if BOINC is already installed
    pub fn is_boinc_installed(&self) -> bool {
        self.get_boinc_path().exists()
    }

    /// Get the expected BOINC binary path
    /// This checks multiple possible locations for the BOINC executable
    pub fn get_boinc_path(&self) -> PathBuf {
        // Check our install directory first (direct binary)
        if self.binary_path.exists() && self.binary_path.is_file() {
            return self.binary_path.clone();
        }

        // Check extracted Debian package locations
        let extracted_paths = [
            self.install_dir
                .join("usr")
                .join("local")
                .join("bin")
                .join("boinc"),
            self.install_dir.join("usr").join("bin").join("boinc"),
        ];

        for path in &extracted_paths {
            if path.exists() && path.is_file() {
                return path.clone();
            }
        }

        // Check for system-wide installations
        let system_paths = [
            PathBuf::from("/usr/local/bin/boinc"),
            PathBuf::from("/usr/bin/boinc"),
            PathBuf::from("/opt/boinc/boinc"),
        ];

        for path in system_paths {
            if path.exists() && path.is_file() {
                return path;
            }
        }

        // Return our expected path even if it doesn't exist yet
        self.binary_path.clone()
    }

    /// Ensure required directories exist
    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.install_dir)?;
        fs::create_dir_all(&self.data_dir)?;
        Ok(())
    }

    /// Clean BOINC data directory for fresh start
    pub fn clean_boinc_data(&self) -> Result<()> {
        if self.data_dir.exists() {
            fs::remove_dir_all(&self.data_dir)?;
        }
        self.ensure_dirs()?;
        Ok(())
    }
}
