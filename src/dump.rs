use std::fmt::Write as _;

use crate::document::{DocBlock, Document, Run};

pub fn document_to_text(document: &Document) -> String {
    let mut out = String::new();

    for block in &document.blocks {
        match block {
            DocBlock::Paragraph(paragraph) => {
                writeln!(out, "{}", run_text(&paragraph.runs, true)).unwrap();
                out.push('\n');
            }

            DocBlock::Heading(heading, level) => {
                let prefix = "#".repeat((*level).max(1) as usize);
                writeln!(out, "{prefix} {}", run_text(&heading.runs, true)).unwrap();
                out.push('\n');
            }

            DocBlock::PageBreak => {
                out.push_str("----------------------------------------\n\n");
            }

            DocBlock::Table(table) => {
                for row in &table.rows {
                    let cells: Vec<String> = row.cells.iter().map(|c| run_text(&c.runs, false)).collect();
                    writeln!(out, "{}", cells.join(" | ")).unwrap();
                }
                out.push('\n');
            }
        }
    }

    out
}

pub fn document_to_markdown(document: &Document) -> String {
    let mut out = String::new();

    for block in &document.blocks {
        match block {
            DocBlock::Paragraph(paragraph) => {
                writeln!(out, "{}", run_markdown(&paragraph.runs, true)).unwrap();
                out.push('\n');
            }

            DocBlock::Heading(heading, level) => {
                let prefix = "#".repeat((*level).max(1).min(6) as usize);
                writeln!(out, "{prefix} {}", run_markdown(&heading.runs, true)).unwrap();
                out.push('\n');
            }

            DocBlock::PageBreak => {
                out.push_str("---\n\n");
            }

            DocBlock::Table(table) => {
                out.push_str(&table_to_markdown(table));
                out.push('\n');
            }
        }
    }

    out
}

fn table_to_markdown(table: &crate::document::TableBlock) -> String {
    let column_count = table
        .column_widths
        .len()
        .max(table.rows.iter().map(|r| r.cells.len()).max().unwrap_or(0));

    if column_count == 0 {
        return String::new();
    }

    let mut out = String::new();

    let row_to_markdown = |cells: &[String]| {
        let mut padded = cells.to_vec();
        padded.resize(column_count, String::new());
        format!("| {} |\n", padded.join(" | "))
    };

    for (row_index, row) in table.rows.iter().enumerate() {
        let cells: Vec<String> = row.cells.iter().map(|c| escape_table_cell(&run_markdown(&c.runs, false))).collect();
        out.push_str(&row_to_markdown(&cells));

        if row_index == 0 {
            let separator = vec!["---".to_string(); column_count];
            out.push_str(&row_to_markdown(&separator));
        }
    }

    out
}

fn escape_table_cell(text: &str) -> String {
    text.replace('\n', "<br>").replace('|', "\\|")
}

fn run_text(runs: &[Run], keep_newlines: bool) -> String {
    let text: String = runs.iter().map(|r| r.text.as_str()).collect();

    if keep_newlines {
        text
    } else {
        text.replace('\n', " ")
    }
}

fn run_markdown(runs: &[Run], keep_newlines: bool) -> String {
    runs.iter()
        .map(|run| {
            let text = if keep_newlines { run.text.clone() } else { run.text.replace('\n', " ") };

            if text.trim().is_empty() {
                return text;
            }

            let mut text = text;
            if run.italic {
                text = format!("_{text}_");
            }
            if run.bold {
                text = format!("**{text}**");
            }
            if run.underline {
                text = format!("<u>{text}</u>");
            }

            text
        })
        .collect()
}
