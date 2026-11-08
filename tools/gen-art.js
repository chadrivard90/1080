// Generates the dashboard artwork (SVG) into ui/img. Run:  node tools/gen-art.js
// All art is original. SVGs contain no <text>: labels are drawn by Slint so fonts match.
const fs = require("fs");
const path = require("path");
const OUT = path.join(__dirname, "..", "ui", "img");
fs.mkdirSync(OUT, { recursive: true });
fs.mkdirSync(path.join(OUT, "icons"), { recursive: true });
fs.mkdirSync(path.join(OUT, "emblems"), { recursive: true });

const write = (name, body, w, h) =>
  fs.writeFileSync(
    path.join(OUT, name),
    `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">\n${body}\n</svg>\n`
  );

// Deterministic pseudo-random so regenerating gives the same art.
let seed = 1337;
const rnd = () => ((seed = (seed * 16807) % 2147483647) - 1) / 2147483646;
const f = (n) => n.toFixed(1);

// ---------------------------------------------------------------- textures
// Carbon fibre weave.
write(
  "carbon.svg",
  `<defs>
  <linearGradient id="a" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2b2b35"/><stop offset="0.5" stop-color="#1b1b22"/><stop offset="1" stop-color="#101015"/></linearGradient>
  <linearGradient id="b" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#25252e"/><stop offset="0.5" stop-color="#17171d"/><stop offset="1" stop-color="#0e0e12"/></linearGradient>
  <pattern id="w" width="28" height="28" patternUnits="userSpaceOnUse">
    <rect width="28" height="28" fill="#0d0d11"/>
    <rect x="1" y="1" width="12" height="12" rx="2" fill="url(#a)"/>
    <rect x="15" y="15" width="12" height="12" rx="2" fill="url(#a)"/>
    <rect x="15" y="1" width="12" height="12" rx="2" fill="url(#b)"/>
    <rect x="1" y="15" width="12" height="12" rx="2" fill="url(#b)"/>
  </pattern>
  <radialGradient id="v" cx="0.5" cy="0.35" r="0.8"><stop offset="0" stop-color="#000" stop-opacity="0"/><stop offset="1" stop-color="#000" stop-opacity="0.7"/></radialGradient>
</defs>
<rect width="1080" height="1920" fill="url(#w)"/>
<rect width="1080" height="1920" fill="url(#v)"/>`,
  1080,
  1920
);

// Grunge / dirt overlay (transparent background).
{
  let s = "";
  for (let i = 0; i < 1400; i++) {
    const x = rnd() * 1080, y = rnd() * 1920, r = 0.6 + rnd() * 2.6;
    const light = rnd() < 0.45;
    s += `<circle cx="${f(x)}" cy="${f(y)}" r="${f(r)}" fill="${light ? "#d8c9a8" : "#000"}" fill-opacity="${(light ? 0.05 : 0.35) * rnd() + 0.02}"/>`;
  }
  for (let i = 0; i < 90; i++) {
    const x = rnd() * 1080, y = rnd() * 1920, l = 20 + rnd() * 140, a = rnd() * Math.PI;
    s += `<path d="M${f(x)} ${f(y)} l${f(Math.cos(a) * l)} ${f(Math.sin(a) * l)}" stroke="#e9dcc0" stroke-opacity="${f(0.03 + rnd() * 0.06)}" stroke-width="${f(0.6 + rnd() * 1.4)}"/>`;
  }
  for (let i = 0; i < 26; i++) {
    const x = rnd() * 1080, y = rnd() * 1920, r = 40 + rnd() * 160;
    s += `<ellipse cx="${f(x)}" cy="${f(y)}" rx="${f(r)}" ry="${f(r * (0.4 + rnd() * 0.6))}" fill="#6b5a3e" fill-opacity="${f(0.03 + rnd() * 0.05)}"/>`;
  }
  write("grunge.svg", s, 1080, 1920);
}

