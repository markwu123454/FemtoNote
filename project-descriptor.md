# Project Descriptor

## Working name
*(TBD — placeholder: **Bandnote** / **Capture**)*

---

## One-line summary
A schedule-aware, low-friction note-capture desktop app: open it and type, it silently files each note under the class you're currently in, and later exports clean per-subject markdown you can feed into an AI research tool (NotebookLM) for studying.

---

## Problem
Existing note tools (Notion, etc.) push structure and formatting into the *capture* moment. During a lecture, that friction (naming notes, choosing folders, formatting text, picking a notebook) competes with the actual job of getting words down fast. The result is time wasted organizing instead of listening.

The goal is to eliminate all decision-making at capture time and defer it to two places where it doesn't hurt: one-time **setup** and occasional, deliberate **export**.

---

## Core principle
**Capture is sacred.** Push all friction out of the capture moment.

- **Setup (once, heavy):** enter schedule, subjects, export config. Done for the semester.
- **Capture (constant, near-zero):** just type. Routing, autosave, and timestamping happen silently.
- **Export (occasional, deliberate):** generate clean per-subject markdown for studying. This is where any "make it nice" work happens, and it's fine for it to take a moment.

Design tension to guard against: nearly every optional feature is a candidate to creep into the capture view and reintroduce friction. When in doubt about where a feature goes, it goes anywhere *but* the hot path.

---

## How it works (user flow)
1. **Once:** user enters their class schedule (subjects + weekly time blocks) and picks where exports go.
2. **Every class:** user opens the app (or hits a global hotkey); the cursor is already in the editor. They type. The app knows from the schedule which subject is active and files the notes there automatically. Autosave is continuous and silent.
3. **When studying:** user exports one subject (or all) to clean, date-structured markdown and imports it into NotebookLM to generate study guides, summaries, and to ask grounded questions about their own notes.

---

## Tech stack
- **Framework:** Tauri (Rust backend + system webview) for sub-second startup and native filesystem access.
- **Frontend:** Svelte (compiles to minimal JS, no virtual DOM, snappy for a lightweight capture UI).
- **Storage:** Local-first. Plain markdown files on disk (optionally a small local DB for indexing/metadata). No cloud dependency. Notes remain readable in any editor and backup-able via the user's own Dropbox/git.
- **Rationale:** fast cold start, native file writes for per-subject export, full privacy for college notes, and a nice-looking UI with no Electron bloat.

---

## NotebookLM integration
- **Primary path (reliable):** clean per-subject markdown export, imported manually. Fully in the user's control, never breaks.
- **No free consumer API** currently exists. Google's official NotebookLM API is enterprise-only (Google Cloud + IAM). A consumer API has been acknowledged as "in the works" but isn't available.
- **Optional convenience (best-effort):** a "push to NotebookLM" button could wrap the community `notebooklm-py` tool (cookie-based browser-session auth). This is unofficial and can break when Google changes their web internals, so it must never be load-bearing. Manual export stays the fallback.
- **Design implication:** treat clean markdown export as the stable core. It keeps the app ready for a future official API without betting on it.

---

## Design language

**One line:** neutral chrome, a single accent, monospace data, dual-themed tokens — so the content is always the brightest thing on screen.

- **Chrome recedes, content dominates.** Thin utilitarian bands (top bar, working area, status bar). No hero sections, big buttons, or cards in the main flow.
- **Neutral-first color.** Near-white / near-black grays do the structural work. Regions separated by `1px` hairline borders, not shadows or fills.
- **One accent color, used only for meaning** — primary actions, active states, links. Never decorative.
- **Semantic colors are scarce** — green/amber/red for status only, always paired with a word or glyph (colorblind-safe). The most saturated colors are reserved for actual content.
- **Two fonts split by role:** system sans for UI labels; monospace for anything representing data (paths, counts, timestamps, subject IDs).
- **Dense and small chrome:** ~13px base, 11px uppercase letter-spaced section labels. Information-rich, technical.
- **Editor is the exception to density:** the writing surface uses a larger, comfortable font with generous line-height and quiet horizontal margins, because notes are typed for long stretches.
- **Dual theme by default:** light/dark via `prefers-color-scheme`, driven by CSS variable tokens (`--bg`, `--bg-elevated`, `--border`, three text weights, one `--accent`), **plus a manual override** (lecture halls are dim regardless of OS setting).
- **Polish through restraint:** thin floating scrollbars, soft-shadowed rounded dialogs, transient toasts instead of modals, subtle motion (2px shimmer, 6px slide-up).
- **Feedback is ambient, not blocking.** Warn, don't wall — errors are clickable shortcuts, not dead ends. A live status bar over popups.

