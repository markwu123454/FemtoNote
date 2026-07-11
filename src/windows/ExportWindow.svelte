<script>
  // Full-window Export surface. Its own OS window (label "export"), opened from
  // the main window's menu bar. Reads config, renders a per-subject preview, and
  // writes clean markdown. On success it toasts the main window and closes.
  import { onMount, onDestroy } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import { loadConfig, previewSubject, exportSubject, exportAll } from "../lib/api.js";
  import { openSettings, toast, closeSelf, CONFIG_CHANGED } from "../lib/appwindows.js";

  let config = { subjects: [], schedule: [], export_dir: null, theme: "system" };
  let ready = false;
  let target = "all"; // "all" or a subject id
  let preview = "";
  let loadingPreview = false;
  let busy = false;
  let error = "";
  let unlisten;

  function applyTheme(theme) {
    const root = document.documentElement;
    if (theme === "light" || theme === "dark") root.dataset.theme = theme;
    else delete root.dataset.theme;
  }

  onMount(async () => {
    try {
      config = await loadConfig();
    } catch (e) {
      console.error("export: load config failed", e);
    }
    applyTheme(config.theme);
    target = config.subjects[0] ? config.subjects[0].id : "all";
    ready = true;

    unlisten = await listen(CONFIG_CHANGED, (e) => {
      config = e.payload;
      applyTheme(config.theme);
    });
  });

  onDestroy(() => unlisten?.());

  async function refreshPreview() {
    error = "";
    if (target === "all") {
      preview = "";
      return;
    }
    loadingPreview = true;
    try {
      const name = config.subjects.find((s) => s.id === target)?.name || target;
      preview = await previewSubject(target, name);
    } catch (e) {
      error = String(e);
    } finally {
      loadingPreview = false;
    }
  }

  $: if (ready) {
    target;
    refreshPreview();
  }

  async function resolveDir() {
    if (config.export_dir) return config.export_dir;
    const dir = await open({ directory: true, multiple: false, title: "Export to…" });
    return typeof dir === "string" ? dir : null;
  }

  async function doExport() {
    error = "";
    busy = true;
    try {
      const dir = await resolveDir();
      if (!dir) {
        busy = false;
        return;
      }
      if (target === "all") {
        const written = await exportAll(config.subjects, dir);
        if (written.length === 0) {
          error = "Nothing to export yet — no notes on disk.";
        } else {
          await toast(`Exported ${written.length} subject${written.length === 1 ? "" : "s"} → ${dir}`);
          await closeSelf();
        }
      } else {
        const name = config.subjects.find((s) => s.id === target)?.name || target;
        const path = await exportSubject(target, name, dir);
        await toast(`Exported → ${path}`);
        await closeSelf();
      }
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  $: previewEmpty = !loadingPreview && target !== "all" && preview.trim().split("\n").length <= 1;
</script>

{#if ready}
  <div class="win">
    <header class="titlebar" data-tauri-drag-region>
      <span class="label" data-tauri-drag-region>Export to markdown</span>
      <button class="x" on:click={closeSelf} title="Close (Esc)">✕</button>
    </header>

    <div class="controls">
      <select bind:value={target}>
        <option value="all">All subjects</option>
        {#each config.subjects as s}
          <option value={s.id}>{s.name}</option>
        {/each}
      </select>
      {#if config.export_dir}
        <span class="mono dest" title={config.export_dir}>→ {config.export_dir}</span>
      {:else}
        <button class="linkish" on:click={openSettings}>set a default folder</button>
      {/if}
    </div>

    <div class="preview">
      {#if target === "all"}
        <p class="note">One clean <span class="mono">.md</span> file per subject, date-ordered — ready for NotebookLM.</p>
      {:else if loadingPreview}
        <p class="note">Building preview…</p>
      {:else if previewEmpty}
        <p class="note empty">No notes captured for this subject yet.</p>
      {:else}
        <pre class="mono">{preview}</pre>
      {/if}
    </div>

    {#if error}
      <p class="error mono">⚠ {error}</p>
    {/if}

    <footer>
      <button class="ghost" on:click={closeSelf}>Cancel</button>
      <button class="accent" on:click={doExport} disabled={busy}>
        {busy ? "Exporting…" : target === "all" ? "Export all" : "Export"}
      </button>
    </footer>
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
    padding: 4px 6px;
    border-radius: 5px;
  }
  .x:hover {
    color: var(--text-strong);
    background: var(--bg-sunken);
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
  }
  select {
    background: var(--bg);
    color: var(--text-strong);
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    padding: 6px 8px;
    font-family: inherit;
    font-size: var(--fs-chrome);
  }
  .dest {
    color: var(--text-muted);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .linkish {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    text-decoration: underline;
    padding: 0;
  }
  .preview {
    flex: 1;
    overflow: auto;
    padding: 14px;
    min-height: 140px;
  }
  .preview pre {
    margin: 0;
    font-size: 12px;
    line-height: 1.55;
    color: var(--text);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .note {
    color: var(--text-muted);
    margin: 0;
  }
  .note.empty {
    font-style: italic;
    color: var(--text-faint);
  }
  .error {
    color: var(--err);
    padding: 0 14px;
    font-size: 12px;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 14px;
    border-top: 1px solid var(--border);
  }
  button.accent {
    background: var(--accent);
    color: var(--accent-contrast);
    border: none;
    border-radius: 6px;
    padding: 7px 14px;
    cursor: pointer;
  }
  button.accent:disabled {
    opacity: 0.6;
    cursor: default;
  }
  button.ghost {
    background: none;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 7px 14px;
    cursor: pointer;
    color: var(--text-muted);
  }
  button.ghost:hover {
    color: var(--text-strong);
    border-color: var(--border-strong);
  }
</style>
