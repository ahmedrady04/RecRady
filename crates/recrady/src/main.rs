//! Command line entry for `recrady image` and `recrady recover`.

use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use core_imaging::ImageReport;
use fs_fat::{Recovery, SkipReason};
use recrady::CliError;

fn main() -> ExitCode {
    match run() {
        Ok(outcome) => {
            let printed = match outcome {
                Outcome::Image(report) => print_image(&report),
                Outcome::Recover(report) => print_recovery(&report),
            };
            match printed {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    let _ = writeln!(io::stderr(), "error: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(error) => {
            let _ = writeln!(io::stderr(), "error: {error}");
            ExitCode::FAILURE
        }
    }
}

enum Outcome {
    Image(ImageReport),
    Recover(Recovery),
}

fn run() -> Result<Outcome, CliError> {
    let mut args = env::args_os().skip(1);
    let command = args.next();
    let source = args.next();
    let destination = args.next();
    if args.next().is_some() {
        return Err(CliError::Usage);
    }
    match (command.as_deref(), source, destination) {
        (Some(command), Some(source), Some(destination)) if command == "image" => {
            recrady::image_files(Path::new(&source), Path::new(&destination)).map(Outcome::Image)
        }
        (Some(command), Some(source), Some(destination)) if command == "recover" => {
            recrady::recover_files(Path::new(&source), Path::new(&destination))
                .map(Outcome::Recover)
        }
        _ => Err(CliError::Usage),
    }
}

fn print_image(report: &ImageReport) -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "bytes_written: {}", report.bytes_written)?;
    if report.bad_ranges.is_empty() {
        writeln!(out, "bad_ranges: none")?;
    } else {
        for range in &report.bad_ranges {
            writeln!(out, "bad_range: offset {} len {}", range.offset, range.len)?;
        }
    }
    writeln!(out, "sha256: {}", report.sha256_hex())?;
    Ok(())
}

fn print_recovery(report: &Recovery) -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(
        out,
        "note: FAT32 deletion clears the cluster chain and the first 8.3 character"
    )?;
    for file in &report.files {
        writeln!(
            out,
            "recovered: {} ({} bytes)",
            file.relative_path,
            file.bytes.len()
        )?;
    }
    for skipped in &report.skipped {
        writeln!(
            out,
            "skipped: {} ({})",
            skipped.relative_path,
            skip_label(skipped.reason)
        )?;
    }
    if report.files.is_empty() && report.skipped.is_empty() {
        writeln!(out, "recovered: none")?;
    }
    Ok(())
}

fn skip_label(reason: SkipReason) -> &'static str {
    match reason {
        SkipReason::ClusterReused => "cluster reused",
        SkipReason::NoCluster => "no starting cluster",
        SkipReason::DoesNotFit => "size does not fit the volume",
    }
}
