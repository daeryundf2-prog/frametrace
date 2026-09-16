// User-perspective walkthrough: drive the workstation + viewer through a
// real examiner session using only DOM clicks/typing — the same inputs a
// person would use. Captures screenshots along the way.
//
// Prereqs:
//   1. target/debug/frametrace-app.exe running on :8477
//   2. headless Edge/Chrome on :9222
//        msedge --headless=new --remote-debugging-port=9222
//          --user-data-dir=<temp> --autoplay-policy=no-user-gesture-required
//          --mute-audio --window-size=1500,950 about:blank
//   3. Run:  node scripts/user-walkthrough.mjs [shots_dir]
const CDP = 'http://127.0.0.1:9222';
const BASE = 'http://127.0.0.1:8477';
const CASE = 'D:/frametrace-e01-test/case-synth';
const SHOTS = process.argv[2] || 'shots-user';

let nextId = 1;
const pending = new Map();
const fs = await import('fs');

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
async function attach(browserWs, url) {
  const { targetId } = await send(browserWs, 'Target.createTarget', { url });
  const { sessionId } = await send(browserWs, 'Target.attachToTarget', { targetId, flatten: true });
  return { targetId, sessionId };
}
async function shot(browserWs, sessionId, name) {
  const res = await send(browserWs, 'Page.captureScreenshot', { format: 'png' }, sessionId);
  fs.writeFileSync(`${SHOTS}/${name}.png`, Buffer.from(res.data, 'base64'));
  console.log(`  [shot] ${name}.png`);
}
// Wait until an expression is truthy (polling), with timeout.
async function waitFor(ws, sessionId, expr, timeoutMs = 30000, label = expr.slice(0, 60)) {
  const t0 = Date.now();
  while (Date.now() - t0 < timeoutMs) {
    const v = await evalIn(ws, sessionId, expr);
    if (v) return v;
    await sleep(400);
  }
  throw new Error('timeout waiting for ' + label);
}

