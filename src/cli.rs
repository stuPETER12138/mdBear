use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    author,
    version,
    about = "A static site generator for Bear Blog style websites."
)]

pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Initialize a new mdbear site")]
    Init {
        #[arg(help = "Name of the new site/project to create")]
        name: String,
    },

    #[command(about = "Build the static site from source files")]
    Build {
        #[arg(
            short,
            long,
            default_value = "config.toml",
            help = "Configuration file to use for building the site"
        )]
        config: String,
    },

    #[command(about = "Serve the site locally with auto-reload")]
    Serve {
        #[arg(
            short,
            long,
            default_value_t = 3000,
            help = "Port number to serve the site on"
        )]
        port: u16,
        #[arg(
            short,
            long,
            default_value = "config.toml",
            help = "Configuration file to use for serving the site"
        )]
        config: String,
    },
}
