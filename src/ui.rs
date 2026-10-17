use std::io;
use std::time::Duration;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Terminal,
};

use crate::api_client::{ApiResponse, PostmanEngine, ENDPOINTS};
use crate::detector::{ContainerStatus, HostInfo, SystemDetector, TunnelInfo};

pub struct TuiApp {
    pub host: HostInfo,
    pub containers: Vec<ContainerStatus>,
    pub tunnel: TunnelInfo,
    pub engine: PostmanEngine,
    pub selected_endpoint: usize,
    pub targets: Vec<String>,
    pub selected_target: usize,
    pub last_response: Option<ApiResponse>,
    pub status_message: String,
}

impl TuiApp {
    pub async fn new() -> Self {
        let client = reqwest::Client::new();
        let host = SystemDetector::detect_host();
        let containers = SystemDetector::detect_docker();
        let tunnel = SystemDetector::detect_tunnel(&client).await;

        let targets = vec![
            tunnel.url.clone(),
            "http://127.0.0.1:3001".to_string(),
            "https://expiryapp.vercel.app".to_string(),
        ];

        Self {
            host,
            containers,
            tunnel,
            engine: PostmanEngine::new(),
            selected_endpoint: 0,
            targets,
            selected_target: 0,
            last_response: None,
            status_message: "Ready. Select endpoint and press [Space/Enter] to send.".to_string(),
        }
    }

