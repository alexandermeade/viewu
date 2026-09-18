use anyhow::{Context, Result};
use ratatui::style::Color;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct ThemeFile {
    theme: ThemeConfig,
}

#[derive(Debug, Deserialize)]
pub struct ThemeConfig {
    pub name: String,

    pub background: String,
    pub foreground: String,

    pub heading: String,
    pub bold: String,

    pub table_border: String,
    pub table_header: String,

    pub table_padding: usize,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub background: Color,
    pub foreground: Color,
    pub heading: Color,
    pub bold: Color,
    pub table_border: Color,
    #[allow(dead_code)]
    pub table_header: Color,
    pub table_padding: usize,
}

pub const DEFAULT_THEME_NAME: &str = "paper";

pub fn fallback_theme() -> Theme {
    Theme {
        name: DEFAULT_THEME_NAME.to_string(),
        background: Color::Rgb(0xF4, 0xF1, 0xEA),
        foreground: Color::Rgb(0x24, 0x24, 0x24),
        heading: Color::Rgb(0x20, 0x20, 0x20),
        bold: Color::Rgb(0x20, 0x20, 0x20),
        table_border: Color::Rgb(0x77, 0x73, 0x6B),
        table_header: Color::Rgb(0xE6, 0xE0, 0xD4),
        table_padding: 4,
    }
}

fn theme_config_to_theme(config: ThemeConfig) -> Result<Theme> {
    Ok(Theme {
        name: config.name,
        background: parse_hex_color(&config.background)?,
        foreground: parse_hex_color(&config.foreground)?,
        heading: parse_hex_color(&config.heading)?,
        bold: parse_hex_color(&config.bold)?,
        table_border: parse_hex_color(&config.table_border)?,
        table_header: parse_hex_color(&config.table_header)?,
        table_padding: config.table_padding,
    })
}

pub fn parse_hex_color(value: &str) -> Result<Color> {
    let value = value.trim().trim_start_matches('#');

    if value.len() != 6 {
        anyhow::bail!(
            "invalid color '{}': expected #RRGGBB",
            value
        );
    }

    let r = u8::from_str_radix(&value[0..2], 16)?;
    let g = u8::from_str_radix(&value[2..4], 16)?;
    let b = u8::from_str_radix(&value[4..6], 16)?;

    Ok(Color::Rgb(r, g, b))
}

pub fn load_theme_from_file(path: &Path) -> Result<Theme> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let file: ThemeFile = toml::from_str(&contents)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    theme_config_to_theme(file.theme)
}

pub fn load_theme_by_name(dir: &Path, name: &str) -> Result<Theme> {
    let path = dir.join(format!("{name}.toml"));

    if !path.exists() {
        anyhow::bail!("theme file not found: {}", path.display());
    }

    load_theme_from_file(&path)
}

pub fn list_theme_names(dir: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();

    let entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read themes directory {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }

        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            names.push(stem.to_string());
        }
    }

    names.sort();
    Ok(names)
}

fn last_theme_state_path(dir: &Path) -> PathBuf {
    dir.join(".last_theme")
}

pub fn load_last_theme_name(dir: &Path) -> Option<String> {
    let contents = fs::read_to_string(last_theme_state_path(dir)).ok()?;
    let name = contents.trim();

    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

pub fn save_last_theme_name(dir: &Path, name: &str) -> Result<()> {
    fs::write(last_theme_state_path(dir), name)
        .with_context(|| format!("failed to save last theme in {}", dir.display()))
}

pub fn ensure_themes_dir(dir: &Path) -> Result<()> {
    if !dir.exists() {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create themes directory {}", dir.display()))?;
    }

    Ok(())
}

pub struct ThemeLoadOutcome {
    pub theme: Theme,
    pub status: String,
}

pub fn load_startup_theme(dir: &Path) -> ThemeLoadOutcome {
    if let Err(e) = ensure_themes_dir(dir) {
        return ThemeLoadOutcome {
            status: format!(
                "Failed to set up themes directory ({e}); using built-in paper theme"
            ),
            theme: fallback_theme(),
        };
    }

    let name_to_try =
        load_last_theme_name(dir).unwrap_or_else(|| DEFAULT_THEME_NAME.to_string());

    match load_theme_by_name(dir, &name_to_try) {
        Ok(theme) => ThemeLoadOutcome {
            status: String::new(),
            theme,
        },

        Err(e) if name_to_try == DEFAULT_THEME_NAME => ThemeLoadOutcome {
            status: format!(
                "Failed to load default theme 'paper' ({e}); using built-in defaults"
            ),
            theme: fallback_theme(),
        },

        Err(e) => match load_theme_by_name(dir, DEFAULT_THEME_NAME) {
            Ok(theme) => ThemeLoadOutcome {
                status: format!(
                    "Failed to load theme '{name_to_try}' ({e}); loaded 'paper' instead"
                ),
                theme,
            },
            Err(_) => ThemeLoadOutcome {
                status: format!(
                    "Failed to load '{name_to_try}' and 'paper' ({e}); using built-in defaults"
                ),
                theme: fallback_theme(),
            },
        },
    }
}
