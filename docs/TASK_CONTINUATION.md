# Task Continuation Options Documentation

## Overview

The Chert miner provides comprehensive task continuation options that ensure work resilience and progress preservation across interruptions, system restarts, and various failure scenarios. This system guarantees that computational work is not lost and can continue from the most recent valid state.

## Continuation Scenarios

### Supported Interruption Types

1. **Graceful Shutdown**: User-initiated shutdown with proper cleanup
2. **System Crash**: Unexpected process termination
3. **Power Loss**: System power interruption
4. **Network Interruption**: Temporary loss of network connectivity
5. **Resource Exhaustion**: Out of memory or disk space
6. **Process Restart**: Service restart or system reboot
7. **Application Update**: Miner software updates during operation

### Work Type Continuation

#### NUW (Non-Useful Work) Continuation
- **Checkpoint System**: Regular hash state checkpoints
- **Nonce Range Tracking**: Current nonce search progress
- **Difficulty Adaptation**: Preserve difficulty adjustment state
- **Share Submission**: Partial work submission capability

#### BOINC Task Continuation
- **BOINC Checkpointing**: Native BOINC checkpoint support
- **Progress Preservation**: Task progress and intermediate results
- **Scientific State**: Computational state preservation
- **Result Recovery**: Ability to recover partial results

## Checkpointing System

### Checkpoint Architecture

```
Work Start → Checkpoint 1 → Checkpoint 2 → ... → Checkpoint N → Completion
     ↓            ↓              ↓                    ↓              ↓
  State Save    State Save      State Save            State Save    Final Result
     ↓            ↓              ↓                    ↓              ↓
  Local Disk   Local Disk     Local Disk           Local Disk   Oracle Submit
     ↓            ↓              ↓                    ↓              ↓
  Recovery      Recovery       Recovery             Recovery      Validation
```

### Checkpoint Data Structures

#### NUW Checkpoint
```rust
pub struct NuwCheckpoint {
    /// Unique checkpoint identifier
    pub checkpoint_id: String,
    /// Work unit identifier
    pub work_unit_id: String,
    /// Current nonce range
    pub nonce_range: NonceRange,
    /// Best hash found so far
    pub best_hash: String,
    /// Current difficulty target
    pub difficulty_target: String,
    /// Checkpoint timestamp
    pub created_at: DateTime<Utc>,
    /// Computational work done
    pub work_done: u64,
    /// Estimated remaining work
    pub estimated_remaining: u64,
    /// Performance metrics
    pub performance_metrics: NuwPerformanceMetrics,
}
```

#### BOINC Checkpoint
```rust
pub struct BoincCheckpoint {
    /// Unique checkpoint identifier
    pub checkpoint_id: String,
    /// BOINC work unit identifier
    pub work_unit_id: String,
    /// Task progress percentage
    pub progress_fraction: f64,
    /// CPU time accumulated
    pub cpu_time: f64,
    /// Elapsed wall-clock time
    pub elapsed_time: f64,
    /// Intermediate results
    pub intermediate_results: Vec<IntermediateResult>,
    /// Scientific computation state
    pub computation_state: ComputationState,
    /// Checkpoint timestamp
    pub created_at: DateTime<Utc>,
    /// Memory usage snapshot
    pub memory_snapshot: MemorySnapshot,
}
```

### Checkpoint Management

#### Checkpoint Creation
```rust
async fn create_checkpoint(
    work_context: &WorkContext,
    checkpoint_type: CheckpointType
) -> Result<Checkpoint> {
    // 1. Gather current state
    let current_state = gather_work_state(work_context).await?;
    
    // 2. Validate state consistency
    validate_checkpoint_state(&current_state)?;
    
    // 3. Serialize checkpoint data
    let checkpoint_data = serialize_checkpoint(&current_state)?;
    
    // 4. Compress checkpoint data
    let compressed_data = compress_data(&checkpoint_data)?;
    
    // 5. Write to persistent storage
    let checkpoint_path = get_checkpoint_path(work_context, checkpoint_type);
    write_checkpoint_file(&checkpoint_path, &compressed_data).await?;
    
    // 6. Update checkpoint index
    update_checkpoint_index(&checkpoint_path, &current_state).await?;
    
    // 7. Cleanup old checkpoints
    cleanup_old_checkpoints(work_context, checkpoint_type).await?;
    
    Ok(Checkpoint::from_state(current_state))
}
```

