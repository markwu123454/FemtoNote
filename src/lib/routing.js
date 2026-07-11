// Schedule -> active subject. All the time math lives here so the UI stays dumb.

export const INBOX_ID = "inbox";
export const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/** "HH:MM" -> minutes since midnight, or null if malformed. */
export function parseHM(hm) {
  const m = /^(\d{1,2}):(\d{2})$/.exec((hm || "").trim());
  if (!m) return null;
  const h = Number(m[1]);
  const min = Number(m[2]);
  if (h > 23 || min > 59) return null;
  return h * 60 + min;
}

function pad(n) {
  return String(n).padStart(2, "0");
}

/** Local "HH:MM" for a Date. */
export function clock(date) {
  return `${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

/** Filesystem-safe, time-sortable session id: "YYYY-MM-DD_HH-MM-SS". */
export function sessionId(date) {
  return (
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}` +
    `_${pad(date.getHours())}-${pad(date.getMinutes())}-${pad(date.getSeconds())}`
  );
}

/** Human/NotebookLM-friendly stamp: "Fri 2026-07-10 · 14:03". */
export function displayStamp(date) {
  return (
    `${DAYS[date.getDay()]} ${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}` +
    ` · ${pad(date.getHours())}:${pad(date.getMinutes())}`
  );
}

/**
 * Which subject is scheduled right now.
 * Returns { subjectId, scheduled, block }. When nothing matches, routes to the
 * Inbox and scheduled=false so the UI can warn without ever blocking capture.
 */
export function scheduledSubject(config, date) {
  const day = date.getDay();
  const mins = date.getHours() * 60 + date.getMinutes();

  for (const b of config.schedule || []) {
    if (b.day !== day) continue;
    const start = parseHM(b.start);
    const end = parseHM(b.end);
    if (start == null || end == null) continue;
    if (mins >= start && mins < end) {
      // Only route to a subject that still exists.
      if ((config.subjects || []).some((s) => s.id === b.subjectId)) {
        return { subjectId: b.subjectId, scheduled: true, block: b };
      }
    }
  }
  return { subjectId: INBOX_ID, scheduled: false, block: null };
}

export function subjectName(config, id) {
  const s = (config.subjects || []).find((x) => x.id === id);
  return s ? s.name : id;
}

/** Cheap unique-ish id for new subjects/blocks (no crypto needed here). */
export function makeId(prefix) {
  return `${prefix}_${Math.random().toString(36).slice(2, 8)}`;
}
