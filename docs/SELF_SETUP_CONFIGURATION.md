
# Self-Setup and Default Configuration Documentation

## Overview

The Chert miner is designed for easy deployment with comprehensive self-setup capabilities and intelligent default configurations. The system automatically detects hardware, optimizes settings, and provides a smooth onboarding experience for users of all technical levels.

## Automated Setup Process

### First-Time Setup Wizard

The miner includes an interactive setup wizard that guides users through initial configuration:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Chert Miner Setup Wizard                                    Step 1/6 │
├─────────────────────────────────────────────────────────────────────────────┤
│ Welcome to Chert Miner!                                           │
│                                                                   │
│ This wizard will help you configure your miner for optimal performance.   │
│                                                                   │
│ System Information:                                                   │
│ • CPU: Intel i7-10700K (8 cores, 16 threads)                   │
│ • GPU: NVIDIA RTX 3080 (10GB VRAM)                               │
│ • Memory: 32GB DDR4                                               │
│ • Storage: 500GB SSD                                               │
│ • Network: 1Gbps Ethernet                                          │
│                                                                   │
│ Recommended Configuration:                                             │
│ • Work Type: BOINC GPU + NUW CPU                                    │
│ • BOINC Project: MilkyWay@Home                                      │
│ • Resource Allocation: 75% GPU to BOINC, 25% CPU to NUW           │
│                                                                   │
│ [Enter] Continue | [Esc] Exit | [F1] Help                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Setup Steps

#### Step 1: Hardware Detection
- **CPU Analysis**: Detect cores, threads, cache sizes, capabilities
- **GPU Detection**: Identify GPUs, memory, compute capabilities
- **Memory Analysis**: Total RAM, available memory, speed
- **Storage Assessment**: Disk space, I/O capabilities
- **Network Evaluation**: Bandwidth, latency, connectivity

#### Step 2: Work Type Selection
- **Automatic Recommendation**: Based on hardware capabilities
- **User Preference**: Manual override options
- **Performance Preview**: Expected performance metrics
- **Resource Impact**: Resource usage estimates

#### Step 3: BOINC Configuration
- **Project Selection**: Choose scientific projects
- **Authentication Setup**: Project account configuration
- **Resource Limits**: Per-project resource allocation
- **Network Settings**: Proxy and bandwidth configuration

#### Step 4: Oracle Configuration
- **Oracle Selection**: Choose optimal oracle server
- **Network Settings**: Connection parameters
- **Security Options**: HTTPS, certificate verification
- **Authentication**: Miner identity setup

#### Step 5: Performance Tuning
- **Resource Allocation**: CPU/GPU percentage settings
- **Priority Configuration**: System priority levels
- **Thermal Management**: Temperature limits and throttling
- **Power Management**: Power usage optimization

#### Step 6: Finalization
- **Configuration Summary**: Review all settings
- **Test Run**: Validate configuration with test work
- **Save Settings**: Persist configuration
- **Start Mining**: Begin mining operations

## Default Configuration System

### Configuration Hierarchy

The miner uses a hierarchical configuration system:

```
1. Built-in Defaults    →    2. Environment Variables    →    3. Config Files    →    4. Command Line
       ↓                           ↓                           ↓                      ↓
   Fallback Values          Runtime Overrides          Persistent Settings      Temporary Changes
```

### Default Configuration Files

#### Main Configuration File
Location: `~/.chert/config.toml`

```toml
[oracle]
# Oracle server configuration
url = "https://oracle.chert.network"
timeout_seconds = 30
require_https = true
verify_certificates = true
retry_attempts = 3
retry_delay_seconds = 5

[boinc]
# BOINC client configuration
install_dir = "~/.chert/boinc"
data_dir = "~/.chert/boinc/data"
log_file = "~/.chert/boinc/boinc_output.log"
auto_install = true
auto_update = true

[boinc.projects]
# Project configuration
primary = "MilkyWay@Home"
secondary = ["Rosetta@Home"]
auto_switch = true
switch_interval_hours = 24
min_run_time_hours = 6

[boinc.projects.milkyway]
enabled = true
gpu_enabled = true
cpu_cores = 4
memory_limit_mb = 2048
priority = 1
resource_percentage = 75

[boinc.projects.rosetta]
enabled = false
gpu_enabled = true
cpu_cores = 2
memory_limit_mb = 4096
priority = 2
resource_percentage = 50

[work_allocation]
# Work type resource allocation
nuw_on_cpu = true
boinc_on_gpu = true
nuw_cpu_percentage = 25
boinc_gpu_percentage = 75
nuw_on_demand = true
min_nuw_difficulty = 1000
max_boinc_tasks = 2

[performance]
# Performance tuning
cpu_priority = "normal"
gpu_priority = "high"
memory_limit_percentage = 80
thermal_throttle_enabled = true
max_temperature_celsius = 85
power_management_enabled = true

[ui]
# User interface settings
mode = "tui"
theme = "default"
update_rate_hz = 4
show_advanced_metrics = false
enable_animations = true

[logging]
# Logging configuration
level = "info"
file_path = "~/.chert/logs/miner.log"
max_file_size_mb = 100
max_files = 5
enable_console = true
enable_file = true

[security]
# Security settings
require_https = true
verify_certificates = true
rate_limit_per_minute = 60
enable_audit_logging = true
encrypt_checkpoints = true

[continuation]
# Task continuation settings
checkpoint_interval_seconds = 300
max_checkpoints = 10
auto_recovery = true
validate_checkpoints = true
backup_enabled = true
```