#### Checkpoint Recovery
```rust
async fn recover_from_checkpoint(
    work_unit_id: &str,
    checkpoint_type: CheckpointType
) -> Result<Option<WorkContext>> {
    // 1. Find latest checkpoint
    let checkpoint_path = find_latest_checkpoint(work_unit_id, checkpoint_type)?;
    
    if checkpoint_path.is_none() {
        return Ok(None); // No checkpoint available
    }
    
    // 2. Read checkpoint data
    let checkpoint_data = read_checkpoint_file(&checkpoint_path.unwrap()).await?;
    
    // 3. Decompress checkpoint data
    let decompressed_data = decompress_data(&checkpoint_data)?;
    
    // 4. Deserialize checkpoint state
    let checkpoint_state = deserialize_checkpoint(&decompressed_data)?;
    
    // 5. Validate checkpoint integrity
    validate_checkpoint_integrity(&checkpoint_state)?;
    
    // 6. Reconstruct work context
    let work_context = reconstruct_work_context(checkpoint_state).await?;
    
    // 7. Verify work unit still valid
    if !is_work_unit_valid(&work_context).await? {
        return Err(anyhow::anyhow!("Work unit no longer valid"));
    }
    
    Ok(Some(work_context))
}
```

## Continuation Strategies

### Automatic Continuation

The miner automatically attempts continuation on restart:

```rust
async fn auto_continue_work() -> Result<()> {
    // 1. Scan for interrupted work
    let interrupted_work = scan_for_interrupted_work().await?;
    
    for work_unit in interrupted_work {
        match work_unit.work_type {
            WorkType::NUW => {
                if let Some(context) = recover_nuw_checkpoint(&work_unit.id).await? {
                    info!("Continuing NUW work from checkpoint: {}", work_unit.id);
                    resume_nuw_work(context).await?;
                }
            }
            WorkType::BOINC => {
                if let Some(context) = recover_boinc_checkpoint(&work_unit.id).await? {
                    info!("Continuing BOINC work from checkpoint: {}", work_unit.id);
                    resume_boinc_work(context).await?;
                }
            }
        }
    }
    
    Ok(())
}
```

### Manual Continuation

Users can manually control work continuation:

#### Command Line Interface
```bash
# List available checkpoints
chert-miner --list-checkpoints

# Continue from specific checkpoint
chert-miner --continue-from-checkpoint cp_001234

# Continue specific work unit
chert-miner --continue-work-unit wu_001234

# Reset and start fresh
chert-miner --reset-work-unit wu_001234 --no-continue

# Export checkpoint data
chert-miner --export-checkpoint cp_001234 --output checkpoint.json
```

#### TUI Interface
The TUI provides continuation controls:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Task Continuation Options                              Status: ● OK │
├─────────────────────────────────────────────────────────────────────────────┤
│ Available Checkpoints                                        │
│ ID: cp_001234    Type: BOINC    Work: mw_001234_001    │
│ Progress: 67.3%    Time: 4h 23m    Size: 125MB        │
│                                                             │
│ ID: cp_001235    Type: NUW      Work: pow_001235        │
│ Nonce Range: 1000000-2000000    Best Hash: 0000abc...    │
│                                                             │
│ Actions                                                      │
│ [F2] Continue Selected    [F3] Delete Checkpoint    [F4] Export   │
│ [F5] Reset Work Unit     [F6] View Details      [F7] Refresh   │
├─────────────────────────────────────────────────────────────────────────────┤
│ Enter: Continue | Esc: Cancel | F1: Help                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

## State Persistence

### Storage Architecture

The miner uses a multi-layered storage approach:

```
Application State → Checkpoint Data → Persistent Storage → Recovery
       ↓               ↓                    ↓              ↓
   Memory Cache    Local Files         Disk/SSD        Work Resume
       ↓               ↓                    ↓              ↓
   Fast Access    Structured Format    Reliable Storage   Continuation
```

