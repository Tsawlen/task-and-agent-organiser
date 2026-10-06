use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const HOOK_CMD: &str =
    "curl -sf -X POST http://127.0.0.1:27384/hook -H 'Content-Type: application/json' -d @-";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HookSupport {
    Supported,    // config-based hook install possible
    FileRequired, // needs a plugin file written (OpenCode)
    Unsupported,  // no hook mechanism
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentHookStatus {
    pub id: String,
    pub name: String,
    pub support: HookSupport,
    pub installed: bool,
    pub config_path: String, // display only
}

fn home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

// ── Claude Code ──────────────────────────────────────────────────────────────

fn claude_config_path() -> Option<PathBuf> {
    Some(home()?.join(".claude/settings.json"))
}

fn claude_hooks_installed() -> bool {
    let path = match claude_config_path() {
        Some(p) => p,
        None => return false,
    };
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let root: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return false,
    };
    json_hooks_installed(&root)
}

fn json_hooks_installed(root: &serde_json::Value) -> bool {
    let hooks = match root.get("hooks") {
        Some(h) => h,
        None => return false,
    };
    // Enough to check one event — if the command is in PostToolUse it was us
    hooks
        .get("PostToolUse")
        .and_then(|v| v.as_array())
        .map_or(false, |arr| {
            arr.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(|h| h.as_array())
                    .map_or(false, |hs| {
                        hs.iter().any(|h| {
                            h.get("command").and_then(|c| c.as_str()) == Some(HOOK_CMD)
                        })
                    })
            })
        })
}

fn set_claude_hooks(install: bool) -> Result<(), String> {
    let path = claude_config_path().ok_or("HOME not set")?;
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let mut root: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("JSON parse error: {e}"))?;

    if install {
        merge_json_hooks(&mut root);
    } else {
        remove_json_hooks(&mut root);
    }

    let updated =
        serde_json::to_string_pretty(&root).map_err(|e| format!("JSON encode error: {e}"))?;
    fs::write(&path, updated).map_err(|e| format!("Write error: {e}"))
}

fn merge_json_hooks(root: &mut serde_json::Value) {
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
    let new_entry = serde_json::json!({
        "matcher": "",
        "hooks": [{ "type": "command", "command": HOOK_CMD }]
    });
    for event in EVENTS {
        let arr = hooks_obj
            .entry(*event)
            .or_insert_with(|| serde_json::Value::Array(vec![]))
            .as_array_mut();
        if let Some(arr) = arr {
            let already = arr.iter().any(|e| {
                e.get("hooks")
                    .and_then(|h| h.as_array())
                    .map_or(false, |hs| {
                        hs.iter().any(|h| {
                            h.get("command").and_then(|c| c.as_str()) == Some(HOOK_CMD)
                        })
                    })
            });
            if !already {
                arr.push(new_entry.clone());
            }
        }
    }
}

fn remove_json_hooks(root: &mut serde_json::Value) {
    let hooks = match root.get_mut("hooks").and_then(|v| v.as_object_mut()) {
        Some(h) => h,
        None => return,
    };
    for arr in hooks.values_mut() {
        if let Some(arr) = arr.as_array_mut() {
            arr.retain(|entry| {
                !entry
                    .get("hooks")
                    .and_then(|h| h.as_array())
                    .map_or(false, |hs| {
                        hs.iter().any(|h| {
                            h.get("command").and_then(|c| c.as_str()) == Some(HOOK_CMD)
                        })
                    })
            });
        }
    }
}

// ── Codex ────────────────────────────────────────────────────────────────────

fn codex_config_path() -> Option<PathBuf> {
    Some(home()?.join(".codex/config.toml"))
}

fn codex_installed() -> bool {
    let path = match codex_config_path() {
        Some(p) => p,
        None => return false,
    };
    fs::read_to_string(&path)
        .map(|c| c.contains(HOOK_CMD))
        .unwrap_or(false)
}

fn set_codex_hooks(install: bool) -> Result<(), String> {
    let path = codex_config_path().ok_or("HOME not set")?;

    let existing = fs::read_to_string(&path).unwrap_or_default();

    if install {
        if existing.contains(HOOK_CMD) {
            return Ok(());
        }
        let block = format!(
            r#"
[[hooks.SessionStart]]
[[hooks.SessionStart.hooks]]
type = "command"
command = "{HOOK_CMD}"

[[hooks.PostToolUse]]
[[hooks.PostToolUse.hooks]]
type = "command"
command = "{HOOK_CMD}"

[[hooks.SessionEnd]]
[[hooks.SessionEnd.hooks]]
type = "command"
command = "{HOOK_CMD}"
"#
        );
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut content = existing;
        content.push_str(&block);
        fs::write(&path, content).map_err(|e| e.to_string())
    } else {
        // Remove the block we added — filter out lines referencing our command
        // and the surrounding [[hooks.*]] table headers that become empty
        let filtered = remove_toml_hook_block(&existing);
        fs::write(&path, filtered).map_err(|e| e.to_string())
    }
}

fn remove_toml_hook_block(content: &str) -> String {
    // Remove any line that is our command value or an [[hooks.*]] header
    // that has no remaining keys after removal. Simple line filter is safe
    // because we own the block we added (all three tables are ours).
    let mut out = Vec::new();
    let mut skip_blank = false;
    for line in content.lines() {
        if line.contains(HOOK_CMD)
            || (line.starts_with("[[hooks.") && line.ends_with("]]"))
            || (line.trim_start().starts_with("type") && line.contains("\"command\""))
            || (line.trim_start().starts_with("command") && line.contains(HOOK_CMD))
        {
            skip_blank = true;
            continue;
        }
        if skip_blank && line.trim().is_empty() {
            skip_blank = false;
            continue;
        }
        skip_blank = false;
        out.push(line);
    }
    out.join("\n")
}

