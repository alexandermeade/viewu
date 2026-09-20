use anyhow::{Context, Result};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use std::{fs, path::PathBuf};

mod app;
mod config;
mod document;
mod docx_reader;
mod dump;
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
    View { file: PathBuf },

    /// Dump the document's content to a file
    Dump {
        file: PathBuf,

        /// Where to write the output (defaults to <file>.txt, or <file>.md with --format markdown)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Writes the content of the output into the terminal
        #[arg(long, action = ArgAction::SetTrue)]
        here: Option<bool>,

        /// Output format. Defaults to markdown if the output file ends in .md, text otherwise.
        #[arg(short, long, value_enum)]
        format: Option<Format>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Markdown,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::View{ file } => {
            let document = read_docx(&file)?;

            let file_name = file
                .file_name()
                .and_then(|f| f.to_str())
                .context("invalid file name")?
                .to_owned();

            app::run_tui(document, PathBuf::from("themes"), &file_name)?;
        }

        Command::Dump{file, output, here, format} => {
            let document = read_docx(&file)?;

            let format = format.unwrap_or_else(|| {
                let is_markdown = output
                    .as_ref()
                    .and_then(|p| p.extension())
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| {
                        e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown")
                    });

                if is_markdown {
                    Format::Markdown
                } else {
                    Format::Text
                }
            });

            let default_ext = match format {
                Format::Text => "txt",
                Format::Markdown => "md",
            };
            let out_path = output.unwrap_or_else(|| file.with_extension(default_ext));

            let text = match format {
                Format::Text => dump::document_to_text(&document),
                Format::Markdown => dump::document_to_markdown(&document),
            };

            if let Some(paste_here) = here
                && paste_here
            {
                print!("{}", text);
                return Ok(());
            }

            fs::write(&out_path, text)
                .with_context(|| format!("failed to write {}", out_path.display()))?;

            println!("Wrote {}", out_path.display());
        }
    }

    Ok(())
}

fn read_docx(path: &PathBuf) -> Result<document::Document> {
    if !path.exists() {
        anyhow::bail!("file does not exist: {}", path.display());
    }

    if path.extension().and_then(|x| x.to_str()) != Some("docx") {
        anyhow::bail!("expected a .docx file");
    }

    docx_reader::read_docx(path)
}
