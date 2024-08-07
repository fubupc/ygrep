use std::{
    fs,
    io::{self, BufRead},
    path,
};

use crate::{
    walk::{walk_path, Error as WalkError},
    BufReadExt, LineDelimiter,
};

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

pub fn search_reader<R: io::Read, W: io::Write, P: AsRef<path::Path>>(
    pattern: &regex::bytes::Regex,
    reader: R,
    path: P,
    writer: &mut W,
) -> io::Result<()> {
    let reader = io::BufReader::new(reader);
    let mut header_printed = false;
    let mut offset = 0;

    for (line_num, line) in reader.lines_ext().enumerate() {
        let (line, delim) = line?;

        if let Some(LineDelimiter::NUL) = delim {
            if !header_printed {
                writeln!(writer, "{}", path.as_ref().display())?;
            }
            write!(
                writer,
                "binary file matches (found \"\\0\" byte around offset {})\n",
                offset + line.len()
            )?;
            break;
        }

        if pattern.is_match(&line) {
            if !header_printed {
                header_printed = true;
                writeln!(writer, "{}", path.as_ref().display())?;
            }

            write!(writer, "{}:", line_num)?;
            writer.write_all(&line)?;
            writer.write(b"\n")?;
        }

        offset += line.len();
    }

    Ok(())
}