// Blueprint grid (transparent background).
{
  let s = "";
  for (let x = 0; x <= 1080; x += 30) s += `<path d="M${x} 0V1920" stroke="#64ffda" stroke-opacity="${x % 150 === 0 ? 0.16 : 0.06}" stroke-width="${x % 150 === 0 ? 1.5 : 1}"/>`;
  for (let y = 0; y <= 1920; y += 30) s += `<path d="M0 ${y}H1080" stroke="#64ffda" stroke-opacity="${y % 150 === 0 ? 0.16 : 0.06}" stroke-width="${y % 150 === 0 ? 1.5 : 1}"/>`;
  write("blueprint.svg", s, 1080, 1920);
}

// Diagonal "festival" light streaks (transparent).
{
  let s = "";
  for (let i = 0; i < 18; i++) {
    const x = rnd() * 1600 - 300, w = 20 + rnd() * 120;
    s += `<path d="M${f(x)} 0 L${f(x + w)} 0 L${f(x + w - 700)} 1920 L${f(x - 700)} 1920 Z" fill="#ffffff" fill-opacity="${f(0.02 + rnd() * 0.05)}"/>`;
  }
  write("festival.svg", s, 1080, 1920);
}

// ---------------------------------------------------------------- analog dial (DiRT Rally)
// 800x800, 270 deg sweep starting bottom-left (-135 deg from 12 o'clock). 0..9 x1000 rpm.
{
  const c = 400, R = 330;
  const pt = (frac, r) => {
    const a = ((-135 + 270 * frac) - 90) * Math.PI / 180;
    return [c + r * Math.cos(a), c + r * Math.sin(a)];
  };
  let ticks = "";
  for (let i = 0; i <= 36; i++) {
    const fr = i / 36, major = i % 4 === 0, red = fr >= 7 / 9;
    const [x0, y0] = pt(fr, major ? R - 52 : R - 26), [x1, y1] = pt(fr, R);
    ticks += `<path d="M${f(x0)} ${f(y0)}L${f(x1)} ${f(y1)}" stroke="${red ? "#e8361c" : "#efe7cf"}" stroke-width="${major ? 10 : 4}" stroke-linecap="round"/>`;
  }
  const [rx0, ry0] = pt(7 / 9, R + 14), [rx1, ry1] = pt(1, R + 14);
  // Plain face + bezel (replaceable by AI art: art/dr2-tach-face.png).
  write(
    "tach_face.svg",
    `<defs>
  <radialGradient id="face" cx="0.5" cy="0.42" r="0.6"><stop offset="0" stop-color="#22211d"/><stop offset="1" stop-color="#090908"/></radialGradient>
  <linearGradient id="bezel" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#8b877c"/><stop offset="0.45" stop-color="#2e2c27"/><stop offset="0.55" stop-color="#1b1a17"/><stop offset="1" stop-color="#6c685e"/></linearGradient>
</defs>
<circle cx="400" cy="400" r="396" fill="url(#bezel)"/>
<circle cx="400" cy="400" r="372" fill="#050505"/>
<circle cx="400" cy="400" r="362" fill="url(#face)"/>
<circle cx="400" cy="400" r="150" fill="none" stroke="#efe7cf" stroke-opacity="0.08" stroke-width="2"/>`,
    800,
    800
  );
  // Tick marks + redline only (transparent), always drawn on top of the face so the numbers
  // and needle line up whatever face art is used.
  write(
    "tach_ticks.svg",
    `<path d="M${f(rx0)} ${f(ry0)} A ${R + 14} ${R + 14} 0 0 1 ${f(rx1)} ${f(ry1)}" fill="none" stroke="#c4250f" stroke-width="22"/>
${ticks}`,
    800,
    800
  );
}

// Needle: 800x800 so it rotates about the dial centre; points straight up at 0 deg.
write(
  "needle.svg",
  `<defs><linearGradient id="n" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#ff8a1f"/><stop offset="0.5" stop-color="#ffb35c"/><stop offset="1" stop-color="#d95a00"/></linearGradient></defs>
<path d="M392 420 L397 92 L403 92 L408 420 Z" fill="url(#n)"/>
<path d="M394 420 L406 420 L404 470 L396 470 Z" fill="#ff7a0a"/>
<circle cx="400" cy="400" r="44" fill="#151412" stroke="#3a372f" stroke-width="6"/>
<circle cx="400" cy="400" r="14" fill="#ff7a0a"/>`,
  800,
  800
);

