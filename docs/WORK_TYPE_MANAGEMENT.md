# Work Type Management Documentation

## Overview

The Chert miner supports flexible work allocation between different computational work types, allowing users to optimize their mining setup based on hardware capabilities and preferences. The system manages three primary work types:

- **NUW (Non-Useful Work)** - Traditional proof-of-work mining
- **BOINC CPU** - Scientific computing tasks using CPU resources
- **BOINC GPU** - Scientific computing tasks using GPU resources

## Work Allocation Configuration

### Configuration Structure

Work allocation is managed through the `WorkAllocationConfig` structure in [`config.rs`](../src/config.rs:41):

```rust
pub struct WorkAllocationConfig {
    /// Enable NUW mining on CPU
    pub nuw_on_cpu: bool,
    /// Enable BOINC processing on GPU
    pub boinc_on_gpu: bool,
    /// Percentage of CPU cores to allocate to NUW (0-100)
    pub nuw_cpu_percentage: u8,
    /// Percentage of GPU resources to allocate to BOINC (0-100)
    pub boinc_gpu_percentage: u8,
    /// Enable on-demand NUW tasks (user can choose when to run)
    pub nuw_on_demand: bool,
    /// Minimum NUW difficulty threshold to accept
    pub min_nuw_difficulty: u32,
    /// Maximum concurrent BOINC tasks
    pub max_boinc_tasks: u8,
}
```

### Environment Variables

Work allocation can be configured via environment variables:

| Variable | Description | Default | Range |
|----------|-------------|----------|--------|
| `CHERT_NUW_ON_CPU` | Enable NUW mining on CPU | `false` | `true`/`false` |
| `CHERT_BOINC_ON_GPU` | Enable BOINC processing on GPU | `true` | `true`/`false` |
| `CHERT_NUW_CPU_PERCENTAGE` | CPU cores for NUW (0-100) | `25` | `0-100` |
| `CHERT_BOINC_GPU_PERCENTAGE` | GPU resources for BOINC (0-100) | `80` | `0-100` |
| `CHERT_NUW_ON_DEMAND` | NUW tasks on-demand only | `true` | `true`/`false` |
| `CHERT_MIN_NUW_DIFFICULTY` | Minimum NUW difficulty threshold | `1000` | `1+` |
| `CHERT_MAX_BOINC_TASKS` | Maximum concurrent BOINC tasks | `2` | `1-10` |

## Work Type Scenarios

### Scenario 1: NUW CPU + BOINC GPU (Recommended Default)

**Configuration:**
```bash
CHERT_NUW_ON_CPU=true
CHERT_BOINC_ON_GPU=true
CHERT_NUW_CPU_PERCENTAGE=25
CHERT_BOINC_GPU_PERCENTAGE=80
```

**Behavior:**
- NUW mining uses 25% of CPU cores
- BOINC tasks use 80% of GPU resources
- Remaining CPU (75%) available for BOINC CPU tasks if needed
- Optimal for systems with dedicated GPUs

### Scenario 2: BOINC Only (CPU + GPU)

**Configuration:**
```bash
CHERT_NUW_ON_CPU=false
CHERT_BOINC_ON_GPU=true
CHERT_NUW_CPU_PERCENTAGE=0
CHERT_BOINC_GPU_PERCENTAGE=100
```

**Behavior:**
- All computational resources dedicated to BOINC projects
- Maximum scientific computing contribution
- No traditional mining activities

### Scenario 3: NUW Only

**Configuration:**
```bash
CHERT_NUW_ON_CPU=true
CHERT_BOINC_ON_GPU=false
CHERT_NUW_CPU_PERCENTAGE=100
CHERT_BOINC_GPU_PERCENTAGE=0
```

**Behavior:**
- All CPU resources dedicated to NUW mining
- Traditional proof-of-work mining only
- No scientific computing tasks

### Scenario 4: BOINC + NUW GPU (NUW Limits)

**Configuration:**
```bash
CHERT_NUW_ON_CPU=false
CHERT_BOINC_ON_GPU=true
CHERT_NUW_CPU_PERCENTAGE=0
CHERT_BOINC_GPU_PERCENTAGE=60
```

