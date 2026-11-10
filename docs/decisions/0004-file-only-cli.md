# ADR 0004 — File-only CLI

Status: accepted (2026-11-10)

## Context

Imaging can be checked on a real disk image before any user interface exists. Opening `/dev/disk` or `\\.\PhysicalDrive` needs a privileged helper and must not be the first manual test.

## Decision

`recrady image <source.img> <destination.img>` accepts regular files only. Paths under `/dev` and Windows device paths (`\\.\`, `\\?\Volume`, `\\?\UNC`) are rejected before the file is opened. A directory is rejected. The copy still goes through `image_raw`, so the destination cannot be the source.

## Consequences

- A USB image made with another tool can be copied and hashed with this command.
- Live disks are unchanged by this binary because it never opens them.
- Raw device access stays a later step, behind the privileged helper.