### Environment Variable Defaults

The miner supports comprehensive environment variable configuration:

```bash
# Oracle Configuration
export CHERT_ORACLE_URL="https://oracle.chert.network"
export CHERT_ORACLE_TIMEOUT_SECS="30"
export CHERT_REQUIRE_HTTPS="true"
export CHERT_VERIFY_CERTIFICATES="true"

# BOINC Configuration
export CHERT_BOINC_INSTALL_DIR="~/.chert/boinc"
export CHERT_BOINC_DATA_DIR="~/.chert/boinc/data"
export CHERT_BOINC_LOG_FILE="~/.chert/boinc/boinc_output.log"
export CHERT_BOINC_AUTO_INSTALL="true"

# Work Allocation
export CHERT_NUW_ON_CPU="true"
export CHERT_BOINC_ON_GPU="true"
export CHERT_NUW_CPU_PERCENTAGE="25"
export CHERT_BOINC_GPU_PERCENTAGE="75"
export CHERT_NUW_ON_DEMAND="true"
export CHERT_MIN_NUW_DIFFICULTY="1000"
export CHERT_MAX_BOINC_TASKS="2"

# Performance
export CHERT_CPU_PRIORITY="normal"
export CHERT_GPU_PRIORITY="high"
export CHERT_MEMORY_LIMIT_PERCENTAGE="80"
export CHERT_THERMAL_THROTTLE_ENABLED="true"
export CHERT_MAX_TEMPERATURE_CELSIUS="85"

# UI Configuration
export CHERT_UI_MODE="tui"
export CHERT_UI_THEME="default"
export CHERT_UI_UPDATE_RATE_HZ="4"
export CHERT_UI_SHOW_ADVANCED_METRICS="false"

# Logging
export CHERT_LOG_LEVEL="info"
export CHERT_LOG_FILE_PATH="~/.chert/logs/miner.log"
export CHERT_LOG_MAX_FILE_SIZE_MB="100"
export CHERT_LOG_MAX_FILES="5"

# Security
export CHERT_RATE_LIMIT_REQUESTS_PER_MINUTE="60"
export CHERT_ENABLE_AUDIT_LOGGING="true"
export CHERT_ENCRYPT_CHECKPOINTS="true"
```

## Hardware Detection and Optimization

### Automatic Hardware Profiling

The miner performs comprehensive hardware detection:

#### CPU Detection
```rust
pub struct CpuProfile {
    /// CPU vendor and model
    pub vendor_model: String,
    /// Number of physical cores
    pub physical_cores: usize,
    /// Number of logical threads
    pub logical_threads: usize,
    /// Cache sizes (L1, L2, L3)
    pub cache_sizes: CacheSizes,
    /// Supported instruction sets
    pub instruction_sets: Vec<String>,
    /// Base clock frequency
    pub base_frequency: f64,
    /// Maximum turbo frequency
    pub max_frequency: f64,
    /// Thermal design power
    pub tdp: f64,
    /// Performance characteristics
    pub performance_score: f64,
}
```

#### GPU Detection
```rust
pub struct GpuProfile {
    /// GPU vendor and model
    pub vendor_model: String,
    /// Available VRAM
    pub total_memory: u64,
    /// Compute capability
    pub compute_capability: (u8, u8),
    /// CUDA cores or stream processors
    pub processor_count: u32,
    /// Memory bandwidth
    pub memory_bandwidth: f64,
    /// Supported features
    pub features: Vec<String>,
    /// Performance characteristics
    pub performance_score: f64,
}
```

