// Bandnote backend: local-first, plain-markdown storage.
//
// Layout on disk (under the OS app-data dir for this app):
//   config.json                      — schedule, subjects, export config
//   notes/<subject_id>/<session>.md  — one markdown file per capture session
//
// Session files are the source of truth and stay human-readable. Each carries a
// tiny YAML-ish frontmatter so a session can be understood standalone, followed
// by the raw note body. Export re-assembles them into clean per-subject files.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Subject {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Block {
    id: String,
    /// Serialized as `subjectId` to match the frontend's schedule model.
    subject_id: String,
    /// 0 = Sunday .. 6 = Saturday
    day: u8,
    /// "HH:MM" 24h
    start: String,
    /// "HH:MM" 24h
    end: String,
}

/// Writing-surface preferences. Every field carries its own serde default so a
/// config written by an older build (missing `editor`, or missing a field)
/// still loads cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditorPrefs {
    /// "serif" | "sans" | "mono"
    #[serde(default = "default_font")]
    font: String,
    #[serde(default = "default_font_size")]
    font_size: u32,
    #[serde(default = "default_line_height")]
    line_height: f32,
    /// Max width of the writing column, in px.
    #[serde(default = "default_max_width")]
    max_width: u32,
    /// "center" | "left"
    #[serde(default = "default_align")]
    align: String,
    /// Underline misspelled words (our own checker, not the OS webview).
    #[serde(default = "default_true")]
    spellcheck: bool,
    /// Auto-correct aggressiveness: "off" | "conservative" | "balanced".
    #[serde(default = "default_autocorrect")]
    autocorrect: String,
    /// Show the transient blue underline marking a word we auto-corrected.
    #[serde(default = "default_true")]
    autocorrect_hint: bool,
}

fn default_font() -> String {
    "serif".to_string()
}
fn default_font_size() -> u32 {
    18
}
fn default_line_height() -> f32 {
    1.7
}
fn default_max_width() -> u32 {
    780
}
fn default_align() -> String {
    "center".to_string()
}
fn default_true() -> bool {
    true
}
fn default_autocorrect() -> String {
    "conservative".to_string()
}

impl Default for EditorPrefs {
    fn default() -> Self {
        EditorPrefs {
            font: default_font(),
            font_size: default_font_size(),
            line_height: default_line_height(),
            max_width: default_max_width(),
            align: default_align(),
            spellcheck: default_true(),
            autocorrect: default_autocorrect(),
            autocorrect_hint: default_true(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    subjects: Vec<Subject>,
    schedule: Vec<Block>,
    #[serde(default)]
    export_dir: Option<String>,
    /// "system" | "light" | "dark"
    #[serde(default = "default_theme")]
    theme: String,
    #[serde(default)]
    editor: EditorPrefs,
    /// Words the user has taught the spell checker ("Save spelling").
    #[serde(default)]
    custom_words: Vec<String>,
}

fn default_theme() -> String {
    "system".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            subjects: vec![Subject {
                id: "inbox".to_string(),
                name: "Inbox".to_string(),
            }],
            schedule: vec![],
            export_dir: None,
            theme: default_theme(),
            editor: EditorPrefs::default(),
            custom_words: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionWrite {
    subject_id: String,
    subject_name: String,
    /// filesystem-safe id, sortable by time, e.g. "2026-07-10_14-03-05"
    session_id: String,
    /// ISO-ish display timestamp, e.g. "2026-07-10T14:03:05"
    started_at: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMeta {
    subject_id: String,
    subject_name: String,
    session_id: String,
    started_at: String,
    /// first ~80 chars of content, for browsing
    preview: String,
    words: usize,
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn data_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("cannot resolve app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create data dir: {e}"))?;
    Ok(dir)
}

fn notes_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = data_root(app)?.join("notes");
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create notes dir: {e}"))?;
    Ok(dir)
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(data_root(app)?.join("config.json"))
}

/// Keep only characters safe for a folder/file name.
fn sanitize(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
            ' ' => '-',
            _ => '_',
        })
        .collect();
    if cleaned.is_empty() {
        "untitled".to_string()
    } else {
        cleaned
    }
}

// ---------------------------------------------------------------------------
// Session file (de)serialization
// ---------------------------------------------------------------------------

fn session_file(app: &tauri::AppHandle, subject_id: &str, session_id: &str) -> Result<PathBuf, String> {
    let dir = notes_root(app)?.join(sanitize(subject_id));
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create subject dir: {e}"))?;
    Ok(dir.join(format!("{}.md", sanitize(session_id))))
}

fn render_session_file(s: &SessionWrite) -> String {
    format!(
        "---\nsubject: {}\nsubject_id: {}\nstarted: {}\n---\n\n{}",
        s.subject_name.replace('\n', " "),
        s.subject_id,
        s.started_at,
        s.content
    )
}

struct ParsedSession {
    subject_name: String,
    started_at: String,
    content: String,
}

fn parse_session_file(raw: &str) -> ParsedSession {
    let mut subject_name = String::from("Unknown");
    let mut started_at = String::new();
    let mut content = raw.to_string();

    if let Some(rest) = raw.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            let header = &rest[..end];
            for line in header.lines() {
                if let Some(v) = line.strip_prefix("subject: ") {
                    subject_name = v.trim().to_string();
                } else if let Some(v) = line.strip_prefix("started: ") {
                    started_at = v.trim().to_string();
                }
            }
            // Body is everything after the closing "---" line.
            let after = &rest[end + 4..];
            content = after.trim_start_matches('\n').to_string();
        }
    }

    ParsedSession {
        subject_name,
        started_at,
        content,
    }
}

