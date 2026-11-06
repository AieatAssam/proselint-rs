#!/usr/bin/env python3
"""Strict live CLI differential comparator for the pinned Python oracle."""

from __future__ import annotations

import argparse
import base64
import datetime as dt
import hashlib
import json
import math
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import time
import uuid
import os
from pathlib import Path
from typing import Any, Sequence

from setup_oracle import ROOT, RUST_ROOT, oracle_integrity, read_lock


MANIFEST = RUST_ROOT / "harness" / "cases" / "manifest.json"
REPORT_SCHEMA_VERSION = 1
EXCLUDED_TREE_PARTS = {".cache", "artifacts", "target", ".git", "__pycache__"}
VALID_COMPARISON = {"stdout": {"bytes", "json"}, "stderr": {"bytes"}, "exit_status": {"exact"}}


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def source_fingerprint() -> str:
    root = RUST_ROOT
    digest = hashlib.sha256()
    paths: list[Path] = []
    for directory, subdirectories, filenames in os.walk(root):
        subdirectories[:] = sorted(name for name in subdirectories if name not in EXCLUDED_TREE_PARTS)
        base = Path(directory)
        paths.extend(base / name for name in filenames if name not in EXCLUDED_TREE_PARTS)
    for path in sorted(path for path in paths if path.is_file()):
        relative = path.relative_to(root).as_posix().encode("utf-8")
        content = path.read_bytes()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def load_manifest(path: Path = MANIFEST) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object, parse_constant=reject_json_constant)
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"invalid manifest: {exc}") from exc
    if not isinstance(value, dict) or type(value.get("schema_version")) is not int or value.get("schema_version") != 1:
        raise ValueError("manifest must be an object with schema_version=1")
    cases = value.get("cases")
    if not isinstance(cases, list):
        raise ValueError("manifest cases must be an array")
    seen: set[str] = set()
    for case in cases:
        validate_case(case)
        if case["id"] in seen:
            raise ValueError(f"duplicate case id: {case['id']}")
        seen.add(case["id"])
    return value


def validate_case(case: Any) -> None:
    required = {
        "id", "capability_id", "provenance", "input_mode", "files", "argv",
        "output_format", "expected_oracle_outcome", "timeout_seconds",
        "environment", "workspace_mutates", "comparison",
    }
    if not isinstance(case, dict) or required - case.keys():
        raise ValueError(f"case is missing required fields: {sorted(required - (case.keys() if isinstance(case, dict) else set()))}")
    if not isinstance(case["id"], str) or not case["id"]:
        raise ValueError("case id must be a non-empty string")
    if not isinstance(case["input_mode"], str) or case["input_mode"] not in {"stdin", "files"}:
        raise ValueError(f"{case['id']}: input_mode must be stdin or files")
    if not isinstance(case["argv"], list) or not all(isinstance(arg, str) for arg in case["argv"]):
        raise ValueError(f"{case['id']}: argv must be a string array")
    if not isinstance(case["files"], list):
        raise ValueError(f"{case['id']}: files must be an array")
    if not isinstance(case["workspace_mutates"], bool):
        raise ValueError(f"{case['id']}: workspace_mutates must be boolean")
    if case["input_mode"] == "stdin":
        if not isinstance(case.get("stdin_base64"), str):
            raise ValueError(f"{case['id']}: stdin cases need stdin_base64")
        try:
            base64.b64decode(case["stdin_base64"], validate=True)
        except ValueError as exc:
            raise ValueError(f"{case['id']}: stdin_base64 is invalid") from exc
    if not isinstance(case["timeout_seconds"], (int, float)) or isinstance(case["timeout_seconds"], bool) or not math.isfinite(case["timeout_seconds"]) or case["timeout_seconds"] <= 0:
        raise ValueError(f"{case['id']}: timeout_seconds must be positive")
    if not isinstance(case["environment"], dict) or not isinstance(case["comparison"], dict):
        raise ValueError(f"{case['id']}: environment and comparison must be objects")
    for key, allowed in VALID_COMPARISON.items():
        if not isinstance(case["comparison"].get(key), str) or case["comparison"][key] not in allowed:
            raise ValueError(f"{case['id']}: comparison.{key} must be one of {sorted(allowed)}")
    if not isinstance(case["comparison"].get("normalize_workspace_uri", False), bool):
        raise ValueError(f"{case['id']}: normalize_workspace_uri must be boolean")
    output_format = case["output_format"]
    if not isinstance(output_format, str) or output_format not in {"json", "full", "compact"}:
        raise ValueError(f"{case['id']}: output_format must be json, full, or compact")
    format_args = []
    for index, arg in enumerate(case["argv"]):
        if arg == "--output-format":
            if index + 1 >= len(case["argv"]):
                raise ValueError(f"{case['id']}: --output-format needs a value")
            format_args.append(case["argv"][index + 1])
        elif arg.startswith("--output-format="):
            format_args.append(arg.partition("=")[2])
    if len(format_args) > 1 or (format_args and format_args[0] != output_format):
        raise ValueError(f"{case['id']}: output_format must match at most one --output-format argv option")
    if not format_args and output_format != "full":
        raise ValueError(f"{case['id']}: only full is the default when --output-format is omitted")
    expected_stdout_policy = "json" if output_format == "json" else "bytes"
    if case["comparison"]["stdout"] != expected_stdout_policy:
        raise ValueError(f"{case['id']}: stdout comparison policy must be {expected_stdout_policy} for {output_format}")
    if not isinstance(case["expected_oracle_outcome"], str) or case["expected_oracle_outcome"] not in {"clean", "findings", "cli_error"}:
        raise ValueError(f"{case['id']}: unsupported expected_oracle_outcome")
    for file in case["files"]:
        if not isinstance(file, dict) or not isinstance(file.get("path"), str) or not isinstance(file.get("content_base64"), str):
            raise ValueError(f"{case['id']}: each file needs path and content_base64")
        rel = Path(file["path"])
        if rel.is_absolute() or ".." in rel.parts:
            raise ValueError(f"{case['id']}: file paths must stay inside workspace")
        try:
            base64.b64decode(file["content_base64"], validate=True)
        except ValueError as exc:
            raise ValueError(f"{case['id']}: invalid file base64 for {file['path']}") from exc
    unused_files = case.get("unused_files", [])
    if not isinstance(unused_files, list) or not all(isinstance(path, str) for path in unused_files):
        raise ValueError(f"{case['id']}: unused_files must be a string array")
    if len(unused_files) != len(set(unused_files)):
        raise ValueError(f"{case['id']}: unused_files must not contain duplicates")
    file_paths = {item["path"] for item in case["files"]}
    if not set(unused_files).issubset(file_paths):
        raise ValueError(f"{case['id']}: unused_files contains an undeclared fixture")
    consumed = set()
    for path in file_paths:
        marker = "{file:" + path + "}"
        if any(marker in arg or arg == path or arg.endswith("=" + path) for arg in case["argv"]):
            consumed.add(path)
    unclassified = file_paths - consumed - set(unused_files)
    if unclassified:
        raise ValueError(f"{case['id']}: file fixtures are neither consumed by argv nor declared unused: {sorted(unclassified)}")
    if consumed.intersection(unused_files):
        raise ValueError(f"{case['id']}: a fixture cannot be both consumed and declared unused")
    if case["input_mode"] == "files" and not case["files"]:
        raise ValueError(f"{case['id']}: files input requires at least one file")


