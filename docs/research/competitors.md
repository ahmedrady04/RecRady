# Competitor and license research

Date: 2026-11-10. This is an engineering note, not legal advice. A lawyer should confirm anything we ship that links third-party code.

Commercial products below (R-Studio, Disk Drill, EaseUS, UFS Explorer, Stellar, Recoverit, DMDE, Recuva, ReclaiMe, GetDataBack) publish no source. They were read as product claims, not as code. Open-source behavior was checked on GitHub and crates.io the same day.

## Do not promise

These are limits of storage, not missing features.

- **SSD TRIM / NVMe deallocate.** After the drive erases a block, the bytes are gone. A name may still sit in metadata. The file body often will not.
- **FileVault, and the internal disk of a T2 or Apple silicon Mac.** Apple's platform security guide says that without the password or the recovery key the internal APFS volumes stay encrypted, even if the storage is moved to another computer. On those machines the volume is encrypted by the Secure Enclave even when FileVault was never turned on. External disks are the realistic macOS target.
- **BitLocker and APFS encryption** without the key. libbde and libfsapfs do not remove that requirement, and both are LGPL.
- **iOS.** Only backups and data the device already exposes. No bypass of the device passcode.
- **Overwritten clusters, and RAID layouts the vendor never published.**

## Feasibility

"Next" means the following implementation steps. "Later" is real but not next. "Never claim" means the product must say so in the UI.

| Capability | Who does it well | Verdict | Why |
| --- | --- | --- | --- |
| RAW image, skip bad sectors, hash the output | R-Studio, UFS Explorer | Next | Reads go through `BlockDevice`. Bad ranges become zeros in the destination image only. SHA-256 is a normal hash of that output. |
| FAT12/16/32 deleted files | Most products | Next | Microsoft FAT specification: `DIR_Name[0] == 0xE5` marks the entry free. The rest often survives until reused. Best effort, tested on images. |
| exFAT deleted files | Most products | Next, best effort | Microsoft exFAT spec: `InUse == 0` means unused, and the other fields are **undefined**. Do not assume the name and cluster survive. Prove it on fixture images. |
| NTFS deleted files | R-Studio, DMDE, GetDataBack | Next | Microsoft MFT notes: a record has an in-use flag; `$DATA` runs hold the cluster map, including fragmented files. Use the `ntfs` crate for structures. Deleted-record scan is ours. |
| Signature carving | PhotoRec, R-Studio | Next | Our signature list, taken from format specifications. Do not copy PhotoRec's signature file (GPL-2.0). |
| Preview, filters, a reason for each hit | Disk Drill, EaseUS | With the scanner | A hit is "name from a directory entry", "clusters still free", or "signature only". Not a fake percentage. |
| Pause, resume, saved scan | R-Studio | Design before the scanner | The scan log has to be serializable from the first scanner. Retrofitting it is expensive. |
| Lost MBR/GPT partitions | TestDisk, DMDE | Later | UEFI GPT and MBR layouts are published. The scan is ours. TestDisk is GPL-2.0 and is not linked. |
| ext2/3/4 | R-Studio, UFS Explorer | Later | `ext4-view` reads a live filesystem (MIT or Apache-2.0). Deleted names live in directory slack; that scanner is ours. |
| HFS+ / APFS, unencrypted | Disk Drill, UFS Explorer | Later | No complete APFS spec from Apple. `apfs` 0.4 and `hfsplus` 0.4 are MIT and read-only; APFS build has no encryption, no compression, first volume only. Treat them as readers to evaluate, not as the recovery engine. |
| Fragmented carving | R-Studio | Later, hard | Metadata run lists do this for NTFS. Carving without metadata is a research problem. Do not promise PhotoRec-style results on fragmented video. |
| Photo/video repair | Stellar, Recoverit | Later, narrow | Rebuilding a broken header is possible. Repairing a truncated encode is not. |
| RAID, NAS | R-Studio, UFS Explorer, ReclaiMe | Much later | Many layouts are unpublished. A wrong reconstruction writes nothing (we are read-only) but it can invent files. Ship it only with known layouts and tests. |
| E01 / EWF | R-Studio | Much later | `libewf` is LGPL-3.0-or-later. Not linked. RAW first. |
| Android ADB, iOS backups | Separate products | Separate track | Authorized access only. |

## Licenses

Policy in `deny.toml` stays: MIT, Apache-2.0, BSD, ISC, Zlib, Unicode-3.0. GPL, AGPL, and LGPL fail CI. CPL / IBM-PL are also not on the allow list.

