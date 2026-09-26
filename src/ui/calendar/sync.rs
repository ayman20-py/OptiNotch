use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use super::model::CalendarEvent;

static HTTP_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

fn get_http_client() -> &'static reqwest::blocking::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default()
    })
}

// Default Desktop OAuth credentials (injected from .env during compilation or loaded at runtime)
const DEFAULT_CLIENT_ID: &str = match option_env!("CALENDAR_CLIENT_ID") {
    Some(val) => val,
    None => "",
};
const DEFAULT_CLIENT_SECRET: &str = match option_env!("CALENDAR_CLIENT_SECRET") {
    Some(val) => val,
    None => "",
};

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_CALENDAR_API: &str = "https://www.googleapis.com/calendar/v3";
const SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";
const REDIRECT_PORT: u16 = 8088;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64, // Unix timestamp in seconds
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoogleCalendarConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Default)]
pub struct GoogleSyncState {
    pub is_authenticated: bool,
    pub is_syncing: bool,
    pub last_sync_time: Option<Instant>,
    pub sync_token: Option<String>,
    pub account_email: Option<String>,
    pub events: HashMap<(u32, u32, u32), Vec<CalendarEvent>>, // (Y, M, D) -> Events
}

pub struct GoogleCalendarService {
    config_path: PathBuf,
    tokens_path: PathBuf,
    pub state: Arc<Mutex<GoogleSyncState>>,
    sync_requested: Arc<AtomicBool>,
    auth_in_progress: Arc<AtomicBool>,
}

impl GoogleCalendarService {
    pub fn new() -> Self {
        let app_dir = get_app_data_dir();
        let _ = fs::create_dir_all(&app_dir);

        let config_path = app_dir.join("calendar_config.json");
        let tokens_path = app_dir.join("tokens.json");

        let state = Arc::new(Mutex::new(GoogleSyncState::default()));
        let sync_requested = Arc::new(AtomicBool::new(false));
        let auth_in_progress = Arc::new(AtomicBool::new(false));

        // Try to load existing tokens on startup
        if let Ok(content) = fs::read_to_string(&tokens_path) {
            if let Ok(tokens) = serde_json::from_str::<AuthTokens>(&content) {
                let mut s = state.lock().unwrap();
                s.is_authenticated = !tokens.access_token.is_empty();
            }
        }

        let service = Self {
            config_path: config_path.clone(),
            tokens_path: tokens_path.clone(),
            state: Arc::clone(&state),
            sync_requested: Arc::clone(&sync_requested),
            auth_in_progress: Arc::clone(&auth_in_progress),
        };

        // Start background worker thread for Delta Sync + Polling
        let state_clone = Arc::clone(&state);
        let tokens_path_clone = service.tokens_path.clone();
        let config_path_clone = service.config_path.clone();
        let sync_req_clone = Arc::clone(&sync_requested);

        thread::spawn(move || {
            background_sync_loop(
                state_clone,
                tokens_path_clone,
                config_path_clone,
                sync_req_clone,
            );
        });

        service
    }

    /// Trigger an immediate delta sync (e.g. when notch expands or week changes)
    pub fn request_sync(&self) {
        self.sync_requested.store(true, Ordering::SeqCst);
        let state = Arc::clone(&self.state);
        let tokens_path = self.tokens_path.clone();
        let config_path = self.config_path.clone();
        thread::spawn(move || {
            perform_sync(&state, &tokens_path, &config_path);
        });
    }