// ---------------------------------------------------------------- cars (top-down, nose up)
const carBody = (color, accent) => `
<path d="M150 18 C205 18 238 48 244 104 L252 210 C256 250 256 380 252 430 L246 520 C240 568 205 586 150 586 C95 586 60 568 54 520 L48 430 C44 380 44 250 48 210 L56 104 C62 48 95 18 150 18 Z" fill="${color}" stroke="#000" stroke-width="6"/>
<path d="M82 150 C110 128 190 128 218 150 L210 230 C180 216 120 216 90 230 Z" fill="#0b0d12"/>
<path d="M92 410 C120 424 180 424 208 410 L214 470 C186 486 114 486 86 470 Z" fill="#0b0d12"/>
<path d="M90 240 L210 240 L206 400 L94 400 Z" fill="${accent}" fill-opacity="0.35"/>
<rect x="138" y="20" width="24" height="566" fill="${accent}" fill-opacity="0.85"/>
<rect x="22" y="96" width="34" height="96" rx="10" fill="#111"/><rect x="244" y="96" width="34" height="96" rx="10" fill="#111"/>
<rect x="22" y="420" width="34" height="100" rx="10" fill="#111"/><rect x="244" y="420" width="34" height="100" rx="10" fill="#111"/>`;
write("car_gt.svg", carBody("#d9dde4", "#1e90ff"), 300, 600);
write("car_drift.svg", carBody("#ff2d87", "#ffe14d"), 300, 600);
// Rally car outline for the WRC health matrix (zones are drawn on top in Slint, same 300x600 box).
write(
  "car_rally.svg",
  `<path d="M150 18 C205 18 238 48 244 104 L252 210 C256 250 256 380 252 430 L246 520 C240 568 205 586 150 586 C95 586 60 568 54 520 L48 430 C44 380 44 250 48 210 L56 104 C62 48 95 18 150 18 Z" fill="#1f1f22" stroke="#ff0055" stroke-width="5"/>
<path d="M82 230 C110 210 190 210 218 230 L212 300 C180 288 120 288 88 300 Z" fill="#000" fill-opacity="0.6"/>
<path d="M100 420 L200 420 L196 470 L104 470 Z" fill="#000" fill-opacity="0.5"/>`,
  300,
  600
);

// ---------------------------------------------------------------- compass tape (ticks only)
{
  // 0..720 degrees across 2880px (4px per degree) so it can scroll and wrap.
  let s = "";
  for (let d = 0; d <= 720; d += 5) {
    const x = d * 4, major = d % 45 === 0, mid = d % 15 === 0;
    s += `<path d="M${x} ${major ? 0 : mid ? 30 : 50}V90" stroke="#ffffff" stroke-opacity="${major ? 0.95 : 0.5}" stroke-width="${major ? 6 : 3}"/>`;
  }
  write("compass_tape.svg", s, 2880, 90);
}

// Wind / direction arrow (points up).
write(
  "arrow.svg",
  `<path d="M100 10 L170 120 L118 104 L118 190 L82 190 L82 104 L30 120 Z" fill="#36f1ff"/>`,
  200,
  200
);

// ---------------------------------------------------------------- tab icons (white; tinted in Slint)
const icon = (name, body) =>
  write(`icons/${name}.svg`, `<g fill="none" stroke="#fff" stroke-width="7" stroke-linecap="round" stroke-linejoin="round">${body}</g>`, 96, 96);
