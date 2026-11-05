from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import compare


class ComparatorTests(unittest.TestCase):
    def test_files_must_be_consumed_or_explicitly_unused_and_format_must_match(self) -> None:
        original = compare.load_manifest()["cases"][1]
        fixture_path = original["files"][0]["path"]

        unreferenced = json.loads(json.dumps(original))
        unreferenced["argv"] = [arg for arg in unreferenced["argv"] if "{file:" not in arg]
        with self.assertRaisesRegex(ValueError, "neither consumed by argv"):
            compare.validate_case(unreferenced)

        classified_unused = json.loads(json.dumps(unreferenced))
        classified_unused["unused_files"] = [fixture_path]
        compare.validate_case(classified_unused)

        multiply_classified = json.loads(json.dumps(original))
        multiply_classified["unused_files"] = [fixture_path]
        with self.assertRaisesRegex(ValueError, "both consumed and declared unused"):
            compare.validate_case(multiply_classified)

        mismatched_format = json.loads(json.dumps(original))
        mismatched_format["output_format"] = "full"
        with self.assertRaisesRegex(ValueError, "must match one --output-format"):
            compare.validate_case(mismatched_format)

        mismatched_policy = json.loads(json.dumps(original))
        mismatched_policy["comparison"]["stdout"] = "bytes"
        with self.assertRaisesRegex(ValueError, "stdout comparison policy"):
            compare.validate_case(mismatched_policy)

        missing_format_arg = json.loads(json.dumps(original))
        missing_format_arg["argv"] = [arg for arg in missing_format_arg["argv"] if arg not in {"--output-format", "json"}]
        with self.assertRaisesRegex(ValueError, "one --output-format"):
            compare.validate_case(missing_format_arg)

    def test_reference_argv_rejects_unlocked_prefix_arguments(self) -> None:
        locked = ["/pinned/reference/proselint"]
        compare.require_locked_reference_argv(locked, locked)
        with self.assertRaisesRegex(ValueError, "exactly match"):
            compare.require_locked_reference_argv(locked + ["--config", "/other/config.json"], locked)

    def test_candidate_binary_and_attestation_are_rechecked_after_batch(self) -> None:
        with tempfile.TemporaryDirectory(prefix="attestation test-é ") as directory:
            binary = Path(directory) / "candidate"
            binary.write_bytes(b"initial binary")
            binary.chmod(0o755)
            lock_path = compare.RUST_ROOT / "Cargo.lock"
            lock_hash = compare.sha256_file(lock_path) if lock_path.exists() else None
            attestation = {
                "schema_version": 1,
                "verified": True,
                "build_exit_code": 0,
                "binary_path": str(binary),
                "binary_sha256": compare.sha256_file(binary),
                "source_fingerprint": compare.source_fingerprint(),
                "cargo_lock_sha256": lock_hash,
            }
            attestation_path = Path(directory) / "attestation.json"
            attestation_path.write_text(json.dumps(attestation), encoding="utf-8")
            argv = [str(binary)]
            source = compare.source_fingerprint()
            original_hash = compare.sha256_file(binary)
            attestation_hash = compare.sha256_file(attestation_path)
            compare.verify_candidate_after_batch(argv, original_hash, attestation_path, attestation_hash, source, oracle_self_check=False)
            attestation_path.write_text(attestation_path.read_text(encoding="utf-8") + " ", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "attestation hash changed"):
                compare.verify_candidate_after_batch(argv, original_hash, attestation_path, attestation_hash, source, oracle_self_check=False)
            attestation_path.write_text(json.dumps(attestation), encoding="utf-8")
            binary.write_bytes(b"changed binary")
            with self.assertRaisesRegex(ValueError, "hash changed"):
                compare.verify_candidate_after_batch(argv, original_hash, attestation_path, attestation_hash, source, oracle_self_check=False)

    def test_fake_cli_subprocess_mutations_fail_end_to_end(self) -> None:
        base = {
            "result": {
                "<stdin>": {
                    "diagnostics": [
                        {"check_path": "sample.first", "message": "first", "pos": [1, 1], "span": [0, 5], "replacements": None},
                        {"check_path": "sample.second", "message": "second", "pos": [1, 7], "span": [6, 12], "replacements": "target"},
                    ]
                }
            }
        }

        def altered(label: str) -> tuple[bytes, bytes, int]:
            value = json.loads(json.dumps(base))
            stderr = b""
            exit_code = 1
            diagnostics = value["result"]["<stdin>"]["diagnostics"]
            if label == "drop":
                diagnostics.pop()
            elif label == "add":
                diagnostics.append({"check_path": "sample.third", "message": "third", "pos": [2, 1], "span": [14, 18], "replacements": None})
            elif label == "reorder":
                diagnostics.reverse()
            elif label == "duplicate":
                diagnostics.append(diagnostics[0])
            elif label == "message":
                diagnostics[0]["message"] = "altered"
            elif label == "rule":
                diagnostics[0]["check_path"] = "altered"
            elif label == "coordinate":
                diagnostics[0]["pos"] = [9, 9]
            elif label == "span":
                diagnostics[0]["span"] = [1, 8]
            elif label == "replacement":
                diagnostics[1]["replacements"] = "altered"
            elif label == "field_presence":
                diagnostics[0].pop("replacements")
            elif label == "file_key":
                value["result"]["other"] = value["result"].pop("<stdin>")
            elif label == "stderr":
                stderr = b"unexpected\n"
            elif label == "exit_status":
                exit_code = 0
            elif label == "invalid_json":
                return b"{", stderr, exit_code
            elif label == "extra_output":
                return json.dumps(value).encode() + b"unexpected", stderr, exit_code
            return json.dumps(value).encode(), stderr, exit_code

        def executable(stdout: bytes, stderr: bytes, exit_code: int) -> list[str]:
            code = (
                "import sys;sys.stdout.buffer.write(bytes.fromhex(sys.argv[1]));"
                "sys.stderr.buffer.write(bytes.fromhex(sys.argv[2]));raise SystemExit(int(sys.argv[3]))"
            )
            return [sys.executable, "-c", code, stdout.hex(), stderr.hex(), str(exit_code)]

        expected_stdout = json.dumps(base).encode()
        reference = executable(expected_stdout, b"", 1)
        base_case = {
            "id": "fake_cli", "capability_id": "harness.test", "provenance": "synthetic comparator discrimination test",
            "input_mode": "stdin", "stdin_base64": "", "files": [], "argv": [], "output_format": "json",
            "expected_oracle_outcome": "findings", "timeout_seconds": 2, "environment": {},
            "workspace_mutates": False,
            "comparison": {"stdout": "json", "stderr": "bytes", "exit_status": "exact"},
        }
        with tempfile.TemporaryDirectory(prefix="fake cli harness-é ") as temp_name:
            root = Path(temp_name)
            labels = ("drop", "add", "reorder", "duplicate", "message", "rule", "coordinate", "span", "replacement", "field_presence", "file_key", "stderr", "exit_status", "invalid_json", "extra_output")
            for label in labels:
                with self.subTest(mutation=label):
                    stdout, stderr, exit_code = altered(label)
                    case = {**base_case, "id": f"case_{label}"}
                    result = compare.run_case(case, reference, executable(stdout, stderr, exit_code), root / label, None)
                    self.assertEqual(result["status"], "MISMATCH", result)

    def test_required_mutation_and_process_discriminators(self) -> None:
        checks = compare.run_self_tests()
        self.assertTrue(checks)
        self.assertTrue(all(item["passed"] for item in checks), checks)
        names = {item["name"] for item in checks}
        self.assertTrue(
            {
                "drop_diagnostic", "add_diagnostic", "reorder_diagnostics", "duplicate_diagnostic",
                "message", "rule_id", "coordinate", "replacement", "field_presence", "file_key",
                "stderr", "exit_status", "invalid_json", "unexpected_extra_output", "spawn_failure",
                "crash_exit", "timeout", "spaces_non_ascii_argv_path", "no_diagnostics",
            }.issubset(names)
        )

    def test_real_manifest_schema_and_unique_ids(self) -> None:
        manifest = compare.load_manifest()
        self.assertEqual(manifest["schema_version"], 1)
        expected_case_ids = {
            "empty_stdin_json", "unicode_space_file_path",
            "cli_json_empty_stdin", "cli_json_no_hit_stdin", "cli_json_empty_unicode_file",
            "annotations_positive_todo", "annotations_negative_mixed_case",
            "annotations_case_sensitive_multiple_order", "annotations_boundary_negative",
            "annotations_quote_suppression", "annotations_unicode_boundaries_positions",
            "annotations_selected_config_interaction",
            "coordinates_multiline_lf_final", "coordinates_multiline_lf_no_final",
            "coordinates_crlf_no_final", "coordinates_unicode_combining_prefix",
            "coordinates_negative_word_boundaries", "coordinates_file_crlf_no_final",
            "cliches_hell_positive", "cliches_hell_case_variants_order",
            "cliches_hell_boundary_negative", "cliches_hell_quote_suppression",
            "cliches_hell_unanchored_substring", "cliches_hell_selected_config_interaction",
            "apologizing_positive_context", "apologizing_case_variants_order",
            "apologizing_boundary_negative", "apologizing_quote_suppression",
            "apologizing_raw_substring", "apologizing_selected_config_interaction",
            "whence_positive_context", "whence_case_variants_order",
            "whence_boundary_negative", "whence_quote_suppression",
            "whence_raw_substring", "whence_selected_config_interaction",
            "kudos_positive_context", "kudos_case_variants_order",
            "kudos_boundary_negative", "kudos_quote_suppression",
            "kudos_raw_substring", "kudos_selected_config_interaction",
            "mental_telepathy_positive_context", "mental_telepathy_case_variants_order",
            "mental_telepathy_boundary_negative", "mental_telepathy_quote_suppression",
            "mental_telepathy_raw_substring", "mental_telepathy_selected_config_interaction",
            "coin_positive_context", "coin_case_variants_order",
            "coin_boundary_negative", "coin_quote_suppression",
            "coin_raw_substring", "coin_selected_config_interaction",
            "collusion_positive_context", "collusion_case_variants_order",
            "collusion_boundary_negative", "collusion_quote_suppression",
            "collusion_raw_substring", "collusion_selected_config_interaction",
            "back_formations_positive_context", "back_formations_case_variants_order",
            "back_formations_word_boundary_negative", "back_formations_quote_suppression",
            "back_formations_punctuation_boundaries", "back_formations_selected_config_interaction",
            "institution_name_positive_context", "institution_name_case_variants_order",
            "institution_name_word_boundary_negative", "institution_name_quote_suppression",
            "institution_name_punctuation_boundaries", "institution_name_selected_config_interaction",
            "professions_shoe_repair_positive", "professions_geometrist_positive",
            "professions_case_variants_order", "professions_boundary_negative",
            "professions_quote_suppression", "professions_selected_config_interaction",
        }
        self.assertEqual({item["id"] for item in manifest["cases"]}, expected_case_ids)
        self.assertEqual(len(manifest["cases"]), len(expected_case_ids))
        self.assertEqual(
            {capability: sum(case["capability_id"] == capability for case in manifest["cases"])
             for capability in {case["capability_id"] for case in manifest["cases"]}},
            {"harness.oracle_smoke": 2, "cli.json_no_findings": 3, "rule.annotations.misc": 7, "cli.coordinates": 6, "rule.cliches.hell": 6, "rule.misc.apologizing": 6, "rule.misc.whence": 6, "rule.misc.false_plurals.kudos": 6, "rule.psychology.mental_telepathy": 6, "rule.misc.illogic.coin": 6, "rule.misc.illogic.collusion": 6, "rule.misc.back_formations": 6, "rule.misc.institution_name": 6, "rule.misc.professions": 6},
        )

    def test_manifest_rejects_duplicate_id_and_bad_base64(self) -> None:
        valid = compare.load_manifest()
        duplicate = {"schema_version": 1, "cases": [valid["cases"][0], valid["cases"][0]]}
        with tempfile.TemporaryDirectory(prefix="manifest space-é ") as directory:
            path = Path(directory) / "cases.json"
            path.write_text(json.dumps(duplicate), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "duplicate case id"):
                compare.load_manifest(path)
            invalid = json.loads(json.dumps(valid))
            invalid["cases"][0]["stdin_base64"] = "!"
            path.write_text(json.dumps(invalid), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "stdin_base64"):
                compare.load_manifest(path)
            invalid = json.loads(json.dumps(valid))
            invalid["schema_version"] = True
            path.write_text(json.dumps(invalid), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "schema_version"):
                compare.load_manifest(path)

    def test_wire_schema_and_duplicate_json_keys(self) -> None:
        good = b'{"result":{"<stdin>":{"diagnostics":[]}}}'
        parsed, error = compare.decode_json_output(good)
        self.assertIsNone(error)
        self.assertIsNone(compare.validate_output_schema(parsed))
        duplicate, error = compare.decode_json_output(b'{"result":{},"result":{}}')
        self.assertIsNone(duplicate)
        self.assertIn("duplicate", error or "")
        invalid = {"result": {"file": {"diagnostics": [{"check": "not upstream schema"}]}}}
        self.assertIn("exactly", compare.validate_output_schema(invalid) or "")

    def test_process_statuses_distinguish_lint_errors_from_crashes(self) -> None:
        with tempfile.TemporaryDirectory(prefix="process space-é ") as directory:
            cwd = Path(directory)
            base_env = dict(os.environ)
            normal = compare.run_command(
                [sys.executable, "-c", "raise SystemExit(1)"], cwd=cwd, env=base_env,
                stdin=b"", timeout=2, role="candidate",
            )
            crash = compare.run_command(
                [sys.executable, "-c", "raise SystemExit(9)"], cwd=cwd, env=base_env,
                stdin=b"", timeout=2, role="candidate",
            )
            self.assertEqual(normal["status"], "OK")
            self.assertEqual(normal["exit_code"], 1)
            self.assertEqual(crash["status"], "CANDIDATE_ERROR")

    def test_source_key_normalization_is_scoped_to_one_workspace_prefix(self) -> None:
        value = {"result": {"file:///tmp/one/doc.txt": {"diagnostics": []}}}
        normalized = compare.normalize_source_keys(value, "file:///tmp/one")
        self.assertIn("file:///tmp/one/doc.txt", value["result"])
        self.assertIn("<workspace>/doc.txt", normalized["result"])
        unchanged = compare.normalize_source_keys(value, "file:///tmp/two")
        self.assertEqual(unchanged, value)

    def test_report_rejects_count_and_case_id_mismatch(self) -> None:
        report = self._valid_report()
        compare.validate_report(report)
        report["case_ids"] = ["missing"]
        with self.assertRaisesRegex(ValueError, "case IDs"):
            compare.validate_report(report)

    def test_report_validates_nested_execution_fields_and_case_status_counts(self) -> None:
        report = self._valid_report()
        report["cases"][0]["executions"][0]["duration_seconds"] = float("nan")
        with self.assertRaisesRegex(ValueError, "duration_seconds"):
            compare.validate_report(report)

        report = self._valid_report()
        report["cases"][0]["status"] = []
        with self.assertRaisesRegex(ValueError, "case report status"):
            compare.validate_report(report)

        report = self._valid_report()
        report["status"] = "MISMATCH"
        with self.assertRaisesRegex(ValueError, "top-level status"):
            compare.validate_report(report)

    def test_report_reconciles_case_status_with_execution_records(self) -> None:
        report = self._valid_report()
        report["cases"][0]["executions"][1]["status"] = "CANDIDATE_ERROR"
        report["cases"][0]["executions"][1]["exit_code"] = None
        report["cases"][0]["executions"][1]["error"] = "candidate spawn failed"
        with self.assertRaisesRegex(ValueError, "PASS case must have two successful executions"):
            compare.validate_report(report)

        report = self._valid_report()
        report["cases"][0]["executions"][0]["exit_code"] = 9
        with self.assertRaisesRegex(ValueError, "accepted CLI exit status"):
            compare.validate_report(report)

        report = self._valid_report()
        report["cases"][0]["status"] = "MISMATCH"
        report["status"] = "MISMATCH"
        report["counts"]["PASS"] = 0
        report["counts"]["MISMATCH"] = 1
        with self.assertRaisesRegex(ValueError, "MISMATCH case must have two successful executions"):
            compare.validate_report(report)

    def test_report_counts_run_level_blocker_separately(self) -> None:
        report = self._valid_report()
        report.update({
            "status": "BLOCKED",
            "run_level_blocker": True,
            "capability_ids": [],
            "case_ids": [],
            "cases": [],
            "counts": {status: 0 for status in {"PASS", "MISMATCH", "NOT_IMPLEMENTED", "ORACLE_ERROR", "CANDIDATE_ERROR", "TIMEOUT", "BLOCKED"}},
            "errors": [{"kind": "BLOCKED", "message": "candidate not built"}],
        })
        report["counts"]["BLOCKED"] = 1
        compare.validate_report(report)
        report["counts"]["BLOCKED"] = 0
        with self.assertRaisesRegex(ValueError, "counts disagree"):
            compare.validate_report(report)

    @staticmethod
    def _valid_report() -> dict[str, object]:
        statuses = {"PASS", "MISMATCH", "NOT_IMPLEMENTED", "ORACLE_ERROR", "CANDIDATE_ERROR", "TIMEOUT", "BLOCKED"}
        digest = "0" * 64
        commands = [["/pinned/reference"], ["/candidate"]]
        executions = []
        for role, command in zip(("reference", "candidate"), commands, strict=True):
            executions.append({
                "argv": command, "cwd": "/tmp/case", "role": role, "status": "OK",
                "duration_seconds": 0.01, "exit_code": 0, "stdout_path": f"{role}.stdout.bin",
                "stderr_path": f"{role}.stderr.bin", "stdout_sha256": digest, "stderr_sha256": digest,
            })
        case = {
            "case_id": "case_one", "capability_id": "sample.capability", "status": "PASS",
            "expected_oracle_outcome": "clean", "commands": commands, "stdin_sha256": digest,
            "workspace_layout": "input-layout", "workspace_mutates": False,
            "normalization": {"source_key_workspace_uri": False},
            "effective_environment": {
                "reference": {"config_scenario": "isolated_default"},
                "candidate": {"config_scenario": "isolated_default"},
            },
            "executions": executions, "diffs": [],
            "replay": {
                "command": ["python3", "rust/harness/compare.py"],
                "saved_case_manifest": "replay-manifest.json", "saved_stdin": "stdin.bin",
                "saved_layout": "input-layout", "candidate_argv": ["/candidate"],
                "requires_same_candidate_binary_and_build_attestation": True,
            },
        }
        return {
            "schema_version": 1, "run_id": "run-one", "started_at_utc": "2026-01-01T00:00:00+00:00",
            "finished_at_utc": "2026-01-01T00:00:01+00:00", "status": "PASS", "scope": "compatibility_gate",
            "run_level_blocker": False, "source_fingerprint": digest,
            "baseline": {
                "integrity": "verified", "revision": "rev", "source_tree_sha256": digest,
                "uv_lock_sha256": digest, "executable": "/oracle", "executable_sha256": digest,
                "python": "/python", "python_version": "3.12.3", "installed_package_tree_sha256": digest,
                "dependencies": {"proselint": "0.16.0"}, "module_provenance": {"proselint": "site-packages/proselint/__init__.py"},
            },
            "candidate": {
                "kind": "rust_build_attested", "argv": ["/candidate"], "executable_sha256": digest,
                "source_fingerprint": digest, "cargo_lock_sha256": digest,
                "build_attestation": "attestation.json", "build_attestation_sha256": digest, "toolchain": "1.99.1",
                "build_commands": [{"command": ["cargo", "build", "--locked"], "exit_code": 0, "stdout_path": "build.stdout", "stderr_path": "build.stderr"}],
            },
            "environment": {"python": "3.12.3", "platform": "test"},
            "capability_ids": ["sample.capability"], "case_ids": ["case_one"],
            "counts": {**{status: 0 for status in statuses}, "PASS": 1}, "cases": [case],
            "raw_artifacts_dir": "artifacts/run-one",
            "self_test": {"required": True, "status": "PASS", "checks": [{"name": "mutation", "passed": True, "observed_kinds": []}]},
            "errors": [],
        }


if __name__ == "__main__":
    unittest.main()
