// Offline test of scripts/make-portable.ps1 (docs/PLAN_TOOLS_C_2026-10-20.md
// P5): a fake repo root with fake binaries and a stub `cargo` (cargo.cmd →
// node) first on PATH, no real build, no network.
// Fix round 5: a stale FFmpeg zip in dist/ and SHA256SUMS listing only this
// run's zip (L4), the "Owner to confirm" release guard (M2).
// Fix round 6: renamed FFmpeg files found by signature strings in their bytes
// under any extension (one match straddles the 4 MiB chunk boundary) while
// our own docs (the real README.md / WINDOWS_USAGE.md, prose that says
// "ffmpeg") still pass (L3b); an -AllowUnconfirmedNotices package is named
// *-UNCONFIRMED.zip with SHA256SUMS.unconfirmed and no SHA256SUMS, and the
// marker is found across a line break (L-CI).
// Fix round 11 M2: tools/manifest.json's notice_confirmed guard.
// Fix round 16: no third-party tool is bundled — any file in tools/bin or in
// the release directory's tools/bin fails the run (M2); the binaries come
// from cargo's real target directory (CARGO_TARGET_DIR,
// CARGO_BUILD_TARGET_DIR, .cargo/config.toml build.target-dir; build JSON
// or cargo metadata) (M4); archives / packed executables are refused by
// magic bytes (L-archives); exact-key notice_confirmed (L-keycase);
// UNCONFIRMED.txt inside an unconfirmed package (L-marker); the manifest
// compiled into the exes must be the working tree's (hunch); pip tools with
// a requirements file need a confirmed notice too (hunch).
// Windows only (needs Windows PowerShell).
//
//   node scripts/test-make-portable.mjs
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  mkdtempSync, mkdirSync, writeFileSync, readFileSync, copyFileSync, existsSync, rmSync, unlinkSync, appendFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

if (process.platform !== 'win32') {
  console.log('test-make-portable: SKIPPED (Windows only)');
  process.exit(0);
}

const repo = join(dirname(fileURLToPath(import.meta.url)), '..');
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

// Entry names from a zip's central directory (any zip, deflated or not).
function zipNames(buf) {
  let eocd = buf.length - 22;
  while (eocd >= 0 && buf.readUInt32LE(eocd) !== 0x06054b50) eocd--;
  if (eocd < 0) throw new Error('no end of central directory');
  const count = buf.readUInt16LE(eocd + 10);
  let at = buf.readUInt32LE(eocd + 16);
  const names = [];
  for (let i = 0; i < count; i++) {
    if (buf.readUInt32LE(at) !== 0x02014b50) throw new Error('bad central directory');
    const n = buf.readUInt16LE(at + 28); const x = buf.readUInt16LE(at + 30); const c = buf.readUInt16LE(at + 32);
    names.push(buf.subarray(at + 46, at + 46 + n).toString('utf8'));
    at += 46 + n + x + c;
  }
  return names;
}

// A minimal stored (uncompressed) zip of `{ name: content }`.
const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc32 = (buf) => {
  let c = 0xffffffff;
  for (const b of buf) c = CRC_TABLE[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
function makeZip(files) {
  const locals = []; const centrals = []; let offset = 0;
  for (const [name, content] of Object.entries(files)) {
    const n = Buffer.from(name, 'utf8'); const data = Buffer.from(content);
    const crc = crc32(data);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0); local.writeUInt16LE(20, 4); local.writeUInt16LE(0x0800, 6);
    local.writeUInt32LE(crc, 14); local.writeUInt32LE(data.length, 18); local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(n.length, 26);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0); central.writeUInt16LE(20, 4); central.writeUInt16LE(20, 6); central.writeUInt16LE(0x0800, 8);
    central.writeUInt32LE(crc, 16); central.writeUInt32LE(data.length, 20); central.writeUInt32LE(data.length, 24);
    central.writeUInt16LE(n.length, 28); central.writeUInt32LE(offset, 42);
    locals.push(local, n, data); centrals.push(central, n);
    offset += 30 + n.length + data.length;
  }
  const cd = Buffer.concat(centrals);
  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0); eocd.writeUInt16LE(Object.keys(files).length, 8); eocd.writeUInt16LE(Object.keys(files).length, 10);
  eocd.writeUInt32LE(cd.length, 12); eocd.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, cd, eocd]);
}

// A .NET library whose PE version information names FFmpeg (C# compiles the
// assembly attributes into the Win32 VERSIONINFO) — a renamed FFmpeg binary.
function ffmpegVersionInfoDll(out) {
  const src = [
    '[assembly: System.Reflection.AssemblyProduct("FFmpeg")]',
    '[assembly: System.Reflection.AssemblyCompany("FFmpeg Project")]',
    '[assembly: System.Reflection.AssemblyTitle("FFmpeg codec library")]',
    'public class FtFakeRenamedTool { }',
  ].join('\n');
  const ps = `$src = @'\n${src}\n'@\nAdd-Type -TypeDefinition $src -OutputAssembly '${out}' -OutputType Library`;
  const env = { ...process.env };
  for (const k of Object.keys(env)) if (k.toLowerCase() === 'psmodulepath') delete env[k];
  const r = spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', ps], { env, encoding: 'utf8' });
  if (r.status !== 0 || !existsSync(out)) throw new Error(`cannot build the version-info fixture: ${r.stdout}${r.stderr}`);
}

