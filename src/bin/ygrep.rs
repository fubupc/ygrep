use std::{io, process};

use clap::{ArgAction, Parser};
use ygrep::search_path;

#[derive(Parser)]
struct Cli {
    /// Pattern
    pattern: String,

    /// Paths
    paths: Vec<String>,

    /// Output with JSON format
    #[arg(long)]
    json: bool,

    /// Follow symlink
    #[arg(short='L', long, action=ArgAction::SetFalse)]
    follow_symlink: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let pattern = regex::bytes::Regex::new(&cli.pattern)?;

    let mut err_occured = false;
    for path in cli.paths {
        if let Err(_) = search_path(&pattern, &path, cli.follow_symlink, &mut io::stdout()) {
            err_occured = true;
        }
    }

    if err_occured {
        process::exit(2);
    }
    Ok(())
}
