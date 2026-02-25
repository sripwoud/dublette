pub mod cli;
pub mod scan;

use cli::Args;

pub fn run(_args: &Args) -> eyre::Result<bool> {
    Ok(false)
}
