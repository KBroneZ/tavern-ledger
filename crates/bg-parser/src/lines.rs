//! Splits a byte stream into log lines with a size cap.
//!
//! Real Power.log lines are a few hundred bytes. A line longer than
//! [`MAX_LINE`] (a corrupt or hostile file) is not buffered: the rest of it
//! is skipped and reported once as [`Chunk::TooLong`], so the game it belongs
//! to is marked unsupported instead of being read with a hole in it.

use std::borrow::Cow;
use std::io::{self, BufRead};

pub const MAX_LINE: usize = 1 << 20;

#[derive(Debug, PartialEq, Eq)]
pub enum Chunk<'a> {
    Line(Cow<'a, str>),
    TooLong,
}

#[derive(Debug, Default)]
pub struct LineSplitter {
    buf: Vec<u8>,
    skipping: bool,
}

impl LineSplitter {
    /// Feeds bytes; calls `on_chunk` for every complete line.
    pub fn push(&mut self, mut bytes: &[u8], on_chunk: &mut dyn FnMut(Chunk<'_>)) {
        while !bytes.is_empty() {
            let newline = bytes.iter().position(|&b| b == b'\n');
            let (part, rest) = match newline {
                Some(i) => (&bytes[..i], Some(&bytes[i + 1..])),
                None => (bytes, None),
            };
            if !self.skipping {
                if self.buf.len() + part.len() > MAX_LINE {
                    self.skipping = true;
                    self.buf.clear();
                    on_chunk(Chunk::TooLong);
                } else {
                    self.buf.extend_from_slice(part);
                }
            }
            match rest {
                Some(rest) => {
                    if !self.skipping {
                        emit(&self.buf, on_chunk);
                    }
                    self.buf.clear();
                    self.skipping = false;
                    bytes = rest;
                }
                None => return,
            }
        }
    }

    /// End of input: a last line without a newline is still a line.
    pub fn finish(&mut self, on_chunk: &mut dyn FnMut(Chunk<'_>)) {
        if !self.skipping && !self.buf.is_empty() {
            emit(&self.buf, on_chunk);
        }
        self.buf.clear();
        self.skipping = false;
    }
}

fn emit(line: &[u8], on_chunk: &mut dyn FnMut(Chunk<'_>)) {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    on_chunk(Chunk::Line(String::from_utf8_lossy(line)));
}

/// Reads a whole input through a [`LineSplitter`].
pub fn for_each_chunk(
    mut input: impl BufRead,
    on_chunk: &mut dyn FnMut(Chunk<'_>),
) -> io::Result<()> {
    let mut splitter = LineSplitter::default();
    loop {
        let data = input.fill_buf()?;
        if data.is_empty() {
            break;
        }
        let n = data.len();
        splitter.push(data, on_chunk);
        input.consume(n);
    }
    splitter.finish(on_chunk);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(pieces: &[&[u8]]) -> Vec<String> {
        let mut out = Vec::new();
        let mut s = LineSplitter::default();
        let mut on = |c: Chunk<'_>| {
            out.push(match c {
                Chunk::Line(l) => l.into_owned(),
                Chunk::TooLong => "<too long>".into(),
            })
        };
        for p in pieces {
            s.push(p, &mut on);
        }
        s.finish(&mut on);
        out
    }

    #[test]
    fn splits_across_pieces_and_strips_cr() {
        assert_eq!(collect(&[b"ab", b"c\r\nde\n", b"f"]), ["abc", "de", "f"]);
    }

    #[test]
    fn a_huge_line_is_reported_once_and_skipped() {
        let huge = vec![b'x'; MAX_LINE + 10];
        assert_eq!(
            collect(&[
                b"a\n",
                &huge[..MAX_LINE / 2],
                &huge[MAX_LINE / 2..],
                b"\nb\n"
            ]),
            ["a", "<too long>", "b"]
        );
    }

    #[test]
    fn a_huge_unterminated_tail_is_not_buffered() {
        let mut s = LineSplitter::default();
        let mut seen = 0;
        let chunk = vec![b'x'; 1 << 16];
        for _ in 0..40 {
            s.push(&chunk, &mut |_| seen += 1);
        }
        assert_eq!(seen, 1);
        assert!(s.buf.capacity() <= MAX_LINE + chunk.len());
    }
}
