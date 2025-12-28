# Chert Miner Documentation

## Overview

This directory contains comprehensive documentation for the Chert miner, covering all aspects of installation, configuration, operation, and management.

## Documentation Structure

### Core Documentation

- **[WORK_TYPE_MANAGEMENT.md](WORK_TYPE_MANAGEMENT.md)** - Managing work types and preferences (NUW CPU + BOINC GPU + CPU, GPU only, BOINC only, etc.)
- **[BOINC_PROJECT_MANAGEMENT.md](BOINC_PROJECT_MANAGEMENT.md)** - Selecting and managing BOINC projects
- **[UI_EXPRESSIVENESS.md](UI_EXPRESSIVENESS.md)** - UI expressiveness for work happening status
- **[SUBMISSION_TRACKING.md](SUBMISSION_TRACKING.md)** - Tracking system for submission validation
- **[TASK_CONTINUATION.md](TASK_CONTINUATION.md)** - Continuation options for tasks
- **[SELF_SETUP_CONFIGURATION.md](SELF_SETUP_CONFIGURATION.md)** - Self-setup and default configurations
- **[COMMAND_LINE_INTERACTIVE.md](COMMAND_LINE_INTERACTIVE.md)** - Command-line and interactive configuration options

### Quick Start

1. **First Time Setup**: See [SELF_SETUP_CONFIGURATION.md](SELF_SETUP_CONFIGURATION.md) for initial setup
2. **Configure Work Types**: See [WORK_TYPE_MANAGEMENT.md](WORK_TYPE_MANAGEMENT.md) for work preferences
3. **BOINC Projects**: See [BOINC_PROJECT_MANAGEMENT.md](BOINC_PROJECT_MANAGEMENT.md) for project management
4. **Monitor Progress**: See [UI_EXPRESSIVENESS.md](UI_EXPRESSIVENESS.md) for status monitoring
5. **Track Submissions**: See [SUBMISSION_TRACKING.md](SUBMISSION_TRACKING.md) for submission status

## Key Features Documented

### ✅ Work Type Management
- NUW CPU + BOINC GPU + CPU (if no NUW)
- GPU work only
- BOINC + NUW GPU (with NUW limits)
- NUW only
- BOINC only

### ✅ BOINC Project Management
- Project selection and attachment
- Project prioritization
- Resource allocation per project
- Automatic project switching

### ✅ UI Expressiveness
- Real-time work status display
- Progress indicators
- Performance metrics
- Alert systems

### ✅ Submission Tracking
- Submission validation timeline
- Status updates for both NUW and BOINC
- Retry mechanisms
- Success/failure notifications

### ✅ Task Continuation
- Automatic task continuation
- Manual continuation options
- Task priority management
- Resource reallocation

### ✅ Self-Setup & Configuration
- Automated setup wizards
- Default configuration templates
- Hardware detection and optimization
- Profile management

### ✅ Command-Line & Interactive Options
- Comprehensive CLI interface
- Interactive configuration modes
- Scriptable operations
- Remote management capabilities

## Getting Started

### Prerequisites
- Rust 1.70+ (for building from source)
- BOINC client (for scientific computing)
- Supported GPU drivers (for GPU acceleration)
- Network connectivity (for oracle communication)

### Installation
```bash
# Clone the repository
git clone https://github.com/chert-network/chert.git
cd chert/miner

# Build the miner
cargo build --release

# Run setup wizard
./target/release/chert-miner setup
```

### Basic Usage
```bash
# Start with default configuration
./target/release/chert-miner

# Start with specific profile
./target/release/chert-miner --profile gaming

# Start in interactive mode
./target/release/chert-miner --interactive
```

## Configuration

### Work Type Configuration
See [WORK_TYPE_MANAGEMENT.md](WORK_TYPE_MANAGEMENT.md) for detailed work type configuration:

```toml
[work_types]
mode = "mixed"  # mixed, nuw_only, boinc_only, gpu_only
nuw_cpu_percentage = 70
boinc_gpu_percentage = 80
fallback_enabled = true
```

### BOINC Configuration
See [BOINC_PROJECT_MANAGEMENT.md](BOINC_PROJECT_MANAGEMENT.md) for BOINC setup:

```toml
[boinc]
install_dir = "/opt/boinc"
data_dir = "/var/lib/boinc"
projects = [
    { url = "https://boinc.example.com/project1", priority = 1 },
    { url = "https://boinc.example.com/project2", priority = 2 }
]
```