// `ffmpeg.EXE ` has a real trailing space (written through the \\?\
// namespace, which keeps it).
const TRAILING = 'ffmpeg.EXE ';
const longPath = (p) => '\\\\?\\' + p.replace(/\//g, '\\');

// --- stub cargo -----------------------------------------------------------
// `cargo metadata` prints target_directory resolved like cargo:
// CARGO_TARGET_DIR, else CARGO_BUILD_TARGET_DIR (relative = to the cwd), else
// .cargo/config.toml [build] target-dir (relative = to the folder holding
// .cargo), else <cwd>/target. `cargo build` (needs
// --message-format=json-render-diagnostics) writes fresh exes holding the
// working tree's manifest there (FT_CARGO_NOWRITE=1: writes nothing) and
// prints compiler-artifact lines (FT_CARGO_OMIT=<bin>: leaves that bin out).
const STUBS = mkdtempSync(join(tmpdir(), 'ft-portable-stubs-'));
writeFileSync(join(STUBS, 'cargo-stub.mjs'), `
import fs from 'node:fs';
import path from 'node:path';
const args = process.argv.slice(2);
const cwd = process.cwd();
if (process.env.FT_CARGO_LOG) fs.appendFileSync(process.env.FT_CARGO_LOG, args.join(' ') + '\\n');
function targetDir() {
  const e = process.env.CARGO_TARGET_DIR || process.env.CARGO_BUILD_TARGET_DIR;
  if (e) return path.resolve(cwd, e);
  const cfg = path.join(cwd, '.cargo', 'config.toml');
  if (fs.existsSync(cfg)) {
    const m = /^\\s*target-dir\\s*=\\s*"([^"]*)"/m.exec(fs.readFileSync(cfg, 'utf8'));
    if (m) return path.resolve(cwd, m[1]);
  }
  return path.join(cwd, 'target');
}
if (args[0] === 'metadata') {
  console.log(JSON.stringify({ packages: [{ name: 'frametrace', version: '0.0.0-stub' }], target_directory: targetDir(), workspace_root: cwd, version: 1 }));
  process.exit(0);
}
if (args[0] === 'build') {
  if (!args.includes('--message-format=json-render-diagnostics')) { console.error('stub cargo: expected --message-format=json-render-diagnostics'); process.exit(2); }
  const rel = path.join(targetDir(), 'release');
  if (process.env.FT_CARGO_NOWRITE !== '1') {
    fs.mkdirSync(rel, { recursive: true });
    const mf = path.join(cwd, 'tools', 'manifest.json');
    const manifest = fs.existsSync(mf) ? fs.readFileSync(mf) : Buffer.alloc(0);
    for (const bin of ['frametrace', 'frametrace-app']) fs.writeFileSync(path.join(rel, bin + '.exe'), Buffer.concat([Buffer.from('MZ fresh-from-cargo\\n'), manifest]));
  }
  console.log(JSON.stringify({ reason: 'compiler-artifact', target: { name: 'frametrace', kind: ['lib'] }, executable: null }));
  const omit = (process.env.FT_CARGO_OMIT || '').split(',');
  for (const bin of ['frametrace', 'frametrace-app']) {
    if (omit.includes(bin)) continue;
    console.log(JSON.stringify({ reason: 'compiler-artifact', target: { name: bin, kind: ['bin'] }, executable: path.join(rel, bin + '.exe'), fresh: true }));
  }
  console.log(JSON.stringify({ reason: 'build-finished', success: true }));
  process.exit(0);
}
console.error('stub cargo: unexpected ' + args.join(' '));
process.exit(1);
`);
writeFileSync(join(STUBS, 'cargo.cmd'), '@node "%~dp0cargo-stub.mjs" %*\r\n@exit /b %ERRORLEVEL%\r\n');

// tools/manifest.json of a fake repo (Fix round 11 M2): one installable tool
// (sha256 pinned) whose notice is confirmed unless `noticeConfirmed` says
// otherwise (`'omit'` leaves the key out), one manual tool (sha256 null) that
// never needs a confirmation, and `extra` tool entries.
const fakeManifest = (noticeConfirmed = true, extra = []) => JSON.stringify({
  schema: 1,
  tools: [
    { name: 'ffmpeg', kind: 'archive', sha256: 'a'.repeat(64), notice: 'n', ...(noticeConfirmed === 'omit' ? {} : { notice_confirmed: noticeConfirmed }) },
    { name: 'libewf', kind: 'archive', sha256: null, notice: null },
    ...extra,
  ],
}, null, 2);

// A fake exe: `head` bytes followed by the manifest (include_str!).
const fakeExe = (head, manifest) => Buffer.concat([Buffer.from(head), Buffer.from(manifest === null ? '' : manifest)]);

function fakeRepo({ notices = null, manifest = fakeManifest(), exeManifest } = {}) {
  const root = mkdtempSync(join(tmpdir(), 'ft-portable-'));
  for (const d of ['scripts', 'target/release', 'docs', 'tools']) mkdirSync(join(root, d), { recursive: true });
  if (manifest !== null) writeFileSync(join(root, 'tools/manifest.json'), manifest);
  copyFileSync(join(repo, 'scripts/make-portable.ps1'), join(root, 'scripts/make-portable.ps1'));
  const embedded = exeManifest === undefined ? manifest : exeManifest;
  for (const f of ['frametrace.exe', 'frametrace-app.exe']) writeFileSync(join(root, 'target/release', f), fakeExe('MZ stale-target-release\n', embedded));
  for (const f of ['LICENSE', 'THIRD_PARTY_NOTICES.md']) writeFileSync(join(root, f), `fake ${f}\n`);
  if (notices !== null) writeFileSync(join(root, 'THIRD_PARTY_NOTICES.md'), notices);
  // Our real docs: they mention ffmpeg in prose ("FFmpeg version text") and
  // must pass the signature scan.
  copyFileSync(join(repo, 'README.md'), join(root, 'README.md'));
  copyFileSync(join(repo, 'docs/WINDOWS_USAGE.md'), join(root, 'docs/WINDOWS_USAGE.md'));
  writeFileSync(join(root, 'scripts/install.ps1'), '﻿# fake\n');
  return root;
}

// Removes a fake repo (the trailing-space file only goes through \\?\).
function removeRoot(root) {
  for (const toolsDir of ['tools/bin', 'target/release/tools/bin']) {
    try { unlinkSync(longPath(join(root, toolsDir, TRAILING))); } catch { /* not there */ }
  }
  rmSync(root, { recursive: true, force: true });
}

function runPortable(root, { version = '0.0.0-test', extra = [], skipBuild = true, env: extraEnv = {} } = {}) {
  const env = { ...process.env };
  for (const k of ['HTTP_PROXY', 'HTTPS_PROXY', 'ALL_PROXY', 'http_proxy', 'https_proxy', 'all_proxy',
    'CARGO_TARGET_DIR', 'CARGO_BUILD_TARGET_DIR', 'CARGO_BUILD_TARGET', 'FT_CARGO_NOWRITE', 'FT_CARGO_OMIT']) delete env[k];
  // A pwsh-7 parent (GitHub's `shell: pwsh`) leaves a PSModulePath that Windows
  // PowerShell 5.1 cannot auto-load modules from; unset it so 5.1 rebuilds its own.
  let pathKey = 'PATH';
  for (const k of Object.keys(env)) {
    if (k.toLowerCase() === 'psmodulepath') delete env[k];
    else if (k.toLowerCase() === 'path') pathKey = k;
  }
  env[pathKey] = `${STUBS};${env[pathKey] || ''}`;
  const log = join(root, 'cargo-calls.log');
  Object.assign(env, { FT_CARGO_LOG: log }, extraEnv);
  const args = ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', join(root, 'scripts/make-portable.ps1')];
  if (skipBuild) args.push('-SkipBuild');
  if (version !== null) args.push('-Version', version);
  args.push(...extra);
  return new Promise((ok) => {
    const child = spawn('powershell.exe', args, { cwd: root, env });
    let out = '';
    child.stdout.on('data', (d) => { out += d; });
    child.stderr.on('data', (d) => { out += d; });
    // Windows PowerShell wraps an error message at the console width (mid
    // word, the break depending on the temp path's length): the checks match
    // the output with its line breaks removed.
    child.on('close', (code) => ok({ code, out: out.replace(/\r?\n/g, ''), calls: existsSync(log) ? readFileSync(log, 'utf8') : '' }));
  });
}

let failures = 0;
let passes = 0;
const check = (cond, what, detail = '') => {
  if (!cond) { failures++; console.error(`FAIL ${what}${detail ? `\n${detail}` : ''}`); } else { passes++; console.log(`ok   ${what}`); }
};

const PREFIX = 'FrameTrace-0.0.0-test-win64/';
const ZIP = 'dist/FrameTrace-0.0.0-test-win64.zip';
const UZIP = 'dist/FrameTrace-0.0.0-test-win64-UNCONFIRMED.zip';
const STAGED_EXE = 'dist/FrameTrace-0.0.0-test-win64/frametrace.exe';
const roots = [];
const newRepo = (opts) => { const r = fakeRepo(opts); roots.push(r); return r; };
try {
  // --- a clean package ------------------------------------------------------
  {
    const root = newRepo();
    // L4: a clean zip left in dist/ by something else is not vouched for.
    mkdirSync(join(root, 'dist'), { recursive: true });
    writeFileSync(join(root, 'dist/old-notes.zip'), makeZip({ 'notes/readme.txt': 'old\n' }));
    const { code, out, calls } = await runPortable(root);
    check(code === 0, 'clean: make-portable.ps1 exits 0', out);
    check(/^metadata /m.test(calls) && !/^build /m.test(calls), 'clean -SkipBuild: the stub cargo answered `cargo metadata`, no build', calls);
    const zipPath = join(root, ZIP);
    check(existsSync(zipPath), 'clean: zip written', out);
    if (existsSync(zipPath)) {
      const bytes = readFileSync(zipPath);
      const names = zipNames(bytes);
      check(names.length > 0 && names.every((n) => !n.includes('\\')), 'clean: every entry name uses forward slashes', names.join('\n'));
      const files = names.filter((n) => !n.endsWith('/')).map((n) => n.slice(PREFIX.length)).sort();
      const want = ['LICENSE', 'README.md', 'THIRD_PARTY_NOTICES.md', 'docs/WINDOWS_USAGE.md', 'frametrace-app.exe', 'frametrace.exe', 'install.ps1', 'tools/bin/README-tools.txt'].sort();
      check(JSON.stringify(files) === JSON.stringify(want), 'clean: the zip holds exactly FrameTrace\'s own files (no tool binaries, no UNCONFIRMED.txt)', files.join('\n'));
      const readme = readFileSync(join(root, 'dist/FrameTrace-0.0.0-test-win64/tools/bin/README-tools.txt'), 'utf8');
      check(readme.includes('ffmpeg.exe / ffprobe.exe — 패키지에 포함되지 않음') && readme.includes('외부 도구 실행 파일이 들어 있지 않습니다')
        && !readme.includes('패키지에 포함된 경우'), 'clean: README-tools says no tool is included', readme);
      const sums = readFileSync(join(root, 'dist/SHA256SUMS'));
      check(sums[0] !== 0xef, 'clean: SHA256SUMS has no BOM');
      check(sums.toString('utf8') === `${sha256(bytes)}  FrameTrace-0.0.0-test-win64.zip\n`, 'clean: SHA256SUMS is this run\'s zip hash only (not old-notes.zip), one LF line', sums.toString('utf8'));
      check(!existsSync(join(root, 'dist/SHA256SUMS.unconfirmed')) && !existsSync(join(root, UZIP)), 'clean: no *.unconfirmed / *-UNCONFIRMED.zip');
    }
  }
  // An empty tools/bin folder (no files) is fine.
  {
    const root = newRepo();
    mkdirSync(join(root, 'tools/bin/licenses'), { recursive: true });
    mkdirSync(join(root, 'target/release/tools/bin'), { recursive: true });
    const { code, out } = await runPortable(root);
    check(code === 0 && existsSync(join(root, ZIP)), 'empty tools/bin folders: packages', out);
  }

  // --- M2: no third-party tool is bundled; tools/bin with files fails -------
  for (const [label, dir, files] of [
    ['tools/bin libewf/TSK', 'tools/bin', ['mmls.exe', 'libewf.dll']],
    ['tools/bin ffmpeg', 'tools/bin', ['ffmpeg.exe']],
    ['tools/bin nested license', 'tools/bin', ['licenses/ffmpeg-LICENSE.txt']],
    ['tools/bin a note only', 'tools/bin', ['NOTES.txt']],
    ['target/release/tools/bin', 'target/release/tools/bin', ['ewfinfo.exe']],
  ]) {
    const root = newRepo();
    for (const f of files) { mkdirSync(dirname(join(root, dir, f)), { recursive: true }); writeFileSync(join(root, dir, f), 'MZ planted tool'); }
    const { code, out } = await runPortable(root);
    check(code !== 0 && /bundles no third-party tool/.test(out) && out.includes(files[0].split('/').pop()) && /Move these files out/.test(out),
      `M2 ${label}: fails and names the file`, out);
    check(!existsSync(join(root, ZIP)) && !existsSync(join(root, 'dist/SHA256SUMS')), `M2 ${label}: no zip, no SHA256SUMS`);
  }
  {
    const root = newRepo();
    mkdirSync(join(root, 'tools/bin'), { recursive: true });
    writeFileSync(longPath(join(root, 'tools/bin', TRAILING)), 'MZ planted ffmpeg');
    const { code, out } = await runPortable(root);
    check(code !== 0 && /bundles no third-party tool/.test(out), 'M2 a trailing-space "ffmpeg.EXE " in tools/bin fails the run', out);
  }
  // M2 + M4: the release directory's tools/bin is the one in cargo's target dir.
  {
    const root = newRepo();
    const ctd = join(root, 'ctd');
    mkdirSync(join(ctd, 'release/tools/bin'), { recursive: true });
    for (const f of ['frametrace.exe', 'frametrace-app.exe']) writeFileSync(join(ctd, 'release', f), fakeExe('MZ ctd\n', fakeManifest()));
    writeFileSync(join(ctd, 'release/tools/bin/fls.exe'), 'MZ tsk');
    const { code, out } = await runPortable(root, { env: { CARGO_TARGET_DIR: ctd } });
    check(code !== 0 && /bundles no third-party tool/.test(out) && /fls\.exe/.test(out), 'M2 CARGO_TARGET_DIR/release/tools/bin with a file fails the run', out);
  }

  // --- L3b: a renamed FFmpeg file is found by signature strings -------------
  {
    const CHUNK = 4 * 1024 * 1024;
    const straddle = Buffer.alloc(CHUNK + 1024, 0x20);
    Buffer.from('Lavf61.7.100').copy(straddle, CHUNK - 5); // across the 4 MiB chunk boundary
    for (const [where, content, sig] of [
      ['target/release/frametrace-app.exe', null, 'the FFmpeg developers'],
      ['docs/WINDOWS_VALIDATION.md', straddle, 'Lavf61.7'],
      ['LICENSE', Buffer.from('\0\0\0FFMPEG VERSION 7.1-full_build-www.gyan.dev\0'), 'FFMPEG VERSION 7'],
      ['scripts/install.ps1', Buffer.from('\0ident\0libavcodec\0Lavc61.19.1\0'), 'libavcodec'],
      ['README.md', Buffer.from('\0\0LIBAVUTIL_59\0'), 'LIBAVUTIL'],
      ['THIRD_PARTY_NOTICES.md', Buffer.from('\0libswscale license: LGPL\0'), 'libswscale'],
    ]) {
      const root = newRepo();
      writeFileSync(join(root, where), content || fakeExe('MZ\0\0Copyright (c) 2000-2024 the FFmpeg developers\0', fakeManifest()));
      const started = Date.now();
      const { code, out } = await runPortable(root);
      const leaf = where.split('/').pop().replace(/\./g, '\\.');
      check(code !== 0 && new RegExp(`${leaf} contains the FFmpeg signature`).test(out) && out.includes(sig),
        `a staged file with an FFmpeg signature (${where}, ${JSON.stringify(sig)}) fails the run`, out);
      check(!existsSync(join(root, ZIP)), `no zip written for ${where}`);
      check(Date.now() - started < 60000, `${where} scan is quick (${Date.now() - started} ms)`);
    }
  }
  // L4: a stale zip in dist/ that contains FFmpeg (the old *-bundled.zip) fails the run.
  {
    const root = newRepo();
    mkdirSync(join(root, 'dist'), { recursive: true });
    writeFileSync(join(root, 'dist/FrameTrace-0.0.0-test-win64-bundled.zip'),
      makeZip({ 'FrameTrace-0.0.0-test-win64/tools/bin/ffmpeg.exe': 'MZ old bundle', 'FrameTrace-0.0.0-test-win64/README.md': 'x' }));
    const { code, out } = await runPortable(root);
    check(code !== 0 && /bundled\.zip contains FFmpeg/.test(out), 'a stale FFmpeg zip in dist/ fails the run', out);
    check(!existsSync(join(root, 'dist/SHA256SUMS')), 'no SHA256SUMS written next to a stale FFmpeg zip');
  }
  // L3: a renamed FFmpeg binary is found by its PE version information (a
  // .NET dll with FFmpeg version info + the manifest appended, as frametrace-app.exe).
  {
    const root = newRepo();
    const dll = join(root, 'renamed-tool.dll');
    ffmpegVersionInfoDll(dll);
    writeFileSync(join(root, 'target/release/frametrace-app.exe'), Buffer.concat([readFileSync(dll), Buffer.from(fakeManifest())]));
    unlinkSync(dll);
    const { code, out } = await runPortable(root);
    check(code !== 0 && /frametrace-app\.exe is an FFmpeg binary by its version information/.test(out), 'a renamed FFmpeg binary (version info) fails the run', out);
    check(!existsSync(join(root, ZIP)), 'no zip written for it');
  }

  // --- L-archives: archives / packed executables by magic bytes ---------------
  {
    const pad = (head, len = 600) => { const b = Buffer.alloc(len, 0x20); Buffer.from(head).copy(b, 0); return b; };
    const tar = Buffer.alloc(1024, 0); Buffer.from('codecs/ffmpeg.exe').copy(tar, 0); Buffer.from('ustar\x0000').copy(tar, 257);
    const upx = Buffer.alloc(8192, 0); Buffer.from('MZ').copy(upx, 0); Buffer.from('UPX0\0UPX1\0UPX!').copy(upx, 0x3e0);
    const upxLate = Buffer.alloc(8192, 0); Buffer.from('MZ').copy(upxLate, 0); Buffer.from('UPX!').copy(upxLate, 5000);
    for (const [label, where, content, reason, refused] of [
      ['zip (PK 03 04)', 'docs/WINDOWS_VALIDATION.md', makeZip({ 'ffmpeg.exe': 'MZ' }), 'zip signature', true],
      ['empty zip (PK 05 06)', 'LICENSE', pad('PK\x05\x06'), 'zip signature', true],
      ['gzip (1F 8B)', 'docs/WINDOWS_VALIDATION.md', pad(Buffer.from([0x1f, 0x8b, 8, 0])), 'gzip signature', true],
      ['7z', 'docs/WINDOWS_VALIDATION.md', pad(Buffer.from([0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c, 0, 4])), '7z signature', true],
      ['cab (MSCF)', 'scripts/install.ps1', pad('MSCF\0\0\0\0'), 'cabinet signature', true],
      ['rar', 'docs/WINDOWS_VALIDATION.md', pad(Buffer.from([0x52, 0x61, 0x72, 0x21, 0x1a, 0x07, 0])), 'rar signature', true],
      ['xz', 'docs/WINDOWS_VALIDATION.md', pad(Buffer.from([0xfd, 0x37, 0x7a, 0x58, 0x5a, 0])), 'xz signature', true],
      ['tar (ustar at 257)', 'docs/WINDOWS_VALIDATION.md', tar, 'tar signature', true],
      ['UPX-packed exe', 'target/release/frametrace-app.exe', Buffer.concat([upx, Buffer.from(fakeManifest())]), 'UPX-packed executable', true],
      ['"UPX!" after the first 4 KiB of a PE', 'target/release/frametrace-app.exe', Buffer.concat([upxLate, Buffer.from(fakeManifest())]), '', false],
      ['"UPX!" in a text file (not a PE)', 'docs/WINDOWS_VALIDATION.md', pad('# notes: UPX! is a packer'), '', false],
    ]) {
      const root = newRepo();
      writeFileSync(join(root, where), content);
      const { code, out } = await runPortable(root);
      if (refused) {
        const leaf = where.split('/').pop().replace(/\./g, '\\.');
        check(code !== 0 && new RegExp(`${leaf} is an archive or packed executable`).test(out) && out.includes(reason),
          `L-archives ${label} (${where}) fails the run`, out);
        check(!existsSync(join(root, ZIP)), `L-archives ${label}: no zip`);
      } else {
        check(code === 0 && existsSync(join(root, ZIP)), `L-archives ${label}: packages (not an archive)`, out);
      }
    }
  }

  // --- M2 (round 5): "Owner to confirm" blocks a release package ------------
  {
    const notices = 'Notices\n\nSome paragraph. (Owner to confirm this paragraph.)\n';
    const root = newRepo({ notices });
    const blocked = await runPortable(root);
    check(blocked.code !== 0 && /Owner to confirm/.test(blocked.out) && /AllowUnconfirmedNotices/.test(blocked.out), 'unconfirmed notices block the package', blocked.out);
    check(!existsSync(join(root, ZIP)), 'no zip while the notices are unconfirmed');
    // … unless -AllowUnconfirmedNotices (CI test package), with a warning —
    // and marked: *-UNCONFIRMED.zip, UNCONFIRMED.txt inside (L-marker),
    // SHA256SUMS.unconfirmed, no SHA256SUMS (a stale one is removed).
    mkdirSync(join(root, 'dist'), { recursive: true });
    writeFileSync(join(root, 'dist/SHA256SUMS'), 'stale  FrameTrace-0.0.0-test-win64.zip\n');
    const allowed = await runPortable(root, { extra: ['-AllowUnconfirmedNotices'] });
    check(allowed.code === 0 && /test package only/.test(allowed.out), '-AllowUnconfirmedNotices packages with a warning', allowed.out);
    check(/UNCONFIRMED TEST PACKAGE/.test(allowed.out) && /NOT FOR RELEASE/.test(allowed.out), '-AllowUnconfirmedNotices prints the loud warning', allowed.out);
    const unconfirmedZip = join(root, UZIP);
    check(existsSync(unconfirmedZip), 'zip written as *-UNCONFIRMED.zip with -AllowUnconfirmedNotices', allowed.out);
    check(!existsSync(join(root, ZIP)), 'no plain (release-looking) zip with -AllowUnconfirmedNotices');
    check(!existsSync(join(root, 'dist/SHA256SUMS')), 'no SHA256SUMS with -AllowUnconfirmedNotices (the stale one removed)');
    const usums = join(root, 'dist/SHA256SUMS.unconfirmed');
    check(existsSync(usums) && existsSync(unconfirmedZip)
      && readFileSync(usums, 'utf8') === `${sha256(readFileSync(unconfirmedZip))}  FrameTrace-0.0.0-test-win64-UNCONFIRMED.zip\n`,
    'SHA256SUMS.unconfirmed lists the UNCONFIRMED zip', existsSync(usums) ? readFileSync(usums, 'utf8') : 'missing');
    if (existsSync(unconfirmedZip)) {
      check(zipNames(readFileSync(unconfirmedZip)).includes(`${PREFIX}UNCONFIRMED.txt`), 'L-marker: the UNCONFIRMED zip holds UNCONFIRMED.txt at its root');
      const txt = readFileSync(join(root, 'dist/FrameTrace-0.0.0-test-win64/UNCONFIRMED.txt'), 'utf8');
      check(/NOT FOR RELEASE/.test(txt) && /THIRD_PARTY_NOTICES\.md still says 'Owner to confirm'/.test(txt) && /build-release\.sh refuses/.test(txt),
        'L-marker: UNCONFIRMED.txt says what is pending', txt);
    }
  }
  // L-CI: the marker reflowed across a line break (CRLF + indent) still blocks.
  {
    const notices = 'Notices\r\n\r\nSome paragraph. (Owner to\r\n   confirm this paragraph.)\r\n';
    const root = newRepo({ notices });
    const blocked = await runPortable(root);
    check(blocked.code !== 0 && /AllowUnconfirmedNotices/.test(blocked.out), 'a line-broken "Owner to / confirm" marker blocks the package', blocked.out);
    check(!existsSync(join(root, ZIP)), 'no zip for the line-broken marker');
  }
  // The real THIRD_PARTY_NOTICES.md: blocked while it carries the marker, and
  // its text (prose about FFmpeg) passes the signature scan as a test package.
  {
    const real = readFileSync(join(repo, 'THIRD_PARTY_NOTICES.md'), 'utf8');
    const root = newRepo({ notices: real });
    const marked = /Owner\s+to\s+confirm/i.test(real);
    const run = await runPortable(root, { extra: marked ? ['-AllowUnconfirmedNotices'] : [] });
    const zip = join(root, marked ? UZIP : ZIP);
    check(run.code === 0 && existsSync(zip), `the real THIRD_PARTY_NOTICES.md passes the signature scan (${marked ? 'unconfirmed' : 'confirmed'})`, run.out);
  }

  // --- Fix round 11 M2 / round 16: tool notices in tools/manifest.json --------
  const blocks = async (label, manifest, name = 'ffmpeg') => {
    const root = newRepo({ manifest });
    const blocked = await runPortable(root);
    check(blocked.code !== 0 && /notice_confirmed/.test(blocked.out) && blocked.out.includes(name) && /AllowUnconfirmedNotices/.test(blocked.out),
      `${label} blocks the package`, blocked.out);
    check(!existsSync(join(root, ZIP)), `${label}: no zip`);
  };
  const passes_ = async (label, manifest) => {
    const root = newRepo({ manifest });
    const run = await runPortable(root);
    check(run.code === 0 && existsSync(join(root, ZIP)), `${label} packages`, run.out);
  };
  await blocks('notice_confirmed false', fakeManifest(false));
  await blocks('notice_confirmed missing', fakeManifest('omit'));
  await blocks('notice_confirmed string "true"', fakeManifest('true'));
  // L-keycase: only the exact key counts (serde is case-sensitive).
  await blocks('"Notice_Confirmed": true (wrong case, no exact key)',
    fakeManifest('omit').replace('"notice": "n"', '"notice": "n",\n      "Notice_Confirmed": true'));
  await blocks('"NOTICE_CONFIRMED": true (wrong case, no exact key)',
    fakeManifest('omit').replace('"notice": "n"', '"notice": "n",\n      "NOTICE_CONFIRMED": true'));
  // Hunch-installable: a pip tool with a requirements file is installable.
  await blocks('a pip tool with a requirements_file and no notice_confirmed',
    fakeManifest(true, [{ name: 'lens-pip', kind: 'pip', requirements_file: 'tools/lens.requirements.txt' }]), 'lens-pip');
  await blocks('a pip tool with a requirements_file and "Notice_Confirmed": true',
    fakeManifest(true, [{ name: 'lens-pip', kind: 'pip', requirements_file: 'tools/lens.requirements.txt', Notice_Confirmed: true }]), 'lens-pip');
  await passes_('a pip tool with a requirements_file and notice_confirmed true',
    fakeManifest(true, [{ name: 'lens-pip', kind: 'pip', requirements_file: 'tools/lens.requirements.txt', notice_confirmed: true }]));
  await passes_('a pip tool with requirements_file null (not installable)',
    fakeManifest(true, [{ name: 'lens-pip', kind: 'pip', requirements_file: null }]));
  await passes_('a sha256 "" tool (not installable)', fakeManifest(true, [{ name: 'manual', kind: 'archive', sha256: '' }]));
  // "SHA256" (wrong case) is not the key serde reads: not installable.
  await passes_('a tool with only a wrong-case "SHA256" (not installable)', fakeManifest(true, [{ name: 'odd', kind: 'archive', SHA256: 'b'.repeat(64) }]));
  {
    const root = newRepo({ manifest: fakeManifest(false) });
    mkdirSync(join(root, 'dist'), { recursive: true });
    writeFileSync(join(root, 'dist/SHA256SUMS'), 'stale  FrameTrace-0.0.0-test-win64.zip\n');
    const allowed = await runPortable(root, { extra: ['-AllowUnconfirmedNotices'] });
    check(allowed.code === 0 && /notice of ffmpeg is not confirmed/.test(allowed.out) && /UNCONFIRMED TEST PACKAGE/.test(allowed.out) && /notice_confirmed is not true for: ffmpeg/.test(allowed.out),
      'an unconfirmed tool notice with -AllowUnconfirmedNotices: a test package with the loud warning', allowed.out);
    check(existsSync(join(root, UZIP)) && !existsSync(join(root, ZIP)), 'an unconfirmed tool notice: the zip is *-UNCONFIRMED.zip', allowed.out);
    check(existsSync(join(root, 'dist/SHA256SUMS.unconfirmed')) && !existsSync(join(root, 'dist/SHA256SUMS')), 'an unconfirmed tool notice: SHA256SUMS.unconfirmed only');
    const txt = join(root, 'dist/FrameTrace-0.0.0-test-win64/UNCONFIRMED.txt');
    check(existsSync(txt) && /notice_confirmed is not true for: ffmpeg/.test(readFileSync(txt, 'utf8')), 'L-marker: UNCONFIRMED.txt names the unconfirmed tool');
  }
  {
    const root = newRepo({ manifest: null });
    const missing = await runPortable(root);
    check(missing.code !== 0 && /tools\/manifest\.json not found/.test(missing.out), 'a missing tools/manifest.json fails the run', missing.out);
  }
  // The real tools/manifest.json: blocked while a notice is unconfirmed.
  {
    const real = readFileSync(join(repo, 'tools/manifest.json'), 'utf8');
    const installable = (t) => (t.sha256 !== undefined && t.sha256 !== null && t.sha256 !== '')
      || (t.kind === 'pip' && t.requirements_file !== undefined && t.requirements_file !== null && t.requirements_file !== '');
    const pending = JSON.parse(real.replace(/^\uFEFF/, '')).tools.filter((t) => installable(t) && t.notice_confirmed !== true).map((t) => t.name);
    const root = newRepo({ manifest: real });
    const run = await runPortable(root);
    check(pending.length ? run.code !== 0 && /notice_confirmed/.test(run.out) && pending.every((p) => run.out.includes(p)) : run.code === 0,
      `the real tools/manifest.json ${pending.length ? `blocks a release package (unconfirmed: ${pending.join(', ')})` : 'packages (every notice confirmed)'}`, run.out);
  }

  // --- hunch: the manifest compiled into the exes is the one checked --------
  {
    const root = newRepo({ exeManifest: fakeManifest(false) });
    const run = await runPortable(root);
    check(run.code !== 0 && /does not contain this working tree's tools\/manifest\.json/.test(run.out) && /rebuild it/.test(run.out),
      '-SkipBuild with an exe built from an unconfirmed manifest (working tree confirmed) fails the run', run.out);
    check(!existsSync(join(root, ZIP)), 'no zip for an exe with another manifest');
  }
  {
    const root = newRepo({ exeManifest: null });
    const run = await runPortable(root);
    check(run.code !== 0 && /does not contain this working tree/.test(run.out), '-SkipBuild with an exe holding no manifest fails the run', run.out);
  }
  {
    const lf = fakeManifest();
    const root = newRepo({ exeManifest: lf.replace(/\n/g, '\r\n') });
    const run = await runPortable(root);
    check(run.code === 0 && existsSync(join(root, ZIP)), '-SkipBuild: an exe embedding the manifest with CRLF line endings (LF working tree) packages', run.out);
  }
  {
    // only frametrace-app.exe is stale
    const root = newRepo();
    writeFileSync(join(root, 'target/release/frametrace-app.exe'), fakeExe('MZ old\n', fakeManifest(false)));
    const run = await runPortable(root);
    check(run.code !== 0 && /frametrace-app\.exe does not contain/.test(run.out), '-SkipBuild with a stale frametrace-app.exe only fails the run', run.out);
  }

  // --- M4: the binaries come from cargo's real target directory --------------
  // fakeRepo leaves STALE exes in target/release ("stale-target-release").
  const fresh = (root, path = STAGED_EXE) => existsSync(join(root, path)) && !readFileSync(join(root, path), 'latin1').includes('stale-target-release');
  for (const skipBuild of [true, false]) {
    const mode = skipBuild ? '-SkipBuild' : 'build';
    const variants = [
      ['CARGO_TARGET_DIR absolute', (root) => ({ env: { CARGO_TARGET_DIR: join(root, 'out-abs') }, dir: join(root, 'out-abs') })],
      ['CARGO_TARGET_DIR relative', (root) => ({ env: { CARGO_TARGET_DIR: 'out/rel' }, dir: join(root, 'out/rel') })],
      ['CARGO_BUILD_TARGET_DIR', (root) => ({ env: { CARGO_BUILD_TARGET_DIR: join(root, 'out-build') }, dir: join(root, 'out-build') })],
      ['.cargo/config.toml build.target-dir', (root) => {
        mkdirSync(join(root, '.cargo'), { recursive: true });
        writeFileSync(join(root, '.cargo/config.toml'), '[build]\ntarget-dir = "cfg-target"\n');
        return { env: {}, dir: join(root, 'cfg-target') };
      }],
    ];
    for (const [label, setup] of variants) {
      const root = newRepo();
      const { env, dir } = setup(root);
      if (skipBuild) {
        mkdirSync(join(dir, 'release'), { recursive: true });
        for (const f of ['frametrace.exe', 'frametrace-app.exe']) writeFileSync(join(dir, 'release', f), fakeExe('MZ prebuilt-in-target-dir\n', fakeManifest()));
      }
      const run = await runPortable(root, { skipBuild, env });
      check(run.code === 0 && fresh(root), `M4 ${mode} ${label}: packages the binary from ${dir.slice(root.length + 1)}, not the stale target/release`, run.out);
      if (!skipBuild) check(/^build --release --locked --message-format=json-render-diagnostics/m.test(run.calls), `M4 build ${label}: cargo build asked for JSON messages`, run.calls);
      // the variable/config points at a directory without binaries: refused
      const empty = newRepo();
      const e = setup(empty);
      const none = await runPortable(empty, { skipBuild, env: { ...e.env, FT_CARGO_NOWRITE: '1' } });
      check(none.code !== 0 && /missing .*frametrace(-app)?\.exe/.test(none.out) && !existsSync(join(empty, ZIP)),
        `M4 ${mode} ${label} without binaries there: fails (the stale target/release is not taken)`, none.out);
    }
    // no variable, no config: target/release (here: stale for -SkipBuild, rebuilt otherwise)
    const root = newRepo();
    const run = await runPortable(root, { skipBuild });
    check(run.code === 0 && existsSync(join(root, STAGED_EXE)), `M4 ${mode} default target dir: packages target/release`, run.out);
    if (!skipBuild) check(fresh(root), 'M4 build default: packages the binary cargo just wrote', run.out);
  }
  {
    const root = newRepo();
    const run = await runPortable(root, { skipBuild: false, env: { FT_CARGO_OMIT: 'frametrace-app' } });
    check(run.code !== 0 && /reported no executable for the frametrace-app binary/.test(run.out), 'M4 build: cargo reporting no frametrace-app executable fails the run', run.out);
  }
  {
    // version from cargo metadata when -Version is not given
    const root = newRepo();
    const run = await runPortable(root, { version: null });
    check(run.code === 0 && existsSync(join(root, 'dist/FrameTrace-0.0.0-stub-win64.zip')), 'no -Version: the version comes from cargo metadata', run.out);
  }
  // A malformed -Version is refused before anything is staged.
  {
    const root = newRepo();
    const { code, out } = await runPortable(root, { version: '../x' });
    check(code !== 0 && /bad -Version/.test(out), 'path-like -Version refused', out);
  }
} finally {
  for (const root of roots) removeRoot(root);
  rmSync(STUBS, { recursive: true, force: true });
}
if (failures) { console.error(`test-make-portable: ${passes} ok, ${failures} failure(s)`); process.exit(1); }
console.log(`test-make-portable: ${passes} ok — all checks passed`);
