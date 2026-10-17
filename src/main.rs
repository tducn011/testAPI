mod api_client;
mod cli;
mod detector;
mod ui;
mod updater;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "expiry-ctl")]
#[command(author = "Expiry DevOps Team")]
#[command(version = "1.0.2")]
#[command(about = "Native Multiplatform Server Identity, Docker Inspector & API Workbench", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "https://bathrooms-boost-breaking-atlas.trycloudflare.com")]
    target: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the interactive Postman-like Terminal TUI (for power users)
    Tui,
    /// Inspect server identity, Tailscale IP, Docker containers, Cloudflare tunnel, and check updates
    Status,
    /// Run the automated 10-stage API behavioral & contract test suite
    Test,
    /// Check GitHub releases and update expiry-ctl to the latest version
    Update,
    /// Launch the guided friendly wizard menu (default)
    Wizard,
    /// Trigger Antigravity Dual-Model Pipeline (High Architect -> Low Builder)
    Agy {
        #[arg(short, long)]
        task: Option<String>,
    },
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
        Some(Commands::Update) => {
            let client = reqwest::Client::new();
            if let Err(e) = updater::Updater::perform_update(&client).await {
                eprintln!("Update failed: {e}");
            }
        }
        Some(Commands::Tui) => {
            let mut app = ui::TuiApp::new().await;
            app.run().await?;
        }
        Some(Commands::Agy { task }) => {
            cli::CliHandler::trigger_antigravity_pipeline(&cli.target, task.as_deref()).await;
        }
        Some(Commands::Wizard) | None => {
            cli::CliHandler::run_wizard(&cli.target).await;
        }
    }

    Ok(())
}
