#!/usr/bin/env python3
"""Measure repeated pinned Python misc.suddenly file checks in one process."""

from __future__ import annotations

import hashlib
import json
import sys
import time
from pathlib import Path

from proselint.checks import __register__
from proselint.config import load_from
from proselint.registry import CheckRegistry
from proselint.tools import LintFile


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit("usage: python_suddenly_scan.py <document> <config> <repetitions>")
    document_path = Path(sys.argv[1])
    config_path = Path(sys.argv[2])
    repetitions = int(sys.argv[3])
    if repetitions < 1:
        raise SystemExit("repetitions must be positive")
    document = document_path.read_text(encoding="utf-8")
    config = load_from(config_path)
    registry = CheckRegistry()
    registry.register_many(__register__)
    enabled = registry.get_all_enabled(config["checks"])
    if [check.path for check in enabled] != ["misc.suddenly"]:
        raise SystemExit(f"expected exactly misc.suddenly; enabled={[check.path for check in enabled]!r}")

    first_diagnostics: list[dict[str, object]] | None = None
    total_diagnostics = 0
    started = time.perf_counter_ns()
    for _ in range(repetitions):
        result = LintFile("<stdin>", document).lint(config)
        current = [item.into_dict() for item in result]
        total_diagnostics += len(current)
        if first_diagnostics is None:
            first_diagnostics = current
    elapsed_ns = time.perf_counter_ns() - started
    assert first_diagnostics is not None
    canonical = json.dumps(
        first_diagnostics, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    print(json.dumps({
        "elapsed_ns": elapsed_ns,
        "repetitions": repetitions,
        "diagnostics_per_iteration": len(first_diagnostics),
        "total_diagnostics": total_diagnostics,
        "diagnostics_sha256": hashlib.sha256(canonical).hexdigest(),
        "diagnostics": first_diagnostics,
    }, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