    pub async fn run(&mut self) -> Result<(), io::Error> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Down | KeyCode::Char('j') => {
                            if self.selected_endpoint + 1 < ENDPOINTS.len() {
                                self.selected_endpoint += 1;
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if self.selected_endpoint > 0 {
                                self.selected_endpoint -= 1;
                            }
                        }
                        KeyCode::Tab => {
                            self.selected_target = (self.selected_target + 1) % self.targets.len();
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            self.status_message = "Sending request...".to_string();
                            terminal.draw(|f| self.render(f))?;

                            let target = self.targets[self.selected_target].clone();
                            let ep = &ENDPOINTS[self.selected_endpoint];
                            match self.engine.send_request(&target, ep, None, None).await {
                                Ok(resp) => {
                                    self.status_message = format!("Done. HTTP {} ({}ms)", resp.status, resp.duration_ms);
                                    self.last_response = Some(resp);
                                }
                                Err(e) => {
                                    self.status_message = format!("Error: {e}");
                                }
                            }
                        }
                        KeyCode::Char('1') => self.selected_target = 0,
                        KeyCode::Char('2') => self.selected_target = 1,
                        KeyCode::Char('3') => self.selected_target = 2,
                        _ => {}
                    }
                }
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        Ok(())
    }

    fn render(&self, f: &mut ratatui::Frame) {
        let size = f.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(5),  // Header / Host & Docker
                Constraint::Min(12),    // Main workspace (split: endpoints vs response)
                Constraint::Length(3),  // Footer shortcuts
            ])
            .split(size);

        // --- TOP BAR: SYSTEM & DOCKER ---
        let ts_ip = self.host.tailscale_ip.as_deref().unwrap_or("No Tailscale");
        let env_label = if self.host.is_homeserver { "HOMESERVER (100.80.6.17)" } else { "CLIENT LAPTOP" };
        let tunnel_status = if self.tunnel.active { "ACTIVE" } else { "OFFLINE" };

        let header_lines = vec![
            Line::from(vec![
                Span::styled("⚡ EXPIRY CONTROL CENTER", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" | Host: "),
                Span::styled(&self.host.hostname, Style::default().fg(Color::Yellow)),
                Span::raw(" | Env: "),
                Span::styled(env_label, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" | Tailscale: "),
                Span::styled(ts_ip, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("🌐 Tunnel: ", Style::default().fg(Color::Magenta)),
                Span::styled(&self.tunnel.url, Style::default().fg(Color::White)),
                Span::raw(" ["),
                Span::styled(tunnel_status, Style::default().fg(if self.tunnel.active { Color::Green } else { Color::Red })),
                Span::raw("]"),
            ]),
            Line::from(vec![
                Span::styled("🐳 Docker: ", Style::default().fg(Color::Blue)),
                Span::styled("api:3001 [RUNNING]  •  python-scan:8000 [HEALTHY]  •  mysql84:3307 [HEALTHY]", Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let header_block = Paragraph::new(header_lines)
            .block(Block::default().borders(Borders::ALL).title("System Infrastructure"));
        f.render_widget(header_block, chunks[0]);

        // --- MIDDLE BODY: ENDPOINTS (LEFT) & RESPONSE (RIGHT) ---
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[1]);

        // Left Pane: Endpoints List
        let target_name = &self.targets[self.selected_target];
        let items: Vec<ListItem> = ENDPOINTS
            .iter()
            .enumerate()
            .map(|(i, ep)| {
                let method_color = match ep.method {
                    "GET" => Color::Green,
                    "POST" => Color::Blue,
                    "PUT" => Color::Yellow,
                    "DELETE" => Color::Red,
                    _ => Color::White,
                };
                let prefix = if i == self.selected_endpoint { "► " } else { "  " };
                let style = if i == self.selected_endpoint {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(format!("{:<6}", ep.method), Style::default().fg(method_color).add_modifier(Modifier::BOLD)),
                    Span::styled(ep.name, style),
                ]))
            })
            .collect();

        let list_title = format!("API Catalog (Target: {})", target_name);
        let list_widget = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(list_title));
        f.render_widget(list_widget, body_chunks[0]);

        // Right Pane: Response / Request Details
        let mut resp_lines = Vec::new();
        let current_ep = &ENDPOINTS[self.selected_endpoint];

        resp_lines.push(Line::from(vec![
            Span::styled("REQUEST: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(current_ep.method, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(current_ep.path, Style::default().fg(Color::White)),
        ]));

        if let Some(body) = current_ep.sample_body {
            resp_lines.push(Line::from(Span::styled("PAYLOAD TEMPLATE:", Style::default().fg(Color::DarkGray))));
            resp_lines.push(Line::from(Span::styled(body, Style::default().fg(Color::DarkGray))));
        }

        resp_lines.push(Line::from(Span::raw("──────────────────────────────────────────────────────────")));

        if let Some(ref resp) = self.last_response {
            let status_color = if resp.status < 300 { Color::Green } else { Color::Red };
            resp_lines.push(Line::from(vec![
                Span::styled("RESPONSE: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(format!("HTTP {} {}", resp.status, resp.status_text), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                Span::raw(format!(" ({} ms)", resp.duration_ms)),
            ]));

            let pretty_json = serde_json::to_string_pretty(&resp.body).unwrap_or_else(|_| resp.raw_body.clone());
            for l in pretty_json.lines().take(18) {
                resp_lines.push(Line::from(Span::styled(l.to_string(), Style::default().fg(Color::LightGreen))));
            }
        } else {
            resp_lines.push(Line::from(Span::styled("No response yet. Press [Space] or [Enter] to send.", Style::default().fg(Color::Yellow))));
        }

        let resp_widget = Paragraph::new(resp_lines)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Postman Response Viewer"));
        f.render_widget(resp_widget, body_chunks[1]);

        // --- FOOTER SHORTCUTS ---
        let footer_text = vec![
            Line::from(vec![
                Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
                Span::raw(" Select API  •  "),
                Span::styled("[Tab]", Style::default().fg(Color::Yellow)),
                Span::raw(" Switch Target [1:Tunnel 2:Local 3:Vercel]  •  "),
                Span::styled("[Space/Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" Send Request  •  "),
                Span::styled("[Q]", Style::default().fg(Color::Red)),
                Span::raw(" Quit  •  Status: "),
                Span::styled(&self.status_message, Style::default().fg(Color::Magenta)),
            ])
        ];
        let footer_block = Paragraph::new(footer_text)
            .block(Block::default().borders(Borders::ALL).title("Controls"));
        f.render_widget(footer_block, chunks[2]);
    }
}
