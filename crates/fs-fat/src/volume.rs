use core_device::BlockDevice;

use crate::error::FatError;

/// A FAT32 volume located at a byte offset inside a device or whole-disk image.
#[derive(Debug, Clone)]
pub struct Fat32Volume {
    base: u64,
    bytes_per_sector: u32,
    sectors_per_cluster: u32,
    reserved_sectors: u32,
    root_cluster: u32,
    count_of_clusters: u32,
    first_data_sector: u32,
}

const FAT32_EOF: u32 = 0x0FFF_FFF8;
const ENTRY_MASK: u32 = 0x0FFF_FFFF;

impl Fat32Volume {
    /// Opens the FAT32 filesystem at the start of `device`, or in an MBR/GPT partition.
    pub fn open(device: &dyn BlockDevice) -> Result<Self, FatError> {
        if let Ok(volume) = Self::parse_at(device, 0) {
            return Ok(volume);
        }
        if let Some(volume) = first_in_mbr(device)? {
            return Ok(volume);
        }
        if let Some(volume) = first_in_gpt(device)? {
            return Ok(volume);
        }
        Err(FatError::NotFat32)
    }

    pub(crate) fn bytes_per_cluster(&self) -> u32 {
        self.bytes_per_sector
            .saturating_mul(self.sectors_per_cluster)
    }

    pub(crate) fn root_cluster(&self) -> u32 {
        self.root_cluster
    }

    /// FAT32 entry for `cluster`, with the reserved high 4 bits cleared.
    ///
    /// Microsoft FAT specification section 4: entry 0 means the cluster is free,
    /// and values `>= 0x0FFFFFF8` mean end of the chain.
    pub(crate) fn fat_entry(
        &self,
        device: &dyn BlockDevice,
        cluster: u32,
    ) -> Result<u32, FatError> {
        let fat_start = self
            .base
            .checked_add(u64::from(self.reserved_sectors) * u64::from(self.bytes_per_sector))
            .ok_or(FatError::InvalidBoot)?;
        let offset = fat_start
            .checked_add(u64::from(cluster) * 4)
            .ok_or(FatError::InvalidBoot)?;
        let mut buf = [0u8; 4];
        device.read_at(offset, &mut buf)?;
        Ok(u32::from_le_bytes(buf) & ENTRY_MASK)
    }

    pub(crate) fn cluster_is_free(
        &self,
        device: &dyn BlockDevice,
        cluster: u32,
    ) -> Result<bool, FatError> {
        Ok(self.fat_entry(device, cluster)? == 0)
    }

    pub(crate) fn cluster_is_eoc(&self, entry: u32) -> bool {
        entry >= FAT32_EOF
    }

    pub(crate) fn read_cluster(
        &self,
        device: &dyn BlockDevice,
        cluster: u32,
        buf: &mut [u8],
    ) -> Result<(), FatError> {
        if buf.len() != usize::try_from(self.bytes_per_cluster()).unwrap_or(usize::MAX) {
            return Err(FatError::InvalidBoot);
        }
        let offset = self.cluster_offset(cluster)?;
        device.read_at(offset, buf)?;
        Ok(())
    }

    pub(crate) fn cluster_in_range(&self, cluster: u32) -> bool {
        cluster >= 2 && u64::from(cluster) < u64::from(self.count_of_clusters) + 2
    }

    fn cluster_offset(&self, cluster: u32) -> Result<u64, FatError> {
        if !self.cluster_in_range(cluster) {
            return Err(FatError::InvalidBoot);
        }
        let sector = u64::from(self.first_data_sector)
            + u64::from(cluster - 2) * u64::from(self.sectors_per_cluster);
        self.base
            .checked_add(sector * u64::from(self.bytes_per_sector))
            .ok_or(FatError::InvalidBoot)
    }

