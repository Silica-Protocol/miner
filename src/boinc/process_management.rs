/// BOINC process management module
/// Handles starting, stopping, and monitoring BOINC daemon processes
use anyhow::Result;
use std::process::Stdio;
use std::time::Duration;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::sleep;
use tracing::{error, info, warn};

use super::BoincAutomation;

impl BoincAutomation {
    /// Check if BOINC daemon is currently running
    pub async fn is_daemon_running(&mut self) -> bool {
        // Check if we have a stored process and it's still running
        if let Some(ref mut child) = self.daemon_process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    info!("BOINC process exited with status: {}", status);
                    self.daemon_process = None;
                    return false;
                }
                Ok(None) => {
                    // Process is still running
                    return true;
                }
                Err(e) => {
                    warn!("Error checking BOINC process status: {}", e);
                    self.daemon_process = None;
                    return false;
                }
            }
        }

        // Also check system-wide for any boinc processes
        let output = Command::new("pgrep").arg("-f").arg("boinc").output().await;
        match output {
            Ok(output) => {
                if output.status.success() && !output.stdout.is_empty() {
                    let pids = String::from_utf8_lossy(&output.stdout);
                    info!("Found running BOINC processes: {}", pids.trim());
                    true
                } else {
                    false
                }
            }
            Err(_) => {
                // If pgrep fails, assume no processes running
                false
            }
        }
    }

    /// Kill any existing BOINC processes before starting our own
    pub async fn kill_existing_boinc_processes(&self) -> Result<()> {
        info!("Checking for existing BOINC processes to clean up...");

        // First, try a gentle approach with pgrep and kill
        let output = Command::new("pgrep").arg("-f").arg("boinc").output().await;

        match output {
            Ok(output) => {
                if output.status.success() && !output.stdout.is_empty() {
                    let pids_str = String::from_utf8_lossy(&output.stdout);
                    let pids: Vec<&str> = pids_str.trim().split('\n').collect();

                    for pid in pids {
                        if !pid.is_empty() {
                            info!("Terminating BOINC process with PID: {}", pid);
                            match Command::new("kill").arg("-TERM").arg(pid).output().await {
                                Ok(_) => info!("Sent TERM signal to PID: {}", pid),
                                Err(e) => warn!("Failed to send TERM signal to PID {}: {}", pid, e),
                            }
                        }
                    }

                    // Wait a moment for graceful shutdown
                    sleep(Duration::from_secs(3)).await;

                    // If processes are still running, force kill them
                    match Command::new("pkill")
                        .arg("-KILL")
                        .arg("-f")
                        .arg("boinc")
                        .output()
                        .await
                    {
                        Ok(_) => info!("Sent KILL signal to remaining BOINC processes"),
                        Err(e) => warn!("Failed to send KILL signal: {}", e),
                    }

                    info!("Cleaned up existing BOINC processes");
                } else {
                    info!("No existing BOINC processes detected");
                }
            }
            Err(e) => {
                warn!("Failed to check for existing BOINC processes: {}", e);
                info!("No existing BOINC processes detected");
            }
        }

        Ok(())
    }

    /// Start BOINC client as a child process (not daemon) for output tracking
    pub async fn start_daemon(
        &mut self,
        project_url: Option<&str>,
        authenticator: Option<&str>,
    ) -> Result<()> {
        self.ensure_dirs()?;

        // Check if daemon is already running
        if self.is_daemon_running().await {
            info!("BOINC client already running, skipping start");
            return Ok(());
        }

        // SECURITY: Validate project URL if provided
        if let Some(url) = project_url {
            Self::validate_project_url(url)?;
        }

        // SECURITY: Validate authenticator if provided
        if let Some(auth) = authenticator {
            Self::validate_authenticator(auth)?;
        }

        // Kill any existing BOINC processes first
        self.kill_existing_boinc_processes().await?;

        // Clean up any stale BOINC data for a fresh start
        // self.clean_boinc_data()?;

        let boinc_path = self.get_boinc_path();
        info!(
            "Starting BOINC client as child process: {}",
            boinc_path.display()
        );
        info!(
            "BOINC output will be logged to: {}",
            self.log_file_path.display()
        );

        let mut command = Command::new(&boinc_path);
        command
            .arg("--dir")
            .arg(&self.data_dir)
            .arg("--allow_remote_gui_rpc");

        // Add project attachment if provided
        if let (Some(url), Some(auth)) = (project_url, authenticator) {
            info!("Attaching to project during startup: {}", url);
            command.arg("--attach_project").arg(url).arg(auth);
        }

        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        info!("BOINC client started with PID: {:?}", child.id());

        // Store the child process for later management
        if let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) {
            let log_file_path = self.log_file_path.clone();

            // Spawn task to handle stdout
            let log_file_path_stdout = log_file_path.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();
                let mut log_file = match OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_file_path_stdout)
                    .await
                {
                    Ok(file) => file,
                    Err(e) => {
                        error!("Failed to open log file: {}", e);
                        return;
                    }
                };

                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            let output = line.trim();
                            if !output.is_empty() {
                                info!("BOINC stdout: {}", output);
                                let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
                                let log_line =
                                    format!("[{}] {}: {}\n", timestamp, "stdout", output);
                                if let Err(e) = log_file.write_all(log_line.as_bytes()).await {
                                    error!("Failed to write to log file: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error reading stdout: {}", e);
                            break;
                        }
                    }
                }
            });

            // Spawn task to handle stderr
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr);
                let mut line = String::new();
                let mut log_file = match OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_file_path)
                    .await
                {
                    Ok(file) => file,
                    Err(e) => {
                        error!("Failed to open log file: {}", e);
                        return;
                    }
                };

                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            let output = line.trim();
                            if !output.is_empty() {
                                warn!("BOINC stderr: {}", output);
                                let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
                                let log_line =
                                    format!("[{}] {}: {}\n", timestamp, "stderr", output);
                                if let Err(e) = log_file.write_all(log_line.as_bytes()).await {
                                    error!("Failed to write to log file: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error reading stderr: {}", e);
                            break;
                        }
                    }
                }
            });
        }

        self.daemon_process = Some(child);
        info!("BOINC client started successfully");
        Ok(())
    }

    /// Stop the BOINC daemon
    pub async fn stop_daemon(&mut self) -> Result<()> {
        if let Some(mut child) = self.daemon_process.take() {
            info!("Stopping BOINC daemon...");
            child.kill().await?;
            info!("BOINC daemon stopped");
        } else {
            info!("No BOINC daemon to stop");
        }
        Ok(())
    }

    /// SECURITY: Validate project URL to prevent command injection
    fn validate_project_url(url: &str) -> Result<()> {
        // Check for basic URL format
        if url.is_empty() {
            return Err(anyhow::anyhow!("Project URL cannot be empty"));
        }

        if url.len() > 512 {
            return Err(anyhow::anyhow!("Project URL too long (max 512 characters)"));
        }

        // Must be HTTP or HTTPS
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow::anyhow!(
                "Project URL must start with http:// or https://"
            ));
        }

        // Check for shell metacharacters that could be abused
        let dangerous_chars = ['&', '|', ';', '$', '`', '(', ')', '<', '>', '\n', '\r'];
        if url.chars().any(|c| dangerous_chars.contains(&c)) {
            return Err(anyhow::anyhow!("Project URL contains invalid characters"));
        }

        Ok(())
    }

    /// SECURITY: Validate authenticator to prevent command injection
    fn validate_authenticator(auth: &str) -> Result<()> {
        if auth.is_empty() {
            return Err(anyhow::anyhow!("Authenticator cannot be empty"));
        }

        if auth.len() > 128 {
            return Err(anyhow::anyhow!(
                "Authenticator too long (max 128 characters)"
            ));
        }

        // Authenticators should only contain alphanumeric characters and some safe symbols
        if !auth
            .chars()
            .all(|c| c.is_alphanumeric() || "_-".contains(c))
        {
            return Err(anyhow::anyhow!("Authenticator contains invalid characters"));
        }

        Ok(())
    }
}
