# BOINC Project Selection and Management Documentation

## Overview

The Chert miner integrates with BOINC (Berkeley Open Infrastructure for Network Computing) to provide useful scientific computing work. The system supports multiple BOINC projects with automatic project selection, manual project management, and intelligent work distribution.

## Supported BOINC Projects

### Currently Supported Projects

1. **MilkyWay@Home** (Primary)
   - **Type**: N-body simulation for galactic structure
   - **Resources**: CPU-intensive, GPU-accelerated variants available
   - **Typical Work Unit Size**: 50-200 MB
   - **Estimated Runtime**: 2-8 hours per work unit
   - **Scientific Impact**: Understanding galaxy formation and dark matter distribution

2. **Rosetta@Home** (Planned)
   - **Type**: Protein folding and disease research
   - **Resources**: CPU and GPU intensive
   - **Typical Work Unit Size**: 100-500 MB
   - **Estimated Runtime**: 4-12 hours per work unit
   - **Scientific Impact**: Drug discovery and disease understanding

### Project Integration Architecture

```
Miner → Oracle → BOINC Project Servers
   ↓        ↓           ↓
Work Request → Project Selection → Work Unit Distribution
   ↓        ↓           ↓
Task Execution → Progress Monitoring → Result Collection
   ↓        ↓           ↓
Result Submission → Validation → Scientific Contribution
```

## Project Configuration

### Automatic Project Selection

The miner can automatically select projects based on:

1. **Hardware Capabilities**: Choose projects optimized for available hardware
2. **Network Conditions**: Adapt to bandwidth and latency constraints
3. **Scientific Preferences**: User-defined research area priorities
4. **Reward Multipliers**: Prefer projects with higher Chert rewards

### Manual Project Management

Users can manually control project participation through:

1. **TUI Interface**: Interactive project selection in terminal UI
2. **Configuration Files**: Persistent project preferences
3. **Command Line Options**: Runtime project control
4. **API Endpoints**: RESTful interface for remote management

## Project Setup and Management

### Initial Project Configuration

#### Environment Variables

```bash
# Primary project selection
CHERT_BOINC_PRIMARY_PROJECT=MilkyWay@Home
CHERT_BOINC_SECONDARY_PROJECTS=Rosetta@Home,Einstein@Home

# Project-specific settings
CHERT_BOINC_MILKYWAY_GPU_ENABLED=true
CHERT_BOINC_MILKYWAY_CPU_CORES=4
CHERT_BOINC_MILKYWAY_MEMORY_LIMIT=2048

# Project switching preferences
CHERT_BOINC_AUTO_SWITCH=true
CHERT_BOINC_SWITCH_INTERVAL=3600  # seconds
CHERT_BOINC_MIN_RUN_TIME=1800      # seconds
```

#### Configuration File Format

```toml
[boinc.projects]
primary = "MilkyWay@Home"
secondary = ["Rosetta@Home", "Einstein@Home"]
auto_switch = true
switch_interval = 3600
min_run_time = 1800

[boinc.projects.milkyway]
enabled = true
gpu_enabled = true
cpu_cores = 4
memory_limit = 2048  # MB
priority = 1

[boinc.projects.rosetta]
enabled = false
gpu_enabled = true
cpu_cores = 2
memory_limit = 4096  # MB
priority = 2
```

### Project Attachment Process

The miner handles BOINC project attachment through:

1. **Oracle Proxy**: Projects are attached through the Chert oracle proxy
2. **Authentication**: Secure authenticator management for each project
3. **Resource Allocation**: Per-project resource limits and preferences
4. **Validation**: Project compatibility and hardware requirement checks

#### Attachment Workflow

```rust
// Simplified attachment process
async fn attach_to_project(
    project_url: &str,
    authenticator: &str,
    resource_limits: ResourceLimits
) -> Result<()> {
    // 1. Validate project compatibility
    validate_project_requirements(project_url)?;
    
    // 2. Configure resource allocation
    configure_project_resources(resource_limits)?;
    
    // 3. Establish secure connection
    let client = create_boinc_client(project_url)?;
    
    // 4. Attach to project
    client.attach_project(project_url, authenticator).await?;
    
    // 5. Configure project-specific settings
    configure_project_settings(client, project_url).await?;
    
    Ok(())
}
```

