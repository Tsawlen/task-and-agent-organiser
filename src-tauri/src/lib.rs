mod accounts;
mod agents;
mod github;
mod gitlab;
mod onboarding;
mod orgs;
mod work;

use accounts::{get_token, AccountKind};
use tauri::Manager;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use work::WorkList;

const PROJECT_REFRESH_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// Tracks when project items were last successfully fetched per account.
struct ProjectFetchTimes(Mutex<HashMap<String, Instant>>);

#[tauri::command]
async fn fetch_work(
    app_handle: tauri::AppHandle,
    account_id: String,
) -> Result<WorkList, String> {
    let accounts = accounts::load_accounts(&app_handle);
    let account = accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| format!("Account {account_id} not found"))?
        .clone();

    let token = get_token(&account_id).ok_or_else(|| "No token in keychain".to_string())?;

    let skip_projects = should_skip_projects(&app_handle, &account_id);
    let list = match account.kind {
        AccountKind::Github | AccountKind::GithubEnterprise => {
            github::fetch_github(&account, &token, skip_projects).await
        }
        AccountKind::Gitlab => gitlab::fetch_gitlab(&account, &token).await,
    };

    if !skip_projects {
        record_project_fetch(&app_handle, &account_id);
    }

    Ok(list)
}

#[tauri::command]
async fn fetch_all(app_handle: tauri::AppHandle) -> Vec<WorkList> {
    let accounts = accounts::load_accounts(&app_handle);
    let mut results = vec![];
    for account in accounts {
        let token = get_token(&account.id);
        let skip_projects = should_skip_projects(&app_handle, &account.id);
        let list = match token {
            None => WorkList {
                account_id: account.id.clone(),
                items: vec![],
                error: Some("No token stored for this account".into()),
            },
            Some(t) => match account.kind {
                AccountKind::Github | AccountKind::GithubEnterprise => {
                    github::fetch_github(&account, &t, skip_projects).await
                }
                AccountKind::Gitlab => gitlab::fetch_gitlab(&account, &t).await,
            },
        };
        if !skip_projects {
            record_project_fetch(&app_handle, &account.id);
        }
        results.push(list);
    }
    results
}

fn should_skip_projects(app_handle: &tauri::AppHandle, account_id: &str) -> bool {
    let state = app_handle.state::<ProjectFetchTimes>();
    let times = state.0.lock().unwrap();
    match times.get(account_id) {
        Some(last) => last.elapsed() < PROJECT_REFRESH_INTERVAL,
        None => false, // never fetched → fetch now
    }
}

fn record_project_fetch(app_handle: &tauri::AppHandle, account_id: &str) {
    let state = app_handle.state::<ProjectFetchTimes>();
    let mut times = state.0.lock().unwrap();
    times.insert(account_id.to_string(), Instant::now());
}

#[tauri::command]
async fn fetch_ghe_projects_meta(
    app_handle: tauri::AppHandle,
    account_id: String,
) -> Result<Vec<github::GheProjectMeta>, String> {
    let accounts = accounts::load_accounts(&app_handle);
    let account = accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| format!("Account {account_id} not found"))?
        .clone();
    let token = get_token(&account_id).ok_or_else(|| "No token in keychain".to_string())?;
    github::fetch_ghe_projects_meta(&account, &token).await
}

#[tauri::command]
fn get_agent_sessions(state: tauri::State<agents::AgentSessions>) -> Vec<agents::AgentSession> {
    state.0.lock().unwrap().values().cloned().collect()
}

#[tauri::command]
fn dismiss_done_sessions(state: tauri::State<agents::AgentSessions>) {
    state.0.lock().unwrap().retain(|_, s| s.status != agents::AgentStatus::Done);
}

#[tauri::command]
fn get_agent_hook_statuses() -> Vec<onboarding::AgentHookStatus> {    onboarding::all_statuses()
}