def decode_json_output(data: bytes) -> tuple[Any | None, str | None]:
    try:
        text = data.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        return None, f"stdout is not UTF-8 at byte {exc.start}"
    try:
        value = json.loads(text, object_pairs_hook=unique_object, parse_constant=reject_json_constant)
    except json.JSONDecodeError as exc:
        return None, f"invalid JSON at line {exc.lineno} column {exc.colno}: {exc.msg}"
    except ValueError as exc:
        return None, f"invalid JSON: {exc}"
    if not isinstance(value, dict):
        return None, "JSON output root is not an object"
    return value, None


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key {key!r}")
        result[key] = value
    return result


def reject_json_constant(value: str) -> None:
    raise ValueError(f"non-standard JSON constant {value}")


def validate_output_schema(value: Any) -> str | None:
    if not isinstance(value, dict) or not set(value).issubset({"result", "error"}):
        return "JSON root must contain only result and/or error"
    if ("result" in value) == ("error" in value):
        return "JSON root must contain exactly one of result or error"
    if "error" in value:
        if not valid_error(value["error"]):
            return "error must be a ResponseError object with code/message and optional data"
        return None
    if not isinstance(value["result"], dict):
        return "result must be an object keyed by source"
    for source, result in value["result"].items():
        if not isinstance(source, str) or not isinstance(result, dict):
            return f"result entry {source!r} must be an object"
        if set(result) == {"error"}:
            if not valid_error(result["error"]):
                return f"error entry {source!r} has an invalid ResponseError"
            continue
        if set(result) != {"diagnostics"}:
            return f"result entry {source!r} must contain diagnostics or error"
        diagnostics = result["diagnostics"]
        if not isinstance(diagnostics, list):
            return f"diagnostics for {source!r} must be an array"
        for index, diagnostic in enumerate(diagnostics):
            required = {"check_path", "message", "span", "pos", "replacements"}
            if not isinstance(diagnostic, dict) or set(diagnostic) != required:
                return f"diagnostic {source}[{index}] must contain exactly {sorted(required)}"
            if not isinstance(diagnostic["check_path"], str) or not isinstance(diagnostic["message"], str):
                return f"diagnostic {source}[{index}] check_path and message must be strings"
            if not valid_position(diagnostic["span"]) or not valid_position(diagnostic["pos"]):
                return f"diagnostic {source}[{index}] span and pos must be integer pairs"
            if diagnostic["replacements"] is not None and not isinstance(diagnostic["replacements"], str):
                return f"diagnostic {source}[{index}] replacements must be a string or null"
    return None


def valid_error(value: Any) -> bool:
    if not isinstance(value, dict) or not {"code", "message"}.issubset(value) or not set(value).issubset({"code", "message", "data"}):
        return False
    return isinstance(value["code"], int) and not isinstance(value["code"], bool) and isinstance(value["message"], str)


def valid_position(value: Any) -> bool:
    return isinstance(value, list) and len(value) == 2 and all(isinstance(n, int) and not isinstance(n, bool) for n in value)


def value_diffs(expected: Any, actual: Any, path: str = "$", diffs: list[dict[str, Any]] | None = None) -> list[dict[str, Any]]:
    if diffs is None:
        diffs = []
    if type(expected) is not type(actual):
        diffs.append({"field": path, "expected": expected, "actual": actual, "kind": "type"})
    elif isinstance(expected, dict):
        expected_keys = set(expected)
        actual_keys = set(actual)
        for key in sorted(expected_keys - actual_keys):
            diffs.append({"field": f"{path}.{key}", "expected": expected[key], "actual": "<absent>", "kind": "missing_field"})
        for key in sorted(actual_keys - expected_keys):
            diffs.append({"field": f"{path}.{key}", "expected": "<absent>", "actual": actual[key], "kind": "additional_field"})
        for key in sorted(expected_keys & actual_keys):
            value_diffs(expected[key], actual[key], f"{path}.{key}", diffs)
    elif isinstance(expected, list):
        if len(expected) != len(actual):
            diffs.append({"field": path, "expected_count": len(expected), "actual_count": len(actual), "kind": "array_length"})
        for index, (left, right) in enumerate(zip(expected, actual)):
            value_diffs(left, right, f"{path}[{index}]", diffs)
    elif expected != actual:
        diffs.append({"field": path, "expected": expected, "actual": actual, "kind": "value"})
    return diffs


def compare_outputs(
    expected: dict[str, Any],
    actual: dict[str, Any],
    output_policy: str,
    workspace_uris: tuple[str, str] | None = None,
) -> list[dict[str, Any]]:
    diffs: list[dict[str, Any]] = []
    if expected["exit_code"] != actual["exit_code"]:
        diffs.append({"field": "exit_code", "expected": expected["exit_code"], "actual": actual["exit_code"], "kind": "exit_code_mismatch"})
    if expected["stderr"] != actual["stderr"]:
        diffs.append({
            "field": "stderr", "kind": "stderr_mismatch",
            "expected_sha256": sha256_bytes(expected["stderr"]),
            "actual_sha256": sha256_bytes(actual["stderr"]),
        })
    if output_policy == "json":
        left, left_error = decode_json_output(expected["stdout"])
        right, right_error = decode_json_output(actual["stdout"])
        if left_error:
            diffs.append({"field": "reference.stdout", "kind": "INVALID_JSON", "detail": left_error})
        if right_error:
            diffs.append({"field": "candidate.stdout", "kind": "INVALID_JSON", "detail": right_error})
        if not left_error and not right_error:
            schema_left = validate_output_schema(left)
            schema_right = validate_output_schema(right)
            if schema_left:
                diffs.append({"field": "reference.stdout", "kind": "JSON_SCHEMA", "detail": schema_left})
            if schema_right:
                diffs.append({"field": "candidate.stdout", "kind": "JSON_SCHEMA", "detail": schema_right})
            if not schema_left and not schema_right:
                if workspace_uris is not None:
                    left = normalize_source_keys(left, workspace_uris[0])
                    right = normalize_source_keys(right, workspace_uris[1])
                diffs.extend(value_diffs(left, right))
    elif expected["stdout"] != actual["stdout"]:
        diffs.append({"field": "stdout", "kind": "stdout_bytes_mismatch", "expected_sha256": sha256_bytes(expected["stdout"]), "actual_sha256": sha256_bytes(actual["stdout"])})
    return diffs


def normalize_source_keys(value: dict[str, Any], workspace_uri: str) -> dict[str, Any]:
    """Replace one exact case workspace prefix in source map keys only."""
    copied = json.loads(json.dumps(value))
    if "result" in copied:
        result = copied["result"]
        for key in list(result):
            if key.startswith(workspace_uri + "/"):
                replacement = "<workspace>" + key[len(workspace_uri):]
                result[replacement] = result.pop(key)
    return copied


