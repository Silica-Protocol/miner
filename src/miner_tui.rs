//! Enhanced Terminal User Interface for the Chert miner
//!
//! Provides a comprehensive real-time dashboard showing:
//! - Multi-panel layout with resizable sections
//! - Real-time performance charts and graphs
//! - Interactive configuration management
//! - Enhanced BOINC task progress visualization
//! - System resource monitoring with alerts
//! - Historical performance data tracking
//! - Alert system for performance issues
//! - Multiple color themes and accessibility options
//! - Responsive design for different terminal sizes

use crate::performance_monitor::{PerformanceMonitor, format_bytes, format_duration, format_flops};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Chart, Clear, Dataset, Gauge, GraphType, List, ListItem, ListState,
        Paragraph, Tabs, Wrap,
    },
};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

/// Enhanced TUI Application state with comprehensive monitoring
pub struct MinerTui {
    /// Performance monitor with enhanced data collection
    performance_monitor: PerformanceMonitor,
    /// Log messages with enhanced filtering and search
    log_messages: Arc<Mutex<VecDeque<LogMessage>>>,
    /// Current active tab
    current_tab: usize,
    /// Is running
    running: bool,
    /// Log scroll state with enhanced navigation
    log_scroll_state: ListState,
    /// Configuration editor state
    config_editor: ConfigEditor,
    /// Alert system state
    alert_system: AlertSystem,
    /// Theme manager
    theme_manager: ThemeManager,
    /// Layout manager for responsive design
    layout_manager: LayoutManager,
    /// Historical data for charts
    historical_data: HistoricalData,
    /// Help system
    help_system: HelpSystem,
    /// Last significant metrics for comparison
    last_boinc_progress: f64,
}

/// Enhanced log message with additional metadata
#[derive(Debug, Clone)]
pub struct LogMessage {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub category: LogCategory,
    pub source: String,
}

/// Log categories for better filtering
#[derive(Debug, Clone, PartialEq)]
pub enum LogCategory {
    System,
    Boinc,
    Network,
    Performance,
    Security,
    Config,
    General,
}

/// Interactive configuration editor
#[derive(Debug, Clone, Default)]
pub struct ConfigEditor {
    pub active_field: usize,
    pub editing_mode: bool,
    pub temp_values: HashMap<String, String>,
    pub validation_errors: Vec<String>,
}

/// Alert system for performance monitoring
#[derive(Debug, Clone)]
pub struct AlertSystem {
    pub active_alerts: Vec<Alert>,
    pub alert_history: VecDeque<Alert>,
    pub thresholds: AlertThresholds,
}

/// Performance alert thresholds
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    pub cpu_usage_high: f32,
    pub memory_usage_high: f32,
    pub boinc_progress_stalled: StdDuration,
    pub network_timeout: StdDuration,
}

/// Individual alert
#[derive(Debug, Clone)]
pub struct Alert {
    pub id: String,
    pub level: AlertLevel,
    pub title: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
}

/// Alert severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Theme manager for color schemes
#[derive(Debug, Clone)]
pub struct ThemeManager {
    pub current_theme: Theme,
    pub available_themes: Vec<Theme>,
}

/// Color theme definition
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
}

/// Theme color palette
#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub background: Color,
    pub foreground: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub border: Color,
    pub highlight: Color,
}

/// Responsive layout manager
#[derive(Debug, Clone)]
pub struct LayoutManager {
    pub terminal_size: (u16, u16),
    pub layout_mode: LayoutMode,
}

/// Layout modes for different terminal sizes
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutMode {
    Compact,  // Small terminals
    Standard, // Normal terminals
    Wide,     // Large terminals
    Full,     // Very large terminals
}

/// Historical data for performance charts
#[derive(Debug, Clone)]
pub struct HistoricalData {
    pub cpu_history: VecDeque<(DateTime<Utc>, f32)>,
    pub memory_history: VecDeque<(DateTime<Utc>, f32)>,
    pub flops_history: VecDeque<(DateTime<Utc>, f64)>,
    pub max_points: usize,
}

/// Integrated help system
#[derive(Debug, Clone)]
pub struct HelpSystem {
    pub show_help: bool,
    pub current_topic: HelpTopic,
}

/// Help topics
#[derive(Debug, Clone, PartialEq)]
pub enum HelpTopic {
    General,
    Navigation,
    Configuration,
    Alerts,
    Charts,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            name: "Default".to_string(),
            colors: ThemeColors {
                background: Color::Black,
                foreground: Color::White,
                accent: Color::Cyan,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Blue,
                border: Color::Gray,
                highlight: Color::Magenta,
            },
        }
    }
}

impl ConfigEditor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn save_changes(&mut self) {
        // TODO: Implement saving configuration changes
        self.editing_mode = false;
        self.validation_errors.clear();
    }

    pub fn cancel_editing(&mut self) {
        self.editing_mode = false;
        self.temp_values.clear();
        self.validation_errors.clear();
    }
}

impl AlertSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_alert(&mut self, id: String, level: AlertLevel, title: String, message: String) {
        // Check if alert already exists
        if self.active_alerts.iter().any(|a| a.id == id) {
            return;
        }

        let alert = Alert {
            id,
            level,
            title,
            message,
            timestamp: Utc::now(),
            acknowledged: false,
        };

