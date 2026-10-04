// Contract drift gate for the vendored lazy-contracts files under contracts/.
// Recomputes the sha256 of every pinned file and compares it to
// contracts/PIN.json. CI fails if a vendored file drifted from its pin, a
// pinned file went missing, or an unpinned file was added to contracts/.
import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const dir = "contracts";
const pin = JSON.parse(readFileSync(join(dir, "PIN.json"), "utf8"));
const pinned = pin.sha256 ?? {};
const origin = `${pin.upstream ?? "upstream"}@${pin.upstream_commit ?? "?"}`;

let failed = false;

for (const [name, expected] of Object.entries(pinned)) {
  let actual;
  try {
    actual = createHash("sha256")
      .update(readFileSync(join(dir, name)))
      .digest("hex");
  } catch {
    console.error(`${dir}/${name}: pinned in PIN.json but file is missing`);
    failed = true;
    continue;
  }
  if (actual !== expected) {
    console.error(
      `${dir}/${name}: sha256 drifted from ${origin}\n` +
        `  pinned ${expected}\n  actual ${actual}`
    );
    failed = true;
    continue;
  }
  console.log(`${dir}/${name}: ok`);
}

for (const entry of readdirSync(dir)) {
  if (entry === "PIN.json" || entry.startsWith(".")) continue;
  if (!(entry in pinned)) {
    console.error(`${dir}/${entry}: present but not pinned in PIN.json`);
    failed = true;
  }
}

if (failed) {
  console.error(
    `contract pin drift detected; re-vendor from ${origin} and update contracts/PIN.json`
  );
  process.exit(1);
}
console.log(`contracts pinned to ${origin}: ok`);
