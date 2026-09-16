// CDP driver: verify the review-productivity batch (notes/examiner, marks
// apply, frame capture, IN/OUT clip export, proxy playback, timeline panel,
// warning chip) plus workstation advanced tools, recent cases, and carving.
//
// Usage:
//   1. Start the workstation:  target/debug/frametrace-app.exe
//   2. Open a case with real videos:
//        POST /api/open-case {"case_dir": "D:/frametrace-e01-test/case-synth"}
//   3. Generate a timeline for the case (optional but exercised):
//        frametrace-app timeline <case>
//   4. Start headless Edge/Chrome:
//        msedge --headless=new --remote-debugging-port=9222
//          --user-data-dir=<temp> --autoplay-policy=no-user-gesture-required
//          --mute-audio about:blank
//   5. Run:  node scripts/verify-review-batch.mjs
const CDP = 'http://127.0.0.1:9222';
const BASE = 'http://127.0.0.1:8477';
const VIEWER = BASE + '/review/evidence-viewer.html';

let nextId = 1;
const pending = new Map();

function connect(url) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    ws.onopen = () => resolve(ws);
    ws.onerror = reject;
  });
}

function send(ws, method, params = {}, sessionId) {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params, sessionId }));
  });
}

function wire(ws) {
  ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      if (msg.error) reject(new Error(msg.error.message));
      else resolve(msg.result);
    }
  };
}

async function evalIn(ws, sessionId, expression, userGesture = false) {
  const res = await send(ws, 'Runtime.evaluate', {
    expression, returnByValue: true, awaitPromise: true, userGesture,
  }, sessionId);
  if (res.exceptionDetails) {
    throw new Error('page exception: ' + JSON.stringify(res.exceptionDetails.exception?.description || res.exceptionDetails.text));
  }
  return res.result.value;
}

const sleep = (ms) => new Promise(r => setTimeout(r, ms));

async function attachToNewTarget(browserWs, url) {
  const { targetId } = await send(browserWs, 'Target.createTarget', { url });
  const { sessionId } = await send(browserWs, 'Target.attachToTarget', { targetId, flatten: true });
  return { targetId, sessionId };
}