### Storage Locations

#### Primary Storage
- **Checkpoints Directory**: `~/.chert/checkpoints/`
- **Work State Directory**: `~/.chert/work_state/`
- **Configuration Directory**: `~/.chert/config/`
- **Logs Directory**: `~/.chert/logs/`

#### Backup Storage
- **Remote Backup**: Configurable remote storage location
- **Cloud Storage**: Optional cloud backup integration
- **Network Storage**: Distributed storage options

### Data Formats

#### Checkpoint File Format
```json
{
  "checkpoint_id": "cp_001234",
  "work_unit_id": "mw_001234_001",
  "work_type": "BOINC",
  "created_at": "2025-01-15T10:30:00Z",
  "version": "1.0",
  "data": {
    "progress_fraction": 0.673,
    "cpu_time": 15840.5,
    "elapsed_time": 16245.2,
    "intermediate_results": [...],
    "computation_state": {...},
    "memory_snapshot": {...}
  },
  "metadata": {
    "miner_version": "1.2.3",
    "platform": "linux-x86_64",
    "compression": "gzip",
    "encryption": "aes256"
  }
}
```

#### Index File Format
```json
{
  "work_units": [
    {
      "work_unit_id": "mw_001234_001",
      "work_type": "BOINC",
      "status": "interrupted",
      "latest_checkpoint": "cp_001234",
      "checkpoints": ["cp_001230", "cp_001231", "cp_001234"],
      "created_at": "2025-01-15T08:00:00Z",
      "last_activity": "2025-01-15T10:30:00Z"
    }
  ],
  "last_cleanup": "2025-01-15T06:00:00Z"
}
```

## Recovery Mechanisms

### Automatic Recovery

#### Startup Recovery Process
```rust
async fn startup_recovery() -> Result<()> {
    // 1. Check for interrupted work
    let interrupted_work = detect_interrupted_work().await?;
    
    if interrupted_work.is_empty() {
        info!("No interrupted work found, starting fresh");
        return Ok(());
    }
    
    // 2. Validate work units still valid
    let valid_work = validate_work_units(&interrupted_work).await?;
    
    // 3. Recover from latest checkpoints
    for work_unit in valid_work {
        match recover_work_unit(&work_unit).await {
            Ok(context) => {
                info!("Successfully recovered work unit: {}", work_unit.id);
                queue_work_for_resumption(context).await?;
            }
            Err(e) => {
                warn!("Failed to recover work unit {}: {}", work_unit.id, e);
                cleanup_work_unit_data(&work_unit.id).await?;
            }
        }
    }
    
    // 4. Cleanup invalid work data
    cleanup_invalid_work_data(&interrupted_work, &valid_work).await?;
    
    Ok(())
}
```

### Manual Recovery

Users can manually trigger recovery:

#### Recovery Commands
```bash
# Scan for recoverable work
chert-miner --scan-recoverable

# Recover specific work unit
chert-miner --recover-work-unit wu_001234

# Recover all interrupted work
chert-miner --recover-all

# Validate checkpoint integrity
chert-miner --validate-checkpoints

# Repair corrupted checkpoints
chert-miner --repair-checkpoints
```

#### Recovery Validation
```rust
async fn validate_recovery() -> Result<ValidationReport> {
    let mut report = ValidationReport::new();
    
    // 1. Check checkpoint integrity
    let checkpoints = list_all_checkpoints().await?;
    for checkpoint in checkpoints {
        match validate_checkpoint_integrity(&checkpoint).await {
            Ok(()) => {
                report.add_valid_checkpoint(checkpoint.id);
            }
            Err(e) => {
                report.add_invalid_checkpoint(checkpoint.id, e);
            }
        }
    }
    
    // 2. Check work unit validity
    let work_units = list_interrupted_work_units().await?;
    for work_unit in work_units {
        match validate_work_unit_status(&work_unit).await {
            Ok(()) => {
                report.add_valid_work_unit(work_unit.id);
            }
            Err(e) => {
                report.add_invalid_work_unit(work_unit.id, e);
            }
        }
    }
    
    // 3. Check storage consistency
    validate_storage_consistency().await?;
    
    Ok(report)
}
```

