use colored::Colorize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub download_url: Option<String>,
}

pub struct Updater;

impl Updater {
    pub async fn check_latest(client: &reqwest::Client) -> UpdateInfo {
        let current = env!("CARGO_PKG_VERSION").to_string();
        let api_url = "https://api.github.com/repos/tducn011/testAPI/releases/latest";

        let resp = client
            .get(api_url)
            .header("User-Agent", "expiry-ctl")
            .timeout(std::time::Duration::from_millis(3000))
            .send()
            .await;

        let mut latest = current.clone();
        let mut download_url = None;

        if let Ok(r) = resp {
            if let Ok(json) = r.json::<Value>().await {
                if let Some(tag) = json.get("tag_name").and_then(|v| v.as_str()) {
                    let clean_tag = tag.trim_start_matches('v').to_string();
                    latest = clean_tag.clone();

                    // Find download url matching platform
                    if let Some(assets) = json.get("assets").and_then(|v| v.as_array()) {
                        let target_asset_name = if cfg!(windows) { "expiry-ctl.exe" } else { "expiry-ctl" };
                        for asset in assets {
                            if let Some(name) = asset.get("name").and_then(|v| v.as_str()) {
                                if name.contains(target_asset_name) {
                                    if let Some(url) = asset.get("browser_download_url").and_then(|v| v.as_str()) {
                                        download_url = Some(url.to_string());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let has_update = Self::is_newer(&latest, &current);

        UpdateInfo {
            current_version: current,
            latest_version: latest,
            has_update,
            download_url,
        }
    }

    fn is_newer(latest: &str, current: &str) -> bool {
        let parse_semver = |s: &str| -> (u32, u32, u32) {
            let mut parts = s.split('.');
            let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
            let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
            let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
            (major, minor, patch)
        };

        parse_semver(latest) > parse_semver(current)
    }

    pub async fn perform_update(client: &reqwest::Client) -> Result<(), String> {
        let info = Self::check_latest(client).await;
        if !info.has_update {
            println!("{} You are already on the latest version (v{}).", "✓".green(), info.current_version);
            return Ok(());
        }

        println!("🚀 New version detected: v{} (Current: v{})", info.latest_version.bold().green(), info.current_version);

        let url = info.download_url.ok_or_else(|| "Could not find release binary asset for your platform on GitHub.".to_string())?;
        println!("⬇ Downloading from: {}", url.cyan());

        let bytes = client
            .get(&url)
            .header("User-Agent", "expiry-ctl")
            .send()
            .await
            .map_err(|e| format!("Download request failed: {e}"))?
            .bytes()
            .await
            .map_err(|e| format!("Failed to read binary bytes: {e}"))?;

        let current_exe = std::env::current_exe().map_err(|e| format!("Failed to locate current executable: {e}"))?;
        let backup_path = current_exe.with_extension("old");
        let new_path = current_exe.with_extension("new");

        // Write to new path
        std::fs::write(&new_path, &bytes).map_err(|e| format!("Failed to write downloaded binary: {e}"))?;

        // On Unix, ensure executable bit is set
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&new_path).map_err(|e| e.to_string())?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&new_path, perms).map_err(|e| e.to_string())?;
        }

        // Rename current to old and new to current
        let _ = std::fs::remove_file(&backup_path);
        if let Err(e) = std::fs::rename(&current_exe, &backup_path) {
            println!("{} Note: Could not replace in-place ({e}). Saved update to {}", "⚠".yellow(), new_path.display());
            return Ok(());
        }

        if let Err(e) = std::fs::rename(&new_path, &current_exe) {
            let _ = std::fs::rename(&backup_path, &current_exe); // rollback
            return Err(format!("Failed to activate new executable: {e}"));
        }

        let _ = std::fs::remove_file(&backup_path);

        println!("🎉 Successfully updated to v{}! Please restart the program.", info.latest_version.bold().green());
        Ok(())
    }
}
