use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use runtime::{Session, MessageRole, ContentBlock, PermissionMode, load_system_prompt};
use compat_harness::{extract_manifest, UpstreamPaths};
use crate::init::InitReport;

pub fn current_date_string() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

pub fn default_permission_mode() -> PermissionMode {
    match env::var("RUSTY_CLAUDE_PERMISSION_MODE").as_deref() {
        Ok("read-only") => PermissionMode::ReadOnly,
        Ok("workspace-write") => PermissionMode::WorkspaceWrite,
        Ok("danger-full-access") => PermissionMode::DangerFullAccess,
        _ => PermissionMode::WorkspaceWrite,
    }
}

pub fn normalize_permission_mode(mode: &str) -> Option<&str> {
    match mode {
        "read-only" | "workspace-write" | "danger-full-access" => Some(mode),
        _ => None,
    }
}

pub fn permission_mode_from_label(label: &str) -> PermissionMode {
    match label {
        "read-only" => PermissionMode::ReadOnly,
        "workspace-write" => PermissionMode::WorkspaceWrite,
        "danger-full-access" => PermissionMode::DangerFullAccess,
        _ => PermissionMode::WorkspaceWrite,
    }
}

pub fn resolve_export_path(requested: Option<&str>, session: &Session) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = requested {
        return Ok(PathBuf::from(path));
    }
    let filename = format!("export-v{}.md", session.version);
    Ok(env::current_dir()?.join(filename))
}

pub fn write_temp_text_file(name: &str, content: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = env::temp_dir().join(name);
    fs::write(&path, content)?;
    Ok(path)
}

pub fn parse_titled_body(text: &str) -> Option<(String, String)> {
    let mut title = None;
    let mut body = Vec::new();
    let mut in_body = false;

    for line in text.lines() {
        if let Some(t) = line.strip_prefix("TITLE:") {
            title = Some(t.trim().to_string());
        } else if line.starts_with("BODY:") {
            in_body = true;
        } else if in_body {
            body.push(line);
        }
    }

    title.map(|t| (t, body.join("\n").trim().to_string()))
}

pub fn sanitize_generated_message(text: &str) -> String {
    text.trim()
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string()
}

pub fn recent_user_context(session: &Session, limit: usize) -> String {
    let messages: Vec<_> = session.messages.iter()
        .filter(|m| m.role == MessageRole::User)
        .collect();
    let take_count = messages.len().min(limit);
    let recent = &messages[messages.len() - take_count..];

    recent.iter()
        .map(|m| m.blocks.iter().filter_map(|b| match b {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        }).collect::<Vec<_>>().join("\n"))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

pub fn truncate_for_prompt(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        text.to_string()
    } else {
        format!("{}... [truncated]", &text[..limit])
    }
}

pub fn slash_command_completion_candidates() -> Vec<String> {
    vec![
        "/help", "/status", "/compact", "/model", "/permissions", "/clear", "/cost",
        "/resume", "/config", "/memory", "/init", "/diff", "/version", "/export",
        "/session", "/exit", "/quit"
    ].into_iter().map(String::from).collect()
}

pub fn dump_manifests() {
    let workspace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = UpstreamPaths::from_workspace_dir(&workspace_dir);
    match extract_manifest(&paths) {
        Ok(manifest) => {
            println!("commands: {}", manifest.commands.entries().len());
            println!("tools: {}", manifest.tools.entries().len());
            println!("bootstrap phases: {}", manifest.bootstrap.phases().len());
        }
        Err(error) => {
            eprintln!("failed to extract manifests: {error}");
            std::process::exit(1);
        }
    }
}

pub fn print_bootstrap_plan() {
    for phase in runtime::BootstrapPlan::claude_code_default().phases() {
        println!("- {phase:?}");
    }
}

pub fn print_system_prompt(cwd: PathBuf, date: String) {
    match load_system_prompt(cwd, date, env::consts::OS, "unknown") {
        Ok(sections) => println!("{}", sections.join("\n\n")),
        Err(error) => {
            eprintln!("failed to build system prompt: {error}");
            std::process::exit(1);
        }
    }
}

pub fn run_init() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let report = crate::init::initialize_repo(&cwd)?;
    println!("{}", report.render());
    Ok(())
}

pub fn init_claude_md() -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    Ok(crate::init::render_init_claude_md(&cwd))
}
