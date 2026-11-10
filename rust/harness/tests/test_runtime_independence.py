"""Prove the release CLI runs with only its ELF runtime and explicit config."""

from __future__ import annotations

import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from compare import source_fingerprint
from setup_oracle import RUST_ROOT, oracle_integrity, read_lock


ROOT = RUST_ROOT.parent
ARTIFACTS = RUST_ROOT / "artifacts"
CONFIG = b'{"checks":{"annotations":{"misc":true}},"max_errors":1000}\n'


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def runtime_paths(binary: Path) -> tuple[Path, list[Path]]:
    """Resolve the ELF interpreter and shared objects without trusting PATH."""
    readelf = shutil.which("readelf")
    ldd = shutil.which("ldd")
    if not readelf or not ldd:
        raise RuntimeError("Linux runtime independence needs readelf and ldd")
    program = subprocess.run([readelf, "-l", str(binary)], capture_output=True, text=True, check=True)
    match = re.search(r"Requesting program interpreter: ([^]]+)", program.stdout)
    if not match:
        raise RuntimeError("Rust binary has no dynamically linked ELF interpreter")
    loader = Path(match.group(1))
    libraries = subprocess.run([ldd, str(binary)], capture_output=True, text=True, check=True)
    paths = set()
    for line in libraries.stdout.splitlines():
        match = re.search(r"(?:=>\s+)?(/[\w./+-]+)\s+\(", line)
        if match:
            paths.add(Path(match.group(1)))
    if not paths:
        raise RuntimeError("ldd resolved no shared libraries")
    return loader, sorted(paths)


def copy_runtime_file(source: Path, root: Path) -> Path:
    target = root / source.as_posix().lstrip("/")
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source.resolve(), target)
    return target


