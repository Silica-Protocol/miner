use anyhow::Result;
use miner::boinc_client::run_job_with_boinc_client;
use miner::config::{MinerConfig, MinerMode};
use miner::miner_core::run_miner;
use miner::miner_tui::MinerTui;
use miner::oracle_profile::OracleProfileManager;
use miner::security_logger::SecurityLogger;
use std::env;
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration from environment
    let config = match MinerConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            eprintln!("Please check your environment variables or create a .env file");
            eprintln!("See .env.template for required configuration");
            std::process::exit(1);
        }
    };

    // Initialize logging based on configuration
    if config.debug.verbose_logging {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    info!(
        "Starting Chert miner with user ID: {}",
        SecurityLogger::redact_user_id(&config.user_id)
    );

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    // Check for specific mode flags
    let use_tui = args.contains(&"--tui".to_string()) || matches!(config.mode, MinerMode::Tui);
    let use_legacy = args.contains(&"--legacy".to_string());
    let nuw_only = args.contains(&"--nuw-only".to_string());
    let boinc_only = args.contains(&"--boinc-only".to_string());

    if use_tui {
        run_with_tui(config).await
    } else if use_legacy {
        // Legacy mode - original BOINC-only implementation
        run_legacy(config).await
    } else {
        // New unified miner core
        run_unified(config, nuw_only, boinc_only).await
    }
}

/// Run with the new unified MinerCore
async fn run_unified(mut config: MinerConfig, nuw_only: bool, boinc_only: bool) -> Result<()> {
    info!("Starting Chert Miner (Unified Mode)");

    // Override work allocation based on flags
    if nuw_only {
        info!("Mode: NUW-only (no BOINC work)");
        config.work_allocation.nuw_on_cpu = true;
        config.work_allocation.boinc_on_gpu = false;
    } else if boinc_only {
        info!("Mode: BOINC-only (no NUW work)");
        config.work_allocation.nuw_on_cpu = false;
        config.work_allocation.boinc_on_gpu = true;
    } else {
        info!("Mode: Mixed (NUW on CPU, BOINC on GPU)");
    }

    // Run the miner
    run_miner(config).await
}

async fn run_with_tui(config: MinerConfig) -> Result<()> {
    info!("Starting TUI mode");

    // Initialize oracle profile manager for hardware detection and registration
    let mut profile_manager = OracleProfileManager::new(&config);
    if let Err(e) = profile_manager.initialize().await {
        warn!("Failed to initialize oracle profile manager: {}", e);
        warn!("Continuing without smart task selection...");
    } else if let Some(profile) = profile_manager.hardware_profile() {
        info!(
            "Hardware profile: {} cores, {} GPU(s), {:.1} GB RAM",
            profile.cpu.physical_cores,
            profile.gpus.len(),
            profile.system.total_memory as f64 / (1024.0 * 1024.0 * 1024.0)
        );
    }

    // Start BOINC client in background
    let boinc_data_dir = config.boinc_data_dir.to_string_lossy().to_string();

    // Start BOINC client task
    let config_clone = config.clone();
    let boinc_handle = tokio::spawn(async move {
        loop {
            if let Err(e) = run_job_with_boinc_client(&config_clone).await {
                error!("BOINC client error: {}", e);
                sleep(Duration::from_secs(10)).await;
            }
        }
    });

    // Start TUI
    let mut tui = MinerTui::new(boinc_data_dir);

    // Run TUI (this blocks until user quits)
    if let Err(e) = tui.run().await {
        error!("TUI error: {}", e);
    }

    // Cancel BOINC client when TUI exits
    boinc_handle.abort();

    Ok(())
}

/// Legacy mode - original BOINC-only implementation for backward compatibility
async fn run_legacy(config: MinerConfig) -> Result<()> {
    info!("Starting Chert BOINC Miner (Legacy Mode)");
    info!(
        "Oracle URL: {}",
        SecurityLogger::redact_url(&config.oracle_url)
    );
    info!(
        "User ID: {}",
        SecurityLogger::redact_user_id(&config.user_id)
    );

    if config.debug.debug_mode {
        info!("Debug mode enabled - additional logging active");
    }

    // Initialize oracle profile manager for smart task selection
    let mut profile_manager = OracleProfileManager::new(&config);

    info!("Detecting hardware capabilities...");
    match profile_manager.initialize().await {
        Ok(()) => {
            if let Some(profile) = profile_manager.hardware_profile() {
                info!("Hardware detected:");
                info!(
                    "  CPU: {} cores / {} threads",
                    profile.cpu.physical_cores, profile.cpu.logical_threads
                );
                if !profile.gpus.is_empty() {
                    for (i, gpu) in profile.gpus.iter().enumerate() {
                        info!(
                            "  GPU {}: {} ({} MB VRAM)",
                            i,
                            gpu.vendor_model,
                            gpu.total_memory / (1024 * 1024)
                        );
                    }
                } else {
                    info!("  GPU: None detected");
                }
                info!(
                    "  RAM: {:.1} GB",
                    profile.system.total_memory as f64 / (1024.0 * 1024.0 * 1024.0)
                );
            }

            if profile_manager.is_registered() {
                info!("Successfully registered hardware profile with oracle");

                // Get project recommendations
                match profile_manager.get_recommendations().await {
                    Ok(recommendations) if !recommendations.is_empty() => {
                        info!("Recommended projects:");
                        for rec in recommendations.iter().take(3) {
                            info!(
                                "  #{} {} (score: {:.1}, reward: {:.2}x)",
                                rec.rank, rec.project_name, rec.score, rec.estimated_reward
                            );
                        }
                    }
                    Ok(_) => {
                        info!("No specific recommendations - using default project selection");
                    }
                    Err(e) => {
                        warn!("Could not fetch recommendations: {}", e);
                    }
                }
            } else {
                warn!("Could not register with oracle - using local project selection");
            }
        }
        Err(e) => {
            warn!("Hardware detection failed: {}", e);
            warn!("Continuing with basic configuration...");
        }
    }

    info!(
        "Setting up BOINC client to connect through oracle at: {}",
        config.oracle_url
    );
    info!("BOINC client will connect to oracle, which forwards to real BOINC projects");

    // Use real BOINC client connected to our oracle
    run_job_with_boinc_client(&config).await?;

    Ok(())
}
