#!/usr/bin/env python3
# /// script
# requires-python = ">=3.12"
# ///
# --- How to run ---
# python3 scripts/qa/verify-plan-evidence.py .omo/plans/frametrace-production-hardening-review-plan.md .omo/evidence/frametrace-production-hardening-review-plan
# python3 scripts/qa/verify-plan-evidence.py --self-test

from __future__ import annotations

import argparse
import json
import re
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Final


TASK_PATTERN: Final = re.compile(r"^- \[.\] T(?P<number>\d+)\. (?P<title>.+)$")
OVERALL_COMPLETION_PLAN: Final = "frametrace-overall-completion-uplift-20260627"
F3_VIEWPORTS: Final = ("375x812", "768x900", "1280x900", "1440x900", "1920x1080")
F3_REQUIRED_TERM_GROUPS: Final = (
    ("증거 소스", "Evidence source"),
    ("영상 후보", "Video candidates"),
    ("검증 상태", "Verification status"),
    ("내보내기", "Export"),
    ("보고서", "Report"),
    ("local-first",),
    ("candidate-unvalidated",),
    ("verification required",),
    ("hash/check pending",),
    ("export draft",),
    ("report draft", "보고서 초안"),
    ("review pending", "examiner review pending", "검토 전 review pending"),
)
F3_REQUIRED_BOOLEAN_FIELDS: Final = (
    "viewer",
    "browser",
    "inspector",
    "previewBtn",
    "exportBtn",
    "reportBtn",
    "verifyBtn",
)
FINAL_REQUIRED_RECEIPTS: Final = (
    "final/F1-plan-compliance.md",
    "final/F2-code-quality.md",
    "final/F3-real-manual-qa/summary.md",
    "final/F3-real-manual-qa/f3-browser-assertions.json",
    "final/F4-scope-fidelity.md",
)
REQUIRED_RECEIPTS: Final[dict[int, tuple[str, ...]]] = {
    0: (
        "baseline.txt",
        "boulder-baseline.json",
        "cleanup-receipt.txt",
        "dirty-out-of-scope.txt",
        "handoff-baseline.md",
        "ledger-tail.txt",
        "scratch-check.txt",
        "t0-adversarial-verify.json",
        "t0-gate-review.md",
        "ui-review-baseline.txt",
    ),
    1: (
        "doneclaim.json",
        "qa-transcripts.txt",
        "t1-adversarial-verify.json",
    ),
    2: (
        "doneclaim.json",
        "qa-transcripts.txt",
        "t2-adversarial-verify.json",
    ),
    3: (
        "doneclaim.json",
        "qa-transcripts.txt",
        "t3-adversarial-verify.json",
    ),
    4: (
        "doneclaim.json",
        "qa-transcripts.txt",
        "t4-adversarial-verify.json",
    ),
    5: (
        "doneclaim.json",
        "t5-adversarial-verify.json",
    ),
    6: (
        "doneclaim.json",
        "t6-adversarial-verify.json",
    ),
    7: (
        "BLOCKED-missing-windows-runner-20260628.json",
        "caseroot-fix-doneclaim.json",
        "caseroot-fix-stop-hook-verification.txt",
        "transcripts/local-host-runner-blocker.txt",
        "transcripts/pwsh-engineonly-macos-blocker.txt",
    ),
    8: (
        "NA-winui-shell-blocked-by-t7.json",
    ),
    9: (
        "NA-package-scripts-blocked-by-t7.json",
    ),
    10: (
        "NA-clean-vm-package-blocked-by-t7.json",
    ),
    11: (
        "NA-field-pilot-corpus-blocked-by-t7.json",
    ),
    12: (
        "release-decision.json",
        "release-readiness.json",
        "release-readiness.md",
        "completion-score.md",
    ),
}
type JsonValue = None | bool | int | float | str | list["JsonValue"] | dict[str, "JsonValue"]
type JsonObject = dict[str, JsonValue]


@dataclass(frozen=True, slots=True)
class Task:
    number: int
    title: str


@dataclass(frozen=True, slots=True)
class VerificationResult:
    passed: bool
    messages: tuple[str, ...]


def repo_root_for_plan(plan_path: Path) -> Path:
    resolved = plan_path.resolve()
    if resolved.parent.name == "plans" and resolved.parent.parent.name == ".omo":
        return resolved.parent.parent.parent
    return Path.cwd()


