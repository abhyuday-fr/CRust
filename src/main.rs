mod ffi;
mod telemetry;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use std::{
    collections::VecDeque,
    io,
    time::{Duration, Instant},
};

// Catppuccin Macchiato palette
// Every named color from the spec  use these constants everywhere,
// never raw hex strings scattered through draw functions.
mod cat {
    use ratatui::style::Color;

    // Backgrounds
    pub const BASE: Color = Color::Rgb(36, 39, 58); // #24273a
    pub const MANTLE: Color = Color::Rgb(30, 32, 48); // #1e2030
    pub const CRUST: Color = Color::Rgb(24, 25, 38); // #181926

    // Surfaces (borders, inactive elements)
    pub const SURFACE0: Color = Color::Rgb(54, 58, 79); // #363a4f
    pub const SURFACE1: Color = Color::Rgb(73, 77, 100); // #494d64
    pub const SURFACE2: Color = Color::Rgb(91, 96, 120); // #5b6078

    // Text hierarchy
    pub const TEXT: Color = Color::Rgb(202, 211, 245); // #cad3f5
    pub const SUBTEXT1: Color = Color::Rgb(184, 192, 224); // #b8c0e0
    pub const SUBTEXT0: Color = Color::Rgb(165, 173, 203); // #a5adcb
    pub const OVERLAY2: Color = Color::Rgb(147, 154, 183); // #939ab7
    pub const OVERLAY1: Color = Color::Rgb(110, 115, 141); // #6e738d
    pub const OVERLAY0: Color = Color::Rgb(110, 115, 141); // #6e738d

    // Accent colors
    pub const ROSEWATER: Color = Color::Rgb(244, 219, 214); // #f4dbd6
    pub const FLAMINGO: Color = Color::Rgb(240, 198, 198); // #f0c6c6
    pub const PINK: Color = Color::Rgb(245, 189, 230); // #f5bde6
    pub const MAUVE: Color = Color::Rgb(198, 160, 246); // #c6a0f6  ← the violet
    pub const RED: Color = Color::Rgb(237, 135, 150); // #ed8796
    pub const MAROON: Color = Color::Rgb(238, 153, 160); // #ee99a0
    pub const PEACH: Color = Color::Rgb(245, 169, 127); // #f5a97f
    pub const YELLOW: Color = Color::Rgb(238, 212, 159); // #eed49f
    pub const GREEN: Color = Color::Rgb(166, 218, 149); // #a6da95
    pub const TEAL: Color = Color::Rgb(139, 213, 202); // #8bd5ca
    pub const SKY: Color = Color::Rgb(145, 215, 227); // #91d7e3
    pub const SAPPHIRE: Color = Color::Rgb(125, 196, 228); // #7dc4e4
    pub const BLUE: Color = Color::Rgb(138, 173, 244); // #8aadf4
    pub const LAVENDER: Color = Color::Rgb(183, 189, 248); // #b7bdf8
}

// App state
struct App {
    frame_count: u64,
    signal_history: VecDeque<u64>,
    log: VecDeque<String>,
    log_scroll: u16,
    last_frame: Option<telemetry::TelemetryFrame>,
}

impl App {
    fn new() -> Self {
        let mut log = VecDeque::new();
        log.push_front("[BOOT] Ground station online".to_string());
        Self {
            frame_count: 0,
            signal_history: VecDeque::with_capacity(32),
            log,
            log_scroll: 0,
            last_frame: None,
        }
    }

    fn update(&mut self) {
        match ffi::get_telemetry_frame() {
            Err(e) => self.log.push_front(format!("[ERR ] {e}")),
            Ok(raw) => match telemetry::parse(&raw) {
                Err(e) => self.log.push_front(format!("[ERR ] {e}")),
                Ok(frame) => {
                    self.frame_count += 1;
                    let sig = frame.signal_percent() as u64;
                    let valid = frame.is_valid();
                    let altitude_m = frame.altitude_m;
                    let (h, m, s) = frame.elapsed_time();

                    if self.signal_history.len() == 32 {
                        self.signal_history.pop_back();
                    }
                    self.signal_history.push_front(sig);

                    self.log.push_front(format!(
                        "[{:02}:{:02}:{:02}] Frame #{} : alt {}m , {}",
                        h,
                        m,
                        s,
                        self.frame_count,
                        altitude_m,
                        if valid { "OK" } else { "FAIL" }
                    ));
                    if self.log.len() > 64 {
                        self.log.pop_back();
                    }
                    self.last_frame = Some(frame);
                }
            },
        }
    }
}

