#!/usr/bin/env python3
# /// script
# requires-python = ">=3.12"
# ///
# --- How to run ---
# python3 scripts/qa/verify-plan-evidence.py .omo/plans/frametrace-production-hardening-review-plan.md .omo/evidence/frametrace-production-hardening-review-plan
# python3 scripts/qa/verify-plan-evidence.py --self-test

from __future__ import annotations

import argparse
import re
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Final


TASK_PATTERN: Final = re.compile(r"^- \[.\] T(?P<number>\d+)\. (?P<title>.+)$")
REQUIRED_RECEIPTS: Final[dict[int, tuple[str, ...]]] = {
    1: (
        "baseline.md",
        "command-00-missing-helper-self-test.txt",
        "command-01-git-baseline-status.txt",
        "command-02-cargo-fmt-check.txt",
        "command-03-cargo-clippy.txt",
        "command-04-cargo-test-locked.txt",
        "command-05-node-check-app-js.txt",
        "command-06-git-diff-check.txt",
        "command-07-debug-binary-build-if-needed.txt",
        "command-08-empty-case-release-fail-closed.txt",
        "command-09-helper-self-test.txt",
        "cleanup-receipt.md",
    ),
}


@dataclass(frozen=True, slots=True)
class Task:
    number: int
    title: str


@dataclass(frozen=True, slots=True)
class VerificationResult:
    passed: bool
    messages: tuple[str, ...]


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

    if messages:
        return VerificationResult(passed=False, messages=tuple(messages))
    return VerificationResult(passed=True, messages=("PASS plan evidence receipts verified",))


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

        for receipt in REQUIRED_RECEIPTS[1]:
            (task_1 / receipt).write_text(f"{receipt}\n", encoding="utf-8")
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
