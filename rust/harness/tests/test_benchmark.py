import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
HARNESS = ROOT / "harness"
if str(HARNESS) not in sys.path:
    sys.path.insert(0, str(HARNESS))

spec = importlib.util.spec_from_file_location("benchmark", HARNESS / "benchmark.py")
benchmark = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(benchmark)


class BenchmarkTests(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()
