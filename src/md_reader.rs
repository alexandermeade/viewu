use anyhow::{Context, Result};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::{fs, path::PathBuf};

use crate::document::{DocBlock, Document, ParagraphBlock, Run, TableBlock, TableCell, TableRow};

pub fn read_md(path: &PathBuf) -> Result<Document> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read markdown file: {}", path.display()))?;

    parse_markdown(&content)
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

fn heading_level_from_tag(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Text styling currently in effect, tracked as a stack so nested
/// `**_bold italic_**` style emphasis composes correctly.
#[derive(Default, Clone, Copy)]
struct RunStyle {
    bold: bool,
    italic: bool,
    // Markdown strikethrough has no equivalent field on `Run`; it is
    // rendered as plain text (see `push_text`).
}

struct ListFrame {
    ordered_next: Option<u64>,
    depth: usize,
}

struct ParserState {
    blocks: Vec<DocBlock>,

    current_paragraph: Option<ParagraphBlock>,
    // Tracks, for each currently-open Paragraph tag, whether *this* tag
    // instance is the one that created `current_paragraph` (so nested
    // paragraph tags inside list items don't clobber or double-flush it).
    paragraph_owner_stack: Vec<bool>,
    heading_level: Option<u8>,
    blockquote_depth: usize,

    current_table: Option<TableBlock>,
    current_row: Option<TableRow>,
    current_cell: Option<TableCell>,
    table_column_count: usize,

    style_stack: Vec<RunStyle>,
    list_stack: Vec<ListFrame>,
}

impl ParserState {
    fn style(&self) -> RunStyle {
        self.style_stack.last().copied().unwrap_or_default()
    }

    /// Ensure a paragraph exists to receive text, creating one if needed.
    /// Returns whether this call created the paragraph.
    fn ensure_paragraph(&mut self) -> bool {
        if self.current_paragraph.is_none() {
            self.current_paragraph = Some(ParagraphBlock { runs: Vec::new() });
            true
        } else {
            false
        }
    }

    fn push_run(&mut self, run: Run) {
        if let Some(cell) = self.current_cell.as_mut() {
            cell.runs.push(run);
        } else if let Some(paragraph) = self.current_paragraph.as_mut() {
            paragraph.runs.push(run);
        }
    }

    fn push_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.ensure_paragraph();
        let style = self.style();
        let mut run = new_run();
        run.text.push_str(text);
        run.bold = style.bold;
        run.italic = style.italic;
        self.push_run(run);
    }

    fn push_break(&mut self) {
        self.ensure_paragraph();
        let mut run = new_run();
        run.text.push('\n');
        self.push_run(run);
    }

    fn flush_paragraph_as(&mut self, heading_level: Option<u8>) {
        if let Some(paragraph) = self.current_paragraph.take() {
            if paragraph.runs.is_empty() {
                return;
            }
            match heading_level {
                Some(level) => self.blocks.push(DocBlock::Heading(paragraph, level)),
                None => self.blocks.push(DocBlock::Paragraph(paragraph)),
            }
        }
    }
}

pub fn parse_markdown(md: &str) -> Result<Document> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(md, options);

    let mut state = ParserState {
        blocks: Vec::new(),
        current_paragraph: None,
        paragraph_owner_stack: Vec::new(),
        heading_level: None,
        blockquote_depth: 0,
        current_table: None,
        current_row: None,
        current_cell: None,
        table_column_count: 0,
        style_stack: Vec::new(),
        list_stack: Vec::new(),
    };

    for event in parser {
        match event {
            Event::Start(tag) => handle_start(tag, &mut state),
            Event::End(tag_end) => handle_end(tag_end, &mut state),

            Event::Text(text) => state.push_text(&text),
            Event::Code(text) => state.push_text(&text),

            Event::SoftBreak => state.push_text(" "),
            Event::HardBreak => state.push_break(),
            Event::Rule => {
                // Render a thematic break as an empty paragraph separator.
                state.blocks.push(DocBlock::Paragraph(ParagraphBlock {
                    runs: vec![Run {
                        text: "---".to_string(),
                        ..new_run()
                    }],
                }));
            }
            _ => {}
        }
    }

    // Flush anything left open (malformed/truncated markdown).
    let leftover_heading_level = state.heading_level.take();
    state.flush_paragraph_as(leftover_heading_level);

    Ok(Document {
        blocks: state.blocks,
    })
}

