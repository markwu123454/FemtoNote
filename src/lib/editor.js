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
};

/** Fill any missing fields so the UI never reads `undefined` before load. */
export function withEditorDefaults(editor) {
  return { ...EDITOR_DEFAULTS, ...(editor || {}) };
}

/** Resolve a font id to its CSS font-family stack. */
export function fontStack(id) {
  return (FONT_OPTIONS.find((f) => f.id === id) || FONT_OPTIONS[0]).stack;
}