def read_json_object(path: Path) -> JsonObject:
    value: JsonValue = json.loads(path.read_text(encoding="utf-8"))
    match value:
        case dict() as data:
            return data
        case _:
            raise TypeError(f"expected JSON object: {path}")


def json_text(value: JsonValue) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True)


def parse_tasks(plan_path: Path) -> tuple[Task, ...]:
    tasks: list[Task] = []
    for line in plan_path.read_text(encoding="utf-8").splitlines():
        match = TASK_PATTERN.match(line)
        if match is None:
            continue
        tasks.append(
            Task(
                number=int(match.group("number")),
                title=match.group("title").strip(),
            )
        )
    return tuple(tasks)


def task_dirs(evidence_root: Path, task_number: int) -> tuple[Path, ...]:
    prefix = f"task-{task_number:02d}-"
    return tuple(sorted(path for path in evidence_root.glob(f"{prefix}*") if path.is_dir()))


def is_overall_completion_plan(plan_path: Path, evidence_root: Path) -> bool:
    return (
        plan_path.stem == OVERALL_COMPLETION_PLAN
        or evidence_root.name == OVERALL_COMPLETION_PLAN
    )


def require_file(messages: list[str], path: Path, label: str) -> None:
    if not path.is_file():
        messages.append(f"FAIL missing {label}: {path}")


def verify_release_decision(messages: list[str], evidence_root: Path) -> None:
    decision_path = evidence_root / "task-12-final-field-pilot" / "release-decision.json"
    if not decision_path.is_file():
        messages.append(f"FAIL missing release decision: {decision_path}")
        return
    decision = read_json_object(decision_path)
    if decision.get("decision") != "BLOCKED":
        messages.append("FAIL T12 release-decision.json must be BLOCKED while T7 is blocked")


def verify_na_receipts(messages: list[str], evidence_root: Path) -> None:
    expected = {
        8: "NA-winui-shell-blocked-by-t7.json",
        9: "NA-package-scripts-blocked-by-t7.json",
        10: "NA-clean-vm-package-blocked-by-t7.json",
        11: "NA-field-pilot-corpus-blocked-by-t7.json",
    }
    for task_number, receipt in expected.items():
        dirs = task_dirs(evidence_root, task_number)
        matches = [path / receipt for path in dirs if (path / receipt).is_file()]
        if not matches:
            messages.append(f"FAIL T{task_number} missing N/A receipt: {receipt}")
            continue
        receipt_data = read_json_object(matches[0])
        if receipt_data.get("status") != "N/A":
            messages.append(f"FAIL T{task_number} receipt must have status N/A: {matches[0]}")


def verify_f3_browser_evidence(messages: list[str], evidence_root: Path) -> None:
    f3_root = evidence_root / "final" / "F3-real-manual-qa"
    assertions_path = f3_root / "f3-browser-assertions.json"
    require_file(messages, assertions_path, "F3 browser assertions")
    if assertions_path.is_file():
        assertions = read_json_object(assertions_path)
        if assertions.get("status") != "PASS":
            messages.append("FAIL F3 browser assertions status must be PASS")

    for viewport in F3_VIEWPORTS:
        required_files = (
            f3_root / "snapshots" / f"f3-{viewport}-snapshot.txt",
            f3_root / "screenshots" / f"f3-{viewport}-screenshot.json",
            f3_root / "screenshots" / f"f3-{viewport}.png",
            f3_root / "console" / f"f3-{viewport}-console.txt",
            f3_root / "text" / f"f3-{viewport}-text.txt",
            f3_root / "layout" / f"f3-{viewport}-layout.json",
        )
        for file_path in required_files:
            require_file(messages, file_path, f"F3 {viewport} artifact")

        layout_path = f3_root / "layout" / f"f3-{viewport}-layout.json"
        if not layout_path.is_file():
            continue
        layout = read_json_object(layout_path)
        if layout.get("overflow") is not False:
            messages.append(f"FAIL F3 {viewport} has horizontal overflow")
        for field in F3_REQUIRED_BOOLEAN_FIELDS:
            if layout.get(field) is not True:
                messages.append(f"FAIL F3 {viewport} missing visible UI field: {field}")
        width = layout.get("width")
        expected_width = int(viewport.split("x", maxsplit=1)[0])
        if width != expected_width:
            messages.append(
                f"FAIL F3 {viewport} captured width {width!r}, expected {expected_width}"
            )
        required_terms = layout.get("requiredTerms")
        if not isinstance(required_terms, list):
            messages.append(f"FAIL F3 {viewport} missing requiredTerms list")
            continue
        terms = {
            item.get("term"): item.get("present")
            for item in required_terms
            if isinstance(item, dict)
        }
        for term_group in F3_REQUIRED_TERM_GROUPS:
            if not any(terms.get(term) is True for term in term_group):
                label = " / ".join(term_group)
                messages.append(f"FAIL F3 {viewport} missing required term group: {label}")


