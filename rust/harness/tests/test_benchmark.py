import importlib.util
import json
import os
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
HARNESS = ROOT / "harness"
if str(HARNESS) not in sys.path:
    sys.path.insert(0, str(HARNESS))

spec = importlib.util.spec_from_file_location("benchmark", HARNESS / "benchmark.py")
benchmark = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(benchmark)


class BenchmarkTests(unittest.TestCase):
    def test_release_build_targets_cli_and_benchmark_example_only(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            cargo = root / "bin" / "cargo"
            cargo.parent.mkdir()
            cargo.touch()
            rust_root = root / "rust"
            rust_root.mkdir()
            (rust_root / "artifacts").mkdir()
            (rust_root / "Cargo.toml").touch()
            (rust_root / "Cargo.lock").touch()
            binary = rust_root / "target" / "release" / "proselint"
            helper = rust_root / "target" / "release" / "examples" / "benchmark_suddenly"

            def run(command, **kwargs):
                if command[1] == "build":
                    binary.parent.mkdir(parents=True)
                    helper.parent.mkdir(parents=True)
                    binary.write_bytes(b"cli")
                    helper.write_bytes(b"helper")
                    return benchmark.subprocess.CompletedProcess(command, 0, b"", b"")
                return benchmark.subprocess.CompletedProcess(command, 0, "tool version", "")

            with (
                mock.patch.object(benchmark, "RUST_ROOT", rust_root),
                mock.patch.object(benchmark, "RUST_HELPER", helper),
                mock.patch.object(benchmark, "locked_cargo", return_value=(cargo, "stable")),
                mock.patch.object(benchmark, "supervised_fingerprint", return_value="f" * 64),
                mock.patch.object(benchmark, "supervised_run", side_effect=run),
            ):
                _, _, attestation, _ = benchmark.build_release_candidate(root / "run", time.monotonic() + 30)
                self.assertEqual(attestation["binary_sha256"], benchmark.sha256_file(binary))
                self.assertEqual(attestation["helper_binary_sha256"], benchmark.sha256_file(helper))

        command = attestation["build_commands"][0]["command"]
        self.assertIn("--bin", command)
        self.assertIn("proselint", command)
        self.assertIn("--example", command)
        self.assertIn("benchmark_suddenly", command)
        self.assertNotIn("--all-targets", command)
        self.assertEqual(attestation["binary_path"], str(binary.resolve()))
        self.assertEqual(attestation["helper_binary_path"], str(helper.resolve()))

    def test_alternates_pair_order(self):
        self.assertEqual(benchmark.pair_order(0), ["reference", "candidate"])
        self.assertEqual(benchmark.pair_order(1), ["candidate", "reference"])
        self.assertEqual(benchmark.pair_order(2), ["reference", "candidate"])

    def test_summary_preserves_count_spread_and_median(self):
        result = benchmark.summarize([10, 20, 30, 40, 50])
        self.assertEqual(result["sample_count"], 5)
        self.assertEqual(result["median_ns"], 30)
        self.assertEqual(result["minimum_ns"], 10)
        self.assertEqual(result["maximum_ns"], 50)
        self.assertEqual(result["iqr_ns"], 20)

    def test_summary_rejects_missing_or_nonpositive_samples(self):
        for values in ([], [1, 0], [True, 2]):
            with self.subTest(values=values), self.assertRaises(ValueError):
                benchmark.summarize(values)

    def test_diagnostic_digest_is_stable_and_field_order_independent(self):
        left = [{"message": "Suddenly", "pos": [1, 1]}]
        right = [{"pos": [1, 1], "message": "Suddenly"}]
        self.assertEqual(
            benchmark.canonical_diagnostics_digest(left),
            benchmark.canonical_diagnostics_digest(right),
        )

    def test_real_workload_manifest_and_fixture_hashes_are_valid(self):
        manifest = benchmark.load_workloads()
        self.assertEqual(manifest["profile_id"], "misc_suddenly")
        self.assertEqual(len(manifest["workloads"]), 3)
        self.assertEqual(
            {workload["kind"] for workload in manifest["workloads"]},
            {
                "licensed_upstream_prose",
                "deterministic_ordinary_generated",
                "deterministic_dense_adversarial_generated",
            },
        )

    def test_combined_profile_requires_liveable_positive_examples_for_each_rule(self):
        manifest = benchmark.load_workloads(benchmark.COMBINED_WORKLOADS)
        self.assertEqual(manifest["capability_id"], "cli.combined_checks")
        self.assertEqual(manifest["selected_check_paths"], ["misc.suddenly", "psychology.p_equals_zero"])
        self.assertTrue(all(set(w["positive_examples"]) == set(manifest["selected_check_paths"]) for w in manifest["workloads"]))
        for workload in manifest["workloads"]:
            text = benchmark.workload_bytes(workload, manifest).decode("utf-8")
            self.assertTrue(all(example in text for example in workload["positive_examples"].values()))

        with tempfile.TemporaryDirectory() as temp:
            copy = Path(temp) / "combined_workloads.json"
            invalid = dict(manifest)
            invalid["selected_check_paths"] = ["misc.suddenly"]
            copy.write_text(json.dumps(invalid), encoding="utf-8")
            for workload in manifest["workloads"]:
                (Path(temp) / workload["path"]).write_bytes(benchmark.workload_bytes(workload, manifest))
            (Path(temp) / manifest["config_file"]).write_bytes((benchmark.WORKLOADS.parent / manifest["config_file"]).read_bytes())
            with self.assertRaisesRegex(ValueError, "capability or selected checks"):
                benchmark.load_workloads(copy)

    def test_workload_hash_mismatch_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "workloads.json"
            path.write_text("{}", encoding="utf-8")
            with self.assertRaises(ValueError):
                benchmark.load_workloads(path)

    def test_parity_only_report_cannot_contain_timing_claim(self):
        with tempfile.TemporaryDirectory() as temp:
            report = benchmark.blocked_report(
                "test", fingerprint="f" * 64, run_id="test", started="now", raw_dir=Path(temp)
            )
        report.update(
            status="PARITY_PASS",
            run_level_blocker=False,
            parity={"status": "PASS", "report": "parity.json"},
            performance_claim="none",
        )
        benchmark.validate_benchmark_report(report)
        report["performance_claim"] = "fast"
        with self.assertRaises(ValueError):
            benchmark.validate_benchmark_report(report)

    def test_blocked_report_cannot_claim_performance(self):
        with tempfile.TemporaryDirectory() as temp:
            report = benchmark.blocked_report(
                "parity failed", fingerprint="f" * 64, run_id="test", started="now", raw_dir=Path(temp)
            )
        benchmark.validate_benchmark_report(report)
        report["performance_claim"] = "fast"
        with self.assertRaises(ValueError):
            benchmark.validate_benchmark_report(report)

    def test_expired_overall_deadline_stops_benchmark_work(self):
        with self.assertRaises(benchmark.subprocess.TimeoutExpired):
            benchmark.remaining_seconds(benchmark.time.monotonic() - 1)

    @unittest.skipUnless(os.name == "posix", "requires POSIX process-group termination")
    def test_timeout_kills_child_inheriting_stdout_and_stderr(self):
        with tempfile.TemporaryDirectory() as temp:
            marker = Path(temp) / "child-survived"
            child = "import sys,time; time.sleep(.4); open(sys.argv[1], 'w').write('alive')"
            parent = "import subprocess,sys,time; subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[2]]); time.sleep(10)"
            command = [sys.executable, "-c", parent, child, str(marker)]
            with self.assertRaises(benchmark.subprocess.TimeoutExpired):
                benchmark.supervised_run(command, timeout=0.1)
            time.sleep(0.6)
            self.assertFalse(marker.exists())

    @unittest.skipUnless(os.name == "posix", "requires POSIX process-group termination")
    def test_timeout_kills_sigterm_ignoring_child_with_redirected_pipes(self):
        with tempfile.TemporaryDirectory() as temp:
            marker = Path(temp) / "child-survived"
            child = "import signal,sys,time; signal.signal(signal.SIGTERM, signal.SIG_IGN); time.sleep(1.5); open(sys.argv[1], 'w').write('alive')"
            parent = "import subprocess,sys,time; subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[2]],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL); time.sleep(10)"
            command = [sys.executable, "-c", parent, child, str(marker)]
            with self.assertRaises(benchmark.subprocess.TimeoutExpired):
                benchmark.supervised_run(command, timeout=0.5)
            time.sleep(0.7)
            self.assertFalse(marker.exists())

    def test_parity_preflight_reserves_half_budget_for_overhead(self):
        per_case = benchmark.parity_case_timeout(120, 3)
        self.assertEqual(per_case, 10)
        self.assertLessEqual(per_case * 2 * 3, 120 / 2)

    def test_deadline_expiring_during_timing_keeps_parity_report(self):
        attestation = {
            "binary_path": "/tmp/target/release/proselint",
            "source_fingerprint": "f" * 64,
            "cargo_lock_sha256": "a" * 64,
            "build_commands": [{"command": ["cargo", "build", "--release", "--locked"], "exit_code": 0}],
        }
        with tempfile.TemporaryDirectory() as temp:
            report_path = Path(temp) / "blocked.json"
            workloads = benchmark.load_workloads()

            def write_parity_report(payload, _deadline):
                path = payload["report_path"]
                Path(path).write_text(
                    json.dumps({"status": "PASS", "case_ids": ["case"], "counts": {"PASS": 1}}),
                    encoding="utf-8",
                )
                return 0

            def expire_during_timing(*args):
                benchmark.remaining_seconds(args[-1])

            with (
                mock.patch.object(benchmark, "RUST_ROOT", Path(temp)),
                mock.patch.object(benchmark, "accepted_capabilities", return_value={"rule.misc.suddenly"}),
                mock.patch.object(benchmark, "load_workloads", return_value=workloads),
                mock.patch.object(benchmark.compare, "default_reference_argv", return_value=["/pinned/python"]),
                mock.patch.object(benchmark, "supervised_fingerprint", return_value="f" * 64),
                mock.patch.object(benchmark, "supervised_oracle_integrity", return_value={"integrity": "verified"}),
                mock.patch.object(
                    benchmark,
                    "build_release_candidate",
                    return_value=(
                        ["/tmp/target/release/proselint"],
                        Path(temp) / "attestation.json",
                        attestation,
                        {},
                    ),
                ),
                mock.patch.object(benchmark.compare, "read_candidate_attestation", return_value=attestation),
                mock.patch.object(benchmark, "supervised_live_compare", side_effect=write_parity_report),
                mock.patch.object(benchmark, "sha256_file", return_value="a" * 64),
                mock.patch.object(benchmark, "timed_cli_workload", side_effect=expire_during_timing) as timed,
                mock.patch.object(benchmark.time, "monotonic", side_effect=[0.0, 0.0, 1.0, 11.0]),
            ):
                exit_code = benchmark.run(
                    report_path,
                    release_attestation=None,
                    sample_pairs=25,
                    warmup_pairs=1,
                    in_memory_repetitions=1,
                    in_memory_pairs=25,
                    parity_only=False,
                    overall_timeout_seconds=10,
                )
            report = json.loads(report_path.read_text(encoding="utf-8"))
        self.assertEqual(timed.call_count, 1)
        self.assertEqual(exit_code, 2)
        self.assertEqual(report["status"], "BLOCKED")
        self.assertEqual(report["parity"]["status"], "PASS")
        self.assertEqual(report["performance_claim"], "none")
        self.assertEqual(report["fresh_process"], [])
        self.assertEqual(report["in_memory"], [])

    def test_parity_manifest_uses_deadline_bounded_case_timeout(self):
        with tempfile.TemporaryDirectory() as temp:
            path = benchmark.make_parity_manifest(
                benchmark.load_workloads(), Path(temp) / "cases.json", timeout_seconds=0.25
            )
            manifest = benchmark.compare.load_manifest(path)
        self.assertEqual(len(manifest["cases"]), 3)
        self.assertTrue(all(case["timeout_seconds"] == 0.25 for case in manifest["cases"]))

    def test_live_preflight_timeout_writes_blocked_report(self):
        attestation = {
            "binary_path": "/tmp/target/release/proselint",
            "source_fingerprint": "f" * 64,
            "cargo_lock_sha256": "a" * 64,
            "build_commands": [{"command": ["cargo", "build", "--release", "--locked"], "exit_code": 0}],
        }
        with tempfile.TemporaryDirectory() as temp:
            report_path = Path(temp) / "blocked.json"
            with (
                mock.patch.object(benchmark, "RUST_ROOT", Path(temp)),
                mock.patch.object(benchmark, "accepted_capabilities", return_value={"rule.misc.suddenly"}),
                mock.patch.object(benchmark, "load_workloads", return_value=benchmark.load_workloads()),
                mock.patch.object(benchmark.compare, "default_reference_argv", return_value=["/pinned/python"]),
                mock.patch.object(benchmark, "supervised_fingerprint", return_value="f" * 64),
                mock.patch.object(benchmark, "supervised_oracle_integrity", return_value={"integrity": "verified"}),
                mock.patch.object(
                    benchmark,
                    "build_release_candidate",
                    return_value=(
                        ["/tmp/target/release/proselint"],
                        Path(temp) / "attestation.json",
                        attestation,
                        {},
                    ),
                ),
                mock.patch.object(benchmark.compare, "read_candidate_attestation", return_value=attestation),
                mock.patch.object(benchmark, "sha256_file", return_value="a" * 64),
                mock.patch.object(
                    benchmark, "supervised_live_compare",
                    side_effect=benchmark.subprocess.TimeoutExpired("live preflight", 1),
                ),
                mock.patch.object(benchmark, "timed_cli_workload") as timed,
            ):
                exit_code = benchmark.run(
                    report_path,
                    release_attestation=None,
                    sample_pairs=25,
                    warmup_pairs=1,
                    in_memory_repetitions=1,
                    in_memory_pairs=25,
                    parity_only=False,
                    overall_timeout_seconds=30,
                )
            report = json.loads(report_path.read_text(encoding="utf-8"))
        self.assertEqual(exit_code, 2)
        self.assertEqual(report["status"], "BLOCKED")
        self.assertEqual(report["parity"]["status"], "not_run")
        self.assertEqual(report["performance_claim"], "none")
        self.assertEqual(report["fresh_process"], [])
        timed.assert_not_called()


if __name__ == "__main__":
    unittest.main()
