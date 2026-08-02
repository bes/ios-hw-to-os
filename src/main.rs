use clap::{Parser, Subcommand};
use std::error::Error;
use std::path::PathBuf;

mod appledb;
mod mappings;
mod source;

const DEFAULT_SOURCE: &str = "source.yaml";
const DEFAULT_MAPPINGS: &str = "mappings.json";

/// Mappings from iOS / iPadOS hardware string to latest supported OS version.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    UpdateSource {
        #[arg(short, long, default_value = DEFAULT_SOURCE)]
        output: PathBuf,
    },
    WriteMappings {
        #[arg(short, long, default_value = DEFAULT_SOURCE)]
        source: PathBuf,
        #[arg(short, long, default_value = DEFAULT_MAPPINGS)]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    match Cli::parse().command {
        Command::UpdateSource { output } => {
            let updated = appledb::fetch_source().await?;
            source::write(&updated, &output).await?;
            let hardware: usize = updated.devices.iter().map(|d| d.hardware.len()).sum();
            eprintln!("Wrote {} ({hardware} hardware strings)", output.display());
        }
        Command::WriteMappings { source, output } => {
            let parsed = source::read(&source).await?;
            mappings::write(&parsed, &output).await?;
            let hardware: usize = parsed.devices.iter().map(|d| d.hardware.len()).sum();
            eprintln!("Wrote {} ({hardware} hardware strings)", output.display());
        }
    }
    Ok(())
}
