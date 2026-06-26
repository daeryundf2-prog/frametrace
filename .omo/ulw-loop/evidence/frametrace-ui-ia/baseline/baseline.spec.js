const fs = require('fs');
const path = require('path');
const { test } = require('@playwright/test');

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
  { id: 'desktop-1280x900', width: 1280, height: 900 },
  { id: 'tablet-768x900', width: 768, height: 900 },
  { id: 'mobile-375x900', width: 375, height: 900 },
];

const matrix = {
  surfaceEvidence: [],
  adversarialCases: [],
  artifactRefs: [],
};

function addArtifact(id, kind, description, artifactPath) {
  const relPath = path.relative(evidenceDir, artifactPath);
  const existing = matrix.artifactRefs.find((artifact) => artifact.id === id);
  if (!existing) {
    matrix.artifactRefs.push({
      id,
      kind,
      description,
      path: artifactPath,
    });
  } else {
    existing.path = artifactPath;
    existing.description = description;
  }
  return relPath;
}

async function firstViewportText(page) {
  return page.evaluate(() => {
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
      if (!intersectsViewport) {
        continue;
      }
      const directText = Array.from(element.childNodes)
        .filter((node) => node.nodeType === Node.TEXT_NODE)
        .map((node) => node.textContent.trim())
        .filter(Boolean)
        .join(' ');
      if (directText) {
        texts.push(directText);
      }
    }
    return texts.join('\n').replace(/\s+/g, ' ').trim();
  });
}

test.describe('FrameTrace evidence viewer first screen baseline', () => {
  for (const viewport of viewports) {
    test(`${viewport.id} first-screen IA baseline`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto(url, { waitUntil: 'networkidle' });

      const screenshotPath = path.join(evidenceDir, `${viewport.id}.png`);
      await page.screenshot({ path: screenshotPath, fullPage: false });
      addArtifact(
        `screenshot-${viewport.id}`,
        'screenshot',
        `First viewport screenshot at ${viewport.width}x${viewport.height}`,
        screenshotPath,
      );

      const visibleText = await firstViewportText(page);
      const textPath = path.join(evidenceDir, `${viewport.id}-visible-text.txt`);
      fs.writeFileSync(textPath, `${visibleText}\n`);
      addArtifact(
        `visible-text-${viewport.id}`,
        'log',
        `Visible first-screen text captured at ${viewport.width}x${viewport.height}`,
        textPath,
      );

      for (const concept of requiredConcepts) {
        const pass = visibleText.includes(concept);
        matrix.surfaceEvidence.push({
          scenarioId: `${viewport.id}-concept-${concept.replace(/[^A-Za-z0-9]+/g, '-').replace(/^-|-$/g, '')}`,
          criterionRef: `first-screen-visible-text:${concept}`,
          surface: 'Chromium browser first viewport',
          exactInvocation:
            `NODE_PATH=/Users/shinyoohag/.npm/_npx/420ff84f11983ee5/node_modules npx --yes -p @playwright/test playwright test .omo/ulw-loop/evidence/frametrace-ui-ia/baseline/baseline.spec.js ` +
            `--browser=chromium --workers=1 --reporter=line --output=.omo/ulw-loop/evidence/frametrace-ui-ia/baseline/test-results; ` +
            `page.goto("${url}") with viewport ${viewport.width}x${viewport.height}; visibleText.includes("${concept}")`,
          verdict: pass ? 'PASS' : 'FAIL',
          artifactRefs: [`screenshot-${viewport.id}`, `visible-text-${viewport.id}`],
        });
      }

      const overflow = await page.evaluate(() => ({
        viewportWidth: window.innerWidth,
        clientWidth: document.documentElement.clientWidth,
        scrollWidth: document.documentElement.scrollWidth,
        bodyScrollWidth: document.body.scrollWidth,
        hasHorizontalOverflow: document.documentElement.scrollWidth > window.innerWidth,
      }));
      const overflowPath = path.join(evidenceDir, `${viewport.id}-overflow.json`);
      fs.writeFileSync(overflowPath, `${JSON.stringify(overflow, null, 2)}\n`);
      addArtifact(
        `overflow-${viewport.id}`,
        'json',
        `Horizontal overflow measurement at ${viewport.width}x${viewport.height}`,
        overflowPath,
      );

      if (viewport.width === 375 || viewport.width === 768) {
        matrix.adversarialCases.push({
          scenarioId: `${viewport.id}-horizontal-overflow`,
          criterionRef: `responsive:no-horizontal-overflow:${viewport.width}`,
          adversarialClass: 'narrow viewport responsive overflow',
          expectedBehavior: 'No horizontal overflow: document.documentElement.scrollWidth <= window.innerWidth',
          verdict: overflow.hasHorizontalOverflow ? 'FAIL' : 'PASS',
          artifactRefs: [`screenshot-${viewport.id}`, `overflow-${viewport.id}`],
        });
      }
    });
  }

  test.afterAll(() => {
    const matrixPath = path.join(evidenceDir, 'manualQa.json');
    fs.writeFileSync(matrixPath, `${JSON.stringify(matrix, null, 2)}\n`);
    addArtifact('manual-qa-json', 'json', 'manualQa matrix generated by browser baseline run', matrixPath);
    fs.writeFileSync(matrixPath, `${JSON.stringify(matrix, null, 2)}\n`);
  });
});
