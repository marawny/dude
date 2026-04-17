use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;

use crate::core::model::Package;

pub struct TuiApp {
    packages: Vec<Package>,
    selected: Vec<bool>,
    list_state: ListState,
    should_quit: bool,
}

impl TuiApp {
    pub fn new(packages: Vec<Package>) -> Self {
        let selected = vec![false; packages.len()];
        let mut list_state = ListState::default();
        if !packages.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            packages,
            selected,
            list_state,
            should_quit: false,
        }
    }

    pub fn selected_packages(&self) -> Vec<Package> {
        self.packages
            .iter()
            .zip(&self.selected)
            .filter_map(|(pkg, &sel)| if sel { Some(pkg.clone()) } else { None })
            .collect()
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.run_app(&mut terminal);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        res
    }

    fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        self.should_quit = true;
                        break;
                    }
                    KeyCode::Enter => {
                        if self.selected.iter().any(|&x| x) {
                            break;
                        }
                    }
                    KeyCode::Up => self.previous(),
                    KeyCode::Down => self.next(),
                    KeyCode::Char(' ') => self.toggle_selection(),
                    KeyCode::Char('a') => self.select_all(),
                    KeyCode::Char('n') => self.select_none(),
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(f.area());

        let items: Vec<ListItem> = self
            .packages
            .iter()
            .enumerate()
            .map(|(i, pkg)| {
                let marker = if self.selected[i] { "✓" } else { " " };
                let line = Line::from(vec![
                    Span::raw(format!("[{}] ", marker)),
                    Span::styled(&pkg.name, Style::default().fg(self.repo_color(&pkg.repo))),
                    Span::raw(format!(" {} ({})", pkg.version, pkg.size_human())),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Orphan Packages"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        f.render_stateful_widget(list, chunks[0], &mut self.list_state);

        let selected_count = self.selected.iter().filter(|&&x| x).count();
        let total_size: u64 = self
            .packages
            .iter()
            .zip(&self.selected)
            .filter_map(|(pkg, &sel)| if sel { Some(pkg.size) } else { None })
            .sum();
        let total_mb = total_size as f64 / 1_048_576.0;

        let summary = Paragraph::new(format!(
            "Selected: {} packages ({:.1} MiB)",
            selected_count, total_mb
        ))
        .block(Block::default().borders(Borders::ALL).title("Summary"));

        f.render_widget(summary, chunks[1]);

        let help = Paragraph::new(
            "↑/↓: Navigate  Space: Toggle  a: Select All  n: Select None  Enter: Remove  q/Esc: Quit",
        )
        .block(Block::default().borders(Borders::ALL).title("Help"));

        f.render_widget(help, chunks[2]);
    }

    fn repo_color(&self, repo: &str) -> Color {
        match repo {
            "core" => Color::Red,
            "extra" => Color::Green,
            "community" => Color::Blue,
            "multilib" => Color::Magenta,
            _ => Color::Yellow,
        }
    }

    fn next(&mut self) {
        if self.packages.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % self.packages.len(),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.packages.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.packages.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn toggle_selection(&mut self) {
        if let Some(i) = self.list_state.selected() {
            self.selected[i] = !self.selected[i];
        }
    }

    fn select_all(&mut self) {
        for sel in &mut self.selected {
            *sel = true;
        }
    }

    fn select_none(&mut self) {
        for sel in &mut self.selected {
            *sel = false;
        }
    }
}
