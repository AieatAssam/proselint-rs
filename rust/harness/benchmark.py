#!/usr/bin/env python3
"""Parity-gated paired benchmarks for the pinned misc.suddenly capability."""

from __future__ import annotations

import argparse
import base64
import datetime as dt
import hashlib
import json
import os
import platform
import statistics
import subprocess
import sys
import time
import uuid
from pathlib import Path
from typing import Any

import compare
from gate import accepted_capabilities, locked_cargo
from setup_oracle import ROOT, RUST_ROOT, oracle_integrity


CAPABILITY = "rule.misc.suddenly"
WORKLOADS = RUST_ROOT / "harness" / "benchmark_inputs" / "workloads.json"
PYTHON_HELPER = WORKLOADS.parent / "python_suddenly_scan.py"
RUST_HELPER = RUST_ROOT / "target" / "release" / "examples" / "benchmark_suddenly"
STATUSES = ("PASS", "MISMATCH", "NOT_IMPLEMENTED", "ORACLE_ERROR", "CANDIDATE_ERROR", "TIMEOUT", "BLOCKED")
REPORT_SCHEMA_VERSION = 1


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def load_workloads(path: Path = WORKLOADS) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=compare.unique_object)
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"invalid benchmark workload manifest: {exc}") from exc
    if not isinstance(value, dict) or value.get("schema_version") != 1:
        raise ValueError("workload manifest must have schema_version=1")
    if value.get("capability_id") != CAPABILITY or value.get("expected_check_path") != "misc.suddenly":
        raise ValueError("benchmark is pinned to rule.misc.suddenly")
    if value.get("output_format") != "json" or value.get("config") != {"max_errors": 1000, "checks": {"misc": {"suddenly": True}}}:
        raise ValueError("benchmark must use the isolated JSON config with max_errors=1000")
    if not isinstance(value.get("workloads"), list) or len(value["workloads"]) < 3:
        raise ValueError("benchmark needs upstream, ordinary, and dense generated workloads")
    ids: set[str] = set()
    base = path.parent.resolve()
    for item in value["workloads"]:
        if not isinstance(item, dict) or not isinstance(item.get("id"), str) or item["id"] in ids:
            raise ValueError("benchmark workload IDs must be unique strings")
        ids.add(item["id"])
        fixture = (base / item.get("path", "")).resolve()
        if base not in fixture.parents or not fixture.is_file():
            raise ValueError(f"unsafe or missing workload fixture for {item['id']}")
        raw = fixture.read_bytes()
        if sha256_bytes(raw) != item.get("sha256") or len(raw) != item.get("bytes"):
            raise ValueError(f"workload fixture integrity mismatch for {item['id']}")
        if not isinstance(item.get("expected_diagnostics_per_iteration"), int) or item["expected_diagnostics_per_iteration"] < 0:
            raise ValueError(f"invalid expected diagnostic count for {item['id']}")
        if item.get("kind") not in {"licensed_upstream_prose", "deterministic_ordinary_generated", "deterministic_dense_adversarial_generated"}:
            raise ValueError(f"unrecognized workload kind for {item['id']}")
    corpus = value.get("upstream_corpus")
    if not isinstance(corpus, dict) or corpus.get("source_revision") != "dbed789caae662d06c7c8a5a13dd31f1acd36f5c":
        raise ValueError("upstream corpus provenance must name the pinned revision")
    license_file = (base / corpus.get("license_file", "")).resolve()
    if base not in license_file.parents or not license_file.is_file() or sha256_file(license_file) != corpus.get("license_sha256"):
        raise ValueError("upstream corpus license evidence is missing or changed")
    config_path = (base / value.get("config_file", "")).resolve()
    config_bytes = config_path.read_bytes() if config_path.is_file() else b""
    if sha256_bytes(config_bytes) != value.get("config_sha256"):
        raise ValueError("selected benchmark config is missing or changed")
    return value


def workload_bytes(workload: dict[str, Any], manifest: dict[str, Any]) -> bytes:
    return (WORKLOADS.parent / workload["path"]).read_bytes()