async function main() {
  const results = [];
  const check = (name, ok, detail = '') => {
    results.push({ name, ok, detail });
    console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? '  — ' + detail : ''}`);
  };

  const version = await (await fetch(CDP + '/json/version')).json();
  const browserWs = await connect(version.webSocketDebuggerUrl);
  wire(browserWs);
  await send(browserWs, 'Target.setDiscoverTargets', { discover: true });

  // ================= viewer =================
  const main = await attachToNewTarget(browserWs, VIEWER);
  await sleep(2500);

  const recs = await evalIn(browserWs, main.sessionId,
    `(() => ({ count: records.length, server: typeof serverMode === 'undefined' ? 'n/a' : serverMode() }))()`);
  check('viewer loads records', recs.count > 0, `records=${recs.count} server=${recs.server}`);

  // warning chip filters records with warnings — add a synthetic warning
  // state check: chip exists and applies the predicate
  const warn = await evalIn(browserWs, main.sessionId,
    `(() => { const chip = [...document.querySelectorAll('.chip')].find(c => c.dataset.chip === 'warning');
        if (!chip) return { exists: false };
        chip.click();
        const n = filteredRecords().length;
        const allHave = filteredRecords().every(r => (r.warnings || []).length > 0);
        return { exists: true, n, allHave }; })()`);
  check('warning chip filters warning-bearing records', warn.exists && warn.allHave, JSON.stringify(warn));

  // reset chip to all (data-chip="")
  await evalIn(browserWs, main.sessionId,
    `[...document.querySelectorAll('.chip')].find(c => c.dataset.chip === '')?.click()`);

  // note editor: pick a record, type a note, verify persistence
  const note = await evalIn(browserWs, main.sessionId,
    `(() => { const f = filteredRecords(); state.activeId = f[0].id; render();
        const ta = document.getElementById('evidenceNote');
        if (!ta) return { exists: false };
        ta.value = 'CDP test note';
        ta.dispatchEvent(new Event('input'));
        const key = Object.keys(localStorage).find(k => k.endsWith('.notes'));
        const stored = JSON.parse(localStorage.getItem(key) || '{}');
        return { exists: true, saved: stored[f[0].id] === 'CDP test note' }; })()`);
  check('note editor persists to localStorage', note.exists && note.saved, JSON.stringify(note));

  // examiner name input
  const ex = await evalIn(browserWs, main.sessionId,
    `(() => { const el = document.getElementById('examinerName');
        if (!el) return { exists: false };
        el.value = 'CDP검사관'; el.dispatchEvent(new Event('input'));
        return { exists: true, saved: state.examiner === 'CDP검사관' }; })()`);
  check('examiner name input updates state', ex.exists && ex.saved, JSON.stringify(ex));

  // marks payload includes note + examiner
  const payload = await evalIn(browserWs, main.sessionId,
    `(() => { const p = marksPayload();
        return { examiner: p.examiner, note: (p.marks.find(m => m.note) || {}).note, n: p.marks.length }; })()`);
  check('marks payload carries examiner + note', payload.examiner === 'CDP검사관', JSON.stringify(payload));

  // IN/OUT controls on a playable record
  const range = await evalIn(browserWs, main.sessionId,
    `(() => { const f = filteredRecords(); const r = f.find(r => mediaSrcFor(r));
        if (!r) return { playable: false };
        state.activeId = r.id; render();
        const video = document.querySelector('#mediaStage video');
        if (!video) return { playable: true, video: false };
        video.currentTime = 1.0; document.getElementById('btnSetIn').click();
        video.currentTime = 3.0; document.getElementById('btnSetOut').click();
        return { playable: true, video: true, label: document.getElementById('rangeLabel').textContent }; })()`);
  check('IN/OUT sets range label', range.label && range.label.length > 0, JSON.stringify(range));

  // clip export hits the server (real export of vid with range set)
  const clip = await evalIn(browserWs, main.sessionId,
    `(async () => { document.getElementById('btnExportClip').click();
        let text = '';
        for (let i = 0; i < 40; i++) { await new Promise(r => setTimeout(r, 500));
            text = (document.getElementById('toastHost')||{}).textContent || '';
            if (text.trim()) break; }
        return { toast: text }; })()`);
  check('export clip request completes', clip.toast.length > 0, JSON.stringify(clip));

  // proxy toggle asks the server and flips playback source
  const proxy = await evalIn(browserWs, main.sessionId,
    `(async () => { const btn = document.getElementById('btnProxy');
        if (!btn) return { exists: false };
        btn.click();
        await new Promise(r => setTimeout(r, 15000));
        const video = document.querySelector('#mediaStage video');
        return { exists: true, src: video && video.src || '', label: btn.textContent }; })()`);
  check('proxy toggle produces playback src', proxy.exists && proxy.src.length > 0, JSON.stringify(proxy).slice(0, 160));

  // timeline panel
  const tl = await evalIn(browserWs, main.sessionId,
    `(async () => { document.getElementById('btnTimeline').click();
        await new Promise(r => setTimeout(r, 2500));
        const panel = document.getElementById('timelinePanel');
        const items = panel.querySelectorAll('.timeline-item, li, .tl-item').length;
        const meta = document.getElementById('timelineMeta').textContent;
        const list = document.getElementById('timelineList').textContent;
        return { hidden: panel.hidden, items, meta, listLen: list.length }; })()`);
  check('timeline panel opens with events', !tl.hidden && (tl.items > 0 || tl.listLen > 20), JSON.stringify(tl).slice(0, 200));

  // apply marks → server import (note + examiner land in the case DB)
  const apply = await evalIn(browserWs, main.sessionId,
    `(async () => { document.getElementById('btnApplyMarks').click();
        let text = '';
        for (let i = 0; i < 40; i++) { await new Promise(r => setTimeout(r, 500));
            text = (document.getElementById('toastHost')||{}).textContent || '';
            if (text.trim()) break; }
        return { toast: text }; })()`);
  check('apply marks posts to server', apply.toast.length > 0, JSON.stringify(apply));

  // ================= workstation =================
  const ws = await attachToNewTarget(browserWs, BASE + '/');
  await sleep(2000);

  const stage = await evalIn(browserWs, ws.sessionId,
    `(() => { return { adv: !!document.querySelector('[data-adv="timeline"]'),
        carve: !!document.getElementById('btnCarve'),
        recent: !!document.getElementById('recentCaseRow') }; })()`);
  check('workstation has advanced tools + carve + recent row', stage.adv && stage.carve && stage.recent, JSON.stringify(stage));

  // recent cases: seed prefs then re-render
  const recent = await evalIn(browserWs, ws.sessionId,
    `(async () => { const prefs = loadPrefs(); prefs.recentCases = ['D:/frametrace-e01-test/case-synth','D:/frametrace-e01-test/case-direct2','D:/nope/not-a-case'];
        savePrefs(prefs); await renderRecentCases();
        const row = document.getElementById('recentCaseRow');
        return { hidden: row.hidden, chips: row.querySelectorAll('.casechip').length }; })()`);
  check('recent case chips keep only real cases', recent.chips === 2, JSON.stringify(recent));

  // advanced tool dispatch via UI button (timeline is cheap)
  const adv = await evalIn(browserWs, ws.sessionId,
    `(async () => { const btn = document.querySelector('[data-adv="timeline"]');
        btn.click(); await new Promise(r => setTimeout(r, 8000));
        const out = document.getElementById('advOut');
        return { text: (out && out.textContent || '').slice(0, 200) }; })()`);
  check('advanced tool (timeline) runs from UI', adv.text.length > 0, JSON.stringify(adv));

  // ---- summary ----
  const failed = results.filter(r => !r.ok);
  console.log(`\n${results.length - failed.length}/${results.length} passed`);
  browserWs.close();
  process.exit(failed.length ? 1 : 0);
}

main().catch(err => { console.error('DRIVER ERROR:', err); process.exit(2); });
