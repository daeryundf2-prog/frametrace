// Mechanical CSS sanity gate for the generated viewer stylesheet.
// CI runs this so a stray token (e.g. a leading quote that once silently
// killed the whole :root token block) can never ship again.
import { readFileSync } from "node:fs";

const files = ["assets/evidence_viewer.css"];

for (const file of files) {
  const css = readFileSync(file, "utf8");
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
