<script>
  import { onMount, onDestroy, tick } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import MenuBar from "./components/MenuBar.svelte";
  import Editor from "./components/Editor.svelte";
  import PastSession from "./components/PastSession.svelte";
  import { loadConfig, saveConfig, writeSession, listSessions, listSubjectSessions, dataDir } from "./lib/api.js";
  import { createSpeller } from "./lib/spellcheck.js";
  import {
    openSetup as openSetupWindow,
    openSettings as openSettingsWindow,
    openExport as openExportWindow,
    broadcastConfig,
    quitApp,
    CONFIG_CHANGED,
    TOAST,
  } from "./lib/appwindows.js";
  import {
    scheduledSubject,
    subjectName,
    clock,
    sessionId,
    displayStamp,
    INBOX_ID,
  } from "./lib/routing.js";
  import { fontStack, withEditorDefaults } from "./lib/editor.js";

  // --- State --------------------------------------------------------------
  let config = { subjects: [{ id: INBOX_ID, name: "Inbox" }], schedule: [], export_dir: null, theme: "system" };
  let ready = false;
  let dataPath = "";

  let now = new Date();
  let editorText = "";
  let saveState = "idle"; // idle | saving | saved | error
  let saveError = "";

  let routingMode = "auto"; // auto | manual
  let session = null; // { subjectId, sessionId, startedAt }
  let pendingRoute = null; // subjectId the schedule wants to switch to

  let sessions = []; // metadata for search
  let showPalette = false;
  let searchOpen = false;
  let searchQuery = "";
  let toasts = [];
  let toastSeq = 0;
  let subjectMenuOpen = false;

  let editorComp; // CodeMirror wrapper instance
  let saveTimer;
  let clockTimer;
  let unlistenConfig;
  let unlistenToast;

  // One speller worker shared by the live editor and every past-session block,
  // so scrolling into a semester of history doesn't spin up a dictionary-
  // loading worker per note.
  const speller = createSpeller();

  // --- Scroll-up session history -------------------------------------------
  // Past sessions for the current subject, oldest-first, loaded lazily as the
  // user scrolls toward the top of the capture window. The live buffer always
  // starts empty; history only becomes visible by scrolling up into it.
  let scrollerEl;
  let pastSessions = []; // [{ sessionId, startedAt, content }], oldest-first
  let historyLoading = false;
  let historyDone = false;
  // Tracked in px (not CSS %) because the scroll container's height isn't a
  // definite value flexbox can resolve percentages against — see live-slot.
  let liveMinHeight = 280;
  let scrollerResize;
  let liveSlotEl;

  // --- Derived ------------------------------------------------------------
  $: activeName = session ? subjectName(config, session.subjectId) : "";
  $: ed = withEditorDefaults(config.editor);
  $: sched = scheduledSubject(config, now);
  $: routingActive = routingMode === "manual" || sched.scheduled;
  $: wordCount = editorText.trim() ? editorText.trim().split(/\s+/).length : 0;
  $: charCount = editorText.length;
  $: filteredSessions = filterSessions(sessions, searchQuery);

  function filterSessions(list, q) {
    const s = q.trim().toLowerCase();
    if (!s) return list.slice(0, 40);
    return list
      .filter(
        (x) =>
          x.subject_name.toLowerCase().includes(s) ||
          x.preview.toLowerCase().includes(s) ||
          (x.started_at || "").toLowerCase().includes(s)
      )
      .slice(0, 40);
  }

  // --- Menu model ---------------------------------------------------------
  $: menus = [
    {
      label: "File",
      items: [
        { label: "New session", action: newSession, disabled: !session },
        { sep: true },
        { label: "Export…", hint: "Ctrl E", action: openExport },
        { sep: true },
        { label: "Subjects & Schedule…", hint: "Ctrl ,", action: openSetup },
        { label: "Preferences…", hint: "Ctrl .", action: openSettings },
        { sep: true },
        { label: "Exit", action: quitApp },
      ],
    },
    {
      label: "Edit",
      items: [
        { label: "Undo", hint: "Ctrl Z", action: () => editCmd("undo") },
        { label: "Redo", hint: "Ctrl Y", action: () => editCmd("redo") },
        { sep: true },
        { label: "Cut", hint: "Ctrl X", action: () => editCmd("cut") },
        { label: "Copy", hint: "Ctrl C", action: () => editCmd("copy") },
        { label: "Paste", hint: "Ctrl V", action: () => editCmd("paste") },
        { label: "Select all", hint: "Ctrl A", action: () => editCmd("selectAll") },
        { sep: true },
        { label: "Find notes…", hint: "Ctrl F", action: openSearch },
      ],
    },
    {
      label: "View",
      items: [
        { label: "Command palette…", hint: "Ctrl K", action: () => (showPalette = true) },
        { sep: true },
        {
          label: "Theme",
          submenu: ["system", "light", "dark"].map((t) => ({
            label: t[0].toUpperCase() + t.slice(1),
            checked: config.theme === t,
            action: () => commitConfig({ ...config, theme: t }),
          })),
        },
      ],
    },
    {
      label: "Help",
      items: [
        { label: "About Bandnote", action: () => pushToast("Bandnote · capture-first notes", "ok") },
      ],
    },
  ];

  // --- Lifecycle ----------------------------------------------------------
  onMount(async () => {
    try {
      config = await loadConfig();
    } catch (e) {
      pushToast("Couldn't load settings: " + e, "err");
    }
    applyTheme(config.theme);
    dataPath = await dataDir().catch(() => "");
    startSession(scheduledSubject(config, new Date()).subjectId, { silent: true, primeHist: false });
    await refreshSessions();
    ready = true;
    await tick();
    focusEditor();
    if (scrollerEl) {
      // Keep the live buffer's floor pinned to the container's actual visible
      // height (in px — percentages don't resolve here, see .live-slot) so a
      // short bit of history can never peek out before you scroll for it.
      liveMinHeight = scrollerEl.clientHeight;
      scrollerResize = new ResizeObserver(() => {
        if (scrollerEl) liveMinHeight = scrollerEl.clientHeight;
      });
      scrollerResize.observe(scrollerEl);
    }
    primeHistory(); // scrollerEl exists now; safe to preload + pin to bottom

    clockTimer = setInterval(onTick, 1000);
    window.addEventListener("keydown", onKey);

    // Stay in sync with the Setup / Export windows.
    unlistenConfig = await listen(CONFIG_CHANGED, (e) => {
      config = e.payload;
      applyTheme(config.theme);
    });
    unlistenToast = await listen(TOAST, (e) => {
      refreshSessions();
      pushToast(e.payload?.text || "", e.payload?.kind || "ok");
    });
  });

  onDestroy(() => {
    clearInterval(clockTimer);
    clearTimeout(saveTimer);
    window.removeEventListener("keydown", onKey);
    unlistenConfig?.();
    unlistenToast?.();
    speller?.destroy();
    scrollerResize?.disconnect();
  });

  // --- Theme --------------------------------------------------------------
  function applyTheme(theme) {
    const root = document.documentElement;
    if (theme === "light" || theme === "dark") root.dataset.theme = theme;
    else delete root.dataset.theme;
  }

  // --- Routing tick -------------------------------------------------------
  function onTick() {
    now = new Date();
    if (routingMode !== "auto" || !session) return;
    const target = scheduledSubject(config, now).subjectId;
    if (target === session.subjectId) {
      pendingRoute = null;
      return;
    }
    // Don't yank text mid-thought: switch silently only when the surface is
    // empty, otherwise surface a clickable prompt in the status bar.
    if (!editorText.trim()) {
      startSession(target, { silent: true });
    } else {
      pendingRoute = target;
    }
  }

  // --- Session management -------------------------------------------------
  function startSession(subjectId, { silent, primeHist = true } = {}) {
    if (session) {
      // Flush whatever's pending for the outgoing session before we move on —
      // otherwise up to 350ms of typing could be missing from its history.
      clearTimeout(saveTimer);
      persist();
    }
    const d = new Date();
    session = {
      subjectId,
      sessionId: sessionId(d),
      startedAt: displayStamp(d),
    };
    editorText = "";
    editorComp?.reset(""); // fresh buffer + cleared undo history for the new session
    pendingRoute = null;
    saveState = "idle";
    resetHistory();
    if (primeHist) primeHistory();
    if (!silent) {
      pushToast(`Now filing under ${subjectName(config, subjectId)}`, "ok");
      focusEditor();
    }
  }

  // --- Scroll-up session history -------------------------------------------
  function resetHistory() {
    pastSessions = [];
    historyDone = false;
    historyLoading = false;
  }

  // Preload the first page right after a session starts, then pin the scroll
  // to the bottom so the capture view still *looks* empty — history is there
  // the instant you scroll up, with no loading flash.
  async function primeHistory() {
    const subj = session?.subjectId;
    const sid = session?.sessionId;
    await loadMoreHistory();
    if (session?.subjectId !== subj || session?.sessionId !== sid) return; // switched again mid-flight
    await tick();
    if (scrollerEl) scrollerEl.scrollTop = scrollerEl.scrollHeight;
  }

  async function loadMoreHistory() {
    if (!session || historyLoading || historyDone) return;
    const subjectId = session.subjectId;
    const cursor = pastSessions.length ? pastSessions[0].sessionId : session.sessionId;
    historyLoading = true;
    try {
      const page = await listSubjectSessions(subjectId, cursor, 8);
      if (session?.subjectId !== subjectId) return; // subject changed while this was in flight
      if (!page.length) {
        historyDone = true;
        return;
      }
      const ordered = [...page].reverse(); // newest-first -> oldest-first for prepending
      const prevHeight = scrollerEl?.scrollHeight || 0;
      const prevTop = scrollerEl?.scrollTop || 0;
      pastSessions = [...ordered, ...pastSessions];
      await tick();
      if (scrollerEl) scrollerEl.scrollTop = prevTop + (scrollerEl.scrollHeight - prevHeight);
    } catch (e) {
      pushToast("Couldn't load past notes: " + e, "err");
    } finally {
      historyLoading = false;
    }
  }

  function onHistoryScroll() {
    if (scrollerEl && scrollerEl.scrollTop < 48) loadMoreHistory();
  }

  async function savePastSession(meta, content) {
    try {
      await writeSession({
        subject_id: meta.subjectId,
        subject_name: subjectName(config, meta.subjectId),
        session_id: meta.sessionId,
        started_at: meta.startedAt,
        content,
      });
    } catch (e) {
      pushToast("Couldn't save edit: " + e, "err");
    }
  }

  function newSession() {
    if (session) startSession(session.subjectId, {});
  }

  function acceptPendingRoute() {
    if (pendingRoute) startSession(pendingRoute, {});
  }

  function toggleSubjectMenu() {
    subjectMenuOpen = !subjectMenuOpen;
  }

  function chooseSubject(id) {
    subjectMenuOpen = false;
    if (id === "__auto__") {
      routingMode = "auto";
      const target = scheduledSubject(config, new Date()).subjectId;
      if (!editorText.trim()) startSession(target, { silent: true });
      else pendingRoute = target === session.subjectId ? null : target;
      return;
    }
    routingMode = "manual";
    if (id !== session.subjectId) startSession(id, { silent: !!editorText.trim() ? false : true });
    focusEditor();
  }

  // --- Autosave -----------------------------------------------------------
  // Called by the editor on every doc change (CodeMirror owns the buffer now).
  function handleChange(text) {
    editorText = text;
    if (!session) return;
    saveState = "saving";
    clearTimeout(saveTimer);
    saveTimer = setTimeout(persist, 350);
  }

  async function persist() {
    if (!session) return;
    try {
      await writeSession({
        subject_id: session.subjectId,
        subject_name: subjectName(config, session.subjectId),
        session_id: session.sessionId,
        started_at: session.startedAt,
        content: editorText,
      });
      saveState = "saved";
      saveError = "";
    } catch (e) {
      saveState = "error";
      saveError = String(e);
    }
  }

  // Editor commands routed through the Edit menu. Undo/redo use CodeMirror's
  // history; clipboard ops run on the focused editor (CodeMirror observes the
  // resulting DOM change and folds it into its own history + autosave).
  function editCmd(cmd) {
    if (cmd === "undo") return editorComp?.doUndo();
    if (cmd === "redo") return editorComp?.doRedo();
    editorComp?.focus();
    try {
      document.execCommand(cmd);
    } catch {
      /* some webviews block programmatic paste */
    }
  }

  // --- Config commits -----------------------------------------------------
  async function commitConfig(newConfig) {
    config = newConfig;
    applyTheme(config.theme);
    try {
      await saveConfig(config);
      await broadcastConfig(config);
    } catch (e) {
      pushToast("Couldn't save settings: " + e, "err");
    }
  }

  // Persist a word the user taught the spell checker ("Save spelling"). Stored
  // globally in config so it survives restarts and applies across subjects.
  function addWord(word) {
    const w = String(word || "").trim();
    if (!w) return;
    const cur = config.custom_words || [];
    if (cur.includes(w)) return;
    commitConfig({ ...config, custom_words: [...cur, w] });
  }

  async function refreshSessions() {
    try {
      sessions = await listSessions();
    } catch (e) {
      // Non-fatal; search just stays empty.
      sessions = [];
    }
  }

  // --- Toasts -------------------------------------------------------------
  function pushToast(text, kind = "ok") {
    const id = ++toastSeq;
    toasts = [...toasts, { id, text, kind }];
    setTimeout(() => (toasts = toasts.filter((t) => t.id !== id)), 3200);
  }

  // --- Windows / search ---------------------------------------------------
  function openSetup() {
    openSetupWindow();
  }

  function openSettings() {
    openSettingsWindow();
  }

  function openExport() {
    persist(); // make sure the current buffer is on disk before previewing
    refreshSessions();
    openExportWindow();
  }

  async function openSearch() {
    searchOpen = true;
    await refreshSessions();
    await tick();
    document.querySelector(".search")?.focus();
  }

  // --- Keyboard -----------------------------------------------------------
  function onKey(e) {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === "k") {
      e.preventDefault();
      showPalette = !showPalette;
      return;
    }
    if (mod && e.key.toLowerCase() === "e") {
      e.preventDefault();
      openExport();
      return;
    }
    if (mod && e.key.toLowerCase() === "f") {
      e.preventDefault();
      openSearch();
      return;
    }
    if (mod && e.key === ",") {
      e.preventDefault();
      openSetup();
      return;
    }
    if (mod && e.key === ".") {
      e.preventDefault();
      openSettings();
      return;
    }
    if (e.key === "Escape") {
      if (showPalette) showPalette = false;
      else if (subjectMenuOpen) subjectMenuOpen = false;
      else if (searchOpen) {
        searchOpen = false;
        searchQuery = "";
        focusEditor();
      }
    }
  }

  async function focusEditor() {
    await tick();
    editorComp?.focus();
  }

  // The live buffer is "flow"-sized (grows with its content, no inner
  // scrollbar) but the slot itself is stretched to fill the pane. Clicking in
  // the blank space below the last line — anywhere the click lands on the
  // slot itself rather than inside the CodeMirror editor — drops the caret at
  // the end of the document, so the page reads as one continuous surface
  // instead of a box you can scroll past.
  function onLiveSlotClick(e) {
    if (e.target === liveSlotEl) editorComp?.focusEnd();
  }

  // Command palette actions
  $: paletteActions = [
    { label: "Export to markdown", hint: "Ctrl E", run: openExport },
    { label: "Subjects & Schedule", hint: "Ctrl ,", run: openSetup },
    { label: "Preferences", hint: "Ctrl .", run: openSettings },
    { label: "Find notes", hint: "Ctrl F", run: openSearch },
    {
      label: `Theme: ${config.theme}`,
      hint: "cycle",
      run: () => commitConfig({ ...config, theme: nextTheme(config.theme) }),
    },
    { label: "New session", hint: "", run: newSession },
    { label: "Focus editor", hint: "Esc", run: focusEditor },
  ];
  function nextTheme(t) {
    return t === "system" ? "light" : t === "light" ? "dark" : "system";
  }
  function runPalette(a) {
    showPalette = false;
    a.run();
  }