    /// Start direct Google OAuth 2.0 PKCE / loopback browser authentication
    pub fn start_oauth_flow(&self) {
        if self.auth_in_progress.swap(true, Ordering::SeqCst) {
            return; // Auth already in progress
        }

        let tokens_path = self.tokens_path.clone();
        let config_path = self.config_path.clone();
        let state_clone = Arc::clone(&self.state);
        let auth_flag = Arc::clone(&self.auth_in_progress);
        let sync_req = Arc::clone(&self.sync_requested);

        thread::spawn(move || {
            let config = load_config(&config_path);
            let client_id = if !config.client_id.is_empty() {
                config.client_id.clone()
            } else {
                DEFAULT_CLIENT_ID.to_string()
            };

            let client_secret = if !config.client_secret.is_empty() {
                config.client_secret.clone()
            } else {
                DEFAULT_CLIENT_SECRET.to_string()
            };

            let redirect_uri = format!("http://127.0.0.1:{}/callback", REDIRECT_PORT);
            let auth_url = format!(
                "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
                GOOGLE_AUTH_URL,
                urlencoding::encode(&client_id),
                urlencoding::encode(&redirect_uri),
                urlencoding::encode(SCOPE)
            );

            // 1. Open default browser directly to Google Sign-In
            let _ = open::that(&auth_url);

            // 2. Start local loopback listener to capture auth code
            if let Ok(server) = tiny_http::Server::http(format!("127.0.0.1:{}", REDIRECT_PORT)) {
                for request in server.incoming_requests() {
                    let url = request.url().to_string();
                    if url.starts_with("/callback") {
                        if let Some(code) = extract_query_param(&url, "code") {
                            let html = "<!DOCTYPE html><html><head><meta charset='utf-8'><title>OptiNotch Connected</title><style>body{background:#09090b;color:#f4f4f5;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;}.card{background:#18181b;padding:36px 48px;border-radius:18px;border:1px solid #27272a;text-align:center;box-shadow:0 20px 40px rgba(0,0,0,0.5);}h1{color:#38bdf8;margin:0 0 12px;font-size:24px;}p{color:#a1a1aa;margin:0;font-size:14px;}</style></head><body><div class='card'><h1>&#10003; OptiNotch Connected!</h1><p>Your Google Calendar has been linked. You can close this tab and return to OptiNotch.</p></div></body></html>";
                            let response = tiny_http::Response::from_string(html).with_header(
                                tiny_http::Header::from_bytes(
                                    &b"Content-Type"[..],
                                    &b"text/html; charset=utf-8"[..],
                                )
                                .unwrap(),
                            );
                            let _ = request.respond(response);

                            // Exchange code for tokens
                            if let Ok(tokens) = exchange_code_for_tokens(
                                &code,
                                &client_id,
                                &client_secret,
                                &redirect_uri,
                            ) {
                                let _ = fs::write(
                                    &tokens_path,
                                    serde_json::to_string_pretty(&tokens).unwrap_or_default(),
                                );
                                {
                                    let mut s = state_clone.lock().unwrap();
                                    s.is_authenticated = true;
                                }
                                sync_req.store(true, Ordering::SeqCst);
                                perform_sync(&state_clone, &tokens_path, &config_path);
                            }
                            break;
                        }
                    }
                }
            }

            auth_flag.store(false, Ordering::SeqCst);
        });
    }

    /// Retrieve the current cached events mapped by (Year, Month, Day)
    pub fn get_events(&self) -> HashMap<(u32, u32, u32), Vec<CalendarEvent>> {
        let s = self.state.lock().unwrap();
        s.events.clone()
    }
}

fn background_sync_loop(
    state: Arc<Mutex<GoogleSyncState>>,
    tokens_path: PathBuf,
    config_path: PathBuf,
    sync_requested: Arc<AtomicBool>,
) {
    let mut last_poll = Instant::now() - Duration::from_secs(300);

    loop {
        let is_req = sync_requested.swap(false, Ordering::SeqCst);
        let elapsed = last_poll.elapsed();

        // Perform sync if requested (on expand/navigation) or every 60 seconds
        if is_req || elapsed >= Duration::from_secs(60) {
            last_poll = Instant::now();
            perform_sync(&state, &tokens_path, &config_path);
        }

        thread::sleep(Duration::from_millis(250));
    }
}