// ── Gemini CLI ────────────────────────────────────────────────────────────────

fn gemini_config_path() -> Option<PathBuf> {
    Some(home()?.join(".gemini/settings.json"))
}

fn gemini_installed() -> bool {
    let path = match gemini_config_path() {
        Some(p) => p,
        None => return false,
    };
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let root: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return false,
    };
    json_hooks_installed(&root)
}

fn set_gemini_hooks(install: bool) -> Result<(), String> {
    let path = gemini_config_path().ok_or("HOME not set")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = fs::read_to_string(&path).unwrap_or_else(|_| "{}".into());
    let mut root: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("JSON parse error: {e}"))?;

    if install {
        merge_gemini_hooks(&mut root);
    } else {
        remove_json_hooks(&mut root);
    }

    let updated =
        serde_json::to_string_pretty(&root).map_err(|e| format!("JSON encode error: {e}"))?;
    fs::write(&path, updated).map_err(|e| e.to_string())
}

fn merge_gemini_hooks(root: &mut serde_json::Value) {
    // Gemini uses AfterTool / SessionStart / SessionEnd
    const EVENTS: &[&str] = &["AfterTool", "SessionStart", "SessionEnd"];
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
    let new_entry = serde_json::json!({
        "hooks": [{ "type": "command", "command": HOOK_CMD }]
    });
    for event in EVENTS {
        let arr = hooks_obj
            .entry(*event)
            .or_insert_with(|| serde_json::Value::Array(vec![]))
            .as_array_mut();
        if let Some(arr) = arr {
            let already = arr.iter().any(|e| {
                e.get("hooks")
                    .and_then(|h| h.as_array())
                    .map_or(false, |hs| {
                        hs.iter().any(|h| {
                            h.get("command").and_then(|c| c.as_str()) == Some(HOOK_CMD)
                        })
                    })
            });
            if !already {
                arr.push(new_entry.clone());
            }
        }
    }
}

// ── OpenCode ──────────────────────────────────────────────────────────────────

fn opencode_plugin_path() -> Option<PathBuf> {
    Some(home()?.join(".config/opencode/plugins/organiser-hook.ts"))
}

fn opencode_installed() -> bool {
    opencode_plugin_path()
        .map(|p| p.exists())
        .unwrap_or(false)
}

fn set_opencode_hooks(install: bool) -> Result<(), String> {
    let path = opencode_plugin_path().ok_or("HOME not set")?;
    if install {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = format!(
            r#"import type {{ Plugin }} from "@opencode-ai/sdk"

const hook = async (event: string, tool?: string) => {{
  const body = JSON.stringify({{ hook_event_name: event, tool_name: tool }})
  await fetch("http://127.0.0.1:27384/hook", {{
    method: "POST",
    headers: {{ "Content-Type": "application/json" }},
    body,
  }}).catch(() => {{}})
}}

export const OrganiserPlugin: Plugin = async () => {{
  return {{
    "session.created": async () => hook("SessionStart"),
    "tool.execute.after": async (input: any) => hook("PostToolUse", input?.tool),
  }}
}}

export default {{ server: OrganiserPlugin }}
"#
        );
        fs::write(&path, content).map_err(|e| e.to_string())
    } else {
        fs::remove_file(&path).map_err(|e| e.to_string())
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn all_statuses() -> Vec<AgentHookStatus> {
    vec![
        AgentHookStatus {
            id: "claude_code".into(),
            name: "Claude Code".into(),
            support: HookSupport::Supported,
            installed: claude_hooks_installed(),
            config_path: claude_config_path()
                .map(display_path)
                .unwrap_or_default(),
        },
        AgentHookStatus {
            id: "codex".into(),
            name: "Codex (OpenAI)".into(),
            support: HookSupport::Supported,
            installed: codex_installed(),
            config_path: codex_config_path()
                .map(display_path)
                .unwrap_or_default(),
        },
        AgentHookStatus {
            id: "gemini_cli".into(),
            name: "Gemini CLI".into(),
            support: HookSupport::Supported,
            installed: gemini_installed(),
            config_path: gemini_config_path()
                .map(display_path)
                .unwrap_or_default(),
        },
        AgentHookStatus {
            id: "opencode".into(),
            name: "OpenCode".into(),
            support: HookSupport::FileRequired,
            installed: opencode_installed(),
            config_path: opencode_plugin_path()
                .map(display_path)
                .unwrap_or_default(),
        },
        AgentHookStatus {
            id: "aider".into(),
            name: "Aider".into(),
            support: HookSupport::Unsupported,
            installed: false,
            config_path: String::new(),
        },
        AgentHookStatus {
            id: "amp".into(),
            name: "Amp (Sourcegraph)".into(),
            support: HookSupport::Unsupported,
            installed: false,
            config_path: String::new(),
        },
    ]
}

pub fn set_hook(agent_id: &str, install: bool) -> Result<(), String> {
    match agent_id {
        "claude_code" => set_claude_hooks(install),
        "codex" => set_codex_hooks(install),
        "gemini_cli" => set_gemini_hooks(install),
        "opencode" => set_opencode_hooks(install),
        _ => Err(format!("Unknown agent: {agent_id}")),
    }
}

fn display_path(p: PathBuf) -> String {
    // Replace $HOME with ~ for display
    let home = std::env::var("HOME").unwrap_or_default();
    let s = p.to_string_lossy().into_owned();
    if !home.is_empty() && s.starts_with(&home) {
        format!("~{}", &s[home.len()..])
    } else {
        s
    }
}

// Called at startup to auto-install Claude Code hooks (existing behaviour)
pub fn auto_install_claude() {
    if !claude_hooks_installed() {
        let _ = set_claude_hooks(true);
    }
}
