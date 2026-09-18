// audit-buttons.mjs — enumerate every button/select in the evidence viewer,
// verify each has a listener (or documented delegation), click it, and report
// observable effects (toast, DOM mutation, menu/modal open, popup, download,
// state change) or exceptions.
//
// Usage:
//   1. Start workstation server + open a case with a review viewer.
//   2. Start Edge/Chrome with --remote-debugging-port=9222 --headless=new.
//   3. node scripts/audit-buttons.mjs
const CDP = 'http://127.0.0.1:9222';
const VIEWER = 'http://127.0.0.1:8477/review/evidence-viewer.html';
const APP = 'http://127.0.0.1:8477/';
const sleep = ms => new Promise(r => setTimeout(r, ms));

let idc = 1; const pending = new Map(); const exceptions = [];
const ver = await (await fetch(CDP + '/json/version')).json();
const ws = new WebSocket(ver.webSocketDebuggerUrl);
await new Promise(r => ws.onopen = r);
ws.onmessage = e => {
  const m = JSON.parse(e.data);
  if (m.method === 'Runtime.exceptionThrown') exceptions.push(m.params.exceptionDetails?.exception?.description || m.params.exceptionDetails?.text);
  if (m.id && pending.has(m.id)) { const { res, rej } = pending.get(m.id); pending.delete(m.id); m.error ? rej(new Error(m.error.message)) : res(m.result); }
};
const send = (method, params = {}, sessionId) => { const i = idc++; return new Promise((res, rej) => { pending.set(i, { res, rej }); ws.send(JSON.stringify({ id: i, method, params, sessionId })); }); };
const ev = async (expr, s, gesture = false) => {
  const r = await send('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true, userGesture: gesture }, s);
  if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description || r.exceptionDetails.text);
  return r.result.value;
};
const openPage = async url => {
  const { targetId } = await send('Target.createTarget', { url });
  const { sessionId } = await send('Target.attachToTarget', { targetId, flatten: true });
  await send('Runtime.enable', {}, sessionId);
  return { targetId, sessionId };
};

// ---------- instrument the viewer ----------
const { sessionId: s } = await openPage(VIEWER);
await sleep(2800);

// install effect tracers once
await ev(`(() => {
  window.__audit = { toasts: [], opens: [], blobs: 0, mutations: 0 };
  const _toast = window.toast; window.toast = function(...a){ __audit.toasts.push(a.join(' ')); return _toast.apply(this,a); };
  const _open = window.open; window.open = function(...a){ __audit.opens.push(a[0]||''); return _open.apply(this,a); };
  const _blob = URL.createObjectURL; URL.createObjectURL = function(o){ __audit.blobs++; return _blob.call(URL,o); };
  new MutationObserver(m => __audit.mutations += m.length).observe(document.body, {subtree:true,childList:true,attributes:true,characterData:true});
  return 'armed';
})()`, s);

// prepare: activate first playable card + select it so selection-dependent
// buttons exercise their real path rather than the empty-selection guard
await ev(`(async()=>{
  const cards=[...document.querySelectorAll('#recordGrid .card')];
  const playable=cards.find(c=>{const d=records.find(r=>r.id===c.dataset.id);return d&&d.src});
  (playable||cards[0]).click();
  await new Promise(r=>setTimeout(r,300));
  document.body.dispatchEvent(new KeyboardEvent('keydown',{key:' ',bubbles:true}));
  await new Promise(r=>setTimeout(r,200));
  return document.getElementById('selectionCount').textContent;
})()`, s);

// enumerate all buttons + selects, get listener count via DOMDebugger
const ids = await ev(`[...document.querySelectorAll('button,select')].map(e=>({id:e.id||'(anon)',tag:e.tagName,cls:e.className.slice(0,30),txt:(e.textContent||'').trim().slice(0,20)}))`, s);
console.log(`controls: ${ids.length}`);

