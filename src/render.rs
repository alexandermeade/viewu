use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::{App, ThemeMenu};
use crate::config::Theme;
use crate::document::{DocBlock, Document, ParagraphBlock, Run, TableBlock};

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
                let underlined: String = trimmed.chars().map(|c| format!("{c}\u{0330}")).collect();
                format!("{underlined}{whitespace}")
            } else {
                word.to_string()
            }
        })
        .collect()
}

const LARGE_TEXT_HALF_POINTS: u16 = 32;
const HUGE_TEXT_HALF_POINTS: u16 = 56;

fn run_is_large(run: &Run) -> bool {
    run.size.is_some_and(|size| size >= LARGE_TEXT_HALF_POINTS)
}

fn run_is_huge(run: &Run) -> bool {
    run.size.is_some_and(|size| size >= HUGE_TEXT_HALF_POINTS)
}

fn run_is_emphasized(run: &Run) -> bool {
    run.bold || run_is_large(run)
}

fn render_row_line(
    wrapped_cells: &[Vec<Vec<(String, Style)>>],
    widths: &[usize],
    column_count: usize,
    line_index: usize,
    left_padding: usize,
    border_color: Color,
) -> Line<'static> {
    let border_style = Style::default().fg(border_color);
    let mut spans = vec![
        Span::raw(" ".repeat(left_padding)),
        Span::styled("│", border_style),
    ];

    for column in 0..column_count {
        let width = widths.get(column).copied().unwrap_or(3);
        let cell_line = wrapped_cells.get(column).and_then(|l| l.get(line_index));

        let text_width: usize = cell_line
            .map(|line| line.iter().map(|(t, _)| t.width()).sum())
            .unwrap_or(0);
        let pad = width.saturating_sub(text_width);

        spans.push(Span::raw(" "));
        if let Some(cell_line) = cell_line {
            spans.extend(cell_line.iter().map(|(t, s)| Span::styled(t.clone(), *s)));
        }
        spans.push(Span::raw(" ".repeat(pad + 1)));
        spans.push(Span::styled("│", border_style));
    }

    Line::from(spans)
}

fn resolve_column_widths(
    table: &TableBlock,
    column_count: usize,
    usable_width: usize,
) -> Vec<usize> {
    let total_twips: u32 = table.column_widths.iter().sum();

    let mut widths = if total_twips > 0 {
        table_widths_from_twips(table, column_count, total_twips, usable_width)
    } else {
        distribute_width(column_count, usable_width)
    };

    if widths.iter().any(|w| *w < 3) {
        widths = distribute_width(column_count, usable_width);
    }

    fit_column_widths(widths, usable_width)
}

fn table_to_lines(table: &TableBlock, available_width: usize, theme: &Theme) -> Vec<Line<'static>> {
    if table.rows.is_empty() {
        return Vec::new();
    }

    let column_count = table
        .column_widths
        .len()
        .max(table.rows.iter().map(|r| r.cells.len()).max().unwrap_or(0));

    if column_count == 0 || available_width < 4 {
        return Vec::new();
    }

    let usable_width = available_width.saturating_sub(theme.table_padding * 2);
    if usable_width < 4 {
        return Vec::new();
    }

    let widths = resolve_column_widths(table, column_count, usable_width);
    let left_padding =
        theme.table_padding + usable_width.saturating_sub(table_line_width(&widths)) / 2;

    let border = |l: char, m: char, r: char| {
        centered_table_border(&widths, l, m, r, left_padding, theme.table_border)
    };

    let mut lines = vec![border('┌', '┬', '┐')];

    for (row_index, row) in table.rows.iter().enumerate() {
        let wrapped_cells: Vec<_> = row
            .cells
            .iter()
            .enumerate()
            .map(|(i, cell)| wrap_runs(&cell.runs, widths.get(i).copied().unwrap_or(3)))
            .collect();

        let row_height = wrapped_cells.iter().map(Vec::len).max().unwrap_or(1);

        lines.extend((0..row_height).map(|line_index| {
            render_row_line(
                &wrapped_cells,
                &widths,
                column_count,
                line_index,
                left_padding,
                theme.table_border,
            )
        }));

        if row_index + 1 < table.rows.len() {
            lines.push(border('├', '┼', '┤'));
        }
    }

    lines.push(border('└', '┴', '┘'));
    lines
}

fn table_widths_from_twips(
    table: &TableBlock,
    column_count: usize,
    total_twips: u32,
    available_width: usize,
) -> Vec<usize> {
    let usable_width = available_width.saturating_sub(column_count * 3);

    (0..column_count)
        .map(|i| {
            let twips = table.column_widths.get(i).copied().unwrap_or(1);
            let width = (twips as f64 / total_twips as f64 * usable_width as f64).round() as usize;
            width.max(3)
        })
        .collect()
}

