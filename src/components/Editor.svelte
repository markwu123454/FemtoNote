<script>
  // Minimal CodeMirror 6 prose editor — a drop-in replacement for the capture
  // textarea. Deliberately NOT a code/markdown editor: no line numbers, no
  // gutters, no syntax highlighting, no bracket matching. What it buys us over a
  // raw textarea is consistent, chunked undo/redo across webviews, plus a clean
  // base for future opt-in QoL features.
  //
  // Typography (font / size / line-height / column width / alignment) is driven
  // by CSS custom properties set by the parent on an ancestor element, so prefs
  // changes apply live without reconfiguring the editor.
  import { onMount, onDestroy } from "svelte";
  import { EditorView, keymap, placeholder as cmPlaceholder } from "@codemirror/view";
  import { EditorState, EditorSelection } from "@codemirror/state";
  import { history, historyKeymap, defaultKeymap, undo, redo } from "@codemirror/commands";
  import { createSpeller, spellcheck, requestRecheck } from "../lib/spellcheck.js";
  import { HINT_LINGER_MS, HINT_LINGER_WORDS } from "../lib/editor.js";

  // --- Outline / hierarchy via tabbing ------------------------------------
  // One tab = one level. Tab indents, Shift-Tab outdents, and Enter carries the
  // current line's leading whitespace to the next line — so once you tab in,
  // every following line stays at that level until you Shift-Tab back out. A
  // real "\t" is used (not spaces) so a single Backspace removes exactly one
  // level, matching the mental model of "delete a tab."
  const INDENT_UNIT = "\t";

  /** Line numbers touched by any selection range (1-based, deduped). */
  function selectedLineNumbers(state) {
    const lines = new Set();
    for (const range of state.selection.ranges) {
      const first = state.doc.lineAt(range.from).number;
      const last = state.doc.lineAt(range.to).number;
      for (let n = first; n <= last; n++) lines.add(n);
    }
    return lines;
  }

  /** Tab: add one indent level to every selected line. */
  function indentLines(view) {
    const { state } = view;
    const changes = [];
    for (const n of selectedLineNumbers(state)) {
      changes.push({ from: state.doc.line(n).from, insert: INDENT_UNIT });
    }
    // No explicit selection: CodeMirror maps the caret through the inserts.
    view.dispatch(state.update({ changes, scrollIntoView: true, userEvent: "input.indent" }));
    return true;
  }

  /** Shift-Tab: remove one indent level (a leading tab, or up to tabSize spaces). */
  function outdentLines(view) {
    const { state } = view;
    const changes = [];
    for (const n of selectedLineNumbers(state)) {
      const line = state.doc.line(n);
      let remove = 0;
      if (line.text.startsWith("\t")) remove = 1;
      else while (remove < state.tabSize && line.text[remove] === " ") remove++;
      if (remove > 0) changes.push({ from: line.from, to: line.from + remove });
    }
    // Always consume Tab (return true) so focus never leaves the editor.
    if (changes.length) {
      view.dispatch(state.update({ changes, scrollIntoView: true, userEvent: "delete.dedent" }));
    }
    return true;
  }

  /** Enter: newline that copies the current line's leading whitespace. */
  function newlineKeepIndent(view) {
    const { state } = view;
    const tr = state.changeByRange((range) => {
      const line = state.doc.lineAt(range.from);
      const indent = /^[\t ]*/.exec(line.text)[0];
      const insert = state.lineBreak + indent;
      return {
        changes: { from: range.from, to: range.to, insert },
        range: EditorSelection.cursor(range.from + insert.length),
      };
    });
    view.dispatch(state.update(tr, { scrollIntoView: true, userEvent: "input" }));
    return true;
  }

  export let value = "";
  export let placeholder = "Start typing…";
  export let onChange = () => {};
  // Spell-check config (subset of editor prefs), the learned-word list, and a
  // callback to persist a newly-saved word. All optional so the editor works
  // standalone.
  export let spell = {};
  export let customWords = [];
  export let onAddWord = () => {};
  // An externally-owned speller (see spellcheck.js's createSpeller) shared by
  // a parent that mounts many editors at once (e.g. scroll-up session
  // history) — avoids spinning up one dictionary-loading worker per block. If
  // omitted, this editor creates and owns its own.
  export let speller = null;
  // "flow": auto-height, no internal scroll clipping — for an editor embedded
  // inline in a larger scrolling page (past sessions) rather than filling a
  // fixed-height pane (the live capture surface).
  export let flow = false;
  // Past-session blocks mount while the user is elsewhere; stealing focus on
  // mount would yank the cursor out of the live capture buffer.
  export let focusOnMount = true;

  let host;
  let view;

  // --- Spell checking -----------------------------------------------------
  // `spellSettings` is mutated in place so the extension reads live values
  // without rebuilding the editor. `ownSpeller` is only created when no
  // externally-owned `speller` prop was passed in, and only this editor may
  // destroy it.
  let ownSpeller = null;
  let seeded = new Set();
  const spellSettings = {
    enabled: true,
    autocorrect: "conservative",
    hint: true,
    indicatorMs: HINT_LINGER_MS,
    indicatorWords: HINT_LINGER_WORDS,
  };
  const spellCtx = {
    getSpeller: () => speller || ownSpeller,
    settings: spellSettings,
    onAddWord: (w) => onAddWord(w),
  };

  function ensureSpeller() {
    if (speller) return speller;
    if (!ownSpeller) {
      ownSpeller = createSpeller();
      seeded = new Set();
    }
    syncCustomWords(customWords);
    return ownSpeller;
  }

  function syncCustomWords(list) {
    const sp = speller || ownSpeller;
    if (!sp) return;
    let added = false;
    for (const w of list || []) {
      if (w && !seeded.has(w)) {
        seeded.add(w);
        sp.add(w);
        added = true;
      }
    }
    if (added) requestRecheck(view);
  }

  function applySpell(s) {
    spellSettings.enabled = s?.spellcheck !== false;
    spellSettings.autocorrect = s?.autocorrect || "conservative";
    spellSettings.hint = s?.autocorrectHint !== false;
    if (spellSettings.enabled) ensureSpeller();
    requestRecheck(view);
  }

  // React to live pref / learned-word / shared-speller changes.
  $: applySpell(spell);
  $: (speller, customWords, syncCustomWords(customWords));

  function buildTheme(isFlow) {
    return EditorView.theme({
      "&": {
        height: isFlow ? "auto" : "100%",
        backgroundColor: "transparent",
        color: "var(--text-strong)",
      },
      "&.cm-focused": { outline: "none" },
      ".cm-scroller": {
        fontFamily: "var(--ed-font, var(--font-editor))",
        fontSize: "var(--ed-size, 18px)",
        lineHeight: "var(--ed-lh, 1.7)",
        overflow: isFlow ? "visible" : "auto",
        padding: isFlow ? "4px 40px 20px" : "28px 40px 40px",
      },
      ".cm-content": {
        maxWidth: "var(--ed-max, 780px)",
        marginInline: "var(--ed-mar, auto)",
        caretColor: "var(--accent)",
        padding: "0",
      },
      ".cm-line": { padding: "0" },
      ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent)" },
      ".cm-placeholder": { color: "var(--text-faint)" },
      "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
        backgroundColor: "var(--accent-tint)",
      },
    });
  }

  function makeState(doc) {
    return EditorState.create({
      doc,
      extensions: [
        history(),
        EditorView.lineWrapping,
        cmPlaceholder(placeholder),
        // Outline keys first so they take precedence over defaultKeymap's Enter.
        keymap.of([
          { key: "Tab", run: indentLines, shift: outdentLines },
          { key: "Enter", run: newlineKeepIndent },
        ]),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        // Our own checker owns spelling now, so the OS/webview one stays off to
        // avoid double underlines.
        EditorView.contentAttributes.of({ spellcheck: "false", "aria-label": "Note editor" }),
        ...spellcheck(spellCtx),
        buildTheme(flow),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) onChange(u.state.doc.toString());
        }),
      ],
    });
  }

  onMount(() => {
    view = new EditorView({ state: makeState(value), parent: host });
    if (spellSettings.enabled) ensureSpeller();
    requestRecheck(view);
    if (focusOnMount) view.focus();
  });
  onDestroy(() => {
    view?.destroy();
    ownSpeller?.destroy(); // never destroy a speller we don't own
  });

  // --- Imperative API for the parent (via bind:this) ----------------------
  export function focus() {
    view?.focus();
  }
  /** Start a fresh document. Replacing the whole state also clears undo history
   *  so Ctrl+Z can't resurrect a previous session's notes. */
  export function reset(doc = "") {
    if (!view) {
      value = doc;
      return;
    }
    view.setState(makeState(doc));
    view.focus();
  }
  export function doUndo() {
    if (view) undo(view);
  }
  export function doRedo() {
    if (view) redo(view);
  }
  /** Move the caret to the very end of the document and focus — used when the
   *  user clicks in blank space below the last line (see App.svelte's
   *  live-slot click handler), so the "infinite page" feel doesn't require an
   *  actual click hit on existing text. */
  export function focusEnd() {
    if (!view) return;
    const end = view.state.doc.length;
    view.dispatch({ selection: EditorSelection.cursor(end), scrollIntoView: true });
    view.focus();
  }
</script>

<div class="cm-host" class:flow bind:this={host}></div>

<style>
  .cm-host {
    height: 100%;
    overflow: hidden;
  }
  .cm-host.flow {
    height: auto;
    overflow: visible;
  }
</style>