### Intelligent Configuration Generation

Based on hardware detection, the miner generates optimal configurations:

#### Configuration Algorithm
```rust
fn generate_optimal_config(
    cpu_profile: &CpuProfile,
    gpu_profile: &GpuProfile,
    system_info: &SystemInfo
) -> MinerConfig {
    let mut config = MinerConfig::default();
    
    // 1. Determine optimal work type mix
    if gpu_profile.performance_score > cpu_profile.performance_score * 2.0 {
        // GPU is significantly more powerful
        config.work_allocation.boinc_on_gpu = true;
        config.work_allocation.boinc_gpu_percentage = 75;
        config.work_allocation.nuw_on_cpu = true;
        config.work_allocation.nuw_cpu_percentage = 25;
    } else {
        // CPU is competitive or better
        config.work_allocation.boinc_on_gpu = true;
        config.work_allocation.boinc_gpu_percentage = 50;
        config.work_allocation.nuw_on_cpu = true;
        config.work_allocation.nuw_cpu_percentage = 50;
    }
    
    // 2. Optimize resource allocation
    config.performance.memory_limit_percentage = calculate_optimal_memory_usage(
        system_info.total_memory,
        &cpu_profile,
        &gpu_profile
    );
    
    // 3. Set thermal limits based on hardware
    config.performance.max_temperature_celsius = determine_safe_temperature(
        &cpu_profile.vendor_model,
        &gpu_profile.vendor_model
    );
    
    // 4. Configure BOINC projects based on capabilities
    config.boinc.projects = select_optimal_projects(
        &cpu_profile,
        &gpu_profile
    );
    
    config
}
```

## Installation and Deployment

### Automated Installation

The miner provides multiple installation methods:

#### Binary Installation (Recommended)
```bash
# Download and install latest release
curl -sSL https://install.chert.network | bash

# Interactive installation
chert-installer --interactive

# Silent installation with defaults
chert-installer --silent --config-path /opt/chert/config.toml
```

#### Package Manager Installation
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install chert-miner

# CentOS/RHEL
sudo yum install chert-miner

# macOS (Homebrew)
brew install chert/chert/chert-miner

# Windows (Chocolatey)
 install chert-miner

# Docker Installation
docker pull chert/miner:latest
docker run -d --name chert-miner -v ~/.chert:/data chert/miner:latest
```

#### Source Installation
```bash
# Clone repository
git clone https://github.com/chert-network/chert-miner.git
cd chert-miner

# Build from source
cargo build --release

# Install to system
sudo cp target/release/chert-miner /usr/local/bin/
sudo mkdir -p /etc/chert
sudo cp config/default.toml /etc/chert/
```

### Post-Installation Setup

#### Directory Structure Creation
```bash
# Create required directories
mkdir -p ~/.chert/{config,logs,checkpoints,boinc,data}

