// PR #6 merge review track U: L3 assets/app/82-evidence-input.js
// normalizeTypedPath + handleEvidenceDrop in a VM with stubbed DOM / toast;
// M2 the viewer's recovery list / metric ignore extract-inode.
// Run from `node scripts/verify-review-batch.mjs --unit`.
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import assert from 'node:assert/strict';

const read = (rel) => readFileSync(new URL('../../' + rel, import.meta.url), 'utf8');

export async function pr6MergeTrackU() {
  const src = read('assets/app/82-evidence-input.js');
  const fn = (name) => src.match(new RegExp(`^(?:async )?function ${name}\\([^]*?^}`, 'm'))?.[0];
  const failed = [];
  const row = async (name, body) => {
    try { await body(); console.log('PASS VM ' + name); } catch (err) { failed.push(name); console.log('FAIL VM ' + name + ': ' + (err && err.message || err)); }
  };
  const make = (wsl = false) => {
    const calls = { set: [], toasts: [], picker: 0 };
    const ctx = {
      PICKER_PLUS: { wsl },
      setPathField: (id, value) => calls.set.push([id, value]),
      toast: (m, k) => calls.toasts.push([m, k]),
      $: () => ({ className: '', textContent: '', value: '' }),
      findDroppedPath: async () => '',
      openPicker: () => { calls.picker += 1; },
    };
    for (const name of ['normalizeTypedPath', 'handleEvidenceDrop']) {
      const f = fn(name);
      assert.ok(f, name + ' exists');
      runInNewContext(f, ctx);
    }
    return { ctx, calls, run: (expr) => runInNewContext(expr, ctx) };
  };
  const dt = (uri) => ({ getData: (type) => (type === 'text/uri-list' ? uri : ''), items: [], files: [] });

  await row('Merge review L3 file://server/share/x.E01 normalizes to //server/share/x.E01 (a network path), local file URIs stay local', () => {
    const v = make();
    const norm = (raw, wsl = false) => v.run(`normalizeTypedPath(${JSON.stringify(raw)}, ${wsl})`);
    assert.equal(norm('file://server/share/x.E01'), '//server/share/x.E01');
    assert.equal(norm('FILE://Server/Share%20A/x.E01'), '//Server/Share A/x.E01');
    assert.equal(norm('file://localhostevil/s/x.E01'), '//localhostevil/s/x.E01', 'a host merely starting with localhost is remote');
    assert.equal(norm('file:////server/share/x.E01'), '//server/share/x.E01', 'four-slash UNC form');
    assert.equal(norm('file:///C:/cases/x.E01'), 'C:\\cases\\x.E01');
    assert.equal(norm('file://localhost/C:/cases/x.E01'), 'C:\\cases\\x.E01');
    assert.equal(norm('file://LOCALHOST/C:/a%20b/x.E01'), 'C:\\a b\\x.E01');
    assert.equal(norm('file:///home/u/x.E01'), '/home/u/x.E01');
    assert.equal(norm('"C:\\cases\\x.E01"'), 'C:\\cases\\x.E01');
    assert.equal(norm('file:///C:/cases/x.E01', true), '/mnt/c/cases/x.E01');
    assert.equal(norm('file://server/share/x.E01', true), '//server/share/x.E01', 'WSL: never rewritten to a drive');
  });

  await row('Merge review L3 a dropped file://server/... URI is refused as a network path (never put in the field)', async () => {
    for (const uri of ['file://server/share/x.E01', 'file://srv.example/c$/x.E01', 'file:////server/share/x.E01', 'file://localhostx/y/x.E01']) {
      const v = make();
      v.ctx.__dt = dt(uri);
      await v.run('handleEvidenceDrop(__dt)');
      assert.deepEqual(v.calls.set, [], uri + ': nothing set');
      assert.equal(v.calls.toasts.length, 1, uri + ': one warning');
      assert.equal(v.calls.toasts[0][1], 'warn');
    }
    const ok = make();
    ok.ctx.__dt = dt('file:///C:/cases/x.E01');
    await ok.run('handleEvidenceDrop(__dt)');
    assert.deepEqual(JSON.parse(JSON.stringify(ok.calls.set)), [['sourcePath', 'C:\\cases\\x.E01']]);
  });

  // Merge review M2: an `extract-inode` (live file copied by
  // recover-batch --allocated-videos, indexed as a video by scan-folder)
  // never becomes a "Filesystem recovery" record nor counts in metricCarved.
  await row('Merge review M2 the viewer lists recover-inode, never extract-inode, as a filesystem recovery (Recovery tree, metricCarved)', () => {
    const viewer = read('assets/viewer/30-records.js');
    const decl = viewer.match(/^const recoveredFilesystemLog = currentLogEntries\([^]*?\);$/m)?.[0];
    assert.ok(decl, 'recoveredFilesystemLog declaration found');
    const extractedRe = viewer.match(/^const EXTRACTED_OUTPUT_RE = .*;$/m)?.[0];
    assert.ok(extractedRe, 'EXTRACTED_OUTPUT_RE declaration found');
    const details = read('assets/viewer/60-details.js');
    const metric = details.match(/^\s*els\.metricCarved\.textContent = .*$/m)?.[0];
    assert.ok(metric, 'metricCarved line found');
    const entry = (event, inode) => ({ event, inode, partition_offset: 2048, output_path: `/case/x/${inode}.mp4` });
    const ctx = {
      filesystemLog: [entry('extract-inode', '10'), entry('recover-inode', '11'), entry('extract-inode', '12'), { event: 'inspect-image-filesystem' }],
      logChains: null, normalizePath: (p) => p, currentLogEntries: (rows) => rows,
      carveLog: [], els: { metricCarved: { textContent: '' } },
      recoveryPreviewKeys: new Set(),
    };
    runInNewContext(extractedRe + '\n' + decl + '\n' + metric + '\nthis.__out = recoveredFilesystemLog;', ctx);
    assert.deepEqual(ctx.__out.map((r) => r.inode), ['11']);
    assert.equal(ctx.els.metricCarved.textContent, 1);
  });

  if (failed.length) throw new Error('PR #6 merge track U failed: ' + failed.join('; '));
}
