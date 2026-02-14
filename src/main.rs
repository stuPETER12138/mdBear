mod cli;
mod cmd;
pub mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Init { name } => {
            cmd::init::execute(&name)?;
        }
        Commands::Build { config } => {
            cmd::build::execute(&config)?;
        }
        Commands::Serve { port, config } => {
            cmd::serve::execute(port, &config).await?;
        }
    }

    Ok(())
}
