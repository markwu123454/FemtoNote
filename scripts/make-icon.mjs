// Generate a 512x512 source PNG for the app icon (no external deps).
// A dark rounded square with an accent-blue capture bar + cursor — a nod to the
// three-band capture UI. `tauri icon` derives every platform size from this.
import { writeFileSync, mkdirSync } from "node:fs";
import { deflateSync } from "node:zlib";

const S = 512;
const buf = Buffer.alloc(S * S * 4);

function px(x, y, r, g, b, a = 255) {
  if (x < 0 || y < 0 || x >= S || y >= S) return;
  const i = (y * S + x) * 4;
  buf[i] = r;
  buf[i + 1] = g;
  buf[i + 2] = b;
  buf[i + 3] = a;
}

const bg = [23, 23, 27];        // near-black chrome
const accent = [91, 130, 246];  // one accent
const faint = [70, 70, 78];

const radius = 96;
function inRounded(x, y, pad) {
  const lo = pad, hi = S - pad;
  if (x < lo || x > hi || y < lo || y > hi) return false;
  const r = radius - pad / 2;
  const cx = Math.min(Math.max(x, lo + r), hi - r);
  const cy = Math.min(Math.max(y, lo + r), hi - r);
  const dx = x - cx, dy = y - cy;
  return dx * dx + dy * dy <= r * r;
}

for (let y = 0; y < S; y++) {
  for (let x = 0; x < S; x++) {
    if (!inRounded(x, y, 24)) {
      px(x, y, 0, 0, 0, 0); // transparent outside
      continue;
    }
    px(x, y, ...bg);
  }
}

// Three stacked "note" bars; the top one is the active (accent) capture line.
const bars = [
  { y: 176, w: 240, c: accent },
  { y: 256, w: 200, c: faint },
  { y: 320, w: 160, c: faint },
];
const x0 = 140;
const h = 30;
for (const bar of bars) {
  for (let y = bar.y; y < bar.y + h; y++) {
    for (let x = x0; x < x0 + bar.w; x++) {
      if (inRounded(x, y, 24)) px(x, y, ...bar.c);
    }
  }
}
// Blinking cursor block at the end of the active line.
for (let y = 168; y < 168 + 46; y++) {
  for (let x = x0 + 252; x < x0 + 252 + 16; x++) {
    if (inRounded(x, y, 24)) px(x, y, ...accent);
  }
}

// --- PNG encode (truecolor + alpha) ---
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])) >>> 0, 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(b) {
  let c = 0xffffffff;
  for (let i = 0; i < b.length; i++) c = CRC_TABLE[(c ^ b[i]) & 0xff] ^ (c >>> 8);
  return c ^ 0xffffffff;
}

const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(S, 0);
ihdr.writeUInt32BE(S, 4);
ihdr[8] = 8;  // bit depth
ihdr[9] = 6;  // color type RGBA
ihdr[10] = 0;
ihdr[11] = 0;
ihdr[12] = 0;

const raw = Buffer.alloc((S * 4 + 1) * S);
for (let y = 0; y < S; y++) {
  raw[y * (S * 4 + 1)] = 0; // no filter
  buf.copy(raw, y * (S * 4 + 1) + 1, y * S * 4, (y + 1) * S * 4);
}
const idat = deflateSync(raw, { level: 9 });

const png = Buffer.concat([
  sig,
  chunk("IHDR", ihdr),
  chunk("IDAT", idat),
  chunk("IEND", Buffer.alloc(0)),
]);

mkdirSync(new URL("../src-tauri/icons/", import.meta.url), { recursive: true });
const out = new URL("../src-tauri/icons/source.png", import.meta.url);
writeFileSync(out, png);
console.log("wrote", out.pathname, png.length, "bytes");
