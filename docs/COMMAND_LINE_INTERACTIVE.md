
# Command-Line and Interactive Configuration Options

## Overview

The Chert miner provides comprehensive command-line interface (CLI) and interactive configuration options for controlling all aspects of mining operations.

## Command-Line Interface

### Basic Usage

```bash
# Start miner with default configuration
chert-miner

# Start with specific configuration file
chert-miner --config /path/to/config.toml

# Start with specific profile
chert-miner --profile gaming

# Start in headless mode (no TUI)
chert-miner --headless

# Start with verbose output
chert-miner --verbose
```

### Global Options

```bash
--config, -c PATH          # Configuration file path
--profile, -p NAME         # Configuration profile to use
--verbose, -v               # Enable verbose logging
--quiet, -q                 # Suppress non-error output
--help, -h                  # Show help information
--version, -V               # Show version information
--log-level LEVEL            # Set logging level (debug, info, warn, error)
--data-dir PATH             # Data directory path
--no-color                   # Disable colored output
--no-interactive             # Run in non-interactive mode
```

### Available Commands

#### Configuration Commands

```bash
chert-miner config init                    # Initialize default configuration
chert-miner config show                    # Show current configuration
chert-miner config validate                 # Validate configuration file
chert-miner config set KEY VALUE           # Set configuration value
chert-miner config get KEY                 # Get configuration value
chert-miner config reset                  # Reset to defaults
chert-miner config edit                    # Open configuration in editor
chert-miner config backup                  # Backup current configuration
chert-miner config restore FILE           # Restore from backup

# Profile management
chert-miner profile create NAME            # Create new profile
chert-miner profile list                   # List available profiles
chert-miner profile switch NAME           # Switch to profile
chert-miner profile delete NAME           # Delete profile
chert-miner profile export NAME FILE      # Export profile to file
chert-miner profile import FILE NAME       # Import profile from file
chert-miner profile edit NAME             # Edit profile
```

#### Mining Commands

```bash
# Mining control
chert-miner start                         # Start mining
chert-miner stop                          # Stop mining
chert-miner restart                       # Restart mining
chert-miner status                        # Show mining status
chert-miner pause                         # Pause mining
chert-miner resume                        # Resume mining

# Work type control
chert-miner work-type nuw                 # Switch to NUW only
chert-miner work-type boinc                # Switch to BOINC only
chert-miner work-type mixed                 # Switch to mixed mode
chert-miner work-type auto                  # Auto-select work type

# Resource management
chert-miner resources cpu-percentage 50   # Set CPU usage percentage
chert-miner resources gpu-percentage 75   # Set GPU usage percentage
chert-miner resources memory-limit 8GB    # Set memory limit
chert-miner resources priority high        # Set process priority
```

#### BOINC Commands

```bash
# BOINC management
chert-miner boinc install                   # Install BOINC client
chert-miner boinc update                    # Update BOINC client
chert-miner boinc status                    # Show BOINC status
chert-miner boinc projects list            # List available projects
chert-miner boinc projects attach URL      # Attach to BOINC project
chert-miner boinc projects detach URL      # Detach from BOINC project
chert-miner boinc tasks list               # List current tasks
chert-miner boinc tasks suspend ID         # Suspend specific task
chert-miner boinc tasks resume ID          # Resume specific task
chert-miner boinc tasks abort ID            # Abort specific task
```

#### Oracle Commands

```bash
# Oracle management
chert-miner oracle status                   # Show oracle connection status
chert-miner oracle test URL                # Test oracle connectivity
chert-miner oracle switch URL              # Switch to different oracle
chert-miner oracle list                    # List available oracles
chert-miner oracle ping                    # Ping current oracle

# Submission management
chert-miner submit work-unit ID RESULT    # Submit work result
chert-miner submit list                    # List pending submissions
chert-miner submit status ID              # Check submission status
chert-miner submit cancel ID               # Cancel pending submission
```

#### Monitoring Commands

```bash
# Performance monitoring
chert-miner monitor start                  # Start performance monitoring
chert-miner monitor stop                   # Stop performance monitoring
chert-miner monitor report                 # Generate performance report
chert-miner monitor export FILE           # Export monitoring data
chert-miner monitor metrics                # Show current metrics

# System monitoring
chert-miner system info                    # Show system information
chert-miner system hardware                 # Show hardware details
chert-miner system temperature              # Show temperature readings
chert-miner system resources                # Show resource usage
chert-miner system network                 # Show network status
```

#### Utility Commands

```bash
# Setup and maintenance
chert-miner setup                         # Run setup wizard
chert-miner setup --silent                 # Run silent setup
chert-miner setup --advanced               # Run advanced setup
chert-miner check-updates                  # Check for updates
chert-miner update                         # Update to latest version
chert-miner repair                         # Repair installation
chert-miner uninstall                     # Uninstall miner

# Debugging and diagnostics
chert-miner debug logs                     # Show debug logs
chert-miner debug config                   # Debug configuration
chert-miner debug network                  # Debug network issues
chert-miner debug performance              # Debug performance issues
chert-miner diagnose                       # Run full diagnostics
chert-miner diagnose --export FILE         # Export diagnostics to file
```

## Interactive Configuration

### Interactive Mode

The miner provides an interactive configuration mode for guided setup:

```bash
# Start interactive configuration
chert-miner --interactive

# Interactive configuration for specific section
chert-miner --interactive --section oracle
chert-miner --interactive --section boinc
chert-miner --interactive --section performance
```

### Interactive Configuration Examples

#### Oracle Configuration Screen
```
┌─────────────────────────────────────────────────────────────┐
│ Oracle Configuration                                    [Back] [Save] │
├─────────────────────────────────────────────────────────────┤
│ Oracle Server Settings                                       │
│                                                             │
│ URL: [https://oracle.chert.network]                       │
│ Timeout: [30] seconds                                      │
│ Require HTTPS: [✓] Yes                                     │
│ Verify Certificates: [✓] Yes                               │
│                                                             │
│ Authentication                                               │
│ User ID: [miner_001]                                      │
│ 