        self.active_alerts.push(alert.clone());
        self.alert_history.push_back(alert);

        // Keep history limited
        if self.alert_history.len() > 100 {
            self.alert_history.pop_front();
        }
    }

    pub fn has_critical_alerts(&self) -> bool {
        self.active_alerts
            .iter()
            .any(|a| matches!(a.level, AlertLevel::Critical))
    }

    pub fn has_warning_alerts(&self) -> bool {
        self.active_alerts
            .iter()
            .any(|a| matches!(a.level, AlertLevel::Warning | AlertLevel::Error))
    }
}

impl Default for AlertSystem {
    fn default() -> Self {
        Self {
            active_alerts: Vec::new(),
            alert_history: VecDeque::new(),
            thresholds: AlertThresholds {
                cpu_usage_high: 90.0,
                memory_usage_high: 95.0,
                boinc_progress_stalled: Duration::minutes(5).to_std().unwrap(),
                network_timeout: Duration::seconds(30).to_std().unwrap(),
            },
        }
    }
}

impl ThemeManager {
    pub fn new(theme: Theme) -> Self {
        let mut available_themes = vec![theme.clone()];

        // Add some predefined themes
        available_themes.push(Theme {
            name: "Dark".to_string(),
            colors: ThemeColors {
                background: Color::Black,
                foreground: Color::White,
                accent: Color::Cyan,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Blue,
                border: Color::Gray,
                highlight: Color::Magenta,
            },
        });

        available_themes.push(Theme {
            name: "Light".to_string(),
            colors: ThemeColors {
                background: Color::White,
                foreground: Color::Black,
                accent: Color::Blue,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Blue,
                border: Color::Gray,
                highlight: Color::Magenta,
            },
        });

        Self {
            current_theme: theme,
            available_themes,
        }
    }

    pub fn next_theme(&mut self) {
        let current_index = self
            .available_themes
            .iter()
            .position(|t| t.name == self.current_theme.name)
            .unwrap_or(0);
        let next_index = (current_index + 1) % self.available_themes.len();
        self.current_theme = self.available_themes[next_index].clone();
    }
}

impl LayoutManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_size(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);

        self.layout_mode = if width < 80 || height < 20 {
            LayoutMode::Compact
        } else if width < 120 || height < 30 {
            LayoutMode::Standard
        } else if width < 160 || height < 40 {
            LayoutMode::Wide
        } else {
            LayoutMode::Full
        };
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self {
            terminal_size: (80, 24),
            layout_mode: LayoutMode::Standard,
        }
    }
}

impl HistoricalData {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for HistoricalData {
    fn default() -> Self {
        Self {
            cpu_history: VecDeque::new(),
            memory_history: VecDeque::new(),
            flops_history: VecDeque::new(),
            max_points: 600,
        }
    }
}

impl HelpSystem {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for HelpSystem {
    fn default() -> Self {
        Self {
            show_help: false,
            current_topic: HelpTopic::General,
        }
    }
}

impl MinerTui {
    /// Create a new enhanced miner TUI
    pub fn new(boinc_data_dir: String) -> Self {
        let mut log_state = ListState::default();
        log_state.select(Some(0));

        let theme = Theme::default();

        Self {
            performance_monitor: PerformanceMonitor::new_with_options(boinc_data_dir, true),
            log_messages: Arc::new(Mutex::new(VecDeque::new())),
            current_tab: 0,
            running: true,
            log_scroll_state: log_state,
            config_editor: ConfigEditor::new(),
            alert_system: AlertSystem::new(),
            theme_manager: ThemeManager::new(theme),
            layout_manager: LayoutManager::new(),
            historical_data: HistoricalData::new(),
            help_system: HelpSystem::new(),
            last_boinc_progress: 0.0,
        }
    }

    /// Run the enhanced TUI application
    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Add initial log messages
        self.add_log_message(
            "INFO".to_string(),
            "Enhanced Chert Miner TUI Started".to_string(),
            LogCategory::System,
            "TUI".to_string(),
        );
        self.add_log_message(
            "INFO".to_string(),
            "Connecting to BOINC client...".to_string(),
            LogCategory::Boinc,
            "TUI".to_string(),
        );
        self.add_log_message(
            "INFO".to_string(),
            "Use Tab/Shift+Tab to navigate tabs, ? for help, q to quit".to_string(),
            LogCategory::General,
            "TUI".to_string(),
        );

        // Main loop
        let mut last_tick = tokio::time::Instant::now();
        let tick_rate = StdDuration::from_millis(250); // Update 4 times per second

        while self.running {
            // Update layout based on terminal size
            let size = terminal.size()?;
            self.layout_manager.update_size(size.width, size.height);

            // Draw UI
            terminal.draw(|f| self.ui(f))?;

            // Handle events
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| StdDuration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key_event(key);
                }
            }

            if last_tick.elapsed() >= tick_rate {
                // Collect metrics and update historical data
                if let Ok(metrics) = self.performance_monitor.collect_metrics() {
                    self.update_historical_data(&metrics);
                    self.check_alerts(&metrics);
                }

                // Update alerts and other periodic tasks
                self.update_alerts();

                last_tick = tokio::time::Instant::now();
            }
        }

        // Cleanup terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }

    /// Handle keyboard events with enhanced navigation
    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.current_tab = self.current_tab.saturating_sub(1);
                } else {
                    self.current_tab = (self.current_tab + 1) % self.get_tab_count();
                }
            }
            KeyCode::Char('?') => self.help_system.show_help = !self.help_system.show_help,
            KeyCode::Char('1') => self.current_tab = 0,
            KeyCode::Char('2') => self.current_tab = 1,
            KeyCode::Char('3') => self.current_tab = 2,
            KeyCode::Char('4') => self.current_tab = 3,
            KeyCode::Char('5') => self.current_tab = 4,
            KeyCode::Char('c') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.config_editor.editing_mode = !self.config_editor.editing_mode;
                }
            }
            KeyCode::Char('t') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.theme_manager.next_theme();
                }
            }
            KeyCode::Up => self.scroll_logs_up(),
            KeyCode::Down => self.scroll_logs_down(),
            KeyCode::PageUp => self.scroll_logs_page_up(),
            KeyCode::PageDown => self.scroll_logs_page_down(),
            KeyCode::Home => self.scroll_logs_to_top(),
            KeyCode::End => self.scroll_logs_to_bottom(),
            KeyCode::Enter => {
                if self.config_editor.editing_mode {
                    self.config_editor.save_changes();
                }
            }
            KeyCode::Esc => {
                if self.config_editor.editing_mode {
                    self.config_editor.cancel_editing();
                } else if self.help_system.show_help {
                    self.help_system.show_help = false;
                }
            }
            _ => {}
        }
    }

    /// Get number of tabs based on layout mode
    fn get_tab_count(&self) -> usize {
        match self.layout_manager.layout_mode {
            LayoutMode::Compact => 3, // Dashboard, Performance, Logs
            _ => 5,                   // Dashboard, Performance, Configuration, Alerts, Logs
        }
    }

    /// Enhanced main UI rendering function
    fn ui(&mut self, f: &mut Frame) {
        let size = f.size();

        // Handle help overlay
        if self.help_system.show_help {
            self.render_help_overlay(f, size);
            return;
        }

        // Handle configuration editor overlay
        if self.config_editor.editing_mode {
            self.render_config_editor(f, size);
            return;
        }

        // Main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Header with tabs and status
                Constraint::Min(0),    // Content
                Constraint::Length(2), // Footer with shortcuts
            ])
            .split(size);

        // Header with enhanced tab bar and status indicators
        self.render_header(f, chunks[0]);

        // Content based on selected tab
        match self.current_tab {
            0 => self.render_dashboard(f, chunks[1]),
            1 => self.render_performance(f, chunks[1]),
            2 => {
                if self.layout_manager.layout_mode == LayoutMode::Compact {
                    self.render_logs(f, chunks[1])
                } else {
                    self.render_configuration(f, chunks[1])
                }
            }
            3 => {
                if self.layout_manager.layout_mode == LayoutMode::Compact {
                    // No tab 3 in compact mode
                } else {
                    self.render_alerts(f, chunks[1])
                }
            }
            4 => self.render_logs(f, chunks[1]),
            _ => {}
        }

        // Footer with keyboard shortcuts
        self.render_footer(f, chunks[2]);
    }

    /// Render enhanced header with tabs and status indicators
    fn render_header(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(20),    // Tabs
                Constraint::Length(30), // Status indicators
            ])
            .split(area);

        // Enhanced tab bar
        let tab_names = match self.layout_manager.layout_mode {
            LayoutMode::Compact => vec!["Dashboard", "Performance", "Logs"],
            _ => vec!["Dashboard", "Performance", "Config", "Alerts", "Logs"],
        };

        let tabs = Tabs::new(tab_names)
            .block(Block::default().borders(Borders::ALL).title("Chert Miner"))
            .style(Style::default().fg(self.theme_manager.current_theme.colors.foreground))
            .highlight_style(
                Style::default()
                    .fg(self.theme_manager.current_theme.colors.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .select(self.current_tab);
        f.render_widget(tabs, chunks[0]);

        // Status indicators
        self.render_status_indicators(f, chunks[1]);
    }

    /// Render status indicators in header
    fn render_status_indicators(&self, f: &mut Frame, area: Rect) {
        let mut status_lines = Vec::new();

        // Connection status
        let connection_status = if self.alert_system.has_critical_alerts() {
            Span::styled("● CRITICAL", Style::default().fg(Color::Red))
        } else if self.alert_system.has_warning_alerts() {
            Span::styled("● WARNING", Style::default().fg(Color::Yellow))
        } else {
            Span::styled("● OK", Style::default().fg(Color::Green))
        };
        status_lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Gray)),
            connection_status,
        ]));

        // Active alerts count
        let active_alerts = self.alert_system.active_alerts.len();
        if active_alerts > 0 {
            status_lines.push(Line::from(vec![
                Span::styled("Alerts: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}", active_alerts),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        }

        // Current theme
        status_lines.push(Line::from(vec![
            Span::styled("Theme: ", Style::default().fg(Color::Gray)),
            Span::styled(
                &self.theme_manager.current_theme.name,
                Style::default().fg(Color::Cyan),
            ),
        ]));

        let status_block = Paragraph::new(status_lines)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .wrap(Wrap { trim: true });
        f.render_widget(status_block, area);
    }

    /// Render enhanced dashboard with multi-panel layout
    fn render_dashboard(&mut self, f: &mut Frame, area: Rect) {
        let layout = match self.layout_manager.layout_mode {
            LayoutMode::Compact => {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(6), // BOINC task
                        Constraint::Length(4), // System resources
                        Constraint::Min(0),    // Charts preview
                    ])
                    .split(area)
            }
            LayoutMode::Standard => {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(8), // BOINC task
                        Constraint::Length(6), // System resources
                        Constraint::Min(0),    // Performance summary
                    ])
                    .split(area)
            }
            _ => {
                // Wide layout with side-by-side panels
                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(60), // Left side
                        Constraint::Percentage(40), // Right side
                    ])
                    .split(area);

                let left_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(10), // BOINC task
                        Constraint::Length(8),  // System resources
                        Constraint::Min(0),     // Charts
                    ])
                    .split(main_chunks[0]);

                let _right_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(50), // Performance summary
                        Constraint::Percentage(50), // Quick stats
                    ])
                    .split(main_chunks[1]);

                // For simplicity, use left_chunks for now
                left_chunks
            }
        };

        // BOINC Task Information (enhanced)
        self.render_boinc_panel(f, layout[0]);

        // System Resources (enhanced)
        self.render_system_panel(f, layout[1]);

        // Performance Charts or Summary
        if self.layout_manager.layout_mode == LayoutMode::Compact {
            self.render_mini_charts(f, layout[2]);
        } else {
            self.render_performance_summary(f, layout[2]);
        }
    }

    /// Render enhanced BOINC task panel
    fn render_boinc_panel(&self, f: &mut Frame, area: Rect) {
        if let Some(metrics) = self.performance_monitor.get_current_metrics() {
            if let Some(ref boinc) = metrics.boinc_task {
                let boinc_info = vec![
                    Line::from(vec![
                        Span::styled(
                            "Task: ",
                            Style::default().fg(self.theme_manager.current_theme.colors.accent),
                        ),
                        Span::raw(&boinc.task_name),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "Project: ",
                            Style::default().fg(self.theme_manager.current_theme.colors.accent),
                        ),
                        Span::raw("MilkyWay@Home"),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "Progress: ",
                            Style::default().fg(self.theme_manager.current_theme.colors.accent),
                        ),
                        Span::raw(format!("{:.2}%", boinc.fraction_done * 100.0)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "CPU Time: ",
                            Style::default().fg(self.theme_manager.current_theme.colors.accent),
                        ),
                        Span::raw(format_duration(boinc.cpu_time)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "Elapsed: ",
                            Style::default().fg(self.theme_manager.current_theme.colors.accent),
                        ),
                        Span::raw(format_duration(boinc.elapsed_time)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "FLOPS Rate: ",
                            Style::default().fg(self.theme_manager.current_theme.colors.accent),
                        ),
                        Span::raw(format_flops(boinc.current_flops_rate)),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "Memory Peak: ",
                            Style::default().fg(self.theme_manager.current_theme.colors.accent),
                        ),
                        Span::raw(format_bytes(boinc.peak_memory)),
                    ]),
                ];

                let boinc_block = Paragraph::new(boinc_info)
                    .block(Block::default().borders(Borders::ALL).title("BOINC Task"))
                    .wrap(Wrap { trim: true });
                f.render_widget(boinc_block, area);

                // Enhanced progress visualization
                if area.height > 8 {
                    let progress_area = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(100)])
                        .split(area.inner(&Margin {
                            vertical: 7,
                            horizontal: 1,
                        }))[0];

                    let progress = Gauge::default()
                        .block(Block::default().borders(Borders::NONE))
                        .gauge_style(
                            Style::default().fg(self.theme_manager.current_theme.colors.success),
                        )
                        .percent((boinc.fraction_done * 100.0) as u16)
                        .label(format!("{:.1}%", boinc.fraction_done * 100.0));
                    f.render_widget(progress, progress_area);
                }
            } else {
                let no_task = Paragraph::new(vec![
                    Line::from("No BOINC task active"),
                    Line::from(""),
                    Line::from("Waiting for work assignment..."),
                    Line::from("Check oracle connection and BOINC client status"),
                ])
                .block(Block::default().borders(Borders::ALL).title("BOINC Task"))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
                f.render_widget(no_task, area);
            }
        }
    }

    /// Render enhanced system resources panel
    fn render_system_panel(&self, f: &mut Frame, area: Rect) {
        if let Some(metrics) = self.performance_monitor.get_current_metrics() {
            let system_info = vec![
                Line::from(vec![
                    Span::styled(
                        "CPU Usage: ",
                        Style::default().fg(self.theme_manager.current_theme.colors.accent),
                    ),
                    Span::raw(format!(
                        "{:.1}% ({} cores)",
                        metrics.system.cpu_usage, metrics.system.cpu_cores
                    )),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Memory: ",
                        Style::default().fg(self.theme_manager.current_theme.colors.accent),
                    ),
                    Span::raw(format!(
                        "{} / {} ({:.1}%)",
                        format_bytes(metrics.system.memory_used),
                        format_bytes(metrics.system.memory_total),
                        metrics.system.memory_percentage
                    )),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Load Average: ",
                        Style::default().fg(self.theme_manager.current_theme.colors.accent),
                    ),
                    Span::raw(format!("{:.2}", metrics.system.load_average)),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Efficiency: ",
                        Style::default().fg(self.theme_manager.current_theme.colors.accent),
                    ),
                    Span::raw(format!(
                        "CPU {:.1}% | Memory {:.1}%",
                        metrics.performance.cpu_efficiency, metrics.performance.memory_efficiency
                    )),
                ]),
            ];

            let system_block = Paragraph::new(system_info)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("System Resources"),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(system_block, area);
        }
    }

    /// Render mini performance charts for compact mode
    fn render_mini_charts(&self, f: &mut Frame, area: Rect) {
        let chart_data = self.prepare_chart_data();

        let chart = Chart::new(chart_data.datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Performance (Last 5min)"),
            )
            .x_axis(
                Axis::default()
                    .title("Time")
                    .bounds([0.0, 60.0])
                    .labels(vec!["-5m".to_string().into(), "Now".to_string().into()]),
            )
            .y_axis(
                Axis::default()
                    .title("Usage %")
                    .bounds([0.0, 100.0])
                    .labels(vec![
                        "0".to_string().into(),
                        "50".to_string().into(),
                        "100".to_string().into(),
                    ]),
            );

        f.render_widget(chart, area);
    }

    /// Render performance summary
    fn render_performance_summary(&self, f: &mut Frame, area: Rect) {
        if let Some(metrics) = self.performance_monitor.get_current_metrics() {
            let perf_info = vec![
                Line::from(vec![
                    Span::styled(
                        "Current FLOPS: ",
                        Style::default().fg(self.theme_manager.current_theme.colors.success),
                    ),
                    Span::raw(format_flops(metrics.performance.flops_per_second)),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Average FLOPS: ",
                        Style::default().fg(self.theme_manager.current_theme.colors.success),
                    ),
                    Span::raw(format_flops(metrics.performance.avg_flops_per_hour)),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Work Units/Hour: ",
                        Style::default().fg(self.theme_manager.current_theme.colors.success),
                    ),
                    Span::raw(format!("{:.2}", metrics.performance.work_units_per_hour)),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Est. Completion: ",
                        Style::default().fg(self.theme_manager.current_theme.colors.info),
                    ),
                    Span::raw(
                        metrics
                            .performance
                            .estimated_completion
                            .map(|t| t.format("%H:%M:%S UTC").to_string())
                            .unwrap_or_else(|| "Unknown".to_string()),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Power Efficiency: ",
                        Style::default().fg(self.theme_manager.current_theme.colors.warning),
                    ),
                    Span::raw(
                        metrics
                            .performance
                            .power_efficiency
                            .map(|p| format!("{:.2} FLOPS/W", p))
                            .unwrap_or_else(|| "Not available".to_string()),
                    ),
                ]),
            ];

            let perf_block = Paragraph::new(perf_info)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Performance Summary"),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(perf_block, area);
        }
    }

    /// Render real-time performance charts
    fn render_performance(&mut self, f: &mut Frame, area: Rect) {
        let chart_data = self.prepare_chart_data();

        let chart = Chart::new(chart_data.datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Performance Charts"),
            )
            .x_axis(
                Axis::default()
                    .title("Time (minutes ago)")
                    .bounds(chart_data.x_bounds)
                    .labels(vec![
                        "-10".to_string().into(),
                        "-5".to_string().into(),
                        "0".to_string().into(),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .title("Usage %")
                    .bounds([0.0, 100.0])
                    .labels(vec![
                        "0".to_string().into(),
                        "25".to_string().into(),
                        "50".to_string().into(),
                        "75".to_string().into(),
                        "100".to_string().into(),
                    ]),
            );

        f.render_widget(chart, area);
    }

    /// Prepare chart data from historical metrics
    fn prepare_chart_data(&self) -> ChartData {
        let mut cpu_data = Vec::new();
        let mut memory_data = Vec::new();

        // Convert historical data to chart points
        for (timestamp, cpu_usage) in self.historical_data.cpu_history.iter() {
            let minutes_ago = (Utc::now() - *timestamp).num_seconds() as f64 / 60.0;
            cpu_data.push((minutes_ago, *cpu_usage as f64));
        }

        for (timestamp, memory_usage) in self.historical_data.memory_history.iter() {
            let minutes_ago = (Utc::now() - *timestamp).num_seconds() as f64 / 60.0;
            memory_data.push((minutes_ago, *memory_usage as f64));
        }

        // Create static references for the datasets
        let cpu_data_static: &'static [(f64, f64)] = Box::leak(cpu_data.into_boxed_slice());
        let memory_data_static: &'static [(f64, f64)] = Box::leak(memory_data.into_boxed_slice());

        let datasets = vec![
            Dataset::default()
                .name("CPU Usage")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Cyan))
                .data(cpu_data_static),
            Dataset::default()
                .name("Memory Usage")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Yellow))
                .data(memory_data_static),
        ];

        ChartData {
            datasets,
            x_bounds: [-10.0, 0.0],
        }
    }

    /// Render configuration management interface
    fn render_configuration(&mut self, f: &mut Frame, area: Rect) {
        let config_options = vec![
            "Oracle URL",
            "BOINC Install Directory",
            "BOINC Data Directory",
            "User ID",
            "Rate Limit (req/min)",
            "Debug Mode",
            "HTTPS Required",
            "Certificate Verification",
            "NUW on CPU",
            "BOINC on GPU",
            "NUW CPU %",
            "BOINC GPU %",
            "NUW On Demand",
            "Min NUW Difficulty",
            "Max BOINC Tasks",
        ];

        let mut list_items = Vec::new();
        for (i, option) in config_options.iter().enumerate() {
            let style = if i == self.config_editor.active_field {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };

            list_items.push(ListItem::new(*option).style(style));
        }

        let config_list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Configuration (Ctrl+C to edit)"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        f.render_stateful_widget(
            config_list,
            area,
            &mut ListState::default().with_selected(Some(self.config_editor.active_field)),
        );
    }

    /// Render alerts panel
    fn render_alerts(&self, f: &mut Frame, area: Rect) {
        let mut alert_items = Vec::new();

        for alert in &self.alert_system.active_alerts {
            let (icon, color) = match alert.level {
                AlertLevel::Info => ("ℹ", Color::Blue),
                AlertLevel::Warning => ("⚠", Color::Yellow),
                AlertLevel::Error => ("✗", Color::Red),
                AlertLevel::Critical => ("🚨", Color::Red),
            };

            let style = Style::default().fg(color);
            let ack_status = if alert.acknowledged { "✓" } else { "○" };

            alert_items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", icon), style),
                Span::styled(&alert.title, style.add_modifier(Modifier::BOLD)),
                Span::raw(format!(
                    " {} {}",
                    ack_status,
                    alert.timestamp.format("%H:%M")
                )),
            ])));
        }

        if alert_items.is_empty() {
            alert_items
                .push(ListItem::new("No active alerts").style(Style::default().fg(Color::Green)));
        }

        let alerts_list = List::new(alert_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Active Alerts"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        f.render_widget(alerts_list, area);
    }

    /// Render enhanced logs panel
    fn render_logs(&mut self, f: &mut Frame, area: Rect) {
        if let Ok(logs) = self.log_messages.lock() {
            let log_items: Vec<ListItem> = logs
                .iter()
                .rev() // Show newest first
                .map(|log| {
                    let (level_color, level_icon) = match log.level.as_str() {
                        "ERROR" => (Color::Red, "✗"),
                        "WARN" => (Color::Yellow, "⚠"),
                        "INFO" => (Color::Green, "ℹ"),
                        "DEBUG" => (Color::Blue, "🔍"),
                        _ => (Color::White, "•"),
                    };

                    let category_color = match log.category {
                        LogCategory::System => Color::Cyan,
                        LogCategory::Boinc => Color::Green,
                        LogCategory::Network => Color::Blue,
                        LogCategory::Performance => Color::Yellow,
                        LogCategory::Security => Color::Red,
                        LogCategory::Config => Color::Magenta,
                        LogCategory::General => Color::White,
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("[{}] ", log.timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::styled(format!("{} ", level_icon), Style::default().fg(level_color)),
                        Span::styled(
                            format!("[{}] ", log.source),
                            Style::default().fg(category_color),
                        ),
                        Span::styled(
                            format!("[{}] ", log.level),
                            Style::default()
                                .fg(level_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(&log.message),
                    ]))
                })
                .collect();

            let logs_list = List::new(log_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Logs (↑/↓ to scroll, PgUp/PgDn for pages, Home/End)"),
                )
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol(">> ");

            f.render_stateful_widget(logs_list, area, &mut self.log_scroll_state);
        }
    }

    /// Render footer with keyboard shortcuts
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let shortcuts = match self.current_tab {
            0 => "1-5:Tabs | Tab:Next | Shift+Tab:Prev | ?:Help | q:Quit",
            1 => "1-5:Tabs | Tab:Next | Shift+Tab:Prev | ?:Help | q:Quit",
            2 => "1-5:Tabs | Tab:Next | Shift+Tab:Prev | Ctrl+C:Edit | ?:Help | q:Quit",
            3 => "1-5:Tabs | Tab:Next | Shift+Tab:Prev | ?:Help | q:Quit",
            4 => "↑/↓:Scroll | PgUp/PgDn:Page | Home/End:Top/Bottom | ?:Help | q:Quit",
            _ => "?:Help | q:Quit",
        };

        let footer = Paragraph::new(shortcuts)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(footer, area);
    }

    /// Render help overlay
    fn render_help_overlay(&self, f: &mut Frame, area: Rect) {
        let help_text = match self.help_system.current_topic {
            HelpTopic::General => vec![
                Line::from("Chert Miner TUI Help"),
                Line::from(""),
                Line::from("Navigation:"),
                Line::from("  Tab/Shift+Tab: Switch tabs"),
                Line::from("  1-5: Jump to specific tab"),
                Line::from("  q: Quit application"),
                Line::from(""),
                Line::from("Dashboard: Real-time mining status"),
                Line::from("Performance: Charts and analytics"),
                Line::from("Configuration: Settings management"),
                Line::from("Alerts: System notifications"),
                Line::from("Logs: Detailed event history"),
            ],
            HelpTopic::Navigation => vec![
                Line::from("Navigation Help"),
                Line::from(""),
                Line::from("Tab Navigation:"),
                Line::from("  Tab: Next tab"),
                Line::from("  Shift+Tab: Previous tab"),
                Line::from("  1-5: Direct tab selection"),
                Line::from(""),
                Line::from("Log Navigation:"),
                Line::from("  ↑/↓: Scroll lines"),
                Line::from("  PgUp/PgDn: Scroll pages"),
                Line::from("  Home/End: Jump to top/bottom"),
            ],
            HelpTopic::Configuration => vec![
                Line::from("Configuration Help"),
                Line::from(""),
                Line::from("  Ctrl+C: Enter/exit edit mode"),
                Line::from("  ↑/↓: Navigate fields"),
                Line::from("  Enter: Save changes"),
                Line::from("  Esc: Cancel editing"),
                Line::from(""),
                Line::from("Changes take effect after restart"),
            ],
            HelpTopic::Alerts => vec![
                Line::from("Alerts Help"),
                Line::from(""),
                Line::from("Alert Levels:"),
                Line::from("  ℹ Info: General information"),
                Line::from("  ⚠ Warning: Potential issues"),
                Line::from("  ✗ Error: Errors requiring attention"),
                Line::from("  🚨 Critical: Immediate action needed"),
                Line::from(""),
                Line::from("○ Unacknowledged | ✓ Acknowledged"),
            ],
            HelpTopic::Charts => vec![
                Line::from("Charts Help"),
                Line::from(""),
                Line::from("Performance Charts:"),
                Line::from("  Cyan: CPU usage over time"),
                Line::from("  Yellow: Memory usage over time"),
                Line::from("  X-axis: Time (minutes ago)"),
                Line::from("  Y-axis: Usage percentage"),
                Line::from(""),
                Line::from("Data retained for last 10 minutes"),
            ],
        };

        let help_block = Block::default()
            .borders(Borders::ALL)
            .title("Help (Esc to close)")
            .style(Style::default().bg(Color::Black));

        let help_paragraph = Paragraph::new(help_text)
            .block(help_block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        let help_area = centered_rect(60, 80, area);
        f.render_widget(Clear, help_area);
        f.render_widget(help_paragraph, help_area);
    }

    /// Render configuration editor overlay
    fn render_config_editor(&mut self, f: &mut Frame, area: Rect) {
        let editor_area = centered_rect(70, 60, area);

        let config_fields = [
            ("Oracle URL", "oracle_url"),
            ("BOINC Install Dir", "boinc_install_dir"),
            ("BOINC Data Dir", "boinc_data_dir"),
            ("User ID", "user_id"),
            ("Rate Limit", "rate_limit"),
            ("Debug Mode", "debug_mode"),
            ("HTTPS Required", "https_required"),
            ("Cert Verification", "cert_verification"),
        ];
        let work_fields = vec![
            ("NUW on CPU", "nuw_on_cpu"),
            ("BOINC on GPU", "boinc_on_gpu"),
            ("NUW CPU %", "nuw_cpu_percentage"),
            ("BOINC GPU %", "boinc_gpu_percentage"),
            ("NUW On Demand", "nuw_on_demand"),
            ("Min NUW Difficulty", "min_nuw_difficulty"),
            ("Max BOINC Tasks", "max_boinc_tasks"),
        ];

        let mut field_lines = Vec::with_capacity(config_fields.len() + work_fields.len() + 6);
        for (i, (label, field)) in config_fields.iter().enumerate() {
            let value = self
                .config_editor
                .temp_values
                .get(*field)
                .map(|s| s.as_str())
                .unwrap_or("default");

            let style = if i == self.config_editor.active_field {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            field_lines.push(Line::from(vec![
                Span::styled(format!("{}: ", label), Style::default().fg(Color::Cyan)),
                Span::styled(value, style),
            ]));
        }

        if !work_fields.is_empty() {
            field_lines.push(Line::from(""));
            field_lines.push(Line::from(Span::styled(
                "Work Allocation:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));

            for (label, field) in work_fields {
                let value = self
                    .config_editor
                    .temp_values
                    .get(field)
                    .map(|s| s.as_str())
                    .unwrap_or("default");

                field_lines.push(Line::from(vec![
                    Span::styled(format!("{}: ", label), Style::default().fg(Color::Cyan)),
                    Span::styled(value, Style::default().fg(Color::White)),
                ]));
            }
        }

        if !self.config_editor.validation_errors.is_empty() {
            field_lines.push(Line::from(""));
            field_lines.push(Line::from(Span::styled(
                "Validation Errors:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            for error in &self.config_editor.validation_errors {
                field_lines.push(Line::from(Span::styled(
                    format!("• {}", error),
                    Style::default().fg(Color::Red),
                )));
            }
        }

        let editor_block = Block::default()
            .borders(Borders::ALL)
            .title("Configuration Editor (Enter: Save, Esc: Cancel)")
            .style(Style::default().bg(Color::Black));

        let editor_paragraph = Paragraph::new(field_lines)
            .block(editor_block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        f.render_widget(Clear, editor_area);
        f.render_widget(editor_paragraph, editor_area);
    }

    /// Add enhanced log message
    pub fn add_log_message(
        &mut self,
        level: String,
        message: String,
        category: LogCategory,
        source: String,
    ) {
        let log_msg = LogMessage {
            timestamp: Utc::now(),
            level,
            message,
            category,
            source,
        };

        if let Ok(mut logs) = self.log_messages.lock() {
            logs.push_back(log_msg);
            // Keep only last 1000 messages
            if logs.len() > 1000 {
                logs.pop_front();
            }
        }
    }

    /// Enhanced log scrolling functions
    fn scroll_logs_up(&mut self) {
        let i = match self.log_scroll_state.selected() {
            Some(i) => {
                if i == 0 {
                    if let Ok(logs) = self.log_messages.lock() {
                        logs.len().saturating_sub(1)
                    } else {
                        0
                    }
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.log_scroll_state.select(Some(i));
    }

    fn scroll_logs_down(&mut self) {
        let i = match self.log_scroll_state.selected() {
            Some(i) => {
                if let Ok(logs) = self.log_messages.lock() {
                    if i >= logs.len().saturating_sub(1) {
                        0
                    } else {
                        i + 1
                    }
                } else {
                    0
                }
            }
            None => 0,
        };
        self.log_scroll_state.select(Some(i));
    }

    fn scroll_logs_page_up(&mut self) {
        let page_size = 10; // Scroll 10 lines at a time
        let i = match self.log_scroll_state.selected() {
            Some(i) => i.saturating_sub(page_size),
            None => 0,
        };
        self.log_scroll_state.select(Some(i));
    }

    fn scroll_logs_page_down(&mut self) {
        let page_size = 10;
        let i = match self.log_scroll_state.selected() {
            Some(i) => {
                if let Ok(logs) = self.log_messages.lock() {
                    (i + page_size).min(logs.len().saturating_sub(1))
                } else {
                    i + page_size
                }
            }
            None => page_size,
        };
        self.log_scroll_state.select(Some(i));
    }

    fn scroll_logs_to_top(&mut self) {
        self.log_scroll_state.select(Some(0));
    }

    fn scroll_logs_to_bottom(&mut self) {
        if let Ok(logs) = self.log_messages.lock() {
            self.log_scroll_state
                .select(Some(logs.len().saturating_sub(1)));
        }
    }

    /// Update historical data for charts
    fn update_historical_data(&mut self, metrics: &crate::performance_monitor::MetricsSnapshot) {
        let now = Utc::now();

        // Add CPU data
        self.historical_data
            .cpu_history
            .push_back((now, metrics.system.cpu_usage));
        if self.historical_data.cpu_history.len() > self.historical_data.max_points {
            self.historical_data.cpu_history.pop_front();
        }

        // Add memory data
        self.historical_data
            .memory_history
            .push_back((now, metrics.system.memory_percentage));
        if self.historical_data.memory_history.len() > self.historical_data.max_points {
            self.historical_data.memory_history.pop_front();
        }

        // Add FLOPS data
        self.historical_data
            .flops_history
            .push_back((now, metrics.performance.flops_per_second));
        if self.historical_data.flops_history.len() > self.historical_data.max_points {
            self.historical_data.flops_history.pop_front();
        }
    }

    /// Check for alerts based on metrics
    fn check_alerts(&mut self, metrics: &crate::performance_monitor::MetricsSnapshot) {
        // CPU usage alert
        if metrics.system.cpu_usage > self.alert_system.thresholds.cpu_usage_high {
            self.alert_system.add_alert(
                "high_cpu_usage".to_string(),
                AlertLevel::Warning,
                "High CPU Usage".to_string(),
                format!(
                    "CPU usage is {:.1}%, above threshold of {:.1}%",
                    metrics.system.cpu_usage, self.alert_system.thresholds.cpu_usage_high
                ),
            );
        }

        // Memory usage alert
        if metrics.system.memory_percentage > self.alert_system.thresholds.memory_usage_high {
            self.alert_system.add_alert(
                "high_memory_usage".to_string(),
                AlertLevel::Warning,
                "High Memory Usage".to_string(),
                format!(
                    "Memory usage is {:.1}%, above threshold of {:.1}%",
                    metrics.system.memory_percentage,
                    self.alert_system.thresholds.memory_usage_high
                ),
            );
        }

        // BOINC progress stalled alert
        if let Some(ref boinc) = metrics.boinc_task {
            if (boinc.fraction_done * 100.0 - self.last_boinc_progress).abs() < 0.01 {
                // Progress hasn't changed significantly
                self.alert_system.add_alert(
                    "boinc_progress_stalled".to_string(),
                    AlertLevel::Info,
                    "BOINC Progress Stalled".to_string(),
                    "BOINC task progress has not changed recently".to_string(),
                );
            }
            self.last_boinc_progress = boinc.fraction_done * 100.0;
        }
    }

    /// Update alerts (remove old ones, etc.)
    fn update_alerts(&mut self) {
        // Remove alerts older than 1 hour
        let one_hour_ago = Utc::now() - Duration::hours(1);
        self.alert_system
            .active_alerts
            .retain(|alert| alert.timestamp > one_hour_ago);
    }

    /// Get shared log message queue for external logging
    pub fn get_log_queue(&self) -> Arc<Mutex<VecDeque<LogMessage>>> {
        Arc::clone(&self.log_messages)
    }
}

/// Chart data structure
struct ChartData {
    datasets: Vec<Dataset<'static>>,
    x_bounds: [f64; 2],
}

/// Helper function to center a rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