// Entry point
fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

// Main loop
fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();
    let tick_rate = Duration::from_millis(500);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw(f, &app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),
                    KeyCode::Down => app.log_scroll = app.log_scroll.saturating_add(1),
                    KeyCode::Up => app.log_scroll = app.log_scroll.saturating_sub(1),
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.update();
            last_tick = Instant::now();
        }
    }
}

// Drawing
fn draw(f: &mut Frame, app: &App) {
    // Dark Macchiato base fills the whole terminal
    f.render_widget(Block::new().style(Style::new().bg(cat::BASE)), f.area());

    let root = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .split(f.area());

    draw_header(f, app, root[0]);

    let body = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(10),
        Constraint::Min(4),
    ])
    .split(root[1]);

    draw_gauges(f, app, body[0]);
    draw_data_row(f, app, body[1]);
    draw_log(f, app, body[2]);
    draw_footer(f, root[2]);
}

// Shared block style
// All panels use the same Macchiato-themed border — one place to change it.
fn panel(title: &str) -> Block {
    Block::bordered()
        .border_style(Style::new().fg(cat::SURFACE1))
        .title(Span::styled(title, Style::new().fg(cat::MAUVE).bold()))
        .style(Style::new().bg(cat::BASE))
}

// Header
fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let (elapsed_text, frame_text) = if let Some(frame) = &app.last_frame {
        let (h, m, s) = frame.elapsed_time();
        (
            format!(" {:02}:{:02}:{:02} MET ", h, m, s),
            format!(" Frame #{} ", app.frame_count),
        )
    } else {
        (" --:--:-- MET ".to_string(), " Waiting... ".to_string())
    };

    let title = Line::from(vec![
        Span::styled(" ■ CRUST ", Style::new().fg(cat::MAUVE).bold()),
        Span::styled("Satellite Ground Station", Style::new().fg(cat::SUBTEXT1)),
    ]);

    let header = Paragraph::new(title).block(
        Block::bordered()
            .border_style(Style::new().fg(cat::SURFACE1))
            .style(Style::new().bg(cat::BASE))
            .title_bottom(
                Line::from(vec![
                    Span::styled(elapsed_text, Style::new().fg(cat::GREEN)),
                    Span::styled(frame_text, Style::new().fg(cat::OVERLAY1)),
                ])
                .right_aligned(),
            ),
    );
    f.render_widget(header, area);
}

// Gauges
fn draw_gauges(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Percentage(40),
        Constraint::Percentage(20),
    ])
    .split(area);

    let (batt_pct, batt_label, sig_pct, sig_label, flags) = if let Some(frame) = &app.last_frame {
        let bp = frame.battery_percent() as u16;
        let sp = frame.signal_percent() as u16;
        (
            bp,
            format!("{:.3}V  ({}%)", frame.battery_volts(), bp),
            sp,
            format!("{}%  RSSI:{}", sp, frame.signal_strength),
            frame.status_summary(),
        )
    } else {
        (0, "---".into(), 0, "---".into(), vec![])
    };

    // Battery: green → peach → red as it drains
    let batt_color = match batt_pct {
        76..=100 => cat::GREEN,
        26..=75 => cat::PEACH,
        _ => cat::RED,
    };
    f.render_widget(
        Gauge::default()
            .block(panel(" Battery "))
            .gauge_style(Style::new().fg(batt_color).bg(cat::SURFACE0))
            .percent(batt_pct)
            .label(Span::styled(batt_label, Style::new().fg(cat::TEXT).bold())),
        cols[0],
    );

    // Signal: teal -> yellow -> maroon
    let sig_color = match sig_pct {
        61..=100 => cat::TEAL,
        31..=60 => cat::YELLOW,
        _ => cat::MAROON,
    };
    f.render_widget(
        Gauge::default()
            .block(panel(" Signal "))
            .gauge_style(Style::new().fg(sig_color).bg(cat::SURFACE0))
            .percent(sig_pct)
            .label(Span::styled(sig_label, Style::new().fg(cat::TEXT).bold())),
        cols[1],
    );

    // Status flags panel
    let flag_lines: Vec<Line> = if flags.is_empty() {
        vec![Line::from(Span::styled(
            "  No active flags",
            Style::new().fg(cat::OVERLAY1),
        ))]
    } else {
        flags
            .iter()
            .map(|flag| {
                let (icon, color) = if flag.contains("ERROR") {
                    ("✗ ", cat::RED)
                } else if flag.contains("BATTERY") {
                    ("⚡ ", cat::PEACH)
                } else {
                    ("✓ ", cat::GREEN)
                };
                Line::from(vec![
                    Span::styled(format!("  {icon}"), Style::new().fg(color)),
                    Span::styled(flag.to_string(), Style::new().fg(cat::TEXT)),
                ])
            })
            .collect()
    };
    f.render_widget(Paragraph::new(flag_lines).block(panel(" Status ")), cols[2]);
}