fn list_prefix(state: &ParserState) -> String {
    let depth = state.list_stack.len().saturating_sub(1);
    let indent = "  ".repeat(depth);

    match state.list_stack.last() {
        Some(ListFrame {
            ordered_next: Some(n),
            ..
        }) => format!("{indent}{n}. "),
        Some(ListFrame {
            ordered_next: None, ..
        }) => format!("{indent}- "),
        None => String::new(),
    }
}

fn handle_start(tag: Tag<'_>, state: &mut ParserState) {
    match tag {
        Tag::Heading { level, .. } => {
            state.ensure_paragraph();
            state.heading_level = Some(heading_level_from_tag(level));
        }

        Tag::Paragraph => {
            let created = state.ensure_paragraph();
            state.paragraph_owner_stack.push(created);
        }

        Tag::BlockQuote(_) => {
            state.blockquote_depth += 1;
            let created = state.ensure_paragraph();
            if created {
                state.push_text(&"> ".repeat(state.blockquote_depth));
            }
        }

        Tag::Strong => state.style_stack.push(RunStyle {
            bold: true,
            ..state.style()
        }),
        Tag::Emphasis => state.style_stack.push(RunStyle {
            italic: true,
            ..state.style()
        }),
        Tag::Strikethrough => state.style_stack.push(state.style()),

        Tag::List(start) => {
            state.list_stack.push(ListFrame {
                ordered_next: start,
                depth: state.list_stack.len(),
            });
        }

        Tag::Item => {
            state.ensure_paragraph();
            let prefix = list_prefix(state);
            state.push_text(&prefix);
        }

        Tag::Link { .. } => {
            // Link text is pushed via subsequent Text events; the URL
            // itself has no home in the Run model, so it is dropped.
        }

        Tag::Image { .. } => {
            // Alt text arrives as a Text event inside the tag; the image
            // data itself is not representable here.
        }

        Tag::CodeBlock(_) => {
            state.ensure_paragraph();
        }

        Tag::Table(_) => {
            state.current_table = Some(TableBlock {
                rows: Vec::new(),
                column_widths: Vec::new(),
            });
            state.table_column_count = 0;
        }

        Tag::TableHead => {
            state.current_row = Some(TableRow { cells: Vec::new() });
        }

        Tag::TableRow => {
            state.current_row = Some(TableRow { cells: Vec::new() });
        }

        Tag::TableCell => {
            state.current_cell = Some(TableCell {
                runs: Vec::new(),
                width: None,
            });
        }

        _ => {}
    }
}

fn handle_end(tag_end: TagEnd, state: &mut ParserState) {
    match tag_end {
        TagEnd::Heading(_) => {
            let level = state.heading_level.take();
            state.flush_paragraph_as(level);
        }

        TagEnd::Paragraph => {
            let owned = state.paragraph_owner_stack.pop().unwrap_or(true);
            if owned {
                state.flush_paragraph_as(None);
            }
        }

        TagEnd::BlockQuote(_) => {
            if state.blockquote_depth > 0 {
                state.blockquote_depth -= 1;
            }
            if state.blockquote_depth == 0 {
                state.flush_paragraph_as(None);
            }
        }

        TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough => {
            state.style_stack.pop();
        }

        TagEnd::List(_) => {
            state.list_stack.pop();
        }

        TagEnd::Item => {
            state.flush_paragraph_as(None);
        }

        TagEnd::CodeBlock => {
            state.flush_paragraph_as(None);
        }

        TagEnd::Table => {
            if let Some(table) = state.current_table.take() {
                // Column count is inferred from the widest row; explicit
                // widths aren't available from markdown source.
                let mut table = table;
                table.column_widths = vec![0; state.table_column_count];
                state.blocks.push(DocBlock::Table(table));
            }
        }

        TagEnd::TableHead => {
            if let Some(row) = state.current_row.take() {
                state.table_column_count = state.table_column_count.max(row.cells.len());
                if let Some(table) = state.current_table.as_mut() {
                    table.rows.push(row);
                }
            }
        }

        TagEnd::TableRow => {
            if let Some(row) = state.current_row.take() {
                state.table_column_count = state.table_column_count.max(row.cells.len());
                if let Some(table) = state.current_table.as_mut() {
                    table.rows.push(row);
                }
            }
        }

        TagEnd::TableCell => {
            if let Some(cell) = state.current_cell.take() {
                if let Some(row) = state.current_row.as_mut() {
                    row.cells.push(cell);
                }
            }
        }

        _ => {}
    }
}