fn fit_column_widths(mut widths: Vec<usize>, available_width: usize) -> Vec<usize> {
    let max_inner_width = available_width.saturating_sub(widths.len() * 3);
    let current = widths.iter().sum::<usize>();

    if current <= max_inner_width {
        return widths;
    }

    let scale = max_inner_width as f64 / current as f64;

    for width in &mut widths {
        *width = (*width as f64 * scale).floor().max(3.0) as usize;
    }

    while widths.iter().sum::<usize>() > max_inner_width {
        match widths.iter().position(|w| *w > 3) {
            Some(index) => widths[index] -= 1,
            None => break,
        }
    }

    widths
}

fn distribute_width(column_count: usize, available_width: usize) -> Vec<usize> {
    if column_count == 0 {
        return Vec::new();
    }

    let usable = available_width.saturating_sub(column_count * 3);
    let base = usable / column_count;
    let remainder = usable % column_count;

    (0..column_count)
        .map(|i| (base + usize::from(i < remainder)).max(3))
        .collect()
}

fn table_line_width(widths: &[usize]) -> usize {
    if widths.is_empty() {
        return 0;
    }

    widths.iter().sum::<usize>() + widths.len() * 2 + 1
}

fn centered_table_border(
    widths: &[usize],
    left: char,
    middle: char,
    right: char,
    left_padding: usize,
    color: Color,
) -> Line<'static> {
    let mut result = " ".repeat(left_padding);
    result.push(left);

    for (i, width) in widths.iter().enumerate() {
        result.push_str(&"─".repeat(*width + 2));
        result.push(if i + 1 == widths.len() { right } else { middle });
    }

    Line::from(Span::styled(result, Style::default().fg(color)))
}

enum Token {
    Word(Vec<(String, Style)>),
    Break,
}

fn tokenize_runs(runs: &[Run]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current_word: Vec<(String, Style)> = Vec::new();
    let mut buf = String::new();

    for run in runs {
        let style = run_style(run);

        macro_rules! flush_buf {
            () => {
                if !buf.is_empty() {
                    current_word.push((std::mem::take(&mut buf), style));
                }
            };
        }

        for c in run.text.chars() {
            if c == '\n' {
                flush_buf!();
                if !current_word.is_empty() {
                    tokens.push(Token::Word(std::mem::take(&mut current_word)));
                }
                tokens.push(Token::Break);
            } else if c.is_whitespace() {
                flush_buf!();
                if !current_word.is_empty() {
                    tokens.push(Token::Word(std::mem::take(&mut current_word)));
                }
            } else {
                buf.push(c);
            }
        }

        flush_buf!();
    }

    if !current_word.is_empty() {
        tokens.push(Token::Word(current_word));
    }

    tokens
}

fn split_at_width(s: &str, max_width: usize) -> (&str, &str) {
    let mut w = 0;
    let mut idx = s.len();

    for (i, c) in s.char_indices() {
        let cw = c.width().unwrap_or(0);
        if w > 0 && w + cw > max_width {
            idx = i;
            break;
        }
        w += cw;
    }

    s.split_at(idx)
}

fn wrap_runs(runs: &[Run], width: usize) -> Vec<Vec<(String, Style)>> {
    if width == 0 {
        return vec![Vec::new()];
    }

    let tokens = tokenize_runs(runs);

    let mut result: Vec<Vec<(String, Style)>> = Vec::new();
    let mut current: Vec<(String, Style)> = Vec::new();
    let mut current_width = 0usize;

    for token in tokens {
        match token {
            Token::Break => {
                result.push(std::mem::take(&mut current));
                current_width = 0;
            }

            Token::Word(fragments) => {
                let word_width: usize = fragments.iter().map(|(t, _)| t.width()).sum();
                let required = if current.is_empty() {
                    word_width
                } else {
                    word_width + 1
                };

                if current_width + required > width && !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                    current_width = 0;
                }

                if !current.is_empty() {
                    current.push((" ".to_string(), Style::default()));
                    current_width += 1;
                }

                for (text, style) in fragments {
                    let mut remaining = text.as_str();

                    while current_width + remaining.width() > width {
                        let space_left = width.saturating_sub(current_width);

                        if space_left == 0 {
                            result.push(std::mem::take(&mut current));
                            current_width = 0;
                            continue;
                        }

                        let (part, rest) = split_at_width(remaining, space_left);

                        if part.is_empty() {
                            result.push(std::mem::take(&mut current));
                            current_width = 0;
                            continue;
                        }

                        current.push((part.to_string(), style));
                        result.push(std::mem::take(&mut current));
                        current_width = 0;
                        remaining = rest;
                    }

                    if !remaining.is_empty() {
                        current_width += remaining.width();
                        current.push((remaining.to_string(), style));
                    }
                }
            }
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    if result.is_empty() {
        result.push(Vec::new());
    }

    result
}

fn run_style(run: &Run) -> Style {
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
