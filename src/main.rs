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

struct Searcher<W> {
    pattern: regex::bytes::Regex,
    writer: W,
}

impl<W: io::Write> Searcher<W> {
    fn search_in_reader<R: io::Read>(
        &mut self,
        reader: R,
        path: &path::Path,
    ) -> anyhow::Result<()> {
        let mut reader = io::BufReader::new(reader);
        let mut header_printed = false;
        let mut line_num = 1;

        let mut offset = 0;

        let mut buf = Vec::new();
        while let Ok(n) = reader.read_until(b'\n', &mut buf) {
            if n == 0 {
                break;
            }
            if buf.ends_with(b"\n") {
                buf.pop();
                if buf.ends_with(b"\r") {
                    buf.pop();
                }
            }

            if self.pattern.is_match(&buf) {
                if !header_printed {
                    header_printed = true;
                    write!(self.writer, "{}", path.display())?;
                }

                if let Some(pos) = buf.iter().position(|&b| b == b'\0') {
                    write!(
                        self.writer,
                        "binary file matches (found \"\\0\" byte around offset {})\n",
                        offset + pos
                    )?;
                    break;
                };

                write!(self.writer, "{}:", line_num)?;
                self.writer.write_all(&buf)?;
                self.writer.write(b"\n")?;
            }

            line_num += 1;
            offset += n;
            buf.clear()
        }

        Ok(())
    }
}

// FileSearch
trait FileSearch {
    fn search(&mut self, file: &path::Path) -> anyhow::Result<()>;
}
impl<W: io::Write> FileSearch for Searcher<W> {
    fn search(&mut self, file: &path::Path) -> anyhow::Result<()> {
        let f = fs::File::open(&file)?;
        self.search_in_reader(&f, file.as_ref())
    }
}

trait ErrorHandle {
    fn handle(&mut self, err: anyhow::Error, path: &path::Path);
}
struct ErrorReporter;
impl ErrorHandle for ErrorReporter {
    fn handle(&mut self, err: anyhow::Error, path: &path::Path) {
        eprintln!("ygrep: {}: {}", path.display(), err)
    }
}

fn walk_path<P, S, E>(path: P, searcher: &mut S, err_handler: &mut E, follow_symlink: bool)
where
    P: AsRef<path::Path>,
    S: FileSearch,
    E: ErrorHandle,
{
    // An helper that returns `Result` so `?` can be used internally.
    fn throw_error<V, E>(
        path: &path::Path,
        searcher: &mut V,
        err_handler: &mut E,
        follow_symlink: bool,
        ignore_special_file: bool,
    ) -> anyhow::Result<()>
    where
        V: FileSearch,
        E: ErrorHandle,
    {
        let meta = if follow_symlink {
            fs::metadata(path)
        } else {
            fs::symlink_metadata(path)
        };

        let ty = meta?.file_type();

        // From now on we can ignore symlink.

        if ty.is_file() || (!ty.is_dir() && !ignore_special_file) {
            return searcher.search(&path);
        }

        if ty.is_dir() {
            for e in fs::read_dir(&path)? {
                let e = e?;
                // Ignore special files not at the top level.
                catch_error(&e.path(), searcher, err_handler, follow_symlink, true);
            }
            return Ok(());
        }

        Ok(())
    }

    fn catch_error<V, E>(
        path: &path::Path,
        searcher: &mut V,
        err_handler: &mut E,
        follow_symlink: bool,
        ignore_special_file: bool,
    ) where
        V: FileSearch,
        E: ErrorHandle,
    {
        if let Err(err) = throw_error(
            path,
            searcher,
            err_handler,
            follow_symlink,
            ignore_special_file,
        ) {
            err_handler.handle(err, path);
        }
    }

    // Don't ignore special files at the top level
    catch_error(path.as_ref(), searcher, err_handler, follow_symlink, false);
}