fn perform_sync(state: &Arc<Mutex<GoogleSyncState>>, tokens_path: &PathBuf, config_path: &PathBuf) {
    let config = load_config(config_path);
    let mut tokens = load_tokens(tokens_path);

    if let Some(ref mut t) = tokens {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let client_id = if !config.client_id.is_empty() {
            &config.client_id
        } else {
            DEFAULT_CLIENT_ID
        };
        let client_secret = if !config.client_secret.is_empty() {
            &config.client_secret
        } else {
            DEFAULT_CLIENT_SECRET
        };

        // If access token is expired (or expires in < 120s), refresh it
        if t.expires_at <= now_secs + 120 {
            if let Some(ref refresh_tok) = t.refresh_token {
                if let Ok(new_tokens) = refresh_access_token(refresh_tok, client_id, client_secret)
                {
                    *t = new_tokens.clone();
                    let _ = fs::write(
                        tokens_path,
                        serde_json::to_string_pretty(&new_tokens).unwrap_or_default(),
                    );
                }
            }
        }

        let is_ok = sync_google_rest_api(state, &t.access_token);
        if !is_ok {
            // Token might be invalid/expired, try force refresh and retry once
            if let Some(ref refresh_tok) = t.refresh_token {
                if let Ok(new_tokens) = refresh_access_token(refresh_tok, client_id, client_secret)
                {
                    *t = new_tokens.clone();
                    let _ = fs::write(
                        tokens_path,
                        serde_json::to_string_pretty(&new_tokens).unwrap_or_default(),
                    );
                    sync_google_rest_api(state, &t.access_token);
                }
            }
        }
    }
}

fn sync_google_rest_api(state: &Arc<Mutex<GoogleSyncState>>, access_token: &str) -> bool {
    {
        let mut s = state.lock().unwrap();
        s.is_syncing = true;
    }

    let client = get_http_client();

    // Query primary calendar events for current active multi-year window
    let now_utc = chrono_lite_now();
    let start_year = now_utc.0.saturating_sub(1);
    let end_year = now_utc.0 + 2;
    let time_min = format!("{:04}-01-01T00:00:00Z", start_year);
    let time_max = format!("{:04}-12-31T23:59:59Z", end_year);

    let default_calendar_color =
        fetch_calendar_default_color(&client, access_token).unwrap_or((59, 130, 246)); // Default Google Blue

    let url = format!("{}/calendars/primary/events", GOOGLE_CALENDAR_API);

    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .query(&[
            ("singleEvents", "true"),
            ("orderBy", "startTime"),
            ("timeMin", &time_min),
            ("timeMax", &time_max),
            ("maxResults", "2500"),
        ])
        .send();

    let mut success = false;

    if let Ok(response) = res {
        if response.status().is_success() {
            if let Ok(json) = response.json::<serde_json::Value>() {
                if let Some(items) = json.get("items").and_then(|i| i.as_array()) {
                    let mut new_events: HashMap<(u32, u32, u32), Vec<CalendarEvent>> =
                        HashMap::new();

                    for item in items {
                        let title = item
                            .get("summary")
                            .and_then(|s| s.as_str())
                            .unwrap_or("(No title)")
                            .to_string();

                        let color_rgb = item
                            .get("colorId")
                            .and_then(|c| c.as_str())
                            .map(parse_event_color_id)
                            .unwrap_or(default_calendar_color);

                        let start = item.get("start");
                        let end = item.get("end");

                        if let Some((y, m, d, time_str, is_all_day)) = parse_event_time(start, end)
                        {
                            let event = CalendarEvent {
                                title,
                                time_str,
                                color_rgb,
                                is_all_day,
                            };
                            new_events.entry((y, m, d)).or_default().push(event);
                        }
                    }

                    // Sort events on each day chronologically
                    for day_events in new_events.values_mut() {
                        day_events.sort_by(|a, b| {
                            if a.is_all_day && !b.is_all_day {
                                std::cmp::Ordering::Less
                            } else if !a.is_all_day && b.is_all_day {
                                std::cmp::Ordering::Greater
                            } else {
                                a.time_str.cmp(&b.time_str)
                            }
                        });
                    }

                    let mut s = state.lock().unwrap();
                    s.events = new_events;
                    s.is_authenticated = true;
                    s.last_sync_time = Some(Instant::now());
                    success = true;
                }
            }
        }
    }

    {
        let mut s = state.lock().unwrap();
        s.is_syncing = false;
    }

    success
}

