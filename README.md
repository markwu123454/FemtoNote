<div align="center">

# BandNote

**A schedule-aware, low-friction note-capture desktop app for students.**

[![Release](https://img.shields.io/github/v/release/markwu123454/BandNote?sort=semver)](https://github.com/markwu123454/BandNote/releases)
[![Downloads](https://img.shields.io/github/downloads/markwu123454/BandNote/total)](https://github.com/markwu123454/BandNote/releases)
[![License: AGPL v3](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-0a7bbb)](https://github.com/markwu123454/BandNote/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)

Open it and type. Bandnote silently files each note under the class you're
currently in, autosaves as you go, and later exports clean per-subject markdown
you can feed straight into an AI research tool like NotebookLM.

<br>

<img src="docs/screenshot.png" width="900" alt="Bandnote open in capture mode: the three-band layout with the accent-colored Subject indicator in the top bar showing the auto-routed class, the editor filling the center with a live note, and the status bar reading 'Autosaved 14:03:05 · Chemistry' with a clickable 'class changed → switch' prompt.">

</div>

---

Built for the everyday case of sitting down in class, typing what you hear, and
having it end up in the right place without a single decision. **Not** built for
long-form manuscripts or as a second brain you groom by hand.

## Download & install

Grab the latest build for your OS from the
[**Releases**](https://github.com/markwu123454/BandNote/releases) page:

| Platform    | File                             | What it is                                                      |
|-------------|----------------------------------|-----------------------------------------------------------------|
| **Windows** | `Bandnote_x.y.z_x64-setup.exe`   | NSIS installer — **recommended**                                |
| **Windows** | `Bandnote_x.y.z_x64_en-US.msi`   | MSI installer — for managed / enterprise deployment             |
| **macOS**   | `Bandnote_x.y.z_universal.dmg`   | Disk image, universal (Intel + Apple Silicon) — **recommended** |
| **macOS**   | `Bandnote_universal.app.tar.gz`  | `.app` bundle tarball (also used by the auto-updater)           |
| **Linux**   | `Bandnote_x.y.z_amd64.AppImage`  | Portable — runs on most distros, no install — **recommended**   |
| **Linux**   | `Bandnote_x.y.z_amd64.deb`       | Debian / Ubuntu package                                         |
| **Linux**   | `Bandnote-x.y.z-1.x86_64.rpm`    | Fedora / RHEL / openSUSE package                                |

The `.sig` files and `latest.json` alongside them are for the built-in
auto-updater, you don't need to download those.

The app isn't code-signed on any platform, so each OS shows a first-run warning:

- **Windows** — SmartScreen may warn; click **More info → Run anyway**. Needs the
  WebView2 runtime, which ships with current Windows.
- **macOS** — Gatekeeper blocks unsigned apps: **right-click the app → Open** the
  first time (or run `xattr -dr com.apple.quarantine "/Applications/Bandnote.app"`).
- **Linux** — for the AppImage, `chmod +x Bandnote*.AppImage` then run it; install
  the `.deb` with `sudo apt install ./Bandnote*.deb` or the `.rpm` with
  `sudo dnf install ./Bandnote*.rpm`.

## How it works

Bandnote has three phases: set up your schedule once, capture constantly, export
occasionally.

- **Setup (once):** open **⚙ Settings** → add your subjects, then add weekly
  schedule blocks (subject · day · start–end). Pick an export folder and theme.
- **Capture (constant):** the cursor lands in the editor the moment the app opens.
  Just type. The top-bar **Subject** indicator turns accent-colored when
  auto-routing is active, so you can see at a glance where the current note is
  going. Off-schedule text falls to **Inbox** so typing is never blocked. Autosave
  is continuous (350 ms debounce, atomic writes). At a class boundary the app won't
  yank your text out from under you — it shows a clickable *"class changed →
  switch"* prompt in the status bar instead.
- **Export (occasional):** **Ctrl/⌘-E** (or the command palette, **Ctrl/⌘-K**) →
  pick one subject or all → preview → export. Produces date-ordered markdown with
  clean `#` / `##` headings that parse well in NotebookLM. Raw notes are never
  altered or deleted.

## Features

- **Schedule-aware auto-routing** — every keystroke is filed under the class that's
  in session right now, decided from your weekly schedule and the current time. No
  picking a folder, no naming a note, no thinking about it.
- **Zero-friction capture** — the editor is focused on launch and stays the center
  of the app; you can go from opening Bandnote to typing your first line in under a
  second, with nothing between you and the text.
- **Silent, continuous autosave** — a 350 ms debounce with atomic writes means your
  notes are always on disk without a save step, and a partial write can never
  corrupt a session file.
- **Non-interrupting class boundaries** — when one class ends and the next begins,
  Bandnote never moves your cursor or reroutes mid-thought; it surfaces a clickable
  *switch* prompt in the status bar and leaves the choice to you.
- **Inbox fallback** — type outside any scheduled block and the text lands in
  **Inbox** rather than being blocked or lost, so capture always works even at
  midnight or on a day off.
- **Per-subject markdown export** — export one subject or all of them at once into
  date-ordered markdown with clean `#` / `##` headings, ready to drop into
  NotebookLM or any AI research tool.
- **Non-destructive by design** — export re-assembles copies; it never edits,
  merges over, or deletes your raw session files, so the source of truth on disk is
  always intact.
- **Command palette (`Ctrl/⌘-K`)** — a single fuzzy launcher for every action:
  export, settings, switch subject, search sessions, and more, without hunting
  through menus.
- **Live subject indicator** — the top bar always shows which subject the current
  note is routing to, accent-colored when a schedule block is active and neutral
  when you're in Inbox.
- **Session search** — a quick search scans your session previews so you can find
  where you wrote something without leaving the capture view.
- **Plain-markdown storage** — one markdown file per capture session, each with a
  tiny frontmatter block, readable in any editor and trivially backed up via
  Dropbox, git, or whatever you already use.
- **OS-following dark mode** — light and dark themes track the system setting, and
  you can override the choice in Settings.
- **Sub-second start** — a Tauri shell over the system webview means a few-MB binary
  that launches almost instantly, so opening Bandnote is never a reason to not
  capture.

## Scope

Optimized for constant, in-the-moment capture across a weekly class schedule, with
manual per-subject markdown export as the stable core. There is no cloud sync, no
collaborative editing, and no rich-media handling — those omissions are deliberate
and are what keep capture instant and the storage format something you fully own.

## Keyboard shortcuts

| Action           | Shortcut     |
|------------------|--------------|
| Command palette  | `Ctrl/⌘-K`   |
| Export           | `Ctrl/⌘-E`   |
| Settings         | `Ctrl/⌘-,`   |
| Close / refocus  | `Esc`        |

`Esc` closes any open dialog or search and drops the cursor back into the editor,
so you're never more than one key from typing again.

## Storage layout

Notes are plain markdown on disk, one file per capture session, under the OS
app-data directory:

```
<app-data>/Bandnote/
  config.json                        # subjects, schedule, export dir, theme
  notes/
    <subject_id>/
      2026-07-10_14-03-05.md         # one markdown file per session
```

Each session file carries a tiny frontmatter block (`subject`, `subject_id`,
`started`) followed by the raw note body, so it stays readable in any editor and
backup-able via your own Dropbox or git. On Windows the app-data dir is
`%APPDATA%\com.bandnote.app\` (shown in **Settings → Output**).

## Stack

| Layer   | Choice                                                    |
|---------|-----------------------------------------------------------|
| Shell   | [Tauri 2](https://tauri.app) (Rust backend, OS webview)   |
| UI      | [Svelte](https://svelte.dev) + [Vite](https://vite.dev)   |
| Storage | Plain markdown on disk, one file per session              |

The result is a few-MB binary that launches almost instantly and stores your notes
in a format you can read and back up without the app.

## Build from source

Prerequisites: [Node.js](https://nodejs.org) 18+, the
[Rust toolchain](https://rustup.rs), and the platform dependencies from the
[Tauri prerequisites](https://tauri.app/start/prerequisites/). On Windows that
means the Visual Studio C++ Build Tools and the WebView2 runtime (bundled with
current Windows).

```bash
git clone https://github.com/markwu123454/BandNote
cd BandNote
npm install            # install frontend dependencies

npm run tauri dev      # launch the native window with hot reload
npm run tauri build    # produce an optimized installer + binary
```

`npm run tauri build` writes the binary and installers under
`src-tauri/target/release/` (the executable) and
`src-tauri/target/release/bundle/` (the `.msi`, `.dmg`, `.AppImage`, and friends).

### Where things live

```
src/                   Frontend (Svelte)
  App.svelte           Three-band capture UI + orchestration
  lib/
    api.js             Wrappers over the Rust commands
    routing.js         Schedule → active-subject logic + time helpers
  components/
    Settings.svelte    Subjects, schedule blocks, export folder, theme
    ExportDialog.svelte  Subject picker + preview + export
src-tauri/             Backend (Rust / Tauri)
  src/lib.rs           Commands: config + session storage, export rendering
  tauri.conf.json      Window, bundle, and app-data config
  capabilities/        Tauri permission grants for the main window
```

## Not yet built

Deferred per the descriptor's MVP cut: a global summon hotkey, `.ics` schedule
import, A/B-week patterns, image paste, a full-text search index (current search
scans session previews), and a best-effort `notebooklm-py` push. Manual markdown
export is the stable core and stays the fallback.

## License

[GNU Affero General Public License v3.0 or later](LICENSE) © Mark Wu.