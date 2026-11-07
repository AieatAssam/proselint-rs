#!/usr/bin/env python3
"""Build a frozen Rust candidate, attest it, and run the cumulative live gate."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import time
import uuid
from pathlib import Path
from typing import Any

from compare import REPORT_SCHEMA_VERSION, load_manifest, run_live, run_self_tests, source_fingerprint, validate_report
from setup_oracle import ROOT, RUST_ROOT, oracle_integrity


STATUSES = ["PASS", "MISMATCH", "NOT_IMPLEMENTED", "ORACLE_ERROR", "CANDIDATE_ERROR", "TIMEOUT", "BLOCKED"]


def add_gate_execution(report: dict[str, Any], duration_seconds: float, stages: dict[str, dict[str, Any]], capability_count: int, case_count: int) -> None:
    report["gate_execution"] = {
        "duration_seconds": round(duration_seconds, 6),
        "stages": stages,
        "accepted_capability_count": capability_count,
        "live_case_count": case_count,
    }


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def locked_cargo() -> tuple[Path, str]:
    toolchain_file = RUST_ROOT / "rust-toolchain.toml"
    match = re.search(r'^channel\s*=\s*"([^"]+)"', toolchain_file.read_text(encoding="utf-8"), re.MULTILINE)
    if not match:
        raise RuntimeError("rust-toolchain.toml has no channel")
    channel = match.group(1)
    candidates = [
        RUST_ROOT / ".cache" / "rustup-home" / "toolchains" / f"{channel}-x86_64-unknown-linux-gnu" / "bin" / "cargo",
        RUST_ROOT / ".cache" / "rustup-home" / "toolchains" / f"stable-x86_64-unknown-linux-gnu" / "bin" / "cargo",
    ]
    found = next((path for path in candidates if path.is_file()), None)
    if found is None:
        system = shutil.which("cargo")
        if system is None:
            raise RuntimeError(f"cargo for pinned toolchain {channel} is unavailable")
        found = Path(system).resolve()
    return found.resolve(), channel


def accepted_capabilities() -> set[str]:
    status_path = RUST_ROOT / "status.json"
    value = json.loads(status_path.read_text(encoding="utf-8"))
    entries = value.get("production_capabilities")
    if not isinstance(entries, list):
        raise RuntimeError("status.json production_capabilities must be an array")
    accepted = set()
    for entry in entries:
        if isinstance(entry, dict) and entry.get("status") in {"independently_verified", "recorded"}:
            if isinstance(entry.get("id"), str):
                accepted.add(entry["id"])
    return accepted


def write_blocked_report(report_path: Path, reason: str) -> None:
    now = dt.datetime.now(dt.timezone.utc).isoformat()
    run_id = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ") + "-blocked"
    try:
        baseline = oracle_integrity()
    except (OSError, RuntimeError, subprocess.SubprocessError, KeyError) as exc:
        baseline = {"integrity": "unavailable", "error": str(exc)}
    report = {
        "schema_version": REPORT_SCHEMA_VERSION,
        "run_id": run_id,
        "started_at_utc": now,
        "finished_at_utc": now,
        "status": "BLOCKED",
        "run_level_blocker": True,
        "source_fingerprint": source_fingerprint(),
        "baseline": baseline,
        "candidate": None,
        "environment": {"python": sys.version, "platform": platform.platform()},
        "capability_ids": [],
        "case_ids": [],
        "counts": {**{status: 0 for status in STATUSES}, "BLOCKED": 1},
        "cases": [],
        "raw_artifacts_dir": str(RUST_ROOT / "artifacts"),
        "self_test": {"required": True, "status": "not_run"},
        "errors": [{"kind": "BLOCKED", "message": reason}],
    }
    validate_report(report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def build_and_attest(candidate_json: str | None) -> tuple[list[str], Path]:
    manifest = RUST_ROOT / "Cargo.toml"
    lockfile = RUST_ROOT / "Cargo.lock"
    if not manifest.is_file() or not lockfile.is_file():
        raise RuntimeError("Cargo.toml and Cargo.lock are required before the live gate can build a candidate")
    cargo, channel = locked_cargo()
    cargo_dir = cargo.parent
    env = os.environ.copy()
    env.pop("RUSTFLAGS", None)
    env["CARGO_HOME"] = str(RUST_ROOT / ".cache" / "cargo-home")
    env["RUSTUP_HOME"] = str(RUST_ROOT / ".cache" / "rustup-home")
    env["RUSTUP_TOOLCHAIN"] = channel
    env["PATH"] = str(cargo_dir) + os.pathsep + env.get("PATH", "")
    commands = [
        [str(cargo), "fmt", "--manifest-path", str(manifest), "--check"],
        [str(cargo), "clippy", "--manifest-path", str(manifest), "--all-targets", "--", "-D", "warnings"],
        [str(cargo), "test", "--locked", "--manifest-path", str(manifest)],
        [str(cargo), "build", "--locked", "--manifest-path", str(manifest)],
    ]
    before = source_fingerprint()
    run_id = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ") + "-build"
    artifacts = RUST_ROOT / "artifacts"
    artifacts.mkdir(parents=True, exist_ok=True)
    build_runs = []
    for index, command in enumerate(commands):
        output = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, timeout=900, check=False)
        stdout_path = artifacts / f"{run_id}-{index}.stdout.bin"
        stderr_path = artifacts / f"{run_id}-{index}.stderr.bin"
        stdout_path.write_bytes(output.stdout)
        stderr_path.write_bytes(output.stderr)
        build_runs.append({"command": command, "exit_code": output.returncode, "stdout_path": str(stdout_path), "stderr_path": str(stderr_path)})
        if output.returncode != 0:
            raise RuntimeError(f"Rust gate command failed with exit status {output.returncode}; logs: {run_id}-{index}")
        if source_fingerprint() != before:
            raise RuntimeError(f"Rust source tree changed during gate command {index}")
    after = source_fingerprint()
    if before != after:
        raise RuntimeError("Rust source tree changed during candidate build")
    binary = RUST_ROOT / "target" / "debug" / ("proselint.exe" if os.name == "nt" else "proselint")
    if not binary.is_file():
        raise RuntimeError(f"expected Rust CLI binary not produced: {binary}")
    if candidate_json is None:
        candidate = [str(binary.resolve())]
    else:
        try:
            candidate = json.loads(candidate_json)
        except json.JSONDecodeError as exc:
            raise ValueError(f"invalid --candidate-argv-json: {exc}") from exc
        if not isinstance(candidate, list) or not candidate or not all(isinstance(value, str) for value in candidate):
            raise ValueError("candidate argv must be a non-empty JSON string array")
        if Path(candidate[0]).resolve() != binary.resolve():
            raise ValueError("candidate argv must begin with the binary produced by this locked build")
    version = subprocess.run([str(cargo), "--version"], check=True, capture_output=True, text=True, env=env).stdout.strip()
    rustc = subprocess.run([str(cargo_dir / "rustc"), "--version"], check=True, capture_output=True, text=True, env=env).stdout.strip()
    lock_hash = sha256_file(lockfile)
    attestation = {
        "schema_version": 1,
        "verified": True,
        "source_fingerprint": after,
        "cargo_lock_sha256": lock_hash,
        "binary_path": str(binary.resolve()),
        "binary_sha256": sha256_file(binary),
        "cargo_version": version,
        "rustc_version": rustc,
        "toolchain": channel,
        "build_commands": build_runs,
        "build_exit_code": 0,
    }
    path = artifacts / f"{run_id}.attestation.json"
    path.write_text(json.dumps(attestation, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return candidate, path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--live", action="store_true")
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--candidate-argv-json")
    parser.add_argument("--reference-argv-json")
    args = parser.parse_args()
    if not args.live:
        parser.error("gate requires --live")
    started = time.monotonic()
    stages: dict[str, dict[str, Any]] = {}
    selected_capabilities = 0
    selected_cases = 0

    def begin_stage() -> float:
        return time.monotonic()

    def finish_stage(name: str, stage_started: float, **counts: int) -> None:
        stages[name] = {"duration_seconds": round(time.monotonic() - stage_started, 6), **counts}

    def record_execution() -> None:
        if not args.report.is_file():
            return
        report = json.loads(args.report.read_text(encoding="utf-8"))
        add_gate_execution(report, time.monotonic() - started, stages, selected_capabilities, selected_cases)
        args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    try:
        stage_started = begin_stage()
        try:
            oracle_integrity()
        finally:
            finish_stage("oracle_integrity", stage_started)
        stage_started = begin_stage()
        checks = []
        try:
            checks = run_self_tests()
        finally:
            finish_stage("comparator_self_tests", stage_started, check_count=len(checks))
        if not checks or not all(item["passed"] for item in checks):
            raise RuntimeError("comparator self-tests did not all pass")
        capabilities = accepted_capabilities()
        selected_capabilities = len(capabilities)
        if not capabilities:
            raise RuntimeError("zero production capabilities are independently verified; strict gate has no required cases")
        manifest = load_manifest()
        present = {case["capability_id"] for case in manifest["cases"]}
        missing = sorted(capabilities - present)
        if missing:
            raise RuntimeError(f"accepted capabilities have no live cases: {missing}")
        selected_cases = sum(case["capability_id"] in capabilities for case in manifest["cases"])
        stage_started = begin_stage()
        try:
            candidate, attestation = build_and_attest(args.candidate_argv_json)
        finally:
            finish_stage("rust_checks_and_build", stage_started)
        stage_started = begin_stage()
        try:
            result = run_live(None, args.report, json.dumps(candidate), args.reference_argv_json, attestation, required_capabilities=capabilities)
        finally:
            finish_stage("cumulative_live_comparison", stage_started, capability_count=selected_capabilities, case_count=selected_cases)
        record_execution()
        return result
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as exc:
        write_blocked_report(args.report, str(exc))
        record_execution()
        print(f"BLOCKED: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