fn fetch_calendar_default_color(
    client: &reqwest::blocking::Client,
    access_token: &str,
) -> Option<(u8, u8, u8)> {
    let url = format!("{}/users/me/calendarList/primary", GOOGLE_CALENDAR_API);
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .ok()?;

    if res.status().is_success() {
        let json = res.json::<serde_json::Value>().ok()?;
        if let Some(bg_hex) = json.get("backgroundColor").and_then(|c| c.as_str()) {
            return parse_hex_color(bg_hex);
        }
    }
    None
}

fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let clean = hex.trim_start_matches('#');
    if clean.len() == 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
        let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
        let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
        Some((r, g, b))
    } else {
        None
    }
}

fn parse_event_color_id(color_id: &str) -> (u8, u8, u8) {
    match color_id {
        "1" => (121, 134, 203), // Lavender
        "2" => (51, 182, 121),  // Sage
        "3" => (142, 36, 170),  // Grape
        "4" => (230, 124, 115), // Flamingo
        "5" => (246, 191, 38),  // Banana
        "6" => (244, 81, 30),   // Tangerine
        "7" => (3, 155, 229),   // Peacock
        "8" => (63, 81, 181),   // Graphite
        "9" => (57, 73, 171),   // Blueberry
        "10" => (11, 128, 67),  // Basil
        "11" => (213, 0, 0),    // Tomato
        _ => (59, 130, 246),    // Default Google Blue
    }
}

fn parse_event_time(
    start: Option<&serde_json::Value>,
    end: Option<&serde_json::Value>,
) -> Option<(u32, u32, u32, String, bool)> {
    let start_obj = start?;
    let end_obj = end;

    // 1. Check all-day date: "2026-09-24"
    if let Some(date_str) = start_obj.get("date").and_then(|d| d.as_str()) {
        let parts: Vec<&str> = date_str.split('-').collect();
        if parts.len() == 3 {
            let y: u32 = parts[0].parse().ok()?;
            let m: u32 = parts[1].parse().ok()?;
            let d: u32 = parts[2].parse().ok()?;
            return Some((y, m, d, "All-day".to_string(), true));
        }
    }

    // 2. Check timed dateTime: "2026-09-24T14:30:00+08:00"
    if let Some(dt_str) = start_obj.get("dateTime").and_then(|d| d.as_str()) {
        if dt_str.len() >= 16 {
            let date_part = &dt_str[0..10];
            let parts: Vec<&str> = date_part.split('-').collect();
            if parts.len() == 3 {
                let y: u32 = parts[0].parse().ok()?;
                let m: u32 = parts[1].parse().ok()?;
                let d: u32 = parts[2].parse().ok()?;

                let time_part = &dt_str[11..16]; // "14:30"
                let sh: u32 = time_part[0..2].parse().unwrap_or(0);
                let sm: u32 = time_part[3..5].parse().unwrap_or(0);
                let ampm = if sh >= 12 { "PM" } else { "AM" };
                let h12 = if sh == 0 {
                    12
                } else if sh > 12 {
                    sh - 12
                } else {
                    sh
                };
                let start_formatted = format!("{:02}:{:02} {}", h12, sm, ampm);

                let mut time_str = start_formatted;

                if let Some(end_dt_str) = end_obj
                    .and_then(|e| e.get("dateTime"))
                    .and_then(|d| d.as_str())
                {
                    if end_dt_str.len() >= 16 {
                        let end_time_part = &end_dt_str[11..16];
                        let eh: u32 = end_time_part[0..2].parse().unwrap_or(0);
                        let em: u32 = end_time_part[3..5].parse().unwrap_or(0);
                        let e_ampm = if eh >= 12 { "PM" } else { "AM" };
                        let eh12 = if eh == 0 {
                            12
                        } else if eh > 12 {
                            eh - 12
                        } else {
                            eh
                        };
                        time_str = format!("{} - {:02}:{:02} {}", time_str, eh12, em, e_ampm);
                    }
                }

                return Some((y, m, d, time_str, false));
            }
        }
    }

    None
}

