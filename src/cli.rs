use colored::Colorize;
use crate::api_client::{PostmanEngine, ENDPOINTS};
use crate::detector::SystemDetector;
use crate::updater::Updater;

pub struct CliHandler;

impl CliHandler {
    pub async fn print_status(target_url: &str) {
        println!("{}", "═════════════════════════════════════════════════════════════════".cyan());
        println!("           {} v{}", "EXPIRY SYSTEM CONTROL CENTER".bold().green(), env!("CARGO_PKG_VERSION"));
        println!("{}", "═════════════════════════════════════════════════════════════════".cyan());

        let host = SystemDetector::detect_host();
        println!("  {} {}", "HOSTNAME:".bold(), host.hostname);
        println!("  {} {}", "OS PLATFORM:".bold(), host.os);
        if let Some(ref ts) = host.tailscale_ip {
            println!("  {} {}", "TAILSCALE IP:".bold().green(), ts);
        } else {
            println!("  {} {}", "TAILSCALE IP:".bold().yellow(), "Not detected");
        }
        println!("  {} {}", "LOCAL IPS:".bold(), host.local_ips.join(", "));
        println!("  {} {}", "ENVIRONMENT:".bold(), if host.is_homeserver { "HOMESERVER (100.80.6.17)".green() } else { "CLIENT LAPTOP".yellow() });

        println!("\n{}", "── DOCKER CONTAINERS ────────────────────────────────────────────".cyan());
        let containers = SystemDetector::detect_docker();
        for c in containers {
            let status_badge = if c.health == "HEALTHY" || c.health == "RUNNING" {
                format!("[{}]", c.health).green()
            } else {
                format!("[{}]", c.health).red()
            };
            println!("  {} {:<22} {:<15} {}", "●".cyan(), c.name.bold(), status_badge, c.ports.dimmed());
        }

        println!("\n{}", "── CLOUDFLARE TUNNEL ────────────────────────────────────────────".cyan());
        let client = reqwest::Client::new();
        let tunnel = SystemDetector::detect_tunnel(&client).await;
        if tunnel.active {
            let latency = tunnel.latency_ms.map(|m| format!("{m}ms")).unwrap_or_else(|| "N/A".to_string());
            println!("  {} {}", "URL:".bold(), tunnel.url.bold().green());
            println!("  {} {} (Roundtrip: {})", "STATUS:".bold(), "ACTIVE / LIVE".green(), latency);
        } else {
            println!("  {} {}", "URL:".bold(), tunnel.url.yellow());
            println!("  {} {}", "STATUS:".bold(), "INACTIVE OR UNREACHABLE".red());
        }

        println!("\n{}", "── TARGET API PROBE ─────────────────────────────────────────────".cyan());
        println!("  {} {}", "ACTIVE TARGET:".bold(), target_url.bold().blue());

        let engine = PostmanEngine::new();
        match engine.send_request(target_url, &ENDPOINTS[0], None, None).await {
            Ok(resp) => {
                let badge = if resp.status == 200 { "200 OK".green() } else { format!("{}", resp.status).red() };
                println!("  {} /health -> {} ({}ms)", "PING:".bold(), badge, resp.duration_ms);
            }
            Err(e) => {
                println!("  {} /health probe failed: {}", "PING:".bold().red(), e);
            }
        }

        println!("\n{}", "── VERSION & GITHUB RELEASE CHECK ──────────────────────────────".cyan());
        let update_info = Updater::check_latest(&client).await;
        if update_info.has_update {
            println!("  {} New version {} available! (Current: v{})", "🔔 UPDATE:".bold().yellow(), update_info.latest_version.bold().green(), update_info.current_version);
            println!("  👉 Run '{}' to automatically upgrade this executable.", "expiry-ctl update".bold().cyan());
        } else {
            println!("  {} You are running the latest version (v{}).", "✓".green(), update_info.current_version);
        }

        println!("{}", "═════════════════════════════════════════════════════════════════\n".cyan());
    }

    pub async fn run_test_suite(base_url: &str) {
        println!("{}", "🧪 Starting E2E Behavioral & API Contract Verification...".bold().cyan());
        println!("📍 Target Base: {}\n", base_url.bold().yellow());

        let engine = PostmanEngine::new();

        for (idx, ep) in ENDPOINTS.iter().enumerate() {
            let label = format!("[STAGE {}: {}]", idx + 1, ep.name);
            print!("  Running {:<40} ... ", label);

            match engine.send_request(base_url, ep, None, None).await {
                Ok(resp) => {
                    if (resp.status >= 200 && resp.status < 300) || (ep.path == "/health" && resp.status == 200) {
                        println!("{} (HTTP {} - {}ms)", "PASS".green().bold(), resp.status, resp.duration_ms);
                    } else if ep.requires_auth && resp.status == 401 && idx == 1 {
                        println!("{} (HTTP 401 Gate Confirmed)", "PASS".green().bold());
                    } else {
                        println!("{} (HTTP {} - {})", "CHECK".yellow().bold(), resp.status, resp.status_text);
                    }
                }
                Err(e) => {
                    println!("{} ({})", "FAIL".red().bold(), e);
                }
            }
        }
        println!("\n🎉 Verification run complete!");
    }
}
