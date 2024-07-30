use std::{
    fs,
    io::{self, BufRead},
    path,
};

use crate::walk::{walk_path, Error as WalkError};

pub fn search_path<W: io::Write, P: AsRef<path::Path>>(
    pattern: &regex::bytes::Regex,
    path: P,
    follow_symlink: bool,
    writer: &mut W,
) -> Result<(), ()> {
    let iter = walk_path(&path, follow_symlink).map_err(|e| {
        eprintln!("ygrep: {}: {}", path.as_ref().display(), e);
    })?;

    let mut err_occured = false;
    for file in iter {
        if let Err(e) = file.and_then(|file| {
            search_file(pattern, &file, writer).map_err(|e| WalkError::new(e, file))
        }) {
            eprintln!("ygrep: {}: {}", e.path.display(), e.err);
            err_occured = true;
        }
    }

    if err_occured {
        Err(())
    } else {
        Ok(())
    }
}

pub fn search_file<W: io::Write, P: AsRef<path::Path>>(
    pattern: &regex::bytes::Regex,
    file: P,
    writer: &mut W,
) -> Result<(), io::Error> {
    let r = fs::File::open(&file)?;
    search_reader(pattern, r, file, writer)
}

// TODO: To refactor reading line logic to an iterator.
pub fn search_reader<R: io::Read, W: io::Write, P: AsRef<path::Path>>(
    pattern: &regex::bytes::Regex,
    reader: R,
    path: P,
    writer: &mut W,
) -> Result<(), io::Error> {
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

        if pattern.is_match(&buf) {
            if !header_printed {
                header_printed = true;
                writeln!(writer, "{}", path.as_ref().display())?;
            }

            if let Some(pos) = buf.iter().position(|&b| b == b'\0') {
                write!(
                    writer,
                    "binary file matches (found \"\\0\" byte around offset {})\n",
                    offset + pos
                )?;
                break;
            };

            write!(writer, "{}:", line_num)?;
            writer.write_all(&buf)?;
            writer.write(b"\n")?;
        }

        line_num += 1;
        offset += n;
        buf.clear()
    }

    Ok(())
}
