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
    let stdout = io::stdout();

    for path in cli.paths {
        for file in FileIter::read(path)? {
            let file = fs::File::open(file?)?;
            search(file, &stdout, &pattern)?;
        }
    }

    Ok(())
}

enum FileIter {
    File(path::PathBuf),
    Dir {
        iter: fs::ReadDir,
        child_iter: Option<Box<FileIter>>,
    },
    Done,
}

impl FileIter {
    fn read<P: AsRef<path::Path>>(path: P) -> io::Result<FileIter> {
        let meta = fs::metadata(&path)?;

        if meta.is_dir() {
            Ok(FileIter::Dir {
                iter: fs::read_dir(&path)?,
                child_iter: None,
            })
        } else {
            Ok(FileIter::File(path.as_ref().to_path_buf()))
        }
    }
}

impl Iterator for FileIter {
    type Item = io::Result<path::PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            FileIter::Done => None,
            FileIter::File(path) => {
                let path = path.clone();
                *self = FileIter::Done; // Iteration end.
                Some(Ok(path))
            }
            FileIter::Dir { iter, child_iter } => {
                if let Some(child_iter_inner) = child_iter {
                    match child_iter_inner.next() {
                        Some(item) => return Some(item),
                        None => *child_iter = None,
                    }
                }

                let Some(next) = iter.next() else {
                    *self = FileIter::Done; // Iteration end.
                    return None;
                };

                let next = match next {
                    Ok(next) => next,
                    Err(err) => return Some(Err(err)),
                };

                match FileIter::read(next.path()) {
                    Ok(iter) => *child_iter = Some(Box::new(iter)),
                    Err(err) => return Some(Err(err)),
                };

                self.next()
            }
        }
    }
}

fn search<R: io::Read, W: io::Write>(
    reader: R,
    mut writer: W,
    pattern: &regex::Regex,
) -> anyhow::Result<()> {
    let reader = io::BufReader::new(reader);

    for line in reader.lines() {
        let line = line?;
        if pattern.is_match(&line) {
            writer.write_all(line.as_bytes())?;
            writer.write(b"\n")?;
        }
    }

    writer.flush()?;

    Ok(())
}
