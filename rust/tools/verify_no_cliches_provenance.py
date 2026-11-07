#!/usr/bin/env python3
"""Recheck the pinned write-good list and its identified MIT data source."""

from __future__ import annotations

import argparse
import ast
import hashlib
import io
import json
import re
import tarfile
from difflib import SequenceMatcher
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT / "rust/provenance/no_cliches_write_good.json"
PINNED_SOURCE = ROOT / "rust/.cache/oracle-src/source/proselint/checks/cliches/write-good"
RUST_DATA = ROOT / "rust/data/third_party/no_cliches_write_good.txt"
LICENSE_NOTICE = ROOT / "rust/licenses/no-cliches-MIT.txt"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def package_phrases(source: str) -> list[str]:
    start = source.index("let cliches = [") + len("let cliches = [")
    end = source.index("];", start)
    literals = re.findall(r"'(?:\\.|[^'\\])*'", source[start:end])
    return [ast.literal_eval(literal) for literal in literals]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("npm_tarball", type=Path, help="path to no-cliches-0.3.0.tgz")
    args = parser.parse_args()
    packet = json.loads(PACKET.read_text(encoding="utf-8"))
    source = PINNED_SOURCE.read_bytes()
    assert sha256(source) == packet["baseline"]["resource_sha256"]
    assert RUST_DATA.read_bytes() == source
    npm = packet["identified_data_distribution"]
    tarball = args.npm_tarball.read_bytes()
    assert sha256(tarball) == npm["npm_tarball_sha256"]
    with tarfile.open(fileobj=io.BytesIO(tarball), mode="r:gz") as archive:
        metadata = json.load(archive.extractfile("package/package.json"))
        package_source = archive.extractfile("package/cliches.js").read().decode("utf-8")
        license = archive.extractfile("package/LICENSE").read()
    assert metadata["version"] == npm["version"] == "0.3.0"
    assert metadata["license"] == npm["npm_metadata_license"] == "MIT"
    assert sha256(license) == npm["license_notice_sha256"]
    assert license == LICENSE_NOTICE.read_bytes()
    pinned = [line for line in source.decode("utf-8").splitlines() if line]
    published = package_phrases(package_source)
    differences = []
    for op, first_start, first_end, second_start, second_end in SequenceMatcher(
        a=pinned, b=published, autojunk=False
    ).get_opcodes():
        if op == "insert":
            differences.append(
                {"kind": "package_addition", "index": first_start, "phrase": published[second_start]}
            )
        elif op == "replace":
            differences.append(
                {
                    "kind": "baseline_trailing_space_difference",
                    "baseline_index": first_start,
                    "baseline_phrase": pinned[first_start],
                    "package_index": second_start,
                    "package_phrase": published[second_start],
                }
            )
        elif op != "equal":
            raise AssertionError(f"unexpected list difference: {op}")
    assert differences == packet["comparison"]["only_differences"]
    print(
        f"PASS: {len(pinned)} pinned entries; {len(published)} MIT package entries; "
        f"only {len(differences)} documented differences"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
