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
  import { EditorState } from "@codemirror/state";
  import { history, historyKeymap, defaultKeymap, undo, redo } from "@codemirror/commands";

  export let value = "";
  export let placeholder = "Start typing…";
  export let onChange = () => {};

  let host;
  let view;

  const theme = EditorView.theme({
    "&": {
      height: "100%",
      backgroundColor: "transparent",
      color: "var(--text-strong)",
    },
    "&.cm-focused": { outline: "none" },
    ".cm-scroller": {
      fontFamily: "var(--ed-font, var(--font-editor))",
      fontSize: "var(--ed-size, 18px)",
      lineHeight: "var(--ed-lh, 1.7)",
      overflow: "auto",
      padding: "28px 40px 40px",
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

  function makeState(doc) {
    return EditorState.create({
      doc,
      extensions: [
        history(),
        EditorView.lineWrapping,
        cmPlaceholder(placeholder),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        EditorView.contentAttributes.of({ spellcheck: "true", "aria-label": "Note editor" }),
        theme,
        EditorView.updateListener.of((u) => {
          if (u.docChanged) onChange(u.state.doc.toString());
        }),
      ],
    });
  }

  onMount(() => {
    view = new EditorView({ state: makeState(value), parent: host });
    view.focus();
  });
  onDestroy(() => view?.destroy());

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
</script>

<div class="cm-host" bind:this={host}></div>

<style>
  .cm-host {
    height: 100%;
    overflow: hidden;
  }
</style>
