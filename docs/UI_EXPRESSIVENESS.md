# UI Expressiveness Documentation

## Overview

The Chert miner provides a comprehensive Terminal User Interface (TUI) that delivers real-time visibility into all mining operations. The interface is designed to be highly expressive, showing detailed information about work progress, system performance, and operational status through multiple interactive panels and visualizations.

## TUI Architecture

### Multi-Panel Layout

The TUI uses a responsive multi-panel layout that adapts to different terminal sizes:

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│ Chert Miner                    [Dashboard] [Performance] [Config] [Alerts] [Logs] │ Status: ● OK │
├─────────────────────────────────────────────────────────────────────────────────────┤
│ BOINC Task                    │ System Resources                │ Performance │
│ Task: mw_001234_001          │ CPU Usage: 45.2% (8 cores)    │ Summary    │
│ Project: MilkyWay@Home         │ Memory: 8.2GB / 16GB (51.2%) │            │
│ Progress: ████████░░ 67.3%    │ Load Average: 2.34              │ Current:   │
│ CPU Time: 4h 23m 15s          │ Efficiency: CPU 89% | Mem 76%   │ 3.2 GFLOPS │
│ Elapsed: 6h 12m 04s           │                                 │            │
│ FLOPS Rate: 3.45 GFLOPS       │                                 │ Average:   │
│ Memory Peak: 1.2GB             │                                 │ 2.8 GFLOPS │
│                                │                                 │            │
├─────────────────────────────────────────────────────────────────────────────────────┤
│ 1-5:Tabs | Tab:Next | Shift+Tab:Prev | ?:Help | q:Quit                    │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

### Responsive Design Modes

The interface automatically adapts to terminal size:

1. **Compact Mode** (80x24 minimum): Essential panels only
2. **Standard Mode** (120x30+): Full feature set
3. **Wide Mode** (160x40+): Side-by-side panels
4. **Full Mode** (200x50+): Maximum information density

## Real-Time Information Display

### Dashboard Panel

The main dashboard provides comprehensive mining status:

#### BOINC Task Information
- **Task Name**: Current work unit identifier
- **Project**: Active BOINC project (MilkyWay@Home, Rosetta@Home, etc.)
- **Progress Bar**: Visual progress indicator with percentage
- **CPU Time**: Accumulated computation time
- **Elapsed Time**: Wall-clock time since task start
- **FLOPS Rate**: Current computational performance
- **Memory Peak**: Maximum memory usage during task

#### System Resources Panel
- **CPU Usage**: Real-time CPU utilization with core count
- **Memory Usage**: Used/total memory with percentage
- **Load Average**: System load over 1 minute
- **Efficiency Metrics**: CPU and memory efficiency calculations

#### Performance Summary
- **Current FLOPS**: Real-time computational rate
- **Average FLOPS**: Performance over last hour
- **Work Units/Hour**: Task completion rate
- **Estimated Completion**: Time remaining for current task
- **Power Efficiency**: FLOPS per watt (when available)

### Performance Charts Panel

Interactive real-time charts show historical performance:

#### CPU Usage Chart
- **Time Range**: Last 10 minutes of data
- **Update Rate**: 4 times per second
- **Resolution**: 1-second intervals
- **Color Coding**: Cyan line with gradient fill

#### Memory Usage Chart
- **Time Range**: Last 10 minutes of data
- **Update Rate**: 4 times per second
- **Resolution**: 1-second intervals
- **Color Coding**: Yellow line with gradient fill

#### FLOPS Performance Chart
- **Time Range**: Last 10 minutes of data
- **Update Rate**: 4 times per second
- **Resolution**: 1-second intervals
- **Color Coding**: Green line with gradient fill

### Configuration Management Panel

Interactive configuration editor with real-time validation:

#### Configuration Categories
1. **Oracle Settings**: URL, timeout, authentication
2. **BOINC Settings**: Installation paths, data directories
3. **Work Allocation**: CPU/GPU resource distribution
4. **Security Settings**: HTTPS requirements, certificate verification
5. **Debug Options**: Logging levels, debug modes

#### Interactive Editing
- **Field Navigation**: Arrow keys to move between fields
- **Edit Mode**: Ctrl+C to enter/exit editing
- **Validation**: Real-time input validation with error display
- **Save/Cancel**: Enter to save, Esc to cancel

### Alerts Panel

Comprehensive alert system with severity levels:

#### Alert Types
1. **Info** (ℹ): General information and status updates
2. **Warning** (⚠): Potential issues requiring attention
3. **Error** (✗): Errors affecting mining operations
4. **Critical** (🚨): Immediate action required

