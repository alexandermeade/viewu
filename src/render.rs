use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use unicode_width::UnicodeWidthStr;

use crate::app::{App, ThemeMenu};
use crate::config::Theme;
use crate::document::{DocBlock, Document, ParagraphBlock, Run};

use crate::table::table_to_lines;

pub fn draw(frame: &mut ratatui::Frame, app: &mut App) {
    let area = frame.area();

    frame.render_widget(
        Block::default().style(
            Style::default()
                .bg(app.theme.background)
                .fg(app.theme.foreground),
        ),
        area,
    );

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    let header = Paragraph::new(format!(" ViewU {}", app.theme.name)).style(
        Style::default()
            .fg(app.theme.heading)
            .bg(app.theme.background)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_widget(header, layout[0]);

    let document_width = layout[1].width.saturating_sub(2) as usize;
    let lines = document_to_lines(&app.document, document_width, &app.theme);

    let document = Paragraph::new(lines)
        .style(
            Style::default()
                .fg(app.theme.foreground)
                .bg(app.theme.background),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(
                    Style::default()
                        .fg(app.theme.table_border)
                        .bg(app.theme.background),
                )
                .title(app.src_file.clone()),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));

    frame.render_widget(document, layout[1]);

    draw_footer(frame, app, layout[2]);

    if let Some(menu) = &app.menu {
        draw_theme_menu(frame, app, menu);
    }
}

const FOOTER_HINT: &str = " ↑/k Up   ↓/j Down   PgUp/PgDn Scroll   t Theme   q/Esc Quit ";

fn draw_footer(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let footer_style = Style::default()
        .fg(app.theme.foreground)
        .bg(app.theme.background);
    let status_text = app.status.as_deref().unwrap_or("");

    if status_text.is_empty() {
        frame.render_widget(Paragraph::new(FOOTER_HINT).style(footer_style), area);
        return;
    }

    let status_width = (status_text.width() as u16 + 2).min(area.width);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(status_width)])
        .split(area);

    frame.render_widget(Paragraph::new(FOOTER_HINT).style(footer_style), columns[0]);
    frame.render_widget(
        Paragraph::new(format!("{status_text} "))
            .alignment(Alignment::Right)
            .style(footer_style),
        columns[1],
    );
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(area.width.saturating_sub(width) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vertical[1]);

    horizontal[1]
}

fn draw_theme_menu(frame: &mut ratatui::Frame, app: &App, menu: &ThemeMenu) {
    let area = frame.area();

    let longest_name = menu.names.iter().map(|n| n.width()).max().unwrap_or(10) as u16;
    let max_width = area.width.saturating_sub(4).max(24);
    let width = (longest_name + 8).clamp(24, max_width);

    let max_height = area.height.saturating_sub(2).max(3);
    let height = (menu.names.len() as u16 + 2).clamp(3, max_height);

    let popup_area = centered_rect(width, height, area);
    frame.render_widget(Clear, popup_area);

    let lines: Vec<Line> = menu
        .names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let marker = if name == &app.theme.name {
                "→ "
            } else {
                "  "
            };

            let style = if i == menu.selected {
                Style::default()
                    .fg(app.theme.background)
                    .bg(app.theme.heading)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(app.theme.foreground)
                    .bg(app.theme.background)
            };

            Line::from(Span::styled(format!("{marker}{name}"), style))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.table_border))
        .style(Style::default().bg(app.theme.background))
        .title(" (↑/↓ · Enter · Esc) ");

    frame.render_widget(Paragraph::new(lines).block(block), popup_area);
}

pub fn document_to_lines(
    document: &Document,
    available_width: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for block in &document.blocks {
        match block {
            DocBlock::Paragraph(paragraph) => {
                lines.extend(paragraph_to_lines(paragraph, None, theme));
                lines.push(Line::from(""));
            }

            DocBlock::Heading(heading, level) => {
                lines.extend(paragraph_to_lines(heading, Some(*level), theme));
                lines.push(Line::from(""));
            }

            DocBlock::PageBreak => {
                lines.push(Line::from("────────────────────────────────────────"));
            }

            DocBlock::Table(table) => {
                lines.extend(table_to_lines(table, available_width, theme));
                lines.push(Line::from(""));
            }
        }
    }

    lines
}

fn paragraph_to_lines(
    paragraph: &ParagraphBlock,
    heading_level: Option<u8>,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let mut spans = Vec::new();

    if let Some(level) = heading_level {
        let level = level.clamp(1, 6);
        let prefix = match level {
            1 => "◆ H1  ",
            2 => "▸ H2  ",
            3 => "• H3  ",
            4 => "  H4  ",
            5 => "    H5  ",
            _ => "      H6  ",
        };

        spans.push(Span::styled(prefix, heading_style(level, theme)));
    }

    for run in &paragraph.runs {
        let mut style = run_style(run);

        if let Some(level) = heading_level {
            style = heading_style(level.clamp(1, 6), theme);
        } else if run_is_emphasized(run) && run.color.is_none() {
            style = style.fg(theme.bold);
        }

        if contains_url(&run.text) {
            spans.push(Span::styled(
                underline_urls(&run.text),
                style.fg(Color::Blue).remove_modifier(Modifier::UNDERLINED),
            ));
        } else {
            spans.push(Span::styled(run.text.clone(), style));
        }
    }

    vec![Line::from(spans)]
}

fn heading_style(level: u8, theme: &Theme) -> Style {
    let style = Style::default().fg(theme.heading);

    match level {
        1 => style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        2 | 4 => style.add_modifier(Modifier::BOLD),
        3 => style.add_modifier(Modifier::BOLD | Modifier::ITALIC),
        5 => style.add_modifier(Modifier::ITALIC),
        6 => style.add_modifier(Modifier::DIM),
        _ => style.add_modifier(Modifier::BOLD),
    }
}

fn contains_url(text: &str) -> bool {
    text.split_whitespace().any(is_url)
}

fn is_url(text: &str) -> bool {
    let text = text.trim_matches(|c: char| {
        matches!(
            c,
            '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\''
        )
    });

    text.starts_with("http://") || text.starts_with("https://") || text.starts_with("www.")
}

fn underline_urls(text: &str) -> String {
    text.split_inclusive(char::is_whitespace)
        .map(|word| {
            let trimmed = word.trim_end_matches(char::is_whitespace);
            let whitespace = &word[trimmed.len()..];

            if is_url(trimmed) {
                let underlined: String = trimmed
                    .chars()
                    .map(|c| format!("{c}\u{0330}"))
                    .collect();
                format!("{underlined}{whitespace}")
            } else {
                word.to_string()
            }
        })
        .collect()
}

const LARGE_TEXT_HALF_POINTS: u16 = 32;
const HUGE_TEXT_HALF_POINTS: u16 = 56;

pub fn run_is_large(run: &Run) -> bool {
    run.size.is_some_and(|size| size >= LARGE_TEXT_HALF_POINTS)
}

pub fn run_is_huge(run: &Run) -> bool {
    run.size.is_some_and(|size| size >= HUGE_TEXT_HALF_POINTS)
}

pub fn run_is_emphasized(run: &Run) -> bool {
    run.bold || run_is_large(run)
}

pub fn run_style(run: &Run) -> Style {
    let mut style = Style::default();

    if run.bold || run_is_large(run) {
        style = style.add_modifier(Modifier::BOLD);
    }

    if run.italic {
        style = style.add_modifier(Modifier::ITALIC);
    }

    if run.underline || run_is_huge(run) {
        style = style.add_modifier(Modifier::UNDERLINED);
    }

    if let Some(color) = run.color {
        style = style.fg(color);
    }

    style
}