// Data row
fn draw_data_row(f: &mut Frame, app: &App, area: Rect) {
    let cols =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).split(area);

    draw_telemetry_panel(f, app, cols[0]);
    draw_sparkline(f, app, cols[1]);
}

fn draw_telemetry_panel(f: &mut Frame, app: &App, area: Rect) {
    // helper: one label+value row
    fn row<'a>(label: &'a str, value: String, value_color: Color) -> Line<'a> {
        Line::from(vec![
            Span::styled(format!("  {label:<13}"), Style::new().fg(cat::OVERLAY2)),
            Span::styled(value, Style::new().fg(value_color)),
        ])
    }

    let lines = if let Some(frame) = &app.last_frame {
        let altitude_m = frame.altitude_m;
        let status_flags = frame.status_flags;
        vec![
            row(
                "Temperature",
                format!("{:.2} °C", frame.temperature_celsius()),
                cat::PEACH,
            ),
            row("Altitude", format!("{} m", altitude_m), cat::SKY),
            row(
                "Velocity",
                format!("{:.2} m/s", frame.velocity_metres_per_sec()),
                cat::SAPPHIRE,
            ),
            row(
                "Battery",
                format!("{:.3} V", frame.battery_volts()),
                cat::GREEN,
            ),
            row("Flags", format!("0b{:08b}", status_flags), cat::MAUVE),
            row(
                "Checksum",
                if frame.is_valid() {
                    "OK".into()
                } else {
                    "FAIL".into()
                },
                if frame.is_valid() {
                    cat::GREEN
                } else {
                    cat::RED
                },
            ),
        ]
    } else {
        vec![Line::from(Span::styled(
            "  Awaiting first frame...",
            Style::new().fg(cat::OVERLAY1),
        ))]
    };

    f.render_widget(Paragraph::new(lines).block(panel(" Telemetry ")), area);
}

fn draw_sparkline(f: &mut Frame, app: &App, area: Rect) {
    let data: Vec<u64> = app.signal_history.iter().copied().rev().collect();
    f.render_widget(
        Sparkline::default()
            .block(panel(" Signal history (%) "))
            .data(&data)
            .max(100)
            .style(Style::new().fg(cat::MAUVE).bg(cat::BASE)),
        area,
    );
}

// Log
fn draw_log(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .log
        .iter()
        .enumerate()
        .map(|(i, line)| {
            // Most recent entry gets full text color, older ones fade through the overlay ramp
            let color = match i {
                0 => cat::SUBTEXT1,
                1 => cat::OVERLAY2,
                2 => cat::OVERLAY1,
                _ => cat::SURFACE2,
            };
            ListItem::new(Span::styled(format!("  {line}"), Style::new().fg(color)))
        })
        .collect();

    f.render_stateful_widget(
        List::new(items).block(panel(" Event log  ↑↓ scroll ")),
        area,
        &mut ListState::default().with_offset(app.log_scroll as usize),
    );
}

// Footer
fn draw_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" q ", Style::new().fg(cat::BASE).bg(cat::MAUVE).bold()),
        Span::styled(" quit   ", Style::new().fg(cat::SUBTEXT0)),
        Span::styled(" ↑↓ ", Style::new().fg(cat::BASE).bg(cat::SURFACE2).bold()),
        Span::styled(" scroll log", Style::new().fg(cat::SUBTEXT0)),
    ]))
    .block(
        Block::bordered()
            .border_style(Style::new().fg(cat::SURFACE1))
            .style(Style::new().bg(cat::BASE)),
    );
    f.render_widget(footer, area);
}