#### Alert Management
- **Active Alerts**: Current unresolved issues
- **Alert History**: Recent alert timeline
- **Acknowledgment**: Mark alerts as acknowledged
- **Auto-Cleanup**: Remove old alerts automatically

### Logs Panel

Enhanced logging with categorization and filtering:

#### Log Categories
1. **System**: Core miner operations and lifecycle events
2. **BOINC**: Project-specific operations and communications
3. **Network**: Oracle and project server interactions
4. **Performance**: Metrics collection and analysis events
5. **Security**: Authentication and validation events
6. **Config**: Configuration changes and validation
7. **General**: Uncategorized events and messages

#### Log Features
- **Real-time Updates**: New logs appear immediately
- **Color Coding**: Level-based color coding (ERROR=red, WARN=yellow, etc.)
- **Timestamps**: Precise timing for all events
- **Source Identification**: Component that generated each log
- **Scrolling Navigation**: Full keyboard navigation support
- **Search Capability**: Find specific log entries (planned)

## Interactive Features

### Keyboard Navigation

#### Global Shortcuts
- **q**: Quit application
- **Tab**: Next tab
- **Shift+Tab**: Previous tab
- **1-5**: Jump to specific tab
- **?**: Toggle help overlay
- **Ctrl+C**: Enter/exit configuration edit mode
- **Ctrl+T**: Switch theme

#### Tab-Specific Shortcuts
- **Dashboard**: No special shortcuts (display only)
- **Performance**: No special shortcuts (display only)
- **Configuration**: 
  - **↑/↓**: Navigate fields
  - **Enter**: Save changes
  - **Esc**: Cancel editing
- **Alerts**: No special shortcuts (display only)
- **Logs**:
  - **↑/↓**: Scroll lines
  - **PgUp/PgDn**: Scroll pages
  - **Home/End**: Jump to top/bottom

### Theme System

Multiple color themes for different preferences and accessibility:

#### Available Themes
1. **Default**: High contrast dark theme
2. **Dark**: Pure black background with bright colors
3. **Light**: White background with dark text
4. **High Contrast**: Enhanced contrast for accessibility
5. **Colorblind**: Colorblind-friendly palette

#### Theme Customization
- **Dynamic Switching**: Change themes without restart
- **Persistent Selection**: Remember theme choice
- **Accessibility**: High contrast and colorblind options
- **Custom Themes**: User-defined color schemes (planned)

### Help System

Context-sensitive help with multiple topics:

#### Help Topics
1. **General**: Application overview and basic usage
2. **Navigation**: Keyboard shortcuts and interface navigation
3. **Configuration**: Settings management and options
4. **Alerts**: Alert system understanding and management
5. **Charts**: Performance chart interpretation

#### Help Features
- **Contextual**: Help relevant to current tab
- **Searchable**: Find specific help topics (planned)
- **Navigable**: Browse between help sections
- **Examples**: Practical usage examples

## Performance Monitoring

### Real-Time Metrics Collection

The TUI collects and displays metrics from multiple sources:

#### System Metrics
- **CPU Usage**: Per-core and total utilization
- **Memory Usage**: Used, available, and swap usage
- **Disk I/O**: Read/write rates and space usage
- **Network I/O**: Transfer rates and connection status
- **Temperature**: CPU and GPU temperature monitoring

#### BOINC Metrics
- **Task Progress**: Real-time progress from BOINC logs
- **Computation Rate**: FLOPS calculation from CPU time
- **Memory Usage**: Peak and current memory consumption
- **Checkpoint Status**: Progress saving and recovery information

#### Mining Metrics
- **Hash Rate**: Traditional mining performance (when active)
- **Share Submission**: Mining share submission statistics
- **Block Finding**: Block discovery events and rewards
- **Network Difficulty**: Current mining difficulty

### Data Visualization

#### Progress Indicators
- **Progress Bars**: Visual task completion indicators
- **Percentage Display**: Precise progress percentages
- **Time Estimates**: Remaining time calculations
- **Rate Information**: Current and average rates

#### Charts and Graphs
- **Line Charts**: Time-series performance data
- **Area Charts**: Filled area under performance curves
- **Real-time Updates**: Smooth 4Hz update rate
- **Historical Data**: 10-minute rolling window

#### Status Indicators
- **Color Coding**: Intuitive color-based status
- **Icons**: Unicode symbols for quick recognition
- **Blinking**: Attention-grabbing animations for critical issues
- **Animations**: Smooth transitions and updates