icon("wheel", `<circle cx="48" cy="48" r="38"/><circle cx="48" cy="48" r="9"/><path d="M12 42 C30 36 66 36 84 42 M48 57 L48 86 M39 52 L18 72 M57 52 L78 72"/>`);
icon("wrench", `<path d="M60 14 a20 20 0 0 0 -16 30 L14 74 a8 8 0 0 0 11 11 L55 55 a20 20 0 0 0 30 -16 L72 52 L60 49 L57 37 Z"/>`);
icon("map", `<path d="M20 70 C10 50 30 22 50 30 C66 36 60 54 74 58 C88 62 86 82 66 84 C46 86 30 88 20 70 Z"/><circle cx="50" cy="30" r="6" fill="#fff"/>`);
icon("gauge", `<path d="M14 66 A36 36 0 1 1 82 66"/><path d="M48 58 L68 32"/><circle cx="48" cy="58" r="6" fill="#fff"/>`);
icon("tyre", `<circle cx="48" cy="48" r="38"/><circle cx="48" cy="48" r="18"/><path d="M48 10 V22 M48 74 V86 M10 48 H22 M74 48 H86 M21 21 L30 30 M66 66 L75 75 M75 21 L66 30 M30 66 L21 75"/>`);
icon("flag", `<path d="M20 88 V10"/><path d="M20 12 H76 V52 H20" fill="#fff" fill-opacity="0.25"/><path d="M34 12 V52 M48 12 V52 M62 12 V52 M20 26 H76 M20 40 H76"/>`);
icon("chart", `<path d="M12 84 H88 M12 84 V12"/><path d="M20 70 L38 46 L52 58 L70 26 L84 36"/>`);
icon("stopwatch", `<circle cx="48" cy="54" r="32"/><path d="M48 54 L48 34 M40 12 H56 M48 12 V22 M74 28 L80 22"/>`);
icon("fuel", `<path d="M18 86 V18 a6 6 0 0 1 6 -6 H52 a6 6 0 0 1 6 6 V86 Z M18 44 H58"/><path d="M58 30 H70 L80 42 V72 a6 6 0 0 1 -12 0 V58 H58"/>`);
icon("radar", `<circle cx="48" cy="48" r="38"/><circle cx="48" cy="48" r="22"/><path d="M48 48 L80 28"/><circle cx="48" cy="48" r="5" fill="#fff"/>`);
icon("list", `<path d="M30 22 H86 M30 48 H86 M30 74 H86"/><circle cx="14" cy="22" r="4" fill="#fff"/><circle cx="14" cy="48" r="4" fill="#fff"/><circle cx="14" cy="74" r="4" fill="#fff"/>`);
icon("thermo", `<path d="M40 60 V18 a8 8 0 0 1 16 0 V60 a16 16 0 1 1 -16 0 Z"/><path d="M48 66 V34"/>`);
icon("target", `<circle cx="48" cy="48" r="36"/><path d="M48 6 V26 M48 70 V90 M6 48 H26 M70 48 H90"/><circle cx="58" cy="40" r="7" fill="#fff"/>`);
icon("bars", `<path d="M16 84 V58 M32 84 V40 M48 84 V24 M64 84 V46 M80 84 V30"/>`);
icon("drift", `<path d="M30 70 C30 40 50 22 74 18"/><path d="M58 14 L76 18 L70 34"/><rect x="18" y="58" width="22" height="30" rx="5" transform="rotate(-25 29 73)"/>`);
icon("compass", `<circle cx="48" cy="48" r="38"/><path d="M48 18 L58 48 L48 78 L38 48 Z"/><path d="M48 18 L58 48 L38 48 Z" fill="#fff"/>`);
icon("cloud", `<path d="M26 66 a16 16 0 0 1 2 -32 a22 22 0 0 1 42 6 a14 14 0 0 1 0 26 Z"/><path d="M32 78 L28 88 M48 78 L44 88 M64 78 L60 88"/>`);
icon("pit", `<path d="M12 84 H84 M22 84 V40 L48 18 L74 40 V84"/><path d="M38 84 V60 H58 V84"/>`);
icon("music", `<path d="M36 72 V22 L80 12 V62"/><circle cx="26" cy="72" r="10"/><circle cx="70" cy="62" r="10"/>`);
icon("menu", `<path d="M14 46 L48 16 L82 46 M24 38 V82 H72 V38"/><path d="M40 82 V60 H56 V82"/>`);

// ---------------------------------------------------------------- game emblems (256x256, original art)
const emblem = (name, bg1, bg2, body) =>
  write(
    `emblems/${name}.svg`,
    `<defs><radialGradient id="g" cx="0.5" cy="0.35" r="0.75"><stop offset="0" stop-color="${bg1}"/><stop offset="1" stop-color="${bg2}"/></radialGradient></defs>
<circle cx="128" cy="128" r="122" fill="url(#g)" stroke="#ffffff" stroke-opacity="0.85" stroke-width="6"/>
${body}`,
    256,
    256
  );
