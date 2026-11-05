#!/usr/bin/env python3
"""Provision and verify the immutable Python reference installation."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
RUST_ROOT = ROOT / "rust"
LOCK_PATH = RUST_ROOT / "oracle.lock.json"
ORACLE_ROOT = RUST_ROOT / ".cache" / "oracle-src"
ORACLE_SOURCE = ORACLE_ROOT / "source"
LOCK_SHA = "149ff01d4be05c243f3fb332c971091b961c77187af5be5261ba35386d0f29a1"
REVISION = "dbed789caae662d06c7c8a5a13dd31f1acd36f5c"
UV_VERSION = "0.13.7"
REFERENCE_PYTHON = "3.12.3"
UV_LINUX_X86_64_SHA = "c5221ff41b7892bfc223543883600125f6a47fe845093e0f5bdff4f359196d83"
EXCLUDED_PARTS = {
    ".git",
    ".venv",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".hypothesis",
    ".pdm-build",
}


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def tree_fingerprint(root: Path) -> str:
    """Hash all source paths and bytes using stable relative POSIX names."""
    digest = hashlib.sha256()
    paths = sorted(
        path
        for path in root.rglob("*")
        if path.is_file() and not (set(path.relative_to(root).parts) & EXCLUDED_PARTS)
    )
    for path in paths:
        rel = path.relative_to(root).as_posix().encode("utf-8")
        data = path.read_bytes()
        digest.update(len(rel).to_bytes(8, "big"))
        digest.update(rel)
        digest.update(len(data).to_bytes(8, "big"))
        digest.update(data)
    return digest.hexdigest()


def package_fingerprint(root: Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        rel = path.relative_to(root).as_posix().encode("utf-8")
        data = path.read_bytes()
        digest.update(len(rel).to_bytes(8, "big"))
        digest.update(rel)
        digest.update(len(data).to_bytes(8, "big"))
        digest.update(data)
    return digest.hexdigest()


def observed_build_backend() -> dict[str, str]:
    cache = RUST_ROOT / ".cache" / "oracle-home" / ".cache" / "uv"
    metadata_files = sorted(cache.glob("archive-v*/**/pdm_backend-*.dist-info/METADATA"))
    if not metadata_files:
        return {
            "name": "pdm-backend",
            "requirement": "pdm-backend>=2.0.0",
            "resolved_version": "unavailable",
            "lockfile_status": "not present in uv.lock",
        }
    metadata = max(metadata_files, key=lambda path: path.stat().st_mtime_ns)
    version = next((line.partition(":")[2].strip() for line in metadata.read_text(encoding="utf-8").splitlines() if line.startswith("Version:")), "unknown")
    return {
        "name": "pdm-backend",
        "requirement": "pdm-backend>=2.0.0",
        "resolved_version": version,
        "metadata_sha256": sha256_file(metadata),
        "lockfile_status": "not present in uv.lock; resolved by uv's isolated build environment",
    }


def runtime_provenance(python_path: Path) -> dict[str, Any]:
    probe = (
        "import importlib.metadata,json,pathlib,proselint,proselint.command_line,proselint.checks,sys; "
        "print(json.dumps({'package':str(pathlib.Path(proselint.__file__).resolve().parent),"
        "'modules':{n:str(pathlib.Path(m.__file__).resolve()) for n,m in sys.modules.items() "
        "if n=='proselint' or n.startswith('proselint.')},"
        "'dependencies':{d.metadata['Name'].lower().replace('_','-'):d.version "
        "for d in importlib.metadata.distributions() if d.metadata.get('Name')}}))"
    )
    result = subprocess.run(
        [str(python_path), "-c", probe],
        cwd=RUST_ROOT,
        env=scrubbed_env(),
        check=True,
        capture_output=True,
        text=True,
    )
    data = json.loads(result.stdout)
    package = Path(data["package"]).resolve()
    module_files = {name: Path(path).resolve() for name, path in data["modules"].items()}
    invalid = [str(path) for path in module_files.values() if package not in path.parents]
    if invalid:
        raise RuntimeError(f"Loaded proselint modules outside installed package: {invalid}")
    return {
        "package_root": package,
        "package_tree_sha256": package_fingerprint(package),
        "module_provenance": {name: str(path) for name, path in sorted(module_files.items())},
        "dependencies": dict(sorted(data["dependencies"].items())),
    }


def read_lock() -> dict[str, Any]:
    if not LOCK_PATH.exists():
        raise RuntimeError(f"Missing oracle lock: {LOCK_PATH}")
    return json.loads(LOCK_PATH.read_text(encoding="utf-8"))


def resolve_locked(path_value: str) -> Path:
    path = Path(path_value)
    return path if path.is_absolute() else RUST_ROOT / path


def oracle_integrity() -> dict[str, Any]:
    lock = read_lock()
    if lock.get("revision") != REVISION:
        raise RuntimeError("oracle.lock.json revision does not match this setup script")
    if sha256_file(ROOT / "uv.lock") != lock["uv_lock_sha256"]:
        raise RuntimeError("uv.lock hash differs from the pinned oracle lock")
    source = resolve_locked(lock["source_dir"])
    if not source.is_dir():
        raise RuntimeError(f"Oracle source snapshot missing: {source}")
    actual = tree_fingerprint(source)
    if actual != lock["source_tree_sha256"]:
        raise RuntimeError(
            f"Oracle source integrity mismatch: expected {lock['source_tree_sha256']}, got {actual}"
        )
    executable = resolve_locked(lock["executable"])
    if not executable.is_file() or not os.access(executable, os.X_OK):
        raise RuntimeError(f"Reference executable missing or not executable: {executable}")
    env_python = resolve_locked(lock["python"])
    if not env_python.is_file():
        raise RuntimeError(f"Reference Python missing: {env_python}")
    interpreter_version = subprocess.run(
        [str(env_python), "--version"], check=True, capture_output=True, text=True
    ).stdout.strip().split()[-1]
    if interpreter_version != lock.get("python_version"):
        raise RuntimeError(f"Reference Python version differs from oracle.lock.json: {interpreter_version}")
    if sha256_file(executable) != lock.get("reference_executable_sha256"):
        raise RuntimeError("Reference executable hash differs from oracle.lock.json")
    runtime = runtime_provenance(env_python)
    if runtime["package_tree_sha256"] != lock.get("installed_package_tree_sha256"):
        raise RuntimeError("Installed reference package tree differs from oracle.lock.json")
    if runtime["dependencies"] != lock.get("dependencies"):
        raise RuntimeError("Reference environment dependency versions differ from oracle.lock.json")
    if observed_build_backend() != lock.get("build_backend"):
        raise RuntimeError("Observed uv package-build backend differs from oracle.lock.json")
    locked_modules = {
        name: Path(path).as_posix() for name, path in lock.get("module_provenance", {}).items()
    }
    actual_modules = {
        name: Path(path).relative_to(RUST_ROOT).as_posix()
        for name, path in runtime["module_provenance"].items()
    }
    if not locked_modules or locked_modules != actual_modules:
        raise RuntimeError("Loaded reference module provenance differs from oracle.lock.json")
    return {
        "revision": REVISION,
        "source_tree_sha256": actual,
        "uv_lock_sha256": sha256_file(ROOT / "uv.lock"),
        "executable": str(executable.resolve()),
        "python": str(env_python.resolve()),
        "python_version": interpreter_version,
        "integrity": "verified",
        "executable_sha256": sha256_file(executable),
        "installed_package_tree_sha256": runtime["package_tree_sha256"],
        "dependencies": runtime["dependencies"],
        "module_provenance": actual_modules,
    }


def download_uv() -> Path:
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system != "linux" or machine not in {"x86_64", "amd64"}:
        candidate = shutil.which("uv")
        if not candidate:
            raise RuntimeError(
                "uv is not installed; local bootstrap is pinned only for Linux x86_64. "
                "Install uv 0.13.7 or extend the verified platform mapping."
            )
        return Path(candidate).resolve()

    tools_dir = RUST_ROOT / ".cache" / "tools"
    uv_path = tools_dir / "uv-x86_64-unknown-linux-gnu" / "uv"
    if uv_path.exists():
        return uv_path.resolve()
    tools_dir.mkdir(parents=True, exist_ok=True)
    archive = tools_dir / "uv-bootstrap.tar.gz"
    url = f"https://releases.astral.sh/github/uv/releases/download/{UV_VERSION}/uv-x86_64-unknown-linux-gnu.tar.gz"
    try:
        request = urllib.request.Request(url, headers={"User-Agent": "proselint-rs-oracle-setup"})
        with urllib.request.urlopen(request, timeout=60) as response, archive.open("wb") as output:
            shutil.copyfileobj(response, output)
    except Exception:
        # Some environments reject urllib's default request but permit curl.
        subprocess.run(["curl", "-fL", url, "-o", str(archive)], check=True)
    actual_hash = sha256_file(archive)
    if actual_hash != UV_LINUX_X86_64_SHA:
        archive.unlink(missing_ok=True)
        raise RuntimeError(f"uv archive checksum mismatch: {actual_hash}")
    with tarfile.open(archive, "r:gz") as bundle:
        bundle.extractall(tools_dir, filter="data")
    archive.unlink(missing_ok=True)
    if not uv_path.is_file():
        raise RuntimeError("Pinned uv archive did not contain expected executable")
    uv_path.chmod(0o755)
    return uv_path.resolve()


def build_snapshot() -> tuple[str, str]:
    source = RUST_ROOT / ".cache" / "oracle-src" / "source"
    source.parent.mkdir(parents=True, exist_ok=True)
    archive_path = RUST_ROOT / ".cache" / "oracle-src" / "source.tar"
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["git", "archive", "--format=tar", REVISION],
        cwd=ROOT,
        check=True,
        stdout=archive_path.open("wb"),
        stderr=subprocess.PIPE,
    )
    archive_hash = sha256_file(archive_path)
    if source.exists():
        shutil.rmtree(source)
    source.mkdir(parents=True)
    with tarfile.open(archive_path, "r:") as bundle:
        bundle.extractall(source, filter="data")
    archive_path.unlink()
    return archive_hash, tree_fingerprint(source)


def scrubbed_env(extra: dict[str, str] | None = None) -> dict[str, str]:
    env = {
        "PATH": os.environ.get("PATH", ""),
        "HOME": str(RUST_ROOT / ".cache" / "oracle-home"),
        "XDG_CONFIG_HOME": str(RUST_ROOT / ".cache" / "oracle-xdg"),
        "PYTHONNOUSERSITE": "1",
        "PYTHONUTF8": "1",
        "LC_ALL": "C.UTF-8",
        "LANG": "C.UTF-8",
    }
    env.pop("PYTHONPATH", None)
    if extra:
        env.update(extra)
    return env


def write_lock(
    archive_sha: str,
    source_sha: str,
    uv_path: Path,
    uv_version: str,
    setup_command: list[str],
    reference_python_version: str,
) -> dict[str, Any]:
    venv = ORACLE_SOURCE / ".venv"
    exe_dir = "Scripts" if os.name == "nt" else "bin"
    executable_name = "proselint.exe" if os.name == "nt" else "proselint"
    python_name = "python.exe" if os.name == "nt" else "python"
    lock = {
        "schema_version": 1,
        "status": "provisioned",
        "repository": "https://github.com/amperser/proselint",
        "revision": REVISION,
        "package_version": "0.16.0",
        "source_archive_sha256": archive_sha,
        "source_tree_sha256": source_sha,
        "source_hash_exclusions": sorted(EXCLUDED_PARTS),
        "source_dir": ".cache/oracle-src/source",
        "python_version": reference_python_version,
        "python_implementation": platform.python_implementation(),
        "platform": platform.platform(),
        "uv_version": uv_version,
        "uv_lock_sha256": sha256_file(ROOT / "uv.lock"),
        "uv_executable_sha256": sha256_file(uv_path),
        "build_backend": observed_build_backend(),
        "dependencies": {},
        "installed_package_tree_sha256": "",
        "reference_executable_sha256": "",
        "environment_setup_command": ["uv", "sync", "--locked", "--python", REFERENCE_PYTHON, "--group", "test", "--no-default-groups", "--no-editable"],
        "python": str((venv / exe_dir / python_name).relative_to(RUST_ROOT)),
        "executable": str((venv / exe_dir / executable_name).relative_to(RUST_ROOT)),
        "reference_argv": [str((venv / exe_dir / executable_name).relative_to(RUST_ROOT))],
        "import_provenance": {
            "verification": "The setup script imports proselint.command_line and proselint.checks with PYTHONPATH unset and PYTHONNOUSERSITE=1, then asserts all loaded proselint modules are installed in the oracle venv.",
            "verified": False,
        },
        "upstream_tests": {"command": [], "exit_code": None, "status": "not_run"},
        "oracle_test_environment": {
            "PYTHONPATH": "unset",
            "PYTHONNOUSERSITE": "1",
            "HOME": ".cache/oracle-home",
            "XDG_CONFIG_HOME": ".cache/oracle-xdg",
            "locale": "C.UTF-8",
        },
    }
    LOCK_PATH.write_text(json.dumps(lock, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return lock


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    try:
        if args.verify_only:
            print(json.dumps(oracle_integrity(), indent=2, sort_keys=True))
            return 0
        if sha256_file(ROOT / "uv.lock") != LOCK_SHA:
            raise RuntimeError("Repository uv.lock differs from the reviewed setup hash")
        uv_path = download_uv()
        version_text = subprocess.run(
            [str(uv_path), "--version"], check=True, capture_output=True, text=True
        ).stdout.strip()
        expected = f"uv {UV_VERSION} "
        if not version_text.startswith(expected):
            raise RuntimeError(f"Expected uv {UV_VERSION}, got {version_text}")
        archive_sha, source_sha = build_snapshot()
        venv = ORACLE_SOURCE / ".venv"
        setup_command = [
            str(uv_path),
            "sync",
            "--locked",
            "--group",
            "test",
            "--no-default-groups",
            "--no-editable",
            "--python",
            REFERENCE_PYTHON,
        ]
        env = scrubbed_env()
        subprocess.run(setup_command, cwd=ORACLE_SOURCE, env=env, check=True)
        venv_python_version = subprocess.run(
            [str(ORACLE_SOURCE / ".venv" / ("Scripts" if os.name == "nt" else "bin") / ("python.exe" if os.name == "nt" else "python")), "--version"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip().split()[-1]
        if venv_python_version != REFERENCE_PYTHON:
            raise RuntimeError(f"Expected reference Python {REFERENCE_PYTHON}, got {venv_python_version}")
        lock = write_lock(archive_sha, source_sha, uv_path, version_text, setup_command, venv_python_version)
        python_path = resolve_locked(lock["python"])
        provenance = subprocess.run(
            [str(python_path), "-c", "import json,proselint,proselint.version,pathlib; print(json.dumps({'file':str(pathlib.Path(proselint.__file__).resolve()),'version':proselint.version.__version__}))"],
            cwd=RUST_ROOT,
            env=scrubbed_env(),
            check=True,
            capture_output=True,
            text=True,
        )
        data = json.loads(provenance.stdout)
        imported = Path(data["file"]).resolve()
        runtime = runtime_provenance(python_path)
        if imported != Path(runtime["module_provenance"]["proselint"]):
            raise RuntimeError("proselint package import path disagrees with provenance probe")
        executable = resolve_locked(lock["executable"])
        lock["dependencies"] = runtime["dependencies"]
        lock["installed_package_tree_sha256"] = runtime["package_tree_sha256"]
        lock["reference_executable_sha256"] = sha256_file(executable)
        lock["module_provenance"] = {
            name: Path(path).relative_to(RUST_ROOT).as_posix()
            for name, path in runtime["module_provenance"].items()
        }
        lock["import_provenance"].update(
            {"verified": True, "file": str(imported.relative_to(RUST_ROOT)), "reported_version": data["version"]}
        )
        test_command = [str(python_path), "-m", "pytest", "-q"]
        test_run = subprocess.run(
            test_command,
            cwd=ORACLE_SOURCE,
            env=scrubbed_env(),
            capture_output=True,
            text=True,
        )
        (RUST_ROOT / ".cache" / "oracle-src" / "upstream-pytest.stdout.txt").write_text(test_run.stdout, encoding="utf-8")
        (RUST_ROOT / ".cache" / "oracle-src" / "upstream-pytest.stderr.txt").write_text(test_run.stderr, encoding="utf-8")
        lock["upstream_tests"] = {
            "command": [lock["python"], "-m", "pytest", "-q"],
            "exit_code": test_run.returncode,
            "status": "passed" if test_run.returncode == 0 else "baseline_failure",
            "stdout_path": ".cache/oracle-src/upstream-pytest.stdout.txt",
            "stderr_path": ".cache/oracle-src/upstream-pytest.stderr.txt",
        }
        lock["import_provenance"].update({"verified": True, "file": str(imported.relative_to(RUST_ROOT)), "reported_version": data["version"]})
        LOCK_PATH.write_text(json.dumps(lock, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(json.dumps({"integrity": oracle_integrity(), "upstream_tests": lock["upstream_tests"]}, indent=2, sort_keys=True))
        return test_run.returncode
    except (OSError, RuntimeError, subprocess.CalledProcessError, tarfile.TarError) as exc:
        print(f"oracle setup failed: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
