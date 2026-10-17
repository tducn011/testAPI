mod api_client;
mod cli;
mod detector;
mod ui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "expiry-ctl")]
#[command(author = "Expiry DevOps Team")]
#[command(version = "1.0.0")]
#[command(about = "Native Multiplatform Server Identity, Docker Inspector & API Workbench", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "https://bathrooms-boost-breaking-atlas.trycloudflare.com")]
    target: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the interactive Postman-like Terminal TUI
    Tui,
    /// Inspect server identity, Tailscale IP, Docker containers, and Cloudflare tunnel
    Status,
    /// Run the automated 13-stage API behavioral & contract test suite
    Test,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Status) => {
            cli::CliHandler::print_status(&cli.target).await;
        }
        Some(Commands::Test) => {
            cli::CliHandler::run_test_suite(&cli.target).await;
        }
        Some(Commands::Tui) | None => {
            let mut app = ui::TuiApp::new().await;
            app.run().await?;
        }
    }

    Ok(())
}
