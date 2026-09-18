use ratatui::style::Color;


pub struct Document {
    pub blocks: Vec<DocBlock>,
}

pub enum DocBlock {
    Paragraph(ParagraphBlock),
    Heading(ParagraphBlock, u8),
    Table(TableBlock),
    #[allow(dead_code)]
    PageBreak,
}

#[derive(Debug)]
pub struct ParagraphBlock {
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Option<Color>,
    pub size: Option<u16>,
}

#[derive(Debug)]
pub struct TableBlock {
    pub rows: Vec<TableRow>,
    pub column_widths: Vec<u32>,
}

#[derive(Debug)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug)]
pub struct TableCell {
    pub runs: Vec<Run>,
    pub width: Option<u32>,
}
