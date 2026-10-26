// Mechanical CSS sanity gate for the generated viewer stylesheet.
// CI runs this so a stray token (e.g. a leading quote that once silently
// killed the whole :root token block) can never ship again. It also
// validates the shared design tokens (assets/tokens.css): they must parse,
// be bundled first into BOTH pages, and meet the contrast contract in both
// themes.
import { readFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { appBundleCss, viewerBundleCss } from "./lib/app-bundle.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
// The shell layout stylesheet is assets/app/00-layout.css after T2 renames
// app.css (docs/ui-rebuild-contract.md section 1.2); accept either so the gate
// works on both sides of that rename.
const layoutCss = existsSync(join(root, "assets/app/00-layout.css")) ? "assets/app/00-layout.css" : "assets/app/app.css";
const files = ["assets/evidence_viewer.css", layoutCss];
// These have no token block of their own (kit.css is bundled right after
// tokens.css, the rest after the page stylesheet): only brace balance is
// checked. Optional ones are skipped until their track lands.
for (const extra of ["assets/kit.css", "assets/viewer/controls.css", "assets/app/30-jobs.css", "assets/app/40-finalize.css", "assets/app/56-tools.css"]) {
  if (!existsSync(join(root, extra))) {
    if (extra === "assets/kit.css" || extra === "assets/viewer/controls.css") {
      console.error(`${extra}: missing`);
      process.exitCode = 1;
    }
    continue;
  }
  const css = readFileSync(join(root, extra), "utf8");
  const open = (css.match(/{/g) ?? []).length;
  const close = (css.match(/}/g) ?? []).length;
  if (open !== close) {
    console.error(`${extra}: unbalanced braces (${open} open / ${close} close)`);
    process.exitCode = 1;
  } else console.log(`${extra}: ok`);
}

for (const file of files) {
  const css = readFileSync(join(root, file), "utf8");
  const open = (css.match(/{/g) ?? []).length;
  const close = (css.match(/}/g) ?? []).length;
  if (open !== close) {
    console.error(`${file}: unbalanced braces (${open} open / ${close} close)`);
    process.exitCode = 1;
    continue;
  }
  if (!css.trimStart().startsWith(":root")) {
    console.error(`${file}: must start with the :root token block`);
    process.exitCode = 1;
    continue;
  }
  if (!css.includes("color-scheme: light")) {
    console.error(`${file}: color-scheme guard missing`);
    process.exitCode = 1;
    continue;
  }
  console.log(`${file}: ok`);
}

// ---- shared design tokens (assets/tokens.css) ----
let tokenErrors = 0;
const bad = (msg) => {
  console.error(`tokens.css: ${msg}`);
  tokenErrors++;
};
const tokensCss = readFileSync(join(root, "assets/tokens.css"), "utf8");
{
  const open = (tokensCss.match(/{/g) ?? []).length;
  const close = (tokensCss.match(/}/g) ?? []).length;
  if (open !== close) bad(`unbalanced braces (${open} open / ${close} close)`);
  if (!tokensCss.includes('html[data-theme="dark"]')) bad("dark theme block missing");
  if (/@import|url\(\s*["']?https?:/i.test(tokensCss)) bad("must not load remote resources (offline tool)");
}
const nthBanner = (css, n) => css.split("\n").filter((l) => l.startsWith("/* ==== "))[n] ?? "";
const firstBanner = (css) => css.split("\n").find((l) => l.startsWith("/* ==== ")) ?? "";
for (const [name, css] of [
  ["shell", appBundleCss(root)],
  ["viewer", viewerBundleCss(root)],
]) {
  if (!firstBanner(css).includes("assets/tokens.css")) {
    bad(`${name} CSS bundle does not start with assets/tokens.css (node mirror)`);
  }
  if (!nthBanner(css, 1).includes("assets/kit.css")) {
    bad(`${name} CSS bundle must carry assets/kit.css second (node mirror)`);
  }
  if ((css.match(/\/\* ==== assets\/kit\.css/g) ?? []).length !== 1) {
    bad(`${name} CSS bundle must include assets/kit.css exactly once`);
  }
}
{
  const build = readFileSync(join(root, "build.rs"), "utf8");
  if ((build.match(/kit_path\(\)/g) ?? []).length < 3) {
    bad("build.rs must bundle kit_path() into both the shell and the viewer");
  }
  if ((build.match(/tokens_path\(\)/g) ?? []).length < 3) {
    bad("build.rs must bundle tokens_path() into both the shell and the viewer");
  }
  const report = readFileSync(join(root, "src/html_report.rs"), "utf8");
  if (!report.includes('"/evidence_viewer.css"')) {
    bad("src/html_report.rs must embed the bundled viewer css from OUT_DIR");
  }
}

function tokenMap(block) {
  const m = {};
  for (const [, k, v] of block.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)) m[k] = v.trim();
  return m;
}
const darkStart = tokensCss.indexOf('html[data-theme="dark"]');
const light = tokenMap(tokensCss.slice(tokensCss.indexOf(":root {"), darkStart));
const dark = { ...light, ...tokenMap(tokensCss.slice(darkStart, tokensCss.indexOf("}", darkStart))) };

const lum = (hex) => {
  const c = [1, 3, 5]
    .map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
    .map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
};
const ratio = (a, b) => {
  const x = lum(a);
  const y = lum(b);
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
};
const isHex = (v) => /^#[0-9a-f]{6}$/i.test(v ?? "");
const surfaces = ["--bg", "--surface", "--surface-2"];
const statuses = ["verified", "failed", "candidate", "original", "carved", "fs-recovered", "deleted-candidate", "anomaly", "warn", "info"];
const marks = ["reviewed", "important", "needs-verification"];
const pairs = [];
for (const s of surfaces) {
  for (const t of ["--text", "--muted", "--accent", "--accent-ink", "--ok", "--danger", "--warn", "--info"]) {
    pairs.push([t, s, 4.5]);
  }
}
pairs.push(["--on-accent", "--accent", 4.5], ["--text", "--accent-soft", 4.5], ["--accent-ink", "--accent-soft", 4.5], ["--muted", "--accent-soft", 4.5]);
pairs.push(["--disabled-fg", "--disabled-bg", 3]);
for (const st of statuses) {
  pairs.push([`--${st}-fg`, `--${st}-bg`, 4.5]);
  for (const s of surfaces) pairs.push([`--${st}-fg`, s, 4.5]);
}
for (const m of marks) pairs.push([`--mark-${m}-fg`, `--mark-${m}-bg`, 4.5]);
// Section 5.2 additions: video stage text, non-text contrast (WCAG 1.4.11) for
// control outlines / accent, and single-colour mark glyphs.
pairs.push(["--media-muted", "--media-bg", 4.5]);
for (const s of surfaces) {
  pairs.push(["--border-strong", s, 3], ["--accent", s, 3]);
  for (const m of marks) pairs.push([`--mark-${m}-ink`, s, 3]);
}
// Media stage / toolbar (dark in both themes): text on every media fill,
// non-text control outline and focus ring >= 3:1, disabled toolbar text >= 3:1.
// --media-border is a decorative hairline (controls use --media-control-border).
for (const s of ["--media-bg", "--media-surface", "--media-hover", "--media-active"]) pairs.push(["--media-fg", s, 4.5]);
for (const s of ["--media-surface", "--media-hover"]) pairs.push(["--media-muted", s, 4.5]);
for (const s of ["--media-bg", "--media-surface"]) pairs.push(["--media-ring", s, 3], ["--media-control-border", s, 3]);
pairs.push(["--media-disabled-fg", "--media-surface", 3]);
// Pressed toolbar toggle (.on): accent fill bounded by --media-ring on the bar.
pairs.push(["--media-ring", "--media-hover", 3]);
// Calendar heat levels carry the day number (--text) and, on l1, the muted count.
for (const l of ["l1", "l2", "l3"]) pairs.push(["--text", `--cal-${l}-bg`, 4.5]);
pairs.push(["--muted", "--cal-l1-bg", 4.5], ["--mark-hl-fg", "--mark-hl-bg", 4.5]);
for (const [theme, map] of [
  ["light", light],
  ["dark", dark],
]) {
  // A disabled toolbar button must read dimmer than the .muted toolbar labels.
  if (isHex(map["--media-disabled-fg"]) && isHex(map["--media-muted"]) && isHex(map["--media-surface"])
    && ratio(map["--media-disabled-fg"], map["--media-surface"]) * 1.5 > ratio(map["--media-muted"], map["--media-surface"])) {
    bad(`${theme}: --media-disabled-fg is not clearly dimmer than --media-muted on --media-surface`);
  }
  // The dashed disabled outline is visible (>= 1.5:1) but dimmer than the
  // 3:1 control outline, so disabled reads as "off" and not as a live control.
  if (isHex(map["--media-disabled-border"]) && isHex(map["--media-control-border"]) && isHex(map["--media-surface"])) {
    const d = ratio(map["--media-disabled-border"], map["--media-surface"]);
    const c = ratio(map["--media-control-border"], map["--media-surface"]);
    if (d < 1.5 || d >= c) bad(`${theme}: --media-disabled-border ${d.toFixed(2)}:1 must be >= 1.5 and below --media-control-border ${c.toFixed(2)}:1`);
  } else bad(`${theme}: --media-disabled-border / --media-control-border missing`);
  // Calendar heat ramp: each level steps at least as far as the one before
  // (surface -> l1 -> l2 -> l3), so the upper levels never blur together.
  {
    const ramp = ["--surface", "--cal-l1-bg", "--cal-l2-bg", "--cal-l3-bg"].map((k) => map[k]);
    if (ramp.every(isHex)) {
      const steps = ramp.slice(1).map((v, i) => ratio(v, ramp[i]));
      if (steps.some((x, i) => i && x + 1e-9 < steps[i - 1])) bad(`${theme}: calendar ramp steps ${steps.map((x) => x.toFixed(2)).join(" / ")} must not shrink`);
    }
  }
  for (const [fg, bg, min] of pairs) {
    if (!isHex(map[fg]) || !isHex(map[bg])) {
      bad(`${theme}: ${fg} or ${bg} missing / not #rrggbb`);
      continue;
    }
    const r = ratio(map[fg], map[bg]);
    if (r + 1e-9 < min) bad(`${theme}: ${fg} ${map[fg]} on ${bg} ${map[bg]} = ${r.toFixed(2)}:1 < ${min}:1`);
  }
}
// DESIGN.md mirrors tokens.css: every token it names must exist, and every
// token-table row ("| `--a` / `--b` | light / light | dark / dark | role |")
// must carry exactly the CSS values.
if (existsSync(join(root, "DESIGN.md"))) {
  const design = readFileSync(join(root, "DESIGN.md"), "utf8");
  for (const [, k] of design.matchAll(/`(--[a-z0-9-]+)`/g)) {
    if (!(k in light) && !(k in dark)) bad(`DESIGN.md names ${k}, which tokens.css does not define`);
  }
  let rows = 0;
  const listed = new Set();
  for (const line of design.split("\n")) {
    const cells = line.split("|").slice(1, -1).map((c) => c.trim());
    if (cells.length < 3 || !cells[0].startsWith("`--")) continue;
    const vals = (c) => [...c.matchAll(/`([^`]+)`/g)].map((m) => m[1]);
    const names = vals(cells[0]);
    const [lv, dv] = [vals(cells[1]), vals(cells[2])];
    if (!names.length || lv.length !== names.length || dv.length !== names.length) {
      bad(`DESIGN.md token row "${cells[0]}" needs one backticked light and dark value per token (${names.length} / ${lv.length} / ${dv.length})`);
      continue;
    }
    names.forEach((n) => listed.add(n));
    rows++;
    names.forEach((n, i) => {
      if (light[n] !== lv[i]) bad(`DESIGN.md light ${n} = ${lv[i]}, tokens.css has ${light[n]}`);
      if (dark[n] !== dv[i]) bad(`DESIGN.md dark ${n} = ${dv[i]}, tokens.css has ${dark[n]}`);
    });
  }
  if (rows < 20) bad(`DESIGN.md token table not found (${rows} rows)`);
  // Reverse: every colour token in tokens.css has a row in the table.
  for (const k of new Set([...Object.keys(light), ...Object.keys(dark)])) {
    if ((isHex(light[k]) || isHex(dark[k])) && !listed.has(k)) bad(`DESIGN.md token table has no row for ${k}`);
  }
}
if (!/--fs-xs:\s*13px/.test(tokensCss)) bad("--fs-xs must be 13px (minimum text size)");
if (!/--fs-sm:\s*14px/.test(tokensCss)) bad("--fs-sm must be 14px (body size)");
if (!/--radius:\s*3px/.test(tokensCss)) bad("--radius must be 3px");
for (const [theme, map] of [["light", light], ["dark", dark]]) {
  for (const [k, v] of Object.entries({ "--ctl-h": "32px", "--ctl-h-lg": "36px", "--ctl-h-icon": "28px", "--hit-min": "24px" })) {
    if (map[k] !== v) bad(`${theme}: ${k} must be ${v}`);
  }
}
if (tokenErrors) process.exitCode = 1;
else console.log(`assets/tokens.css: ok (${pairs.length} contrast pairs x 2 themes, bundled first into shell + viewer)`);
