/// BOINC work processing module
/// Handles job submission, status checking, and result processing
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

use super::BoincAutomation;

impl BoincAutomation {
    /// Submit a work unit to the BOINC client (placeholder implementation)
    pub async fn submit_work_unit(&self, _work_unit_data: &str) -> Result<String> {
        // This would integrate with actual BOINC work unit submission
        // For now, simulate by running a BOINC command
        let boinc_path = self.get_boinc_path();

        let output = tokio::process::Command::new(&boinc_path)
            .arg("--get_project_status")
            .current_dir(&self.data_dir)
            .output()
            .await?;

        let status_output = String::from_utf8_lossy(&output.stdout);
        info!(response = status_output.trim(), "queried BOINC status");
        Ok(format!(
            "work_unit_12345_{}",
            chrono::Utc::now().timestamp()
        ))
    }

    /// Get the status of a submitted job
    pub async fn get_job_status(&self, _job_id: &str) -> Result<String> {
        // Placeholder: In a real implementation, this would query BOINC client
        // for the actual job status using the job ID
        Ok("running".to_string())
    }

    /// Wait for a job to complete with timeout
    pub async fn wait_for_completion(&self, job_id: &str, timeout_secs: u64) -> Result<String> {
        let start_time = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(timeout_secs);

        loop {
            if start_time.elapsed() > timeout_duration {
                return Err(anyhow::anyhow!(
                    "Job {} timed out after {} seconds",
                    job_id,
                    timeout_secs
                ));
            }

            let status = self.get_job_status(job_id).await?;
            if status == "completed" {
                return Ok("Job completed successfully".to_string());
            }

            sleep(Duration::from_secs(5)).await;
        }
    }

    /// Execute a local binary with work input and capture output
    /// This is used for running BOINC work units that have been downloaded
    pub fn run_local_binary(
        binary_path: &Path,
        work_dir: &Path,
        args: &[&str],
        result_file: &Path,
    ) -> Result<String> {
        // SECURITY: Validate binary path
        if !binary_path.exists() {
            return Err(anyhow::anyhow!("Binary does not exist: {:?}", binary_path));
        }

        if !binary_path.is_file() {
            return Err(anyhow::anyhow!(
                "Binary path is not a file: {:?}",
                binary_path
            ));
        }

        // SECURITY: Ensure the work directory exists and is safe
        if !work_dir.exists() {
            return Err(anyhow::anyhow!(
                "Work directory does not exist: {:?}",
                work_dir
            ));
        }

        if !work_dir.is_dir() {
            return Err(anyhow::anyhow!(
                "Work path is not a directory: {:?}",
                work_dir
            ));
        }

        // SECURITY: Validate arguments don't contain shell metacharacters
        for arg in args {
            if arg.contains(['&', '|', ';', '$', '`', '(', ')', '<', '>', '\n', '\r']) {
                return Err(anyhow::anyhow!(
                    "Argument contains dangerous characters: {}",
                    arg
                ));
            }
        }

        info!(
            "SECURITY: Executing binary with validated inputs: {:?}",
            binary_path
        );

        // Ensure executable bit on unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(binary_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(binary_path, perms)?;
        }

        let status = std::process::Command::new(binary_path)
            .args(args)
            .current_dir(work_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("binary failed with status: {:?}", status));
        }

        let content = fs::read_to_string(result_file)?;
        Ok(content)
    }
}
