use crate::accounts::{Account, AccountKind};
use crate::work::{CiStatus, TaskState, WorkItem, WorkKind, WorkList};
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde::Deserialize;
use std::collections::HashSet;

const GH_COM_GRAPHQL: &str = "https://api.github.com/graphql";
const GH_COM_REST: &str = "https://api.github.com";

fn graphql_url(account: &Account) -> String {
    if account.base_url.is_empty() {
        GH_COM_GRAPHQL.to_string()
    } else {
        format!("{}/api/graphql", account.base_url.trim_end_matches('/'))
    }
}

fn rest_base(account: &Account) -> String {
    if account.base_url.is_empty() {
        GH_COM_REST.to_string()
    } else {
        format!("{}/api/v3", account.base_url.trim_end_matches('/'))
    }
}

// ── Shared GraphQL response envelope ────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GqlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GqlError>>,
}

#[derive(Debug, Deserialize)]
struct GqlError {
    message: String,
}

// ── github.com GraphQL shapes ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ViewerData {
    viewer: Viewer,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Viewer {
    login: String,
    assigned_issues: IssueConnection,
    pull_requests: PrConnection,
    projects_v2: ProjectConnection,
    review_requests: ReviewRequestConnection,
}

#[derive(Debug, Deserialize)]
struct ReviewRequestConnection {
    nodes: Vec<GhReviewRequestPr>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhReviewRequestPr {
    id: String,
    number: u64,
    title: String,
    url: String,
    updated_at: String,
    is_draft: bool,
    state: String,
    review_decision: Option<String>,
    repository: GhRepo,
    author: Option<GhUser>,
    commits: GhCommitConnection,
}

#[derive(Debug, Deserialize)]
struct IssueConnection {
    nodes: Vec<GhIssue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhIssue {
    id: String,
    number: u64,
    title: String,
    url: String,
    updated_at: String,
    repository: GhRepo,
    labels: LabelConnection,
}

#[derive(Debug, Deserialize)]
struct GhRepo {
    #[serde(rename = "nameWithOwner")]
    name_with_owner: String,
}

#[derive(Debug, Default, Deserialize)]
struct LabelConnection {
    nodes: Vec<GhLabel>,
}

#[derive(Debug, Deserialize)]
struct GhLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct PrConnection {
    nodes: Vec<GhPr>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhPr {
    id: String,
    number: u64,
    title: String,
    url: String,
    updated_at: String,
    is_draft: bool,
    state: String,
    review_decision: Option<String>,
    repository: GhRepo,
    commits: GhCommitConnection,
}

#[derive(Debug, Deserialize)]
struct GhCommitConnection {
    nodes: Vec<GhCommitNode>,
}

#[derive(Debug, Deserialize)]
struct GhCommitNode {
    commit: GhCommit,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhCommit {
    status_check_rollup: Option<GhStatusRollup>,
}

#[derive(Debug, Deserialize)]
struct GhStatusRollup {
    state: String, // SUCCESS | FAILURE | ERROR | PENDING | EXPECTED
}

#[derive(Debug, Deserialize)]
struct ProjectConnection {
    nodes: Vec<GhProject>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhProject {
    title: String,
    url: String,
    items: ProjectItemConnection,
}

#[derive(Debug, Deserialize)]
struct ProjectItemConnection {
    nodes: Vec<GhProjectItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhProjectItem {
    id: String,
    updated_at: Option<String>,
    content: Option<GhProjectItemContent>,
    field_values: ProjectFieldValueConnection,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
enum GhProjectItemContent {
    Issue(GhProjectIssue),
    PullRequest(GhProjectPr),
    DraftIssue(GhProjectDraft),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhProjectIssue {
    id: String,
    assignees: AssigneeConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhProjectPr {
    id: String,
    assignees: AssigneeConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhProjectDraft {
    id: String,
    title: String,
    assignees: AssigneeConnection,
}

#[derive(Debug, Deserialize)]
struct AssigneeConnection {
    nodes: Vec<GhUser>,
}

#[derive(Debug, Deserialize)]
struct GhUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct ProjectFieldValueConnection {
    nodes: Vec<ProjectFieldValue>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
enum ProjectFieldValue {
    ProjectV2ItemFieldSingleSelectValue(SingleSelectValue),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct SingleSelectValue {
    name: Option<String>,
    field: ProjectField,
}

#[derive(Debug, Deserialize)]
struct ProjectField {
    name: Option<String>,
}

// ── GHE REST shapes ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RestSearchResult {
    total_count: u64,
    items: Vec<RestIssue>,
}

#[derive(Debug, Deserialize)]
struct RestIssue {
    id: u64,
    number: u64,
    title: String,
    html_url: String,
    updated_at: String,
    labels: Vec<RestLabel>,
    repository_url: String,
    #[serde(default)]
    draft: bool,
    user: Option<RestUser>,
}

impl RestIssue {
    fn author_login(&self) -> Option<String> {
        self.user.as_ref().map(|u| u.login.clone())
    }
}

#[derive(Debug, Deserialize)]
struct RestUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct RestLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RestPrDetail {
    head: RestPrHead,
}

#[derive(Debug, Deserialize)]
struct RestPrHead {
    sha: String,
}

#[derive(Debug, Deserialize)]
struct RestCombinedStatus {
    state: String, // "success" | "failure" | "error" | "pending"
}

// Review state from the PR reviews endpoint
#[derive(Debug, Deserialize)]
struct RestReview {
    state: String, // "APPROVED" | "CHANGES_REQUESTED" | "COMMENTED" | …
    #[serde(default)]
    body: String,
}

#[derive(Debug, Deserialize)]
struct RestInlineComment {
    body: String,
    path: String,
}

// ── State helpers ────────────────────────────────────────────────────────────

fn status_field(item: &GhProjectItem) -> Option<String> {
    for fv in &item.field_values.nodes {
        if let ProjectFieldValue::ProjectV2ItemFieldSingleSelectValue(sv) = fv {
            if sv.field.name.as_deref().map(|n| n.to_lowercase()) == Some("status".into()) {
                return sv.name.clone();
            }
        }
    }
    None
}

fn map_project_status(status: Option<&str>) -> TaskState {
    match status.map(|s| s.to_lowercase()).as_deref() {
        Some(s) if s.contains("in progress") || s.contains("doing") || s.contains("wip") => {
            TaskState::InProgress
        }
        Some(s) if s.contains("done") || s.contains("closed") || s.contains("complete") => {
            TaskState::Closed
        }
        _ => TaskState::Open,
    }
}

fn issue_state_from_labels(labels: &[impl AsRef<str>]) -> TaskState {
    if labels.iter().any(|l| {
        let l = l.as_ref().to_lowercase();
        l.contains("in progress") || l.contains("doing") || l == "wip"
    }) {
        TaskState::InProgress
    } else {
        TaskState::Open
    }
}

fn map_pr_state(is_draft: bool, state: &str, review_decision: Option<&str>) -> TaskState {
    if is_draft {
        return TaskState::Draft;
    }
    match state {
        "MERGED" | "merged" => return TaskState::Merged,
        "CLOSED" | "closed" => return TaskState::Closed,
        _ => {}
    }
    match review_decision {
        Some("CHANGES_REQUESTED") => TaskState::ChangesRequested,
        Some("APPROVED") => TaskState::Approved,
        _ => TaskState::Open,
    }
}

fn is_bot_login(login: &str) -> bool {
    let l = login.to_lowercase();
    l.ends_with("[bot]") || l.ends_with("_bot") || l == "bot"
}

fn map_ci_status(state: &str) -> Option<CiStatus> {
    match state {
        "SUCCESS" | "success" => Some(CiStatus::Success),
        "FAILURE" | "ERROR" | "failure" | "error" | "failed" => Some(CiStatus::Failure),
        "PENDING" | "pending" | "waiting_for_resource" | "preparing" => Some(CiStatus::Success),
        "IN_PROGRESS" | "RUNNING" | "running" => Some(CiStatus::Running),
        // EXPECTED = checks expected but not yet triggered (no CI, stale branch, etc.)
        // STALE    = results outdated — treat both as no meaningful status
        _ => None,
    }
}

fn ci_from_commits(commits: &GhCommitConnection) -> Option<CiStatus> {
    commits.nodes.first()
        .and_then(|n| n.commit.status_check_rollup.as_ref())
        .and_then(|r| map_ci_status(&r.state))
}

fn repo_from_url(repository_url: &str) -> String {
    // "https://github.example.com/api/v3/repos/org/repo" → "org/repo"
    repository_url
        .split("/repos/")
        .nth(1)
        .unwrap_or(repository_url)
        .to_string()
}

/// Returns true if the ISO 8601 timestamp is within the last 24 hours.
/// Uses only string comparison on the hour portion — no chrono dependency.
fn recently_updated(updated_at: &str) -> bool {
    // updated_at looks like "2026-09-30T14:23:00Z"
    // We compare the full datetime string: anything >= (now - 24h) qualifies.
    // Approximate by keeping the last 48 hours to avoid timezone edge cases.
    // Since ISO 8601 sorts lexicographically this is correct.
    use std::time::{SystemTime, UNIX_EPOCH};
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format a threshold timestamp 48 h ago as "YYYY-MM-DDTHH" prefix
    let threshold_secs = now_secs.saturating_sub(48 * 3600);
    let threshold = format_iso_prefix(threshold_secs);
    updated_at >= threshold.as_str()
}

/// Format Unix seconds as "YYYY-MM-DDTHH" (first 13 chars of ISO 8601 UTC).
fn format_iso_prefix(secs: u64) -> String {
    let s = secs;
    let days = s / 86400;
    let time_of_day = s % 86400;
    let hour = time_of_day / 3600;

    // Compute year/month/day from days since epoch (1970-01-01)
    let (y, m, d) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}T{:02}", y, m, d, hour)
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut y = 1970u64;
    let mut rem = days;
    loop {
        let dy = if is_leap(y) { 366 } else { 365 };
        if rem < dy { break; }
        rem -= dy;
        y += 1;
    }
    let months = if is_leap(y) {
        [31u64, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31u64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1u64;
    for &dm in &months {
        if rem < dm { break; }
        rem -= dm;
        m += 1;
    }
    (y, m, rem + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

// ── github.com — GraphQL ─────────────────────────────────────────────────────

const GH_COM_QUERY: &str = r#"
query {
  viewer {
    login
    assignedIssues(first: 50, states: [OPEN]) {
      nodes {
        id number title url updatedAt
        repository { nameWithOwner }
        labels(first: 5) { nodes { name } }
      }
    }
    pullRequests(first: 50, states: [OPEN]) {
      nodes {
        id number title url updatedAt isDraft state reviewDecision
        repository { nameWithOwner }
        commits(last: 1) { nodes { commit { statusCheckRollup { state } } } }
      }
    }
    reviewRequests(first: 50) {
      nodes {
        id number title url updatedAt isDraft state reviewDecision
        repository { nameWithOwner }
        author { login }
        commits(last: 1) { nodes { commit { statusCheckRollup { state } } } }
      }
    }
    projectsV2(first: 20) {
      nodes {
        title url
        items(first: 50) {
          nodes {
            id updatedAt
            content {
              __typename
              ... on Issue { id assignees(first: 10) { nodes { login } } }
              ... on PullRequest { id assignees(first: 10) { nodes { login } } }
              ... on DraftIssue { id title assignees(first: 10) { nodes { login } } }
            }
            fieldValues(first: 10) {
              nodes {
                __typename
                ... on ProjectV2ItemFieldSingleSelectValue {
                  name
                  field { ... on ProjectV2FieldCommon { name } }
                }
              }
            }
          }
        }
      }
    }
  }
}
"#;

async fn fetch_github_com(account: &Account, token: &str, client: &reqwest::Client) -> WorkList {
    let body = serde_json::json!({ "query": GH_COM_QUERY });
    let resp = client
        .post(graphql_url(account))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .json(&body)
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => return err_list(account, format!("Network error: {e}")),
    };

    let gql: GqlResponse<ViewerData> = match resp.json().await {
        Ok(v) => v,
        Err(e) => return err_list(account, format!("Parse error: {e}")),
    };

    if let Some(errors) = gql.errors {
        let msg = errors.iter().map(|e| e.message.as_str()).collect::<Vec<_>>().join("; ");
        return err_list(account, msg);
    }

    let viewer = match gql.data {
        Some(d) => d.viewer,
        None => return err_list(account, "Empty response".into()),
    };

    let login = &viewer.login;
    let mut items: Vec<WorkItem> = vec![];
    let mut seen_ids: HashSet<String> = HashSet::new();

    for issue in &viewer.assigned_issues.nodes {
        seen_ids.insert(issue.id.clone());
        let label_names: Vec<&str> = issue.labels.nodes.iter().map(|l| l.name.as_str()).collect();
        items.push(WorkItem {
            id: issue.id.clone(),
            kind: WorkKind::Issue,
            title: issue.title.clone(),
            url: issue.url.clone(),
            updated_at: issue.updated_at.clone(),
            state: issue_state_from_labels(&label_names),
            account: account.id.clone(),
            repo: Some(issue.repository.name_with_owner.clone()),
            number: Some(issue.number),
            project: None,
            author: None,
            ci_status: None,
        });
    }

    for pr in &viewer.pull_requests.nodes {
        seen_ids.insert(pr.id.clone());
        items.push(WorkItem {
            id: pr.id.clone(),
            kind: WorkKind::Pr,
            title: pr.title.clone(),
            url: pr.url.clone(),
            updated_at: pr.updated_at.clone(),
            state: map_pr_state(pr.is_draft, &pr.state, pr.review_decision.as_deref()),
            account: account.id.clone(),
            repo: Some(pr.repository.name_with_owner.clone()),
            number: Some(pr.number),
            project: None,
            author: None,
            ci_status: ci_from_commits(&pr.commits),
        });
    }

    for project in &viewer.projects_v2.nodes {
        for item in &project.items.nodes {
            let assigned_to_me = item.content.as_ref().map_or(false, |c| match c {
                GhProjectItemContent::Issue(i) => i.assignees.nodes.iter().any(|u| &u.login == login),
                GhProjectItemContent::PullRequest(p) => p.assignees.nodes.iter().any(|u| &u.login == login),
                GhProjectItemContent::DraftIssue(d) => d.assignees.nodes.iter().any(|u| &u.login == login),
            });
            if !assigned_to_me { continue; }

            let content_id = item.content.as_ref().and_then(|c| match c {
                GhProjectItemContent::Issue(i) => Some(i.id.clone()),
                GhProjectItemContent::PullRequest(p) => Some(p.id.clone()),
                GhProjectItemContent::DraftIssue(d) => Some(d.id.clone()),
            });

            if let Some(ref cid) = content_id {
                if seen_ids.contains(cid) {
                    if let Some(w) = items.iter_mut().find(|w| &w.id == cid) {
                        w.project = Some(project.title.clone());
                    }
                    continue;
                }
            }

            let (title, url, kind) = match &item.content {
                Some(GhProjectItemContent::DraftIssue(d)) => {
                    (d.title.clone(), project.url.clone(), WorkKind::ProjectItem)
                }
                _ => continue,
            };

            items.push(WorkItem {
                id: item.id.clone(),
                kind,
                title,
                url,
                updated_at: item.updated_at.clone().unwrap_or_default(),
                state: map_project_status(status_field(item).as_deref()),
                account: account.id.clone(),
                repo: None,
                number: None,
                project: Some(project.title.clone()),
                author: None,
                ci_status: None,
            });
        }
    }

    for pr in &viewer.review_requests.nodes {
        if pr.author.as_ref().map_or(false, |a| is_bot_login(&a.login)) { continue; }
        if seen_ids.contains(&pr.id) { continue; }
        items.push(WorkItem {
            id: format!("gh-review-{}", pr.id),
            kind: WorkKind::ReviewRequest,
            title: pr.title.clone(),
            url: pr.url.clone(),
            updated_at: pr.updated_at.clone(),
            state: map_pr_state(pr.is_draft, &pr.state, pr.review_decision.as_deref()),
            account: account.id.clone(),
            repo: Some(pr.repository.name_with_owner.clone()),
            number: Some(pr.number),
            project: None,
            author: pr.author.as_ref().map(|a| a.login.clone()),
            ci_status: ci_from_commits(&pr.commits),
        });
    }

    WorkList { account_id: account.id.clone(), items, error: None }
}

// Response shape for node(id:) query — returns a single ProjectV2
#[derive(Debug, Deserialize)]
struct GheNodeData {
    node: Option<GheProject>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GheProject {
    title: String,
    url: String,
    items: GheProjectItemConnection,
}

#[derive(Debug, Deserialize)]
struct GheProjectItemConnection {
    nodes: Vec<GheProjectItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GheProjectItem {
    id: String,
    updated_at: Option<String>,
    content: Option<GheProjectItemContent>,
    field_values: ProjectFieldValueConnection,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
enum GheProjectItemContent {
    Issue(GheProjectIssue),
    DraftIssue(GheProjectDraft),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GheProjectIssue {
    id: String,
    number: u64,
    title: String,
    url: String,
    updated_at: String,
    repository: GhRepo,
    assignees: AssigneeConnection,
    #[serde(default)]
    labels: LabelConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GheProjectDraft {
    id: String,
    title: String,
    assignees: AssigneeConnection,
}


/// Fetch items assigned to `login` from a single project by its node ID.
async fn fetch_ghe_project_by_id(
    client: &reqwest::Client,
    token: &str,
    graphql_url: &str,
    project_id: &str,
    login: &str,
) -> Vec<WorkItem> {
    let query = format!(r#"
query {{
  node(id: "{project_id}") {{
    ... on ProjectV2 {{
      title url
      items(first: 50) {{
        nodes {{
          id updatedAt
          content {{
            __typename
            ... on Issue {{
              id number title url updatedAt
              repository {{ nameWithOwner }}
              assignees(first: 5) {{ nodes {{ login }} }}
              labels(first: 3) {{ nodes {{ name }} }}
            }}
            ... on DraftIssue {{
              id title
              assignees(first: 5) {{ nodes {{ login }} }}
            }}
          }}
          fieldValues(first: 3) {{
            nodes {{
              __typename
              ... on ProjectV2ItemFieldSingleSelectValue {{
                name
                field {{ ... on ProjectV2FieldCommon {{ name }} }}
              }}
            }}
          }}
        }}
      }}
    }}
  }}
}}
"#);

    let body = serde_json::json!({ "query": query });
    let resp = match client
        .post(graphql_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => { eprintln!("[ghe-projects] network error: {e}"); return vec![]; }
    };

    let raw = match resp.text().await {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    let gql: GqlResponse<GheNodeData> = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => { eprintln!("[ghe-projects] parse error: {e}"); return vec![]; }
    };

    if let Some(ref errors) = gql.errors {
        for e in errors { eprintln!("[ghe-projects] GraphQL error: {}", e.message); }
    }

    let project = match gql.data.and_then(|d| d.node) {
        Some(p) => p,
        None => return vec![],
    };

    let mut result: Vec<WorkItem> = vec![];

    for item in &project.items.nodes {
        let status_val = status_field_ghe(item).map(|s| s.to_lowercase()).unwrap_or_default();
        let active = status_val.contains("in progress")
            || status_val.contains("sprint backlog")
            || status_val.contains("in review")
            || status_val.contains("doing");
        if !active { continue; }

        let assigned = item.content.as_ref().map_or(false, |c| match c {
            GheProjectItemContent::Issue(i) => i.assignees.nodes.iter().any(|u| u.login == login),
            GheProjectItemContent::DraftIssue(d) => d.assignees.nodes.iter().any(|u| u.login == login),
        });
        if !assigned { continue; }

        match &item.content {
            Some(GheProjectItemContent::Issue(i)) => {
                let label_names: Vec<&str> = i.labels.nodes.iter().map(|l| l.name.as_str()).collect();
                result.push(WorkItem {
                    id: format!("ghe-proj-issue-{}", i.id),
                    kind: WorkKind::Issue,
                    title: i.title.clone(),
                    url: i.url.clone(),
                    updated_at: i.updated_at.clone(),
                    state: issue_state_from_labels(&label_names),
                    account: String::new(),
                    repo: Some(i.repository.name_with_owner.clone()),
                    number: Some(i.number),
                    project: Some(project.title.clone()),
                    author: None,
                    ci_status: None,
                });
            }
            Some(GheProjectItemContent::DraftIssue(d)) => {
                result.push(WorkItem {
                    id: format!("ghe-proj-draft-{}", d.id),
                    kind: WorkKind::ProjectItem,
                    title: d.title.clone(),
                    url: project.url.clone(),
                    updated_at: item.updated_at.clone().unwrap_or_default(),
                    state: map_project_status(status_field_ghe(item).as_deref()),
                    account: String::new(),
                    repo: None,
                    number: None,
                    project: Some(project.title.clone()),
                    author: None,
                    ci_status: None,
                });
            }
            _ => {}
        }
    }

    result
}

fn status_field_ghe(item: &GheProjectItem) -> Option<String> {
    for fv in &item.field_values.nodes {
        if let ProjectFieldValue::ProjectV2ItemFieldSingleSelectValue(sv) = fv {
            if sv.field.name.as_deref().map(|n| n.to_lowercase()) == Some("status".into()) {
                return sv.name.clone();
            }
        }
    }
    None
}

/// Fetch all pages of a GitHub search query (per_page=100, page=1..N).
async fn search_all_pages(
    client: &reqwest::Client,
    token: &str,
    base_url: &str,
) -> Vec<RestIssue> {
    let per_page = 100u64;
    let mut all: Vec<RestIssue> = vec![];
    let mut page = 1u64;

    loop {
        let url = format!("{base_url}&per_page={per_page}&page={page}");
        let result = match client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(USER_AGENT, "organiser/0.1")
            .send()
            .await
        {
            Ok(r) => {
                let text = match r.text().await {
                    Ok(t) => t,
                    Err(_) => break,
                };
                match serde_json::from_str::<RestSearchResult>(&text) {
                    Ok(r) => r,
                    Err(e) => { eprintln!("[search] parse error: {e}\nraw: {}", &text[..text.len().min(300)]); break; }
                }
            },
            Err(e) => { eprintln!("[search] network error: {e}"); break; },
        };

        let fetched = result.items.len() as u64;
        all.extend(result.items);

        if all.len() as u64 >= result.total_count || fetched < per_page {
            break;
        }
        page += 1;
    }

    all
}

/// Fetch review decision + CI status for a single GHE PR.
/// Returns (review_decision, ci_status). Uses 2 REST calls: detail (SHA) + combined status.
async fn fetch_ghe_pr_extras(
    client: &reqwest::Client,
    token: &str,
    base: &str,
    repo: &str,
    number: u64,
) -> (Option<String>, Option<CiStatus>) {
    let detail_url = format!("{base}/repos/{repo}/pulls/{number}");
    let sha = match client
        .get(&detail_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .send()
        .await
    {
        Ok(r) => match r.json::<RestPrDetail>().await {
            Ok(d) => d.head.sha,
            Err(_) => return (None, None),
        },
        Err(_) => return (None, None),
    };

    let reviews_url = format!("{base}/repos/{repo}/pulls/{number}/reviews");
    let status_url = format!("{base}/repos/{repo}/commits/{sha}/status");

    let review_decision = match client
        .get(&reviews_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .send()
        .await
    {
        Ok(r) => match r.json::<Vec<RestReview>>().await {
            Ok(reviews) => reviews.iter().rev()
                .find(|r| r.state != "COMMENTED" && r.state != "DISMISSED")
                .map(|r| r.state.clone()),
            Err(_) => None,
        },
        Err(_) => None,
    };

    let ci_status = match client
        .get(&status_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .send()
        .await
    {
        Ok(r) => match r.json::<RestCombinedStatus>().await {
            Ok(s) => map_ci_status(&s.state),
            Err(_) => None,
        },
        Err(_) => None,
    };

    (review_decision, ci_status)
}

async fn fetch_ghe(
    account: &Account,
    token: &str,
    client: &reqwest::Client,
    skip_projects: bool,
) -> WorkList {
    let base = rest_base(account);

    // Get login first
    let login: String = match client
        .get(format!("{base}/user"))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .send()
        .await
    {
        Ok(r) => match r.json::<serde_json::Value>().await {
            Ok(v) => v["login"].as_str().unwrap_or("").to_string(),
            Err(e) => return err_list(account, format!("Auth error: {e}")),
        },
        Err(e) => return err_list(account, format!("Network error: {e}")),
    };

    if login.is_empty() {
        return err_list(account, "Could not determine login".into());
    }

    let mut items: Vec<WorkItem> = vec![];

    // Assigned open issues (not PRs) — paginated
    let issue_base = format!("{base}/search/issues?q=assignee:{login}+is:issue+is:open");
    for issue in search_all_pages(client, token, &issue_base).await {
        let label_names: Vec<&str> = issue.labels.iter().map(|l| l.name.as_str()).collect();
        items.push(WorkItem {
            id: format!("gh-issue-{}", issue.id),
            kind: WorkKind::Issue,
            title: issue.title,
            url: issue.html_url,
            updated_at: issue.updated_at,
            state: issue_state_from_labels(&label_names),
            account: account.id.clone(),
            repo: Some(repo_from_url(&issue.repository_url)),
            number: Some(issue.number),
            project: None,
            author: None,
            ci_status: None,
        });
    }

    // My open PRs — paginated; search returns them as issues with pull_request field
    let pr_base = format!("{base}/search/issues?q=author:{login}+is:pr+is:open");
    for pr_stub in search_all_pages(client, token, &pr_base).await {
        let repo = repo_from_url(&pr_stub.repository_url);

        // For recently-updated PRs: one detail call gives us head SHA (for CI) + reviews
        let (review_decision, ci_status) = if recently_updated(&pr_stub.updated_at) {
            fetch_ghe_pr_extras(client, token, &base, &repo, pr_stub.number).await
        } else {
            (None, None)
        };

        items.push(WorkItem {
            id: format!("gh-pr-{}", pr_stub.id),
            kind: WorkKind::Pr,
            title: pr_stub.title,
            url: pr_stub.html_url,
            updated_at: pr_stub.updated_at,
            state: map_pr_state(pr_stub.draft, "open", review_decision.as_deref()),
            account: account.id.clone(),
            repo: Some(repo),
            number: Some(pr_stub.number),
            project: None,
            author: None,
            ci_status,
        });
    }

    // Review-requested PRs
    let review_base = format!("{base}/search/issues?q=review-requested:{login}+is:pr+is:open");
    for pr_stub in search_all_pages(client, token, &review_base).await {
        let author = pr_stub.author_login();
        if author.as_deref().map_or(false, |a| is_bot_login(a)) { continue; }
        let repo = repo_from_url(&pr_stub.repository_url);
        let ci_status = if recently_updated(&pr_stub.updated_at) {
            fetch_ghe_pr_extras(client, token, &base, &repo, pr_stub.number).await.1
        } else {
            None
        };
        items.push(WorkItem {
            id: format!("gh-review-{}", pr_stub.id),
            kind: WorkKind::ReviewRequest,
            title: pr_stub.title,
            url: pr_stub.html_url,
            updated_at: pr_stub.updated_at,
            state: map_pr_state(pr_stub.draft, "open", None),
            account: account.id.clone(),
            repo: Some(repo),
            number: Some(pr_stub.number),
            project: None,
            author,
            ci_status,
        });
    }

    // Project items — only fetch if the user has selected specific projects in settings
    if account.selected_projects.is_empty() || skip_projects {
        return WorkList { account_id: account.id.clone(), items, error: None };
    }

    let gql_url = graphql_url(account);
    let seen: std::collections::HashSet<(String, u64)> = items.iter()
        .filter_map(|i| i.repo.as_ref().zip(i.number).map(|(r, n)| (r.clone(), n)))
        .collect();

    for project_id in &account.selected_projects {
        let mut proj_items = fetch_ghe_project_by_id(client, token, &gql_url, project_id, &login).await;
        for item in proj_items.iter_mut() {
            item.account = account.id.clone();
        }
        for item in proj_items {
            let key = item.repo.as_ref().zip(item.number).map(|(r, n)| (r.clone(), n));
            if key.as_ref().map_or(false, |k| seen.contains(k)) {
                if let Some(existing) = items.iter_mut().find(|i| {
                    i.repo == item.repo && i.number == item.number
                }) {
                    existing.project = item.project;
                }
                continue;
            }
            items.push(item);
        }
    }

    WorkList { account_id: account.id.clone(), items, error: None }
}

// ── Public entry point ───────────────────────────────────────────────────────

pub async fn fetch_github(account: &Account, token: &str, skip_projects: bool) -> WorkList {
    let client = reqwest::Client::new();
    match account.kind {
        AccountKind::Github => fetch_github_com(account, token, &client).await,
        AccountKind::GithubEnterprise => fetch_ghe(account, token, &client, skip_projects).await,
        _ => err_list(account, "Wrong provider kind".into()),
    }
}

fn err_list(account: &Account, error: String) -> WorkList {
    WorkList { account_id: account.id.clone(), items: vec![], error: Some(error) }
}

// ── GHE project metadata (for settings UI) ──────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct GheProjectMeta {
    pub id: String,
    pub title: String,
    pub org: String,
}

pub async fn fetch_ghe_projects_meta(
    account: &Account,
    token: &str,
) -> Result<Vec<GheProjectMeta>, String> {
    let client = reqwest::Client::new();
    let base = rest_base(account);
    let gql_url = graphql_url(account);

    // Fetch orgs (skip blocked ones)
    let blocked: std::collections::HashSet<String> = account.blocked_orgs
        .iter().map(|o| o.to_lowercase()).collect();
    let orgs: Vec<String> = {
        let resp = client
            .get(format!("{base}/user/orgs?per_page=100"))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(USER_AGENT, "organiser/0.1")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        resp.json::<Vec<serde_json::Value>>()
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter_map(|v| v["login"].as_str().map(|s| s.to_string()))
            .filter(|o| !blocked.contains(&o.to_lowercase()))
            .collect()
    };

    let mut result = vec![];

    for org in &orgs {
        let query = format!(r#"query {{ organization(login: "{org}") {{ projectsV2(first: 20) {{ nodes {{ id title }} }} }} }}"#);
        let body = serde_json::json!({ "query": query });
        let resp = match client
            .post(&gql_url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(USER_AGENT, "organiser/0.1")
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => continue,
        };
        let v: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(nodes) = v["data"]["organization"]["projectsV2"]["nodes"].as_array() {
            for node in nodes {
                if let (Some(id), Some(title)) = (
                    node["id"].as_str(),
                    node["title"].as_str(),
                ) {
                    result.push(GheProjectMeta {
                        id: id.to_string(),
                        title: title.to_string(),
                        org: org.clone(),
                    });
                }
            }
        }
    }

    Ok(result)
}

pub async fn fetch_pr_review_comments(
    account: &Account,
    token: &str,
    repo: &str,
    number: u64,
) -> Result<String, String> {
    let base = rest_base(account);
    let client = reqwest::Client::new();

    let reviews: Vec<RestReview> = client
        .get(format!("{base}/repos/{repo}/pulls/{number}/reviews"))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let inline: Vec<RestInlineComment> = client
        .get(format!("{base}/repos/{repo}/pulls/{number}/comments"))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, "organiser/0.1")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let mut parts: Vec<String> = vec![];

    let review_bodies: Vec<&str> = reviews
        .iter()
        .filter(|r| r.state == "CHANGES_REQUESTED" && !r.body.trim().is_empty())
        .map(|r| r.body.as_str())
        .collect();

    if !review_bodies.is_empty() {
        parts.push("Review comments:".into());
        for body in review_bodies {
            parts.push(format!("- {body}"));
        }
    }

    if !inline.is_empty() {
        parts.push("\nInline comments:".into());
        for c in &inline {
            if !c.body.trim().is_empty() {
                parts.push(format!("[{}] {}", c.path, c.body));
            }
        }
    }

    if parts.is_empty() {
        Ok("No review comments found.".into())
    } else {
        Ok(parts.join("\n"))
    }
}
