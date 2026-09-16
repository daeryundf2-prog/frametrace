// Verify stage-3 fullscreen handoff: compact embed inline, full chrome in fullscreen.
const CDP = 'http://127.0.0.1:9222';
const BASE = 'http://127.0.0.1:8477';
const CASE = 'D:/frametrace-e01-test/case-synth';
const fs = await import('fs');
let nextId = 1;
const pending = new Map();
function connect(url) { return new Promise((res, rej) => { const w = new WebSocket(url); w.onopen = () => res(w); w.onerror = rej; }); }
function send(ws, method, params = {}, sessionId) {
  const id = nextId++;
  return new Promise((resolve, reject) => { pending.set(id, { resolve, reject }); ws.send(JSON.stringify({ id, method, params, sessionId })); });
}
function wire(ws) { ws.onmessage = e => { const m = JSON.parse(e.data); if (m.id && pending.has(m.id)) { const { resolve, reject } = pending.get(m.id); pending.delete(m.id); m.error ? reject(new Error(m.error.message)) : resolve(m.result); } }; }
async function ev(ws, sid, expr) {
  const r = await send(ws, 'Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true, userGesture: true }, sid);
  if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails.exception?.description || r.exceptionDetails.text));
  return r.result.value;
}
const sleep = ms => new Promise(r => setTimeout(r, ms));
const version = await (await fetch(CDP + '/json/version')).json();
const ws = await connect(version.webSocketDebuggerUrl); wire(ws);
const { targetId } = await send(ws, 'Target.createTarget', { url: BASE + '/' });
const { sessionId: sid } = await send(ws, 'Target.attachToTarget', { targetId, flatten: true });
await send(ws, 'Page.enable', {}, sid);
await sleep(2500);
const results = [];
const check = (n, ok, d = '') => { results.push(ok); console.log(`${ok ? 'PASS' : 'FAIL'}  ${n}${d ? '  — ' + d : ''}`); };
async function shot(name) { const r = await send(ws, 'Page.captureScreenshot', { format: 'png' }, sid); fs.mkdirSync('shots-user', { recursive: true }); fs.writeFileSync(`shots-user/${name}.png`, Buffer.from(r.data, 'base64')); console.log('  [shot]', name); }

await ev(ws, sid, `(() => { const el = document.getElementById('caseDir'); el.value = '${CASE}'; el.dispatchEvent(new Event('input')); document.getElementById('btnOpenCase').click(); return 1; })()`);
await sleep(4000);

// inline = embed mode
const inline = await ev(ws, sid, `(() => { const f = document.getElementById('reviewFrame');
  return { embed: f.contentWindow.document.body.classList.contains('embed'),
    headerHidden: !f.contentWindow.document.querySelector('header') || getComputedStyle(f.contentWindow.document.querySelector('header')).display === 'none' }; })()`);
check('inline iframe is embed mode (no header)', inline.embed && inline.headerHidden, JSON.stringify(inline));

// fullscreen → full chrome
await ev(ws, sid, `document.getElementById('btnViewerFull').click()`);
await sleep(1500);
const fs1 = await ev(ws, sid, `(() => { const f = document.getElementById('reviewFrame');
  return { fs: document.fullscreenElement === f,
    embed: f.contentWindow.document.body.classList.contains('embed'),
    headerShown: f.contentWindow.document.querySelector('header') && getComputedStyle(f.contentWindow.document.querySelector('header')).display !== 'none' }; })()`);
check('fullscreen shows full viewer chrome', fs1.fs && !fs1.embed && fs1.headerShown, JSON.stringify(fs1));
await shot('70-viewer-fullscreen');

// exit fullscreen → compact again
await ev(ws, sid, `document.exitFullscreen()`);
await sleep(1000);
const fs2 = await ev(ws, sid, `(() => { const f = document.getElementById('reviewFrame');
  return { fs: !!document.fullscreenElement, embed: f.contentWindow.document.body.classList.contains('embed') }; })()`);
check('exit fullscreen returns to compact', !fs2.fs && fs2.embed, JSON.stringify(fs2));

// re-enter fullscreen → floating close button appears, click exits
await ev(ws, sid, `document.getElementById('btnViewerFull').click()`);
await sleep(1200);
const fs3 = await ev(ws, sid, `(() => { const f = document.getElementById('reviewFrame');
  const b = f.contentWindow.document.getElementById('btnExitFs');
  return { fs: document.fullscreenElement === f, btnShown: b && !b.hidden }; })()`);
check('close button appears in fullscreen', fs3.fs && fs3.btnShown, JSON.stringify(fs3));
await ev(ws, sid, `(() => { document.getElementById('reviewFrame').contentWindow.document.getElementById('btnExitFs').click(); return 1; })()`);
await sleep(1000);
const fs4 = await ev(ws, sid, `(() => { const f = document.getElementById('reviewFrame');
  const b = f.contentWindow.document.getElementById('btnExitFs');
  return { fs: !!document.fullscreenElement, embed: f.contentWindow.document.body.classList.contains('embed'), btnHidden: b.hidden }; })()`);
check('close button exits fullscreen → compact', !fs4.fs && fs4.embed && fs4.btnHidden, JSON.stringify(fs4));

console.log(`\n${results.filter(Boolean).length}/${results.length} passed`);
ws.close(); process.exit(results.every(Boolean) ? 0 : 1);