</script>

{#if ready}
  <div class="shell">
    <!-- ===== MENU BAR ===== -->
    <MenuBar {menus} />

    <!-- ===== TOP BAR (subject trust anchor + inline search) ===== -->
    <header class="topbar">
      <div class="left">
        <span class="label subjlabel">Subject</span>
        <div class="subject-picker">
          <button
            class="subject mono"
            class:active={routingActive}
            on:click={toggleSubjectMenu}
          >
            {activeName}
            <span class="subject-caret" class:open={subjectMenuOpen}>›</span>
          </button>
          {#if subjectMenuOpen}
            <div class="subject-dropdown" role="menu">
              <button
                class="sd-item"
                role="menuitemradio"
                aria-checked={routingMode === "auto"}
                on:click={() => chooseSubject("__auto__")}
              >
                <span class="check">{routingMode === "auto" ? "✓" : ""}</span>
                <span class="sd-label">Auto — {subjectName(config, sched.subjectId)}</span>
              </button>
              <div class="sd-sep"></div>
              {#each config.subjects as s (s.id)}
                <button
                  class="sd-item"
                  role="menuitemradio"
                  aria-checked={routingMode === "manual" && session?.subjectId === s.id}
                  on:click={() => chooseSubject(s.id)}
                >
                  <span class="check">{routingMode === "manual" && session?.subjectId === s.id ? "✓" : ""}</span>
                  <span class="sd-label">{s.name}</span>
                </button>
              {/each}
            </div>
            <button class="scrim" tabindex="-1" aria-hidden="true" on:click={() => (subjectMenuOpen = false)}></button>
          {/if}
        </div>
        {#if routingMode === "manual"}
          <span class="mono modetag">manual</span>
        {/if}
      </div>

      <div class="right">
        {#if searchOpen}
          <!-- svelte-ignore a11y-autofocus -->
          <input
            class="search mono"
            placeholder="search notes…"
            bind:value={searchQuery}
            autofocus
          />
          <button class="icon" title="Close search (Esc)" on:click={() => { searchOpen = false; searchQuery = ""; focusEditor(); }}>✕</button>
        {/if}
      </div>

      {#if searchOpen}
        <div class="search-results">
          {#if filteredSessions.length === 0}
            <div class="sr-empty">no matches</div>
          {:else}
            {#each filteredSessions as r (r.subject_id + r.session_id)}
              <div class="sr-row">
                <span class="mono sr-subj">{r.subject_name}</span>
                <span class="mono sr-when">{r.started_at}</span>
                <span class="sr-prev">{r.preview}</span>
                <span class="mono sr-words">{r.words}w</span>
              </div>
            {/each}
          {/if}
        </div>
      {/if}
    </header>

    <!-- ===== EDITOR (hot path) ===== -->
    <!-- Scroll up to reveal past sessions for this subject (lazy-loaded,
         dividers between them); the live buffer at the bottom is the hot path
         and always starts empty. -->
    <main
      class="editor-wrap"
      bind:this={scrollerEl}
      on:scroll={onHistoryScroll}
      style="--ed-font: {fontStack(ed.font)}; --ed-size: {ed.fontSize}px; --ed-lh: {ed.lineHeight}; --ed-max: {ed.maxWidth === 0 ? 'none' : ed.maxWidth + 'px'}; --ed-mar: {ed.align === 'left' ? '0' : 'auto'}; --live-min: {liveMinHeight}px;"
    >
      <div class="history-col">
        {#each pastSessions as ps (ps.sessionId)}
          <PastSession
            meta={{ subjectId: session.subjectId, sessionId: ps.sessionId, startedAt: ps.startedAt }}
            content={ps.content}
            {speller}
            spell={ed}
            customWords={config.custom_words || []}
            onAddWord={addWord}
            onSave={savePastSession}
          />
        {/each}
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div class="live-slot" bind:this={liveSlotEl} on:click={onLiveSlotClick}>
          <Editor
            bind:this={editorComp}
            value={editorText}
            onChange={handleChange}
            spell={ed}
            customWords={config.custom_words || []}
            onAddWord={addWord}
            {speller}
            flow={true}
          />
        </div>
      </div>
    </main>

    <!-- ===== STATUS BAR ===== -->
    <footer class="statusbar mono">
      <span class="save" class:saving={saveState === "saving"} class:err={saveState === "error"}>
        <span class="dot" class:ok={saveState === "saved"} class:saving={saveState === "saving"} class:err={saveState === "error"}></span>
        {saveState === "saving" ? "saving" : saveState === "saved" ? "saved" : saveState === "error" ? "save failed" : "ready"}
      </span>

      <span class="sep">·</span>

      {#if pendingRoute}
        <button class="warn-btn" on:click={acceptPendingRoute}>
          ▲ class changed → switch to {subjectName(config, pendingRoute)}
        </button>
      {:else if !sched.scheduled && routingMode === "auto"}
        <button class="warn-btn amber" on:click={openSetup}>
          ▲ no class scheduled → {activeName}
        </button>
      {:else}
        <span class="routing" class:active={routingActive}>→ {activeName}</span>
      {/if}

      <span class="spacer"></span>

      {#if saveState === "error"}
        <span class="err" title={saveError}>{saveError.slice(0, 40)}</span>
        <span class="sep">·</span>
      {/if}
      <span class="clock">{clock(now)}</span>
      <span class="sep">·</span>
      <span class="words">{wordCount} words · {charCount} chars</span>
    </footer>
  </div>

  <!-- ===== TOASTS ===== -->
  <div class="toasts">
    {#each toasts as t (t.id)}
      <div class="toast" class:err={t.kind === "err"}>{t.text}</div>
    {/each}
  </div>

  <!-- ===== COMMAND PALETTE ===== -->
  {#if showPalette}
    <div class="palette-root">
      <button class="scrim" aria-label="Close command palette" on:click={() => (showPalette = false)}></button>
      <div class="palette" role="dialog" aria-modal="true" aria-label="Command palette">
        {#each paletteActions as a}
          <button class="pal-row" on:click={() => runPalette(a)}>
            <span>{a.label}</span>
            {#if a.hint}<span class="mono pal-hint">{a.hint}</span>{/if}
          </button>
        {/each}
      </div>
    </div>
  {/if}
{:else}
  <div class="boot">…</div>
{/if}

<style>
  .shell {
    display: grid;
    grid-template-rows: auto var(--band-top) 1fr var(--band-bottom);
    height: 100%;
  }

  /* ---- Top bar ---- */
  .topbar {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
  }
  .left {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .subjlabel {
    opacity: 0.8;
  }
  .subject {
    font-size: 13px;
    color: var(--text-muted);
    padding: 2px 8px;
    border-radius: 5px;
    border: 1px solid transparent;
    white-space: nowrap;
  }
  .subject.active {
    color: var(--accent);
    background: var(--accent-tint);
  }
  .subject-picker {
    position: relative;
  }
  button.subject {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    background: none;
    border: 1px solid transparent;
    cursor: default;
    font-family: var(--font-mono);
  }
  .subject-caret {
    display: inline-block;
    font-size: 12px;
    color: var(--text-faint);
    transition: transform 120ms ease-out;
  }
  .subject.active .subject-caret {
    color: var(--accent);
  }
  .subject-caret.open {
    transform: rotate(90deg);
  }
  .subject-dropdown {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    z-index: 3;
    min-width: 240px;
    padding: 10px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 16px;
    box-shadow: var(--shadow-dialog);
    animation: slide-up 110ms ease-out;
  }
  .sd-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    background: none;
    border: none;
    border-radius: 10px;
    padding: 9px 12px;
    cursor: default;
    color: var(--text);
    text-align: left;
    font-size: var(--fs-chrome);
    white-space: nowrap;
  }
  .sd-item:hover {
    background: var(--accent-tint);
    color: var(--text-strong);
  }
  .sd-label {
    flex: 1;
  }
  .sd-sep {
    height: 1px;
    margin: 8px 8px;
    background: var(--border);
  }
  .sd-item .check {
    width: 14px;
    flex: none;
    text-align: center;
    color: var(--accent);
    font-size: 12px;
  }
  .subject-picker .scrim {
    position: fixed;
    inset: 0;
    z-index: 1;
    background: transparent;
    border: none;
    margin: 0;
    padding: 0;
    cursor: default;
  }
  .modetag {
    font-size: 11px;
    color: var(--text-faint);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 1px 6px;
  }
  .right {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .icon {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 13px;
    width: 26px;
    height: 26px;
    border-radius: 5px;
  }
  .icon:hover {
    color: var(--text-strong);
    background: var(--bg-sunken);
  }
  .search {
    background: var(--bg-sunken);
    border: 1px solid var(--border-strong);
    border-radius: 5px;
    padding: 4px 8px;
    color: var(--text-strong);
    font-size: 12px;
    width: 200px;
  }
  .search:focus {
    outline: none;
    border-color: var(--accent);
  }
  .search-results {
    position: absolute;
    top: var(--band-top);
    right: 12px;
    width: min(460px, 80vw);
    max-height: 320px;
    overflow-y: auto;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-top: none;
    border-radius: 0 0 var(--radius) var(--radius);
    box-shadow: var(--shadow-dialog);
    z-index: 30;
  }
  .sr-empty {
    padding: 12px;
    color: var(--text-faint);
    font-style: italic;
  }
  .sr-row {
    display: grid;
    grid-template-columns: auto auto 1fr auto;
    gap: 8px;
    align-items: baseline;
    padding: 7px 10px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
  }
  .sr-row:last-child {
    border-bottom: none;
  }
  .sr-subj {
    color: var(--accent);
  }
  .sr-when {
    color: var(--text-faint);
    font-size: 11px;
  }
  .sr-prev {
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-ui);
  }
  .sr-words {
    color: var(--text-faint);
    font-size: 11px;
  }

  /* ---- Editor ---- */
  /* The writing surface is CodeMirror (see Editor.svelte). Typography/width/
     alignment are passed down as the --ed-* custom properties on this wrap.
     It's also the scroll container for session history: scrolling up reveals
     past sessions stacked above the live buffer. */
  .editor-wrap {
    overflow-y: auto;
    background: var(--bg);
    user-select: text;
  }
  .history-col {
    min-height: 100%;
    display: flex;
    flex-direction: column;
  }
  .live-slot {
    /* Always at least the container's full visible height (tracked in px via
       --live-min, see App.svelte's ResizeObserver — percentages can't resolve
       against a shrink-to-fit ancestor), so any history sits entirely above
       the fold: a fresh session looks exactly as empty as before, no matter
       how short the loaded history happens to be. */
    flex: 1 0 auto;
    min-height: var(--live-min, 280px);
    cursor: text;
  }

  /* ---- Status bar ---- */
  .statusbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    border-top: 1px solid var(--border);
    background: var(--bg);
    font-size: 11px;
    color: var(--text-muted);
  }
  .dot {
    display: inline-block;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-faint);
    margin-right: 5px;
    vertical-align: middle;
  }
  .dot.ok {
    background: var(--ok);
  }
  .dot.saving {
    background: var(--warn);
    animation: shimmer 900ms ease-in-out infinite;
  }
  .dot.err {
    background: var(--err);
  }
  .save.err {
    color: var(--err);
  }
  .routing {
    color: var(--text-muted);
  }
  .routing.active {
    color: var(--accent);
  }
  .sep {
    color: var(--text-faint);
  }
  .spacer {
    flex: 1;
  }
  .err {
    color: var(--err);
  }
  .warn-btn {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 2px 4px;
    border-radius: 4px;
  }
  .warn-btn:hover {
    background: var(--accent-tint);
  }
  .warn-btn.amber {
    color: var(--warn);
  }
  .warn-btn.amber:hover {
    background: rgba(154, 103, 0, 0.12);
  }

  /* ---- Toasts ---- */
  .toasts {
    position: fixed;
    bottom: calc(var(--band-bottom) + 12px);
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: center;
    z-index: 60;
    pointer-events: none;
  }
  .toast {
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-left: 3px solid var(--ok);
    border-radius: 6px;
    padding: 7px 12px;
    font-size: 12px;
    color: var(--text-strong);
    box-shadow: var(--shadow-dialog);
    animation: slide-up 160ms ease-out;
    max-width: 80vw;
  }
  .toast.err {
    border-left-color: var(--err);
  }

  /* ---- Command palette ---- */
  .palette-root {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
  }
  .scrim {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.28);
    border: none;
    margin: 0;
    padding: 0;
    cursor: default;
  }
  .palette {
    position: relative;
    z-index: 1;
    width: min(460px, 88vw);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow-dialog);
    overflow: hidden;
    animation: slide-up 120ms ease-out;
  }
  .pal-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border);
    padding: 10px 14px;
    cursor: pointer;
    color: var(--text-strong);
    text-align: left;
    font-size: 13px;
  }
  .pal-row:last-child {
    border-bottom: none;
  }
  .pal-row:hover {
    background: var(--accent-tint);
  }
  .pal-hint {
    color: var(--text-faint);
    font-size: 11px;
  }

  .boot {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--text-faint);
  }
</style>
