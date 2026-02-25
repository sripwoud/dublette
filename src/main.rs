use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::Parser;

use imgdedup::cli::Args;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

fn main() -> ExitCode {
    color_eyre::install().ok();

    ctrlc::set_handler(|| {
        if INTERRUPTED.swap(true, Ordering::Relaxed) {
            std::process::exit(130);
        }
        eprintln!("\nInterrupted.");
    })
    .ok();

    let args = Args::parse();

    if !args.directory.exists() {
        eprintln!(
            "error: directory '{}' does not exist",
            args.directory.display()
        );
        return ExitCode::from(2);
    }

    if !args.directory.is_dir() {
        eprintln!("error: '{}' is not a directory", args.directory.display());
        return ExitCode::from(2);
    }

    match imgdedup::run(&args) {
        Ok(found_duplicates) => {
            if args.dry_run && found_duplicates {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("Error: {e:?}");
            ExitCode::FAILURE
        }
    }
}
