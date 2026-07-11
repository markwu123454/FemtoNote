// Writing-surface preferences. Kept in one place so the capture window (which
// applies them) and the Setup window (which edits them) can't drift apart.
// Keys are camelCase to match the Rust EditorPrefs serde contract.

export const FONT_OPTIONS = [
  { id: "serif", label: "Serif", stack: "var(--font-editor)" },
  { id: "sans", label: "Sans", stack: "var(--font-ui)" },
  { id: "mono", label: "Mono", stack: "var(--font-mono)" },
];

export const EDITOR_DEFAULTS = {
  font: "serif",
  fontSize: 18,
  lineHeight: 1.7,
  maxWidth: 780,
  align: "center", // "center" | "left"
  spellcheck: true, // underline misspellings (our checker, not the OS webview)
  autocorrect: "conservative", // "off" | "conservative" | "balanced"
  autocorrectHint: true, // transient blue underline on auto-corrected words
};

// How long the auto-correct underline lingers before fading, and how many
// further words you can type before it clears — whichever comes first. Kept out
// of the settings UI to avoid clutter; tuned here.
export const HINT_LINGER_MS = 15000;
export const HINT_LINGER_WORDS = 8;

export const AUTOCORRECT_OPTIONS = [
  { id: "off", label: "Off" },
  { id: "conservative", label: "Careful" },
  { id: "balanced", label: "Balanced" },
];

/** Fill any missing fields so the UI never reads `undefined` before load. */
export function withEditorDefaults(editor) {
  return { ...EDITOR_DEFAULTS, ...(editor || {}) };
}

/** Resolve a font id to its CSS font-family stack. */
export function fontStack(id) {
  return (FONT_OPTIONS.find((f) => f.id === id) || FONT_OPTIONS[0]).stack;
}