## Accessibility Features

### Visual Accessibility
- **High Contrast Themes**: Enhanced contrast for visibility
- **Colorblind Support**: Colorblind-friendly color palettes
- **Large Text Options**: Scalable text sizes
- **Unicode Symbols**: Clear, recognizable icons

### Navigation Accessibility
- **Keyboard Only**: Full keyboard navigation support
- **Consistent Shortcuts**: Logical keyboard shortcuts
- **Help Integration**: Built-in help and guidance
- **Error Recovery**: Clear error messages and recovery paths

### Customization Options
- **Theme Selection**: Multiple built-in themes
- **Layout Preferences**: Adjustable panel sizes
- **Update Rates**: Configurable refresh frequencies
- **Information Density**: Control amount of displayed information

## Advanced Features

### Multi-Miner Support
- **Tabbed Interface**: Separate tabs for multiple miners
- **Aggregate Views**: Combined performance metrics
- **Cross-Miner Alerts**: System-wide alert management
- **Resource Sharing**: Coordinate resource allocation

### Remote Monitoring
- **Network Status**: Connection quality and latency
- **Remote Logs**: Access logs from remote instances
- **Performance Export**: Export performance data
- **Alert Forwarding**: Forward alerts to external systems

### Plugin Architecture
- **Custom Panels**: User-defined interface panels
- **Extended Metrics**: Additional metric sources
- **Custom Themes**: User-created color schemes
- **Integration Hooks**: External system integration

## Troubleshooting

### Common UI Issues

#### Display Problems
**Symptoms**: Garbled display, incorrect colors, layout issues
**Solutions**:
```bash
# Reset terminal
reset

# Check terminal capabilities
echo $TERM

# Force 256-color mode
export TERM=xterm-256color

# Use compatible terminal
chert-miner --tui --force-basic
```

#### Performance Issues
**Symptoms**: Slow UI updates, high CPU usage
**Solutions**:
```bash
# Reduce update rate
export CHERT_TUI_UPDATE_RATE=2  # Hz

# Disable animations
export CHERT_TUI_ANIMATIONS=false

# Use minimal layout
export CHERT_TUI_LAYOUT=compact
```

#### Keyboard Issues
**Symptoms**: Keys not working, incorrect behavior
**Solutions**:
```bash
# Check key bindings
chert-miner --help-keys

# Reset key bindings
chert-miner --reset-keys

# Use alternative key set
export CHERT_TUI_KEYSET=vi
```

### Debug Mode

Enable comprehensive UI debugging:

```bash
# Enable UI debug logging
CHERT_DEBUG_TUI=true
CHERT_DEBUG_LAYOUT=true
CHERT_DEBUG_RENDERING=true

# Enable performance profiling
CHERT_PROFILE_TUI=true
CHERT_DEBUG_UPDATES=true
```

Debug information includes:
- Layout calculation details
- Rendering performance metrics
- Event handling traces
- Memory usage statistics

## Best Practices

### Terminal Configuration
1. **Use Modern Terminal**: Ensure proper Unicode and color support
2. **Appropriate Size**: Minimum 80x24, recommended 120x30+
3. **Font Selection**: Use monospace fonts for proper alignment
4. **Color Support**: Enable 256-color or truecolor support

### Usage Optimization
1. **Start Simple**: Begin with default configuration
2. **Learn Navigation**: Master keyboard shortcuts
3. **Monitor Performance**: Watch performance charts regularly
4. **Configure Alerts**: Set up appropriate alert thresholds

### Customization
1. **Theme Selection**: Choose comfortable color scheme
2. **Layout Adjustment**: Adapt panel sizes to preferences
3. **Update Rates**: Balance responsiveness with resource usage
4. **Information Density**: Adjust displayed information amount

## Future Enhancements

### Planned UI Improvements
1. **Web Interface**: Browser-based management interface
2. **Mobile Support**: Mobile-optimized interface
3. **Touch Support**: Touch-enabled navigation
4. **Voice Control**: Voice command integration
5. **AR/VR Support**: Immersive monitoring interfaces

### Advanced Visualizations
1. **3D Charts**: Three-dimensional performance visualization
2. **Heat Maps**: Resource utilization heat maps
3. **Network Graphs**: Visual network topology and flow
4. **Predictive Analytics**: Future performance predictions
5. **Comparative Analysis**: Side-by-side performance comparisons

The Chert miner TUI provides a comprehensive, expressive interface for monitoring and managing all aspects of mining operations, with extensive customization options and accessibility features to suit diverse user needs.