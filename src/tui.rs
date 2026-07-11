use crate::{
    ports::{PortEntry, list_ports},
    process::{force_kill_pid, is_pid_running, terminate_pid},
};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Row, Table, TableState, Wrap},
};
use std::{io, time::Duration};

type Backend = CrosstermBackend<io::Stdout>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Search,
    Details,
    ConfirmKill,
    ConfirmForceKill,
}

pub struct App {
    entries: Vec<PortEntry>,
    filtered: Vec<PortEntry>,
    selected: usize,
    search: String,
    old_search: String,
    mode: Mode,
    status: String,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            entries: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            search: String::new(),
            old_search: String::new(),
            mode: Mode::Normal,
            status: String::new(),
            should_quit: false,
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        match list_ports() {
            Ok(entries) => {
                self.entries = entries;
                self.apply_filter();
                self.status = format!("{} listening process(es)", self.entries.len());
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn apply_filter(&mut self) {
        let query = self.search.to_lowercase();
        self.filtered = self
            .entries
            .iter()
            .filter(|entry| {
                query.is_empty()
                    || entry.port.to_string().contains(&query)
                    || entry.pid.to_string().contains(&query)
                    || entry.process_name.to_lowercase().contains(&query)
                    || entry
                        .command
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
            })
            .cloned()
            .collect();
        self.selected = self.selected.min(self.filtered.len().saturating_sub(1));
    }

    fn selected(&self) -> Option<&PortEntry> {
        self.filtered.get(self.selected)
    }

    fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    fn confirm_kill(&mut self) {
        let Some(entry) = self.selected().cloned() else {
            return;
        };

        match terminate_pid(entry.pid) {
            Ok(()) => {
                self.status = format!("Sent SIGTERM to {} PID {}", entry.process_name, entry.pid);
                if is_pid_running(entry.pid) {
                    self.mode = Mode::ConfirmForceKill;
                    return;
                } else {
                    self.refresh();
                }
            }
            Err(error) => {
                self.status = format!("{error}\nTry running with sudo or terminate it manually.")
            }
        }
        self.mode = Mode::Normal;
    }

    fn confirm_force_kill(&mut self) {
        let Some(entry) = self.selected().cloned() else {
            return;
        };

        match force_kill_pid(entry.pid) {
            Ok(()) => {
                self.status = format!("Sent SIGKILL to {} PID {}", entry.process_name, entry.pid);
                self.refresh();
            }
            Err(error) => {
                self.status = format!("{error}\nTry running with sudo or terminate it manually.")
            }
        }
        self.mode = Mode::Normal;
    }
}

pub fn run() -> Result<()> {
    let mut terminal = TerminalGuard::enter()?;
    let mut app = App::new();

    loop {
        terminal.terminal.draw(|frame| draw(frame, &mut app))?;
        if app.should_quit {
            break Ok(());
        }
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
        {
            handle_key(&mut app, key.code);
        }
    }
}

struct TerminalGuard {
    terminal: Terminal<Backend>,
}

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn handle_key(app: &mut App, code: KeyCode) {
    match app.mode {
        Mode::Normal => match code {
            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => app.move_up(),
            KeyCode::Down | KeyCode::Char('j') => app.move_down(),
            KeyCode::Char('r') => app.refresh(),
            KeyCode::Char('/') => {
                app.old_search = app.search.clone();
                app.mode = Mode::Search;
            }
            KeyCode::Enter if app.selected().is_some() => app.mode = Mode::Details,
            KeyCode::Char('x') if app.selected().is_some() => app.mode = Mode::ConfirmKill,
            _ => {}
        },
        Mode::Search => match code {
            KeyCode::Enter => app.mode = Mode::Normal,
            KeyCode::Esc => {
                app.search = app.old_search.clone();
                app.apply_filter();
                app.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                app.search.pop();
                app.apply_filter();
            }
            KeyCode::Char(c) => {
                app.search.push(c);
                app.apply_filter();
            }
            _ => {}
        },
        Mode::Details => match code {
            KeyCode::Esc => app.mode = Mode::Normal,
            KeyCode::Char('x') => app.mode = Mode::ConfirmKill,
            _ => {}
        },
        Mode::ConfirmKill => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_kill(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.mode = Mode::Normal,
            _ => {}
        },
        Mode::ConfirmForceKill => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_force_kill(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.mode = Mode::Normal,
            _ => {}
        },
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    let area = frame.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(2)])
        .split(area);

    let rows = app.filtered.iter().map(|entry| {
        Row::new([
            entry.port.to_string(),
            entry.protocol.to_string(),
            entry.pid.to_string(),
            entry.process_name.clone(),
            entry.address.clone(),
            entry.command.clone().unwrap_or_default(),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Length(8),
            Constraint::Length(16),
            Constraint::Length(18),
            Constraint::Min(12),
        ],
    )
    .header(
        Row::new(["Port", "Protocol", "PID", "Process", "Address", "Command"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .title(format!(
                "Unbind - Local listening ports ({})",
                app.filtered.len()
            ))
            .borders(Borders::ALL),
    )
    .highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = TableState::default();
    if !app.filtered.is_empty() {
        state.select(Some(app.selected));
    }
    frame.render_stateful_widget(table, chunks[0], &mut state);
    if app.filtered.is_empty() {
        frame.render_widget(
            Paragraph::new(if app.search.is_empty() {
                "No local TCP listening ports found."
            } else {
                "No ports match the current search."
            })
            .style(Style::default().fg(Color::DarkGray)),
            inner(chunks[0], 2, 2),
        );
    }

    let help = match app.mode {
        Mode::Search => format!("Search: {}  [Enter] Confirm  [Esc] Cancel", app.search),
        _ => format!(
            "{}  [j/k] Move  [/] Search  [Enter] Details  [x] Kill  [r] Refresh  [q] Quit",
            app.status
        ),
    };
    frame.render_widget(Paragraph::new(help).wrap(Wrap { trim: true }), chunks[1]);

    match app.mode {
        Mode::Details => draw_details(frame, app, centered_rect(60, 35, area)),
        Mode::ConfirmKill => draw_confirm(frame, app, centered_rect(60, 40, area), false),
        Mode::ConfirmForceKill => draw_confirm(frame, app, centered_rect(60, 40, area), true),
        _ => {}
    }
}

fn draw_details(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let Some(entry) = app.selected() else {
        return;
    };
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::raw("Port:      "),
                Span::raw(entry.port.to_string()),
            ]),
            Line::from(vec![
                Span::raw("Protocol:  "),
                Span::raw(entry.protocol.to_string()),
            ]),
            Line::from(vec![Span::raw("Address:   "), Span::raw(&entry.address)]),
            Line::from(vec![
                Span::raw("Process:   "),
                Span::raw(&entry.process_name),
            ]),
            Line::from(vec![
                Span::raw("PID:       "),
                Span::raw(entry.pid.to_string()),
            ]),
            Line::from(vec![
                Span::raw("Command:   "),
                Span::raw(entry.command.as_deref().unwrap_or("")),
            ]),
            Line::from(""),
            Line::from("[Esc] Back  [x] Kill"),
        ])
        .block(Block::default().title("Details").borders(Borders::ALL))
        .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_confirm(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect, force: bool) {
    let Some(entry) = app.selected() else {
        return;
    };
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(if force {
                "Process did not exit. Force kill with SIGKILL?"
            } else {
                "Kill this process with SIGTERM?"
            }),
            Line::from(""),
            Line::from(format!("Process: {}", entry.process_name)),
            Line::from(format!("PID:     {}", entry.pid)),
            Line::from(format!("Port:    {}", entry.port)),
            Line::from(format!(
                "Command: {}",
                entry.command.as_deref().unwrap_or("")
            )),
            Line::from(""),
            Line::from("[y] Yes    [n] No"),
        ])
        .block(Block::default().title("Confirm kill").borders(Borders::ALL))
        .wrap(Wrap { trim: true }),
        area,
    );
}

fn inner(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(horizontal),
        y: area.y.saturating_add(vertical),
        width: area.width.saturating_sub(horizontal * 2),
        height: area.height.saturating_sub(vertical * 2),
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