def make_parity_manifest(workloads: dict[str, Any], path: Path) -> Path:
    cases = []
    config = (WORKLOADS.parent / workloads["config_file"]).read_bytes()
    for workload in workloads["workloads"]:
        input_name = "benchmark-document" + Path(workload["path"]).suffix
        cases.append({
            "id": "benchmark_" + workload["id"],
            "capability_id": CAPABILITY,
            "provenance": f"Exact benchmark input {workload['path']} ({workload['kind']}); expected count from pinned oracle CLI.",
            "input_mode": "files",
            "files": [
                {"path": "selected config.json", "content_base64": base64.b64encode(config).decode("ascii")},
                {"path": input_name, "content_base64": base64.b64encode(workload_bytes(workload, workloads)).decode("ascii")},
            ],
            "argv": ["check", "--config", "{file:selected config.json}", "--output-format", "json", "{file:" + input_name + "}"],
            "output_format": "json",
            "expected_oracle_outcome": "findings" if workload["expected_diagnostics_per_iteration"] else "clean",
            "timeout_seconds": 60,
            "environment": {"locale": "C.UTF-8"},
            "config_scenario": "misc_suddenly_only_benchmark",
            "workspace_mutates": False,
            "comparison": {"stdout": "json", "stderr": "bytes", "exit_status": "exact"},
        })
    value = {"schema_version": 1, "cases": cases}
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    compare.load_manifest(path)
    return path


def pair_order(pair_index: int) -> list[str]:
    return ["reference", "candidate"] if pair_index % 2 == 0 else ["candidate", "reference"]


def percentile(values: list[int], fraction: float) -> int:
    ordered = sorted(values)
    if not ordered:
        raise ValueError("cannot summarize an empty sample")
    position = (len(ordered) - 1) * fraction
    low = int(position)
    high = min(low + 1, len(ordered) - 1)
    return round(ordered[low] + (ordered[high] - ordered[low]) * (position - low))


def summarize(values: list[int]) -> dict[str, int]:
    if not values or any(type(value) is not int or value <= 0 for value in values):
        raise ValueError("timing samples must be non-empty positive integer nanoseconds")
    p25, p75 = percentile(values, 0.25), percentile(values, 0.75)
    median = int(statistics.median(values))
    return {
        "sample_count": len(values), "median_ns": median, "minimum_ns": min(values),
        "maximum_ns": max(values), "range_ns": max(values) - min(values),
        "p25_ns": p25, "p75_ns": p75, "iqr_ns": p75 - p25,
    }


