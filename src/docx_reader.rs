use anyhow::{Context, Result};
use quick_xml::{
    Reader,
    events::{BytesStart, Event as XmlEvent},
};
use ratatui::style::Color;
use std::{fs::File, io::Read, path::PathBuf};
use zip::ZipArchive;

use crate::document::{DocBlock, Document, ParagraphBlock, Run, TableBlock, TableCell, TableRow};

pub fn read_docx(path: &PathBuf) -> Result<Document> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut document_xml = String::new();
    archive
        .by_name("word/document.xml")
        .context("DOCX does not contain word/document.xml")?
        .read_to_string(&mut document_xml)?;

    parse_document(&document_xml)
}

fn heading_level_from_style(value: &str) -> Option<u8> {
    let lower = value.to_lowercase();

    if lower.contains("title") {
        return Some(0);
    }

    let pos = lower.find("heading")?;
    let rest = &lower[pos + "heading".len()..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();

    match digits.parse::<u8>() {
        Ok(level) => Some(level.max(1)),
        Err(_) => Some(1),
    }
}

fn new_run() -> Run {
    Run {
        text: String::new(),
        bold: false,
        italic: false,
        underline: false,
        color: None,
        size: None,
    }
}

struct ParserState {
    current_run: Option<Run>,
    current_table: Option<TableBlock>,
    current_cell: Option<TableCell>,
    paragraph_heading_level: Option<u8>,
}

fn apply_formatting_tag(name: &str, e: &BytesStart<'_>, state: &mut ParserState) -> bool {
    match name {
        "pStyle" => {
            if let Some(value) = attribute(e, "val") {
                if let Some(level) = heading_level_from_style(&value) {
                    state.paragraph_heading_level = Some(level);
                }
            }
        }

        "b" => set_run(state, |run| run.bold = true),
        "i" => set_run(state, |run| run.italic = true),
        "u" => set_run(state, |run| run.underline = true),

        "color" => {
            if let Some(value) = attribute(e, "val") {
                let color = parse_docx_color(&value);
                set_run(state, |run| run.color = color);
            }
        }

        "sz" => {
            if let Some(half_points) = attribute(e, "val").and_then(|v| v.parse::<u16>().ok()) {
                set_run(state, |run| run.size = Some(half_points));
            }
        }

        "br" => set_run(state, |run| run.text.push('\n')),

        "gridCol" => {
            if let (Some(table), Some(width)) = (
                state.current_table.as_mut(),
                attribute(e, "w").and_then(|w| w.parse::<u32>().ok()),
            ) {
                table.column_widths.push(width);
            }
        }

        "tcW" => {
            if let (Some(cell), Some(width)) = (
                state.current_cell.as_mut(),
                attribute(e, "w").and_then(|w| w.parse::<u32>().ok()),
            ) {
                cell.width = Some(width);
            }
        }

        _ => return false,
    }

    true
}

fn set_run(state: &mut ParserState, f: impl FnOnce(&mut Run)) {
    if let Some(run) = state.current_run.as_mut() {
        f(run);
    }
}

pub fn parse_document(xml: &str) -> Result<Document> {
    let mut reader = Reader::from_str(xml);

    let mut blocks = Vec::new();
    let mut current_paragraph: Option<ParagraphBlock> = None;
    let mut current_row: Option<TableRow> = None;
    let mut in_text = false;

    let mut state = ParserState {
        current_run: None,
        current_table: None,
        current_cell: None,
        paragraph_heading_level: None,
    };

    loop {
        match reader.read_event()? {
            XmlEvent::Start(ref e) => {
                let name = local_name(e.name().as_ref());

                if apply_formatting_tag(&name, e, &mut state) {
                    continue;
                }

                match name.as_str() {
                    "p" => {
                        current_paragraph = Some(ParagraphBlock { runs: Vec::new() });
                        state.paragraph_heading_level = None;
                    }
                    "r" => state.current_run = Some(new_run()),
                    "t" => in_text = true,
                    "tbl" => {
                        state.current_table = Some(TableBlock {
                            rows: Vec::new(),
                            column_widths: Vec::new(),
                        })
                    }
                    "tr" => current_row = Some(TableRow { cells: Vec::new() }),
                    "tc" => {
                        state.current_cell = Some(TableCell {
                            runs: Vec::new(),
                            width: None,
                        })
                    }
                    _ => {}
                }
            }

            XmlEvent::Empty(ref e) => {
                let name = local_name(e.name().as_ref());
                apply_formatting_tag(&name, e, &mut state);
            }

            XmlEvent::End(ref e) => {
                let name = local_name(e.name().as_ref());

                match name.as_str() {
                    "p" => {
                        if let Some(paragraph) = current_paragraph.take() {
                            if let Some(cell) = state.current_cell.as_mut() {
                                if !cell.runs.is_empty() {
                                    cell.runs.push(Run {
                                        text: "\n".to_string(),
                                        ..new_run()
                                    });
                                }
                                cell.runs.extend(paragraph.runs);
                            } else if let Some(level) = state.paragraph_heading_level {
                                blocks.push(DocBlock::Heading(paragraph, level));
                            } else {
                                blocks.push(DocBlock::Paragraph(paragraph));
                            }
                        }
                    }

                    "r" => {
                        if let Some(run) = state.current_run.take() {
                            if let Some(paragraph) = current_paragraph.as_mut() {
                                paragraph.runs.push(run);
                            }
                        }
                    }

                    "t" => in_text = false,

                    "tc" => {
                        if let Some(cell) = state.current_cell.take() {
                            if let Some(row) = current_row.as_mut() {
                                row.cells.push(cell);
                            }
                        }
                    }

                    "tr" => {
                        if let Some(row) = current_row.take() {
                            if let Some(table) = state.current_table.as_mut() {
                                table.rows.push(row);
                            }
                        }
                    }

                    "tbl" => {
                        if let Some(table) = state.current_table.take() {
                            blocks.push(DocBlock::Table(table));
                        }
                    }

                    _ => {}
                }
            }

            XmlEvent::Text(e) => {
                if in_text {
                    let text = e.decode()?.to_string();
                    set_run(&mut state, |run| run.text.push_str(&text));
                }
            }

            XmlEvent::Eof => break,

            _ => {}
        }
    }

    Ok(Document { blocks })
}

pub fn parse_docx_color(value: &str) -> Option<Color> {
    let value = value.trim();

    if value.eq_ignore_ascii_case("auto") {
        return None;
    }

    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(Color::Rgb(r, g, b))
}

pub fn local_name(name: &[u8]) -> String {
    let name = name.split(|b| *b == b':').last().unwrap_or(name);
    String::from_utf8_lossy(name).into_owned()
}

pub fn attribute(element: &BytesStart<'_>, wanted: &str) -> Option<String> {
    element.attributes().flatten().find_map(|attr| {
        let key = attr.key.as_ref();
        let key = key.split(|b| *b == b':').last().unwrap_or(key);

        if key == wanted.as_bytes() {
            Some(String::from_utf8_lossy(&attr.value).into_owned())
        } else {
            None
        }
    })
}