fn words_in(s: &str) -> usize {
    s.split_whitespace().count()
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn load_config(app: tauri::AppHandle) -> Result<Config, String> {
    let path = config_path(&app)?;
    if !path.exists() {
        let cfg = Config::default();
        save_config(app.clone(), cfg.clone())?;
        return Ok(cfg);
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("cannot read config: {e}"))?;
    let cfg: Config = serde_json::from_str(&raw).map_err(|e| format!("cannot parse config: {e}"))?;
    Ok(cfg)
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, config: Config) -> Result<(), String> {
    let path = config_path(&app)?;
    let json = serde_json::to_string_pretty(&config).map_err(|e| format!("cannot serialize config: {e}"))?;
    atomic_write(&path, json.as_bytes())
}

/// Write (or overwrite) a session's markdown file. Called continuously by
/// autosave, so it must be cheap and crash-safe.
#[tauri::command]
fn write_session(app: tauri::AppHandle, session: SessionWrite) -> Result<(), String> {
    let path = session_file(&app, &session.subject_id, &session.session_id)?;
    // An empty session leaves no file behind (avoids litter from a stray open).
    if session.content.trim().is_empty() {
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
        return Ok(());
    }
    let body = render_session_file(&session);
    atomic_write(&path, body.as_bytes())
}

#[tauri::command]
fn list_sessions(app: tauri::AppHandle) -> Result<Vec<SessionMeta>, String> {
    let root = notes_root(&app)?;
    let mut out: Vec<SessionMeta> = Vec::new();

    let subject_dirs = match fs::read_dir(&root) {
        Ok(d) => d,
        Err(_) => return Ok(out),
    };

    for subj in subject_dirs.flatten() {
        if !subj.path().is_dir() {
            continue;
        }
        let subject_id = subj.file_name().to_string_lossy().to_string();
        let files = match fs::read_dir(subj.path()) {
            Ok(f) => f,
            Err(_) => continue,
        };
        for f in files.flatten() {
            let p = f.path();
            if p.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let session_id = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let raw = fs::read_to_string(&p).unwrap_or_default();
            let parsed = parse_session_file(&raw);
            let preview: String = parsed.content.chars().take(80).collect();
            out.push(SessionMeta {
                subject_id: subject_id.clone(),
                subject_name: parsed.subject_name,
                session_id,
                started_at: parsed.started_at,
                preview: preview.replace('\n', " "),
                words: words_in(&parsed.content),
            });
        }
    }

    // Newest first.
    out.sort_by(|a, b| b.session_id.cmp(&a.session_id));
    Ok(out)
}

/// Build clean, date-ordered markdown for one subject and return it (without
/// writing). Used for the export preview.
#[tauri::command]
fn preview_subject(app: tauri::AppHandle, subject_id: String, subject_name: String) -> Result<String, String> {
    render_subject_markdown(&app, &subject_id, &subject_name)
}

#[tauri::command]
fn export_subject(
    app: tauri::AppHandle,
    subject_id: String,
    subject_name: String,
    dest_dir: String,
) -> Result<String, String> {
    let md = render_subject_markdown(&app, &subject_id, &subject_name)?;
    let dest = PathBuf::from(&dest_dir);
    fs::create_dir_all(&dest).map_err(|e| format!("cannot create export dir: {e}"))?;
    let file = dest.join(format!("{}.md", sanitize(&subject_name)));
    atomic_write(&file, md.as_bytes())?;
    Ok(file.to_string_lossy().to_string())
}

#[tauri::command]
fn export_all(
    app: tauri::AppHandle,
    subjects: Vec<Subject>,
    dest_dir: String,
) -> Result<Vec<String>, String> {
    let mut written = Vec::new();
    for s in subjects {
        // Skip subjects with no notes on disk.
        let md = render_subject_markdown(&app, &s.id, &s.name)?;
        if md.trim().lines().count() <= 1 {
            continue;
        }
        let path = export_subject(app.clone(), s.id, s.name, dest_dir.clone())?;
        written.push(path);
    }
    Ok(written)
}

#[tauri::command]
fn data_dir(app: tauri::AppHandle) -> Result<String, String> {
    Ok(notes_root(&app)?.to_string_lossy().to_string())
}

// ---------------------------------------------------------------------------
// Rendering + IO helpers
// ---------------------------------------------------------------------------

fn render_subject_markdown(app: &tauri::AppHandle, subject_id: &str, subject_name: &str) -> Result<String, String> {
    let dir = notes_root(app)?.join(sanitize(subject_id));
    let mut sessions: Vec<(String, ParsedSession)> = Vec::new();

    if let Ok(files) = fs::read_dir(&dir) {
        for f in files.flatten() {
            let p = f.path();
            if p.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let sid = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let raw = fs::read_to_string(&p).unwrap_or_default();
            let parsed = parse_session_file(&raw);
            if parsed.content.trim().is_empty() {
                continue;
            }
            sessions.push((sid, parsed));
        }
    }

    // Chronological (session ids are timestamp-sortable).
    sessions.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = String::new();
    out.push_str(&format!("# {}\n", subject_name));
    for (_sid, s) in sessions {
        let stamp = if s.started_at.is_empty() {
            "(undated)".to_string()
        } else {
            s.started_at.clone()
        };
        out.push_str(&format!("\n## {}\n\n{}\n", stamp, s.content.trim_end()));
    }
    Ok(out)
}

/// Write to a temp file then rename over the target so a crash mid-write never
/// corrupts an existing file.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("cannot create dir: {e}"))?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp).map_err(|e| format!("cannot open temp file: {e}"))?;
        f.write_all(bytes).map_err(|e| format!("cannot write temp file: {e}"))?;
        f.flush().map_err(|e| format!("cannot flush temp file: {e}"))?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("cannot finalize file: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            write_session,
            list_sessions,
            preview_subject,
            export_subject,
            export_all,
            data_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Bandnote");
}
