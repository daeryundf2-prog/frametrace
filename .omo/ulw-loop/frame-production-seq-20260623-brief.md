FrameTrace production-sequence continuation.

User request:
Proceed sequentially through all remaining work identified so far using omo:ulw-loop.

Context:
- Existing branch: codex/frametrace-forensic-hardening.
- Prior verified slice added a Windows/WinUI prerequisite gate.
- Prior ULW session frame-windows-prereq-gate-20260622 has C001/C002/C003 pass, but the goal is blocked because the final complete checkpoint could not reconcile the completed Codex goal objective with the aggregate ULW objective. Do not fake completion.
- The current worktree is dirty with GUI inventory, media validation, QA release, Windows prerequisite, documentation, scripts, and evidence artifacts.

Objective:
Turn the current worktree into production-safe, verified, atomic work units and continue the roadmap as far as this macOS environment can honestly execute.

Required sequence:
1. Recover and preserve current evidence state.
2. Classify dirty worktree into atomic units.
3. Verify the already-implemented Windows prerequisite gate from a fresh run.
4. Commit verified safe units atomically using observed repository commit style.
5. Run full fresh validation after each committed unit or record a no-commit blocker if committing is unsafe.
6. Continue executable slices for GUI inventory, SQLite query layer, media validation, audit/report defensibility, and release readiness.
7. Stop only at a true hard blocker, especially Windows-only WinUI build/test execution that cannot run on macOS.

Hard constraints:
- Do not claim Windows or WinUI GA from macOS.
- Do not hand-edit .omo/ulw-loop goal state.
- Do not fake PASS evidence or Codex goal snapshots.
- Preserve unrelated dirty changes unless they are part of the verified work unit.
- Original evidence paths must remain read-only; derived artifacts must stay separate.
- Tests alone are not done; use CLI/browser/computer-use surfaces as appropriate.