**Behavior:**
- NUW mining limited to GPU resources (if supported)
- BOINC uses 60% of GPU, leaving 40% for NUW
- CPU fully available for system operations

## Resource Management

### CPU Allocation

The miner enforces CPU resource allocation through:

1. **Core Assignment**: Specific CPU cores are assigned to NUW vs BOINC
2. **Priority Management**: Process priorities ensure fair resource sharing
3. **Load Balancing**: Dynamic adjustment based on system load

### GPU Allocation

GPU resources are managed through:

1. **Memory Partitioning**: GPU memory divided between work types
2. **Compute Units**: CUDA/OpenCL units allocated proportionally
3. **Thermal Management**: Temperature-based throttling to prevent overheating

### Validation Rules

The configuration system enforces these rules:

1. **Total Resource Limit**: CPU + GPU allocation cannot exceed 100%
2. **Minimum Resources**: Each work type requires minimum resources to function
3. **Hardware Compatibility**: Work types only enabled if hardware supports them
4. **Conflict Prevention**: Mutually exclusive configurations are rejected

## Dynamic Work Switching

### Automatic Switching

The miner can automatically switch between work types based on:

1. **Profitability**: Real-time profitability calculations
2. **System Load**: Adaptive to system resource availability
3. **Task Completion**: Switch when current work type completes
4. **User Preferences**: Respect user-defined priorities

### Manual Override

Users can manually control work type switching through:

1. **TUI Interface**: Interactive controls in the terminal UI
2. **Command Line**: Runtime commands to change work types
3. **API Endpoints**: RESTful interface for remote management
4. **Configuration Files**: Persistent preference settings

## Performance Optimization

### Work Type Prioritization

The system prioritizes work types based on:

1. **User Preferences**: Explicit user-defined priorities
2. **Hardware Efficiency**: Work types that run efficiently on available hardware
3. **Network Conditions**: Adapt to latency and bandwidth constraints
4. **Reward Multipliers**: Prefer work types with higher rewards

### Resource Monitoring

Continuous monitoring ensures optimal resource utilization:

1. **Real-time Metrics**: CPU, GPU, memory, and disk usage
2. **Performance Tracking**: FLOPS, hash rates, and task completion rates
3. **Efficiency Calculations**: Resource utilization vs. theoretical maximums
4. **Alert System**: Notifications for resource conflicts or inefficiencies

## Troubleshooting

### Common Issues

1. **Resource Conflicts**: CPU/GPU allocation exceeds 100%
   - **Solution**: Adjust percentage settings to stay within limits
   - **Check**: `CHERT_NUW_CPU_PERCENTAGE + CHERT_BOINC_GPU_PERCENTAGE <= 100`

2. **Poor Performance**: Work type not utilizing expected resources
   - **Solution**: Verify hardware compatibility and driver installation
   - **Check**: GPU drivers, CUDA/OpenCL runtime, CPU affinity settings

3. **Task Starvation**: One work type preventing others from running
   - **Solution**: Adjust resource allocation percentages
   - **Check**: `max_boinc_tasks` and `nuw_on_demand` settings

### Debug Information

Enable debug logging to troubleshoot work type issues:

```bash
CHERT_DEBUG_MODE=true
CHERT_VERBOSE_LOGGING=true
```

This provides detailed information about:
- Resource allocation decisions
- Work type switching events
- Performance metrics
- Error conditions and recovery actions

## Best Practices

1. **Start Conservative**: Begin with lower resource allocations and increase gradually
2. **Monitor Performance**: Use the TUI to watch real-time performance metrics
3. **Adjust Based on Hardware**: Optimize for your specific hardware configuration
4. **Consider Power Costs**: Balance profitability against electricity consumption
5. **Regular Updates**: Keep BOINC clients and drivers updated for optimal performance

## Future Enhancements

Planned improvements to work type management:

1. **Machine Learning**: AI-driven work type selection based on historical performance
2. **Advanced Scheduling**: Time-based and priority-based work scheduling
3. **Cross-Platform**: Enhanced support for different operating systems
4. **Cloud Integration**: Support for cloud-based GPU resources
5. **Multi-Miner**: Coordination across multiple mining instances