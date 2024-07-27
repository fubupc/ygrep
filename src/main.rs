use std::{
    fs,
    io::{self, BufRead},
    path,
};

use clap::Parser;

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

    let pattern = regex::Regex::new(&cli.pattern)?;
    let mut searcher = Searcher {
        pattern,
        writer: io::stdout(),
    };

    for path in cli.paths {
        walk_path(&path, &mut searcher, false);
    }

    Ok(())
}

struct Searcher<W: io::Write> {
    pattern: regex::Regex,
    writer: W,
}

impl<W: io::Write> Searcher<W> {
    fn search<R: io::Read>(&mut self, reader: R) -> anyhow::Result<()> {
        let reader = io::BufReader::new(reader);

        for line in reader.lines() {
            let line = line?;
            if self.pattern.is_match(&line) {
                self.writer.write_all(line.as_bytes())?;
                self.writer.write(b"\n")?;
                self.writer.flush()?;
            }
        }

        Ok(())
    }
}

impl<W: io::Write> Visitor for Searcher<W> {
    fn visit_file<P: AsRef<path::Path>>(&mut self, path: P) -> anyhow::Result<()> {
        let file = fs::File::open(&path)?;
        self.search(&file)
    }

    fn on_error(&self, path: &path::Path, err: anyhow::Error) {
        eprintln!("ygrep: {}: {}", path.display(), err);
    }
}

trait Visitor {
    fn visit_file<P: AsRef<path::Path>>(&mut self, path: P) -> anyhow::Result<()>;
    fn on_error(&self, path: &path::Path, err: anyhow::Error);
}

fn walk_path<P, V>(path: P, visitor: &mut V, follow_symlink: bool)
where
    P: AsRef<path::Path>,
    V: Visitor,
{
    fn walk_path_inner<V>(path: &path::Path, visitor: &mut V, follow_symlink: bool)
    where
        V: Visitor,
    {
        let meta = if follow_symlink {
            fs::metadata(path)
        } else {
            fs::symlink_metadata(path)
        };
        let meta = match meta {
            Ok(meta) => meta,
            Err(err) => {
                visitor.on_error(path, err.into());
                return;
            }
        };

        let ty = meta.file_type();
        if ty.is_file() {
            if let Err(err) = visitor.visit_file(&path) {
                visitor.on_error(path, err.into());
            };
            return;
        }

        if ty.is_dir() {
            let entries = match fs::read_dir(&path) {
                Ok(entries) => entries,
                Err(err) => {
                    visitor.on_error(path, err.into());
                    return;
                }
            };
            for e in entries {
                let e = match e {
                    Ok(e) => e,
                    Err(err) => {
                        visitor.on_error(path, err.into());
                        return;
                    }
                };
                walk_path_inner(&e.path(), visitor, follow_symlink);
            }
        }
    }

    walk_path_inner(path.as_ref(), visitor, follow_symlink)
}
