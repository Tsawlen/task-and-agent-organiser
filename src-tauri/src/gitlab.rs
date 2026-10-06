use crate::accounts::Account;
use crate::work::{CiStatus, TaskState, WorkItem, WorkKind, WorkList};
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde::Deserialize;

fn api_base(account: &Account) -> String {
    let base = account.base_url.trim_end_matches('/');
    format!("{base}/api/v4")
}

#[derive(Debug, Deserialize)]
struct GlUser {
    username: String,
}

#[derive(Debug, Deserialize)]
struct GlIssue {
    id: u64,
    iid: u64,
    title: String,
    web_url: String,
    updated_at: String,
    project_id: u64,
    labels: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GlMr {
    id: u64,
    iid: u64,
    title: String,
    web_url: String,
    updated_at: String,
    work_in_progress: bool,
    draft: bool,
    state: String,
    project_id: u64,
}

#[derive(Debug, Deserialize)]
struct GlApprovals {
    approved: bool,
}

#[derive(Debug, Deserialize)]
struct GlProject {
    #[serde(rename = "path_with_namespace")]
    path_with_namespace: String,
}

#[derive(Debug, Deserialize)]
struct GlPipeline {
    status: String, // "success" | "failed" | "running" | "pending" | "canceled" | …
}

async fn fetch_mr_ci(
    client: &reqwest::Client,
    token: &str,
    api_base: &str,
    project_id: u64,
    mr_iid: u64,
) -> Option<CiStatus> {
    let url = format!("{api_base}/projects/{project_id}/merge_requests/{mr_iid}/pipelines?per_page=1");
    let pipelines = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .send()
        .await
        .ok()?
        .json::<Vec<GlPipeline>>()
        .await
        .ok()?;

    let status = pipelines.first().map(|p| p.status.as_str())?;
    match status {
        "success" => Some(CiStatus::Success),
        "failed" | "canceled" => Some(CiStatus::Failure),
        "running" => Some(CiStatus::Running),
        "pending" | "waiting_for_resource" | "preparing" | "created" => Some(CiStatus::Success),
        _ => None,
    }
}

fn map_issue_state(labels: &[String]) -> TaskState {
    if labels.iter().any(|l| {
        let l = l.to_lowercase();
        l.contains("in progress") || l.contains("doing") || l == "wip"
    }) {
        TaskState::InProgress
    } else {
        TaskState::Open
    }
}

async fn fetch_mr_state(
    client: &reqwest::Client,
    token: &str,
    api_base: &str,
    project_id: u64,
    mr_iid: u64,
    mr: &GlMr,
) -> TaskState {
    if mr.draft || mr.work_in_progress {
        return TaskState::Draft;
    }
    match mr.state.as_str() {
        "merged" => return TaskState::Merged,
        "closed" => return TaskState::Closed,
        _ => {}
    }

    let url = format!("{api_base}/projects/{project_id}/merge_requests/{mr_iid}/approvals");
    let resp = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .send()
        .await;

    match resp {
        Ok(r) => match r.json::<GlApprovals>().await {
            Ok(a) if a.approved => TaskState::Approved,
            _ => TaskState::Open,
        },
        Err(_) => TaskState::Open,
    }
}

pub async fn fetch_gitlab(account: &Account, token: &str) -> WorkList {
    let base = api_base(account);
    let client = reqwest::Client::new();

    // Get current user
    let me: GlUser = match client
        .get(format!("{base}/user"))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .send()
        .await
        .and_then(|r| Ok(r))
    {
        Ok(r) => match r.json().await {
            Ok(u) => u,
            Err(e) => {
                return WorkList {
                    account_id: account.id.clone(),
                    items: vec![],
                    error: Some(format!("Auth error: {e}")),
                }
            }
        },
        Err(e) => {
            return WorkList {
                account_id: account.id.clone(),
                items: vec![],
                error: Some(format!("Network error: {e}")),
            }
        }
    };

    let mut items: Vec<WorkItem> = vec![];

    // Assigned issues
    let issues_url = format!(
        "{base}/issues?assignee_username={}&state=opened&per_page=50",
        me.username
    );
    if let Ok(resp) = client
        .get(&issues_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .send()
        .await
    {
        if let Ok(issues) = resp.json::<Vec<GlIssue>>().await {
            for issue in issues {
                // Fetch project name
                let repo = fetch_project_name(&client, token, &base, issue.project_id).await;
                items.push(WorkItem {
                    id: format!("gl-issue-{}", issue.id),
                    kind: WorkKind::Issue,
                    title: issue.title.clone(),
                    url: issue.web_url.clone(),
                    updated_at: issue.updated_at.clone(),
                    state: map_issue_state(&issue.labels),
                    account: account.id.clone(),
                    repo,
                    number: Some(issue.iid),
                    project: None,
                    author: None,
                    ci_status: None,
                });
            }
        }
    }

    // My MRs
    let mrs_url = format!(
        "{base}/merge_requests?author_username={}&state=opened&per_page=50",
        me.username
    );
    if let Ok(resp) = client
        .get(&mrs_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .send()
        .await
    {
        if let Ok(mrs) = resp.json::<Vec<GlMr>>().await {
            for mr in mrs {
                let state =
                    fetch_mr_state(&client, token, &base, mr.project_id, mr.iid, &mr).await;
                let repo = fetch_project_name(&client, token, &base, mr.project_id).await;
                let ci_status = fetch_mr_ci(&client, token, &base, mr.project_id, mr.iid).await;
                items.push(WorkItem {
                    id: format!("gl-mr-{}", mr.id),
                    kind: WorkKind::Pr,
                    title: mr.title.clone(),
                    url: mr.web_url.clone(),
                    updated_at: mr.updated_at.clone(),
                    state,
                    account: account.id.clone(),
                    repo,
                    number: Some(mr.iid),
                    project: None,
                    author: None,
                    ci_status,
                });
            }
        }
    }

    // Review-requested MRs
    let review_url = format!(
        "{base}/merge_requests?reviewer_username={}&state=opened&per_page=50",
        me.username
    );
    if let Ok(resp) = client
        .get(&review_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .send()
        .await
    {
        if let Ok(mrs) = resp.json::<Vec<GlMrWithAuthor>>().await {
            for mr in mrs {
                if is_bot_username(&mr.author.username) { continue; }
                let repo = fetch_project_name(&client, token, &base, mr.mr.project_id).await;
                let ci_status = fetch_mr_ci(&client, token, &base, mr.mr.project_id, mr.mr.iid).await;
                items.push(WorkItem {
                    id: format!("gl-review-{}", mr.mr.id),
                    kind: WorkKind::ReviewRequest,
                    title: mr.mr.title.clone(),
                    url: mr.mr.web_url.clone(),
                    updated_at: mr.mr.updated_at.clone(),
                    state: map_gl_mr_state(&mr.mr),
                    account: account.id.clone(),
                    repo,
                    number: Some(mr.mr.iid),
                    project: None,
                    author: Some(mr.author.username.clone()),
                    ci_status,
                });
            }
        }
    }

    WorkList { account_id: account.id.clone(), items, error: None }
}

#[derive(Debug, Deserialize)]
struct GlMrWithAuthor {
    #[serde(flatten)]
    mr: GlMr,
    author: GlUser,
}

fn map_gl_mr_state(mr: &GlMr) -> TaskState {
    if mr.draft || mr.work_in_progress { return TaskState::Draft; }
    match mr.state.as_str() {
        "merged" => TaskState::Merged,
        "closed" => TaskState::Closed,
        _ => TaskState::Open,
    }
}

fn is_bot_username(login: &str) -> bool {
    let l = login.to_lowercase();
    l.ends_with("[bot]") || l.ends_with("_bot") || l == "bot"
}

async fn fetch_project_name(
    client: &reqwest::Client,
    token: &str,
    base: &str,
    project_id: u64,
) -> Option<String> {
    let url = format!("{base}/projects/{project_id}");
    client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .send()
        .await
        .ok()?
        .json::<GlProject>()
        .await
        .ok()
        .map(|p| p.path_with_namespace)
}

#[derive(Debug, serde::Deserialize)]
struct GlNote {
    body: String,
    #[serde(default)]
    system: bool,
}

pub async fn fetch_mr_review_comments(
    account: &Account,
    token: &str,
    mr_url: &str,
    number: u64,
) -> Result<String, String> {
    let base = api_base(account);
    let web_base = account.base_url.trim_end_matches('/');

    // Extract project path from the MR URL:
    // e.g. https://gitlab.example.com/group/project/-/merge_requests/42
    //   → "group/project"
    let after_base = mr_url
        .strip_prefix(web_base)
        .unwrap_or(mr_url)
        .trim_start_matches('/');
    let project_path = after_base
        .split("/-/")
        .next()
        .unwrap_or("")
        .trim_end_matches('/');
    let encoded = project_path.replace('/', "%2F");

    let client = reqwest::Client::new();
    let notes: Vec<GlNote> = client
        .get(format!("{base}/projects/{encoded}/merge_requests/{number}/notes?sort=asc&per_page=100"))
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .header(reqwest::header::USER_AGENT, "organiser/0.1")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let comments: Vec<&str> = notes
        .iter()
        .filter(|n| !n.system && !n.body.trim().is_empty())
        .map(|n| n.body.as_str())
        .collect();

    if comments.is_empty() {
        Ok("No review comments found.".into())
    } else {
        Ok(comments
            .iter()
            .map(|c| format!("- {c}"))
            .collect::<Vec<_>>()
            .join("\n"))
    }
}
