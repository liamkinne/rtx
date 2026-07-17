#![cfg_attr(not(test), no_std)]

pub struct LineReader<const N: usize> {
    buf: heapless::Vec<u8, N>,
    ended: bool,
}

impl<const N: usize> LineReader<N> {
    pub fn new() -> Self {
        Self {
            buf: heapless::Vec::new(),
            ended: false,
        }
    }

    /// Feed me data.
    pub fn feed(&mut self, data: &[u8]) -> Result<Option<&[u8]>, Overflow> {
        if self.ended {
            self.buf.clear();
            self.ended = false;
        }

        for &b in data {
            match b {
                b'\n' => {
                    // strip trailing \r if present (CRLF)
                    if !self.buf.is_empty() && self.buf.last() == Some(&b'\r') {
                        let _ = self.buf.pop();
                    }
                    self.ended = true;
                    return Ok(Some(&self.buf));
                }
                _ => self.buf.push(b).map_err(|_| Overflow)?,
            }
        }

        Ok(None)
    }
}

impl<const N: usize> Default for LineReader<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Line length overflow error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overflow;

impl core::fmt::Display for Overflow {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Line length overflow")
    }
}

impl defmt::Format for Overflow {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "Line length overflow")
    }
}

impl core::error::Error for Overflow {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_terminator_returns_none() {
        let mut r: LineReader<64> = LineReader::new();
        assert_eq!(r.feed(b"hello").unwrap(), None);
    }

    #[test]
    fn simple_lf_line() {
        let mut r: LineReader<64> = LineReader::new();
        assert_eq!(r.feed(b"hello\n").unwrap(), Some(&b"hello"[..]));
    }

    #[test]
    fn simple_crlf_line() {
        let mut r: LineReader<64> = LineReader::new();
        assert_eq!(r.feed(b"hello\r\n").unwrap(), Some(&b"hello"[..]));
    }

    #[test]
    fn line_accumulated_over_multiple_feeds() {
        let mut r: LineReader<64> = LineReader::new();
        assert_eq!(r.feed(b"hel").unwrap(), None);
        assert_eq!(r.feed(b"lo").unwrap(), None);
        assert_eq!(r.feed(b"\n").unwrap(), Some(&b"hello"[..]));
    }

    #[test]
    fn crlf_split_across_feed_calls() {
        // \r and \n arrive in separate feed() calls; this should still
        // yield exactly one line, with the \r stripped.
        let mut r: LineReader<64> = LineReader::new();
        assert_eq!(r.feed(b"hello\r").unwrap(), None);
        assert_eq!(r.feed(b"\n").unwrap(), Some(&b"hello"[..]));
    }

    #[test]
    fn empty_line() {
        let mut r: LineReader<64> = LineReader::new();
        assert_eq!(r.feed(b"\n").unwrap(), Some(&b""[..]));
    }

    #[test]
    fn empty_crlf_line() {
        let mut r: LineReader<64> = LineReader::new();
        assert_eq!(r.feed(b"\r\n").unwrap(), Some(&b""[..]));
    }

    #[test]
    fn reader_resets_after_line_and_starts_new_line() {
        let mut r: LineReader<64> = LineReader::new();
        assert_eq!(r.feed(b"first\n").unwrap(), Some(&b"first"[..]));
        // Next feed should start a fresh buffer, not append to the old one.
        assert_eq!(r.feed(b"second").unwrap(), None);
        assert_eq!(r.feed(b"\n").unwrap(), Some(&b"second"[..]));
    }

    #[test]
    fn bare_cr_without_lf_does_not_terminate() {
        // Only \n terminates a line; a lone \r is retained in the buffer
        // (and stripped later if followed by \n).
        let mut r: LineReader<64> = LineReader::new();
        assert_eq!(r.feed(b"a\rb").unwrap(), None);
        assert_eq!(r.feed(b"\n").unwrap(), Some(&b"a\rb"[..]));
    }

    #[test]
    fn buffer_overflow_returns_err() {
        let mut r: LineReader<4> = LineReader::new();
        assert_eq!(r.feed(b"12345"), Err(Overflow));
    }

    #[test]
    fn buffer_exact_capacity_then_terminator_ok() {
        let mut r: LineReader<4> = LineReader::new();
        assert_eq!(r.feed(b"1234").unwrap(), None);
        assert_eq!(r.feed(b"\n").unwrap(), Some(&b"1234"[..]));
    }

    #[test]
    fn multiple_lines_one_at_a_time() {
        let mut r: LineReader<64> = LineReader::new();
        assert_eq!(r.feed(b"one\n").unwrap(), Some(&b"one"[..]));
        assert_eq!(r.feed(b"two\n").unwrap(), Some(&b"two"[..]));
        assert_eq!(r.feed(b"three\n").unwrap(), Some(&b"three"[..]));
    }

    #[test]
    fn new_reader_is_not_ended() {
        let r: LineReader<64> = LineReader::new();
        assert_eq!(r.ended, false);
        assert_eq!(r.buf.len(), 0);
    }
}
