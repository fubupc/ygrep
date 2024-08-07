use std::io;

pub enum LineDelimiter {
    LF,
    CR,
    CRLF,
    NUL,
}

pub trait BufReadExt: io::BufRead {
    /// Read line by any one of these delimiters: LF, CR, CRLF, NULL. `buf` would NOT include delimiter.
    fn read_line_ext(&mut self, buf: &mut Vec<u8>) -> io::Result<(usize, Option<LineDelimiter>)> {
        read_line_ext(self, buf)
    }

    fn lines_ext(self) -> Lines<Self>
    where
        Self: Sized,
    {
        Lines { buf: self }
    }
}

impl<R: io::BufRead> BufReadExt for R {}

fn read_line_ext<R: BufReadExt + ?Sized>(
    r: &mut R,
    buf: &mut Vec<u8>,
) -> io::Result<(usize, Option<LineDelimiter>)> {
    let mut read = 0;
    loop {
        let data = match r.fill_buf() {
            Ok(a) => a,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        let data_len = data.len();
        if data_len == 0 {
            return Ok((read, None));
        }

        let delim_pos = data.iter().enumerate().find_map(|(pos, b)| match b {
            b'\x00' => Some((LineDelimiter::NUL, pos)),
            b'\n' => Some((LineDelimiter::LF, pos)),
            b'\r' => Some((LineDelimiter::CR, pos)),
            _ => None,
        });

        match delim_pos {
            Some((mut delim, pos)) => {
                let next_byte = data.iter().nth(pos + 1).copied();

                buf.extend_from_slice(&data[..pos]); // Exclude delimiter
                r.consume(pos + 1);
                read += pos + 1;

                if let LineDelimiter::CR = delim {
                    let crlf = match next_byte {
                        Some(b) => b == b'\n',
                        None => {
                            let next_buf = loop {
                                match r.fill_buf() {
                                    Ok(a) => break a,
                                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                                    Err(e) => return Err(e),
                                };
                            };
                            match next_buf {
                                [b, ..] if *b == b'\n' => true,
                                _ => false,
                            }
                        }
                    };

                    if crlf {
                        delim = LineDelimiter::CRLF;
                        read += 1;
                        r.consume(1);
                    }
                }

                return Ok((read, Some(delim)));
            }
            None => {
                buf.extend_from_slice(data);
                r.consume(data_len);
                read += data_len;
            }
        }
    }
}

pub struct Lines<R> {
    buf: R,
}

impl<R: BufReadExt> Iterator for Lines<R> {
    type Item = io::Result<(Vec<u8>, Option<LineDelimiter>)>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf: Vec<u8> = vec![];
        match self.buf.read_line_ext(&mut buf) {
            Ok((0, _)) => None,
            Ok((_, delim)) => Some(Ok((buf, delim))),
            Err(e) => Some(Err(e)),
        }
    }
}