def run_command(argv: Sequence[str], *, cwd: Path, env: dict[str, str], stdin: bytes, timeout: float, role: str) -> dict[str, Any]:
    started = time.monotonic()
    result: dict[str, Any] = {"argv": list(argv), "cwd": str(cwd), "role": role, "status": "OK"}
    try:
        process = subprocess.run(
            list(argv),
            input=stdin,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=cwd,
            env=env,
            timeout=timeout,
            shell=False,
            check=False,
        )
    except subprocess.TimeoutExpired as exc:
        result.update({"status": "TIMEOUT", "exit_code": None, "stdout": exc.stdout or b"", "stderr": exc.stderr or b"", "error": f"deadline exceeded after {timeout}s"})
    except OSError as exc:
        result.update({"status": f"{role.upper()}_ERROR", "exit_code": None, "stdout": b"", "stderr": b"", "error": f"{type(exc).__name__}: {exc}"})
    else:
        result.update({"exit_code": process.returncode, "stdout": process.stdout, "stderr": process.stderr})
        if process.returncode < 0:
            result.update({"status": f"{role.upper()}_ERROR", "error": f"terminated by signal {-process.returncode}"})
        elif process.returncode not in {0, 1, 2, 3, 4}:
            result.update({"status": f"{role.upper()}_ERROR", "error": f"unexpected process exit status {process.returncode}"})
    result["duration_seconds"] = time.monotonic() - started
    return result


def outcome(exit_code: int | None) -> str:
    if exit_code == 0:
        return "clean"
    if exit_code == 1:
        return "findings"
    return "cli_error"


def fresh_env(home: Path, extra: dict[str, str]) -> dict[str, str]:
    env = {
        "PATH": os.environ.get("PATH", ""),
        "HOME": str(home),
        "XDG_CONFIG_HOME": str(home / "xdg"),
        "XDG_CACHE_HOME": str(home / "cache"),
        "TMPDIR": str(home / "tmp"),
        "PYTHONNOUSERSITE": "1",
        "PYTHONUTF8": "1",
        "LC_ALL": "C.UTF-8",
        "LANG": "C.UTF-8",
    }
    for key in ("PYTHONPATH", "PYTHONHOME", "PROSELINT_CONFIG"):
        env.pop(key, None)
    for key, value in extra.items():
        if key not in {"HOME", "XDG_CONFIG_HOME", "PYTHONPATH", "PYTHONHOME"}:
            env[key] = value
    return env


def materialize_files(case: dict[str, Any], workspace: Path) -> None:
    for item in case["files"]:
        target = workspace / item["path"]
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(base64.b64decode(item["content_base64"], validate=True))


