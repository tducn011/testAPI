use std::sync::Arc;
use std::time::Instant;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub name: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub sample_body: Option<&'static str>,
    pub requires_auth: bool,
    pub requires_csrf: bool,
    pub requires_idempotency: bool,
}

pub const ENDPOINTS: &[ApiEndpoint] = &[
    ApiEndpoint {
        name: "1. Healthcheck",
        method: "GET",
        path: "/health",
        sample_body: None,
        requires_auth: false,
        requires_csrf: false,
        requires_idempotency: false,
    },
    ApiEndpoint {
        name: "2. Auth: Register",
        method: "POST",
        path: "/api/v1/auth/register",
        sample_body: Some(r#"{"email":"team_user@expiry.test","password":"Password123!","display_name":"Team Member","timezone":"Asia/Ho_Chi_Minh"}"#),
        requires_auth: false,
        requires_csrf: false,
        requires_idempotency: false,
    },
    ApiEndpoint {
        name: "3. Auth: Login",
        method: "POST",
        path: "/api/v1/auth/login",
        sample_body: Some(r#"{"email":"team_user@expiry.test","password":"Password123!"}"#),
        requires_auth: false,
        requires_csrf: false,
        requires_idempotency: false,
    },
    ApiEndpoint {
        name: "4. Auth: Get Session",
        method: "GET",
        path: "/api/v1/auth/session",
        sample_body: None,
        requires_auth: true,
        requires_csrf: false,
        requires_idempotency: false,
    },
    ApiEndpoint {
        name: "5. Inventory: List Foods",
        method: "GET",
        path: "/api/v1/food-entries?limit=10&page=1",
        sample_body: None,
        requires_auth: true,
        requires_csrf: false,
        requires_idempotency: false,
    },
    ApiEndpoint {
        name: "6. Inventory: Create Food Item",
        method: "POST",
        path: "/api/v1/food-entries",
        sample_body: Some(r#"{"name":"Thịt Bò Úc Premium","quantity":"1500.000","unit":"g","storage_location":"Ngăn đông","date_certainty":"known","date_source":"printed_label","date_label_type":"use_by","expiry_date":"2026-12-31"}"#),
        requires_auth: true,
        requires_csrf: true,
        requires_idempotency: true,
    },
    ApiEndpoint {
        name: "7. User: Preferences",
        method: "GET",
        path: "/api/v1/me/preferences",
        sample_body: None,
        requires_auth: true,
        requires_csrf: false,
        requires_idempotency: false,
    },
    ApiEndpoint {
        name: "8. Trash: List Deleted Items",
        method: "GET",
        path: "/api/v1/trash/food-entries",
        sample_body: None,
        requires_auth: true,
        requires_csrf: false,
        requires_idempotency: false,
    },
    ApiEndpoint {
        name: "9. OCR Scan Health (Python)",
        method: "GET",
        path: "/health",
        sample_body: None,
        requires_auth: false,
        requires_csrf: false,
        requires_idempotency: false,
    },
    ApiEndpoint {
        name: "10. Auth: Logout",
        method: "POST",
        path: "/api/v1/auth/logout",
        sample_body: None,
        requires_auth: true,
        requires_csrf: true,
        requires_idempotency: false,
    },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    pub status_text: String,
    pub duration_ms: u128,
    pub headers: Vec<(String, String)>,
    pub body: Value,
    pub raw_body: String,
}

#[derive(Clone)]
pub struct PostmanEngine {
    client: reqwest::Client,
    pub cookie_session: Arc<Mutex<Option<String>>>,
    pub csrf_token: Arc<Mutex<Option<String>>>,
    pub active_user: Arc<Mutex<Option<String>>>,
}

impl PostmanEngine {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .unwrap_or_default();

        Self {
            client,
            cookie_session: Arc::new(Mutex::new(None)),
            csrf_token: Arc::new(Mutex::new(None)),
            active_user: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn send_request(
        &self,
        base_url: &str,
        endpoint: &ApiEndpoint,
        custom_body: Option<&str>,
        custom_headers: Option<Vec<(String, String)>>,
    ) -> Result<ApiResponse, String> {
        let full_url = format!("{}{}", base_url.trim_end_matches('/'), endpoint.path);
        let start = Instant::now();

        let mut req = match endpoint.method {
            "GET" => self.client.get(&full_url),
            "POST" => self.client.post(&full_url),
            "PUT" => self.client.put(&full_url),
            "DELETE" => self.client.delete(&full_url),
            _ => self.client.get(&full_url),
        };

        // Attach Origin header to satisfy CORS & CSFR protection
        req = req.header("Origin", "https://demoexpiry.vercel.app");

        // Idempotency Key
        if endpoint.requires_idempotency {
            let key = format!("rust_agent_{}", Uuid::new_v4());
            req = req.header("Idempotency-Key", key);
        }

        // Attach CSRF Token if available
        let csrf_guard = self.csrf_token.lock().await;
        if let Some(ref csrf) = *csrf_guard {
            req = req.header("X-CSRF-Token", csrf.clone());
        }
        drop(csrf_guard);

        // Attach Session Cookie if stored
        let cookie_guard = self.cookie_session.lock().await;
        if let Some(ref cookie) = *cookie_guard {
            req = req.header("Cookie", cookie.clone());
        }
        drop(cookie_guard);

        // Custom headers
        if let Some(headers) = custom_headers {
            for (k, v) in headers {
                if let (Ok(hname), Ok(hval)) = (HeaderName::from_bytes(k.as_bytes()), HeaderValue::from_str(&v)) {
                    req = req.header(hname, hval);
                }
            }
        }

        // Attach Body
        let body_str = custom_body.or(endpoint.sample_body);
        if let Some(body) = body_str {
            req = req.header("Content-Type", "application/json").body(body.to_string());
        }

        let resp = req.send().await.map_err(|e| format!("Network request failed: {e}"))?;
        let duration_ms = start.elapsed().as_millis();
        let status = resp.status().as_u16();
        let status_text = resp.status().canonical_reason().unwrap_or("Unknown").to_string();

        let mut header_list = Vec::new();
        for (k, v) in resp.headers() {
            let v_str = v.to_str().unwrap_or("").to_string();
            header_list.push((k.to_string(), v_str.clone()));

            // Capture session cookie from Set-Cookie
            if k.as_str().eq_ignore_ascii_case("set-cookie") {
                if v_str.contains("expiry_session=") || v_str.contains("__Host-expiry_session=") {
                    let mut lock = self.cookie_session.lock().await;
                    *lock = Some(v_str.clone());
                }
            }
        }

        let raw_text = resp.text().await.unwrap_or_default();
        let json_val: Value = serde_json::from_str(&raw_text).unwrap_or(Value::Null);

        // Capture CSRF token and User if returned in JSON
        if let Some(token) = json_val.get("csrf_token").and_then(|v| v.as_str()) {
            let mut lock = self.csrf_token.lock().await;
            *lock = Some(token.to_string());
        }
        if let Some(user_email) = json_val.pointer("/user/email").and_then(|v| v.as_str()) {
            let mut lock = self.active_user.lock().await;
            *lock = Some(user_email.to_string());
        }

        Ok(ApiResponse {
            status,
            status_text,
            duration_ms,
            headers: header_list,
            body: json_val,
            raw_body: raw_text,
        })
    }
}
