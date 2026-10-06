use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Working,
    Waiting,
    Done,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub session_id: String,
    pub cwd: String,
    pub last_tool: Option<String>,
    pub status: AgentStatus,
    pub updated_at: String,
}

pub struct AgentSessions(pub Arc<Mutex<HashMap<String, AgentSession>>>);

impl AgentSessions {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }

    pub fn arc(&self) -> Arc<Mutex<HashMap<String, AgentSession>>> {
        Arc::clone(&self.0)
    }
}

#[derive(Debug, Deserialize)]
struct HookPayload {
    session_id: Option<String>,
    cwd: Option<String>,
    hook_event_name: Option<String>,
    tool_name: Option<String>,
}

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as ISO 8601 UTC without chrono: YYYY-MM-DDTHH:MM:SSZ
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hour = (s / 3600) % 24;
    let days = s / 86400; // days since epoch
    let (y, mo, d) = days_to_ymd(days);
    format!("{y:04}-{mo:02}-{d:02}T{hour:02}:{min:02}:{sec:02}Z")
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Gregorian calendar calculation from days since 1970-01-01
    let z = days + 719468;
    let era = z / 146097;
    let doe = z % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y, mo, d)
}

async fn handle_hook(
    State(sessions): State<Arc<Mutex<HashMap<String, AgentSession>>>>,
    Json(payload): Json<HookPayload>,
) -> StatusCode {
    let session_id = match payload.session_id {
        Some(id) if !id.is_empty() => id,
        _ => return StatusCode::BAD_REQUEST,
    };

    let event = payload.hook_event_name.as_deref().unwrap_or("");
    let cwd = payload.cwd.unwrap_or_default();
    let updated_at = now_iso();

    let mut map = sessions.lock().unwrap();
    let entry = map.entry(session_id.clone()).or_insert_with(|| AgentSession {
        session_id: session_id.clone(),
        cwd: cwd.clone(),
        last_tool: None,
        status: AgentStatus::Waiting,
        updated_at: updated_at.clone(),
    });

    // Always update cwd if provided (it may change)
    if !cwd.is_empty() {
        entry.cwd = cwd;
    }
    entry.updated_at = updated_at;

    match event {
        "PostToolUse" => {
            entry.status = AgentStatus::Working;
            entry.last_tool = payload.tool_name;
        }
        "Stop" | "SessionEnd" => {
            entry.status = AgentStatus::Done;
        }
        // SessionStart, UserPromptSubmit, or anything else → waiting for model/user
        _ => {
            entry.status = AgentStatus::Waiting;
        }
    }

    StatusCode::OK
}

pub async fn start_hook_server(sessions: Arc<Mutex<HashMap<String, AgentSession>>>) {
    let app = Router::new()
        .route("/hook", post(handle_hook))
        .with_state(sessions);

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:27384").await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[agents] Failed to bind hook server on :27384 — {e}");
            return;
        }
    };
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("[agents] Hook server error: {e}");
    }
}
