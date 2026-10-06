use crate::accounts::{get_token, load_accounts, AccountKind};
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GhOrg {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GlGroup {
    path: String,
}

fn rest_base_github(base_url: &str) -> String {
    if base_url.is_empty() {
        "https://api.github.com".to_string()
    } else {
        format!("{}/api/v3", base_url.trim_end_matches('/'))
    }
}

#[tauri::command]
pub async fn fetch_orgs(
    app_handle: tauri::AppHandle,
    account_id: String,
) -> Result<Vec<String>, String> {
    let accounts = load_accounts(&app_handle);
    let account = accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| format!("Account {account_id} not found"))?
        .clone();

    let token = get_token(&account_id).ok_or("No token in keychain")?;
    let client = reqwest::Client::new();

    let orgs = match account.kind {
        AccountKind::Github | AccountKind::GithubEnterprise => {
            let base = rest_base_github(&account.base_url);
            let resp = client
                .get(format!("{base}/user/orgs?per_page=100"))
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header(USER_AGENT, "organiser/0.1")
                .send()
                .await
                .map_err(|e| e.to_string())?;
            resp.json::<Vec<GhOrg>>()
                .await
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|o| o.login)
                .collect()
        }
        AccountKind::Gitlab => {
            let base = format!("{}/api/v4", account.base_url.trim_end_matches('/'));
            let resp = client
                .get(format!("{base}/groups?min_access_level=10&per_page=100"))
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header(USER_AGENT, "organiser/0.1")
                .send()
                .await
                .map_err(|e| e.to_string())?;
            resp.json::<Vec<GlGroup>>()
                .await
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|g| g.path)
                .collect()
        }
    };

    Ok(orgs)
}