---

## Main page layout

Three bands: two thin monospace chrome bands top and bottom, a dominant comfortable editor between them.

**Top bar (~36px, dense chrome):**
- Left: **current subject indicator** (monospace, accent-tinted when routing is active) — the trust anchor for auto-routing — plus a small **override dropdown** for one-tap manual subject switching.
- Right: quiet **search** (icon that expands inline) and **settings gear** (opens schedule/export config). Neutral, no accent.
- Separated from the editor by a single 1px hairline.

**Working area (dominant, content):**
- Borderless full-height editor, comfortable writing font, roomy line-height, wide quiet margins.
- No toolbar, no formatting buttons, no panels. Cursor lands here on open.
- **Session separation** shown subtly inside the log: a thin hairline rule with a small mono timestamp where a new class session begins.

**Status bar (~28px, ambient feedback):**
- Mono data, left to right: **save-state dot**, **routing target** (accent-tinted), **current time**, **word count**.
- Home for all non-blocking feedback and **clickable warnings** (e.g., "no class scheduled → pick subject"), plus transient export toasts (6px slide-up).

**Deliberately absent from the main page:** notebook/folder sidebar, tag panel, formatting toolbar, dominant export button. Secondary actions live behind the settings gear, search, and a **command palette (Ctrl/Cmd-K)**. A **global hotkey** summons the window pre-focused in the editor.

---

## Feature list

### Setup phase
**Essential**
- Schedule entry: subjects + weekly time blocks (day, start, end) — powers auto-routing.
- Subject management: add / rename / delete.
- Export config: output location and markdown structure (sane defaults).

**Strong**
- Recurring patterns incl. alternating A/B weeks.
- Manual subject override.
- Flexible time granularity (non-hour boundaries).

**Nice**
- Import schedule from `.ics` / calendar file.
- Semester/term boundaries (auto-retire old schedules).
- Multiple schedule profiles per term.

### Capture phase
**Essential**
- Instant open to a ready editor, zero clicks.
- Auto-routing to the currently scheduled subject.
- Continuous silent autosave (no data loss on crash/close).
- Silent timestamping of entries.
- Current-subject indicator (trust the routing).

**Strong**
- One-tap manual subject switch from the capture view.
- Session separation (new entries append to the subject log; clear session boundaries).
- Off-schedule capture: default "inbox" subject so typing is never blocked.
- Quick undo / recent session history.
- Keyboard-first, incl. global summon hotkey.

**Nice**
- Optional markdown-as-you-type (never enforced).
- Paste/inline images (lecture slide snapshots).
- Quick "★ important" / manual time-marker hotkey.
- Subtle word/character count.
- Distraction-free / full-screen mode.
- Optional voice-to-text input.

### Export phase
**Essential**
- Per-subject export to clean, date-ordered markdown (one file per subject).
- Export all subjects or one at a time.
- Consistent headings/date stamps that parse well in NotebookLM.

**Strong**
- Date-range export ("this week," "since last exam").
- Export preview before writing.
- Format choice: markdown primary, `.txt` / `.json` alternates.
- Non-destructive (never alters or deletes raw notes).

**Nice**
- One-click "push to NotebookLM" (best-effort `notebooklm-py` wrapper).
- Term bundling (zip all subjects).
- Configurable file naming (e.g., `subject_2026-week3.md`).
- Export history/log.

### Cross-cutting
**Essential**
- Local-first storage (plain files on disk).
- Data durability across crashes, force-quits, updates.
- Sub-second cold start.

**Strong**
- Local full-text search across all notes.
- Plain-file access for user-owned backup/sync (Dropbox/git).
- Edit past notes outside capture mode.
- Settings persistence with sane defaults.

**Nice**
- Theming / dark mode (arguably Strong — dim lecture halls).
- Cross-device sync (user's own cloud folder is the cheap version).
- Stats (notes per subject, active days, streaks).
- Tagging/linking (deliberately low priority — keep out of the hot path).
- Encryption at rest.
- First-run onboarding for schedule setup.

---

## MVP cut
Smallest thing that delivers the full value proposition:
1. Schedule setup.
2. Instant-open, auto-routed capture.
3. Silent autosave with timestamps.
4. Local markdown storage.
5. Per-subject export.

Build that, use it for a week of real classes, and let actual friction dictate what to add next — rather than building the full list up front.

---

## Target environment
- Personal/student use, single user.
- Developer's hardware (reference): laptop, RTX 4090 16GB VRAM, 48GB RAM — comfortably capable of local AI tooling too, should the project later add a fully-offline local-RAG alternative to NotebookLM.