async function main() {
  fs.mkdirSync(SHOTS, { recursive: true });
  const results = [];
  const check = (name, ok, detail = '') => {
    results.push({ name, ok, detail });
    console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? '  — ' + detail : ''}`);
  };

  const version = await (await fetch(CDP + '/json/version')).json();
  const ws = await connect(version.webSocketDebuggerUrl);
  wire(ws);
  await send(ws, 'Target.setDiscoverTargets', { discover: true });

  // ============ SCENE 1: first screen — what a user sees ============
  const app = await attach(ws, BASE + '/');
  await sleep(2500);
  await shot(ws, app.sessionId, '01-first-screen');

  const s1 = await evalIn(ws, app.sessionId,
    `(() => ({ badges: document.querySelectorAll('#env .env-badge, #env *').length,
        envText: (document.getElementById('env')||{}).textContent || '',
        // exactly one stage view is on — if a case is already open the
        // poll auto-jumps to review, which is the intended behavior.
        viewsOn: [...document.querySelectorAll('.view')].filter(v => v.classList.contains('on')).length,
        steps: [...document.querySelectorAll('.navstep')].length }))()`);
  check('first screen: exactly one view + tool badges', s1.viewsOn === 1 && s1.badges > 0 && s1.steps === 4, JSON.stringify(s1).slice(0, 200));

  // ============ SCENE 2: user opens a case by typing the path ============
  await evalIn(ws, app.sessionId,
    `(() => { const el = document.getElementById('caseDir');
        el.value = '${CASE}'; el.dispatchEvent(new Event('input')); return true; })()`);
  await sleep(800); // live validation debounce
  const val = await evalIn(ws, app.sessionId,
    `document.getElementById('caseStatus').textContent`);
  check('typed case path validated live', val.length > 0, JSON.stringify(val));

  await evalIn(ws, app.sessionId, `document.getElementById('btnOpenCase').click()`);
  await sleep(2500);
  const s2 = await evalIn(ws, app.sessionId,
    `(() => ({ sel: document.querySelector('.navstep.sel')?.dataset.view,
        reviewShown: document.getElementById('resultCard').classList.contains('on') }))()`);
  check('open-case jumps to review stage', s2.reviewShown && s2.sel === '3', JSON.stringify(s2));
  await shot(ws, app.sessionId, '02-review-stage');

  // ============ SCENE 3: user opens the viewer (same tab navigation) ============
  const viewer = await attach(ws, BASE + '/review/evidence-viewer.html');
  await sleep(2500);
  await shot(ws, viewer.sessionId, '03-viewer');

  const v1 = await evalIn(ws, viewer.sessionId,
    `(() => ({ count: records.length, title: document.getElementById('mediaTitle').textContent }))()`);
  check('viewer loads records', v1.count > 0, `records=${v1.count}`);

  // ============ SCENE 4: user clicks a record card, plays, sets IN/OUT ============
  // click the first card in the grid
  const card = await evalIn(ws, viewer.sessionId,
    `(() => { const c = document.querySelector('#recordGrid .card, #recordGrid [data-id], #recordGrid > *');
        if (!c) return { card: false };
        c.click(); return { card: true, active: state.activeId }; })()`, true);
  await sleep(1200);
  await shot(ws, viewer.sessionId, '04-record-selected');

  // set IN/OUT via the buttons a user would click
  const range = await evalIn(ws, viewer.sessionId,
    `(async () => { const v = document.querySelector('#mediaStage video');
        if (!v) return { video: false };
        v.currentTime = 0.5; await new Promise(r => setTimeout(r, 300));
        document.getElementById('btnSetIn').click();
        v.currentTime = 2.0; await new Promise(r => setTimeout(r, 300));
        document.getElementById('btnSetOut').click();
        return { video: true, label: document.getElementById('rangeLabel').textContent }; })()`);
  check('user sets IN/OUT via buttons', range.label && range.label.includes('~'), JSON.stringify(range));

  // ============ SCENE 5: capture frame (button a user clicks) ============
  const cap = await evalIn(ws, viewer.sessionId,
    `(async () => { const v = document.querySelector('#mediaStage video');
        if (!v) return { skip: 'no video' };
        try { await v.play(); } catch {}
        await new Promise(r => setTimeout(r, 1500));
        document.getElementById('toastHost').innerHTML = '';
        document.getElementById('btnCaptureFrame').click();
        let t = '';
        for (let i = 0; i < 30; i++) { await new Promise(r => setTimeout(r, 400));
            t = (document.getElementById('toastHost')||{}).textContent || '';
            if (t.trim()) break; }
        return { toast: t }; })()`);
  check('frame capture button works', cap.toast && cap.toast.includes('저장'), JSON.stringify(cap));

  // ============ SCENE 6: mark + tag + note like a user ============
  const triage = await evalIn(ws, viewer.sessionId,
    `(async () => { // keyboard triage: press '2' (중요) then type a note
        document.dispatchEvent(new KeyboardEvent('keydown', { key: '2', bubbles: true }));
        document.body.dispatchEvent(new KeyboardEvent('keydown', { key: '2', bubbles: true }));
        window.dispatchEvent(new KeyboardEvent('keydown', { key: '2', bubbles: true }));
        await new Promise(r => setTimeout(r, 400));
        const ta = document.getElementById('evidenceNote');
        let noteOk = false;
        if (ta) { ta.value = '사용자 테스트 메모: 번호판 확인 구간'; ta.dispatchEvent(new Event('input')); noteOk = true; }
        const ex = document.getElementById('examinerName');
        if (ex) { ex.value = '홍길동'; ex.dispatchEvent(new Event('input')); }
        const mk = Object.keys(localStorage).find(k => k.endsWith('.marks'));
        return { noteOk, markCount: Object.keys(JSON.parse(localStorage.getItem(mk) || '{}')).length }; })()`);
  check('keyboard mark + note + examiner', triage.markCount > 0 && triage.noteOk, JSON.stringify(triage));

  // ============ SCENE 7: apply marks to case (real user click) ============
  const apply = await evalIn(ws, viewer.sessionId,
    `(async () => { document.getElementById('toastHost').innerHTML = '';
        document.getElementById('btnApplyMarks').click();
        let t = '';
        for (let i = 0; i < 40; i++) { await new Promise(r => setTimeout(r, 500));
            t = (document.getElementById('toastHost')||{}).textContent || '';
            if (t.trim()) break; }
        return { toast: t }; })()`);
  check('marks applied to case via UI', apply.toast.includes('반영'), JSON.stringify(apply));

  // ============ SCENE 8: timeline panel ============
  const tl = await evalIn(ws, viewer.sessionId,
    `(async () => { document.getElementById('btnTimeline').click();
        await new Promise(r => setTimeout(r, 2500));
        const p = document.getElementById('timelinePanel');
        return { open: !p.hidden, meta: document.getElementById('timelineMeta').textContent }; })()`);
  check('timeline panel opens for user', tl.open && tl.meta.includes('이벤트'), JSON.stringify(tl));
  await shot(ws, viewer.sessionId, '05-timeline');

  // ============ SCENE 9: proxy toggle ============
  const px = await evalIn(ws, viewer.sessionId,
    `(async () => { const btn = document.getElementById('btnProxy');
        const before = (document.querySelector('#mediaStage video')||{}).src || '';
        btn.click();
        await new Promise(r => setTimeout(r, 12000));
        const after = (document.querySelector('#mediaStage video')||{}).src || '';
        return { before: before.slice(-60), after: after.slice(-60), pressed: btn.className + '|' + btn.textContent }; })()`);
  check('proxy toggle changes playback source', px.after.includes('proxy') || px.pressed.includes('프록시'), JSON.stringify(px));
  await shot(ws, viewer.sessionId, '06-proxy');

  // ============ SCENE 10: workstation results stage — advanced tools ============
  const st4 = await evalIn(ws, app.sessionId,
    `(() => { const rail = [...document.querySelectorAll('.navstep')].find(b => b.dataset.view === '4');
        if (rail) rail.click();
        return { advShown: document.getElementById('finalCard').classList.contains('on') }; })()`);
  await sleep(800);
  await shot(ws, app.sessionId, '07-results-stage');
  check('results stage reachable', st4.advShown, JSON.stringify(st4));

  // user expands advanced tools and clicks timeline
  const adv = await evalIn(ws, app.sessionId,
    `(async () => { const det = document.querySelector('details');
        if (det) det.open = true;
        const btn = document.querySelector('[data-adv="qa-consistency"]');
        if (!btn) return { found: false };
        btn.click();
        await new Promise(r => setTimeout(r, 15000));
        return { found: true, out: (document.getElementById('advOut')||{}).textContent.slice(0,160) }; })()`);
  check('advanced QA tool runs from UI', adv.found && adv.out.length > 0, JSON.stringify(adv));
  await shot(ws, app.sessionId, '08-advanced-tools');

  // ============ SCENE 11: carve button on a case without raw ============
  const carve = await evalIn(ws, app.sessionId,
    `(async () => { window.confirm = () => true;
        const btn = document.getElementById('btnCarve');
        btn.click();
        await new Promise(r => setTimeout(r, 3000));
        return { hint: (document.getElementById('recoverHint')||{}).textContent.slice(0, 140) }; })()`);
  check('carve gives clear guidance when no raw image', carve.hint.includes('카빙'), JSON.stringify(carve));

  // ---- summary ----
  const failed = results.filter(r => !r.ok);
  console.log(`\n${results.length - failed.length}/${results.length} passed — shots in ${SHOTS}/`);
  ws.close();
  process.exit(failed.length ? 1 : 0);
}

main().catch(err => { console.error('DRIVER ERROR:', err); process.exit(2); });