#[tauri::command]
fn set_agent_hook(agent_id: String, install: bool) -> Result<(), String> {
    onboarding::set_hook(&agent_id, install)
}

#[tauri::command]
async fn assign_to_agent(
    app_handle: tauri::AppHandle,
    account_id: String,
    repo: String,
    number: u64,
    title: String,
    url: String,
    local_path: String,
) -> Result<(), String> {
    let accounts = accounts::load_accounts(&app_handle);
    let account = accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| format!("Account {account_id} not found"))?
        .clone();
    let token = get_token(&account_id).ok_or_else(|| "No token in keychain".to_string())?;

    let comments = match account.kind {
        AccountKind::Gitlab => {
            gitlab::fetch_mr_review_comments(&account, &token, &url, number).await?
        }
        _ => {
            github::fetch_pr_review_comments(&account, &token, &repo, number).await?
        }
    };

    let prompt = format!(
        "You are working on PR #{number} \"{title}\" ({url}).\n\
         The following review feedback needs to be addressed:\n\n\
         {comments}\n\n\
         Please address all review comments and push the fixes."
    );

    let escaped_prompt = prompt.replace('\'', "'\\''");
    let escaped_path = local_path.replace('\'', "'\\''");
    let script = format!(
        "tell application \"Terminal\" to do script \"cd '{escaped_path}' && claude '{escaped_prompt}'\""
    );

    std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn()
        .map_err(|e| format!("Failed to open Terminal: {e}"))?;

    Ok(())
}

fn install_claude_hooks() {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return,
    };
    let settings_path = std::path::Path::new(&home).join(".claude/settings.json");
    let content = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut root: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return,
    };

    const HOOK_CMD: &str =
        "curl -sf -X POST http://127.0.0.1:27384/hook -H 'Content-Type: application/json' -d @-";
    const EVENTS: &[&str] = &["PostToolUse", "SessionStart", "Stop", "SessionEnd"];

    let root_obj = match root.as_object_mut() {
        Some(o) => o,
        None => return,
    };
    root_obj
        .entry("hooks")
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let hooks_obj = match root_obj.get_mut("hooks").and_then(|v| v.as_object_mut()) {
        Some(o) => o,
        None => return,
    };

    let new_entry = serde_json::json!([{
        "matcher": "",
        "hooks": [{ "type": "command", "command": HOOK_CMD }]
    }]);

    let mut changed = false;
    for event in EVENTS {
        let arr = hooks_obj
            .entry(*event)
            .or_insert_with(|| serde_json::Value::Array(vec![]))
            .as_array_mut();
        if let Some(arr) = arr {
            let already = arr.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(|h| h.as_array())
                    .map_or(false, |hooks| {
                        hooks
                            .iter()
                            .any(|h| h.get("command").and_then(|c| c.as_str()) == Some(HOOK_CMD))
                    })
            });
            if !already {
                arr.push(new_entry.as_array().unwrap()[0].clone());
                changed = true;
            }
        }
    }

    if changed {
        if let Ok(updated) = serde_json::to_string_pretty(&root) {
            let _ = std::fs::write(&settings_path, updated);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let agent_sessions = agents::AgentSessions::new();

    tauri::Builder::default()
        .manage(ProjectFetchTimes(Mutex::new(HashMap::new())))
        .manage(agent_sessions)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let sessions_arc = app.state::<agents::AgentSessions>().arc();
            tauri::async_runtime::spawn(agents::start_hook_server(sessions_arc));
            install_claude_hooks();
            onboarding::auto_install_claude();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            accounts::list_accounts,
            accounts::add_account,
            accounts::remove_account,
            accounts::update_account_orgs,
            accounts::update_account_projects,
            fetch_work,
            fetch_all,
            orgs::fetch_orgs,
            fetch_ghe_projects_meta,
            get_agent_sessions,
            dismiss_done_sessions,
            get_agent_hook_statuses,
            set_agent_hook,
            assign_to_agent,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
