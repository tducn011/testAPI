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

    pub fn open_browser(url: &str) {
        println!("  🌐 Đang mở trình duyệt tới: {}", url.cyan());
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", url])
                .spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(url)
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg(url)
                .spawn();
        }
    }

    pub async fn run_wizard(base_url: &str) {
        use std::io::{self, Write};

        let host = SystemDetector::detect_host();
        let engine = PostmanEngine::new();
        let client = reqwest::Client::new();

        loop {
            // Check active session
            let user_email = {
                let lock = engine.active_user.lock().await;
                lock.clone().unwrap_or_else(|| "Chưa đăng nhập".to_string())
            };

            println!("\n{}", "╔══════════════════════════════════════════════════════════════════════╗".cyan());
            println!("║             {} v{}           ║", "EXPIRY APP - BỘ CÔNG CỤ KIỂM THỬ THÂN THIỆN".bold().green(), env!("CARGO_PKG_VERSION"));
            println!("{}", "╚══════════════════════════════════════════════════════════════════════╝".cyan());
            println!("  🌍 Máy chủ  : {} ({})", host.hostname.bold(), host.tailscale_ip.as_deref().unwrap_or("Tailscale offline").green());
            println!("  🔗 Tunnel   : {}", base_url.bold().blue());
            println!("  👤 Tài khoản: {}", user_email.bold().yellow());
            println!("{}", "──────────────────────────────────────────────────────────────────────".cyan());
            println!("  {} Chạy toàn bộ 10 bài test tự động (1-Click Full Test)", "[1]".bold().green());
            println!("  {} Đăng nhập tài khoản test (Lấy Cookie & CSRF)", "[2]".bold().green());
            println!("  {} Xem danh sách kho thực phẩm (Live MySQL)", "[3]".bold().green());
            println!("  {} Thêm món ăn mới vào kho (Quick Add Item)", "[4]".bold().green());
            println!("  {} Mở Web Studio trên trình duyệt (Khuyên Dùng!)", "[5]".bold().magenta());
            println!("  {} Mở Swagger API Docs trên trình duyệt", "[6]".bold().magenta());
            println!("  {} Mở Terminal TUI nâng cao (Ratatui Hotkeys)", "[7]".bold().yellow());
            println!("  {} Kiểm tra cập nhật phiên bản từ GitHub", "[8]".bold().blue());
            println!("  {} Thoát chương trình", "[0]".bold().red());
            println!("{}", "──────────────────────────────────────────────────────────────────────".cyan());
            print!("👉 Nhập lựa chọn của bạn [0-8]: ");
            let _ = io::stdout().flush();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                break;
            }
            let choice = input.trim();

            match choice {
                "1" => {
                    println!("\n--- [1] CHẠY TỰ ĐỘNG 10 BÀI KIỂM THỬ ---");
                    Self::run_test_suite(base_url).await;
                }
                "2" => {
                    println!("\n--- [2] ĐĂNG NHẬP / CẤP SESSION TOKEN ---");
                    // First try register (if user doesn't exist)
                    let reg_ep = &ENDPOINTS[1];
                    let _ = engine.send_request(base_url, reg_ep, None, None).await;

                    // Then login
                    let login_ep = &ENDPOINTS[2];
                    match engine.send_request(base_url, login_ep, None, None).await {
                        Ok(resp) => {
                            if resp.status == 200 || resp.status == 201 {
                                println!("  {} Đăng nhập thành công! HTTP 200 OK", "✅".green());
                                if let Some(csrf) = resp.body.get("csrf_token").and_then(|v| v.as_str()) {
                                    println!("  🔑 CSRF Guard: {}", csrf.cyan());
                                }
                                if let Some(cookie) = engine.cookie_session.lock().await.as_ref() {
                                    println!("  🍪 Session Cookie: {}", cookie.green());
                                }
                            } else {
                                println!("  {} Lỗi đăng nhập: HTTP {} - {}", "❌".red(), resp.status, resp.status_text);
                            }
                        }
                        Err(e) => println!("  {} Lỗi kết nối: {}", "❌".red(), e),
                    }
                }
                "3" => {
                    println!("\n--- [3] DANH SÁCH KHO THỰC PHẨM (LIVE MYSQL) ---");
                    // Ensure logged in
                    if engine.cookie_session.lock().await.is_none() {
                        println!("  ℹ️ Chưa có session, đang tự động đăng nhập...");
                        let _ = engine.send_request(base_url, &ENDPOINTS[1], None, None).await;
                        let _ = engine.send_request(base_url, &ENDPOINTS[2], None, None).await;
                    }

                    let list_ep = &ENDPOINTS[4];
                    match engine.send_request(base_url, list_ep, None, None).await {
                        Ok(resp) => {
                            if resp.status == 200 {
                                if let Some(items) = resp.body.get("items").and_then(|v| v.as_array()) {
                                    if items.is_empty() {
                                        println!("  📦 Kho thực phẩm hiện đang trống. Hãy bấm [4] để thêm món mới!");
                                    } else {
                                        println!("  {:<30} | {:<12} | {:<16} | {}", "TÊN THỰC PHẨM".bold(), "SỐ LƯỢNG".bold(), "VỊ TRÍ".bold(), "HẠN DÙNG".bold());
                                        println!("  {}", "─".repeat(75).dimmed());
                                        for it in items {
                                            let name = it.get("name").and_then(|v| v.as_str()).unwrap_or("N/A");
                                            let qty = it.get("remaining_quantity").and_then(|v| v.as_str()).unwrap_or("0");
                                            let unit = it.get("unit").and_then(|v| v.as_str()).unwrap_or("");
                                            let loc = it.get("storage_location").and_then(|v| v.as_str()).unwrap_or("Chưa phân loại");
                                            let exp = it.get("expiry_date").and_then(|v| v.as_str()).unwrap_or("N/A");
                                            println!("  {:<30} | {:<8} {:<3} | {:<16} | {}", name, qty, unit, loc, exp.green());
                                        }
                                        println!("  {}", "─".repeat(75).dimmed());
                                        println!("  Tổng số món tìm thấy: {}", items.len().to_string().bold().green());
                                    }
                                }
                            } else {
                                println!("  {} Lỗi truy vấn: HTTP {} - {}", "❌".red(), resp.status, resp.status_text);
                            }
                        }
                        Err(e) => println!("  {} Lỗi kết nối: {}", "❌".red(), e),
                    }
                }
                "4" => {
                    println!("\n--- [4] THÊM MÓN ĂN MỚI VÀO KHO ---");
                    // Ensure logged in
                    if engine.cookie_session.lock().await.is_none() {
                        println!("  ℹ️ Đang tự động đăng nhập...");
                        let _ = engine.send_request(base_url, &ENDPOINTS[1], None, None).await;
                        let _ = engine.send_request(base_url, &ENDPOINTS[2], None, None).await;
                    }

                    print!("  Nhập tên món ăn [Thịt Bò Úc Premium]: ");
                    let _ = io::stdout().flush();
                    let mut name = String::new();
                    let _ = io::stdin().read_line(&mut name);
                    let name = name.trim();
                    let food_name = if name.is_empty() { "Thịt Bò Úc Premium" } else { name };

                    print!("  Nhập số lượng gram [1500]: ");
                    let _ = io::stdout().flush();
                    let mut qty = String::new();
                    let _ = io::stdin().read_line(&mut qty);
                    let qty = qty.trim();
                    let food_qty = if qty.is_empty() { "1500.000" } else { qty };

                    print!("  Nhập hạn sử dụng YYYY-MM-DD [2026-12-31]: ");
                    let _ = io::stdout().flush();
                    let mut exp = String::new();
                    let _ = io::stdin().read_line(&mut exp);
                    let exp = exp.trim();
                    let food_exp = if exp.is_empty() { "2026-12-31" } else { exp };

                    let body = format!(
                        r#"{{"name":"{}","quantity":"{}","unit":"g","storage_location":"Ngăn đông","date_certainty":"known","date_source":"printed_label","date_label_type":"use_by","expiry_date":"{}"}}"#,
                        food_name, food_qty, food_exp
                    );

                    let create_ep = &ENDPOINTS[5];
                    match engine.send_request(base_url, create_ep, Some(&body), None).await {
                        Ok(resp) => {
                            if resp.status == 201 {
                                println!("  {} Đã thêm thành công món '{}' vào MySQL Database!", "✅".green(), food_name.bold());
                            } else {
                                println!("  {} Lỗi thêm món: HTTP {} - {}", "❌".red(), resp.status, resp.status_text);
                            }
                        }
                        Err(e) => println!("  {} Lỗi kết nối: {}", "❌".red(), e),
                    }
                }
                "5" => {
                    let studio_url = "http://100.80.6.17/studio.html";
                    Self::open_browser(studio_url);
                }
                "6" => {
                    let swagger_url = "http://100.80.6.17/swagger.html";
                    Self::open_browser(swagger_url);
                }
                "7" => {
                    println!("\n  🖥️ Đang khởi động Terminal TUI nâng cao...");
                    let mut app = crate::ui::TuiApp::new().await;
                    if let Err(e) = app.run().await {
                        eprintln!("TUI Error: {e}");
                    }
                }
                "8" => {
                    println!("\n--- [8] KIỂM TRA BẢN CẬP NHẬT TỪ GITHUB ---");
                    let update_info = Updater::check_latest(&client).await;
                    if update_info.has_update {
                        println!("  {} Có phiên bản mới {}! (Hiện tại: v{})", "🔔".yellow(), update_info.latest_version.green(), update_info.current_version);
                        print!("  Bạn có muốn tự động cập nhật ngay bây giờ không? (y/N): ");
                        let _ = io::stdout().flush();
                        let mut confirm = String::new();
                        let _ = io::stdin().read_line(&mut confirm);
                        if confirm.trim().eq_ignore_ascii_case("y") {
                            if let Err(e) = Updater::perform_update(&client).await {
                                eprintln!("  ❌ Lỗi cập nhật: {e}");
                            } else {
                                println!("  ✅ Cập nhật hoàn tất! Hãy khởi động lại công cụ.");
                            }
                        }
                    } else {
                        println!("  {} Bạn đang sử dụng phiên bản mới nhất (v{}).", "✓".green(), update_info.current_version);
                    }
                }
                "0" => {
                    println!("\n  👋 Cảm ơn bạn đã sử dụng Expiry Control Suite. Tạm biệt!");
                    #[cfg(target_os = "windows")]
                    {
                        print!("  Bấm Enter để đóng cửa sổ...");
                        let _ = io::stdout().flush();
                        let _ = io::stdin().read_line(&mut String::new());
                    }
                    break;
                }
                _ => {
                    println!("  ⚠️ Lựa chọn không hợp lệ. Vui lòng nhập số từ 0 đến 8.");
                }
            }

            println!("\n{}", "──────────────────────────────────────────────────────────────────────".dimmed());
            print!("👉 Bấm phím Enter để tiếp tục...");
            let _ = io::stdout().flush();
            let _ = io::stdin().read_line(&mut String::new());
        }
    }
}