## Work Unit Management

### Work Unit Fetching

The miner implements intelligent work unit fetching:

1. **Prefetching**: Download multiple work units in advance
2. **Size Optimization**: Select work units based on available bandwidth
3. **Deadline Awareness**: Consider work unit deadlines when fetching
4. **Load Balancing**: Distribute work across multiple projects

### Work Unit Processing

#### CPU Work Units

- **Core Assignment**: Specific CPU cores assigned to each work unit
- **Priority Management**: Real-time priority adjustment based on progress
- **Memory Management**: Efficient memory usage for large datasets
- **Checkpointing**: Regular progress saving for long-running tasks

#### GPU Work Units

- **Memory Partitioning**: GPU memory divided between concurrent tasks
- **Compute Unit Allocation**: CUDA/OpenCL units managed efficiently
- **Thermal Management**: Temperature-based throttling to prevent overheating
- **Driver Compatibility**: Support for multiple GPU driver versions

### Work Unit Validation

Before processing, each work unit is validated:

1. **Cryptographic Verification**: Ensure work unit integrity
2. **Resource Requirements**: Verify sufficient resources available
3. **Deadline Check**: Confirm sufficient time for completion
4. **Compatibility**: Validate hardware/software compatibility

## Project Switching

### Automatic Switching Criteria

The miner can automatically switch between projects based on:

1. **Work Unit Availability**: Switch when current project has no work
2. **Performance Metrics**: Switch to more efficient projects
3. **Reward Changes**: Adapt to changing reward multipliers
4. **User Preferences**: Respect user-defined project priorities

### Switching Process

```rust
async fn switch_projects(
    from_project: &str,
    to_project: &str
) -> Result<()> {
    // 1. Complete current work units
    complete_active_work_units(from_project).await?;
    
    // 2. Update project configuration
    update_project_config(to_project).await?;
    
    // 3. Fetch new work units
    fetch_work_units(to_project).await?;
    
    // 4. Start processing new work units
    start_work_processing(to_project).await?;
    
    // 5. Log switching event
    log_project_switch(from_project, to_project).await?;
    
    Ok(())
}
```

### Manual Project Control

Users can manually control project participation:

#### TUI Controls

- **Project Selection Tab**: Browse and select available projects
- **Resource Allocation**: Adjust per-project resource limits
- **Priority Settings**: Set project execution priorities
- **Real-time Switching**: Immediate project switching capability

#### Command Line Interface

```bash
# List available projects
chert-miner --list-projects

# Switch to specific project
chert-miner --switch-project Rosetta@Home

# Set project priority
chert-miner --set-priority MilkyWay@Home:1,Rosetta@Home:2

# Configure project resources
chert-miner --configure-project MilkyWay@Home --cpu-cores 4 --memory 2048
```

## Performance Monitoring

### Project-Specific Metrics

The miner tracks detailed metrics for each project:

1. **Work Unit Completion**: Rate and success statistics
2. **Resource Efficiency**: CPU/GPU utilization per project
3. **Scientific Contribution**: Credits earned and work validated
4. **Performance Trends**: Historical performance data

### Real-time Monitoring

Through the TUI, users can monitor:

1. **Active Work Units**: Current progress and estimated completion
2. **Resource Usage**: Per-project CPU/GPU/memory utilization
3. **Network Activity**: Data transfer rates and project communication
4. **Error Rates**: Failed work units and error categorization

## Troubleshooting

### Common Project Issues

#### Attachment Failures

**Symptoms**: Unable to attach to BOINC projects
**Causes**: Network issues, invalid authenticators, server problems
**Solutions**:
```bash
# Check network connectivity
ping boincproject.local.com

# Verify authenticator
chert-miner --verify-authenticator MilkyWay@Home

# Reset project configuration
chert-miner --reset-project MilkyWay@Home
```

#### Work Unit Failures

