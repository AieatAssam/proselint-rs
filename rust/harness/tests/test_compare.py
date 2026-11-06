from __future__ import annotations

import json
import base64
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import compare


class ComparatorTests(unittest.TestCase):
    def test_source_fingerprint_ignores_docs_and_status_but_tracks_inputs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "harness" / "cases").mkdir(parents=True)
            (root / "docs").mkdir()
            source = root / "src" / "main.rs"
            manifest = root / "harness" / "cases" / "manifest.json"
            inventory = root / "inventory.json"
            status = root / "status.json"
            docs = root / "docs" / "PROGRESS.md"
            migration = root / "MIGRATION.md"
            history = root / "status-history.jsonl"
            source.write_text("fn main() {}", encoding="utf-8")
            manifest.write_text("{}", encoding="utf-8")
            inventory.write_text("{}", encoding="utf-8")
            status.write_text("{}", encoding="utf-8")
            docs.write_text("progress", encoding="utf-8")
            migration.write_text("protocol", encoding="utf-8")
            history.write_text("{}\n", encoding="utf-8")
            with patch.object(compare, "RUST_ROOT", root):
                initial = compare.source_fingerprint()
                docs.write_text("updated progress", encoding="utf-8")
                migration.write_text("updated protocol", encoding="utf-8")
                history.write_text("{}\n{}\n", encoding="utf-8")
                inventory.write_text('{"accepted": ["new"]}', encoding="utf-8")
                status.write_text('{"accepted": ["new"]}', encoding="utf-8")
                self.assertEqual(compare.source_fingerprint(), initial)
                source.write_text("fn main() { changed() }", encoding="utf-8")
                self.assertNotEqual(compare.source_fingerprint(), initial)
                changed_source = compare.source_fingerprint()
                manifest.write_text('{"cases": []}', encoding="utf-8")
                self.assertNotEqual(compare.source_fingerprint(), changed_source)
                changed_manifest = compare.source_fingerprint()
                self.assertNotEqual(changed_manifest, initial)

    def test_working_directory_must_be_relative_and_contained(self) -> None:
        case = json.loads(json.dumps(compare.load_manifest()["cases"][0]))
        for path in ("../outside", "/tmp/outside", "nested/../../outside"):
            with self.subTest(path=path):
                case["working_directory"] = path
                with self.assertRaisesRegex(ValueError, "working_directory must stay inside workspace"):
                    compare.validate_case(case)

    def test_working_directory_is_materialized_for_both_processes(self) -> None:
        case = {"id":"cwd_probe", "capability_id":"test", "provenance":"test", "input_mode":"stdin", "stdin_base64":"", "files":[], "argv":[], "output_format":"json", "expected_oracle_outcome":"clean", "timeout_seconds":2, "environment":{}, "workspace_mutates":False, "working_directory":"nested/deeper", "comparison":{"stdout":"json","stderr":"bytes","exit_status":"exact"}}
        probe = [sys.executable, "-c", "import os,json;print(json.dumps({'result':{'<stdin>':{'diagnostics':[]}}}));print(os.getcwd(), file=__import__('sys').stderr)"]
        with tempfile.TemporaryDirectory() as directory:
            result = compare.run_case(case, probe, probe, Path(directory), None)
        self.assertEqual(result["status"], "PASS", result)
        self.assertTrue(all(Path(run["cwd"]).parts[-2:] == ("nested", "deeper") for run in result["executions"]))

    def test_home_fixtures_and_xdg_unset_are_isolated(self) -> None:
        case = {"id":"home_probe", "capability_id":"test", "provenance":"test", "input_mode":"stdin", "stdin_base64":"", "files":[], "home_files":[{"path":".config/proselint/config.json", "content_base64":"e30="}], "argv":["--output-format","json"], "output_format":"json", "expected_oracle_outcome":"clean", "timeout_seconds":2, "environment":{"XDG_CONFIG_HOME":None}, "workspace_mutates":False, "comparison":{"stdout":"json","stderr":"bytes","exit_status":"exact"}}
        compare.validate_case(case)
        self.assertNotIn("XDG_CONFIG_HOME", compare.fresh_env(Path("/isolated"), case["environment"]))
        for item in (dict(case["home_files"][0], path="../outside"), dict(case["home_files"][0], content_base64="not-base64")):
            invalid = json.loads(json.dumps(case))
            invalid["home_files"] = [item]
            with self.assertRaises(ValueError):
                compare.validate_case(invalid)
        invalid = json.loads(json.dumps(case))
        invalid["environment"] = {"PATH": None}
        with self.assertRaisesRegex(ValueError, "only XDG_CONFIG_HOME may be unset"):
            compare.validate_case(invalid)

    def test_home_fixtures_are_materialized_inside_each_role_home(self) -> None:
        case = {"id":"home_probe", "capability_id":"test", "provenance":"test", "input_mode":"stdin", "stdin_base64":"", "files":[], "home_files":[{"path":"fixture.txt", "content_base64":"dGVzdA=="}], "argv":[], "output_format":"json", "expected_oracle_outcome":"clean", "timeout_seconds":2, "environment":{}, "workspace_mutates":False, "comparison":{"stdout":"json","stderr":"bytes","exit_status":"exact"}}
        probe = [sys.executable, "-c", "import os,json,pathlib;print(json.dumps({'result':{'<stdin>':{'diagnostics':[]}}}));print(pathlib.Path(os.environ['HOME'],'fixture.txt').read_text(),file=__import__('sys').stderr)"]
        with tempfile.TemporaryDirectory() as directory:
            result = compare.run_case(case, probe, probe, Path(directory), None)
        self.assertEqual(result["status"], "PASS", result)
        self.assertTrue(all(run["stderr_sha256"] == compare.sha256_bytes(b"test\n") for run in result["executions"]))

    def test_files_must_be_consumed_or_explicitly_unused_and_format_must_match(self) -> None:
        original = next(case for case in compare.load_manifest()["cases"] if case["id"] == "cli_config_consumed_checks_array_nan")
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
        with self.assertRaisesRegex(ValueError, "must match at most one --output-format"):
            compare.validate_case(mismatched_format)

        mismatched_policy = json.loads(json.dumps(original))
        mismatched_policy["comparison"]["stdout"] = "bytes"
        with self.assertRaisesRegex(ValueError, "stdout comparison policy"):
            compare.validate_case(mismatched_policy)

        missing_format_arg = json.loads(json.dumps(original))
        missing_format_arg["argv"] = [arg for arg in missing_format_arg["argv"] if arg not in {"--output-format", "json"}]
        with self.assertRaisesRegex(ValueError, "only full is the default"):
            compare.validate_case(missing_format_arg)

        default_full = json.loads(json.dumps(original))
        default_full["output_format"] = "full"
        default_full["argv"] = [arg for arg in default_full["argv"] if arg not in {"--output-format", "json"}]
        default_full["comparison"]["stdout"] = "bytes"
        compare.validate_case(default_full)

        missing_compact_arg = json.loads(json.dumps(default_full))
        missing_compact_arg["output_format"] = "compact"
        with self.assertRaisesRegex(ValueError, "only full is the default"):
            compare.validate_case(missing_compact_arg)

        duplicate_format_arg = json.loads(json.dumps(original))
        duplicate_format_arg["argv"].extend(["--output-format", "json"])
        with self.assertRaisesRegex(ValueError, "at most one --output-format"):
            compare.validate_case(duplicate_format_arg)

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

    def test_binary_stderr_mismatch_report_keeps_only_hashes_and_artifacts(self) -> None:
        stdout = b'{"result":{"<stdin>":{"diagnostics":[]}}}'
        binary_stderr = b"\xff\x00invalid utf8\xfe"

        def executable(stderr: bytes) -> list[str]:
            code = (
                "import sys;sys.stdout.buffer.write(bytes.fromhex(sys.argv[1]));"
                "sys.stderr.buffer.write(bytes.fromhex(sys.argv[2]));raise SystemExit(0)"
            )
            return [sys.executable, "-c", code, stdout.hex(), stderr.hex()]

        case = {
            "id": "binary_stderr", "capability_id": "harness.test", "provenance": "synthetic stderr serialization regression",
            "input_mode": "stdin", "stdin_base64": "", "files": [], "argv": [], "output_format": "json",
            "expected_oracle_outcome": "clean", "timeout_seconds": 2, "environment": {},
            "workspace_mutates": False,
            "comparison": {"stdout": "json", "stderr": "bytes", "exit_status": "exact"},
        }
        with tempfile.TemporaryDirectory(prefix="binary stderr harness-é ") as temp_name:
            root = Path(temp_name)
            result = compare.run_case(case, executable(b""), executable(binary_stderr), root / "case", None)
            self.assertEqual(result["status"], "MISMATCH", result)
            stderr_diff = next(diff for diff in result["diffs"] if diff["field"] == "stderr")
            self.assertEqual(stderr_diff["expected_sha256"], compare.sha256_bytes(b""))
            self.assertEqual(stderr_diff["actual_sha256"], compare.sha256_bytes(binary_stderr))
            self.assertNotIn("expected", stderr_diff)
            self.assertNotIn("actual", stderr_diff)
            self.assertEqual(Path(result["executions"][1]["stderr_path"]).read_bytes(), binary_stderr)
            self.assertEqual(result["executions"][1]["stderr_sha256"], compare.sha256_bytes(binary_stderr))
            serialized = json.dumps(compare.json_report_value(result))
            self.assertIn('"stderr_mismatch"', serialized)
            self.assertNotIn('"actual":', serialized)

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
            "cli_user_discovery_xdg_path", "cli_user_discovery_home_fallback_unset", "cli_user_discovery_home_fallback_empty",
            "cli_user_discovery_cwd_precedes_user", "cli_user_discovery_ancestor_precedes_user",
            "cli_user_discovery_malformed_first_user_errors", "cli_user_discovery_explicit_overrides_user", "cli_user_discovery_xdg_relative_path",
            "cli_config_cwd_discovers_finding",
            "cli_config_cwd_discovers_empty_checks",
            "cli_config_explicit_precedes_cwd_discovery",
            "cli_config_nested_cwd_discovers_ancestor",
            "cli_config_nearest_ancestor_wins",
            "cli_config_explicit_overrides_ancestors",
            "cli_config_malformed_nearest_ancestor_errors",
            "cli_config_directory_nearest_skips_to_ancestor",
            "cli_output_format_full_clean",
            "cli_output_format_full_default_apologizing_finding",
            "cli_output_format_full_explicit_apologizing_finding",
            "cli_config_consumed_max_errors_nan",
            "cli_config_checks_error_precedes_max_errors_error",
            "cli_config_missing_source_precedes_max_errors_error",
            "cli_config_duplicate_max_errors_last_value_wins",
            "cli_config_consumed_max_errors_overflow",
            "cli_config_consumed_max_errors_surrogate",
            "cli_config_consumed_checks_nan",
            "cli_config_consumed_checks_overflow",
            "cli_config_consumed_checks_array_nan",
            "cli_config_consumed_checks_nested_array_nan",
            "cli_config_consumed_checks_lone_surrogate",
            "cli_config_consumed_checks_duplicate_annotations_last_wins",
            "cli_config_ignored_nested_special_values",
            "cli_config_escaped_checks_key_with_ignored_nan",
            "cli_config_ignored_nan",
            "cli_config_ignored_overflow_float",
            "cli_config_ignored_lone_surrogate",
            "cli_config_directory_json_error",
            "cli_config_directory_parent_json_error",
            "cli_config_directory_full_error",
            "cli_config_directory_compact_error",
            "cli_config_recursion_valid_array_depth150", "cli_config_recursion_valid_object_depth150",
            "cli_config_recursion_valid_alternating_depth150", "cli_config_recursion_malformed_array_depth10000",
            "cli_config_recursion_malformed_object_depth1100", "cli_config_recursion_malformed_alternating_depth1000",
            "cli_config_recursion_stack_overflow_depth66000",
            "cli_config_recursion_array_unclosed_depth100000",
            "cli_config_recursion_valid_array_depth9996", "cli_config_recursion_array_recursion_depth9997",
            "cli_config_recursion_object_recursion_depth9997", "cli_config_recursion_alternating_recursion_depth9997",
            "empty_stdin_json", "unicode_space_file_path",
            "cli_json_empty_stdin", "cli_json_no_hit_stdin", "cli_json_empty_unicode_file", "file_uri_colon_percent_encoding",
            "cli_json_missing_source_per_file_error", "cli_json_missing_config_structured_error", "per_file_checks_glob_disables_global_annotation",
            "cli_config_malformed_json_empty", "cli_config_malformed_json_trailing_comma",
            "cli_config_malformed_json_invalid_escape", "cli_config_malformed_json_unterminated_string",
            "cli_config_malformed_json_extra_data", "cli_config_malformed_json_missing_colon",
            "cli_config_malformed_json_leading_zero", "cli_config_malformed_json_multibyte_multiline",
            "cli_config_malformed_json_crlf_newlines", "cli_config_malformed_json_incomplete_exponent",
            "cli_config_malformed_json_incomplete_unicode_escape",
            "cli_config_malformed_json_array_trailing_comma", "cli_config_malformed_json_literal_control_character",
            "cli_config_malformed_json_utf8_bom", "cli_config_malformed_json_leading_newline",
            "cli_config_malformed_json_nonstandard_constants", "cli_config_malformed_json_surrogate_escape",
            "cli_config_malformed_json_long_integer", "cli_config_malformed_json_deep_arrays",
            "cli_config_malformed_json_partial_literal", "cli_config_malformed_json_invalid_minus",
            "cli_config_malformed_json_root_incomplete_number",
            "cli_config_invalid_utf8_invalid_lead_85", "cli_config_invalid_utf8_invalid_lead_ff",
            "cli_config_invalid_utf8_invalid_continuation", "cli_config_invalid_utf8_truncated_2byte",
            "cli_config_invalid_utf8_truncated_4byte",
            "annotations_positive_todo", "annotations_negative_mixed_case",
            "annotations_case_sensitive_multiple_order", "annotations_boundary_negative",
            "annotations_quote_suppression", "annotations_unicode_boundaries_positions",
            "annotations_selected_config_interaction", "annotations_max_errors_one", "annotations_fastset_activation_raw_copy", "lie_detector_unicode_boundary", "file_uri_colon_percent_encoding",
            "coordinates_multiline_lf_final", "coordinates_multiline_lf_no_final",
            "coordinates_crlf_no_final", "coordinates_unicode_combining_prefix",
            "coordinates_negative_word_boundaries", "coordinates_file_crlf_no_final",
            "cliches_hell_positive", "cliches_hell_case_variants_order",
            "cliches_hell_boundary_negative", "cliches_hell_quote_suppression",
            "cliches_hell_unanchored_substring", "cliches_hell_selected_config_interaction",
            "cliches_hell_kelvin_casefold_window",
            "apologizing_positive_context", "apologizing_case_variants_order",
            "apologizing_boundary_negative", "apologizing_quote_suppression",
            "apologizing_raw_substring", "apologizing_selected_config_interaction", "apologizing_long_s_casefold",
            "whence_positive_context", "whence_case_variants_order",
            "whence_boundary_negative", "whence_quote_suppression",
            "whence_raw_substring", "whence_selected_config_interaction",
            "kudos_positive_context", "kudos_case_variants_order",
            "kudos_boundary_negative", "kudos_quote_suppression",
            "kudos_raw_substring", "kudos_selected_config_interaction", "kudos_kelvin_casefold",
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
            "back_formations_punctuation_boundaries", "back_formations_selected_config_interaction", "back_formations_fastset_activation_raw_copy", "inferior_superior_fastset_activation_raw_copy",
            "institution_name_positive_context", "institution_name_case_variants_order",
            "institution_name_word_boundary_negative", "institution_name_quote_suppression",
            "institution_name_punctuation_boundaries", "institution_name_selected_config_interaction", "institution_name_fastset_activation_raw_copy",
            "professions_shoe_repair_positive", "professions_geometrist_positive",
            "professions_case_variants_order", "professions_boundary_negative",
            "professions_quote_suppression", "professions_selected_config_interaction", "professions_fastset_activation_raw_copy",
            "false_plurals_phenomenons_positive", "false_plurals_talismen_positive",
            "false_plurals_case_variants_order", "false_plurals_boundary_negative",
            "false_plurals_quote_suppression", "false_plurals_selected_config_interaction", "false_plurals_fastset_activation_raw_copy",
            "false_plurals_talismen_long_s_mapping_lookup",
            "ellipsis_ascii_approximation_positive", "ellipsis_adjacent_nonoverlapping_order",
            "ellipsis_negative_unicode_symbol_and_short_run", "ellipsis_quote_suppression",
            "ellipsis_unicode_codepoint_spans", "ellipsis_selected_config_interaction",
            "copyright_ascii_approximation_positive", "copyright_case_variant_nested_and_substring_order",
            "copyright_negative_spacing_and_symbol", "copyright_quote_suppression",
            "copyright_unicode_codepoint_positions", "copyright_selected_config_interaction",
            "sentence_spacing_three_spaces_positive", "sentence_spacing_four_spaces_prefix_span",
            "sentence_spacing_multiple_order", "sentence_spacing_two_spaces_and_other_punctuation_negative",
            "sentence_spacing_quote_suppression", "sentence_spacing_unicode_codepoint_positions",
            "sentence_spacing_selected_config_interaction",
            "trademark_tm_positive", "trademark_registered_r_positive",
            "trademark_case_variants_both_entries_order", "trademark_negative_near_misses",
            "trademark_quote_suppression", "trademark_unicode_codepoint_positions",
            "trademark_selected_config_interaction",
            "multiplication_compact_positive", "multiplication_optional_spaces_case_variants",
            "multiplication_raw_substring_multiple", "multiplication_negative_symbol_spacing_and_unicode_digits",
            "multiplication_quote_suppression", "multiplication_unicode_codepoint_positions",
            "multiplication_selected_config_interaction",
            "suddenly_case_sensitive_positive", "suddenly_lowercase_case_sensitive_negative",
            "suddenly_raw_substring_multiple", "suddenly_paired_quote_suppression",
            "suddenly_comma_and_case_requirement_negative", "suddenly_unicode_codepoint_positions",
            "suddenly_selected_config_interaction",
            "but_start_of_input_positive", "but_multiple_blank_paragraphs",
            "but_single_newline_not_new_paragraph", "but_case_sensitive_word_boundary",
            "but_inline_and_indented_negative", "but_quoted_paragraph_start_suppression",
            "but_crlf_file_blank_paragraphs", "but_selected_config_interaction",
            "am_pm_lowercase_one_digit_compact", "am_pm_lowercase_two_digit_single_space",
            "am_pm_lowercase_multiple_order", "am_pm_uppercase_and_periods_negative",
            "am_pm_multi_space_and_three_digit_negative", "am_pm_word_boundary_controls",
            "am_pm_quote_suppression", "am_pm_unicode_codepoint_positions",
            "am_pm_selected_config_interaction",
            "am_pm_spacing_case_insensitive_optional_periods", "am_pm_spacing_one_and_two_digit_hours",
            "am_pm_spacing_required_touching_negative", "am_pm_spacing_three_digit_partial_substrings",
            "am_pm_spacing_embedded_unanchored_matches", "am_pm_spacing_multiple_order",
            "am_pm_spacing_quote_suppression", "am_pm_spacing_selected_config_interaction",
            "midnight_noon_optional_forms", "midnight_noon_case_and_period_variants",
            "midnight_noon_unanchored_and_near_misses", "midnight_noon_quote_suppression",
            "midnight_noon_unicode_codepoint_position", "midnight_noon_shared_path_limit_one",
            "midnight_noon_shared_path_limit_two", "midnight_noon_shared_path_limit_three",
            "midnight_noon_shared_path_all_findings",
            "midnight_noon_fastset_activation_preserves_embedded_raw_matches",
            "decade_apostrophes_overlap_both_registrations",
            "decade_apostrophes_case_and_length_variants",
            "decade_apostrophes_negative_near_misses",
            "decade_apostrophes_quote_suppression",
            "decade_apostrophes_embedded_raw_match",
            "decade_apostrophes_unicode_codepoint_positions",
            "decade_apostrophes_max_errors_one", "decade_apostrophes_max_errors_two",
            "decade_apostrophes_max_errors_three",
            "decade_apostrophes_ascii_digit_semantics_arabic_indic_prefix",
            "narcissism_scientists_positive", "narcissism_case_insensitive",
            "narcissism_extra_context_negative", "narcissism_have_suffix_negative",
            "narcissism_embedded_raw_match", "narcissism_quote_suppression",
            "narcissism_unicode_codepoint_positions", "narcissism_max_errors_one",
            "narcissism_max_errors_two",
            "debased_phrase_not_unjustifiable", "debased_phrase_leaves_desired",
            "debased_phrase_serve_no_purpose", "debased_phrase_consideration_bear_in_mind",
            "debased_case_variants_all_phrases_order", "debased_word_boundary_negatives",
            "debased_spacing_negatives", "debased_punctuation_boundaries_all_phrases",
            "debased_multiple_occurrences_order", "debased_quote_suppression",
            "debased_unicode_codepoint_positions", "debased_selected_config_interaction",
            "currency_no_digits", "currency_one_and_multiple_digits",
            "currency_all_suffixes_case_variants", "currency_ascii_space_tab_two_space",
            "currency_leading_nonboundary", "currency_suffix_word_boundary",
            "currency_quote_suppression", "currency_non_ascii_digit_engine_behavior",
            "currency_multiple_suffix_order", "currency_unicode_codepoint_positions",
            "currency_selected_config_interaction",
            "inferior_superior_inferior_form", "inferior_superior_superior_form",
            "inferior_superior_case_variants_order", "inferior_superior_correct_forms_clean",
            "inferior_superior_embedded_word_boundary_negatives", "inferior_superior_punctuation_boundaries",
            "inferior_superior_quote_suppression", "inferior_superior_reverse_source_order",
            "inferior_superior_unicode_codepoint_positions", "inferior_superior_selected_config_interaction",
            "greylist_obviously_independent", "greylist_utilize_independent",
            "greylist_case_variants_repeated_hits", "greylist_embedded_substrings",
            "greylist_mixed_terms_source_order", "greylist_quote_suppression",
            "greylist_unicode_codepoint_positions", "greylist_selected_config_interaction",
            "bureaucratese_meet_form", "bureaucratese_meets_form",
            "bureaucratese_case_variants_order", "bureaucratese_embedded_unanchored",
            "bureaucratese_punctuation_boundaries", "bureaucratese_near_misses",
            "bureaucratese_quote_suppression", "bureaucratese_repeated_matches_order",
            "bureaucratese_unicode_positions", "bureaucratese_selected_config_interaction",
            "hedging_i_would_argue", "hedging_so_to_speak_after_word",
            "hedging_comma_boundary_start_whitespace_negative", "hedging_to_certain_degree",
            "hedging_case_variants_mixed_order", "hedging_word_boundary_near_misses",
            "hedging_punctuation_boundaries", "hedging_quote_suppression",
            "hedging_repeated_match_order", "hedging_unicode_codepoint_positions",
            "hedging_selected_config_interaction", "hedging_fastset_activation_raw_copy",
            "jargon_all_six_phrases", "jargon_case_variants_surface_messages",
            "jargon_longer_word_boundary_negatives", "jargon_punctuation_boundaries",
            "jargon_quote_suppression_with_unquoted_hits", "jargon_repeats_mixed_source_order",
            "jargon_unicode_codepoint_positions", "jargon_selected_config_interaction",
            "jargon_fastset_activation_raw_copy",
            "airlinese_all_eight_inflections", "airlinese_taking_off_momentarily",
            "airlinese_case_variants_surface_messages", "airlinese_upstream_nonmatches",
            "airlinese_longer_word_boundary_negatives", "airlinese_spacing_near_misses",
            "airlinese_punctuation_boundaries", "airlinese_quote_suppression_with_unquoted_hits",
            "airlinese_repeated_mixed_source_order", "airlinese_unicode_codepoint_positions",
            "airlinese_selected_config_interaction",
            "many_a_all_three_forms", "many_a_case_variants_replacements_order",
            "many_a_punctuation_boundaries", "many_a_longer_word_boundary_negatives",
            "many_a_correct_forms_clean", "many_a_quote_suppression_with_unquoted_match",
            "many_a_repeated_mixed_source_order", "many_a_unicode_codepoint_positions",
            "many_a_selected_config_interaction", "many_a_fastset_activation_raw_copy",
            "metadiscourse_all_five_phrases", "metadiscourse_case_variants_order",
            "metadiscourse_preserves_upstream_misspelling", "metadiscourse_word_boundary_negatives",
            "metadiscourse_near_misses", "metadiscourse_quote_suppression_with_unquoted",
            "metadiscourse_repeated_source_order", "metadiscourse_unicode_codepoint_positions",
            "metadiscourse_selected_config_interaction",
            "pretension_all_four_terms", "pretension_case_variants_surface_messages",
            "pretension_longer_word_boundary_negatives", "pretension_punctuation_boundaries",
            "pretension_quote_suppression_with_unquoted", "pretension_repeated_mixed_source_order",
            "pretension_unicode_codepoint_positions", "pretension_selected_config_interaction",
            "latin_all_four_preferred_forms", "latin_case_variants_and_replacements",
            "latin_longer_word_boundary_negatives", "latin_near_miss_forms",
            "latin_punctuation_boundaries", "latin_quote_suppression_with_unquoted",
            "latin_repeated_mixed_source_order", "latin_unicode_codepoint_positions",
            "latin_selected_config_interaction",
            "seasons_winter_summer_mappings", "seasons_lower_upper_mixed_case_clean",
            "seasons_commented_spring_fall_clean", "seasons_longer_word_boundary_negatives",
            "seasons_punctuation_boundaries", "seasons_quote_suppression_with_unquoted",
            "seasons_repeated_source_order", "seasons_unicode_codepoint_positions",
            "seasons_selected_config_interaction", "seasons_fastset_activation_raw_copy",
            "capitalization_terms_all_mappings", "capitalization_terms_preferred_forms_clean",
            "capitalization_terms_case_variants_clean", "capitalization_terms_longer_word_boundary_negatives",
            "capitalization_terms_punctuation_boundaries", "capitalization_terms_quote_suppression_with_unquoted",
            "capitalization_terms_repeated_source_order", "capitalization_terms_unicode_codepoint_positions",
            "capitalization_terms_selected_config_interaction", "capitalization_terms_fastset_activation_raw_copy",
            "capitalization_days_all_seven_mappings", "capitalization_days_case_sensitive_clean",
            "capitalization_days_longer_word_boundaries_clean", "capitalization_days_punctuation_boundaries",
            "capitalization_days_quote_suppression_with_unquoted", "capitalization_days_repeated_source_order",
            "capitalization_days_max_errors_first_source_order", "capitalization_days_unicode_codepoint_positions",
            "capitalization_days_fastset_activation_raw_copy", "capitalization_days_selected_config_interaction",
            "months_all_ten_lowercase_mappings", "months_uppercase_and_excluded_march_may_are_clean",
            "months_embedded_term_without_boundary_activation_is_clean", "months_boundary_activation_then_raw_substrings",
            "months_quote_suppression_and_repeated_matches", "months_unicode_codepoint_coordinates",
            "lie_detector_both_preferred_forms", "lie_detector_case_variants_replacements",
            "lie_detector_longer_word_boundary_negatives", "lie_detector_near_miss_phrases",
            "lie_detector_punctuation_boundaries", "lie_detector_quote_suppression_with_unquoted",
            "lie_detector_repeated_mixed_source_order", "lie_detector_unicode_codepoint_positions",
            "lie_detector_selected_config_interaction", "lie_detector_fastset_activation_raw_copy", "lie_detector_unicode_boundary",
            "wallace_both_redundant_forms", "wallace_case_variants_surface_text",
            "wallace_longer_word_boundary_negatives", "wallace_near_miss_phrases",
            "wallace_punctuation_boundaries", "wallace_quote_suppression_with_unquoted",
            "wallace_repeated_mixed_source_order", "wallace_unicode_codepoint_positions",
            "wallace_selected_config_interaction",
            "bureaucratese_casefold_long_s", "hedging_casefold_long_s", "jargon_casefold_long_s", "airlinese_casefold_kelvin", "debased_casefold_long_s", "currency_casefold_long_s", "inferior_superior_casefold_long_s", "coin_casefold_long_s", "collusion_casefold_long_s", "back_formations_casefold_long_s", "greylist_casefold_long_s", "institution_name_casefold_long_s", "latin_casefold_long_s", "metadiscourse_casefold_long_s", "professions_casefold_long_s", "lie_detector_casefold_long_s", "wallace_casefold_long_s", "pretension_casefold_long_s", "annotations_max_errors_first_source_order", "false_plurals_misc_max_errors_source_order", "cli_json_missing_parent_source_absolute_error", "cli_json_invalid_config_structured_error",
            "per_file_parent_group_specific_nested_precedence", "per_file_bracket_character_class", "per_file_overlap_insertion_order_precedence",
            "rule_misc_phrasal_adjectives_ly_positive", "rule_misc_phrasal_adjectives_ly_negative",
            "rule_misc_phrasal_adjectives_ly_whitespace", "rule_misc_phrasal_adjectives_ly_nbsp",
            "rule_misc_phrasal_adjectives_ly_unicode", "rule_misc_phrasal_adjectives_ly_quoted",
            "rule_misc_phrasal_adjectives_ly_config_only",
            "psychology_p_equals_zero_positive_casefold", "psychology_p_equals_zero_unescaped_dot",
            "psychology_p_equals_zero_negative_controls", "psychology_p_equals_zero_excess_zeros",
            "psychology_p_equals_zero_dot_excludes_newline", "psychology_p_equals_zero_quote_suppression",
            "psychology_p_equals_zero_repeated_matches", "psychology_p_equals_zero_unicode_positions",
            "spelling_ance_ence_all_three", "spelling_ance_ence_correct_forms",
            "spelling_ance_ence_mixed_case", "spelling_ance_ence_longer_word_controls",
            "spelling_ance_ence_quote_suppression", "spelling_ance_ence_repeats_pattern_order",
            "spelling_ance_ence_unicode_codepoint_prefix", "spelling_ance_ence_selected_config",
            "spelling_ance_ence_max_errors_pattern_order", "spelling_ance_ence_boundary_then_embedded_substring",
            "composition_all_simple_forms", "composition_regex_captures_and_variants",
            "composition_duplicate_simple_regex_overlap", "composition_nested_matches_unlimited",
            "composition_nested_matches_max_errors_one", "composition_case_insensitive_surface",
            "composition_boundaries_and_correct_forms_clean", "composition_quote_suppression_with_outside_matches",
            "composition_source_order_and_tied_duplicates", "composition_unicode_codepoint_positions",
            "composition_selected_only_config",
            "composition_regex_long_s_uses_registered_replacement", "composition_regex_ascii_word_boundaries_adjacent_to_unicode_letters",
            "composition_matcher_order_not_important_first_limit_1",
            "composition_matcher_order_not_important_first_limit_2",
            "composition_matcher_order_not_important_first_unlimited",
            "composition_matcher_order_not_honest_repeated_limit_1",
            "composition_matcher_order_not_honest_repeated_limit_2",
            "composition_matcher_order_not_honest_repeated_unlimited",
            "composition_matcher_order_not_succeed_then_did_not_limit_1",
            "composition_matcher_order_not_succeed_then_did_not_limit_2",
            "composition_matcher_order_not_succeed_then_did_not_unlimited",
            "composition_matcher_order_did_not_succeed_then_not_limit_1",
            "composition_matcher_order_did_not_succeed_then_not_limit_2",
            "composition_matcher_order_did_not_succeed_then_not_unlimited",
            "composition_matcher_order_quoted_activation_limit_1",
            "composition_matcher_order_quoted_activation_limit_2",
            "composition_matcher_order_quoted_activation_unlimited",
            "composition_matcher_order_raw_xnot_activation_limit_1",
            "composition_matcher_order_raw_xnot_activation_limit_2",
            "composition_matcher_order_raw_xnot_activation_unlimited",
            "composition_matcher_order_regex_overlap_before_other_simple_limit_1",
            "composition_matcher_order_regex_overlap_before_other_simple_limit_2",
            "composition_matcher_order_regex_overlap_before_other_simple_unlimited",
            "composition_unicode_boundary_non_ascii_prefix",
            "composition_unicode_boundary_non_ascii_suffix",
            "composition_unicode_boundary_combining_mark",
            "composition_unicode_casefold_long_s",
            "cli_combined_registry_order_limit_before_sort",
            "cli_combined_position_sort_after_unlimited_checks",
            "cli_combined_equal_span_duplicates_preserved",
            "terms_eponymous_adjectives_simple_forms",
            "terms_eponymous_adjectives_clean_boundaries",
            "terms_eponymous_adjectives_unicode_positions",
            "terms_eponymous_adjectives_max_errors_one",
            "spelling_ally_ly_all_mappings",
            "spelling_ally_ly_case_and_repetition",
            "spelling_ally_ly_clean_and_word_boundaries",
            "spelling_ally_ly_raw_occurrences_after_activation",
            "spelling_ally_ly_quote_suppression",
            "spelling_ally_ly_unicode_positions",
            "spelling_ally_ly_max_errors_one",
            "spelling_ally_ly_limit_activation_order",
            "spelling_ally_ly_long_s_lookup",
            "spelling_ally_ly_quoted_activation",
            "spelling_ely_ly_all_mappings", "spelling_ely_ly_case_and_repetition",
            "spelling_ely_ly_clean_and_word_boundaries", "spelling_ely_ly_raw_occurrences_after_activation",
            "spelling_ely_ly_quote_suppression", "spelling_ely_ly_unicode_positions",
            "spelling_ely_ly_max_errors_one", "spelling_ely_ly_limit_activation_order",
            "spelling_ely_ly_quoted_activation",
            "spelling_ve_of_all_mappings", "spelling_ve_of_case_and_repetition",
            "spelling_ve_of_clean_and_word_boundaries", "spelling_ve_of_raw_occurrences_after_activation",
            "spelling_ve_of_quote_suppression", "spelling_ve_of_unicode_positions",
            "spelling_ve_of_max_errors_one", "spelling_ve_of_limit_activation_order",
            "spelling_ve_of_quoted_activation", "spelling_ve_of_long_s_lookup",
            "spelling_ve_of_long_s_no_activation",
            "malapropisms_all_phrases",
            "malapropisms_mixed_case_repetition",
            "malapropisms_clean_and_boundaries",
            "malapropisms_raw_after_activation",
            "malapropisms_quote_suppression",
            "malapropisms_unicode_positions",
            "malapropisms_max_errors_one",
            "malapropisms_cross_pattern_activation_order",
            "malapropisms_quoted_activation",
            "malapropisms_long_s_surface",
            "spelling_in_un_all_mappings",
            "spelling_in_un_case_and_repetition",
            "spelling_in_un_clean_boundaries",
            "spelling_in_un_raw_after_activation",
            "spelling_in_un_quote_suppression",
            "spelling_in_un_unicode_positions",
            "spelling_in_un_max_errors_one",
            "spelling_in_un_activation_order",
            "spelling_in_un_long_s_lookup",
            "spelling_in_un_quoted_activation_limit",
            "mondegreens_all_mappings",
            "mondegreens_mixed_case_and_repetition",
            "mondegreens_correct_target_forms_clean",
            "mondegreens_embedded_only_clean",
            "mondegreens_raw_occurrences_after_activation",
            "mondegreens_quote_suppression",
            "mondegreens_unicode_positions",
            "mondegreens_max_errors_activation_order",
            "mondegreens_punctuation_context",
            "mondegreens_quoted_activation_limit_one",
            "skunked_terms_all_standard", "skunked_terms_thankfully_word_boundary",
            "skunked_terms_thankfully_end_clean", "skunked_terms_thankfully_space_clean",
            "skunked_terms_mixed_case_repetition", "skunked_terms_clean_boundaries",
            "skunked_terms_quote_suppression", "skunked_terms_unicode_positions",
            "skunked_terms_punctuation_context", "skunked_terms_max_errors_one",
            "skunked_terms_quoted_activation_limit_one",
            "oxymorons_all_registered_phrases", "oxymorons_optional_s_variants",
            "oxymorons_backwardness_isolated_clean", "oxymorons_backwardness_activation_order",
            "oxymorons_boundary_checks", "oxymorons_quote_suppression",
            "oxymorons_overlapping_phrase_variants", "oxymorons_unicode_positions",
            "oxymorons_punctuation_case_repetition", "oxymorons_max_errors_one",
            "oxymorons_quoted_activation_limit_one",
            "oxymorons_ascii_word_activation_after_non_ascii_letter",
            "oxymorons_matcher_order_truncation_before_position_sort",
            "oxymorons_interleaved_pattern_order_max_errors_two",
            "oxymorons_long_s_nonword_boundaries_clean",
            "oxymorons_optional_s_backtracks_at_nonword_boundary",
            "not_guilty_optional_articles_case_variants",
            "not_guilty_near_misses_clean",
            "not_guilty_quote_filter_before_limit",
            "not_guilty_unicode_codepoint_position",
            "nordquist_ordinary_mappings", "nordquist_escaped_dot_matches_and_none_replacements",
            "nordquist_escaped_dot_alternates_clean", "nordquist_case_insensitive_surface",
            "nordquist_word_boundaries", "nordquist_quote_suppression",
            "nordquist_unicode_ascii_boundary", "nordquist_raw_occurrences_after_activation",
            "nordquist_max_errors_one", "nordquist_max_errors_two_matcher_order",
            "nordquist_max_errors_three",
            "spelling_er_or_all_mappings", "spelling_er_or_clean_forms_and_negative_controls",
            "spelling_er_or_mixed_case_repetition", "spelling_er_or_boundaries", "spelling_er_or_quotes",
            "spelling_er_or_unicode_positions", "spelling_er_or_max_errors_one", "spelling_er_or_max_errors_order",
            "spelling_able_ible_all_mappings", "spelling_able_ible_clean_forms_and_negative_controls",
            "spelling_able_ible_mixed_case_repetition", "spelling_able_ible_boundaries", "spelling_able_ible_quotes",
            "spelling_able_ible_unicode_positions", "spelling_able_ible_max_errors_one", "spelling_able_ible_max_errors_order",
            "spelling_able_ible_unicode_casefolds",
            "spelling_able_atable_representative_mappings", "spelling_able_atable_clean_controls",
            "spelling_able_atable_case_and_repetition", "spelling_able_atable_ascii_boundaries",
            "spelling_able_atable_quote_behavior", "spelling_able_atable_unicode_spans",
            "spelling_able_atable_max_errors_one", "spelling_able_atable_max_errors_two",
            "spelling_em_im_en_in_all_mappings", "spelling_em_im_en_in_preferred_and_negative_controls",
            "spelling_em_im_en_in_mixed_case_repetition", "spelling_em_im_en_in_ascii_boundaries_and_embedded",
            "spelling_em_im_en_in_quote_behavior", "spelling_em_im_en_in_unicode_positions_casefold",
            "spelling_em_im_en_in_max_errors_one_order", "spelling_em_im_en_in_max_errors_two_order",
        }
        self.assertEqual({item["id"] for item in manifest["cases"]}, expected_case_ids)
        self.assertEqual(len(manifest["cases"]), len(expected_case_ids))

        self.assertEqual(
            {capability: sum(case["capability_id"] == capability for case in manifest["cases"])
             for capability in {case["capability_id"] for case in manifest["cases"]}},
            {'rule.misc.debased': 13, 'rule.misc.capitalization.days': 10, 'rule.cliches.hell': 7, 'harness.oracle_smoke': 2, 'rule.dates_times.am_pm.lowercase_periods': 9, 'rule.dates_times.am_pm.midnight_noon': 10, 'rule.dates_times.dates.decade_apostrophes': 10, 'rule.misc.narcissism': 9, 'rule.misc.latin': 10, 'rule.misc.back_formations': 8, 'rule.misc.capitalization.terms': 10, 'rule.industrial_language.airlinese': 12, 'cli.json_no_findings': 8, 'rule.misc.phrasal_adjectives.ly': 7, 'rule.misc.greylist': 9, 'rule.misc.many_a': 10, 'cli.output_format.full': 3, 'cli.config.ignored_values': 5, 'rule.misc.professions': 8, 'cli.config.cwd_discovery': 3, 'cli.config.ancestor_discovery': 5, 'rule.misc.illogic.coin': 7, 'rule.hedging': 13, 'cli.config.invalid_utf8': 5, 'rule.misc.whence': 6, 'rule.annotations.misc': 10, 'rule.typography.symbols.sentence_spacing': 7, 'cli.config.consumed_special_values': 12, 'cli.config.malformed_json': 22, 'rule.typography.symbols.trademark': 7, 'rule.typography.symbols.copyright': 6, 'rule.misc.pretension': 9, 'cli.config.recursion_depth': 12, 'rule.industrial_language.bureaucratese': 11, 'rule.misc.institution_name': 8, 'rule.misc.currency': 12, 'rule.misc.suddenly': 7, 'cli.coordinates': 6, 'rule.misc.but': 8, 'rule.psychology.mental_telepathy': 6, 'rule.misc.illogic.collusion': 7, 'rule.misc.false_plurals.kudos': 7, 'rule.typography.symbols.multiplication': 7, 'rule.misc.capitalization.seasons': 10, 'cli.config.directory_path': 4, 'rule.misc.apologizing': 7, 'rule.misc.metadiscourse': 10, 'rule.misc.inferior_superior': 12, 'rule.misc.false_plurals.misc': 9, 'cli.config.per_file_checks': 4, 'rule.redundancy.misc.wallace': 10, 'rule.dates_times.am_pm.spacing': 8, 'rule.typography.symbols.ellipsis': 6, 'rule.psychology.lie_detector': 12, 'rule.industrial_language.jargon': 10, 'cli.config.user_discovery': 8, 'rule.misc.capitalization.months': 6, 'rule.psychology.p_equals_zero': 8, 'spelling.ance_ence': 10, 'rule.misc.composition': 38, 'cli.combined_checks': 3, 'rule.terms.eponymous_adjectives': 4, 'rule.spelling.ally_ly': 10, 'rule.spelling.ely_ly': 9, 'rule.spelling.ve_of': 11, 'rule.malapropisms': 10, 'rule.spelling.in_un': 10, 'rule.mondegreens': 10, 'rule.skunked_terms': 11, 'rule.oxymorons': 16, 'rule.redundancy.misc.nordquist': 11, 'rule.misc.not_guilty': 4, 'spelling.er_or': 8, 'spelling.able_ible': 9, 'spelling.able_atable': 8, 'spelling.em_im_en_in': 8},
        )

    def test_mondegreens_cases_select_only_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.mondegreens"]
        self.assertEqual({case["id"] for case in cases}, {
            "mondegreens_all_mappings", "mondegreens_mixed_case_and_repetition",
            "mondegreens_correct_target_forms_clean", "mondegreens_embedded_only_clean",
            "mondegreens_raw_occurrences_after_activation", "mondegreens_quote_suppression",
            "mondegreens_unicode_positions", "mondegreens_max_errors_activation_order",
            "mondegreens_punctuation_context", "mondegreens_quoted_activation_limit_one",
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"mondegreens": True})
        self.assertEqual(
            [json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"] for case in cases],
            [1000, 1000, 1000, 1000, 1000, 1000, 1000, 1, 1000, 1],
        )

    def test_skunked_terms_cases_select_only_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.skunked_terms"]
        self.assertEqual({case["id"] for case in cases}, {
            "skunked_terms_all_standard", "skunked_terms_thankfully_word_boundary",
            "skunked_terms_thankfully_end_clean", "skunked_terms_thankfully_space_clean",
            "skunked_terms_mixed_case_repetition", "skunked_terms_clean_boundaries",
            "skunked_terms_quote_suppression", "skunked_terms_unicode_positions",
            "skunked_terms_punctuation_context", "skunked_terms_max_errors_one",
            "skunked_terms_quoted_activation_limit_one",
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"skunked_terms": True})
        limits = {
            case["id"]: json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"]
            for case in cases
        }
        self.assertEqual(limits["skunked_terms_max_errors_one"], 1)
        self.assertEqual(limits["skunked_terms_quoted_activation_limit_one"], 1)

    def test_oxymorons_cases_select_only_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.oxymorons"]
        self.assertEqual({case["id"] for case in cases}, {
            "oxymorons_all_registered_phrases", "oxymorons_optional_s_variants",
            "oxymorons_backwardness_isolated_clean", "oxymorons_backwardness_activation_order",
            "oxymorons_boundary_checks", "oxymorons_quote_suppression",
            "oxymorons_overlapping_phrase_variants", "oxymorons_unicode_positions",
            "oxymorons_punctuation_case_repetition", "oxymorons_max_errors_one",
            "oxymorons_quoted_activation_limit_one",
            "oxymorons_ascii_word_activation_after_non_ascii_letter",
            "oxymorons_matcher_order_truncation_before_position_sort",
            "oxymorons_interleaved_pattern_order_max_errors_two",
            "oxymorons_long_s_nonword_boundaries_clean",
            "oxymorons_optional_s_backtracks_at_nonword_boundary",
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"oxymorons": True})
        limits = {
            case["id"]: json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"]
            for case in cases
        }
        self.assertEqual(limits["oxymorons_max_errors_one"], 1)
        self.assertEqual(limits["oxymorons_quoted_activation_limit_one"], 1)
        self.assertEqual(limits["oxymorons_interleaved_pattern_order_max_errors_two"], 2)
        outcomes = {case["id"]: case["expected_oracle_outcome"] for case in cases}
        self.assertEqual(outcomes["oxymorons_backwardness_isolated_clean"], "clean")
        self.assertEqual(outcomes["oxymorons_boundary_checks"], "clean")
        self.assertEqual(outcomes["oxymorons_backwardness_activation_order"], "findings")
        self.assertEqual(outcomes["oxymorons_long_s_nonword_boundaries_clean"], "clean")
        self.assertEqual(outcomes["oxymorons_interleaved_pattern_order_max_errors_two"], "findings")
        self.assertEqual(outcomes["oxymorons_optional_s_backtracks_at_nonword_boundary"], "findings")

    def test_not_guilty_cases_select_only_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.misc.not_guilty"]
        self.assertEqual({case["id"] for case in cases}, {
            "not_guilty_optional_articles_case_variants",
            "not_guilty_near_misses_clean",
            "not_guilty_quote_filter_before_limit",
            "not_guilty_unicode_codepoint_position",
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"misc": {"not_guilty": True}})
        limits = {
            case["id"]: json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"]
            for case in cases
        }
        self.assertEqual(limits["not_guilty_quote_filter_before_limit"], 1)
        outcomes = {case["id"]: case["expected_oracle_outcome"] for case in cases}
        self.assertEqual(outcomes["not_guilty_near_misses_clean"], "clean")
        self.assertEqual(outcomes["not_guilty_quote_filter_before_limit"], "findings")

    def test_nordquist_cases_select_only_registered_check_and_error_limits(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.redundancy.misc.nordquist"]
        self.assertEqual(len(cases), 11)
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"redundancy": {"misc": {"nordquist": True}}})
        limits = {case["id"]: json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"] for case in cases}
        self.assertEqual([limits[f"nordquist_max_errors_{n}"] for n in ("one", "two_matcher_order", "three")], [1, 2, 3])
        outcomes = {case["id"]: case["expected_oracle_outcome"] for case in cases}
        self.assertEqual(outcomes["nordquist_escaped_dot_alternates_clean"], "clean")

    def test_eponymous_adjectives_cases_select_only_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.terms.eponymous_adjectives"]
        self.assertEqual({case["id"] for case in cases}, {
            "terms_eponymous_adjectives_simple_forms",
            "terms_eponymous_adjectives_clean_boundaries",
            "terms_eponymous_adjectives_unicode_positions",
            "terms_eponymous_adjectives_max_errors_one",
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"terms": {"eponymous_adjectives": True}})
        self.assertEqual(
            [json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"] for case in cases],
            [1000, 1000, 1000, 1],
        )

    def test_er_or_cases_select_only_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "spelling.er_or"]
        self.assertEqual({case["id"] for case in cases}, {
            "spelling_er_or_all_mappings", "spelling_er_or_clean_forms_and_negative_controls",
            "spelling_er_or_mixed_case_repetition", "spelling_er_or_boundaries", "spelling_er_or_quotes",
            "spelling_er_or_unicode_positions", "spelling_er_or_max_errors_one", "spelling_er_or_max_errors_order",
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"spelling": {"er_or": True}})
        self.assertEqual([json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"] for case in cases],
                         [1000, 1000, 1000, 1000, 1000, 1000, 1, 2])

    def test_able_ible_cases_select_only_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "spelling.able_ible"]
        self.assertEqual({case["id"] for case in cases}, {
            "spelling_able_ible_all_mappings", "spelling_able_ible_clean_forms_and_negative_controls",
            "spelling_able_ible_mixed_case_repetition", "spelling_able_ible_boundaries", "spelling_able_ible_quotes",
            "spelling_able_ible_unicode_positions", "spelling_able_ible_max_errors_one", "spelling_able_ible_max_errors_order",
            "spelling_able_ible_unicode_casefolds",
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"spelling": {"able_ible": True}})
        self.assertEqual([json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"] for case in cases],
                         [1000, 1000, 1000, 1000, 1000, 1000, 1, 2, 1000])

    def test_able_atable_cases_select_only_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "spelling.able_atable"]
        self.assertEqual({case["id"] for case in cases}, {
            "spelling_able_atable_representative_mappings", "spelling_able_atable_clean_controls",
            "spelling_able_atable_case_and_repetition", "spelling_able_atable_ascii_boundaries",
            "spelling_able_atable_quote_behavior", "spelling_able_atable_unicode_spans",
            "spelling_able_atable_max_errors_one", "spelling_able_atable_max_errors_two",
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"spelling": {"able_atable": True}})
        self.assertEqual([json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"] for case in cases],
                         [1000, 1000, 1000, 1000, 1000, 1000, 1, 2])

    def test_em_im_en_in_cases_select_only_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "spelling.em_im_en_in"]
        self.assertEqual({case["id"] for case in cases}, {
            "spelling_em_im_en_in_all_mappings", "spelling_em_im_en_in_preferred_and_negative_controls",
            "spelling_em_im_en_in_mixed_case_repetition", "spelling_em_im_en_in_ascii_boundaries_and_embedded",
            "spelling_em_im_en_in_quote_behavior", "spelling_em_im_en_in_unicode_positions_casefold",
            "spelling_em_im_en_in_max_errors_one_order", "spelling_em_im_en_in_max_errors_two_order",
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"spelling": {"em_im_en_in": True}})
        self.assertEqual([json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"] for case in cases],
                         [1000, 1000, 1000, 1000, 1000, 1000, 1, 2])

    def test_ally_ly_cases_select_only_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.spelling.ally_ly"]
        self.assertEqual({case["id"] for case in cases}, {
            "spelling_ally_ly_all_mappings",
            "spelling_ally_ly_case_and_repetition",
            "spelling_ally_ly_clean_and_word_boundaries",
            "spelling_ally_ly_raw_occurrences_after_activation",
            "spelling_ally_ly_quote_suppression",
            "spelling_ally_ly_unicode_positions",
            "spelling_ally_ly_max_errors_one",
            "spelling_ally_ly_limit_activation_order",
            "spelling_ally_ly_long_s_lookup",
            "spelling_ally_ly_quoted_activation",
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"spelling": {"ally_ly": True}})
        self.assertEqual(
            [json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"] for case in cases],
            [1000, 1000, 1000, 1000, 1000, 1000, 1, 1, 1, 1],
        )

    def test_ely_ly_cases_select_only_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.spelling.ely_ly"]
        self.assertEqual({case["id"] for case in cases}, {
            "spelling_ely_ly_all_mappings", "spelling_ely_ly_case_and_repetition",
            "spelling_ely_ly_clean_and_word_boundaries", "spelling_ely_ly_raw_occurrences_after_activation",
            "spelling_ely_ly_quote_suppression", "spelling_ely_ly_unicode_positions",
            "spelling_ely_ly_max_errors_one", "spelling_ely_ly_limit_activation_order",
            "spelling_ely_ly_quoted_activation",
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"spelling": {"ely_ly": True}})
        self.assertEqual(
            [json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"] for case in cases],
            [1000, 1000, 1000, 1000, 1000, 1000, 1, 1, 1],
        )

    def test_ve_of_cases_select_only_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.spelling.ve_of"]
        self.assertEqual({case["id"] for case in cases}, {
            "spelling_ve_of_all_mappings", "spelling_ve_of_case_and_repetition",
            "spelling_ve_of_clean_and_word_boundaries", "spelling_ve_of_raw_occurrences_after_activation",
            "spelling_ve_of_quote_suppression", "spelling_ve_of_unicode_positions",
            "spelling_ve_of_max_errors_one", "spelling_ve_of_limit_activation_order",
            "spelling_ve_of_quoted_activation", "spelling_ve_of_long_s_lookup",
            "spelling_ve_of_long_s_no_activation",
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"spelling": {"ve_of": True}})
        self.assertEqual(
            [json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"] for case in cases],
            [1000, 1000, 1000, 1000, 1000, 1000, 1, 1, 1, 1000, 1000],
        )

    def test_malapropisms_cases_select_only_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.malapropisms"]
        self.assertEqual({case["id"] for case in cases}, {
            "malapropisms_all_phrases", "malapropisms_mixed_case_repetition",
            "malapropisms_clean_and_boundaries", "malapropisms_raw_after_activation",
            "malapropisms_quote_suppression", "malapropisms_unicode_positions",
            "malapropisms_max_errors_one", "malapropisms_cross_pattern_activation_order",
            "malapropisms_quoted_activation", "malapropisms_long_s_surface",
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"malapropisms": True})
        self.assertEqual(
            [json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"] for case in cases],
            [1000, 1000, 1000, 1000, 1000, 1000, 1, 1, 1, 1000],
        )

    def test_combined_check_cases_enable_the_expected_paths(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "cli.combined_checks"]
        self.assertEqual({case["id"] for case in cases}, {
            "cli_combined_registry_order_limit_before_sort",
            "cli_combined_position_sort_after_unlimited_checks",
            "cli_combined_equal_span_duplicates_preserved",
        })
        expected = (
            {"misc": {"suddenly": True}, "psychology": {"p_equals_zero": True}},
            {"misc": {"suddenly": True}, "psychology": {"p_equals_zero": True}},
            {"misc": {"composition": True, "suddenly": True}},
        )
        for case, checks in zip(cases, expected, strict=True):
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], checks)
        self.assertEqual([json.loads(base64.b64decode(case["files"][0]["content_base64"]))["max_errors"] for case in cases], [1, 1000, 2])

    def test_phrasal_adjective_cases_select_only_the_registered_check(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.misc.phrasal_adjectives.ly"]
        self.assertEqual({case["id"] for case in cases}, {
            f"rule_misc_phrasal_adjectives_ly_{name}"
            for name in ("positive", "negative", "whitespace", "nbsp", "unicode", "quoted", "config_only")
        })
        for case in cases:
            self.assertEqual(len(case["files"]), 1)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"misc": {"phrasal_adjectives": {"ly": True}}})


    def test_manifest_rejects_duplicate_id_and_bad_base64(self) -> None:
        valid = compare.load_manifest()
        duplicate = {"schema_version": 1, "cases": [valid["cases"][0], valid["cases"][0]]}
        with tempfile.TemporaryDirectory(prefix="manifest space-é ") as directory:
            path = Path(directory) / "cases.json"
            path.write_text(json.dumps(duplicate), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "duplicate case id"):
                compare.load_manifest(path)
            invalid = json.loads(json.dumps(valid))
            stdin_case = next(case for case in invalid["cases"] if case["input_mode"] == "stdin")
            stdin_case["stdin_base64"] = "!"
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
