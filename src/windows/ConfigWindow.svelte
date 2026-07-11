<script>
  // Full-window config surface, shared by two OS windows:
  //   section="setup"    (label "setup")    -> Subjects + Schedule
  //   section="settings" (label "settings") -> Editor + Output
  // Every change persists to disk and is broadcast so the main capture window
  // (and the other config window, if open) re-syncs live.
  import { onMount, onDestroy } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import { loadConfig, saveConfig, dataDir } from "../lib/api.js";
  import { broadcastConfig, closeSelf, CONFIG_CHANGED } from "../lib/appwindows.js";
  import { DAYS, makeId, INBOX_ID } from "../lib/routing.js";
  import { FONT_OPTIONS, AUTOCORRECT_OPTIONS, withEditorDefaults } from "../lib/editor.js";

  export let section = "setup"; // "setup" | "settings"

  const SECTIONS = {
    setup: { title: "Setup", tabs: ["subjects", "schedule"] },
    settings: { title: "Settings", tabs: ["editor", "output"] },
  };
  const TAB_LABELS = { subjects: "Subjects", schedule: "Schedule", editor: "Editor", output: "Output" };

  let config = { subjects: [{ id: INBOX_ID, name: "Inbox" }], schedule: [], export_dir: null, theme: "system" };
  let dataPath = "";
  let ready = false;
  let newSubject = "";
  let unlisten;

  $: meta = SECTIONS[section] || SECTIONS.setup;
  let tab = null;
  $: if (tab === null || !meta.tabs.includes(tab)) tab = meta.tabs[0];

  function applyTheme(theme) {
    const root = document.documentElement;
    if (theme === "light" || theme === "dark") root.dataset.theme = theme;
    else delete root.dataset.theme;
  }

  onMount(async () => {
    try {
      config = await loadConfig();
    } catch (e) {
      console.error("config: load failed", e);
    }
    applyTheme(config.theme);
    dataPath = await dataDir().catch(() => "");
    ready = true;

    // Live-sync if the config changes elsewhere (e.g. theme toggled on main).
    unlisten = await listen(CONFIG_CHANGED, (e) => {
      if (JSON.stringify(e.payload) !== JSON.stringify(config)) {
        config = e.payload;
        applyTheme(config.theme);
      }
    });
  });

  onDestroy(() => unlisten?.());

  async function commit(patch) {
    config = { ...config, ...patch };
    applyTheme(config.theme);
    try {
      await saveConfig(config);
      await broadcastConfig(config);
    } catch (e) {
      console.error("config: save failed", e);
    }
  }

  // --- Subjects -----------------------------------------------------------
  function addSubject() {
    const name = newSubject.trim();
    if (!name) return;
    commit({ subjects: [...config.subjects, { id: makeId("subj"), name }] });
    newSubject = "";
  }
  function renameSubject(id, name) {
    commit({ subjects: config.subjects.map((s) => (s.id === id ? { ...s, name } : s)) });
  }
  function deleteSubject(id) {
    if (id === INBOX_ID) return;
    commit({
      subjects: config.subjects.filter((s) => s.id !== id),
      schedule: config.schedule.filter((b) => b.subjectId !== id),
    });
  }

  // --- Schedule -----------------------------------------------------------
  function addBlock() {
    const first = config.subjects.find((s) => s.id !== INBOX_ID) || config.subjects[0];
    const block = {
      id: makeId("blk"),
      subjectId: first ? first.id : INBOX_ID,
      day: 1,
      start: "09:00",
      end: "10:00",
    };
    commit({ schedule: [...config.schedule, block] });
  }
  function updateBlock(id, patch) {
    commit({ schedule: config.schedule.map((b) => (b.id === id ? { ...b, ...patch } : b)) });
  }
  function deleteBlock(id) {
    commit({ schedule: config.schedule.filter((b) => b.id !== id) });
  }
  $: sortedSchedule = [...config.schedule].sort(
    (a, b) => a.day - b.day || a.start.localeCompare(b.start)
  );

  // --- Editor -------------------------------------------------------------
  $: ed = withEditorDefaults(config.editor);
  function setEditor(patch) {
    // Read the current config (not the derived `ed`, which lags a render) so
    // back-to-back edits compose instead of clobbering each other.
    commit({ editor: { ...withEditorDefaults(config.editor), ...patch } });
  }

  // --- Output -------------------------------------------------------------
  async function chooseExportDir() {
    const dir = await open({ directory: true, multiple: false, title: "Choose export folder" });
    if (typeof dir === "string") commit({ export_dir: dir });
  }
</script>

