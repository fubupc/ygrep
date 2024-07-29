use std::io;

use clap::Parser;

use ygrep::{walk_path, ErrorReporter, Searcher};

#[derive(Parser)]
struct Cli {
    /// Pattern
    pattern: String,

    /// Paths
    paths: Vec<String>,

    /// Output with JSON format
    #[arg(long)]
    json: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let pattern = regex::bytes::Regex::new(&cli.pattern)?;
    let mut searcher = Searcher {
        pattern,
        writer: io::stdout(),
    };

    for path in cli.paths {
        walk_path(&path, &mut searcher, &mut ErrorReporter, false);
    }

    Ok(())
}
