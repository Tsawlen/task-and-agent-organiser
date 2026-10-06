use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub kind: AccountKind,
    pub base_url: String,
    pub label: String,
    #[serde(default)]
    pub blocked_orgs: Vec<String>,
    #[serde(default)]
    pub selected_projects: Vec<String>, // GHE project node IDs to fetch
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Github,
    GithubEnterprise,
    Gitlab,
}

fn config_path(app_handle: &tauri::AppHandle) -> PathBuf {
    app_handle
        .path()
        .app_config_dir()
        .expect("no app config dir")
        .join("accounts.json")
}

pub fn load_accounts(app_handle: &tauri::AppHandle) -> Vec<Account> {
    let path = config_path(app_handle);
    if !path.exists() {
        return vec![];
    }
    let data = fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&data).unwrap_or_default()
}

fn save_accounts(app_handle: &tauri::AppHandle, accounts: &[Account]) {
    let path = config_path(app_handle);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let data = serde_json::to_string_pretty(accounts).expect("serialize accounts");
    fs::write(&path, data).expect("write accounts");
}

fn keychain_entry(account_id: &str) -> Entry {
    Entry::new("organiser", account_id).expect("keychain entry")
}

#[tauri::command]
pub fn list_accounts(app_handle: tauri::AppHandle) -> Vec<Account> {
    load_accounts(&app_handle)
}

#[tauri::command]
pub fn add_account(
    app_handle: tauri::AppHandle,
    kind: AccountKind,
    base_url: String,
    label: String,
    token: String,
) -> Result<Account, String> {
    let id = Uuid::new_v4().to_string();
    let account = Account {
        id: id.clone(), kind, base_url, label,
        blocked_orgs: vec![],
        selected_projects: vec![],
    };

    keychain_entry(&id)
        .set_password(&token)
        .map_err(|e| format!("keychain error: {e}"))?;

    let mut accounts = load_accounts(&app_handle);
    accounts.push(account.clone());
    save_accounts(&app_handle, &accounts);

    Ok(account)
}

#[tauri::command]
pub fn remove_account(app_handle: tauri::AppHandle, id: String) -> Result<(), String> {
    keychain_entry(&id).delete_credential().ok();

    let mut accounts = load_accounts(&app_handle);
    accounts.retain(|a| a.id != id);
    save_accounts(&app_handle, &accounts);

    Ok(())
}

#[tauri::command]
pub fn update_account_orgs(
    app_handle: tauri::AppHandle,
    id: String,
    blocked_orgs: Vec<String>,
) -> Result<(), String> {
    let mut accounts = load_accounts(&app_handle);
    if let Some(a) = accounts.iter_mut().find(|a| a.id == id) {
        a.blocked_orgs = blocked_orgs;
    }
    save_accounts(&app_handle, &accounts);
    Ok(())
}

#[tauri::command]
pub fn update_account_projects(
    app_handle: tauri::AppHandle,
    id: String,
    selected_projects: Vec<String>,
) -> Result<(), String> {
    let mut accounts = load_accounts(&app_handle);
    if let Some(a) = accounts.iter_mut().find(|a| a.id == id) {
        a.selected_projects = selected_projects;
    }
    save_accounts(&app_handle, &accounts);
    Ok(())
}

pub fn get_token(account_id: &str) -> Option<String> {
    keychain_entry(account_id).get_password().ok()
}