def run_case(
    case: dict[str, Any],
    reference_argv: list[str],
    candidate_argv: list[str],
    run_dir: Path,
    attestation_path: Path | None,
) -> dict[str, Any]:
    if not reference_argv or not candidate_argv:
        raise ValueError("executable argv arrays cannot be empty")
    if not Path(reference_argv[0]).is_absolute() or not Path(candidate_argv[0]).is_absolute():
        raise ValueError("reference and candidate executable paths must be absolute")
    case_dir = run_dir / "cases" / case["id"]
    case_dir.mkdir(parents=True, exist_ok=False)
    replay_manifest = case_dir / "replay-manifest.json"
    replay_manifest.write_text(json.dumps({"schema_version": 1, "cases": [case]}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    layout = case_dir / "input-layout"
    layout.mkdir()
    materialize_files(case, layout)
    stdin = base64.b64decode(case.get("stdin_base64", ""), validate=True)
    (case_dir / "stdin.bin").write_bytes(stdin)
    args = list(case["argv"])
    for index, value in enumerate(args):
        for item in case["files"]:
            value = value.replace("{file:" + item["path"] + "}", item["path"])
        args[index] = value
    commands = []
    executions = []
    with tempfile.TemporaryDirectory(prefix="proselint harness space-é ") as temp_root:
        temp_base = Path(temp_root)
        for role, executable in (("reference", reference_argv), ("candidate", candidate_argv)):
            role_workspace = temp_base / role if case["workspace_mutates"] else temp_base / "shared"
            if not role_workspace.exists():
                shutil.copytree(layout, role_workspace)
            home = case_dir / role / "home"
            for subdir in ("xdg", "cache", "tmp"):
                (home / subdir).mkdir(parents=True, exist_ok=True)
            argv = executable + args
            env = fresh_env(home, case["environment"])
            commands.append(argv)
            run = run_command(argv, cwd=role_workspace, env=env, stdin=stdin, timeout=case["timeout_seconds"], role=role)
            run["workspace"] = str(role_workspace)
            executions.append(run)
        workspace_paths = tuple(Path(run["workspace"]) for run in executions)
        workspace_uris = (workspace_paths[0].as_uri(), workspace_paths[1].as_uri())
        if case["workspace_mutates"]:
            for path, role in zip(workspace_paths, ("reference", "candidate"), strict=True):
                shutil.copytree(path, case_dir / f"{role}-input-layout")
        else:
            shutil.copytree(workspace_paths[0], case_dir / "materialized-workspace")
    left, right = executions
    status = "PASS"
    diffs: list[dict[str, Any]] = []
    if left["status"] != "OK":
        status = "ORACLE_ERROR" if left["status"] != "TIMEOUT" else "TIMEOUT"
    elif outcome(left["exit_code"]) != case["expected_oracle_outcome"]:
        status = "ORACLE_ERROR"
        diffs.append({"field": "oracle.outcome", "expected": case["expected_oracle_outcome"], "actual": outcome(left["exit_code"]), "kind": "unexpected_oracle_outcome"})
    elif right["status"] != "OK":
        status = right["status"]
    else:
        normalization = workspace_uris if case["comparison"].get("normalize_workspace_uri", False) else None
        diffs = compare_outputs(left, right, case["comparison"]["stdout"], normalization)
        if diffs:
            status = "MISMATCH"
    for role, execution in zip(("reference", "candidate"), executions, strict=True):
        stem = case_dir / role
        (stem.with_suffix(".stdout.bin")).write_bytes(execution["stdout"])
        (stem.with_suffix(".stderr.bin")).write_bytes(execution["stderr"])
        execution["stdout_path"] = str(stem.with_suffix(".stdout.bin"))
        execution["stderr_path"] = str(stem.with_suffix(".stderr.bin"))
        execution["stdout_sha256"] = sha256_bytes(execution["stdout"])
        execution["stderr_sha256"] = sha256_bytes(execution["stderr"])
        del execution["stdout"]
        del execution["stderr"]
    if attestation_path is None:
        replay_command = [
            "python3", "rust/harness/compare.py", "--oracle-self-check", "--live",
            "--report", "<new-report.json>",
        ]
    else:
        replay_command = [
            "python3", "rust/harness/compare.py", "--case-manifest", str(replay_manifest),
            "--live", "--candidate-argv-json", json.dumps(candidate_argv),
            "--candidate-attestation", str(attestation_path.resolve()),
            "--report", "<new-report.json>",
        ]
    return {
        "case_id": case["id"],
        "capability_id": case["capability_id"],
        "status": status,
        "expected_oracle_outcome": case["expected_oracle_outcome"],
        "commands": commands,
        "stdin_sha256": sha256_bytes(stdin),
        "workspace_layout": str(layout),
        "workspace_mutates": case["workspace_mutates"],
        "normalization": {"source_key_workspace_uri": bool(case["comparison"].get("normalize_workspace_uri", False))},
        "effective_environment": {
            role: {
                **fresh_env(case_dir / role / "home", case["environment"]),
                "config_scenario": case.get("config_scenario", "isolated_default"),
            }
            for role in ("reference", "candidate")
        },
        "executions": executions,
        "diffs": diffs,
        "replay": {
            "command": replay_command,
            "saved_case_manifest": str(replay_manifest),
            "saved_stdin": str(case_dir / "stdin.bin"),
            "saved_layout": str(layout),
            "candidate_argv": candidate_argv,
            "requires_same_candidate_binary_and_build_attestation": not (attestation_path is None),
        },
    }


def parse_argv_json(value: str | None, fallback: list[str]) -> list[str]:
    if value is None:
        argv = fallback
    else:
        try:
            argv = json.loads(value, object_pairs_hook=unique_object)
        except json.JSONDecodeError as exc:
            raise ValueError(f"invalid executable argv JSON: {exc}") from exc
    if not isinstance(argv, list) or not argv or not all(isinstance(item, str) for item in argv):
        raise ValueError("executable argv must be a non-empty JSON string array")
    if not Path(argv[0]).is_absolute():
        raise ValueError("executable path must be absolute")
    if not Path(argv[0]).is_file():
        raise ValueError(f"executable does not exist: {argv[0]}")
    return argv


def default_reference_argv() -> list[str]:
    lock = read_lock()
    locked_argv = lock.get("reference_argv")
    if not isinstance(locked_argv, list) or not locked_argv or not all(isinstance(arg, str) for arg in locked_argv):
        raise RuntimeError("oracle.lock.json reference_argv must be a non-empty string array")
    executable = Path(locked_argv[0])
    absolute = executable if executable.is_absolute() else RUST_ROOT / executable
    return [str(absolute.resolve()), *locked_argv[1:]]


def require_locked_reference_argv(reference: list[str], pinned: list[str]) -> None:
    if reference != pinned:
        raise ValueError("reference argv must exactly match oracle.lock.json reference_argv")


def json_report_value(case_result: dict[str, Any]) -> dict[str, Any]:
    return case_result


def validate_report(report: dict[str, Any]) -> None:
    required = {
        "schema_version", "run_id", "started_at_utc", "finished_at_utc", "status",
        "source_fingerprint", "baseline", "candidate", "environment", "capability_ids",
        "case_ids", "counts", "cases", "raw_artifacts_dir", "self_test", "errors", "run_level_blocker",
    }
    if not isinstance(report, dict) or type(report.get("schema_version")) is not int or report.get("schema_version") != REPORT_SCHEMA_VERSION or required - report.keys():
        raise ValueError("report does not satisfy schema version 1")
    case_statuses = {"PASS", "MISMATCH", "NOT_IMPLEMENTED", "ORACLE_ERROR", "CANDIDATE_ERROR", "TIMEOUT", "BLOCKED"}
    report_statuses = {"PASS", "MISMATCH", "BLOCKED"}
    if not isinstance(report["status"], str) or report["status"] not in report_statuses:
        raise ValueError(f"invalid report status {report['status']!r}")
    if not isinstance(report["run_level_blocker"], bool):
        raise ValueError("run_level_blocker must be boolean")
    for field in ("run_id", "started_at_utc", "finished_at_utc", "raw_artifacts_dir"):
        if not isinstance(report[field], str) or not report[field]:
            raise ValueError(f"report {field} must be a non-empty string")
    for field in ("started_at_utc", "finished_at_utc"):
        try:
            dt.datetime.fromisoformat(report[field])
        except ValueError as exc:
            raise ValueError(f"report {field} must be an ISO timestamp") from exc
    if not _is_sha256(report["source_fingerprint"]):
        raise ValueError("report source_fingerprint must be a SHA-256 hex digest")
    if report.get("scope") is not None and (not isinstance(report["scope"], str) or report["scope"] not in {"harness_health_only", "compatibility_gate"}):
        raise ValueError("report scope is invalid")
    _validate_baseline(report["baseline"])
    if report["candidate"] is not None:
        _validate_report_candidate(report["candidate"])
    if not isinstance(report["environment"], dict) or not all(isinstance(report["environment"].get(key), str) for key in ("python", "platform")):
        raise ValueError("report environment must include Python and platform strings")
    if not isinstance(report["capability_ids"], list) or not all(isinstance(value, str) and value for value in report["capability_ids"]):
        raise ValueError("capability_ids must be a string array")
    if len(report["capability_ids"]) != len(set(report["capability_ids"])):
        raise ValueError("capability_ids must be unique")
    if not isinstance(report["case_ids"], list) or not all(isinstance(value, str) and value for value in report["case_ids"]):
        raise ValueError("case_ids must be a string array")
    if not isinstance(report["cases"], list):
        raise ValueError("report cases must be an array")
    ids = []
    observed = {status: 0 for status in case_statuses}
    for case in report["cases"]:
        _validate_report_case(case, case_statuses)
        ids.append(case["case_id"])
        observed[case["status"]] += 1
    if len(ids) != len(set(ids)) or ids != report["case_ids"]:
        raise ValueError("report case IDs must be unique and agree with case records")
    expected_capabilities = sorted({case["capability_id"] for case in report["cases"]})
    if sorted(report["capability_ids"]) != expected_capabilities:
        raise ValueError("report capability IDs must agree with case records")
    if not isinstance(report["counts"], dict) or set(report["counts"]) != set(case_statuses):
        raise ValueError("counts must contain exactly the defined status keys")
    if any(not isinstance(count, int) or isinstance(count, bool) or count < 0 for count in report["counts"].values()):
        raise ValueError("status counts must be non-negative integers")
    if report["run_level_blocker"]:
        observed["BLOCKED"] += 1
    if observed != report["counts"]:
        raise ValueError("report status counts disagree with case records")
    if not isinstance(report["errors"], list):
        raise ValueError("report errors must be an array")
    for error in report["errors"]:
        if not isinstance(error, dict) or not isinstance(error.get("kind"), str) or not isinstance(error.get("message"), str):
            raise ValueError("each report error must contain string kind and message")
    if report["run_level_blocker"] != (report["status"] == "BLOCKED"):
        raise ValueError("BLOCKED status and run_level_blocker must agree")
    if report["run_level_blocker"] and not report["errors"]:
        raise ValueError("a run-level blocker must include an error record")
    if not report["run_level_blocker"]:
        if not ids:
            raise ValueError("a non-blocked live report must include at least one case")
        if report.get("scope") not in {"harness_health_only", "compatibility_gate"}:
            raise ValueError("a non-blocked report must identify its scope")
        if not isinstance(report["baseline"], dict) or report["baseline"].get("integrity") != "verified":
            raise ValueError("a non-blocked report requires a verified live baseline")
        if not isinstance(report["candidate"], dict):
            raise ValueError("a non-blocked report requires candidate identity")
        if report.get("scope") == "harness_health_only" and report["candidate"]["kind"] != "pinned_reference_self_comparison":
            raise ValueError("harness health report must identify oracle self-comparison")
        if report.get("scope") == "compatibility_gate" and report["candidate"]["kind"] != "rust_build_attested":
            raise ValueError("compatibility report must identify an attested Rust candidate")
        expected_status = "PASS" if all(case["status"] == "PASS" for case in report["cases"]) else "MISMATCH"
        if report["status"] != expected_status:
            raise ValueError("top-level status disagrees with case outcomes")
        if report["errors"]:
            raise ValueError("non-blocked report cannot contain run-level errors")
    _validate_self_test(report["self_test"])


def _is_sha256(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(char in "0123456789abcdef" for char in value)


def _validate_report_candidate(candidate: Any) -> None:
    if not isinstance(candidate, dict):
        raise ValueError("candidate must be an object or null")
    if not isinstance(candidate.get("kind"), str) or candidate["kind"] not in {"pinned_reference_self_comparison", "rust_build_attested"}:
        raise ValueError("candidate kind is invalid")
    argv = candidate.get("argv")
    if not isinstance(argv, list) or not argv or not all(isinstance(arg, str) for arg in argv):
        raise ValueError("candidate argv must be a non-empty string array")
    if not _is_sha256(candidate.get("executable_sha256")):
        raise ValueError("candidate executable_sha256 is invalid")
    if candidate["kind"] == "pinned_reference_self_comparison":
        if candidate.get("parity_evidence") is not False or candidate.get("source_fingerprint") is not None:
            raise ValueError("self-comparison candidate must not claim Rust parity evidence")
    else:
        if not _is_sha256(candidate.get("source_fingerprint")) or not _is_sha256(candidate.get("cargo_lock_sha256")):
            raise ValueError("Rust candidate identity must include source and Cargo.lock hashes")
        if not _is_sha256(candidate.get("build_attestation_sha256")):
            raise ValueError("Rust candidate build_attestation_sha256 must be a SHA-256 hex digest")
        for field in ("build_attestation", "toolchain"):
            if not isinstance(candidate.get(field), str) or not candidate[field]:
                raise ValueError(f"Rust candidate {field} must be a non-empty string")
        if not isinstance(candidate.get("build_commands"), list) or not candidate["build_commands"]:
            raise ValueError("Rust candidate build_commands must be a non-empty array")
        for build in candidate["build_commands"]:
            if not isinstance(build, dict) or not isinstance(build.get("command"), list) or not build["command"] or not all(isinstance(arg, str) for arg in build["command"]):
                raise ValueError("Rust candidate build command record is malformed")
            if build.get("exit_code") != 0 or not isinstance(build.get("exit_code"), int) or isinstance(build.get("exit_code"), bool):
                raise ValueError("Rust candidate build command must have exit status 0")
            for field in ("stdout_path", "stderr_path"):
                if not isinstance(build.get(field), str) or not build[field]:
                    raise ValueError(f"Rust candidate build command {field} is invalid")


def _validate_baseline(baseline: Any) -> None:
    if baseline is None:
        return
    if not isinstance(baseline, dict):
        raise ValueError("baseline must be an object or null")
    if baseline.get("integrity") == "unavailable":
        if not isinstance(baseline.get("error"), str) or not baseline["error"]:
            raise ValueError("unavailable baseline must include an error string")
        return
    required = {"integrity", "revision", "source_tree_sha256", "uv_lock_sha256", "executable", "executable_sha256", "python", "python_version", "installed_package_tree_sha256", "dependencies", "module_provenance"}
    if required - baseline.keys() or baseline["integrity"] != "verified":
        raise ValueError("verified baseline is missing integrity identity fields")
    for field in ("revision", "executable", "python", "python_version"):
        if not isinstance(baseline[field], str) or not baseline[field]:
            raise ValueError(f"baseline {field} must be a non-empty string")
    for field in ("source_tree_sha256", "uv_lock_sha256", "executable_sha256", "installed_package_tree_sha256"):
        if not _is_sha256(baseline[field]):
            raise ValueError(f"baseline {field} must be a SHA-256 hex digest")
    for field in ("dependencies", "module_provenance"):
        mapping = baseline[field]
        if not isinstance(mapping, dict) or not mapping or not all(isinstance(key, str) and key and isinstance(value, str) and value for key, value in mapping.items()):
            raise ValueError(f"baseline {field} must be a non-empty string map")


def _validate_report_case(case: Any, allowed_statuses: set[str]) -> None:
    required = {
        "case_id", "capability_id", "status", "expected_oracle_outcome", "commands", "stdin_sha256",
        "workspace_layout", "workspace_mutates", "normalization", "effective_environment", "executions",
        "diffs", "replay",
    }
    if not isinstance(case, dict) or required - case.keys():
        raise ValueError("case report is missing required fields")
    for field in ("case_id", "capability_id", "workspace_layout"):
        if not isinstance(case[field], str) or not case[field]:
            raise ValueError(f"case report {field} must be a non-empty string")
    if not isinstance(case["status"], str) or case["status"] not in allowed_statuses:
        raise ValueError("case report status is invalid")
    if not isinstance(case["expected_oracle_outcome"], str) or case["expected_oracle_outcome"] not in {"clean", "findings", "cli_error"}:
        raise ValueError("case expected_oracle_outcome is invalid")
    if not _is_sha256(case["stdin_sha256"]):
        raise ValueError("case stdin_sha256 is invalid")
    if not isinstance(case["workspace_mutates"], bool):
        raise ValueError("case workspace_mutates must be boolean")
    normalization = case["normalization"]
    if not isinstance(normalization, dict) or set(normalization) != {"source_key_workspace_uri"} or not isinstance(normalization["source_key_workspace_uri"], bool):
        raise ValueError("case normalization schema is invalid")
    commands = case["commands"]
    if not isinstance(commands, list) or len(commands) != 2 or any(not isinstance(argv, list) or not argv or not all(isinstance(arg, str) for arg in argv) for argv in commands):
        raise ValueError("case commands must contain two non-empty argv arrays")
    environments = case["effective_environment"]
    if not isinstance(environments, dict) or set(environments) != {"reference", "candidate"}:
        raise ValueError("case effective_environment must contain reference and candidate")
    for env in environments.values():
        if not isinstance(env, dict) or not all(isinstance(key, str) and isinstance(value, str) for key, value in env.items()):
            raise ValueError("case effective environment values must be strings")
        if not isinstance(env.get("config_scenario"), str) or not env["config_scenario"]:
            raise ValueError("case environment must identify the config scenario")
    executions = case["executions"]
    if not isinstance(executions, list) or len(executions) != 2:
        raise ValueError("case executions must contain reference and candidate records")
    for expected_role, execution, command in zip(("reference", "candidate"), executions, commands, strict=True):
        _validate_execution(execution, expected_role)
        if execution["argv"] != command:
            raise ValueError("case command argv differs from execution argv")
    reference, candidate = executions
    status = case["status"]
    if status == "PASS" and (reference["status"] != "OK" or candidate["status"] != "OK" or case["diffs"]):
        raise ValueError("PASS case must have two successful executions and no diffs")
    if status == "MISMATCH" and (reference["status"] != "OK" or candidate["status"] != "OK" or not case["diffs"]):
        raise ValueError("MISMATCH case must have two successful executions and at least one diff")
    if status == "ORACLE_ERROR" and reference["status"] == "OK" and not case["diffs"]:
        raise ValueError("ORACLE_ERROR case must record a reference execution or outcome error")
    if status == "CANDIDATE_ERROR" and candidate["status"] != "CANDIDATE_ERROR":
        raise ValueError("CANDIDATE_ERROR case must record a candidate execution error")
    if status == "TIMEOUT" and "TIMEOUT" not in {reference["status"], candidate["status"]}:
        raise ValueError("TIMEOUT case must record a timed out execution")
    if status == "NOT_IMPLEMENTED" and candidate["status"] not in {"OK", "CANDIDATE_ERROR"}:
        raise ValueError("NOT_IMPLEMENTED case has an invalid candidate execution status")
    if not isinstance(case["diffs"], list):
        raise ValueError("case diffs must be an array")
    for diff in case["diffs"]:
        if (not isinstance(diff, dict) or not isinstance(diff.get("field"), str) or not diff["field"]
                or not isinstance(diff.get("kind"), str) or not diff["kind"]):
            raise ValueError("each case diff must contain string field and kind")
        if "detail" in diff and not isinstance(diff["detail"], str):
            raise ValueError("case diff detail must be a string")
    replay = case["replay"]
    if not isinstance(replay, dict) or not isinstance(replay.get("command"), list) or not replay["command"] or not all(isinstance(arg, str) for arg in replay["command"]):
        raise ValueError("case replay must contain a string argv array")
    for field in ("saved_case_manifest", "saved_stdin", "saved_layout"):
        if not isinstance(replay.get(field), str) or not replay[field]:
            raise ValueError(f"case replay {field} must be a non-empty string")
    if not isinstance(replay.get("candidate_argv"), list) or not all(isinstance(arg, str) for arg in replay["candidate_argv"]):
        raise ValueError("case replay candidate_argv must be a string array")
    if not isinstance(replay.get("requires_same_candidate_binary_and_build_attestation"), bool):
        raise ValueError("case replay attestation requirement must be boolean")


def _validate_execution(execution: Any, expected_role: str) -> None:
    required = {
        "argv", "cwd", "role", "status", "duration_seconds", "exit_code", "stdout_path", "stderr_path",
        "stdout_sha256", "stderr_sha256",
    }
    if not isinstance(execution, dict) or required - execution.keys():
        raise ValueError("execution report is missing required fields")
    if not isinstance(execution["role"], str) or execution["role"] != expected_role:
        raise ValueError("execution roles must be reference then candidate")
    if not isinstance(execution["status"], str) or execution["status"] not in {"OK", "TIMEOUT", "REFERENCE_ERROR", "CANDIDATE_ERROR"}:
        raise ValueError("execution status is invalid")
    if not isinstance(execution["argv"], list) or not execution["argv"] or not all(isinstance(arg, str) for arg in execution["argv"]):
        raise ValueError("execution argv must be a non-empty string array")
    for field in ("cwd", "stdout_path", "stderr_path"):
        if not isinstance(execution[field], str) or not execution[field]:
            raise ValueError(f"execution {field} must be a non-empty string")
    duration = execution["duration_seconds"]
    if not isinstance(duration, (int, float)) or isinstance(duration, bool) or not math.isfinite(duration) or duration < 0:
        raise ValueError("execution duration_seconds must be a finite non-negative number")
    exit_code = execution["exit_code"]
    if exit_code is not None and (not isinstance(exit_code, int) or isinstance(exit_code, bool)):
        raise ValueError("execution exit_code must be an integer or null")
    if execution["status"] == "OK" and exit_code is None:
        raise ValueError("successful execution must contain an exit code")
    if execution["status"] == "OK" and exit_code not in {0, 1, 2, 3, 4}:
        raise ValueError("successful execution must have an accepted CLI exit status")
    if execution["status"] == "TIMEOUT" and exit_code is not None:
        raise ValueError("timed out execution must have a null exit code")
    if execution["status"] in {"REFERENCE_ERROR", "CANDIDATE_ERROR"} and execution["exit_code"] is None and not execution.get("error"):
        raise ValueError("failed execution must retain an error description")
    if execution["status"] != "OK" and not isinstance(execution.get("error"), str):
        raise ValueError("failed execution must describe its error")
    if not _is_sha256(execution["stdout_sha256"]) or not _is_sha256(execution["stderr_sha256"]):
        raise ValueError("execution output hashes must be SHA-256 hex digests")


def _validate_self_test(value: Any) -> None:
    if not isinstance(value, dict) or value.get("required") is not True or not isinstance(value.get("status"), str) or value["status"] not in {"PASS", "FAIL", "not_run"}:
        raise ValueError("self_test report schema is invalid")
    checks = value.get("checks")
    if checks is None and value["status"] == "not_run":
        return
    if not isinstance(checks, list):
        raise ValueError("self_test checks must be an array")
    for check in checks:
        if not isinstance(check, dict) or not isinstance(check.get("name"), str) or not isinstance(check.get("passed"), bool) or not isinstance(check.get("observed_kinds"), list) or not all(isinstance(item, str) for item in check["observed_kinds"]):
            raise ValueError("self_test check record is malformed")
    if value["status"] == "PASS" and (not checks or not all(check["passed"] for check in checks)):
        raise ValueError("passing self_test must contain at least one all-passing check")
    if value["status"] == "FAIL" and (not checks or all(check["passed"] for check in checks)):
        raise ValueError("failed self_test must include a failed check")


def read_candidate_attestation(path: Path, candidate: list[str], source_before: str) -> dict[str, Any]:
    try:
        attestation = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        raise ValueError(f"invalid candidate build attestation: {exc}") from exc
    if not isinstance(attestation, dict) or attestation.get("schema_version") != 1:
        raise ValueError("candidate build attestation must be an object with schema_version=1")
    if attestation.get("verified") is not True or attestation.get("build_exit_code") != 0:
        raise ValueError("candidate build attestation is not a successful verified build")
    binary_path = Path(attestation.get("binary_path", ""))
    if not binary_path.is_absolute() or not binary_path.is_file():
        raise ValueError("attestation binary_path must be an existing absolute file")
    if Path(candidate[0]).resolve() != binary_path.resolve():
        raise ValueError("candidate executable does not match attested binary")
    binary_hash = sha256_file(binary_path)
    if attestation.get("binary_sha256") != binary_hash:
        raise ValueError("candidate binary hash differs from build attestation")
    if attestation.get("source_fingerprint") != source_before:
        raise ValueError("candidate build attestation has a stale source fingerprint")
    cargo_lock = RUST_ROOT / "Cargo.lock"
    lock_hash = sha256_file(cargo_lock) if cargo_lock.exists() else None
    if attestation.get("cargo_lock_sha256") != lock_hash:
        raise ValueError("candidate Cargo.lock hash differs from build attestation")
    return attestation


def verify_candidate_after_batch(
    candidate: list[str],
    expected_binary_sha256: str,
    attestation_path: Path | None,
    expected_attestation_sha256: str | None,
    source_fingerprint_before: str,
    *,
    oracle_self_check: bool,
) -> None:
    if sha256_file(Path(candidate[0])) != expected_binary_sha256:
        raise ValueError("candidate executable hash changed during comparison")
    if not oracle_self_check:
        if attestation_path is None:
            raise ValueError("candidate build attestation disappeared during comparison")
        if expected_attestation_sha256 is None or sha256_file(attestation_path) != expected_attestation_sha256:
            raise ValueError("candidate build attestation hash changed during comparison")
        read_candidate_attestation(attestation_path, candidate, source_fingerprint_before)


def run_live(
    capability: str | None,
    report_path: Path,
    candidate_json: str | None,
    reference_json: str | None,
    attestation_path: Path | None,
    oracle_self_check: bool = False,
    required_capabilities: set[str] | None = None,
    case_manifest_path: Path = MANIFEST,
) -> int:
    started_at = dt.datetime.now(dt.timezone.utc).isoformat()
    run_id = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ") + "-" + uuid.uuid4().hex[:8]
    run_dir = RUST_ROOT / "artifacts" / run_id
    run_dir.mkdir(parents=True, exist_ok=False)
    statuses = ["PASS", "MISMATCH", "NOT_IMPLEMENTED", "ORACLE_ERROR", "CANDIDATE_ERROR", "TIMEOUT", "BLOCKED"]
    report: dict[str, Any] = {
        "schema_version": REPORT_SCHEMA_VERSION,
        "run_id": run_id,
        "started_at_utc": started_at,
        "finished_at_utc": None,
        "status": "BLOCKED",
        "run_level_blocker": False,
        "scope": "harness_health_only" if oracle_self_check else "compatibility_gate",
        "source_fingerprint": source_fingerprint(),
        "baseline": None,
        "candidate": None,
        "environment": {"python": sys.version, "platform": platform.platform()},
        "capability_ids": [],
        "case_ids": [],
        "counts": {status: 0 for status in statuses},
        "cases": [],
        "raw_artifacts_dir": str(run_dir),
        "self_test": {"required": True, "status": "not_run"},
        "errors": [],
    }
    try:
        integrity = oracle_integrity()
        report["baseline"] = integrity
        self_test = run_self_tests()
        report["self_test"] = {"required": True, "status": "PASS" if self_test else "FAIL", "checks": self_test}
        if not self_test or not all(item["passed"] for item in self_test):
            raise RuntimeError("comparator self-tests failed")
        lock = read_lock()
        reference = parse_argv_json(reference_json, default_reference_argv())
        pinned_reference_argv = default_reference_argv()
        require_locked_reference_argv(reference, pinned_reference_argv)
        if sha256_file(Path(reference[0])) != lock.get("reference_executable_sha256"):
            raise RuntimeError("reference executable override is not the pinned oracle executable")
        candidate = parse_argv_json(candidate_json, [])
        source_before = report["source_fingerprint"]
        if oracle_self_check:
            if candidate != reference:
                raise RuntimeError("oracle self-check must compare the pinned reference executable against itself")
            report["candidate"] = {
                "kind": "pinned_reference_self_comparison",
                "argv": candidate,
                "executable_sha256": sha256_file(Path(candidate[0])),
                "source_fingerprint": None,
                "parity_evidence": False,
            }
        elif attestation_path is None:
            raise RuntimeError("live parity requires --candidate-attestation from a locked rebuild")
        else:
            attestation = read_candidate_attestation(attestation_path, candidate, source_before)
            report["candidate"] = {
                "kind": "rust_build_attested",
                "argv": candidate,
                "executable_sha256": sha256_file(Path(candidate[0])),
                "source_fingerprint": attestation["source_fingerprint"],
                "cargo_lock_sha256": sha256_file(RUST_ROOT / "Cargo.lock") if (RUST_ROOT / "Cargo.lock").exists() else None,
                "build_attestation": str(attestation_path.resolve()),
                "build_attestation_sha256": sha256_file(attestation_path),
                "build_commands": attestation.get("build_commands"),
                "toolchain": attestation.get("toolchain"),
            }
        manifest = load_manifest(case_manifest_path)
        cases = [
            case for case in manifest["cases"]
            if (capability is None or case["capability_id"] == capability)
            and (required_capabilities is None or case["capability_id"] in required_capabilities)
        ]
        if not cases:
            raise RuntimeError(f"zero cases selected for capability {capability!r}")
        report["capability_ids"] = sorted({case["capability_id"] for case in cases})
        report["case_ids"] = [case["id"] for case in cases]
        for case in cases:
            report["cases"].append(json_report_value(run_case(case, reference, candidate, run_dir, attestation_path)))
        for case_result in report["cases"]:
            report["counts"][case_result["status"]] += 1
        report["status"] = "PASS" if all(case["status"] == "PASS" for case in report["cases"]) else "MISMATCH"
        try:
            verify_candidate_after_batch(
                candidate,
                report["candidate"]["executable_sha256"],
                attestation_path,
                report["candidate"].get("build_attestation_sha256"),
                source_before,
                oracle_self_check=oracle_self_check,
            )
        except (ValueError, OSError, RuntimeError) as exc:
            report["status"] = "BLOCKED"
            report["run_level_blocker"] = True
            report["errors"].append({"kind": "CandidateChangedDuringRun", "message": str(exc)})
        if source_fingerprint() != source_before:
            report["status"] = "BLOCKED"
            report["run_level_blocker"] = True
            report["errors"].append({"kind": "SourceChangedDuringRun", "message": "Rust source tree changed during comparison"})
        if oracle_integrity() != integrity:
            report["status"] = "BLOCKED"
            report["run_level_blocker"] = True
            report["errors"].append({"kind": "OracleChangedDuringRun", "message": "Oracle integrity metadata changed during comparison"})
    except (RuntimeError, ValueError, OSError, subprocess.SubprocessError, KeyError, TypeError) as exc:
        report["errors"].append({"kind": type(exc).__name__, "message": str(exc)})
        report["run_level_blocker"] = True
        report["status"] = "BLOCKED"
    report["finished_at_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report["counts"] = {status: sum(case.get("status") == status for case in report["cases"]) for status in statuses}
    if report["run_level_blocker"]:
        report["counts"]["BLOCKED"] += 1
    validate_report(report)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"report": str(report_path), "run_id": run_id, "status": report["status"], "counts": report["counts"], "errors": report["errors"]}, indent=2, sort_keys=True))
    return 0 if report["status"] == "PASS" else 1


def run_self_tests() -> list[dict[str, Any]]:
    """Mutation-test the comparator using valid expected CLI wire output."""
    base_json = {
        "result": {
            "file:///work/a b/é.txt": {
                "diagnostics": [
                    {"check_path": "sample.first", "message": "one", "pos": [1, 2], "span": [1, 4], "replacements": None},
                    {"check_path": "sample.second", "message": "two", "pos": [2, 1], "span": [5, 8], "replacements": "x"},
                ]
            }
        }
    }
    expected = {"exit_code": 1, "stdout": json.dumps(base_json).encode(), "stderr": b""}
    def changed(change: Any) -> dict[str, Any]:
        value = json.loads(json.dumps(base_json))
        change(value)
        return {"exit_code": 1, "stdout": json.dumps(value).encode(), "stderr": b""}
    mutations: list[tuple[str, dict[str, Any], str]] = [
        ("drop_diagnostic", changed(lambda v: v["result"]["file:///work/a b/é.txt"]["diagnostics"].pop()), "array_length"),
        ("add_diagnostic", changed(lambda v: v["result"]["file:///work/a b/é.txt"]["diagnostics"].append({"check_path": "sample.third", "message": "three", "pos": [3, 1], "span": [9, 10], "replacements": None})), "array_length"),
        ("reorder_diagnostics", changed(lambda v: v["result"]["file:///work/a b/é.txt"]["diagnostics"].reverse()), "field"),
        ("duplicate_diagnostic", changed(lambda v: v["result"]["file:///work/a b/é.txt"]["diagnostics"].append(v["result"]["file:///work/a b/é.txt"]["diagnostics"][0])), "array_length"),
        ("message", changed(lambda v: v["result"]["file:///work/a b/é.txt"]["diagnostics"][0].update(message="changed")), "value"),
        ("rule_id", changed(lambda v: v["result"]["file:///work/a b/é.txt"]["diagnostics"][0].update(check_path="changed")), "value"),
        ("coordinate", changed(lambda v: v["result"]["file:///work/a b/é.txt"]["diagnostics"][0].update(pos=[4, 1])), "value"),
        ("replacement", changed(lambda v: v["result"]["file:///work/a b/é.txt"]["diagnostics"][1].update(replacements="different")), "value"),
        ("null_vs_empty_replacement", changed(lambda v: v["result"]["file:///work/a b/é.txt"]["diagnostics"][0].update(replacements="")), "type"),
        ("field_presence", changed(lambda v: v["result"]["file:///work/a b/é.txt"]["diagnostics"][0].pop("replacements")), "JSON_SCHEMA"),
        ("file_key", changed(lambda v: (v["result"].update({"other": v["result"].pop("file:///work/a b/é.txt")}))), "additional_field"),
    ]
    checks = []
    for name, actual, expected_kind in mutations:
        diffs = compare_outputs(expected, actual, "json")
        passed = bool(diffs) and any(diff.get("kind") == expected_kind or expected_kind == "field" for diff in diffs)
        checks.append({"name": name, "passed": passed, "observed_kinds": sorted({diff.get("kind", "?") for diff in diffs})})
    for name, actual, kind in [
        ("stderr", {**expected, "stderr": b"unexpected"}, "stderr_mismatch"),
        ("exit_status", {**expected, "exit_code": 0}, "exit_code_mismatch"),
        ("invalid_json", {**expected, "stdout": b"{"}, "INVALID_JSON"),
        ("unexpected_extra_output", {**expected, "stdout": expected["stdout"] + b"extra"}, "INVALID_JSON"),
    ]:
        diffs = compare_outputs(expected, actual, "json")
        checks.append({"name": name, "passed": any(diff.get("kind") == kind for diff in diffs), "observed_kinds": sorted({diff.get("kind", "?") for diff in diffs})})
    for name, stdout in [("wire_schema", expected["stdout"]), ("no_diagnostics", b'{"result":{"<stdin>":{"diagnostics":[]}}}')]:
        parsed, error = decode_json_output(stdout)
        valid = error is None and validate_output_schema(parsed) is None
        checks.append({"name": name, "passed": valid, "observed_kinds": [] if valid else [error or "schema"]})
    error_output = b'{"result":{"source":{"error":{"code":-31998,"message":"missing"}}}}'
    parsed_error, error = decode_json_output(error_output)
    valid_error_wire = error is None and validate_output_schema(parsed_error) is None
    checks.append({"name": "upstream_error_schema", "passed": valid_error_wire, "observed_kinds": [] if valid_error_wire else [error or "schema"]})
    duplicate_key = b'{"result":{},"result":{}}'
    _, error = decode_json_output(duplicate_key)
    checks.append({"name": "duplicate_json_key", "passed": error is not None, "observed_kinds": ["INVALID_JSON"] if error else []})
    with tempfile.TemporaryDirectory(prefix="harness space-é-") as temp_name:
        root = Path(temp_name)
        odd_path = root / "draft é title.txt"
        odd_path.write_text("content", encoding="utf-8")
        path_probe = run_command(
            [sys.executable, "-c", "import sys; print(sys.argv[1])", str(odd_path)],
            cwd=root,
            env=os.environ.copy(),
            stdin=b"",
            timeout=2,
            role="candidate",
        )
        checks.append({"name": "spaces_non_ascii_argv_path", "passed": path_probe["status"] == "OK" and path_probe["stdout"] == str(odd_path).encode() + b"\n", "observed_kinds": [path_probe["status"]]})
        absent = run_command([str(root / "not-there")], cwd=root, env={}, stdin=b"", timeout=1, role="candidate")
        checks.append({"name": "spawn_failure", "passed": absent["status"] == "CANDIDATE_ERROR", "observed_kinds": [absent["status"]]})
        crash = run_command([sys.executable, "-c", "raise SystemExit(9)"], cwd=root, env=os.environ.copy(), stdin=b"", timeout=2, role="candidate")
        checks.append({"name": "crash_exit", "passed": crash["status"] == "CANDIDATE_ERROR" and crash["exit_code"] == 9, "observed_kinds": [crash["status"], str(crash["exit_code"]) ]})
        timeout = run_command([sys.executable, "-c", "import time; time.sleep(2)"], cwd=root, env=os.environ.copy(), stdin=b"", timeout=0.05, role="candidate")
        checks.append({"name": "timeout", "passed": timeout["status"] == "TIMEOUT", "observed_kinds": [timeout["status"]]})
    return checks


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--oracle-self-check", action="store_true")
    parser.add_argument("--capability")
    parser.add_argument("--live", action="store_true")
    parser.add_argument("--report", type=Path)
    parser.add_argument("--candidate-argv-json")
    parser.add_argument("--candidate-attestation", type=Path)
    parser.add_argument("--reference-argv-json")
    parser.add_argument("--case-manifest", type=Path, default=MANIFEST)
    args = parser.parse_args()
    if args.self_test:
        checks = run_self_tests()
        passed = bool(checks) and all(item["passed"] for item in checks)
        print(json.dumps({"status": "PASS" if passed else "FAIL", "checks": checks}, indent=2))
        return 0 if passed else 1
    if args.oracle_self_check:
        if not args.live or args.report is None:
            parser.error("oracle self-check requires --live and --report")
        reference = default_reference_argv()
        return run_live("harness.oracle_smoke", args.report, json.dumps(reference), args.reference_argv_json, None, oracle_self_check=True)
    if not args.live or args.report is None or args.candidate_argv_json is None:
        parser.error("live comparison requires --live --report and --candidate-argv-json")
    try:
        return run_live(args.capability, args.report, args.candidate_argv_json, args.reference_argv_json, args.candidate_attestation, case_manifest_path=args.case_manifest)
    except (ValueError, OSError) as exc:
        print(f"BLOCKED: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