def canonical_diagnostics_digest(value: Any) -> str:
    return sha256_bytes(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8"))


def validate_benchmark_report(report: dict[str, Any]) -> None:
    required = {
        "schema_version", "run_id", "started_at_utc", "finished_at_utc", "status", "run_level_blocker",
        "source_fingerprint", "baseline", "candidate", "environment", "capability_id", "workload_ids",
        "counts", "parity", "fresh_process", "in_memory", "rss", "raw_artifacts_dir", "errors", "performance_claim",
    }
    if not isinstance(report, dict) or required - report.keys() or report.get("schema_version") != REPORT_SCHEMA_VERSION:
        raise ValueError("benchmark report does not satisfy schema version 1")
    if report["status"] not in {"BLOCKED", "MISMATCH", "PARITY_PASS", "PASS"}:
        raise ValueError("invalid benchmark status")
    if not isinstance(report["run_level_blocker"], bool) or not isinstance(report["errors"], list):
        raise ValueError("invalid benchmark blocker fields")
    if report["capability_id"] != CAPABILITY or not isinstance(report["workload_ids"], list):
        raise ValueError("benchmark report capability/workloads are invalid")
    if not isinstance(report["counts"], dict) or set(report["counts"]) != set(STATUSES):
        raise ValueError("benchmark counts must include every harness status")
    if any(type(count) is not int or count < 0 for count in report["counts"].values()):
        raise ValueError("benchmark status counts must be non-negative integers")
    if not isinstance(report["parity"], dict) or report["parity"].get("status") not in {"PASS", "MISMATCH", "BLOCKED", "not_run"}:
        raise ValueError("benchmark parity record is malformed")
    if not isinstance(report["fresh_process"], list) or not isinstance(report["in_memory"], list):
        raise ValueError("benchmark timing fields must be arrays")
    if report["status"] == "PARITY_PASS":
        if report["parity"]["status"] != "PASS" or report["fresh_process"] or report["in_memory"] or report["performance_claim"] != "none":
            raise ValueError("parity-only report must contain no timing results or performance claim")
    if report["status"] in {"BLOCKED", "MISMATCH"}:
        if not report["run_level_blocker"] or not report["errors"] or report["performance_claim"] != "none":
            raise ValueError("blocked/mismatched reports need an error and no performance claim")
    if report["status"] == "PASS":
        if report["run_level_blocker"] or report["errors"] or report["parity"]["status"] != "PASS":
            raise ValueError("passing benchmark must have clean parity and no blockers")
        for item in report["fresh_process"]:
            samples = item.get("samples")
            if not isinstance(samples, list) or len(samples) < 25 or item.get("sample_pairs") != len(samples):
                raise ValueError("each fresh-process workload needs at least 25 recorded sample pairs")
            if [sample.get("pair_index") for sample in samples] != list(range(len(samples))):
                raise ValueError("fresh-process sample pairs must be in order")
            if any(sample.get("order") != pair_order(i) or sample.get("outputs_match") is not True or sample.get("status_match") is not True for i, sample in enumerate(samples)):
                raise ValueError("fresh-process pairs must be interleaved and output/status equivalent")
        for item in report["in_memory"]:
            if item.get("diagnostics_match") is not True or item.get("checksum_match") is not True:
                raise ValueError("in-memory Python/Rust diagnostic parity failed")
    rss = report["rss"]
    if not isinstance(rss, dict) or rss.get("status") not in {"measured", "unavailable"}:
        raise ValueError("RSS measurement status is required")


def blocked_report(reason: str, *, fingerprint: str, run_id: str, started: str, raw_dir: Path) -> dict[str, Any]:
    now = dt.datetime.now(dt.timezone.utc).isoformat()
    return {
        "schema_version": REPORT_SCHEMA_VERSION, "run_id": run_id, "started_at_utc": started, "finished_at_utc": now,
        "status": "BLOCKED", "run_level_blocker": True, "source_fingerprint": fingerprint,
        "baseline": {"integrity": "not_verified"}, "candidate": None,
        "environment": {"python": sys.version, "platform": platform.platform()}, "capability_id": CAPABILITY,
        "workload_ids": [], "counts": {**{s: 0 for s in STATUSES}, "BLOCKED": 1},
        "parity": {"status": "not_run", "report": None}, "fresh_process": [], "in_memory": [],
        "rss": {"status": "unavailable", "method": None, "reason": "no benchmark process was measured"},
        "raw_artifacts_dir": str(raw_dir), "errors": [{"kind": "BLOCKED", "message": reason}], "performance_claim": "none",
    }


def build_release_candidate(run_dir: Path) -> tuple[list[str], Path, dict[str, Any], dict[str, str]]:
    cargo, channel = locked_cargo()
    cargo_dir = cargo.parent
    manifest = RUST_ROOT / "Cargo.toml"
    lockfile = RUST_ROOT / "Cargo.lock"
    if not manifest.is_file() or not lockfile.is_file():
        raise RuntimeError("locked Rust manifest and Cargo.lock are required")
    env = os.environ.copy()
    env.pop("RUSTFLAGS", None)
    env.update({"CARGO_HOME": str(RUST_ROOT / ".cache" / "cargo-home"), "RUSTUP_HOME": str(RUST_ROOT / ".cache" / "rustup-home"), "RUSTUP_TOOLCHAIN": channel})
    env["PATH"] = str(cargo_dir) + os.pathsep + env.get("PATH", "")
    command = [str(cargo), "build", "--release", "--locked", "--all-targets", "--manifest-path", str(manifest)]
    before = compare.source_fingerprint()
    build_dir = run_dir / "release_build"
    build_dir.mkdir(parents=True, exist_ok=True)
    started_ns = time.perf_counter_ns()
    process = subprocess.run(command, cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=1800, check=False)
    elapsed_ns = time.perf_counter_ns() - started_ns
    (build_dir / "stdout.bin").write_bytes(process.stdout)
    (build_dir / "stderr.bin").write_bytes(process.stderr)
    if process.returncode != 0:
        raise RuntimeError(f"locked release build failed ({process.returncode}); logs are in {build_dir}")
    after = compare.source_fingerprint()
    if after != before:
        raise RuntimeError("Rust source fingerprint changed during release build")
    binary = RUST_ROOT / "target" / "release" / ("proselint.exe" if os.name == "nt" else "proselint")
    helper = RUST_HELPER
    if not binary.is_file() or not helper.is_file():
        raise RuntimeError("release build did not produce CLI and in-memory example binaries")
    cargo_version = subprocess.run([str(cargo), "--version"], cwd=ROOT, env=env, check=True, capture_output=True, text=True).stdout.strip()
    rustc_version = subprocess.run([str(cargo_dir / "rustc"), "--version"], cwd=ROOT, env=env, check=True, capture_output=True, text=True).stdout.strip()
    attestation = {
        "schema_version": 1, "verified": True, "build_exit_code": 0, "source_fingerprint": after,
        "cargo_lock_sha256": sha256_file(lockfile), "binary_path": str(binary.resolve()), "binary_sha256": sha256_file(binary),
        "cargo_version": cargo_version, "rustc_version": rustc_version, "toolchain": channel,
        "build_commands": [{"command": command, "exit_code": process.returncode, "stdout_path": str(build_dir / "stdout.bin"), "stderr_path": str(build_dir / "stderr.bin")}],
        "build_duration_ns_excluded_from_timing": elapsed_ns, "helper_binary_path": str(helper.resolve()), "helper_binary_sha256": sha256_file(helper),
    }
    attestation_path = RUST_ROOT / "artifacts" / f"{run_dir.name}-release.attestation.json"
    attestation_path.write_text(json.dumps(attestation, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    argv = [str(binary.resolve())]
    tools = {"cargo": cargo_version, "rustc": rustc_version, "toolchain": channel}
    return argv, attestation_path, attestation, tools


def load_release_candidate(attestation_path: Path) -> tuple[list[str], dict[str, Any], dict[str, str]]:
    att = json.loads(attestation_path.read_text(encoding="utf-8"), object_pairs_hook=compare.unique_object)
    binary_path = Path(att.get("binary_path", ""))
    argv = [str(binary_path.resolve())]
    att = compare.read_candidate_attestation(attestation_path, argv, compare.source_fingerprint())
    if "target/release/" not in binary_path.as_posix() and "\\target\\release\\" not in str(binary_path):
        raise ValueError("candidate attestation must name a release-mode binary")
    commands = att.get("build_commands", [])
    if not any("build" in command.get("command", []) and "--release" in command.get("command", []) and "--locked" in command.get("command", []) for command in commands if isinstance(command, dict)):
        raise ValueError("candidate attestation does not prove a locked release build")
    helper = RUST_HELPER
    if not helper.is_file():
        raise ValueError("release in-memory Rust helper is missing")
    tools = {"cargo": str(att.get("cargo_version", "")), "rustc": str(att.get("rustc_version", "")), "toolchain": str(att.get("toolchain", ""))}
    return argv, att, tools


def make_env(home: Path) -> dict[str, str]:
    for name in ("xdg", "cache", "tmp"):
        (home / name).mkdir(parents=True, exist_ok=True)
    return compare.fresh_env(home, {"locale": "C.UTF-8", "benchmark_capability": CAPABILITY})


def output_diagnostic_count(result: dict[str, Any]) -> int:
    value, error = compare.decode_json_output(result["stdout"])
    if error:
        raise ValueError(f"benchmark CLI produced invalid JSON: {error}")
    schema_error = compare.validate_output_schema(value)
    if schema_error:
        raise ValueError(f"benchmark CLI output schema failed: {schema_error}")
    return sum(len(source["diagnostics"]) for source in value["result"].values())


def invoke_pair(command: list[str], cwd: Path, env: dict[str, str], timeout: float) -> dict[str, Any]:
    return compare.run_command(command, cwd=cwd, env=env, stdin=b"", timeout=timeout, role="candidate" if "target/release" in command[0] else "reference")


def record_run(result: dict[str, Any], workload: dict[str, Any], role: str, path: Path) -> dict[str, Any]:
    stdout_hash = sha256_bytes(result["stdout"]); stderr_hash = sha256_bytes(result["stderr"])
    if result["status"] != "OK":
        raise RuntimeError(f"{role} benchmark CLI failed for {workload['id']}: {result['status']} {result.get('error', '')}")
    count = output_diagnostic_count(result)
    if count != workload["expected_diagnostics_per_iteration"]:
        raise RuntimeError(f"{role} diagnostic count changed for {workload['id']}: expected {workload['expected_diagnostics_per_iteration']}, got {count}")
    return {"status": result["status"], "exit_code": result["exit_code"], "diagnostic_count": count, "stdout_sha256": stdout_hash, "stderr_sha256": stderr_hash, "duration_ns": int(result["duration_seconds"] * 1e9)}


def compare_pair(reference: dict[str, Any], candidate: dict[str, Any]) -> list[dict[str, Any]]:
    return compare.compare_outputs(reference, candidate, "json")


def timed_cli_workload(workload: dict[str, Any], manifest: dict[str, Any], ref: list[str], candidate: list[str], run_dir: Path, sample_pairs: int, warmup_pairs: int) -> dict[str, Any]:
    base = run_dir / "cli" / workload["id"]
    workspace = base / "workspace"
    workspace.mkdir(parents=True, exist_ok=True)
    config = (WORKLOADS.parent / manifest["config_file"]).read_bytes()
    document = workload_bytes(workload, manifest)
    (workspace / "selected config.json").write_bytes(config)
    input_path = "benchmark-document" + Path(workload["path"]).suffix
    (workspace / input_path).write_bytes(document)
    commands = {
        "reference": [*ref, "check", "--config", "selected config.json", "--output-format", "json", input_path],
        "candidate": [*candidate, "check", "--config", "selected config.json", "--output-format", "json", input_path],
    }
    homes = {side: base / side / "home" for side in ("reference", "candidate")}
    envs = {side: make_env(home) for side, home in homes.items()}
    for pair in range(warmup_pairs):
        results = {}
        for side in pair_order(pair):
            results[side] = invoke_pair(commands[side], workspace, envs[side], 90)
        diffs = compare_pair(results["reference"], results["candidate"])
        if diffs:
            raise RuntimeError(f"warm-up parity failed for {workload['id']}: {diffs[:3]}")
    samples = []
    for pair in range(sample_pairs):
        results = {}
        records = {}
        for side in pair_order(pair):
            results[side] = invoke_pair(commands[side], workspace, envs[side], 90)
            records[side] = record_run(results[side], workload, side, base)
        diffs = compare_pair(results["reference"], results["candidate"])
        if diffs:
            for side, result in results.items():
                (base / f"mismatch-{pair}-{side}.stdout.bin").write_bytes(result["stdout"])
                (base / f"mismatch-{pair}-{side}.stderr.bin").write_bytes(result["stderr"])
            raise RuntimeError(f"timed sample parity failed for {workload['id']} pair {pair}: {diffs[:3]}")
        samples.append({"pair_index": pair, "order": pair_order(pair), "reference": records["reference"], "candidate": records["candidate"], "outputs_match": True, "status_match": records["reference"]["exit_code"] == records["candidate"]["exit_code"], "diffs": []})
    ref_times = [sample["reference"]["duration_ns"] for sample in samples]
    rust_times = [sample["candidate"]["duration_ns"] for sample in samples]
    return {
        "workload_id": workload["id"], "kind": workload["kind"], "label": workload["label"],
        "input_sha256": workload["sha256"], "input_bytes": workload["bytes"], "sample_pairs": sample_pairs,
        "warmup_pairs_excluded": warmup_pairs, "commands": commands, "cwd": str(workspace),
        "effective_environment": {side: env for side, env in envs.items()}, "samples": samples,
        "reference_summary": summarize(ref_times), "candidate_summary": summarize(rust_times),
        "paired_speedup_median": statistics.median([a / b for a, b in zip(ref_times, rust_times, strict=True)]),
        "median_speedup": statistics.median(ref_times) / statistics.median(rust_times),
        "timed_region": "fresh executable process from spawn through exit; includes startup, input reading, lint, serialization, and output capture",
    }


def in_memory_command(side: str, workload: dict[str, Any], reps: int, manifest: dict[str, Any], python: Path, rust_helper: Path, rust_document: Path) -> tuple[list[str], dict[str, str]]:
    if side == "reference":
        return [str(python), str(PYTHON_HELPER.resolve()), str((WORKLOADS.parent / workload["path"]).resolve()), str((WORKLOADS.parent / manifest["config_file"]).resolve()), str(reps)], {"PYTHONNOUSERSITE": "1", "PYTHONUTF8": "1", "PYTHONPATH": "", "LC_ALL": "C.UTF-8", "LANG": "C.UTF-8"}
    return [str(rust_helper.resolve()), str(rust_document.resolve()), str(reps)], {"LC_ALL": "C.UTF-8", "LANG": "C.UTF-8"}


def run_in_memory_helper(command: list[str], extra_env: dict[str, str]) -> dict[str, Any]:
    env = {**os.environ, **extra_env}
    env.pop("PYTHONHOME", None)
    p = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, timeout=900, check=False)
    if p.returncode != 0 or p.stderr:
        raise RuntimeError(f"in-memory helper failed ({p.returncode}): {p.stderr.decode('utf-8', 'replace')[:1000]}")
    try:
        return json.loads(p.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"in-memory helper returned invalid JSON: {exc}") from exc


def in_memory_workload(workload: dict[str, Any], manifest: dict[str, Any], reps: int, pairs: int, warmup_pairs: int, run_dir: Path, python: Path) -> dict[str, Any]:
    doc = WORKLOADS.parent / workload["path"]
    base = run_dir / "in_memory" / workload["id"]
    base.mkdir(parents=True, exist_ok=True)
    rust_doc = base / "document.txt"
    rust_doc.write_bytes(doc.read_bytes())
    commands = {}
    for side in ("reference", "candidate"):
        commands[side], _ = in_memory_command(side, workload, reps, manifest, python, RUST_HELPER, rust_doc)
    warm_reps = min(10, reps)
    for pair in range(warmup_pairs):
        values = {}
        for side in pair_order(pair):
            warm_cmd, warm_env = in_memory_command(side, workload, warm_reps, manifest, python, RUST_HELPER, rust_doc)
            values[side] = run_in_memory_helper(warm_cmd, warm_env)
        if values["reference"].get("diagnostics") != values["candidate"].get("diagnostics"):
            raise RuntimeError(f"in-memory warm-up parity failed for {workload['id']}")
    samples = []
    last_results: dict[str, dict[str, Any]] = {}
    for pair in range(pairs):
        values = {}
        for side in pair_order(pair):
            command, env = in_memory_command(side, workload, reps, manifest, python, RUST_HELPER, rust_doc)
            values[side] = run_in_memory_helper(command, env)
        for side, value in values.items():
            diagnostics = value.get("diagnostics")
            if not isinstance(diagnostics, list) or value.get("repetitions") != reps:
                raise RuntimeError(f"malformed in-memory result from {side} for {workload['id']}")
            digest = canonical_diagnostics_digest(diagnostics)
            reported_digest = value.get("diagnostics_sha256")
            if reported_digest is not None and digest != reported_digest:
                raise RuntimeError(f"in-memory checksum mismatch inside {side} helper for {workload['id']}")
            value["diagnostics_sha256"] = digest
            if len(diagnostics) != workload["expected_diagnostics_per_iteration"] or value.get("total_diagnostics") != len(diagnostics) * reps:
                raise RuntimeError(f"in-memory diagnostic count mismatch from {side} for {workload['id']}")
            value["computed_diagnostics_sha256"] = digest
        a, b = values["reference"], values["candidate"]
        same = a["diagnostics"] == b["diagnostics"]
        checksum_match = a["diagnostics_sha256"] == b["diagnostics_sha256"]
        if not same or not checksum_match or a["diagnostics_per_iteration"] != b["diagnostics_per_iteration"]:
            raise RuntimeError(f"Python/Rust in-memory diagnostics differ for {workload['id']}")
        samples.append({"pair_index": pair, "order": pair_order(pair), "reference_elapsed_ns": a["elapsed_ns"], "candidate_elapsed_ns": b["elapsed_ns"], "diagnostics_match": same, "checksum_match": checksum_match, "diagnostics_per_iteration": a["diagnostics_per_iteration"], "total_diagnostics_each": a["total_diagnostics"], "diagnostics_sha256": a["diagnostics_sha256"]})
        last_results = values
    ref_times = [x["reference_elapsed_ns"] for x in samples]
    rust_times = [x["candidate_elapsed_ns"] for x in samples]
    return {
        "workload_id": workload["id"], "input_sha256": workload["sha256"], "input_bytes": workload["bytes"],
        "repetitions_per_sample": reps, "sample_pairs": pairs, "warmup_pairs_excluded": warmup_pairs, "commands": commands, "samples": samples,
        "reference_summary": summarize(ref_times), "candidate_summary": summarize(rust_times),
        "median_speedup": statistics.median(ref_times) / statistics.median(rust_times),
        "diagnostics_match": True, "checksum_match": True,
        "timed_region": "repeated in-memory checks only; helper startup, file reads, setup, result hashing, and JSON output are excluded",
    }


def validate_release_attestation(attestation: dict[str, Any]) -> None:
    commands = attestation.get("build_commands", [])
    if not any(isinstance(item, dict) and item.get("exit_code") == 0 and "--release" in item.get("command", []) and "--locked" in item.get("command", []) for item in commands):
        raise ValueError("release attestation must contain a successful cargo --release --locked build")
    if "target/release/" not in str(attestation.get("binary_path", "")).replace("\\", "/"):
        raise ValueError("release attestation binary must be under target/release")


def environment_info(tools: dict[str, str], baseline: dict[str, Any]) -> dict[str, Any]:
    return {
        "python": sys.version, "platform": platform.platform(), "machine": platform.machine(),
        "processor": platform.processor() or None, "cpu_count": os.cpu_count(),
        "rust_toolchain": tools, "oracle_python": baseline.get("python_version"),
        "oracle_package_version": baseline.get("dependencies", {}).get("proselint"),
    }


def run(report_path: Path, *, release_attestation: Path | None, sample_pairs: int, warmup_pairs: int, in_memory_repetitions: int, in_memory_pairs: int, parity_only: bool) -> int:
    started = dt.datetime.now(dt.timezone.utc).isoformat()
    run_id = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ") + "-benchmark-" + uuid.uuid4().hex[:8]
    raw_dir = RUST_ROOT / "artifacts" / run_id
    raw_dir.mkdir(parents=True, exist_ok=False)
    fingerprint = compare.source_fingerprint()
    report = blocked_report("benchmark setup not complete", fingerprint=fingerprint, run_id=run_id, started=started, raw_dir=raw_dir)
    try:
        if sample_pairs < 25 or warmup_pairs < 1 or in_memory_repetitions < 1 or in_memory_pairs < 25:
            raise ValueError("require >=25 fresh-process and in-memory pairs, >=1 warmup pair, and >=1 in-memory repetition")
        if CAPABILITY not in accepted_capabilities():
            raise ValueError(f"capability {CAPABILITY} is not accepted by rust/status.json")
        workload_manifest = load_workloads()
        reference = compare.default_reference_argv()
        baseline = oracle_integrity()
        if release_attestation is None:
            candidate, attestation_path, attestation, tools = build_release_candidate(raw_dir)
        else:
            candidate, attestation, tools = load_release_candidate(release_attestation)
            attestation_path = release_attestation
        validate_release_attestation(attestation)
        compare.read_candidate_attestation(attestation_path, candidate, fingerprint)
        parity_manifest = make_parity_manifest(workload_manifest, raw_dir / "benchmark-parity-cases.json")
        parity_path = raw_dir / "benchmark-parity-report.json"
        parity_code = compare.run_live(CAPABILITY, parity_path, json.dumps(candidate), json.dumps(reference), attestation_path, case_manifest_path=parity_manifest)
        parity = json.loads(parity_path.read_text(encoding="utf-8"))
        report.update({
            "baseline": baseline,
            "candidate": {"argv": candidate, "kind": "rust_build_attested_release", "binary_sha256": sha256_file(Path(candidate[0])), "build_attestation": str(attestation_path.resolve()), "build_attestation_sha256": sha256_file(attestation_path), "source_fingerprint": attestation["source_fingerprint"], "cargo_lock_sha256": attestation["cargo_lock_sha256"], "toolchain": attestation.get("toolchain"), "build_commands": attestation.get("build_commands"), "helper_binary_path": str(RUST_HELPER.resolve()), "helper_binary_sha256": sha256_file(RUST_HELPER)},
            "environment": environment_info(tools, baseline), "workload_ids": [w["id"] for w in workload_manifest["workloads"]],
            "parity": {"status": parity["status"], "report": str(parity_path.resolve()), "case_ids": parity["case_ids"], "counts": parity["counts"], "exit_code": parity_code},
        })
        if parity_code != 0 or parity["status"] != "PASS":
            report["status"] = "MISMATCH" if parity["status"] == "MISMATCH" else "BLOCKED"
            report["run_level_blocker"] = True
            report["errors"] = [{"kind": report["status"], "message": "exact benchmark workload parity failed; no timings were collected"}]
        elif parity_only:
            report["status"] = "PARITY_PASS"
            report["run_level_blocker"] = False
            report["counts"] = {**{s: 0 for s in STATUSES}, "PASS": len(workload_manifest["workloads"])}
            report["errors"] = []
            report["performance_claim"] = "none"
        else:
            python_exe = Path(reference[0]).resolve().parent / "python"
            if not python_exe.is_file() or not RUST_HELPER.is_file():
                raise RuntimeError("pinned Python or locked release Rust in-memory helper is unavailable")
            for workload in workload_manifest["workloads"]:
                report["fresh_process"].append(timed_cli_workload(workload, workload_manifest, reference, candidate, raw_dir, sample_pairs, warmup_pairs))
            for workload in workload_manifest["workloads"]:
                report["in_memory"].append(in_memory_workload(workload, workload_manifest, in_memory_repetitions, in_memory_pairs, warmup_pairs, raw_dir, python_exe))
            report["status"] = "PASS"
            report["run_level_blocker"] = False
            report["counts"] = {**{s: 0 for s in STATUSES}, "PASS": len(workload_manifest["workloads"])}
            report["errors"] = []
            report["performance_claim"] = "Per-workload measurements only; no cross-workload or universal speed claim."
        report["rss"] = {"status": "unavailable", "method": None, "reason": "No per-process RSS sampler is configured; ru_maxrss is cumulative across children and would not attribute a sample reliably."}
        if compare.source_fingerprint() != fingerprint:
            raise RuntimeError("Rust source fingerprint changed during benchmark run")
        if oracle_integrity() != baseline:
            raise RuntimeError("pinned oracle integrity changed during benchmark run")
        if sha256_file(Path(candidate[0])) != report["candidate"]["binary_sha256"]:
            raise RuntimeError("release CLI binary changed during benchmark run")
        report["finished_at_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
        validate_benchmark_report(report)
    except (OSError, RuntimeError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        report["status"] = "BLOCKED"
        report["run_level_blocker"] = True
        report["errors"] = [{"kind": type(exc).__name__, "message": str(exc)}]
        report["fresh_process"] = []
        report["in_memory"] = []
        report["performance_claim"] = "none"
        report["finished_at_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
        if report.get("parity", {}).get("status") not in {"PASS", "MISMATCH", "BLOCKED"}:
            report["parity"] = {"status": "not_run", "report": None}
        report["counts"] = {**{s: 0 for s in STATUSES}, "BLOCKED": 1}
        report["rss"] = {"status": "unavailable", "method": None, "reason": "benchmark setup or parity failed before RSS measurement"}
        validate_benchmark_report(report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"report": str(report_path), "status": report["status"], "errors": report["errors"]}, indent=2))
    return 0 if report["status"] in {"PARITY_PASS", "PASS"} else 2


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--live", action="store_true")
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--release-attestation", type=Path)
    parser.add_argument("--sample-pairs", type=int, default=25)
    parser.add_argument("--warmup-pairs", type=int, default=3)
    parser.add_argument("--in-memory-repetitions", type=int, default=None)
    parser.add_argument("--in-memory-pairs", type=int, default=None)
    parser.add_argument("--parity-only", action="store_true", help="build/verify release candidate and run exact workload parity without collecting timings")
    args = parser.parse_args()
    if not args.live:
        parser.error("benchmark requires --live")
    try:
        manifest = load_workloads()
    except (ValueError, OSError) as exc:
        print(f"BLOCKED: {exc}", file=sys.stderr)
        return 2
    repetitions = args.in_memory_repetitions or manifest["in_memory_repetitions"]
    in_memory_pairs = args.in_memory_pairs if args.in_memory_pairs is not None else manifest["in_memory_pairs"]
    return run(args.report, release_attestation=args.release_attestation, sample_pairs=args.sample_pairs, warmup_pairs=args.warmup_pairs, in_memory_repetitions=repetitions, in_memory_pairs=in_memory_pairs, parity_only=args.parity_only)


if __name__ == "__main__":
    raise SystemExit(main())