const results = [];
for (const c of ids) {
  if (!c.id || c.id === '(anon)') continue;
  // listener count
  const { result } = await send('Runtime.evaluate', { expression: `document.getElementById(${JSON.stringify(c.id)})`, returnByValue: false }, s);
  let listeners = [];
  try { listeners = (await send('DOMDebugger.getEventListeners', { objectId: result.objectId, depth: 0 }, s)).listeners; } catch {}
  // click with gesture, measure effects
  exceptions.length = 0;
  const before = await ev(`JSON.stringify({t:__audit.toasts.length,o:__audit.opens.length,b:__audit.blobs,m:__audit.mutations,menus:[...document.querySelectorAll('.menu-list:not([hidden])')].length,modal:!document.getElementById('shortcutModal')?.hidden,detail:!!document.querySelector('#summaryList').children.length})`, s);
  await ev(`document.getElementById(${JSON.stringify(c.id)}).click(); 'clicked'`, s, true).catch(e => exceptions.push('click:' + e.message.slice(0, 80)));
  await sleep(280);
  const after = await ev(`JSON.stringify({t:__audit.toasts.length,o:__audit.opens.length,b:__audit.blobs,m:__audit.mutations,menus:[...document.querySelectorAll('.menu-list:not([hidden])')].length,modal:!document.getElementById('shortcutModal')?.hidden})`, s);
  const b = JSON.parse(before), a = JSON.parse(after);
  const effects = [];
  const toastTxt = await ev(`__audit.toasts.slice(${b.t}).join(' | ')`, s);
  if (a.t > b.t) effects.push('toast');
  if (a.o > b.o) effects.push('popup');
  if (a.b > b.b) effects.push('download/blob');
  if (a.m > b.m) effects.push(`mut+${a.m - b.m}`);
  if (a.menus !== b.menus) effects.push('menu');
  if (a.modal !== b.modal) effects.push('modal');
  // close anything that opened so the next control starts clean
  await ev(`(()=>{document.querySelectorAll('.menu-list').forEach(l=>l.hidden=true); const m=document.getElementById('shortcutModal'); if(m&&!m.hidden)m.hidden=true; return 'cleaned'})()`, s);
  results.push({ id: c.id, txt: c.txt, listeners: listeners.map(l => l.type), effects, toast: (toastTxt || '').slice(0, 70), err: exceptions.slice() });
}
console.log('\n=== VIEWER RESULTS ===');
for (const r of results) {
  const flag = (r.listeners.length === 0 && r.effects.length === 0) ? '⚠ NO-LISTENER' : (r.err.length ? '✗ ERR' : '✓');
  console.log(`${flag} ${r.id} [${r.txt}] listen=[${r.listeners.join(',')}] fx=[${r.effects.join(',')}] ${r.toast}${r.err.length ? ' ERR=' + r.err.join(';') : ''}`);
}
const dead = results.filter(r => r.listeners.length === 0 && r.effects.length === 0);
console.log(`\nview-checks: ${results.length}, unresponsive: ${dead.length}`);
if (dead.length) console.log('DEAD: ' + dead.map(d => d.id).join(', '));

// ---------- workstation app sweep (listener existence only — buttons need real inputs) ----------
const { sessionId: sa } = await openPage(APP);
await sleep(2200);
const appIds = await ev(`[...document.querySelectorAll('button,select,input[type=file]')].map(e=>e.id||'(anon)')`, sa);
const appDead = [];
for (const cid of appIds) {
  if (cid === '(anon)') continue;
  const { result } = await send('Runtime.evaluate', { expression: `document.getElementById(${JSON.stringify(cid)})`, returnByValue: false }, sa);
  let listeners = [];
  try { listeners = (await send('DOMDebugger.getEventListeners', { objectId: result.objectId, depth: 0 }, sa)).listeners; } catch {}
  if (!listeners.length) appDead.push(cid);
}
console.log(`\napp controls: ${appIds.length}, no-listener: ${appDead.length}`);
if (appDead.length) console.log('NO-LISTENER: ' + appDead.join(', '));
process.exit(0);
