#!/usr/bin/env python3
"""Benchmark gate placeholder; no timings before behavior-equivalent parity."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import platform
import sys
import uuid
from pathlib import Path

from compare import REPORT_SCHEMA_VERSION, load_manifest, source_fingerprint
from gate import accepted_capabilities
from setup_oracle import RUST_ROOT, oracle_integrity


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--live", action="store_true")
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--capability")
    args = parser.parse_args()
    if not args.live:
        parser.error("benchmark requires --live")
    started = dt.datetime.now(dt.timezone.utc).isoformat()
    run_id = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ") + "-benchmark-" + uuid.uuid4().hex[:8]
    errors = []
    baseline = None
    cases = []
    try:
        baseline = oracle_integrity()
        manifest = load_manifest()
        accepted = accepted_capabilities()
        cases = [
            case for case in manifest["cases"]
            if case["capability_id"] in accepted
            and (args.capability is None or case["capability_id"] == args.capability)
            and case.get("benchmark_eligible") is True
        ]
        if not cases:
            errors.append({
                "kind": "BLOCKED",
                "message": "no behavior-equivalent capability is marked benchmark_eligible; complete an independently verified live parity gate first",
            })
        else:
            errors.append({"kind": "BLOCKED", "message": "the benchmark runner is not implemented yet; no timings have been collected"})
    except (OSError, RuntimeError, ValueError) as exc:
        errors.append({"kind": "BLOCKED", "message": str(exc)})
    report = {
        "schema_version": REPORT_SCHEMA_VERSION,
        "run_id": run_id,
        "started_at_utc": started,
        "finished_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "status": "BLOCKED" if errors else "NOT_IMPLEMENTED",
        "run_level_blocker": bool(errors),
        "source_fingerprint": source_fingerprint(),
        "baseline": baseline,
        "candidate": None,
        "environment": {"python": sys.version, "platform": platform.platform()},
        "capability_ids": [case["capability_id"] for case in cases],
        "case_ids": [case["id"] for case in cases],
        "counts": {"PASS": 0, "MISMATCH": 0, "NOT_IMPLEMENTED": 0, "ORACLE_ERROR": 0, "CANDIDATE_ERROR": 0, "TIMEOUT": 0, "BLOCKED": 1 if errors else 0},
        "samples": [],
        "raw_artifacts_dir": str(RUST_ROOT / "artifacts" / run_id),
        "errors": errors,
        "performance_claim": "none",
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"report": str(args.report), "status": report["status"], "errors": errors}, indent=2))
    return 2 if errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
