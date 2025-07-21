use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use std::collections::HashSet;
use std::io;
use std::path::PathBuf;

use crate::app::{App, Pane, TransferItem};
use crate::sftp::FileInfo;
use crate::ssh_config::SshHost;

pub struct Ui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl Ui {
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Ui { terminal })
    }

    pub fn draw(&mut self, app: &App) -> Result<()> {
        let current_host = app.current_host.clone();
        let active_pane = app.active_pane.clone();
        let local_path = app.local_path.clone();
        let remote_path = app.remote_path.clone();
        let local_cursor = app.local_cursor;
        let remote_cursor = app.remote_cursor;
        let local_selected = app.local_selected.clone();
        let remote_selected = app.remote_selected.clone();
        let show_connection_dialog = app.show_connection_dialog;
        let show_transfer_dialog = app.show_transfer_dialog;
        let available_hosts = app.available_hosts.clone();
        let connection_cursor = app.connection_cursor;
        let transfer_queue = app.transfer_queue.clone();

        self.terminal.draw(move |f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(0),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            Ui::draw_header(f, chunks[0], &current_host);
            Ui::draw_panes(
                f,
                chunks[1],
                &active_pane,
                &local_path,
                &remote_path,
                app.get_current_local_files(),
                app.get_current_remote_files(),
                local_cursor,
                remote_cursor,
                &local_selected,
                &remote_selected,
            );
            Ui::draw_footer(f, chunks[2], app.search_mode, &app.search_query);

            if show_connection_dialog {
                Ui::draw_connection_dialog(f, &available_hosts, connection_cursor);
            }

            if show_transfer_dialog {
                Ui::draw_transfer_dialog(f, &transfer_queue);
            }
        })?;

        Ok(())
    }

    fn draw_header(f: &mut Frame, area: Rect, current_host: &Option<String>) {
        let title = format!(
            "SFTP TUI - Connected to: {}",
            current_host
                .as_ref()
                .unwrap_or(&"Not Connected".to_string())
        );
        let header = Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(header, area);
    }

    fn draw_panes(
        f: &mut Frame,
        area: Rect,
        active_pane: &Pane,
        local_path: &PathBuf,
        remote_path: &PathBuf,
        local_files: &[FileInfo],
        remote_files: &[FileInfo],
        local_cursor: usize,
        remote_cursor: usize,
        local_selected: &HashSet<usize>,
        remote_selected: &HashSet<usize>,
    ) {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        Ui::draw_local_pane(
            f,
            panes[0],
            active_pane,
            local_path,
            local_files,
            local_cursor,
            local_selected,
        );
        Ui::draw_remote_pane(
            f,
            panes[1],
            active_pane,
            remote_path,
            remote_files,
            remote_cursor,
            remote_selected,
        );
    }

    fn draw_local_pane(
        f: &mut Frame,
        area: Rect,
        active_pane: &Pane,
        local_path: &PathBuf,
        local_files: &[FileInfo],
        local_cursor: usize,
        local_selected: &HashSet<usize>,
    ) {
        let title = format!("Local: {} ({})", local_path.display(), local_files.len());
        let style = if *active_pane == Pane::Local {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        let items: Vec<ListItem> = local_files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let prefix = if file.name == ".." {
                    " "
                } else if file.is_dir {
                    " "
                } else {
                    " "
                };
                let name = format!("{}{}", prefix, file.name);
                let mut item_style = Style::default();

                if local_selected.contains(&i) {
                    item_style = item_style.bg(Color::Blue);
                }

                ListItem::new(name).style(item_style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(local_cursor));
        f.render_stateful_widget(list, area, &mut state);
    }

    fn draw_remote_pane(
        f: &mut Frame,
        area: Rect,
        active_pane: &Pane,
        remote_path: &PathBuf,
        remote_files: &[FileInfo],
        remote_cursor: usize,
        remote_selected: &HashSet<usize>,
    ) {
        let title = format!("Remote: {} ({})", remote_path.display(), remote_files.len());
        let style = if *active_pane == Pane::Remote {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        let items: Vec<ListItem> = remote_files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let prefix = if file.name == ".." {
                    " "
                } else if file.is_dir {
                    " "
                } else {
                    " "
                };
                let name = format!("{}{}", prefix, file.name);
                let mut item_style = Style::default();

                if remote_selected.contains(&i) {
                    item_style = item_style.bg(Color::Blue);
                }

                ListItem::new(name).style(item_style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(remote_cursor));
        f.render_stateful_widget(list, area, &mut state);
    }

    fn draw_footer(f: &mut Frame, area: Rect, search_mode: bool, search_query: &str) {
        let footer_text = if search_mode {
            format!("Search: {search_query} | Esc: Cancel | Enter: Exit search")
        } else {
            [
                "Tab: Switch panes",
                "Space: Select/deselect",
                "Enter: Change directory",
                "T: Transfer files",
                "C: Change connection",
                "/: Search",
                "Q: Quit",
            ]
            .join(" | ")
        };

        let footer = Paragraph::new(footer_text)
            .block(Block::default().borders(Borders::ALL))
            .style(if search_mode {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Cyan)
            });
        f.render_widget(footer, area);
    }

    fn draw_connection_dialog(
        f: &mut Frame,
        available_hosts: &[SshHost],
        connection_cursor: usize,
    ) {
        let area = Ui::centered_rect(60, 20, f.area());

        f.render_widget(Clear, area);

        let hosts: Vec<ListItem> = available_hosts
            .iter()
            .map(|host| {
                let display = format!(
                    "{} ({})",
                    host.host,
                    host.hostname.as_ref().unwrap_or(&host.host)
                );
                ListItem::new(display)
            })
            .collect();

        let list = List::new(hosts)
            .block(Block::default().borders(Borders::ALL).title("Select Host"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(connection_cursor));
        f.render_stateful_widget(list, area, &mut state);
    }

    fn draw_transfer_dialog(f: &mut Frame, transfer_queue: &[TransferItem]) {
        let area = Ui::centered_rect(80, 30, f.area());

        f.render_widget(Clear, area);

        let items: Vec<ListItem> = transfer_queue
            .iter()
            .map(|item| {
                let direction = match item.direction {
                    crate::app::TransferDirection::Upload => "",
                    crate::app::TransferDirection::Download => "",
                };
                let text = format!(
                    "{} {} -> {}",
                    direction,
                    item.source.display(),
                    item.destination.display()
                );
                ListItem::new(text)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Transfer Queue (Enter to confirm, Esc to cancel)"),
            )
            .style(Style::default().fg(Color::Yellow));

        f.render_widget(list, area);
    }

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

    pub fn handle_events(&self) -> Result<Option<Event>> {
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            let event = crossterm::event::read()?;
            if let Event::Key(key) = &event {
                if key.kind == KeyEventKind::Press {
                    return Ok(Some(event));
                }
            }
        }
        Ok(None)
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
    }
}

