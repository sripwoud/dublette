use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::Parser;

use dublette::cli::Args;

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

    for dir in &args.directories {
        if !dir.exists() {
            eprintln!("error: directory '{}' does not exist", dir.display());
            return ExitCode::from(2);
        }
        if !dir.is_dir() {
            eprintln!("error: '{}' is not a directory", dir.display());
            return ExitCode::from(2);
        }
    }

    match dublette::run(&args) {
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