# Set appropriate permissions
chmod 755 ~/.chert
chmod 644 ~/.chert/config/*
chmod 600 ~/.chert/keys/*
```

#### Service Configuration
```bash
# Create systemd service (Linux)
sudo tee /etc/systemd/system/chert-miner.service > /dev/null <<EOF
[Unit]
Description=Chert Miner
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$HOME/.chert
ExecStart=/usr/local/bin/chert-miner --config ~/.chert/config.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl enable chert-miner
sudo systemctl start chert-miner
```

## Configuration Management

### Configuration Validation

The miner includes comprehensive configuration validation:

#### Validation Rules
```rust
pub struct ConfigValidator {
    /// Validation rules for each configuration section
    pub rules: HashMap<String, Vec<ValidationRule>>,
    /// Error messages for validation failures
    pub error_messages: HashMap<String, String>,
    /// Warning messages for suboptimal settings
    pub warning_messages: HashMap<String, String>,
}

impl ConfigValidator {
    pub fn validate_config(&self, config: &MinerConfig) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        // 1. Validate oracle configuration
        self.validate_oracle_config(&config.oracle, &mut result);
        
        // 2. Validate BOINC configuration
        self.validate_boinc_config(&config.boinc, &mut result);
        
        // 3. Validate work allocation
        self.validate_work_allocation(&config.work_allocation, &mut result);
        
        // 4. Validate performance settings
        self.validate_performance_config(&config.performance, &mut result);
        
        // 5. Validate security settings
        self.validate_security_config(&config.security, &mut result);
        
        // 6. Check for logical inconsistencies
        self.validate_logical_consistency(config, &mut result);
        
        result
    }
}
```

#### Configuration Repair
```rust
pub fn repair_config(config: &mut MinerConfig, issues: &[ValidationIssue]) -> Result<()> {
    for issue in issues {
        match issue.severity {
            Severity::Error => {
                return Err(anyhow::anyhow!("Cannot auto-repair error: {}", issue.message));
            }
            Severity::Warning => {
                warn!("Auto-repairing warning: {}", issue.message);
                apply_auto_repair(config, issue)?;
            }
            Severity::Info => {
                info!("Configuration suggestion: {}", issue.message);
            }
        }
    }
    Ok(())
}
```

### Configuration Templates

The miner provides pre-configured templates for common scenarios:

#### High Performance Template
```toml
[template]
name = "high_performance"
description = "Maximum performance with aggressive resource usage"

[work_allocation]
nuw_on_cpu = true
boinc_on_gpu = true
nuw_cpu_percentage = 40
boinc_gpu_percentage = 90
max_boinc_tasks = 4

[performance]
cpu_priority = "high"
gpu_priority = "high"
memory_limit_percentage = 95
thermal_throttle_enabled = false
power_management_enabled = false

[boinc.projects.milkyway]
enabled = true
gpu_enabled = true
cpu_cores = 6
memory_limit_mb = 6144
resource_percentage = 90
```

#### Balanced Template
```toml
[template]
name = "balanced"
description = "Balanced performance with system responsiveness"

[work_allocation]
nuw_on_cpu = true
boinc_on_gpu = true
nuw_cpu_percentage = 25
boinc_gpu_percentage = 75
max_boinc_tasks = 2

[performance]
cpu_priority = "normal"
gpu_priority = "normal"
memory_limit_percentage = 80
thermal_throttle_enabled = true
power_management_enabled = true

[boinc.projects.milkyway]
enabled = true
gpu_enabled = true
cpu_cores = 4
memory_limit_mb = 4096
resource_percentage = 75
```

#### Low Power Template
```toml
[template]
name = "low_power"
description = "Energy efficient operation with minimal resource usage"

[work_allocation]
nuw_on_cpu = false
boinc_on_gpu = true
nuw_cpu_percentage = 0
boinc_gpu_percentage = 50
max_boinc_tasks = 1

[performance]
cpu_priority = "low"
gpu_priority = "low"
memory_limit_percentage = 60
thermal_throttle_enabled = true
power_management_enabled = true
max_temperature_celsius = 70

[boinc.projects.milkyway]
enabled = true
gpu_enabled = true
cpu_cores = 2
memory_limit_mb = 2048
resource_percentage = 50
```

## User Experience Optimization

### Interactive Configuration Editor

The miner provides a user-friendly configuration editor:

#### TUI Configuration Interface
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Configuration Editor                                    [Save] [Reset] │
├─────────────────────────────────────────────────────────────────────────────┤
│ Oracle Settings                                               │
│ URL: [https://oracle.chert.network]                           │
│ Timeout: [30] seconds                                          │
│ HTTPS Required: [✓] Yes                                        │
│ Verify Certificates: [✓] Yes                                   │
│                                                               │
│ Work Allocation                                               │
│ NUW on CPU: [✓] Enabled                                      │
│ BOINC on GPU: [✓] Enabled                                     │
│ NUW CPU Percentage: [25%]                                     │
│ BOINC GPU Percentage: [75%]                                     │
│                                                               │
│ Performance Settings                                           │
│ CPU Priority: [Normal ▼]                                       │
│ GPU Priority: [High ▼]                                         │
│ Memory Limit: [80%]                                            │
│ Max Temperature: [85°C]                                         │
│                                                               │
│ [Tab] Next Field | [Enter] Save | [Esc] Cancel | [F1] Help       │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Configuration Validation Feedback
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Configuration Validation                                Status: ⚠ Warnings │
├─────────────────────────────────────────────────────────────────────────────┤
│ Validation Results                                            │
│ ✓ Oracle URL is valid and reachable                           │
│ ✓ HTTPS configuration is secure                                 │
│ ✓ Work allocation percentages are within limits                   │
│ ⚠ GPU temperature limit is high (85°C)                       │
│   Recommendation: Consider reducing to 80°C for hardware longevity │
│ ⚠ Memory usage is high (80%)                                 │
│   Recommendation: Consider reducing to 75% for system stability │
│                                                               │
│ [Enter] Continue | [F2] Fix Issues | [Esc] Cancel                │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Configuration Profiles

Users can create and manage multiple configuration profiles:

#### Profile Management
```bash
# Create new profile
chert-miner --create-profile gaming --template balanced

# Switch between profiles
chert-miner --profile gaming

# List available profiles
chert-miner --list-profiles

# Delete profile
chert-miner --delete-profile old_profile

# Export profile
chert-miner --export-profile gaming --output gaming_config.toml

# Import profile
chert-miner --import-profile gaming_config.toml --name imported_gaming
```

#### Profile Structure
```toml
[profile]
name = "gaming"
description = "Optimized for gaming while mining"
created_at = "2025-01-15T10:30:00Z"
last_used = "2025-01-20T15:45:00Z"

[profile.inherits_from]
template = "balanced"
overrides = ["performance", "work_allocation"]

[profile.custom_settings]
# Custom overrides for this profile
performance.cpu_priority = "low"
performance.memory_limit_percentage = 60
work_allocation.nuw_cpu_percentage = 10
```

## Troubleshooting Setup Issues

### Common Setup Problems

#### Permission Issues
**Symptoms**: Cannot create directories, write config files
**Solutions**:
```bash
# Fix directory permissions
chmod 755 ~/.chert
chmod 644 ~/.chert/config/*

# Run with correct user
sudo -u $USER chert-miner

# Use alternative data directory
export CHERT_DATA_DIR=/tmp/chert_data
mkdir -p /tmp/chert_data
```

#### Network Configuration
**Symptoms**: Cannot connect to oracle, BOINC projects
**Solutions**:
```bash
# Test network connectivity
ping oracle.chert.network
curl -I https://oracle.chert.network

# Configure proxy
export CHERT_HTTP_PROXY=http://proxy.example.com:8080
export CHERT_HTTPS_PROXY=https://proxy.example.com:8080

# Configure firewall
sudo ufw allow out 443
sudo ufw allow out 80
```

#### Hardware Detection Issues
**Symptoms**: Incorrect hardware detection, poor performance
**Solutions**:
```bash
# Force hardware detection
chert-miner --detect-hardware --verbose

# Override hardware detection
export CHERT_FORCE_CPU_CORES=8
export CHERT_FORCE_GPU_MEMORY=10737418240

# Use manual configuration
chert-miner --config manual_config.toml
```

### Debug Setup Process

Enable comprehensive setup debugging:

```bash
# Setup debugging
CHERT_DEBUG_SETUP=true
CHERT_DEBUG_HARDWARE_DETECTION=true
CHERT_DEBUG_CONFIG_VALIDATION=true
CHERT_DEBUG_AUTO_CONFIGURATION=true

# Run setup with debug output
chert-miner --setup --debug --verbose
```

## Best Practices

### Initial Setup
1. **Use Setup Wizard**: Let wizard guide initial configuration
2. **Validate Hardware**: Ensure hardware detection is accurate
3. **Test Configuration**: Run with test work before full deployment
4. **Monitor Performance**: Watch initial performance metrics
5. **Adjust Settings**: Fine-tune based on observed performance

### Configuration Management
1. **Use Profiles**: Create profiles for different use cases
2. **Regular Backups**: Backup configuration files regularly
3. **Version Control**: Track configuration changes
4. **Documentation**: Document custom configuration choices
5. **Regular Updates**: Keep configuration updated with new features

### Performance Optimization
1. **Start Conservative**: Begin with conservative resource allocation
2. **Monitor System**: Watch system resource usage
3. **Gradual Increases**: Slowly increase resource allocation
4. **Temperature Monitoring**: Monitor hardware temperatures
5. **Power Efficiency**: Balance performance with power consumption

## Future Enhancements

### Advanced Setup Features
1. **AI Configuration**: Machine learning-based configuration optimization
2. **Cloud Profiles**: Synchronize configurations across devices
3. **Community Templates**: Share and download community configurations
4. **Dynamic Optimization**: Real-time configuration adjustment
5. **Predictive Setup**: Anticipate optimal settings based on usage patterns

### Enhanced User Experience
1. **Web Interface**: Browser-based configuration management
2. **Mobile App**: Mobile configuration and monitoring
3. **Voice Configuration**: Voice-activated setup and control
4. **AR/VR Setup**: Immersive configuration experience
5. **Automated Tuning**: Self-optimizing configuration system

The self-setup and configuration system provides a comprehensive, user-friendly experience that makes the Chert miner accessible to users of all technical levels while maintaining the flexibility needed for advanced optimization.
choco