fn exchange_code_for_tokens(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> Result<AuthTokens, Box<dyn std::error::Error>> {
    let client = get_http_client();
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("code", code),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
    ];

    let res = client.post(GOOGLE_TOKEN_URL).form(&params).send()?;
    let json: serde_json::Value = res.json()?;

    let access_token = json
        .get("access_token")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let refresh_token = json
        .get("refresh_token")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());
    let expires_in = json
        .get("expires_in")
        .and_then(|e| e.as_u64())
        .unwrap_or(3600);

    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + expires_in;

    Ok(AuthTokens {
        access_token,
        refresh_token,
        expires_at,
    })
}

fn refresh_access_token(
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<AuthTokens, Box<dyn std::error::Error>> {
    let client = get_http_client();
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    let res = client.post(GOOGLE_TOKEN_URL).form(&params).send()?;
    let json: serde_json::Value = res.json()?;

    let access_token = json
        .get("access_token")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let expires_in = json
        .get("expires_in")
        .and_then(|e| e.as_u64())
        .unwrap_or(3600);

    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + expires_in;

    Ok(AuthTokens {
        access_token,
        refresh_token: Some(refresh_token.to_string()),
        expires_at,
    })
}

fn extract_query_param(url: &str, param: &str) -> Option<String> {
    let query = url.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut kv = pair.split('=');
        if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
            if k == param {
                return Some(urlencoding::decode(v).unwrap_or_default().to_string());
            }
        }
    }
    None
}

fn load_config(path: &PathBuf) -> GoogleCalendarConfig {
    let mut client_id = std::env::var("CALENDAR_CLIENT_ID").unwrap_or_default();
    let mut client_secret = std::env::var("CALENDAR_CLIENT_SECRET").unwrap_or_default();

    // Check .env files in working dir, appdata dir, or current dir
    for env_path in [
        PathBuf::from(".env"),
        get_app_data_dir().join(".env"),
        PathBuf::from("../.env"),
    ] {
        if let Ok(content) = fs::read_to_string(&env_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = trimmed.split_once('=') {
                    let key = k.trim();
                    let val = v.trim().trim_matches('"').trim_matches('\'').to_string();
                    if key == "CALENDAR_CLIENT_ID" && client_id.is_empty() {
                        client_id = val;
                    } else if key == "CALENDAR_CLIENT_SECRET" && client_secret.is_empty() {
                        client_secret = val;
                    }
                }
            }
        }
    }

    let mut config = if let Ok(content) = fs::read_to_string(path) {
        serde_json::from_str::<GoogleCalendarConfig>(&content).unwrap_or_default()
    } else {
        GoogleCalendarConfig::default()
    };

    if !client_id.is_empty() {
        config.client_id = client_id;
    }
    if !client_secret.is_empty() {
        config.client_secret = client_secret;
    }

    config
}

fn load_tokens(path: &PathBuf) -> Option<AuthTokens> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn get_app_data_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("OptiNotch")
    } else {
        PathBuf::from("./data")
    }
}

fn chrono_lite_now() -> (u32, u32, u32) {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut st) };
    (st.wYear as u32, st.wMonth as u32, st.wDay as u32)
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for b in s.bytes() {
            match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(b as char);
                }
                _ => {
                    encoded.push_str(&format!("%{:02X}", b));
                }
            }
        }
        encoded
    }

    pub fn decode(s: &str) -> Result<String, std::string::FromUtf8Error> {
        let mut bytes = Vec::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(b) = u8::from_str_radix(&hex, 16) {
                    bytes.push(b);
                }
            } else if c == '+' {
                bytes.push(b' ');
            } else {
                bytes.push(c as u8);
            }
        }
        String::from_utf8(bytes)
    }
}