| Component | License checked | Use |
| --- | --- | --- |
| TestDisk, PhotoRec ([COPYING](https://github.com/cgsecurity/testdisk/blob/master/COPYING)) | GPL-2.0 | Ideas only. No source, no signature file. |
| The Sleuth Kit core ([licenses page](https://sleuthkit.org/sleuthkit/licenses.php)) | CPL-1.0 and IBM-PL-1.0. `strings` is GPL-2.0 and is not part of the library. | External test oracle. Run the binary, compare output. Do not link. |
| libfsntfs, libfsapfs, libewf, libbde | LGPL-3.0-or-later (GitHub, 2026-11-10). libfsapfs has no T2 encryption, no fusion, no snapshots. | Reference. Dynamic linking still carries LGPL-3 duties, including on a signed Mac app. Needs a lawyer before any exception. |
| `ntfs` 0.4.0 (Colin Finck) | MIT or Apache-2.0 | Candidate for reading NTFS structures. Last release 2023-06. It is a reader, not a deleted-file product. |
| `ext4-view` 1.0 | MIT or Apache-2.0 | Candidate later, read-only. |
| `fatfs` 0.3.6 | MIT | Reader only. It can write; recovery code must not call write methods. Deleted entries are not its job. |
| `apfs` 0.4.1, `hfsplus` 0.4.0 | MIT | Evaluate later. Young, documented gaps. |
| `exfat` (Obliteration), `exfat-slim` | MIT, Apache-2.0 | Readers. Neither is a recovery scanner. |
| `ntfs-reader` 0.6 | MIT or Apache-2.0 | Documents deleted MFT records, but it is Windows-only. Does not fit the shared engine. |
| `unearth` 0.4, `ntfs-forensic` 0.8 | MIT, Apache-2.0 | Not dependencies. One release, almost no use, and the claims are broader than the specs. |

## UX gaps worth building for

- Consumer tools start a long scan and explain TRIM or FileVault afterwards. Say it on the device screen, before the scan.
- PhotoRec returns bytes without the original name and without a preview. Pair carving with metadata names when the entry still exists.
- R-Studio, DMDE, and UFS Explorer are built for technicians. The same engine can sit behind a short wizard: pick a disk, pick a destination that is not the source, scan, preview, copy.
- "Recovery chance" with no reason is marketing. Show the evidence instead.
- Several products are two applications, one per OS. One Rust engine on both is the gap we can actually hold.

## Acceptance tests for the next steps

1. **Imaging.** A fixture reader errors on one range. The destination image matches the source everywhere else and is zero in that range. Source bytes and mtime are unchanged. The report includes the destination SHA-256.
2. **FAT.** A FAT32 image with one deleted file whose clusters are still free. Recovered bytes match. The name matches the entry, with the known `0xE5` first-byte loss when there is no long name.
3. **exFAT.** Two fixtures: one where the unused entry still holds a name and cluster, and one where those fields are wiped (allowed by the spec). The first recovers. The second does not invent a file.
4. **NTFS.** A deleted file with two fragments. Recovered bytes match, in order. A second file whose clusters were reused is reported, not silently returned as intact.
5. **Carving.** Signatures authored from the format spec, with a test image that has no filesystem. A truncated file is labeled truncated.

## Sources

- [Microsoft FAT specification](https://academy.cba.mit.edu/classes/networking_communications/SD/FAT.pdf) (the `0xE5` rule is in the directory-entry section).
- [Microsoft exFAT specification](https://learn.microsoft.com/en-us/windows/win32/fileio/exfat-specification), sections 6.2 and 6.2.1.4.
- [Microsoft MFT notes](https://learn.microsoft.com/en-us/windows/win32/devnotes/master-file-table) and [NTFS attribute types](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/a82e9105-2405-4e37-b2c3-28c773902d85).
- [Apple, FileVault volume encryption](https://support.apple.com/guide/security/volume-encryption-with-filevault-sec4c6dc1b6e/web).
- [The Sleuth Kit licenses](https://sleuthkit.org/sleuthkit/licenses.php).
- GitHub: [testdisk](https://github.com/cgsecurity/testdisk), [libfsntfs](https://github.com/libyal/libfsntfs), [libfsapfs](https://github.com/libyal/libfsapfs), [libewf](https://github.com/libyal/libewf).
- crates.io on 2026-11-10: `ntfs`, `fatfs`, `ext4-view`, `apfs`, `hfsplus`, `exfat`, `exfat-slim`, `ntfs-reader`, `unearth`, `ntfs-forensic`.
