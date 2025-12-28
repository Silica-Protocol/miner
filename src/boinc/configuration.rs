/// BOINC configuration management module
/// Handles client configuration files and system setup
use anyhow::Result;
use std::fs;
use std::io::Write;
use tracing::{info, warn};

use super::BoincAutomation;

impl BoincAutomation {
    /// Create optimized BOINC client configuration
    pub fn create_client_config(&self) -> Result<()> {
        self.ensure_dirs()?;

        let config_content = r#"
<cc_config>
    <options>
        <!-- General settings -->
        <max_ncpus_pct>90.0</max_ncpus_pct>
        <cpu_usage_limit>95.0</cpu_usage_limit>
        
        <!-- Networking -->
        <max_bytes_sec_up>1000000</max_bytes_sec_up>
        <max_bytes_sec_down>10000000</max_bytes_sec_down>
        <network_test_url>http://www.google.com/</network_test_url>
        
        <!-- Work management -->
        <work_buf_min_days>0.1</work_buf_min_days>
        <work_buf_additional_days>0.25</work_buf_additional_days>
        <max_ncpus>0</max_ncpus>
        
        <!-- Disk management -->
        <disk_max_used_gb>50.0</disk_max_used_gb>
        <disk_max_used_pct>90.0</disk_max_used_pct>
        <disk_min_free_gb>1.0</disk_min_free_gb>
        
        <!-- Memory management -->
        <ram_max_used_busy_frac>50.0</ram_max_used_busy_frac>
        <ram_max_used_idle_frac>90.0</ram_max_used_idle_frac>
        <vm_max_used_frac>75.0</vm_max_used_frac>
        
        <!-- Scheduling -->
        <cpu_scheduling_period_minutes>60</cpu_scheduling_period_minutes>
        <dont_verify_images>0</dont_verify_images>
        
        <!-- For proxy testing -->
        <proxy_info>
            <use_http_proxy>0</use_http_proxy>
            <use_socks_proxy>0</use_socks_proxy>
            <use_http_authentication>0</use_http_authentication>
        </proxy_info>
    </options>
    
    <log_flags>
        <task>1</task>
        <file_xfer>1</file_xfer>
        <sched_ops>1</sched_ops>
        <task_debug>0</task_debug>
        <file_xfer_debug>0</file_xfer_debug>
        <sched_op_debug>0</sched_op_debug>
        <http_debug>0</http_debug>
        <proxy_debug>0</proxy_debug>
        <time_debug>0</time_debug>
        <net_xfer_debug>0</net_xfer_debug>
        <measurement_debug>0</measurement_debug>
        <poll_debug>0</poll_debug>
        <guirpc_debug>0</guirpc_debug>
        <scrsave_debug>0</scrsave_debug>
        <app_msg_debug>0</app_msg_debug>
        <statefile_debug>0</statefile_debug>
        <task_debug>0</task_debug>
        <benchmark_debug>0</benchmark_debug>
        <unparsed_xml>0</unparsed_xml>
        <std_cerr>0</std_cerr>
    </log_flags>
</cc_config>
"#;

        let config_path = self.data_dir.join("cc_config.xml");
        fs::write(&config_path, config_content)?;
        info!(
            "Created BOINC client configuration: {}",
            config_path.display()
        );

        // Create global preferences file for more granular control
        let global_prefs_content = r#"
<global_preferences>
    <run_on_batteries>0</run_on_batteries>
    <run_if_user_active>1</run_if_user_active>
    <run_gpu_if_user_active>1</run_gpu_if_user_active>
    <suspend_cpu_usage>25.0</suspend_cpu_usage>
    <suspend_if_no_recent_input>0.0</suspend_if_no_recent_input>
    <start_hour>0.0</start_hour>
    <end_hour>24.0</end_hour>
    <net_start_hour>0.0</net_start_hour>
    <net_end_hour>24.0</net_end_hour>
    <leave_apps_in_memory>0</leave_apps_in_memory>
    <confirm_before_connecting>0</confirm_before_connecting>
    <hangup_if_dialed>0</hangup_if_dialed>
    <dont_verify_images>0</dont_verify_images>
    <work_buf_min_days>0.1</work_buf_min_days>
    <work_buf_additional_days>0.5</work_buf_additional_days>
    <max_ncpus_pct>100.0</max_ncpus_pct>
    <cpu_scheduling_period_minutes>60</cpu_scheduling_period_minutes>
    <disk_interval>60</disk_interval>
    <disk_max_used_gb>0.0</disk_max_used_gb>
    <disk_max_used_pct>50.0</disk_max_used_pct>
    <disk_min_free_gb>0.1</disk_min_free_gb>
    <vm_max_used_frac>75.0</vm_max_used_frac>
    <ram_max_used_busy_frac>50.0</ram_max_used_busy_frac>
    <ram_max_used_idle_frac>90.0</ram_max_used_idle_frac>
    <max_bytes_sec_up>0.0</max_bytes_sec_up>
    <max_bytes_sec_down>0.0</max_bytes_sec_down>
    <cpu_usage_limit>100.0</cpu_usage_limit>
    <daily_xfer_limit_mb>0.0</daily_xfer_limit_mb>
    <daily_xfer_period_days>0</daily_xfer_period_days>
</global_preferences>
"#;

        let global_prefs_path = self.data_dir.join("global_prefs_override.xml");
        fs::write(&global_prefs_path, global_prefs_content)?;
        info!(
            "Created BOINC global preferences: {}",
            global_prefs_path.display()
        );

        Ok(())
    }

    /// Setup hosts file entry for boincproject.local.com -> localhost
    /// This is needed because BOINC client rejects localhost URLs
    pub fn setup_hosts_entry(&self) -> Result<()> {
        let hosts_entry = "127.0.0.1 boincproject.local.com";

        #[cfg(unix)]
        {
            let hosts_file = "/etc/hosts";

            // Check if entry already exists
            if let Ok(content) = std::fs::read_to_string(hosts_file)
                && content.contains("boincproject.local.com")
            {
                info!("Hosts entry for boincproject.local.com already exists");
                return Ok(());
            }

            // Try to add the entry (requires root privileges)
            match std::fs::OpenOptions::new().append(true).open(hosts_file) {
                Ok(mut file) => {
                    writeln!(file, "\n# Added by Chert BOINC Miner")?;
                    writeln!(file, "{}", hosts_entry)?;
                    info!("Added hosts entry: {}", hosts_entry);
                    Ok(())
                }
                Err(_) => {
                    warn!("Cannot write to /etc/hosts (no root privileges)");
                    warn!("Please manually add this line to /etc/hosts:");
                    warn!("  {}", hosts_entry);
                    warn!("Or run with: sudo echo '{}' >> /etc/hosts", hosts_entry);
                    Ok(()) // Continue anyway - user can add manually
                }
            }
        }

        #[cfg(windows)]
        {
            let hosts_file = r"C:\Windows\System32\drivers\etc\hosts";
            warn!("Windows hosts file setup not implemented yet");
            warn!("Please manually add this line to {}:", hosts_file);
            warn!("  {}", hosts_entry);
            Ok(())
        }
    }
}
