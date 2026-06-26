const fs = require("fs");
const path = require("path");
const { chromium } = require("playwright");

const evidenceDir = __dirname;
const url = "http://127.0.0.1:4177/index.html";
const requiredConcepts = [
  "Evidence sources",
  "Video candidates",
  "Validation queue",
  "Export preview",
  "Report package",
  "local-first",
  "verification required",
  "candidate-unvalidated",
];

const viewports = [
  { id: "desktop-1280", width: 1280, height: 900 },
  { id: "tablet-768", width: 768, height: 900 },
  { id: "mobile-375", width: 375, height: 900 },
];

function firstViewportTextScript() {
  const viewportWidth = window.innerWidth;
  const viewportHeight = window.innerHeight;
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT);
  const texts = [];
  while (walker.nextNode()) {
    const element = walker.currentNode;
    const style = window.getComputedStyle(element);
    if (
      style.display === "none" ||
      style.visibility === "hidden" ||
      Number(style.opacity) === 0 ||
      element.getAttribute("aria-hidden") === "true"
    ) {
      continue;
    }
    const rect = element.getBoundingClientRect();
    const intersectsViewport =
      rect.width > 0 &&
      rect.height > 0 &&
      rect.bottom >= 0 &&
      rect.right >= 0 &&
      rect.top <= viewportHeight &&
      rect.left <= viewportWidth;
    if (!intersectsViewport) continue;
    const directText = Array.from(element.childNodes)
      .filter((node) => node.nodeType === Node.TEXT_NODE)
      .map((node) => node.textContent.trim())
      .filter(Boolean)
      .join(" ");
    if (directText) texts.push(directText);
  }
  return texts.join("\n").replace(/\s+/g, " ").trim();
}

(async () => {
  const browser = await chromium.launch({ headless: true });
  const assertions = [];
  const failures = [];

  try {
    for (const viewport of viewports) {
      const page = await browser.newPage({ viewport: { width: viewport.width, height: viewport.height } });
      const pageErrors = [];
      page.on("pageerror", (error) => pageErrors.push(error.message));
      page.on("console", (message) => {
        if (message.type() === "error") pageErrors.push(message.text());
      });
      await page.goto(url, { waitUntil: "networkidle" });

      const screenshotPath = path.join(evidenceDir, `${viewport.id}.png`);
      await page.screenshot({ path: screenshotPath, fullPage: false });

      const visibleText = await page.evaluate(firstViewportTextScript);
      const visibleTextPath = path.join(evidenceDir, `${viewport.id}-visible-text.txt`);
      fs.writeFileSync(visibleTextPath, `${visibleText}\n`);

      const overflow = await page.evaluate(() => ({
        viewportWidth: window.innerWidth,
        clientWidth: document.documentElement.clientWidth,
        scrollWidth: document.documentElement.scrollWidth,
        bodyScrollWidth: document.body.scrollWidth,
        hasHorizontalOverflow:
          document.documentElement.scrollWidth > window.innerWidth + 1 ||
          document.body.scrollWidth > window.innerWidth + 1,
      }));
      const overflowPath = path.join(evidenceDir, `${viewport.id}-overflow.json`);
      fs.writeFileSync(overflowPath, `${JSON.stringify(overflow, null, 2)}\n`);

      const conceptResults = requiredConcepts.map((label) => ({
        label,
        visible: visibleText.includes(label),
      }));
      for (const result of conceptResults) {
        if (!result.visible) failures.push(`${viewport.id} missing visible text: ${result.label}`);
      }
      if (overflow.hasHorizontalOverflow) {
        failures.push(`${viewport.id} horizontal overflow: scrollWidth=${overflow.scrollWidth}`);
      }
      if (pageErrors.length) {
        failures.push(`${viewport.id} page errors: ${pageErrors.join(" | ")}`);
      }

      const renderedState = await page.evaluate(() => ({
        visibleRows: document.querySelectorAll(".file-row.data-row").length,
        visibleCandidateRows: Array.from(document.querySelectorAll(".file-row.data-row"))
          .filter((row) => row.textContent.includes("Candidate")).length,
        decisionGateRows: document.querySelectorAll(".decision-gate-row").length,
        mediaCanvasPixels: (() => {
          const canvas = document.querySelector("#viewerCanvas");
          if (!canvas) return 0;
          const context = canvas.getContext("2d");
          const image = context.getImageData(0, 0, canvas.width, canvas.height).data;
          let nonBlank = 0;
          for (let index = 0; index < image.length; index += 4) {
            if (image[index] || image[index + 1] || image[index + 2]) nonBlank += 1;
          }
          return nonBlank;
        })(),
      }));
      if (renderedState.visibleRows < 1) failures.push(`${viewport.id} rendered no candidate rows`);
      if (renderedState.visibleCandidateRows < 1) failures.push(`${viewport.id} rendered no visible candidate rows`);
      if (renderedState.decisionGateRows < 5) failures.push(`${viewport.id} rendered incomplete decision gate`);
      if (renderedState.mediaCanvasPixels < 1000) failures.push(`${viewport.id} media canvas appears blank`);

      assertions.push({
        viewport,
        screenshotPath,
        visibleTextPath,
        overflowPath,
        conceptResults,
        overflow,
        pageErrors,
        renderedState,
      });
      await page.close();
    }
  } finally {
    await browser.close();
  }

  fs.writeFileSync(
    path.join(evidenceDir, "playwright-assertions.json"),
    `${JSON.stringify({
      url,
      requiredConcepts,
      assertions,
      failures,
      cleanup: "browser closed by standalone Playwright script; python http server stopped by parent shell trap",
    }, null, 2)}\n`,
  );

  if (failures.length) {
    console.error(failures.join("\n"));
    process.exit(1);
  }
  console.log(`PASS ${viewports.length} viewports; screenshots and assertions written to ${evidenceDir}`);
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
