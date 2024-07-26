use std::{
    fs,
    io::{self, BufRead},
    path,
};

use anyhow::Context;
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
        if let Err(err) = search_in_path(&path, &stdout, &pattern) {
            eprintln!("ygrep: {}: {}", &path, err);
        }
    }

    Ok(())
}

fn search_in_path<P: AsRef<path::Path>, W: io::Write>(
    path: P,
    writer: W,
    pattern: &regex::Regex,
) -> anyhow::Result<()> {
    let ty = fs::metadata(&path)?.file_type();
    if ty.is_file() {
        return search_in_file(&path, writer, &pattern);
    }

    if ty.is_dir() {
        return search_in_dir(path, writer, pattern);
    }

    Ok(())
}

fn search_in_file<P: AsRef<path::Path>, W: io::Write>(
    path: P,
    writer: W,
    pattern: &regex::Regex,
) -> anyhow::Result<()> {
    let file = fs::File::open(&path)?;
    search(&file, writer, &pattern)
}

fn search_in_dir<P: AsRef<path::Path>, W: io::Write>(
    path: P,
    mut writer: W,
    pattern: &regex::Regex,
) -> anyhow::Result<()> {
    for file in FileIter::read_dir(&path)? {
        let file = file?;
        if let Err(err) = search_in_file(&file, &mut writer, &pattern) {
            eprintln!("ygrep: {}: {}", file.display(), err);
        };
    }
    Ok(())
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

struct FileIter {
    dir: path::PathBuf,
    inner: fs::ReadDir,
    child_iter: Option<Box<FileIter>>,
}

impl FileIter {
    fn read_dir<P: AsRef<path::Path>>(dir: P) -> io::Result<FileIter> {
        Ok(FileIter {
            dir: dir.as_ref().to_path_buf(),
            inner: fs::read_dir(&dir)?,
            child_iter: None,
        })
    }
}

impl Iterator for FileIter {
    type Item = anyhow::Result<path::PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        // helper transpose Option<Result<T, E>> to Result<Option<T>, E>
        fn helper(me: &mut FileIter) -> anyhow::Result<Option<path::PathBuf>> {
            if let Some(child_iter) = &mut me.child_iter {
                if let Some(item) = child_iter.next() {
                    return Ok(Some(item?));
                }
                me.child_iter = None; // Clear when child iteration finish.
            }

            while let Some(next) = me.inner.next() {
                let next = next
                    .with_context(|| {
                        format!("Error reading dir entries of `{}`", me.dir.display())
                    })?
                    .path();

                let ty = fs::metadata(&next)
                    .with_context(|| format!("Error reading metadata of `{}`", next.display()))?
                    .file_type();

                if ty.is_file() {
                    return Ok(Some(next));
                }

                if ty.is_dir() {
                    let mut child_iter = FileIter::read_dir(&next).with_context(|| {
                        format!("Error reading dir entries of `{}`", next.display())
                    })?;
                    let child_next = match child_iter.next() {
                        Some(child_next) => child_next?,
                        None => continue,
                    };
                    me.child_iter = Some(Box::new(child_iter)); // Store remaining child iter
                    return Ok(Some(child_next));
                }

                if ty.is_symlink() {
                    panic!("impossible: fs::metadata(path).file_type() cannot be symlink")
                }

                // Ignore other file types e.g. block device, char device, fifo etc.
            }

            // If both self.inner and self.child_iter are empty then iteration has been done.
            Ok(None)
        }

        match helper(self) {
            Ok(next) => match next {
                Some(next) => Some(Ok(next)),
                None => None,
            },
            Err(err) => Some(Err(err)),
        }
    }
}