def read_ledger_records(ledger_path: Path) -> tuple[JsonObject, ...]:
    records: list[JsonObject] = []
    for line in ledger_path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        value: JsonValue = json.loads(line)
        match value:
            case dict() as record:
                records.append(record)
            case _:
                raise TypeError(f"ledger line is not an object: {ledger_path}")
    return tuple(records)


def has_ledger_record(
    records: tuple[JsonObject, ...],
    event: str,
    task_prefix: str | None,
    artifact_fragment: str,
) -> bool:
    for record in records:
        if record.get("event") != event:
            continue
        if task_prefix is not None and not str(record.get("task", "")).startswith(task_prefix):
            continue
        if artifact_fragment in json_text(record):
            return True
    return False


def verify_ledger_terminal_state(messages: list[str], repo_root: Path) -> None:
    ledger_path = repo_root / ".omo" / "start-work" / "ledger.jsonl"
    if not ledger_path.is_file():
        messages.append(f"FAIL missing start-work ledger: {ledger_path}")
        return
    records = read_ledger_records(ledger_path)
    required = (
        ("fix-evidence-received", "T7.", "caseroot-fix-doneclaim.json"),
        ("task-completed", "T7.", "task-07-windows-t13"),
        ("task-n-a", "T8.", "NA-winui-shell-blocked-by-t7.json"),
        ("task-n-a", "T9.", "NA-package-scripts-blocked-by-t7.json"),
        ("task-n-a", "T10.", "NA-clean-vm-package-blocked-by-t7.json"),
        ("task-n-a", "T11.", "NA-field-pilot-corpus-blocked-by-t7.json"),
        ("task-completed", "T12.", "release-decision.json"),
        ("final-verify", "F1.", "F1-plan-compliance.md"),
        ("final-verify", "F2.", "F2-code-quality.md"),
        ("final-verify", "F3.", "F3-real-manual-qa"),
        ("final-verify", "F4.", "F4-scope-fidelity.md"),
        ("plan-blocked-terminal", None, "BLOCKED"),
    )
    for event, task_prefix, artifact_fragment in required:
        if not has_ledger_record(records, event, task_prefix, artifact_fragment):
            task_label = task_prefix or "plan"
            messages.append(
                f"FAIL ledger missing terminal record: {event} {task_label} {artifact_fragment}"
            )


def verify_boulder_terminal_state(messages: list[str], repo_root: Path) -> None:
    boulder_path = repo_root / ".omo" / "boulder.json"
    if not boulder_path.is_file():
        messages.append(f"FAIL missing Boulder state: {boulder_path}")
        return
    boulder = read_json_object(boulder_path)
    works = boulder.get("works")
    if not isinstance(works, dict):
        messages.append("FAIL Boulder state missing works object")
        return
    work = works.get(OVERALL_COMPLETION_PLAN)
    if not isinstance(work, dict):
        messages.append(f"FAIL Boulder state missing work: {OVERALL_COMPLETION_PLAN}")
        return
    if work.get("status") != "blocked":
        messages.append(f"FAIL Boulder work must be blocked, got {work.get('status')!r}")
    if work.get("blocked_task") != "T7. Complete T13 on real Windows or preserve the hard blocker":
        messages.append("FAIL Boulder blocked_task must identify T7")
    receipt = work.get("blocked_receipt")
    if receipt != (
        ".omo/evidence/frametrace-overall-completion-uplift-20260627/"
        "task-07-windows-t13/BLOCKED-missing-windows-runner-20260628.json"
    ):
        messages.append("FAIL Boulder blocked_receipt must point to the T7 blocker")
    stopped = work.get("stopped_tasks")
    if stopped != ["T8", "T9", "T10", "T11"]:
        messages.append(f"FAIL Boulder stopped_tasks must be T8-T11, got {stopped!r}")


