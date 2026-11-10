/// A contiguous run of bytes that could not be read.
///
/// Those bytes are zeros in the destination, so later offsets stay aligned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadRange {
    /// Start offset in the source.
    pub offset: u64,
    /// Length in bytes.
    pub len: u64,
}

/// Result of a finished RAW image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageReport {
    /// Bytes written, including zeros substituted for bad ranges.
    pub bytes_written: u64,
    /// Unreadable ranges, merged when they touch.
    pub bad_ranges: Vec<BadRange>,
    /// SHA-256 of the bytes written to the destination.
    pub sha256: [u8; 32],
}

impl ImageReport {
    /// Lowercase hex encoding of [`sha256`](Self::sha256).
    pub fn sha256_hex(&self) -> String {
        let mut out = String::with_capacity(self.sha256.len() * 2);
        for byte in &self.sha256 {
            out.push(hex_digit(byte >> 4));
            out.push(hex_digit(byte & 0x0f));
        }
        out
    }

    /// Total unreadable bytes.
    pub fn bad_bytes(&self) -> u64 {
        self.bad_ranges
            .iter()
            .fold(0u64, |sum, range| sum.saturating_add(range.len))
    }
}

fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0'.wrapping_add(nibble)),
        _ => char::from(b'a'.wrapping_add(nibble.wrapping_sub(10))),
    }
}

/// Appends a bad run, merging it with the previous run when they are adjacent.
pub(crate) fn note_bad(ranges: &mut Vec<BadRange>, offset: u64, len: u64) {
    if len == 0 {
        return;
    }
    if let Some(last) = ranges.last_mut()
        && last
            .offset
            .checked_add(last.len)
            .is_some_and(|end| end == offset)
    {
        last.len = last.len.saturating_add(len);
        return;
    }
    ranges.push(BadRange { offset, len });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_bad_ranges_merge() {
        let mut ranges = Vec::new();
        note_bad(&mut ranges, 512, 512);
        note_bad(&mut ranges, 1024, 512);
        note_bad(&mut ranges, 2048, 512);
        assert_eq!(
            ranges,
            vec![
                BadRange {
                    offset: 512,
                    len: 1024
                },
                BadRange {
                    offset: 2048,
                    len: 512
                },
            ]
        );
    }

    #[test]
    fn empty_digest_is_the_sha256_of_no_bytes() {
        let report = ImageReport {
            bytes_written: 0,
            bad_ranges: Vec::new(),
            sha256: [
                0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
                0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
                0x78, 0x52, 0xb8, 0x55,
            ],
        };
        assert_eq!(
            report.sha256_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(report.bad_bytes(), 0);
    }
}