@unittest.skipUnless(platform.system() == "Linux", "runtime independence isolation requires Linux user and mount namespaces")
class RuntimeIndependenceTest(unittest.TestCase):
    def test_release_cli_runs_without_repository_or_python_in_chroot(self):
        unshare = shutil.which("unshare")
        chroot = shutil.which("chroot")
        self.assertIsNotNone(unshare, "Linux runtime independence blocked: unshare executable is unavailable")
        self.assertIsNotNone(chroot, "Linux runtime independence blocked: chroot executable is unavailable")
        fingerprint = source_fingerprint()
        attestation_value = os.environ.get("PROSELINT_RS_ATTESTATION")
        if attestation_value:
            attestation_path = Path(attestation_value).resolve()
        else:
            status = json.loads((RUST_ROOT / "status.json").read_text(encoding="utf-8"))
            attestation_path = (ROOT / status["last_strict_gate"]["build_attestation"]).resolve()
        attestation = json.loads(attestation_path.read_text(encoding="utf-8"))
        binary = Path(attestation["binary_path"]).resolve()
        self.assertEqual(attestation.get("profile"), "release", "attested binary must use release profile")
        self.assertEqual(attestation.get("source_fingerprint"), fingerprint, "candidate attestation is stale for current harness/source")
        self.assertTrue(binary.is_file(), f"attested Rust binary is missing: {binary}")
        self.assertEqual(sha256(binary), attestation.get("binary_sha256"), "attested Rust binary hash mismatch")
        oracle_evidence = oracle_integrity()
        lock = read_lock()
        oracle = RUST_ROOT / lock["executable"]
        self.assertTrue(oracle.is_file(), f"pinned Python oracle is missing: {oracle}; run setup_oracle.py")

        binary_hash = sha256(binary)
        loader, libraries = runtime_paths(binary)

        nonce = f"isolation-{time.time_ns()}-café"
        input_bytes = f"A {nonce} TODO marker.\n".encode("utf-8")
        with tempfile.TemporaryDirectory(prefix="proselint-isolation-") as temp_name:
            temp = Path(temp_name)
            root = temp / "root"
            root.mkdir()
            (root / "home").mkdir()
            (root / "tmp").mkdir()
            (root / "empty-bin").mkdir()
            (root / "bin").mkdir()
            (root / "config.json").write_bytes(CONFIG)
            # The executable is exposed at a stable path inside the minimal root.
            rust_target = root / "bin" / "proselint-rs"
            shutil.copy2(binary, rust_target)
            copied_runtime = {"loader": copy_runtime_file(loader, root)}
            copied_runtime.update({str(path): copy_runtime_file(path, root) for path in libraries})

            rust_command = [
                unshare, "--user", "--map-root-user", "--mount", "--fork", "--",
                chroot, str(root), "/bin/proselint-rs", "check", "--config", "/config.json",
                "--output-format", "compact",
            ]
            clean_env = {"PATH": "/empty-bin"}
            try:
                rust = subprocess.run(
                    rust_command, cwd=root, env=clean_env, input=input_bytes,
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30, close_fds=True,
                )
                rust_execution = {"exit_status": rust.returncode, "stdout": rust.stdout, "stderr": rust.stderr}
            except (OSError, subprocess.SubprocessError) as exc:
                rust_execution = {"error": f"{type(exc).__name__}: {exc}"}

            oracle_command = [str(oracle), "check", "--config", str(root / "config.json"), "--output-format", "compact"]
            oracle_env = {
                "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
                "HOME": str(temp / "oracle-home"),
                "XDG_CONFIG_HOME": str(temp / "oracle-xdg"),
                "PYTHONNOUSERSITE": "1",
                "PYTHONPATH": "",
                "LC_ALL": "C.UTF-8",
                "LANG": "C.UTF-8",
            }
            Path(oracle_env["HOME"]).mkdir()
            Path(oracle_env["XDG_CONFIG_HOME"]).mkdir()
            oracle_result = subprocess.run(
                oracle_command, cwd=temp, env=oracle_env, input=input_bytes,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30, close_fds=True,
            )

            expected = {"exit_status": oracle_result.returncode, "stdout": oracle_result.stdout, "stderr": oracle_result.stderr}
            rust_ok = "error" not in rust_execution and rust_execution == expected
            inventory = [
                "/", "/bin", "/bin/proselint-rs", "/config.json", "/empty-bin", "/home", "/tmp",
                *sorted({"/" + str(path.relative_to(root)).replace(os.sep, "/") for path in copied_runtime.values()}),
            ]
            evidence = {
                "schema_version": 1,
                "test": "runtime_independence_linux_user_mount_namespace_chroot",
                "created_utc_epoch_seconds": time.time(),
                "status": "passed" if rust_ok else "failed",
                "input_sha256": hashlib.sha256(input_bytes).hexdigest(),
                "input_utf8": input_bytes.decode("utf-8"),
                "config_sha256": hashlib.sha256(CONFIG).hexdigest(),
                "rust_binary": {"path": str(binary), "sha256": binary_hash},
                "source_fingerprint": fingerprint,
                "attestation": {"path": str(attestation_path), "sha256": sha256(attestation_path)},
                "attestation_build_commands": attestation["build_commands"],
                "oracle_integrity": oracle_evidence,
                "dynamic_runtime": {
                    "loader_host_path": str(loader),
                    "libraries": [
                        {"host_path": str(path), "sha256": sha256(path)}
                        for path in [loader, *libraries]
                    ],
                },
                "commands": {
                    "rust_isolated": rust_command,
                    "oracle": oracle_command,
                },
                "minimal_root_inventory": inventory,
                "rust_environment": clean_env,
                "rust_child_fds": "stdin/stdout/stderr only (close_fds=True)",
                "oracle": {"exit_status": oracle_result.returncode, "stdout_hex": oracle_result.stdout.hex(), "stderr_hex": oracle_result.stderr.hex()},
                "rust": {
                    key: value.hex() if isinstance(value, bytes) else value
                    for key, value in rust_execution.items()
                },
                "exact_match": rust_ok,
            }
            ARTIFACTS.mkdir(parents=True, exist_ok=True)
            evidence_path = ARTIFACTS / f"runtime-independence-{time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())}-{os.getpid()}.json"
            evidence_path.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            self.assertNotIn("error", rust_execution, f"isolated Rust execution failed; evidence: {evidence_path}")
            self.assertEqual(rust_execution, expected, f"isolated Rust CLI differs from pinned oracle; evidence: {evidence_path}")


if __name__ == "__main__":
    unittest.main()
