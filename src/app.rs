use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::io;
use std::path::PathBuf;

use crate::config::{self, Theme};
use crate::document::Document;
use crate::render;

pub struct ThemeMenu {
    pub names: Vec<String>,
    pub selected: usize,
}

pub struct App {
    pub document: Document,
    pub scroll: u16,
    pub theme: Theme,
    pub themes_dir: PathBuf,
    pub status: Option<String>,
    pub menu: Option<ThemeMenu>,
    pub src_file: String,
}

impl App {
    pub fn new(document: Document, themes_dir: PathBuf, src_file: String) -> Self {
        let outcome = config::load_startup_theme(&themes_dir);

        Self {
            document,
            scroll: 0,
            theme: outcome.theme,
            themes_dir,
            status: Some(outcome.status),
            menu: None,
            src_file: src_file,
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn open_theme_menu(&mut self) {
        match config::list_theme_names(&self.themes_dir) {
            Ok(names) if !names.is_empty() => {
                let selected = names
                    .iter()
                    .position(|n| n == &self.theme.name)
                    .unwrap_or(0);

                self.menu = Some(ThemeMenu { names, selected });
            }
            Ok(_) => {
                self.status = Some(format!("No themes found in {}", self.themes_dir.display()));
            }
            Err(e) => {
                self.status = Some(format!("Failed to list themes: {e}"));
            }
        }
    }

    pub fn close_theme_menu(&mut self) {
        self.menu = None;
    }

    pub fn menu_move(&mut self, delta: isize) {
        if let Some(menu) = &mut self.menu {
            let len = menu.names.len() as isize;

            if len == 0 {
                return;
            }

            let next = (menu.selected as isize + delta).rem_euclid(len);
            menu.selected = next as usize;
        }
    }

    pub fn apply_selected_theme(&mut self) {
        let Some(menu) = self.menu.take() else {
            return;
        };

        let Some(name) = menu.names.get(menu.selected).cloned() else {
            return;
        };

        match config::load_theme_by_name(&self.themes_dir, &name) {
            Ok(theme) => {
                self.theme = theme;
                self.status = Some(format!("Loaded theme '{name}'"));

                if let Err(e) = config::save_last_theme_name(&self.themes_dir, &name) {
                    self.status = Some(format!(
                        "Loaded '{name}' but failed to save preference: {e}"
                    ));
                }
            }
            Err(e) => {
                self.status = Some(format!("Failed to load '{name}': {e}"));
            }
        }
    }
}

pub fn run_tui(document: Document, themes_dir: PathBuf, src_name: &str, quit_init: bool) -> Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();

    execute!(stdout, EnterAlternateScreen)?;

    let result = run_app(document, themes_dir, src_name, quit_init);

    disable_raw_mode()?;

    execute!(stdout, LeaveAlternateScreen)?;

    result
}

fn run_app(document: Document, themes_dir: PathBuf, src_name: &str, quit_init: bool) -> Result<()> {
    let mut terminal = ratatui::init();

    let mut app = App::new(document, themes_dir, src_name.to_owned());

    loop {
        terminal.draw(|frame| render::draw(frame, &mut app))?;
        if quit_init {
            break;                    
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if app.menu.is_some() {
                    match key.code {
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.menu_move(1);
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.menu_move(-1);
                        }
                        KeyCode::Enter => {
                            app.apply_selected_theme();
                        }
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.close_theme_menu();
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,

                        KeyCode::Down | KeyCode::Char('j') => {
                            app.scroll_down();
                        }

                        KeyCode::Up | KeyCode::Char('k') => {
                            app.scroll_up();
                        }

                        KeyCode::PageDown => {
                            app.scroll = app.scroll.saturating_add(10);
                        }

                        KeyCode::PageUp => {
                            app.scroll = app.scroll.saturating_sub(10);
                        }

                        KeyCode::Char('t') => {
                            app.open_theme_menu();
                        }

                        _ => {}
                    }
                }

            }
        }
    }

    ratatui::restore();

    Ok(())
}