## Configuration Options

### Continuation Settings

#### Environment Variables
```bash
# Checkpoint configuration
CHERT_CHECKPOINT_INTERVAL=300        # seconds between checkpoints
CHERT_MAX_CHECKPOINTS=10             # maximum checkpoints per work unit
CHERT_CHECKPOINT_COMPRESSION=gzip   # compression algorithm
CHERT_CHECKPOINT_ENCRYPTION=aes256   # encryption algorithm

# Recovery configuration
CHERT_AUTO_RECOVERY=true           # enable automatic recovery
CHERT_RECOVERY_TIMEOUT=60          # seconds to wait for recovery
CHERT_VALIDATE_CHECKPOINTS=true      # validate checkpoints on load

# Storage configuration
CHERT_CHECKPOINT_DIR=~/.chert/checkpoints
CHERT_BACKUP_ENABLED=true          # enable checkpoint backup
CHERT_REMOTE_BACKUP_URL=https://backup.example.com
```

#### Configuration File Format
```toml
[continuation]
checkpoint_interval = 300
max_checkpoints = 10
compression = "gzip"
encryption = "aes256"
auto_recovery = true
recovery_timeout = 60
validate_checkpoints = true

[continuation.storage]
checkpoint_dir = "~/.chert/checkpoints"
backup_enabled = true
remote_backup_url = "https://backup.example.com"
backup_interval = 3600

[continuation.cleanup]
auto_cleanup = true
cleanup_interval = 86400
keep_checkpoints = 5
cleanup_failed_work = true
```

## Performance Optimization

### Efficient Checkpointing

#### Checkpoint Frequency Optimization
```rust
fn calculate_optimal_checkpoint_interval(
    work_type: WorkType,
    work_complexity: f64,
    system_performance: &PerformanceMetrics
) -> Duration {
    match work_type {
        WorkType::NUW => {
            // NUW: Checkpoint based on hash rate
            let hashes_per_second = system_performance.hash_rate;
            let optimal_interval = (1000000.0 / hashes_per_second) as u64;
            Duration::seconds(optimal_interval.max(60)) // Minimum 1 minute
        }
        WorkType::BOINC => {
            // BOINC: Checkpoint based on progress rate
            let progress_per_second = work_complexity * system_performance.efficiency;
            let optimal_interval = (0.05 / progress_per_second) as u64; // 5% progress
            Duration::seconds(optimal_interval.max(300)) // Minimum 5 minutes
        }
    }
}
```

#### Checkpoint Size Optimization
```rust
fn optimize_checkpoint_size(
    checkpoint_data: &CheckpointData,
    target_size: usize
) -> Result<Vec<u8>> {
    // 1. Compress checkpoint data
    let compressed = compress_data(checkpoint_data)?;
    
    // 2. If still too large, reduce precision
    if compressed.len() > target_size {
        let reduced_data = reduce_checkpoint_precision(checkpoint_data);
        return optimize_checkpoint_size(&reduced_data, target_size);
    }
    
    // 3. Encrypt if configured
    if should_encrypt() {
        return encrypt_data(&compressed);
    }
    
    Ok(compressed)
}
```

### Memory Management

#### Checkpoint Memory Usage
```rust
fn manage_checkpoint_memory(
    active_checkpoints: &mut HashMap<String, Checkpoint>
) -> Result<()> {
    // 1. Monitor memory usage
    let current_usage = calculate_memory_usage(active_checkpoints);
    
    // 2. Remove oldest checkpoints if memory pressure
    if current_usage > MAX_CHECKPOINT_MEMORY {
        let oldest = find_oldest_checkpoint(active_checkpoints);
        if let Some(oldest_id) = oldest {
            active_checkpoints.remove(&oldest_id);
            info!("Removed oldest checkpoint {} due to memory pressure", oldest_id);
        }
    }
    
    // 3. Compress in-memory checkpoints
    compress_in_memory_checkpoints(active_checkpoints)?;
    
    Ok(())
}
```

