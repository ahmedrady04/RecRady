use std::collections::HashSet;

use core_device::BlockDevice;

use crate::error::FatError;
use crate::name::short_name;
use crate::volume::Fat32Volume;

const SCAN_LIMIT: u32 = 200_000;
const LONG_NAME: u8 = 0x0F;
const DIRECTORY: u8 = 0x10;
const VOLUME_ID: u8 = 0x08;

/// A deleted file whose bytes were copied out of the image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredFile {
    /// Path relative to the volume root. The lost 8.3 character is `_`.
    pub relative_path: String,
    /// File bytes. This is the contiguous run starting at the directory's first cluster.
    pub bytes: Vec<u8>,
}

/// A deleted directory entry that was not copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedFile {
    /// Path relative to the volume root.
    pub relative_path: String,
    /// Why the bytes were not returned.
    pub reason: SkipReason,
}

/// Why a deleted entry was not treated as intact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// One of the clusters after the first is still allocated, so the old chain is not free.
    ClusterReused,
    /// The entry has a size but no starting cluster.
    NoCluster,
    /// The size does not fit in the clusters that follow the first cluster.
    DoesNotFit,
}

/// Deleted files found on one volume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovery {
    /// Files whose cluster run was entirely free.
    pub files: Vec<RecoveredFile>,
    /// Entries that were seen and not copied.
    pub skipped: Vec<SkippedFile>,
}

impl Fat32Volume {
    /// Walks live directories and copies deleted files whose clusters are still free.
    pub fn recover_deleted(&self, device: &dyn BlockDevice) -> Result<Recovery, FatError> {
        let mut recovery = Recovery {
            files: Vec::new(),
            skipped: Vec::new(),
        };
        let mut visited = HashSet::new();
        let mut seen = 0u32;
        self.walk(
            device,
            self.root_cluster(),
            "",
            &mut recovery,
            &mut visited,
            &mut seen,
        )?;
        Ok(recovery)
    }

    fn walk(
        &self,
        device: &dyn BlockDevice,
        start: u32,
        prefix: &str,
        recovery: &mut Recovery,
        visited: &mut HashSet<u32>,
        seen: &mut u32,
    ) -> Result<(), FatError> {
        if !self.cluster_in_range(start) || !visited.insert(start) {
            return Ok(());
        }
        let cluster_bytes =
            usize::try_from(self.bytes_per_cluster()).map_err(|_| FatError::InvalidBoot)?;
        let mut cluster = start;
        loop {
            let mut buf = vec![0u8; cluster_bytes];
            self.read_cluster(device, cluster, &mut buf)?;
            let mut offset = 0usize;
            while let Some(end) = offset.checked_add(32)
                && let Some(entry) = buf.get(offset..end)
            {
                offset = end;
                *seen = seen.checked_add(1).ok_or(FatError::ScanLimit)?;
                if *seen > SCAN_LIMIT {
                    return Err(FatError::ScanLimit);
                }
                let first = entry.first().copied().ok_or(FatError::Truncated)?;
                if first == 0x00 {
                    return Ok(());
                }
                let attr = entry.get(11).copied().ok_or(FatError::Truncated)?;
                if attr & 0x3F == LONG_NAME {
                    continue;
                }
                let deleted = first == 0xE5;
                let name = short_name(entry, deleted)?;
                if name == "." || name == ".." {
                    continue;
                }
                let entry_cluster = entry_cluster(entry)?;
                if attr & DIRECTORY != 0 {
                    if !deleted {
                        self.walk(
                            device,
                            entry_cluster,
                            &join_path(prefix, &name),
                            recovery,
                            visited,
                            seen,
                        )?;
                    }
                    continue;
                }
                if attr & VOLUME_ID != 0 || !deleted {
                    continue;
                }
                self.recover_one(device, &join_path(prefix, &name), entry, recovery)?;
            }
            let next = self.fat_entry(device, cluster)?;
            if self.cluster_is_eoc(next) || next == 0 || !self.cluster_in_range(next) {
                return Ok(());
            }
            if !visited.insert(next) {
                return Err(FatError::ClusterLoop);
            }
            cluster = next;
        }
    }

    fn recover_one(
        &self,
        device: &dyn BlockDevice,
        path: &str,
        entry: &[u8],
        recovery: &mut Recovery,
    ) -> Result<(), FatError> {
        let size = u32_at(entry, 28)?;
        let first = entry_cluster(entry)?;
        if u64::from(size) > device.size() {
            recovery.skipped.push(SkippedFile {
                relative_path: path.to_string(),
                reason: SkipReason::DoesNotFit,
            });
            return Ok(());
        }
        if size == 0 {
            recovery.files.push(RecoveredFile {
                relative_path: path.to_string(),
                bytes: Vec::new(),
            });
            return Ok(());
        }
        if first < 2 {
            recovery.skipped.push(SkippedFile {
                relative_path: path.to_string(),
                reason: SkipReason::NoCluster,
            });
            return Ok(());
        }
        let cluster_bytes = self.bytes_per_cluster();
        if cluster_bytes == 0 {
            return Err(FatError::InvalidBoot);
        }
        let needed = size.div_ceil(cluster_bytes);
        let last = match first.checked_add(needed - 1) {
            Some(last) if self.cluster_in_range(last) => last,
            _ => {
                recovery.skipped.push(SkippedFile {
                    relative_path: path.to_string(),
                    reason: SkipReason::DoesNotFit,
                });
                return Ok(());
            }
        };
        for cluster in first..=last {
            if !self.cluster_is_free(device, cluster)? {
                recovery.skipped.push(SkippedFile {
                    relative_path: path.to_string(),
                    reason: SkipReason::ClusterReused,
                });
                return Ok(());
            }
        }
        let mut bytes = vec![0u8; size as usize];
        let mut filled = 0usize;
        for cluster in first..=last {
            let mut buf = vec![0u8; cluster_bytes as usize];
            self.read_cluster(device, cluster, &mut buf)?;
            let remaining = bytes.len().checked_sub(filled).ok_or(FatError::Truncated)?;
            let take = remaining.min(buf.len());
            let Some(dest) = bytes.get_mut(filled..filled + take) else {
                return Err(FatError::Truncated);
            };
            let Some(src) = buf.get(..take) else {
                return Err(FatError::Truncated);
            };
            dest.copy_from_slice(src);
            filled += take;
        }
        recovery.files.push(RecoveredFile {
            relative_path: path.to_string(),
            bytes,
        });
        Ok(())
    }
}

fn entry_cluster(entry: &[u8]) -> Result<u32, FatError> {
    let high = u16_at(entry, 20)? as u32;
    let low = u16_at(entry, 26)? as u32;
    Ok((high << 16) | low)
}

fn join_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}/{name}")
    }
}

fn u16_at(buf: &[u8], offset: usize) -> Result<u16, FatError> {
    let end = offset.checked_add(2).ok_or(FatError::Truncated)?;
    let bytes = buf.get(offset..end).ok_or(FatError::Truncated)?;
    let mut raw = [0u8; 2];
    raw.copy_from_slice(bytes);
    Ok(u16::from_le_bytes(raw))
}

fn u32_at(buf: &[u8], offset: usize) -> Result<u32, FatError> {
    let end = offset.checked_add(4).ok_or(FatError::Truncated)?;
    let bytes = buf.get(offset..end).ok_or(FatError::Truncated)?;
    let mut raw = [0u8; 4];
    raw.copy_from_slice(bytes);
    Ok(u32::from_le_bytes(raw))
}
