use anyhow::{Context, Result};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use std::{fs, path::PathBuf};

mod app;
mod config;
mod document;
mod docx_reader;
mod dump;
mod md_reader;
mod render;
mod table;

#[derive(Parser)]
#[command(
    name = "docx_tools",
    version,
    about = "View and inspect file formats in the terminal"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Open the interactive terminal viewer
    View {
        file: PathBuf,

        /// Treat the file as this format instead of guessing from its extension
        /// (useful for extensionless files, or files with a misleading extension)
        #[arg(long = "as", value_enum)]
        format_override: Option<InputFormat>,



        /// quit after init 
        #[arg(long = "quit-init", action = ArgAction::SetTrue)]
        quit_init: bool,
    },

    /// Dump the document's content to a file
    Dump {
        file: PathBuf,

        /// Where to write the output (defaults to <file>.txt, or <file>.md with --format markdown)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Print the output to the terminal instead of writing a file
        #[arg(long, action = ArgAction::SetTrue)]
        here: bool,

        /// Output format. Defaults to markdown if the output file ends in .md, text otherwise.
        #[arg(short, long, value_enum)]
        format: Option<OutputFormat>,

        /// Treat the input file as this format instead of guessing from its extension
        /// (useful for extensionless files, or files with a misleading extension)
        #[arg(long = "as", value_enum)]
        format_override: Option<InputFormat>,
    },
}

/// The format a document is *read from*. Normally inferred from the input
/// file's extension, but can be overridden per-command with `--as` when the
/// extension is missing or misleading.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum InputFormat {
    Docx,
    Markdown,
}

impl InputFormat {
    fn from_path(path: &PathBuf) -> Result<Self> {
        match path.extension().and_then(|e| e.to_str()) {
            Some(ext) if ext.eq_ignore_ascii_case("docx") => Ok(InputFormat::Docx),
            Some(ext) if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown") => {
                Ok(InputFormat::Markdown)
            }
            Some(ext) => anyhow::bail!(
                "unsupported input file type: .{ext} (use --as to specify the format explicitly)"
            ),
            None => anyhow::bail!(
                "file has no extension; use --as to specify the format explicitly"
            ),
        }
    }
}

/// The format a document is *written to*. Only `Dump` produces output, so
/// only `Dump` has this flag.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Text,
    Markdown,
}

impl OutputFormat {
    fn extension(self) -> &'static str {
        match self {
            OutputFormat::Text => "txt",
            OutputFormat::Markdown => "md",
        }
    }

    /// Infer from the requested output path's extension, defaulting to Text.
    fn from_output_path(path: Option<&PathBuf>) -> Self {
        let is_markdown = path
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"));

        if is_markdown {
            OutputFormat::Markdown
        } else {
            OutputFormat::Text
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {

        Command::View { file, format_override, quit_init } => {
            let document = read_document(&file, format_override)?;
            let file_name = file.file_name().and_then(|f| f.to_str()).context("invalid file name")?.to_owned();

            let themes_dir = config::themes_dir()?;
            config::install_default_themes(&themes_dir)?;

            app::run_tui(document, themes_dir, &file_name, quit_init)?;
        },
        Command::Dump {
            file,
            output,
            here,
            format,
            format_override,
        } => {
            let document = read_document(&file, format_override)?;

            let format = format.unwrap_or_else(|| OutputFormat::from_output_path(output.as_ref()));
            let out_path = output.unwrap_or_else(|| file.with_extension(format.extension()));

            let text = match format {
                OutputFormat::Text => dump::document_to_text(&document),
                OutputFormat::Markdown => dump::document_to_markdown(&document),
            };

            if here {
                print!("{text}");
                return Ok(());
            }

            fs::write(&out_path, text)
                .with_context(|| format!("failed to write {}", out_path.display()))?;

            println!("Wrote {}", out_path.display());
        }
    }

    Ok(())
}

/// Reads any supported document type, using `override_format` if given and
/// otherwise sniffing the input file's extension. Adding a new input format
/// only requires a new `InputFormat` variant and a new match arm here — no
/// other CLI surface changes needed.
fn read_document(path: &PathBuf, override_format: Option<InputFormat>) -> Result<document::Document> {
    if !path.exists() {
        anyhow::bail!("file does not exist: {}", path.display());
    }

    let format = match override_format {
        Some(format) => format,
        None => InputFormat::from_path(path)?,
    };

    match format {
        InputFormat::Docx => docx_reader::read_docx(path),
        InputFormat::Markdown => md_reader::read_md(path),
    }
}
