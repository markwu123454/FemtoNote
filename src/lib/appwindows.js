// Multi-window plumbing. The app is a single Vite bundle; each window loads the
// same index.html and picks its root component from the URL hash (see main.js).
//
//   (no hash)   -> main capture window
//   #setup      -> Setup / config window
//   #export     -> Export window
//
// Windows are created on demand from the menu bar and reused if already open.
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow, getAllWindows } from "@tauri-apps/api/window";
import { emit } from "@tauri-apps/api/event";

// Event names broadcast across windows.
export const CONFIG_CHANGED = "bandnote:config-changed";
export const TOAST = "bandnote:toast";

async function openOrFocus(label, options) {
  const existing = await WebviewWindow.getByLabel(label);
  if (existing) {
    try {
      await existing.show();
      await existing.unminimize();
    } catch {
      /* already visible */
    }
    await existing.setFocus();
    return existing;
  }
  const w = new WebviewWindow(label, options);
  w.once("tauri://error", (e) => console.error(`window "${label}" failed to open`, e));
  return w;
}

export function openSetup() {
  return openOrFocus("setup", {
    url: "index.html#setup",
    title: "Setup — Bandnote",
    width: 640,
    height: 580,
    minWidth: 460,
    minHeight: 380,
    resizable: true,
    center: true,
    decorations: false,
  });
}

export function openSettings() {
  return openOrFocus("settings", {
    url: "index.html#settings",
    title: "Settings — Bandnote",
    width: 560,
    height: 560,
    minWidth: 440,
    minHeight: 380,
    resizable: true,
    center: true,
    decorations: false,
  });
}

export function openExport() {
  return openOrFocus("export", {
    url: "index.html#export",
    title: "Export — Bandnote",
    width: 660,
    height: 560,
    minWidth: 460,
    minHeight: 360,
    resizable: true,
    center: true,
    decorations: false,
  });
}

/** Tell every other window the config changed so they can re-sync live. */
export function broadcastConfig(config) {
  return emit(CONFIG_CHANGED, config);
}

/** Ask the main window to surface a transient toast. */
export function toast(text, kind = "ok") {
  return emit(TOAST, { text, kind });
}

/** Close the current window. */
export function closeSelf() {
  return getCurrentWindow().close();
}

/** Quit the whole app by closing every window. */
export async function quitApp() {
  const all = await getAllWindows();
  for (const w of all) {
    try {
      await w.close();
    } catch {
      /* ignore */
    }
  }
}