### Oracle Configuration
```toml
[oracle]
url = "https://oracle.chert.network"
timeout = 30
require_https = true
verify_certificates = true
user_id = "your_miner_id"
api_key = "your_api_key"
```

## Monitoring and Management

### TUI Interface
The miner provides a comprehensive Terminal User Interface with:
- Real-time status dashboard
- Performance charts
- Resource usage monitoring
- Submission tracking
- Alert management

### CLI Management
All operations can be performed via command line:
```bash
# Check status
chert-miner status

# Switch work type
chert-miner work-type mixed

# Manage BOINC projects
chert-miner boinc projects list

# Monitor submissions
chert-miner submit list
```

## Troubleshooting

### Common Issues
1. **BOINC Connection Issues**: Check [BOINC_PROJECT_MANAGEMENT.md](BOINC_PROJECT_MANAGEMENT.md)
2. **Work Type Problems**: See [WORK_TYPE_MANAGEMENT.md](WORK_TYPE_MANAGEMENT.md)
3. **Submission Failures**: Refer to [SUBMISSION_TRACKING.md](SUBMISSION_TRACKING.md)
4. **UI Issues**: Check [UI_EXPRESSIVENESS.md](UI_EXPRESSIVENESS.md)

### Debug Mode
```bash
# Enable debug logging
chert-miner --log-level debug

# Run diagnostics
chert-miner diagnose

# Export diagnostics
chert-miner diagnose --export debug_info.json
```

## Advanced Configuration

### Profiles
Create and manage different configuration profiles:
```bash
# Create gaming profile
chert-miner profile create gaming

# Switch to profile
chert-miner profile switch gaming

# Edit profile
chert-miner profile edit gaming
```

### Templates
Apply pre-configured templates:
```bash
# List available templates
chert-miner template list

# Apply high-performance template
chert-miner template apply high-performance
```

## API Reference

### Configuration API
All configuration options are documented in their respective files:
- Work types: [WORK_TYPE_MANAGEMENT.md](WORK_TYPE_MANAGEMENT.md)
- BOINC: [BOINC_PROJECT_MANAGEMENT.md](BOINC_PROJECT_MANAGEMENT.md)
- UI: [UI_EXPRESSIVENESS.md](UI_EXPRESSIVENESS.md)
- Tracking: [SUBMISSION_TRACKING.md](SUBMISSION_TRACKING.md)

### Command Reference
Complete command reference: [COMMAND_LINE_INTERACTIVE.md](COMMAND_LINE_INTERACTIVE.md)

## Contributing

When contributing to the documentation:
1. Update the relevant section-specific documentation
2. Update this README.md if adding new sections
3. Ensure all cross-references are correct
4. Test all documented procedures

## Support

For additional support:
- Check the specific documentation for your issue
- Run diagnostics: `chert-miner diagnose`
- Export debug information: `chert-miner diagnose --export`
- Review logs in the TUI or via `chert-miner debug logs`

## Documentation Index

| Document | Purpose | Key Topics |
|----------|---------|-------------|
| [WORK_TYPE_MANAGEMENT.md](WORK_TYPE_MANAGEMENT.md) | Work type configuration | NUW, BOINC, GPU settings, fallback modes |
| [BOINC_PROJECT_MANAGEMENT.md](BOINC_PROJECT_MANAGEMENT.md) | BOINC integration | Project selection, attachment, resource allocation |
| [UI_EXPRESSIVENESS.md](UI_EXPRESSIVENESS.md) | User interface | Real-time status, progress indicators, alerts |
| [SUBMISSION_TRACKING.md](SUBMISSION_TRACKING.md) | Submission management | Validation tracking, retry logic, status updates |
| [TASK_CONTINUATION.md](TASK_CONTINUATION.md) | Task management | Continuation options, priority handling |
| [SELF_SETUP_CONFIGURATION.md](SELF_SETUP_CONFIGURATION.md) | Initial setup | Auto-configuration, templates, profiles |
| [COMMAND_LINE_INTERACTIVE.md](COMMAND_LINE_INTERACTIVE.md) | CLI reference | Commands, options, interactive mode |

---

**Last Updated**: October 2025  
**Version**: 1.0.0  
**Compatible with**: Chert Miner v2.0+