def verify_overall_completion_final_state(
    messages: list[str],
    plan_path: Path,
    evidence_root: Path,
) -> None:
    repo_root = repo_root_for_plan(plan_path)
    for receipt in FINAL_REQUIRED_RECEIPTS:
        require_file(messages, evidence_root / receipt, f"final receipt {receipt}")
    verify_f3_browser_evidence(messages, evidence_root)
    verify_release_decision(messages, evidence_root)
    verify_na_receipts(messages, evidence_root)
    verify_ledger_terminal_state(messages, repo_root)
    verify_boulder_terminal_state(messages, repo_root)


def verify_evidence(plan_path: Path, evidence_root: Path) -> VerificationResult:
    messages: list[str] = []
    tasks = parse_tasks(plan_path)
    if not tasks:
        messages.append(f"FAIL no plan todos found: {plan_path}")
        return VerificationResult(passed=False, messages=tuple(messages))
    if not evidence_root.is_dir():
        messages.append(f"FAIL missing evidence root: {evidence_root}")
        return VerificationResult(passed=False, messages=tuple(messages))

    for task in tasks:
        dirs = task_dirs(evidence_root, task.number)
        if not dirs:
            messages.append(f"FAIL T{task.number} missing evidence directory")
            continue
        receipts = REQUIRED_RECEIPTS.get(task.number, ())
        for receipt in receipts:
            if not any((path / receipt).is_file() for path in dirs):
                messages.append(f"FAIL T{task.number} missing receipt: {receipt}")

    if is_overall_completion_plan(plan_path, evidence_root):
        verify_overall_completion_final_state(messages, plan_path, evidence_root)

    if messages:
        return VerificationResult(passed=False, messages=tuple(messages))
    return VerificationResult(
        passed=True,
        messages=("PASS plan evidence receipts verified, including final F1-F4 state",),
    )


def write_self_test_plan(path: Path) -> None:
    path.write_text(
        "\n".join(
            (
                "# Test plan",
                "- [ ] T1. Baseline",
                "- [ ] T2. Follow-up",
                "",
            )
        ),
        encoding="utf-8",
    )


def expect_result(result: VerificationResult, expected_passed: bool, label: str) -> str:
    if result.passed != expected_passed:
        state = "PASS" if result.passed else "FAIL"
        raise AssertionError(f"{label}: expected {expected_passed}, got {state}")
    return f"PASS self-test {label}"


def run_self_test() -> VerificationResult:
    lines: list[str] = []
    with tempfile.TemporaryDirectory(prefix="frametrace-evidence-helper-") as temp:
        root = Path(temp)
        plan = root / "plan.md"
        evidence = root / "evidence"
        write_self_test_plan(plan)

        lines.append(expect_result(verify_evidence(plan, evidence), False, "missing-root"))
        evidence.mkdir()
        lines.append(expect_result(verify_evidence(plan, evidence), False, "missing-task-dirs"))

        task_1 = evidence / "task-01-baseline"
        task_2 = evidence / "task-02-follow-up"
        task_1.mkdir()
        task_2.mkdir()
        lines.append(expect_result(verify_evidence(plan, evidence), False, "missing-receipts"))

        for task_dir, receipts in (
            (task_1, REQUIRED_RECEIPTS[1]),
            (task_2, REQUIRED_RECEIPTS[2]),
        ):
            for receipt in receipts:
                path = task_dir / receipt
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(f"{receipt}\n", encoding="utf-8")
        lines.append(expect_result(verify_evidence(plan, evidence), True, "valid-evidence"))

    return VerificationResult(passed=True, messages=tuple(lines))


def print_result(result: VerificationResult) -> None:
    for message in result.messages:
        print(message)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Verify that plan todos have evidence directories and required receipts."
    )
    parser.add_argument("plan", nargs="?", type=Path)
    parser.add_argument("evidence_root", nargs="?", type=Path)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.self_test:
        result = run_self_test()
    elif args.plan is not None and args.evidence_root is not None:
        result = verify_evidence(args.plan, args.evidence_root)
    elif args.plan is None and args.evidence_root is None:
        result = VerificationResult(
            passed=False,
            messages=("FAIL expected PLAN and EVIDENCE_ROOT, or --self-test",),
        )
    else:
        result = VerificationResult(
            passed=False,
            messages=("FAIL expected both PLAN and EVIDENCE_ROOT",),
        )

    print_result(result)
    return 0 if result.passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