**Symptoms**: High failure rate for specific projects
**Causes**: Insufficient resources, hardware incompatibility, software bugs
**Solutions**:
```bash
# Check resource allocation
chert-miner --check-resources --project MilkyWay@Home

# Reduce concurrent tasks
export CHERT_MAX_BOINC_TASKS=1

# Enable debug logging
export CHERT_DEBUG_MODE=true
export CHERT_VERBOSE_LOGGING=true
```

#### Performance Issues

**Symptoms**: Slow work unit processing or low efficiency
**Causes**: Resource contention, hardware limitations, configuration issues
**Solutions**:
```bash
# Optimize resource allocation
chert-miner --optimize-resources

# Check hardware compatibility
chert-miner --check-compatibility --project MilkyWay@Home

# Update BOINC client
chert-miner --update-boinc
```

### Debug Information

Enable comprehensive debugging for project issues:

```bash
# Enable project-specific debugging
CHERT_DEBUG_BOINC=true
CHERT_DEBUG_PROJECT_SELECTION=true
CHERT_DEBUG_WORK_UNITS=true

# Enable performance monitoring
CHERT_PERFORMANCE_MONITORING=true
CHERT_RESOURCE_TRACKING=true
```

## Best Practices

### Project Selection

1. **Hardware Matching**: Choose projects optimized for your hardware
2. **Network Considerations**: Consider bandwidth requirements and availability
3. **Scientific Interest**: Support projects aligned with your interests
4. **Reward Optimization**: Balance scientific contribution with rewards

### Resource Management

1. **Conservative Allocation**: Start with lower resource limits
2. **Monitor Performance**: Use TUI to watch real-time metrics
3. **Adjust Gradually**: Make incremental changes to resource allocation
4. **Thermal Management**: Monitor temperatures and adjust accordingly

### Reliability

1. **Multiple Projects**: Participate in multiple projects for redundancy
2. **Regular Updates**: Keep BOINC clients and drivers updated
3. **Backup Configuration**: Maintain backup of project settings
4. **Monitoring**: Set up alerts for project issues

## Future Enhancements

### Planned Features

1. **AI Project Selection**: Machine learning for optimal project matching
2. **Dynamic Resource Allocation**: Real-time resource optimization
3. **Cross-Project Coordination**: Collaborative computing across projects
4. **Mobile Support**: Project management from mobile devices
5. **Advanced Analytics**: Detailed performance analysis and recommendations

### Project Expansion

Planned support for additional BOINC projects:

1. **Einstein@Home**: Gravitational wave detection
2. **SETI@Home**: Extraterrestrial signal detection
3. **World Community Grid**: Medical and humanitarian research
4. **Climateprediction.net**: Climate change modeling
5. **GPUGRID**: GPU-accelerated scientific computing

## API Reference

### Project Management API

```rust
// List available projects
pub async fn list_available_projects() -> Result<Vec<ProjectInfo>>;

// Get project details
pub async fn get_project_info(project_name: &str) -> Result<ProjectInfo>;

// Attach to project
pub async fn attach_to_project(
    project_name: &str,
    authenticator: &str,
    config: ProjectConfig
) -> Result<()>;

// Switch active project
pub async fn switch_project(project_name: &str) -> Result<()>;

// Configure project resources
pub async fn configure_project_resources(
    project_name: &str,
    resources: ResourceConfig
) -> Result<()>;
```

### Work Unit Management API

```rust
// Fetch work units
pub async fn fetch_work_units(
    project_name: &str,
    count: usize
) -> Result<Vec<WorkUnit>>;

// Submit work unit results
pub async fn submit_work_unit_result(
    project_name: &str,
    work_unit_id: &str,
    result: WorkUnitResult
) -> Result<SubmissionReceipt>;

// Get work unit status
pub async fn get_work_unit_status(
    project_name: &str,
    work_unit_id: &str
) -> Result<WorkUnitStatus>;
```

This comprehensive BOINC project management system provides flexible, efficient, and user-friendly control over scientific computing participation while maximizing both scientific contribution and mining rewards.