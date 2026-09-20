use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::config::Theme;
use crate::document::{Run, TableBlock};
use crate::render::{run_is_large, run_is_huge, run_style, run_is_emphasized};

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

pub fn table_to_lines(
    table: &TableBlock,
    available_width: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
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
            let width =
                (twips as f64 / total_twips as f64 * usable_width as f64).round() as usize;
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
        result.push(if i + 1 == widths.len() {
            right
        } else {
            middle
        });
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
    let mut width = 0;
    let mut index = s.len();

    for (i, c) in s.char_indices() {
        let column_width = c.width().unwrap_or(0);
        if width > 0 && width + column_width > max_width {
            index = i;
            break;
        }
        width += column_width;
    }

    s.split_at(index)
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