    fn parse_at(device: &dyn BlockDevice, base: u64) -> Result<Self, FatError> {
        let mut prefix = [0u8; 512];
        device.read_at(base, &mut prefix)?;
        let bytes_per_sector = u16_at(&prefix, 11)? as u32;
        if !matches!(bytes_per_sector, 512 | 1024 | 2048 | 4096) {
            return Err(FatError::InvalidBoot);
        }
        let boot = if bytes_per_sector == 512 {
            prefix.to_vec()
        } else {
            let mut full = vec![0u8; bytes_per_sector as usize];
            device.read_at(base, &mut full)?;
            full
        };
        let sig_at = usize::try_from(bytes_per_sector - 2).map_err(|_| FatError::InvalidBoot)?;
        let signature = boot.get(sig_at..sig_at + 2).ok_or(FatError::Truncated)?;
        if signature != [0x55, 0xAA] {
            return Err(FatError::InvalidBoot);
        }

        // Section 3.1 and 3.3 field offsets.
        let sectors_per_cluster = u8_at(&boot, 13)? as u32;
        let reserved_sectors = u16_at(&boot, 14)? as u32;
        let num_fats = u8_at(&boot, 16)? as u32;
        let root_entries = u16_at(&boot, 17)?;
        let total_16 = u16_at(&boot, 19)? as u32;
        let fat_sz_16 = u16_at(&boot, 22)? as u32;
        let total_32 = u32_at(&boot, 32)?;
        let fat_sz_32 = u32_at(&boot, 36)?;
        let fs_ver = u16_at(&boot, 42)?;
        let root_cluster = u32_at(&boot, 44)?;

        if sectors_per_cluster == 0
            || !sectors_per_cluster.is_power_of_two()
            || sectors_per_cluster > 128
            || reserved_sectors == 0
            || num_fats == 0
            || root_entries != 0
            || fat_sz_16 != 0
            || fat_sz_32 == 0
            || fs_ver != 0
            || root_cluster < 2
        {
            return Err(FatError::InvalidBoot);
        }

        let total_sectors = if total_16 != 0 { total_16 } else { total_32 };
        let root_dir_sectors = 0u32;
        let fat_area = num_fats
            .checked_mul(fat_sz_32)
            .ok_or(FatError::InvalidBoot)?;
        let used = reserved_sectors
            .checked_add(fat_area)
            .and_then(|sum| sum.checked_add(root_dir_sectors))
            .ok_or(FatError::InvalidBoot)?;
        let data_sectors = total_sectors
            .checked_sub(used)
            .ok_or(FatError::InvalidBoot)?;
        let count_of_clusters = data_sectors / sectors_per_cluster;
        // Section 3.5: FAT32 is a cluster count of 65525 or more.
        if count_of_clusters < 65_525 {
            return Err(FatError::NotFat32);
        }
        let volume_bytes = u64::from(total_sectors) * u64::from(bytes_per_sector);
        let end = base
            .checked_add(volume_bytes)
            .ok_or(FatError::InvalidBoot)?;
        if end > device.size() {
            return Err(FatError::InvalidBoot);
        }
        Ok(Self {
            base,
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            root_cluster,
            count_of_clusters,
            first_data_sector: used,
        })
    }
}

fn first_in_mbr(device: &dyn BlockDevice) -> Result<Option<Fat32Volume>, FatError> {
    let mut sector = [0u8; 512];
    if device.read_at(0, &mut sector).is_err() {
        return Ok(None);
    }
    if sector.get(510..512) != Some(&[0x55, 0xAA]) {
        return Ok(None);
    }
    for index in 0..4 {
        let start = 446 + index * 16;
        let Some(entry) = sector.get(start..start + 16) else {
            return Ok(None);
        };
        let lba = u32_at(entry, 8)?;
        if lba == 0 {
            continue;
        }
        let offset = u64::from(lba) * 512;
        if let Ok(volume) = Fat32Volume::parse_at(device, offset) {
            return Ok(Some(volume));
        }
    }
    Ok(None)
}

/// UEFI Specification, GPT header: signature at LBA 1, partition array LBA at byte 72.
fn first_in_gpt(device: &dyn BlockDevice) -> Result<Option<Fat32Volume>, FatError> {
    let mut header = [0u8; 512];
    if device.read_at(512, &mut header).is_err() {
        return Ok(None);
    }
    if header.get(..8) != Some(b"EFI PART") {
        return Ok(None);
    }
    let entry_lba = u64_at(&header, 72)?;
    let entry_count = u32_at(&header, 80)?;
    let entry_size = u32_at(&header, 84)?;
    if !(128..=4096).contains(&entry_size) {
        return Ok(None);
    }
    let count = entry_count.min(128);
    for index in 0..count {
        let offset = entry_lba
            .checked_mul(512)
            .and_then(|base| base.checked_add(u64::from(index) * u64::from(entry_size)))
            .ok_or(FatError::InvalidBoot)?;
        let mut entry = vec![0u8; entry_size as usize];
        if device.read_at(offset, &mut entry).is_err() {
            continue;
        }
        if entry
            .get(..16)
            .is_some_and(|guid| guid.iter().all(|byte| *byte == 0))
        {
            continue;
        }
        let start_lba = u64_at(&entry, 32)?;
        if start_lba == 0 {
            continue;
        }
        let volume_offset = start_lba.checked_mul(512).ok_or(FatError::InvalidBoot)?;
        if let Ok(volume) = Fat32Volume::parse_at(device, volume_offset) {
            return Ok(Some(volume));
        }
    }
    Ok(None)
}

fn u8_at(buf: &[u8], offset: usize) -> Result<u8, FatError> {
    buf.get(offset).copied().ok_or(FatError::Truncated)
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

fn u64_at(buf: &[u8], offset: usize) -> Result<u64, FatError> {
    let end = offset.checked_add(8).ok_or(FatError::Truncated)?;
    let bytes = buf.get(offset..end).ok_or(FatError::Truncated)?;
    let mut raw = [0u8; 8];
    raw.copy_from_slice(bytes);
    Ok(u64::from_le_bytes(raw))
}
