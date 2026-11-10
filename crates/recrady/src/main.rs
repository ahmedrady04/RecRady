//! Command line entry for `recrady image`.

use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use core_imaging::ImageReport;
use recrady::CliError;

fn main() -> ExitCode {
    match run() {
        Ok(report) => match print_report(&report) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                let _ = writeln!(io::stderr(), "error: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            let _ = writeln!(io::stderr(), "error: {error}");
            if matches!(error, CliError::Usage) {
                let _ = writeln!(
                    io::stderr(),
                    "usage: recrady image <source.img> <destination.img>"
                );
            }
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ImageReport, CliError> {
    let mut args = env::args_os().skip(1);
    let command = args.next();
    let source = args.next();
    let destination = args.next();
    if args.next().is_some() {
        return Err(CliError::Usage);
    }
    match (command.as_deref(), source, destination) {
        (Some(command), Some(source), Some(destination)) if command == "image" => {
            recrady::image_files(Path::new(&source), Path::new(&destination))
        }
        _ => Err(CliError::Usage),
    }
}

fn print_report(report: &ImageReport) -> io::Result<()> {
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