{#if ready}
  <div class="win">
    <header class="titlebar" data-tauri-drag-region>
      <span class="label" data-tauri-drag-region>{meta.title}</span>
      <button class="x" on:click={closeSelf} title="Close (Esc)">✕</button>
    </header>

    <nav class="tabs">
      {#each meta.tabs as t}
        <button class:active={tab === t} on:click={() => (tab = t)}>{TAB_LABELS[t]}</button>
      {/each}
    </nav>

    <div class="body">
      {#if tab === "subjects"}
        <p class="hint">Subjects your notes are filed under. <span class="mono">Inbox</span> catches anything typed off-schedule.</p>
        <ul class="rows">
          {#each config.subjects as s (s.id)}
            <li class="row">
              <input
                class="grow"
                value={s.name}
                on:input={(e) => renameSubject(s.id, e.target.value)}
                disabled={s.id === INBOX_ID}
              />
              <span class="mono id">{s.id}</span>
              {#if s.id !== INBOX_ID}
                <button class="ghost danger" on:click={() => deleteSubject(s.id)}>delete</button>
              {:else}
                <span class="mono tag">default</span>
              {/if}
            </li>
          {/each}
        </ul>
        <div class="row add">
          <input
            class="grow"
            placeholder="Add a subject…"
            bind:value={newSubject}
            on:keydown={(e) => e.key === "Enter" && addSubject()}
          />
          <button class="accent" on:click={addSubject}>add</button>
        </div>

      {:else if tab === "schedule"}
        <p class="hint">Weekly class times. Whatever block is active <em>now</em> is where notes go automatically.</p>
        {#if config.schedule.length === 0}
          <p class="empty">No blocks yet — notes will file to <span class="mono">Inbox</span> until you add some.</p>
        {/if}
        <ul class="rows">
          {#each sortedSchedule as b (b.id)}
            <li class="row block">
              <select value={b.subjectId} on:change={(e) => updateBlock(b.id, { subjectId: e.target.value })}>
                {#each config.subjects as s}
                  <option value={s.id}>{s.name}</option>
                {/each}
              </select>
              <select value={b.day} on:change={(e) => updateBlock(b.id, { day: Number(e.target.value) })}>
                {#each DAYS as d, i}
                  <option value={i}>{d}</option>
                {/each}
              </select>
              <input class="time mono" type="time" value={b.start} on:input={(e) => updateBlock(b.id, { start: e.target.value })} />
              <span class="dash">–</span>
              <input class="time mono" type="time" value={b.end} on:input={(e) => updateBlock(b.id, { end: e.target.value })} />
              <button class="ghost danger" on:click={() => deleteBlock(b.id)}>✕</button>
            </li>
          {/each}
        </ul>
        <button class="accent" on:click={addBlock}>add block</button>

      {:else if tab === "editor"}
        <p class="hint">The writing surface. Changes apply to the capture window instantly.</p>

        <div class="field">
          <span class="label">Font</span>
          <div class="seg">
            {#each FONT_OPTIONS as f}
              <button class:on={ed.font === f.id} on:click={() => setEditor({ font: f.id })}>{f.label}</button>
            {/each}
          </div>
        </div>

        <div class="field">
          <span class="label">Font size <span class="mono val">{ed.fontSize}px</span></span>
          <input
            type="range"
            min="13"
            max="30"
            step="1"
            value={ed.fontSize}
            on:input={(e) => setEditor({ fontSize: Number(e.target.value) })}
          />
        </div>

        <div class="field">
          <span class="label">Line height <span class="mono val">{ed.lineHeight.toFixed(2)}</span></span>
          <input
            type="range"
            min="1.2"
            max="2.4"
            step="0.05"
            value={ed.lineHeight}
            on:input={(e) => setEditor({ lineHeight: Number(e.target.value) })}
          />
        </div>

        <div class="field">
          <span class="label">Column width <span class="mono val">{ed.maxWidth === 0 ? "Max" : ed.maxWidth + "px"}</span></span>
          <input
            type="range"
            min="480"
            max="1220"
            step="20"
            value={ed.maxWidth === 0 ? 1220 : ed.maxWidth}
            on:input={(e) => setEditor({ maxWidth: Number(e.target.value) >= 1220 ? 0 : Number(e.target.value) })}
          />
          <p class="hint tiny">Drag fully right for <span class="mono">Max</span> — the editor fills the window.</p>
        </div>

        <div class="field">
          <span class="label">Alignment</span>
          <div class="seg">
            <button class:on={ed.align === "center"} on:click={() => setEditor({ align: "center" })}>Center</button>
            <button class:on={ed.align === "left"} on:click={() => setEditor({ align: "left" })}>Left</button>
          </div>
        </div>

        <div class="field">
          <span class="label">Spell check</span>
          <div class="seg">
            <button class:on={ed.spellcheck} on:click={() => setEditor({ spellcheck: true })}>On</button>
            <button class:on={!ed.spellcheck} on:click={() => setEditor({ spellcheck: false })}>Off</button>
          </div>
          <p class="hint tiny">A built-in dictionary underlines misspellings and learns words you save — independent of the OS. Right-click a word for suggestions.</p>
        </div>

        {#if ed.spellcheck}
          <div class="field">
            <span class="label">Auto-correct</span>
            <div class="seg">
              {#each AUTOCORRECT_OPTIONS as o}
                <button class:on={ed.autocorrect === o.id} on:click={() => setEditor({ autocorrect: o.id })}>{o.label}</button>
              {/each}
            </div>
            <p class="hint tiny">Fixes only clear, single-candidate typos as you type; anything ambiguous stays a right-click suggestion. <span class="mono">Careful</span> is strictest. Undo (Ctrl Z) reverts any fix.</p>
          </div>

          {#if ed.autocorrect !== "off"}
            <div class="field">
              <span class="label">Auto-correct marker</span>
              <div class="seg">
                <button class:on={ed.autocorrectHint} on:click={() => setEditor({ autocorrectHint: true })}>Show</button>
                <button class:on={!ed.autocorrectHint} on:click={() => setEditor({ autocorrectHint: false })}>Hide</button>
              </div>
              <p class="hint tiny">A brief blue underline on corrected words so a change never slips by. It fades after a few seconds.</p>
            </div>
          {/if}
        {/if}

      {:else}
        <p class="hint">Where exports land and how the app looks.</p>

        <div class="field">
          <span class="label">Export folder</span>
          <div class="row">
            <span class="mono path grow" class:muted={!config.export_dir}>
              {config.export_dir || "— not set —"}
            </span>
            <button class="accent" on:click={chooseExportDir}>choose…</button>
          </div>
        </div>

        <div class="field">
          <span class="label">Theme</span>
          <div class="seg">
            {#each ["system", "light", "dark"] as t}
              <button class:on={config.theme === t} on:click={() => commit({ theme: t })}>{t}</button>
            {/each}
          </div>
        </div>

        <div class="field">
          <span class="label">Notes stored at</span>
          <span class="mono path muted">{dataPath || "…"}</span>
          <p class="hint tiny">Plain markdown — back it up with your own Dropbox/git.</p>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .win {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg);
    user-select: text;
  }
  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  .x {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 13px;
    padding: 4px 6px;
    border-radius: 5px;
  }
  .x:hover {
    color: var(--text-strong);
    background: var(--bg-sunken);
  }
  .tabs {
    display: flex;
    gap: 2px;
    padding: 8px 12px 0;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-muted);
    padding: 6px 10px 8px;
    cursor: pointer;
    font-size: var(--fs-chrome);
  }
  .tabs button.active {
    color: var(--text-strong);
    border-bottom-color: var(--accent);
  }
  .body {
    flex: 1;
    padding: 14px;
    overflow-y: auto;
  }
  .hint {
    margin: 0 0 12px;
    color: var(--text-muted);
    line-height: 1.5;
  }
  .hint.tiny {
    font-size: 11px;
    margin-top: 4px;
  }
  .empty {
    color: var(--text-faint);
    font-style: italic;
    margin: 0 0 10px;
  }
  .rows {
    list-style: none;
    margin: 0 0 12px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .row.add {
    margin-top: 4px;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  input,
  select {
    background: var(--bg);
    color: var(--text-strong);
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    padding: 6px 8px;
    font-family: inherit;
    font-size: var(--fs-chrome);
  }
  input:focus,
  select:focus {
    outline: none;
    border-color: var(--accent);
  }
  input:disabled {
    color: var(--text-muted);
    background: var(--bg-sunken);
  }
  input[type="range"] {
    width: 280px;
    max-width: 100%;
    padding: 0;
    border: none;
    background: transparent;
    accent-color: var(--accent);
    cursor: pointer;
  }
  .val {
    text-transform: none;
    letter-spacing: 0;
    color: var(--text-muted);
    margin-left: 6px;
  }
  .time {
    width: 110px;
  }
  .dash {
    color: var(--text-faint);
  }
  .id,
  .tag {
    color: var(--text-faint);
    font-size: 11px;
  }
  .tag {
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: 5px;
  }
  button.accent {
    background: var(--accent);
    color: var(--accent-contrast);
    border: none;
    border-radius: 6px;
    padding: 6px 12px;
    cursor: pointer;
  }
  button.accent:hover {
    filter: brightness(1.06);
  }
  button.ghost {
    background: none;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 5px 9px;
    cursor: pointer;
    color: var(--text-muted);
  }
  button.ghost:hover {
    border-color: var(--border-strong);
    color: var(--text-strong);
  }
  button.danger:hover {
    color: var(--err);
    border-color: var(--err);
  }
  .field {
    margin-bottom: 18px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .path {
    padding: 6px 8px;
    background: var(--bg-sunken);
    border-radius: 6px;
    overflow-wrap: anywhere;
  }
  .muted {
    color: var(--text-muted);
  }
  .seg {
    display: inline-flex;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    overflow: hidden;
    width: fit-content;
  }
  .seg button {
    background: var(--bg);
    border: none;
    padding: 6px 14px;
    cursor: pointer;
    color: var(--text-muted);
    border-right: 1px solid var(--border-strong);
    text-transform: capitalize;
  }
  .seg button:last-child {
    border-right: none;
  }
  .seg button.on {
    background: var(--accent-tint);
    color: var(--accent);
  }
</style>
