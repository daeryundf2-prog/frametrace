const fs = require('fs');
const path = require('path');
const { expect, test } = require('@playwright/test');

const evidenceDir = __dirname;
const url = 'http://127.0.0.1:4177/index.html';
const requiredConcepts = [
  'Evidence intake',
  'Video candidates',
  'Validation state',
  'Export',
  'Report',
  'local-first',
  'verification required',
  'candidate-unvalidated',
];

const viewports = [
  { id: 'desktop-1280', width: 1280, height: 900 },
  { id: 'tablet-768', width: 768, height: 900 },
  { id: 'mobile-375', width: 375, height: 900 },
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
      style.display === 'none' ||
      style.visibility === 'hidden' ||
      Number(style.opacity) === 0 ||
      element.getAttribute('aria-hidden') === 'true'
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
      .join(' ');
    if (directText) texts.push(directText);
  }
  return texts.join('\n').replace(/\s+/g, ' ').trim();
}

test.describe('FrameTrace evidence viewer first-screen IA', () => {
  const assertionSummary = [];

  for (const viewport of viewports) {
    test(`${viewport.id} shows forensic workflow IA without horizontal overflow`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto(url, { waitUntil: 'networkidle' });

      const screenshotPath = path.join(evidenceDir, `${viewport.id}.png`);
      await page.screenshot({ path: screenshotPath, fullPage: false });

      const visibleText = await page.evaluate(firstViewportTextScript);
      fs.writeFileSync(path.join(evidenceDir, `${viewport.id}-visible-text.txt`), `${visibleText}\n`);

      const overflow = await page.evaluate(() => ({
        viewportWidth: window.innerWidth,
        clientWidth: document.documentElement.clientWidth,
        scrollWidth: document.documentElement.scrollWidth,
        bodyScrollWidth: document.body.scrollWidth,
        hasHorizontalOverflow:
          document.documentElement.scrollWidth > window.innerWidth + 1 ||
          document.body.scrollWidth > window.innerWidth + 1,
      }));
      fs.writeFileSync(path.join(evidenceDir, `${viewport.id}-overflow.json`), `${JSON.stringify(overflow, null, 2)}\n`);

      const conceptResults = requiredConcepts.map((label) => ({
        label,
        visible: visibleText.includes(label),
      }));
      assertionSummary.push({
        viewport,
        screenshotPath,
        visibleTextPath: path.join(evidenceDir, `${viewport.id}-visible-text.txt`),
        overflowPath: path.join(evidenceDir, `${viewport.id}-overflow.json`),
        conceptResults,
        overflow,
      });

      for (const result of conceptResults) {
        expect(result.visible, `${viewport.id} visible text includes ${result.label}`).toBe(true);
      }
      expect(overflow.hasHorizontalOverflow, `${viewport.id} has no horizontal overflow`).toBe(false);
    });
  }

  test.afterAll(() => {
    fs.writeFileSync(
      path.join(evidenceDir, 'playwright-assertions.json'),
      `${JSON.stringify({
        url,
        requiredConcepts,
        assertions: assertionSummary,
        cleanup: 'browser contexts closed by Playwright; python http server stopped by parent shell trap',
      }, null, 2)}\n`,
    );
  });
});