## Troubleshooting

### Common Continuation Issues

#### Checkpoint Corruption
**Symptoms**: Unable to load checkpoints, recovery failures
**Causes**: Disk corruption, incomplete writes, software bugs
**Solutions**:
```bash
# Validate checkpoint integrity
chert-miner --validate-checkpoints --verbose

# Repair corrupted checkpoints
chert-miner --repair-checkpoints --backup

# Reset checkpoint storage
chert-miner --reset-checkpoints --confirm

# Restore from backup
chert-miner --restore-from-backup --url https://backup.example.com
```

#### Recovery Failures
**Symptoms**: Work units not resuming, lost progress
**Causes**: Invalid work units, expired deadlines, configuration issues
**Solutions**:
```bash
# Check work unit validity
chert-miner --check-work-unit wu_001234

# Force recovery attempt
chert-miner --force-recovery --work-unit wu_001234

# Reset and restart
chert-miner --reset-work-unit wu_001234 --restart

# Debug recovery process
CHERT_DEBUG_RECOVERY=true
chert-miner --recover-all --verbose
```

#### Performance Issues
**Symptoms**: Slow checkpointing, excessive disk I/O
**Causes**: Too frequent checkpoints, large checkpoint sizes, slow storage
**Solutions**:
```bash
# Optimize checkpoint interval
export CHERT_CHECKPOINT_INTERVAL=600

# Enable checkpoint compression
export CHERT_CHECKPOINT_COMPRESSION=lz4

# Use faster storage
export CHERT_CHECKPOINT_DIR=/tmp/chert_checkpoints

# Monitor checkpoint performance
chert-miner --profile-checkpoints
```

### Debug Information

Enable comprehensive continuation debugging:

```bash
# Checkpoint debugging
CHERT_DEBUG_CHECKPOINTS=true
CHERT_DEBUG_CHECKPOINT_CREATION=true
CHERT_DEBUG_CHECKPOINT_LOADING=true

# Recovery debugging
CHERT_DEBUG_RECOVERY=true
CHERT_DEBUG_AUTO_RECOVERY=true
CHERT_DEBUG_WORK_UNIT_VALIDATION=true

# Storage debugging
CHERT_DEBUG_STORAGE_OPERATIONS=true
CHERT_DEBUG_BACKUP_OPERATIONS=true
```

## Best Practices

### Checkpoint Strategy
1. **Regular Intervals**: Set appropriate checkpointing frequency
2. **Size Management**: Keep checkpoint sizes reasonable
3. **Validation**: Regularly validate checkpoint integrity
4. **Backup Strategy**: Maintain backup copies of important checkpoints

### Recovery Planning
1. **Test Recovery**: Regularly test recovery procedures
2. **Documentation**: Maintain clear recovery documentation
3. **Monitoring**: Monitor recovery success rates
4. **Fallback Options**: Have multiple recovery strategies

### Performance Optimization
1. **Balanced Frequency**: Optimize checkpointing frequency vs. overhead
2. **Efficient Storage**: Use fast storage for active checkpoints
3. **Memory Management**: Monitor and optimize checkpoint memory usage
4. **Compression**: Use appropriate compression for checkpoint data

## Future Enhancements

### Advanced Continuation Features
1. **Distributed Checkpoints**: Store checkpoints across multiple locations
2. **Incremental Checkpoints**: Store only changes since last checkpoint
3. **Predictive Checkpointing**: AI-driven optimal checkpoint timing
4. **Cross-Platform Continuation**: Continue work across different platforms
5. **Cloud-Native Checkpoints**: Direct cloud storage integration

### Enhanced Recovery
1. **Smart Recovery**: AI-powered recovery optimization
2. **Parallel Recovery**: Recover multiple work units simultaneously
3. **Selective Recovery**: Choose optimal recovery points
4. **Rollback Support**: Roll back to earlier checkpoints if needed
5. **Recovery Analytics**: Detailed recovery performance analysis

The task continuation system provides robust, flexible, and efficient mechanisms for preserving and recovering work progress across all interruption scenarios, ensuring maximum computational efficiency and minimal work loss.