// F1: open-wheel car with halo, side view.
emblem("f1", "#3a0606", "#120203", `
<path d="M34 150 L70 140 L104 128 L150 122 L196 130 L222 142 L222 156 L34 158 Z" fill="#e10600"/>
<path d="M118 122 C128 100 156 100 166 122" fill="none" stroke="#fff" stroke-width="7"/>
<rect x="196" y="112" width="34" height="10" fill="#e10600"/><rect x="216" y="112" width="8" height="34" fill="#e10600"/>
<rect x="26" y="146" width="40" height="8" fill="#e10600"/>
<circle cx="78" cy="160" r="22" fill="#111" stroke="#ddd" stroke-width="5"/><circle cx="188" cy="160" r="24" fill="#111" stroke="#ddd" stroke-width="5"/>
<path d="M40 196 H216" stroke="#fff" stroke-opacity="0.5" stroke-width="5" stroke-dasharray="16 10"/>`);
// NASCAR: stock car side with number roundel and broadcast stripes.
emblem("nascar", "#0b3aa8", "#04153d", `
<path d="M28 160 L40 132 L88 122 L112 98 L170 98 L200 124 L230 132 L232 160 Z" fill="#f2f2f2"/>
<path d="M28 160 L232 160 L232 148 L28 148 Z" fill="#da291c"/>
<path d="M120 104 L166 104 L188 124 L114 124 Z" fill="#0b1a3a"/>
<circle cx="150" cy="138" r="14" fill="#ffd400"/><circle cx="76" cy="164" r="20" fill="#111" stroke="#ccc" stroke-width="5"/><circle cx="192" cy="164" r="20" fill="#111" stroke="#ccc" stroke-width="5"/>
<path d="M30 200 L226 200" stroke="#ffd400" stroke-width="8"/><path d="M30 214 L226 214" stroke="#da291c" stroke-width="8"/>`);
// AC EVO: GT car silhouette over a data-logger trace.
emblem("acevo", "#1a1a1a", "#000000", `
<path d="M20 178 L60 150 L100 140 L180 140 L230 178" fill="none" stroke="#1e90ff" stroke-width="4" stroke-opacity="0.6"/>
<path d="M30 156 C60 140 80 126 112 112 C150 98 186 106 214 128 L232 140 L232 158 Z" fill="#d9dde4"/>
<path d="M112 116 C140 104 170 108 190 124 L124 126 Z" fill="#0b0d12"/>
<rect x="26" y="150" width="210" height="10" fill="#ff1a1a"/>
<circle cx="78" cy="162" r="20" fill="#111" stroke="#888" stroke-width="5"/><circle cx="192" cy="162" r="20" fill="#111" stroke="#888" stroke-width="5"/>`);
// AMS2: waving chequered flag over green/yellow arcs.
{
  let checks = "";
  for (let r = 0; r < 4; r++) for (let c = 0; c < 5; c++) if ((r + c) % 2 === 0) {
    const x = 92 + c * 22, y = 70 + r * 22 + Math.sin(c * 0.9) * 8;
    checks += `<rect x="${f(x)}" y="${f(y)}" width="22" height="22" fill="#111"/>`;
  }
  emblem("ams2", "#0f5132", "#062015", `
<path d="M40 200 A100 100 0 0 1 216 200" fill="none" stroke="#ffd400" stroke-width="10"/>
<path d="M58 214 A80 80 0 0 1 198 214" fill="none" stroke="#00b140" stroke-width="10"/>
<path d="M86 60 L86 206" stroke="#ddd" stroke-width="7"/>
<path d="M92 66 C130 56 160 82 202 70 L202 160 C160 172 130 146 92 156 Z" fill="#f4f4f4"/>${checks}`);
}
// DiRT Rally: rally car airborne over a dust cloud.
emblem("dr2", "#5a2a06", "#1b0c02", `
<ellipse cx="120" cy="200" rx="90" ry="22" fill="#c08a4a" fill-opacity="0.45"/>
<ellipse cx="70" cy="190" rx="40" ry="18" fill="#c08a4a" fill-opacity="0.35"/>
<g transform="rotate(-12 128 128)">
<path d="M40 150 L56 124 L96 116 L118 92 L168 92 L192 118 L222 126 L224 152 Z" fill="#ff6a00"/>
<path d="M124 98 L164 98 L182 118 L118 118 Z" fill="#111"/>
<rect x="54" y="140" width="168" height="8" fill="#fff"/>
<circle cx="82" cy="156" r="20" fill="#111" stroke="#ccc" stroke-width="5"/><circle cx="190" cy="156" r="20" fill="#111" stroke="#ccc" stroke-width="5"/></g>`);
// EA WRC: slashing gravel spray with cyan/magenta blades.
emblem("wrc", "#0d1b24", "#05090c", `
<path d="M40 190 L150 50 L176 50 L66 190 Z" fill="#00e5ff"/>
<path d="M92 200 L202 60 L228 60 L118 200 Z" fill="#ff2bd6"/>
${Array.from({ length: 26 }, () => `<circle cx="${f(60 + rnd() * 150)}" cy="${f(150 + rnd() * 60)}" r="${f(2 + rnd() * 5)}" fill="#c9b28a" fill-opacity="0.8"/>`).join("")}`);
// Forza Motorsport: circuit outline with a racing line dot.
emblem("fm", "#14324f", "#071321", `
<path d="M60 170 C40 120 70 70 120 76 C150 80 150 110 176 112 C210 114 214 160 186 182 C160 200 90 210 60 170 Z" fill="none" stroke="#e6007e" stroke-width="12" stroke-linejoin="round"/>
<path d="M60 170 C40 120 70 70 120 76 C150 80 150 110 176 112 C210 114 214 160 186 182 C160 200 90 210 60 170 Z" fill="none" stroke="#00d1ff" stroke-width="4" stroke-dasharray="14 10"/>
<circle cx="176" cy="112" r="12" fill="#ffffff"/>`);
// Forza Horizon: festival sunburst over hills.
{
  let rays = "";
  for (let i = 0; i < 12; i++) {
    const a = Math.PI + (i / 11) * Math.PI;
    rays += `<path d="M128 160 L${f(128 + Math.cos(a) * 110)} ${f(160 + Math.sin(a) * 110)}" stroke="#ffe14d" stroke-width="7" stroke-opacity="0.8"/>`;
  }
  emblem("fh6", "#7a1050", "#2a0632", `
${rays}
<circle cx="128" cy="160" r="40" fill="#ffe14d"/>
<path d="M14 190 C60 150 100 170 128 180 C160 192 200 150 242 178 L242 230 L14 230 Z" fill="#ff2d87"/>
<path d="M14 210 C70 186 120 214 170 200 C200 192 220 196 242 204 L242 240 L14 240 Z" fill="#36f1ff" fill-opacity="0.85"/>`);
}

// Title hero: stylised speed lines + chequered motif (no text).
{
  let checks = "";
  for (let r = 0; r < 6; r++) for (let c = 0; c < 30; c++) if ((r + c) % 2 === 0) {
    const x = c * 36, y = 380 + r * 36;
    checks += `<rect x="${x}" y="${y}" width="36" height="36" fill="#fff" fill-opacity="${f(0.10 - c * 0.003)}"/>`;
  }
  let lines = "";
  for (let i = 0; i < 16; i++) {
    const y = 40 + rnd() * 320, len = 200 + rnd() * 700, x = 1080 - len - rnd() * 200;
    lines += `<path d="M${f(x)} ${f(y)} H${f(x + len)}" stroke="${i % 3 === 0 ? "#e10600" : "#ffffff"}" stroke-opacity="${f(0.15 + rnd() * 0.4)}" stroke-width="${f(2 + rnd() * 6)}" stroke-linecap="round"/>`;
  }
  write("hero.svg", `${lines}${checks}`, 1080, 600);
}

console.log("Artwork written to", OUT);
