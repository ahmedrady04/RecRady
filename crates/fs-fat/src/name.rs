use crate::error::FatError;

/// Builds an 8.3 display name.
///
/// Microsoft FAT specification section 6: `DIR_Name[0] == 0xE5` means the
/// entry is free, so the original first character is not on disk. Callers
/// pass `deleted` and the first character becomes `_`.
pub(crate) fn short_name(entry: &[u8], deleted: bool) -> Result<String, FatError> {
    let raw = entry.get(..11).ok_or(FatError::Truncated)?;
    let mut stem = String::new();
    let mut ext = String::new();
    for (index, byte) in raw.iter().copied().enumerate() {
        let shown = if index == 0 && deleted { b'_' } else { byte };
        if shown == b' ' {
            continue;
        }
        let ch = if shown.is_ascii_graphic() && shown != b'/' && shown != b'\\' {
            char::from(shown)
        } else {
            '_'
        };
        if index < 8 {
            stem.push(ch);
        } else {
            ext.push(ch);
        }
    }
    if stem.is_empty() {
        stem.push('_');
    }
    if ext.is_empty() {
        Ok(stem)
    } else {
        Ok(format!("{stem}.{ext}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleted_short_name_loses_only_the_first_character() {
        let mut entry = [b' '; 32];
        entry[0] = 0xE5;
        entry[1..5].copy_from_slice(b"ELLO");
        entry[8..11].copy_from_slice(b"TXT");
        assert_eq!(short_name(&entry, true).unwrap(), "_ELLO.TXT");
    }
}
