pub mod cli;
pub mod delete;
pub mod hash;
pub mod report;
pub mod scan;

use cli::Args;

pub fn run(_args: &Args) -> eyre::Result<bool> {
    Ok(false)
}
