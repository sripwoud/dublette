pub mod cli;

use cli::Args;

pub fn run(_args: &Args) -> eyre::Result<bool> {
    Ok(false)
}
