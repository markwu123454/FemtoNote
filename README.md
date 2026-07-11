# Bandnote

A schedule-aware, low-friction note-capture desktop app. Open it and type — it
silently files each note under the class you're currently in, and later exports
clean per-subject markdown you can feed into an AI research tool (e.g. NotebookLM).

Built per [`project-descriptor.md`](project-descriptor.md). This is the MVP cut:
schedule setup → instant, auto-routed capture → silent timestamped autosave →
local markdown storage → per-subject export.

## Stack

- **Tauri v2** (Rust backend, system webview) — sub-second start, native file writes.
- **Svelte + Vite** frontend — minimal JS, snappy capture UI.
- **Storage:** plain markdown on disk. One file per capture *session* under the OS
  app-data dir; export re-assembles them into clean per-subject files.

## Run it

```bash
npm install
npm run tauri dev      # launches the native window with hot reload
```

## Build a release binary / installer

```bash
npm run tauri build    # output under src-tauri/target/release/
```

## How it works

- **Setup (once):** open **⚙ Settings** → add your subjects, then add weekly
  schedule blocks (subject · day · start–end). Pick an export folder and theme.
- **Capture (constant):** the cursor lands in the editor on open. Just type. The
  top-bar **Subject** indicator turns accent-colored when auto-routing is active.
  Off-schedule text falls to **Inbox** so typing is never blocked. Autosave is
  continuous (350 ms debounce, atomic writes). At a class boundary the app won't
  yank your text — it shows a clickable *"class changed → switch"* prompt in the
  status bar instead.
- **Export (occasional):** **Ctrl/⌘-E** (or the command palette, **Ctrl/⌘-K**) →
  pick one subject or all → preview → export. Produces date-ordered markdown with
  clean `#`/`##` headings that parse well in NotebookLM. Raw notes are never
  altered or deleted.

### Keyboard

| Shortcut | Action |
| --- | --- |
| `Ctrl/⌘-K` | Command palette |
| `Ctrl/⌘-E` | Export |
| `Ctrl/⌘-,` | Settings |
| `Esc` | Close dialog / search / refocus editor |

## Storage layout

```
<app-data>/Bandnote/
  config.json                        # subjects, schedule, export dir, theme
  notes/
    <subject_id>/
      2026-07-10_14-03-05.md         # one markdown file per session
```

Each session file carries a tiny frontmatter (`subject`, `subject_id`, `started`)
and the raw note body, so it stays readable in any editor and backup-able via your
own Dropbox/git. On Windows the app-data dir is
`%APPDATA%\com.bandnote.app\` (shown in Settings → Output).

## Project structure

```
src/                 Svelte frontend
  App.svelte         three-band capture UI + orchestration
  lib/api.js         wrappers over the Rust commands
  lib/routing.js     schedule → active-subject logic + time helpers
  components/        Settings, ExportDialog
src-tauri/src/lib.rs Rust: config + session storage, export rendering
```

## Not yet built (deferred per the descriptor's MVP cut)

Global summon hotkey, `.ics` schedule import, A/B-week patterns, image paste,
full-text search index (current search scans session previews), the best-effort
`notebooklm-py` push. Manual markdown export is the stable core and stays the
fallback.
