use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    pub hostname: String,
    pub os: String,
    pub tailscale_ip: Option<String>,
    pub local_ips: Vec<String>,
    pub is_homeserver: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStatus {
    pub name: String,
    pub status: String,
    pub health: String,
    pub ports: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelInfo {
    pub url: String,
    pub active: bool,
    pub latency_ms: Option<u64>,
}

pub struct SystemDetector;

impl SystemDetector {
    pub fn detect_host() -> HostInfo {
        let hostname = Command::new("hostname")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let os = std::env::consts::OS.to_string();

        let tailscale_ip = Command::new("tailscale")
            .arg("ip")
            .arg("-4")
            .output()
            .ok()
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            });

        let local_ips = Command::new("hostname")
            .arg("-I")
            .output()
            .ok()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let is_homeserver = tailscale_ip.as_deref() == Some("100.80.6.17") || hostname.contains("homeserver");

        HostInfo {
            hostname,
            os,
            tailscale_ip,
            local_ips,
            is_homeserver,
        }
    }

    pub fn detect_docker() -> Vec<ContainerStatus> {
        let output = Command::new("docker")
            .args(["ps", "-a", "--format", "{{.Names}}\t{{.Status}}\t{{.Ports}}"])
            .output();

        let mut list = Vec::new();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.is_empty() {
                    continue;
                }
                let name = parts[0].to_string();
                let status_str = parts.get(1).unwrap_or(&"unknown").to_string();
                let ports = parts.get(2).unwrap_or(&"").to_string();

                let health = if status_str.contains("healthy") {
                    "HEALTHY".to_string()
                } else if status_str.contains("unhealthy") {
                    "UNHEALTHY".to_string()
                } else if status_str.starts_with("Up") {
                    "RUNNING".to_string()
                } else {
                    "STOPPED".to_string()
                };

                list.push(ContainerStatus {
                    name,
                    status: status_str,
                    health,
                    ports,
                });
            }
        }

        // If running remotely or without docker daemon, provide fallback probe status
        if list.is_empty() {
            list.push(ContainerStatus {
                name: "expiry-api".to_string(),
                status: "Remote / Probing HTTP".to_string(),
                health: "CHECKING".to_string(),
                ports: "3001".to_string(),
            });
            list.push(ContainerStatus {
                name: "expiry-python-scan".to_string(),
                status: "Remote / Probing HTTP".to_string(),
                health: "CHECKING".to_string(),
                ports: "8000".to_string(),
            });
            list.push(ContainerStatus {
                name: "expiry-mysql84".to_string(),
                status: "Remote / DB Container".to_string(),
                health: "CHECKING".to_string(),
                ports: "3307->3306".to_string(),
            });
        }

        list
    }

    pub async fn detect_tunnel(client: &reqwest::Client) -> TunnelInfo {
        let mut tunnel_url = String::new();

        // 1. Try reading /var/log/cloudflared-tunnel.log if on homeserver Linux
        if let Ok(content) = std::fs::read_to_string("/var/log/cloudflared-tunnel.log") {
            for line in content.lines().rev() {
                if let Some(pos) = line.find("https://") {
                    let sub = &line[pos..];
                    if let Some(end) = sub.find(".trycloudflare.com") {
                        tunnel_url = sub[..end + 18].to_string();
                        break;
                    }
                }
            }
        }

        // 2. Default known active tunnel
        if tunnel_url.is_empty() {
            tunnel_url = "https://bathrooms-boost-breaking-atlas.trycloudflare.com".to_string();
        }

        // 3. Measure latency to tunnel
        let start = std::time::Instant::now();
        let health_url = format!("{}/health", tunnel_url);
        let resp = client.get(&health_url).timeout(std::time::Duration::from_millis(3000)).send().await;

        let (active, latency_ms) = match resp {
            Ok(r) => (r.status().is_success(), Some(start.elapsed().as_millis() as u64)),
            Err(_) => (false, None),
        };

        TunnelInfo {
            url: tunnel_url,
            active,
            latency_ms,
        }
    }
}
