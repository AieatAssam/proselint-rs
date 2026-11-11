from __future__ import annotations

import json
import base64
import hashlib
import os
import py_compile
import re
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import compare
import setup_oracle


class ComparatorTests(unittest.TestCase):
    def _symlink_case(self) -> dict:
        return {
            "id": "symlink_fixture_test", "capability_id": "test", "provenance": "test",
            "input_mode": "files", "files": [{"path": "tree/child.txt", "content_base64": "eA=="}],
            "unused_files": ["tree/child.txt"], "symlinks": [{"path": "alias", "target": "tree"}], "argv": ["check", "alias"],
            "output_format": "full", "expected_oracle_outcome": "clean", "timeout_seconds": 1,
            "environment": {}, "workspace_mutates": False,
            "comparison": {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"},
        }

    def test_symlink_fixture_materializes_as_link(self) -> None:
        case = self._symlink_case()
        case["symlinks"].append({"path": "nested/alias", "target": "tree"})
        with tempfile.TemporaryDirectory() as tmp:
            workspace = Path(tmp)
            compare.materialize_files(case, workspace)
            compare.materialize_symlinks(case, workspace)
            alias = workspace / "alias"
            self.assertTrue(alias.is_symlink())
            self.assertEqual(os.readlink(alias), "tree")
            self.assertEqual((alias / "child.txt").read_text(), "x")
            nested_alias = workspace / "nested" / "alias"
            self.assertEqual(os.readlink(nested_alias), "../tree")
            self.assertEqual((nested_alias / "child.txt").read_text(), "x")

    def test_symlink_fixture_allows_confined_dangling_target(self) -> None:
        case = self._symlink_case()
        case["symlinks"] = [{"path": "alias.md", "target": "missing.txt", "dangling": True}]
        case["argv"] = ["check", "alias.md"]
        with tempfile.TemporaryDirectory() as tmp:
            workspace = Path(tmp)
            compare.materialize_files(case, workspace)
            compare.validate_case(case)
            compare.materialize_symlinks(case, workspace)
            alias = workspace / "alias.md"
            self.assertTrue(alias.is_symlink())
            self.assertFalse(alias.exists())
            self.assertEqual(os.readlink(alias), "missing.txt")

    def test_symlink_fixture_allows_self_loop(self) -> None:
        case = self._symlink_case()
        case["symlinks"] = [{"path": "loop.txt", "target": "loop.txt"}]
        compare.validate_case(case)
        with tempfile.TemporaryDirectory() as tmp:
            workspace = Path(tmp)
            compare.materialize_symlinks(case, workspace)
            self.assertTrue((workspace / "loop.txt").is_symlink())
            self.assertEqual(os.readlink(workspace / "loop.txt"), "loop.txt")

    def test_symlink_fixture_rejects_dangling_marker_for_represented_target(self) -> None:
        for symlinks in (
            [{"path": "alias.md", "target": "tree", "dangling": True}],
            [{"path": "nested/alias.md", "target": "nested", "dangling": True}],
            [{"path": "alias.md", "target": "other.md", "dangling": True}, {"path": "other.md", "target": "tree"}],
        ):
            with self.subTest(symlinks=symlinks):
                case = self._symlink_case()
                case["symlinks"] = symlinks
                with self.assertRaisesRegex(ValueError, "dangling"):
                    compare.validate_case(case)

    def test_symlink_fixture_rejects_unsafe_or_conflicting_declarations(self) -> None:
        for symlinks, message in (
            ([{"path": "/outside", "target": "tree"}], "symlink paths"),
            ([{"path": "alias", "target": "../outside"}], "symlink targets"),
            ([{"path": "alias", "target": "../outside", "dangling": True}], "symlink targets"),
            ([{"path": "alias", "target": "missing", "dangling": "true"}], "boolean"),
            ([{"path": "tree/child.txt", "target": "tree"}], "collides"),
            ([{"path": "tree", "target": "tree"}], "collides"),
            ([{"path": "alias", "target": "tree"}, {"path": "alias", "target": "tree"}], "duplicate"),
            ([{"path": "alias", "target": "missing"}], "target"),
        ):
            with self.subTest(symlinks=symlinks):
                case = self._symlink_case()
                case["symlinks"] = symlinks
                with self.assertRaisesRegex(ValueError, message):
                    compare.validate_case(case)

    def test_long_option_abbreviation_manifest_cases(self) -> None:
        expected = {
            "cli_long_option_abbrev_root_ambiguous_ver": ("--ver", "cli_error"),
            "cli_long_option_abbrev_check_dem": ("check --dem --config {file:selected config.json} --output-format compact", "findings"),
            "cli_long_option_abbrev_dump_config_def": ("dump-config --def", "clean"),
            "cli_long_option_abbrev_check_output_format_out": ("check --out full {file:input.txt}", "clean"),
            "cli_long_option_abbrev_check_help_he": ("check --he", "clean"),
            "cli_long_option_abbrev_global_config_con": ("--con {file:config.json} check {file:input.txt}", "clean"),
            "cli_long_option_abbrev_root_ambiguous_ver_attached": ("--ver=value", "cli_error"),
            "cli_long_option_abbrev_root_version_attached": ("--vers=value", "cli_error"),
            "cli_long_option_abbrev_check_demo_attached": ("check --dem=value", "cli_error"),
            "cli_long_option_abbrev_dump_default_attached": ("dump-config --def=value", "cli_error"),
        }
        cases = {case["id"]: case for case in compare.load_manifest()["cases"] if case["capability_id"] == "cli.long_option_abbreviation"}
        self.assertEqual(set(cases), set(expected))
        for case_id, (argv, outcome) in expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(" ".join(case["argv"]), argv)
                self.assertEqual(case["expected_oracle_outcome"], outcome)
                self.assertEqual(case["comparison"]["stderr"], "bytes")
                self.assertEqual(case["comparison"]["exit_status"], "exact")

    def test_unicode_repr_invalid_output_format_manifest_cases(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["id"].startswith("cli_invalid_output_format_") and case["id"].endswith(("u2028", "u2029", "u200b", "00a0", "1c89"))]
        expected = {
            f"cli_invalid_output_format_{command}_{suffix}": [command.replace("_", "-"), "--output-format", f"bad{character}value"]
            for command in ("check", "version", "dump_config")
            for suffix, character in (("u2028", "\u2028"), ("u2029", "\u2029"), ("u200b", "\u200b"), ("00a0", "\u00a0"), ("1c89", "\u1c89"))
        }
        self.assertEqual(len(cases), 15)
        self.assertEqual({case["id"]: case["argv"] for case in cases}, expected)
        self.assertTrue(all(case["capability_id"] == "cli.invalid_output_format_error" for case in cases))

    def test_non_object_config_root_manifest_cases(self) -> None:
        expected = {
            "cli_config_root_type_array", "cli_config_root_type_null",
            "cli_config_root_type_boolean", "cli_config_root_type_integer",
            "cli_config_root_type_float", "cli_config_root_type_large_integer",
            "cli_config_root_type_overflow_float", "cli_config_root_type_nan",
            "cli_config_root_type_infinity", "cli_config_root_type_negative_infinity",
            "cli_config_root_type_string",
            "cli_config_root_type_array_full", "cli_config_root_type_array_compact",
            "cli_config_root_type_array_version", "cli_config_root_type_array_dump_config",
            "cli_config_root_type_array_version_json", "cli_config_root_type_array_dump_config_json",
        }
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "cli.config.non_object_root"]
        self.assertEqual({case["id"] for case in cases}, expected)
        self.assertEqual(len(cases), 17)
        root_values = {
            "array": "[]", "null": "null", "boolean": "true", "integer": "1",
            "float": "1.0", "large_integer": "1" * 400, "overflow_float": "1e400",
            "nan": "NaN", "infinity": "Infinity", "negative_infinity": "-Infinity",
            "string": '"x"', "array_full": "[]", "array_compact": "[]",
            "array_version": "[]", "array_dump_config": "[]",
            "array_version_json": "[]", "array_dump_config_json": "[]",
        }
        for case in cases:
            with self.subTest(case=case["id"]):
                self.assertEqual(case["input_mode"], "files")
                self.assertEqual(case["files"][0]["path"], "config.json")
                root_kind = case["id"].removeprefix("cli_config_root_type_")
                self.assertEqual(base64.b64decode(case["files"][0]["content_base64"]).decode(), root_values[root_kind])
                self.assertEqual(case["expected_oracle_outcome"], "cli_error")
                if root_kind == "array_version":
                    self.assertEqual(case["argv"], ["--config", "{file:config.json}", "--version"])
                elif root_kind == "array_version_json":
                    self.assertEqual(case["argv"], ["--output-format", "json", "--config", "{file:config.json}", "--version"])
                elif root_kind == "array_dump_config":
                    self.assertEqual(case["argv"], ["dump-config", "--config", "{file:config.json}"])
                elif root_kind == "array_dump_config_json":
                    self.assertEqual(case["argv"], ["dump-config", "--output-format", "json", "--config", "{file:config.json}"])
                else:
                    self.assertEqual(case["argv"][0:3], ["check", "--config", "{file:config.json}"])

    def test_root_output_format_ignored_by_version_subcommand_compares_stdout_as_bytes(self) -> None:
        case = {
            "id": "version_parser_precedence",
            "capability_id": "test",
            "provenance": "test",
            "input_mode": "stdin",
            "stdin_base64": "",
            "files": [],
            "argv": ["--output-format", "json", "--version", "check"],
            "output_format": "json",
            "expected_oracle_outcome": "clean",
            "timeout_seconds": 2,
            "environment": {},
            "workspace_mutates": False,
            "comparison": {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"},
        }
        compare.validate_case(case)

        invalid_policy = json.loads(json.dumps(case))
        invalid_policy["comparison"]["stdout"] = "xml"
        with self.assertRaisesRegex(ValueError, "comparison.stdout"):
            compare.validate_case(invalid_policy)

    def test_case_workers_respect_half_cpu_budget_and_preserve_input_order(self) -> None:
        cases = [{"id": str(index)} for index in range(20)]
        expected = {None: 1, 1: 1, 3: 1, 4: 2, 5: 2, 16: 8}
        for cpu_count, workers in expected.items():
            with self.subTest(cpu_count=cpu_count), patch.object(os, "process_cpu_count", return_value=cpu_count, create=True):
                self.assertEqual(compare.case_worker_count(), workers)

        active = 0
        peak = 0
        lock = threading.Lock()

        def worker(case: dict[str, str]) -> str:
            nonlocal active, peak
            with lock:
                active += 1
                peak = max(peak, active)
            time.sleep(0.03)
            with lock:
                active -= 1
            return case["id"]

        with patch.object(os, "process_cpu_count", return_value=4, create=True):
            results = compare.run_cases_concurrently(cases, worker)
        self.assertEqual(results, [case["id"] for case in cases])
        self.assertEqual(peak, 2)

        with patch.object(os, "process_cpu_count", create=True) as process_cpu_count:
            del os.process_cpu_count
            process_cpu_count.side_effect = AttributeError
            with patch.object(os, "cpu_count", return_value=5):
                self.assertEqual(compare.case_worker_count(), 2)

    def test_focused_attestation_requires_nonempty_capability(self) -> None:
        focused = {"validation_scope": "focused_candidate_build"}
        with self.assertRaisesRegex(ValueError, "requires a capability-scoped comparison"):
            compare.validate_attestation_scope(focused, None)
        with self.assertRaisesRegex(ValueError, "requires a capability-scoped comparison"):
            compare.validate_attestation_scope(focused, "  ")
        compare.validate_attestation_scope(focused, "rule.sample")
        compare.validate_attestation_scope({"validation_scope": "full_gate"}, None)

    def test_reference_python_cache_prefix_bypasses_normal_bytecode_cache(self) -> None:
        with tempfile.TemporaryDirectory(prefix="pycache isolation-é ") as directory:
            root = Path(directory)
            module = root / "victim.py"
            module.write_text("value = 'evil'\n", encoding="utf-8")
            py_compile.compile(str(module), doraise=True)
            source_stat = module.stat()
            module.write_text("value = 'good'\n", encoding="utf-8")
            os.utime(module, (int(source_stat.st_mtime), int(source_stat.st_mtime)))

            code = "import sys; sys.path.insert(0, sys.argv[1]); import victim; print(victim.value)"
            env = os.environ.copy()
            env.pop("PYTHONPATH", None)
            env.pop("PYTHONPYCACHEPREFIX", None)
            unisolated = subprocess.run(
                [sys.executable, "-c", code, str(root)], env=env,
                capture_output=True, text=True, check=True,
            )
            self.assertEqual(unisolated.stdout.strip(), "evil")

            isolated_home = root / "isolated-home"
            isolated_home.mkdir()
            isolated_env = compare.fresh_env(isolated_home, {})
            self.assertEqual(isolated_env["PYTHONPYCACHEPREFIX"], str(isolated_home / "isolated-pycache"))
            isolated = subprocess.run(
                [sys.executable, "-c", code, str(root)], env=isolated_env,
                capture_output=True, text=True, check=True,
            )
            self.assertEqual(isolated.stdout.strip(), "good")

            with setup_oracle.scrubbed_env() as oracle_env:
                self.assertTrue(Path(oracle_env["PYTHONPYCACHEPREFIX"]).is_dir())

    def test_reference_cases_use_isolated_per_case_cache_prefixes(self) -> None:
        case = {"id":"cache_probe", "capability_id":"test", "provenance":"test", "input_mode":"stdin", "stdin_base64":"", "files":[], "argv":[], "output_format":"json", "expected_oracle_outcome":"clean", "timeout_seconds":2, "environment":{}, "workspace_mutates":False, "comparison":{"stdout":"json","stderr":"bytes","exit_status":"exact"}}
        probe = [sys.executable, "-c", "ignored"]
        stdout = b'{"result":{"<stdin>":{"diagnostics":[]}}}'
        observed_envs = []

        def fake_run(argv, *, cwd, env, stdin, timeout, role):
            observed_envs.append((role, env["PYTHONPYCACHEPREFIX"], env.get("PYTHONDONTWRITEBYTECODE")))
            module = cwd / "harness_cache_probe.py"
            module.write_text("value = 1\n", encoding="utf-8")
            subprocess.run([sys.executable, "-c", "import harness_cache_probe"], cwd=cwd, env=env, check=True)
            if role == "reference":
                self.assertEqual(list(Path(env["PYTHONPYCACHEPREFIX"]).rglob("*.pyc")), [])
            return {"argv": argv, "cwd": str(cwd), "role": role, "status": "OK", "exit_code": 0, "duration_seconds": 0.0, "stdout": stdout, "stderr": b""}

        with tempfile.TemporaryDirectory() as directory, patch.object(compare, "run_command", side_effect=fake_run):
            root = Path(directory)
            first = compare.run_case(case, probe, probe, root, None)
            second_case = dict(case, id="cache_probe_2", environment={"PYTHONDONTWRITEBYTECODE": "1"})
            second = compare.run_case(second_case, probe, probe, root, None)
            first_cache = first["effective_environment"]["reference"]["PYTHONPYCACHEPREFIX"]
            second_cache = second["effective_environment"]["reference"]["PYTHONPYCACHEPREFIX"]
            self.assertFalse(Path(first_cache).exists())
            self.assertFalse(Path(second_cache).exists())
            self.assertFalse((root / "cases" / "cache_probe" / "reference-pycache").exists())
            self.assertFalse((root / "cases" / "cache_probe_2" / "reference-pycache").exists())
        self.assertEqual(first["status"], "PASS", first)
        self.assertEqual(second["status"], "PASS", second)
        first_cache = first["effective_environment"]["reference"]["PYTHONPYCACHEPREFIX"]
        second_cache = second["effective_environment"]["reference"]["PYTHONPYCACHEPREFIX"]
        self.assertNotEqual(first_cache, second_cache)
        self.assertEqual([role for role, _, _ in observed_envs], ["reference", "candidate", "reference", "candidate"])
        self.assertEqual([flag for role, _, flag in observed_envs if role == "reference"], ["1", "1"])
        self.assertEqual([flag for role, _, flag in observed_envs if role == "candidate"], [None, "1"])
        self.assertNotEqual(first["effective_environment"]["candidate"]["PYTHONPYCACHEPREFIX"], first_cache)

    def test_reference_disables_bytecode_without_changing_multiplication_output(self) -> None:
        cases = json.loads(compare.MANIFEST.read_text(encoding="utf-8"))["cases"]
        case = next(case for case in cases if case["id"] == "multiplication_selected_config_interaction")
        reference = compare.default_reference_argv()
        stdin = base64.b64decode(case["stdin_base64"])
        outputs = []
        with tempfile.TemporaryDirectory(prefix="multiplication bytecode ") as directory:
            root = Path(directory)
            layout = root / "workspace"
            layout.mkdir()
            compare.materialize_files(case, layout)
            args = [arg.replace("{file:selected config.json}", "selected config.json") for arg in case["argv"]]
            for label, disable_bytecode in (("baseline", False), ("disabled", True)):
                home = root / label / "home"
                for name in ("xdg", "cache", "tmp"):
                    (home / name).mkdir(parents=True, exist_ok=True)
                cache = root / f"{label}-pycache"
                env = compare.fresh_env(home, case["environment"])
                env["PYTHONPYCACHEPREFIX"] = str(cache)
                if disable_bytecode:
                    env["PYTHONDONTWRITEBYTECODE"] = "1"
                result = compare.run_command(reference + args, cwd=layout, env=env, stdin=stdin, timeout=case["timeout_seconds"], role="reference")
                outputs.append((result["status"], result["exit_code"], result["stdout"], result["stderr"]))
                if disable_bytecode:
                    self.assertEqual(list(cache.rglob("*.pyc")) if cache.exists() else [], [])
                else:
                    self.assertTrue(list(cache.rglob("*.pyc")))
        self.assertEqual(outputs[0], outputs[1])

    def test_manifest_bytecode_settings_load_and_preserve_explicit_value(self) -> None:
        cases = compare.load_manifest()["cases"]
        explicit = [case for case in cases if "PYTHONDONTWRITEBYTECODE" in case["environment"]]
        self.assertEqual(len(explicit), 93)
        self.assertTrue(all(case["environment"]["PYTHONDONTWRITEBYTECODE"] == "1" for case in explicit))
        with tempfile.TemporaryDirectory() as directory:
            env = compare.fresh_env(Path(directory), explicit[0]["environment"])
        self.assertEqual(env["PYTHONDONTWRITEBYTECODE"], "1")

    def test_selected_check_order_matches_pinned_inventory_first_occurrence(self) -> None:
        source = (compare.RUST_ROOT / "src" / "main.rs").read_text(encoding="utf-8")
        declaration = re.search(r"enum SelectedCheck\s*\{(.*?)\n\}", source, re.S)
        mapping = re.search(r"fn selected_check_path\(check: SelectedCheck\).*?match check\s*\{(.*?)\n\s*\}\n\}", source, re.S)
        self.assertIsNotNone(declaration)
        self.assertIsNotNone(mapping)
        variants = re.findall(r"^\s*([A-Z][A-Za-z0-9_]*),\s*$", declaration.group(1), re.M)
        paths = dict(re.findall(r'SelectedCheck::([A-Za-z0-9_]+)\s*=>\s*"([^"]+)"', mapping.group(1)))
        self.assertEqual(set(variants), set(paths))
        self.assertTrue({
            "DatesTimesDatesMonthOfYear", "MiscScareQuotes", "TypographyPunctuationHyperbole",
            "TypographyPunctuationMisplaced", "TypographyPunctuationExclamation",
        }.issubset(paths))

        inventory = json.loads((compare.RUST_ROOT / "inventory.json").read_text(encoding="utf-8"))
        first_occurrence = {}
        for rule in inventory["rules"]:
            first_occurrence.setdefault(rule["check_path"], rule["registration_index"])
        indices = [first_occurrence[paths[variant]] for variant in variants]
        self.assertTrue(all(left < right for left, right in zip(indices, indices[1:])), indices)

    def test_installed_package_integrity_ignores_bytecode_but_checks_source(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pinned = root / "pinned" / "proselint"
            installed = root / "installed" / "proselint"
            for package in (pinned, installed):
                (package / "submodule").mkdir(parents=True)
                (package / "__pycache__").mkdir()
                (package / "module.py").write_bytes(b"value = 1\n")
                (package / "submodule" / "__init__.py").write_bytes(b"\n")
                (package / "__pycache__" / "module.cpython-312.pyc").write_bytes(b"initial bytecode")

            setup_oracle.require_package_matches_source(installed, pinned)
            expected = setup_oracle.package_fingerprint(pinned)
            self.assertEqual(setup_oracle.package_fingerprint(installed), expected)

            bytecode = installed / "__pycache__" / "module.cpython-312.pyc"
            bytecode.write_bytes(b"regenerated bytecode")
            self.assertEqual(setup_oracle.package_fingerprint(installed), expected)
            setup_oracle.require_package_matches_source(installed, pinned)
            bytecode.unlink()
            self.assertEqual(setup_oracle.package_fingerprint(installed), expected)
            setup_oracle.require_package_matches_source(installed, pinned)

            (installed / "module.py").write_bytes(b"value = 2\n")
            with self.assertRaisesRegex(RuntimeError, "source differs from pinned source"):
                setup_oracle.require_package_matches_source(installed, pinned)

    def test_next_rule_batch_manifest_ids_counts_and_selected_configs(self) -> None:
        expected = {
            "rule.typography.diacritical_marks": {
                "diacriticals_beyonce", "diacriticals_case_insensitive",
                "diacriticals_embedded_word_boundary", "diacriticals_raw_embedded_match_after_activation",
                "diacriticals_multiple_languages", "diacriticals_non_ascii_ascii_word_boundary",
                "diacriticals_correct_accented_forms", "diacriticals_quote_suppression",
                "diacriticals_unicode_coordinates", "diacriticals_max_errors_one",
                "diacriticals_max_errors_pattern_group_order", "diacriticals_source_typo_mapping",
                "diacriticals_long_s_requires_fastset_activation",
                "diacriticals_long_s_raw_match_after_activation", "diacriticals_kelvin_casefold",
                "sol_quote_embed_limit_1", "sol_overlap_limit_1", "sol_overlap_limit_2",
                "sol_all_keys", "sol_all_targets", "sol_fi_ligature", "sol_dotted_i",
            },
            "rule.dates_times.dates.month_of_year": {
                "month_of_year_all_months",
                "month_of_year_digits_quotes_unicode_positions",
                "month_of_year_ascii_digit_boundary",
            },
            "rule.misc.scare_quotes": {
                "scare_quotes_positive_and_boundary_controls",
                "scare_quotes_unicode_word_boundaries",
                "scare_quotes_quote_suppression",
            },
            "rule.typography.punctuation.hyperbole": {
                "hyperbole_raw_greedy_mixed_punctuation",
                "hyperbole_single_punctuation_negative",
                "hyperbole_unicode_word_match",
            },
            "rule.typography.punctuation.misplaced": {
                "misplaced_punctuation_ordered_fast_patterns",
                "misplaced_punctuation_word_boundaries",
                "misplaced_punctuation_unicode_positions",
            },
            "rule.typography.punctuation.exclamation": {
                "exclamation_two_match_threshold",
                "exclamation_padded_length_threshold_lower_edge",
                "exclamation_padded_length_threshold_upper_edge",
                "exclamation_quote_selected_then_suppressed",
            },
            "rule.typography.symbols.curly_quotes": {
                "curly_quotes_ascii_word_positive",
                "curly_quotes_ascii_digits_and_space",
                "curly_quotes_multiline_whitespace",
                "curly_quotes_punctuation_near_miss",
                "curly_quotes_empty_near_miss",
                "curly_quotes_allow_quotes_true",
                "curly_quotes_unicode_positions_and_content",
                "curly_quotes_max_errors_one",
                "curly_quotes_adjacent_quote_boundary_match",
            },
            "rule.weasel_words.very": {
                "weasel_very_positive_ordinary_case",
                "weasel_very_well_exclusions",
                "weasel_very_tabs_and_newlines",
                "weasel_very_unicode_letter_boundary",
                "weasel_very_quote_suppression",
                "weasel_very_max_errors_one",
                "weasel_very_combining_mark_and_ascii_boundaries",
                "weasel_very_multiline_position",
                "weasel_very_u1c89_leading", "weasel_very_u1c89_trailing",
                "weasel_very_u0345_leading", "weasel_very_u0345_trailing",
                "weasel_very_u05b0_leading", "weasel_very_u05b0_trailing",
            },
        }
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] in expected]
        by_capability = {capability: {case["id"] for case in cases if case["capability_id"] == capability} for capability in expected}
        self.assertEqual(by_capability, expected)

        for case in cases:
            with self.subTest(case=case["id"]):
                self.assertEqual(len(case["files"]), 1)
                config_path = case["files"][0]["path"]
                self.assertIn(config_path, {"config.json", "selected config.json"})
                self.assertEqual(case["argv"][2], "{file:" + config_path + "}")
                config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
                enabled = []

                def collect(value: object, path: str = "") -> None:
                    if isinstance(value, dict):
                        for key, child in value.items():
                            collect(child, f"{path}.{key}" if path else key)
                    elif value is True:
                        enabled.append(path)

                collect(config.get("checks"))
                self.assertEqual(enabled, [case["capability_id"].removeprefix("rule.")])

    def test_weasel_very_manifest_cases_and_exact_selected_config(self) -> None:
        expected = {
            "weasel_very_positive_ordinary_case", "weasel_very_well_exclusions",
            "weasel_very_tabs_and_newlines", "weasel_very_unicode_letter_boundary",
            "weasel_very_quote_suppression", "weasel_very_max_errors_one", "weasel_very_combining_mark_and_ascii_boundaries",
            "weasel_very_multiline_position",
            "weasel_very_u1c89_leading", "weasel_very_u1c89_trailing",
            "weasel_very_u0345_leading", "weasel_very_u0345_trailing",
            "weasel_very_u05b0_leading", "weasel_very_u05b0_trailing",
        }
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.weasel_words.very"]
        self.assertEqual({case["id"] for case in cases}, expected)
        for case in cases:
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"weasel_words": {"very": True}})
            self.assertEqual(case["argv"], ["check", "--config", "{file:" + case["files"][0]["path"] + "}", "--output-format", "json"])
        self.assertEqual(json.loads(base64.b64decode(next(c for c in cases if c["id"].endswith("max_errors_one"))["files"][0]["content_base64"]))["max_errors"], 1)

    def test_lexical_illusions_live_case_contracts(self) -> None:
        expected = {
            "lexical_illusions_single_repeat": ("word word", "findings", 1000),
            "lexical_illusions_exact_exceptions": ("had had; that that", "clean", 1000),
            "lexical_illusions_triple_repeats": ("had had had; that that that", "findings", 1000),
            "lexical_illusions_phrase_lengths_one_to_four": ("red red; a b a b; a b c a b c; a b c d a b c d", "findings", 1000),
            "lexical_illusions_more_than_four_words": ("a b c d e a b c d e", "clean", 1000),
            "lexical_illusions_whitespace_backreference_exact": ("a  b\ta  b", "findings", 1000),
            "lexical_illusions_whitespace_backreference_mismatch": ("a b a  b", "clean", 1000),
            "lexical_illusions_apostrophe_hyphen_boundaries": ("don't don't; well-known well-known", "clean", 1000),
            "lexical_illusions_unicode_casefold_and_word_semantics": ("ſ s; K k; İ i", "findings", 1000),
            "lexical_illusions_quote_suppression": ('She wrote “word word” and said \'that that\'.', "clean", 1000),
            "lexical_illusions_unicode_codepoint_offsets": ("x é😊 word word", "findings", 1000),
            "lexical_illusions_max_errors_one": ("one one; two two; three three", "findings", 1),
        }
        cases = {case["id"]: case for case in compare.load_manifest()["cases"]}
        self.assertTrue(set(expected).issubset(cases))
        for case_id, (text, outcome, max_errors) in expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["capability_id"], "rule.lexical_illusions")
                self.assertEqual(case["input_mode"], "stdin")
                self.assertEqual(base64.b64decode(case["stdin_base64"]).decode("utf-8"), text)
                self.assertEqual(case["expected_oracle_outcome"], outcome)
                self.assertEqual(case["argv"], ["check", "--config", "{file:config.json}", "--output-format", "json"])
                self.assertEqual(case["provenance"].split(";", 2)[0], "Pinned Python Proselint 0.16.0 at dbed789caae662d06c7c8a5a13dd31f1acd36f5c")
                config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
                self.assertEqual(config["checks"], {"lexical_illusions": True})
                self.assertEqual(config["max_errors"], max_errors)

    def test_punctuation_spacing_live_case_contracts(self) -> None:
        expected = {
            "punctuation_spacing_only_one_space": ("A. B. C! D?", "clean", 1000),
            "punctuation_spacing_only_two_spaces": ("A.  B.  C!  D?", "clean", 1000),
            "punctuation_spacing_one_space_majority": ("A. B. C. D.  E!", "findings", 1000),
            "punctuation_spacing_two_space_majority": ("A.  B.  C.  D! E?", "findings", 1000),
            "punctuation_spacing_tie": ("A. B.  C!", "findings", 1000),
            "punctuation_spacing_interleaved_occurrences": ("A. B.  C. D.  E! F? G.  H.", "findings", 1000),
            "punctuation_spacing_quoted_minority_suppressed": ("“A.  B.” C. D. E. F!", "clean", 1000),
            "punctuation_spacing_unicode_coordinates": ("é.  B! C. D? E.", "findings", 1000),
            "punctuation_spacing_max_errors_one": ("A. B.  C. D.  E! F. G?", "findings", 1),
        }
        cases = {case["id"]: case for case in compare.load_manifest()["cases"]}
        self.assertEqual({case_id for case_id in cases if case_id.startswith("punctuation_spacing_")}, set(expected))
        for case_id, (text, outcome, max_errors) in expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["capability_id"], "rule.typography.punctuation.spacing")
                self.assertEqual(case["input_mode"], "stdin")
                self.assertEqual(base64.b64decode(case["stdin_base64"]).decode("utf-8"), text)
                self.assertEqual(case["expected_oracle_outcome"], outcome)
                self.assertEqual(case["argv"], ["check", "--config", "{file:config.json}", "--output-format", "json"])
                self.assertEqual(case["provenance"].split(";", 2)[0], "Pinned Python Proselint 0.16.0 at dbed789caae662d06c7c8a5a13dd31f1acd36f5c")
                config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
                self.assertEqual(config["checks"], {"typography": {"punctuation": {"spacing": True}}})
                self.assertEqual(config["max_errors"], max_errors)

    def test_astra_interleaving_and_unicode_boundary_cases(self) -> None:
        cases = {case["id"]: case for case in compare.load_manifest()["cases"]}
        expected = {
            "jargon_interleave_limit_two": ("rule.industrial_language.jargon", "agendize; disincentivize; agendize", 2),
            "hedging_interleave_limit_two": ("rule.hedging", "I would argue that; to a certain degree; I would argue that", 2),
            "weasel_very_u1c89_leading": ("rule.weasel_words.very", "\u1c89very", 1000),
            "weasel_very_u1c89_trailing": ("rule.weasel_words.very", "very\u1c89", 1000),
            "weasel_very_u0345_leading": ("rule.weasel_words.very", "\u0345very", 1000),
            "weasel_very_u0345_trailing": ("rule.weasel_words.very", "very\u0345", 1000),
            "weasel_very_u05b0_leading": ("rule.weasel_words.very", "\u05b0very", 1000),
            "weasel_very_u05b0_trailing": ("rule.weasel_words.very", "very\u05b0", 1000),
        }
        self.assertTrue(set(expected).issubset(cases))
        for case_id, (capability, text, max_errors) in expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["capability_id"], capability)
                self.assertEqual(case["expected_oracle_outcome"], "findings")
                self.assertEqual(base64.b64decode(case["stdin_base64"]).decode("utf-8"), text)
                config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
                self.assertEqual(config["max_errors"], max_errors)

    def test_waxed_and_bottleneck_live_case_contracts(self) -> None:
        cases = {case["id"]: case for case in compare.load_manifest()["cases"]}
        expected_ids = {
            "rule.misc.waxed": {
                "waxed_all_64_form_modifier_pairs", "waxed_correct_adjectives_clean",
                "waxed_raw_activation_and_case_boundaries", "waxed_quote_suppression",
                "waxed_max_errors_one", "waxed_pattern_group_truncation", "waxed_unicode_coordinates",
            },
            "rule.mixed_metaphors.bottleneck": {
                "bottleneck_all_seven_phrases", "bottleneck_clean_and_near_boundary",
                "bottleneck_case_duplicates_and_overlap", "bottleneck_quote_suppression",
                "bottleneck_max_errors_one", "bottleneck_max_errors_interleaved_pattern_order", "bottleneck_pattern_group_truncation", "bottleneck_unicode_coordinates",
            },
        }
        by_capability = {
            capability: {case_id for case_id, case in cases.items() if case["capability_id"] == capability}
            for capability in expected_ids
        }
        self.assertEqual(by_capability, expected_ids)
        for case_id, case in cases.items():
            if case["capability_id"] not in expected_ids:
                continue
            with self.subTest(case=case_id):
                self.assertEqual(case["input_mode"], "stdin")
                self.assertEqual(case["argv"], ["check", "--config", "{file:config.json}", "--output-format", "json"])
                config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
                self.assertEqual(config["checks"], {case["capability_id"].removeprefix("rule.").split(".")[0]: {case["capability_id"].removeprefix("rule.").split(".")[1]: True}})
                self.assertIn("Pinned Proselint 0.16.0", case["provenance"])
                self.assertEqual(config["max_errors"], 1 if case_id.endswith("max_errors_one") or case_id.endswith("max_errors_interleaved_pattern_order") else 2 if case_id.endswith("pattern_group_truncation") else 1000)

        wax_forms = ("wax", "waxes", "waxed", "waxing")
        modifiers = ("ebulliently", "ecstatically", "eloquently", "enthusiastically", "euphorically", "indignantly", "lyrically", "melancholically", "metaphorically", "nostalgically", "patriotically", "philosophically", "poetically", "rhapsodically", "romantically", "sentimentally")
        adjectives = ("ebullient", "ecstatic", "eloquent", "enthusiastic", "euphoric", "indignant", "lyrical", "melancholic", "metaphorical", "nostalgic", "patriotic", "philosophical", "poetic", "rhapsodic", "romantic", "sentimental")
        wax_input = "; ".join(f"{wax} {modifier}" for wax in wax_forms for modifier in modifiers)
        self.assertEqual(base64.b64decode(cases["waxed_all_64_form_modifier_pairs"]["stdin_base64"]).decode(), wax_input)
        wax_message = "The modifier following 'waxed' must be an adj.: '{}' is correct."
        expected_wax = [(wax_message.format(f"{wax} {adj}"), f"{wax} {adj}") for wax in wax_forms for adj in adjectives]
        self.assertEqual({case_id: cases[case_id]["expected_oracle_outcome"] for case_id in expected_ids["rule.misc.waxed"]}, {
            "waxed_all_64_form_modifier_pairs": "findings", "waxed_correct_adjectives_clean": "clean",
            "waxed_raw_activation_and_case_boundaries": "findings", "waxed_quote_suppression": "clean",
            "waxed_max_errors_one": "findings", "waxed_pattern_group_truncation": "findings",
            "waxed_unicode_coordinates": "findings",
        })
        raw_text = base64.b64decode(cases["waxed_raw_activation_and_case_boundaries"]["stdin_base64"]).decode()
        expected_raw = [(wax_message.format("waxed lyrical"), "waxed lyrical"), (wax_message.format("waxing enthusiastic"), "waxing enthusiastic")]
        self.assertEqual([(re.search(re.escape(source), raw_text, re.IGNORECASE).start() + 1, re.search(re.escape(source), raw_text, re.IGNORECASE).end() + 1) for source in ("WAXED LYRICALLY", "waxing enthusiastically")], [(1, 16), (68, 91)])
        unicode_text = base64.b64decode(cases["waxed_unicode_coordinates"]["stdin_base64"]).decode()
        self.assertEqual((unicode_text.index("Waxed"), len(unicode_text)), (3, 25))

        bottleneck_input = "; ".join(("biggest bottleneck", "big bottleneck", "large bottleneck", "largest bottleneck", "world-wide bottleneck", "huge bottleneck", "massive bottleneck"))
        self.assertEqual(base64.b64decode(cases["bottleneck_all_seven_phrases"]["stdin_base64"]).decode(), bottleneck_input)
        message = "Mixed metaphor — bottles with big necks are easy to pass through."
        expected_spans = []
        cursor = 0
        for phrase in ("biggest bottleneck", "big bottleneck", "large bottleneck", "largest bottleneck", "world-wide bottleneck", "huge bottleneck", "massive bottleneck"):
            start = bottleneck_input.index(phrase, cursor)
            expected_spans.append([start + 1, start + len(phrase) + 1])
            cursor = start + len(phrase)
        self.assertEqual(expected_spans, [[1, 19], [21, 35], [37, 53], [55, 73], [75, 96], [98, 113], [115, 133]])
        self.assertEqual(cases["bottleneck_all_seven_phrases"]["expected_oracle_outcome"], "findings")
        self.assertEqual(cases["bottleneck_clean_and_near_boundary"]["expected_oracle_outcome"], "clean")
        self.assertEqual(cases["bottleneck_quote_suppression"]["expected_oracle_outcome"], "clean")
        self.assertEqual(cases["waxed_correct_adjectives_clean"]["expected_oracle_outcome"], "clean")
        self.assertEqual(cases["waxed_quote_suppression"]["expected_oracle_outcome"], "clean")
        self.assertEqual(cases["waxed_max_errors_one"]["expected_oracle_outcome"], "findings")
        self.assertEqual(cases["bottleneck_max_errors_one"]["expected_oracle_outcome"], "findings")

    def test_interleaved_max_errors_pattern_group_truncation_matches_oracle(self) -> None:
        cases = {case["id"]: case for case in compare.load_manifest()["cases"]}
        expected = {
            "bottleneck_max_errors_interleaved_pattern_order": (
                "massive bottleneck; biggest bottleneck", 1,
                [{"check_path": "mixed_metaphors.bottleneck", "message": "Mixed metaphor — bottles with big necks are easy to pass through.", "pos": [1, 1], "replacements": None, "span": [1, 19]}],
            ),
            "waxed_pattern_group_truncation": (
                "waxed lyrically; waxed ecstatically; waxed lyrically", 2,
                [{"check_path": "misc.waxed", "message": "The modifier following 'waxed' must be an adj.: 'waxed lyrical' is correct.", "pos": [1, 1], "replacements": "waxed lyrical", "span": [1, 16]}, {"check_path": "misc.waxed", "message": "The modifier following 'waxed' must be an adj.: 'waxed lyrical' is correct.", "pos": [1, 38], "replacements": "waxed lyrical", "span": [38, 53]}],
            ),
            "bottleneck_pattern_group_truncation": (
                "big bottleneck; massive bottleneck; big bottleneck", 2,
                [{"check_path": "mixed_metaphors.bottleneck", "message": "Mixed metaphor — bottles with big necks are easy to pass through.", "pos": [1, 1], "replacements": None, "span": [1, 15]}, {"check_path": "mixed_metaphors.bottleneck", "message": "Mixed metaphor — bottles with big necks are easy to pass through.", "pos": [1, 37], "replacements": None, "span": [37, 51]}],
            ),
        }
        with tempfile.TemporaryDirectory() as directory:
            reference = compare.default_reference_argv()
            for case_id, (text, limit, expected_diagnostics) in expected.items():
                with self.subTest(case=case_id):
                    case = cases[case_id]
                    self.assertEqual(base64.b64decode(case["stdin_base64"]).decode(), text)
                    config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
                    self.assertEqual(config["max_errors"], limit)
                    result = compare.run_case(case, reference, reference, Path(directory), None)
                    self.assertEqual(result["status"], "PASS", result)
                    output = json.loads(Path(result["executions"][0]["stdout_path"]).read_text(encoding="utf-8"))
                    self.assertEqual(output["result"]["<stdin>"]["diagnostics"], expected_diagnostics)

    def test_mixed_metaphors_misc_cases_match_pinned_oracle(self) -> None:
        cases = {case["id"]: case for case in compare.load_manifest()["cases"]}
        cases_expected = {
            "mixed_misc_all_five_items_and_regex_alternatives": (
                "cream rises to the crop; button your seatbelts; a minute to decompose; sharpest marble in the shed; sharpest marble in the box; not rocket surgery.",
                1000,
                [
                    ("cream rises to the top", [1, 24]),
                    ("fasten your seatbelts", [26, 47]),
                    ("a minute to decompress", [49, 70]),
                    (None, [72, 99]),
                    (None, [101, 127]),
                    ("not rocket science", [129, 147]),
                ],
            ),
            "mixed_misc_correct_forms_and_near_misses": (
                "cream rises to the top; fasten your seatbelts; a minute to decompress; sharpest tool in the shed; sharpest marble in the barn; not rocket science; not rocket surgeon; rocket surgery.",
                1000,
                [],
            ),
            "mixed_misc_case_and_duplicate_occurrences": (
                "CREAM rises to the crop; not rocket surgery; NOT ROCKET SURGERY; a minute to decompose.",
                1000,
                [
                    ("cream rises to the top", [1, 24]),
                    ("not rocket science", [26, 44]),
                    ("not rocket science", [46, 64]),
                    ("a minute to decompress", [66, 87]),
                ],
            ),
            "mixed_misc_quote_suppression": (
                'They wrote "not rocket surgery" and \'cream rises to the crop\'.',
                1000,
                [],
            ),
            "mixed_misc_unicode_coordinates": (
                "é😀 cream rises to the crop.",
                1000,
                [("cream rises to the top", [4, 27])],
            ),
            "mixed_misc_max_errors_interleaved_pattern_groups": (
                "not rocket surgery; cream rises to the crop; not rocket surgery",
                2,
                [("not rocket science", [1, 19]), ("not rocket science", [46, 64])],
            ),
        }
        self.assertEqual(
            {case_id for case_id, case in cases.items() if case["capability_id"] == "rule.mixed_metaphors.misc"},
            set(cases_expected),
        )
        with tempfile.TemporaryDirectory() as directory:
            reference = compare.default_reference_argv()
            for case_id, (input_text, limit, expected) in cases_expected.items():
                with self.subTest(case=case_id):
                    case = cases[case_id]
                    self.assertEqual(base64.b64decode(case["stdin_base64"]).decode(), input_text)
                    config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
                    self.assertEqual(config["max_errors"], limit)
                    result = compare.run_case(case, reference, reference, Path(directory), None)
                    self.assertEqual(result["status"], "PASS", result)
                    output = json.loads(Path(result["executions"][0]["stdout_path"]).read_text(encoding="utf-8"))
                    actual = output["result"]["<stdin>"]["diagnostics"]
                    expected_diagnostics = [
                        {
                            "check_path": "mixed_metaphors.misc",
                            "message": "Mixed metaphor. Try 'None'." if replacement is None else f"Mixed metaphor. Try '{replacement}'.",
                            "pos": [1, span[0]],
                            "replacements": replacement,
                            "span": span,
                        }
                        for replacement, span in expected
                    ]
                    self.assertEqual(actual, expected_diagnostics)

    def test_ras_syndrome_cases_match_pinned_oracle(self) -> None:
        cases = {case["id"]: case for case in compare.load_manifest()["cases"]}
        all_twenty = "abm missile; act test; abm missiles; abs braking system; atm machine; cd disc; cpi Index; gps system; gui interface; hiv virus; isbn number; lcd display; pdf format; pin number; ras syndrome; rip in peace; please RSVP; salT talks; sat test; upc codes."
        outcomes = {
            "ras_syndrome_all_twenty_mappings": (
                all_twenty, 1000,
                [
                    ("abm missile", "ABM", [1, 12]),
                    ("act test", "ACT", [14, 22]),
                    ("abm missile", "ABM", [24, 35]),
                    ("abm missiles", "ABMs", [24, 36]),
                    ("abs braking system", "ABS", [38, 56]),
                    ("atm machine", "ATM", [58, 69]),
                    ("cd disc", "CD", [71, 78]),
                    ("cpi Index", None, [80, 89]),
                    ("gps system", "GPS", [91, 101]),
                    ("gui interface", "GUI", [103, 116]),
                    ("hiv virus", "HIV", [118, 127]),
                    ("isbn number", "ISBN", [129, 140]),
                    ("lcd display", "LCD", [142, 153]),
                    ("pdf format", "PDF", [155, 165]),
                    ("pin number", "PIN", [167, 177]),
                    ("ras syndrome", "RAS", [179, 191]),
                    ("rip in peace", "RIP", [193, 205]),
                    ("please RSVP", None, [207, 218]),
                    ("salT talks", None, [220, 230]),
                    ("sat test", "SAT", [232, 240]),
                    ("upc codes", "UPC", [242, 251]),
                ],
            ),
            "ras_syndrome_correct_forms_clean": (
                "ABM; ACT; ABMs; ABS; ATM; CD; CPI; GPS; GUI; HIV; ISBN; LCD; PDF; PIN; RAS; RIP; RSVP; SALT; SAT; UPC.",
                1000, [],
            ),
            "ras_syndrome_case_and_duplicate_occurrences": (
                "ABM missile; abm missile; PIN number; pin NUMBER.", 1000,
                [("ABM missile", "ABM", [1, 12]), ("abm missile", "ABM", [14, 25]), ("PIN number", "PIN", [27, 37]), ("pin NUMBER", "PIN", [39, 49])],
            ),
            "ras_syndrome_mixed_case_source_key_lookup": (
                "cpi Index; please RSVP; salT talks; CPI INDEX; PLEASE RSVP; SALT TALKS.", 1000,
                [("cpi Index", None, [1, 10]), ("please RSVP", None, [12, 23]), ("salT talks", None, [25, 35]), ("CPI INDEX", None, [37, 46]), ("PLEASE RSVP", None, [48, 59]), ("SALT TALKS", None, [61, 71])],
            ),
            "ras_syndrome_quote_suppression": ('He said "pin number" and \'abm missile\'.', 1000, []),
            "ras_syndrome_unicode_coordinates": ("é😀 pin number.", 1000, [("pin number", "PIN", [4, 14])]),
            "ras_syndrome_interleaved_pattern_group_limit_two": (
                "pin number; atm machine; PIN NUMBER", 2,
                [("pin number", "PIN", [1, 11]), ("PIN NUMBER", "PIN", [26, 36])],
            ),
            "ras_syndrome_plural_overlap_when_only_plural_activates": (
                "abm missiles", 1000, [("abm missiles", "ABMs", [1, 13])],
            ),
        }
        expected_ids = set(outcomes)
        self.assertEqual(
            {case_id for case_id, case in cases.items() if case["capability_id"] == "rule.redundancy.ras_syndrome"},
            expected_ids,
        )
        with tempfile.TemporaryDirectory() as directory:
            reference = compare.default_reference_argv()
            for case_id, (input_text, limit, expected) in outcomes.items():
                with self.subTest(case=case_id):
                    case = cases[case_id]
                    self.assertEqual(base64.b64decode(case["stdin_base64"]).decode(), input_text)
                    config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
                    self.assertEqual(config["max_errors"], limit)
                    result = compare.run_case(case, reference, reference, Path(directory), None)
                    self.assertEqual(result["status"], "PASS", result)
                    output = json.loads(Path(result["executions"][0]["stdout_path"]).read_text(encoding="utf-8"))
                    actual = output["result"]["<stdin>"]["diagnostics"]
                    expected_diagnostics = [
                        {
                            "check_path": "redundancy.ras_syndrome",
                            "message": f"RAS syndrome. Use '{replacement if replacement is not None else 'None'}' instead of '{surface}'.",
                            "pos": [1, span[0]],
                            "replacements": replacement,
                            "span": span,
                        }
                        for surface, replacement, span in expected
                    ]
                    self.assertEqual(actual, expected_diagnostics)

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

    def test_source_fingerprint_tracks_runtime_markdown_asset(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            demo = root / "data" / "demo.md"
            demo.parent.mkdir()
            demo.write_text("initial demo", encoding="utf-8")
            with patch.object(compare, "RUST_ROOT", root):
                initial = compare.source_fingerprint()
                demo.write_text("changed demo", encoding="utf-8")
                self.assertNotEqual(compare.source_fingerprint(), initial)

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
        unset_home = dict(case, environment={"HOME": None, "XDG_CONFIG_HOME": None})
        compare.validate_case(unset_home)
        unset_env = compare.fresh_env(Path("/isolated"), unset_home["environment"])
        self.assertNotIn("HOME", unset_env)
        empty_home = dict(case, environment={"HOME": "", "XDG_CONFIG_HOME": None})
        compare.validate_case(empty_home)
        env = compare.fresh_env(Path("/isolated"), empty_home["environment"])
        self.assertEqual(env["HOME"], "")
        self.assertNotIn("XDG_CONFIG_HOME", env)
        for value in ("/home/dev", "relative-home", []):
            invalid = dict(case, environment={"HOME": value})
            with self.assertRaisesRegex(ValueError, "HOME may only be unset or overridden with an empty string"):
                compare.validate_case(invalid)
        for item in (dict(case["home_files"][0], path="../outside"), dict(case["home_files"][0], content_base64="not-base64")):
            invalid = json.loads(json.dumps(case))
            invalid["home_files"] = [item]
            with self.assertRaises(ValueError):
                compare.validate_case(invalid)
        invalid = json.loads(json.dumps(case))
        invalid["environment"] = {"PATH": None}
        with self.assertRaisesRegex(ValueError, "only HOME and XDG_CONFIG_HOME may be unset"):
            compare.validate_case(invalid)
        invalid = json.loads(json.dumps(case))
        invalid["environment"] = {"PYTHONPYCACHEPREFIX": "/tmp/shared-pycache"}
        with self.assertRaisesRegex(ValueError, "reserved for oracle isolation"):
            compare.validate_case(invalid)

    def test_fixture_mode_applies_to_shared_workspace_and_rejects_invalid_values(self) -> None:
        case = next(case for case in compare.load_manifest()["cases"] if case["id"] == "cli_user_discovery_xdg_unreadable_file")
        compare.validate_case(case)
        with tempfile.TemporaryDirectory() as temporary:
            shared_workspace = Path(temporary) / "shared"
            compare.materialize_files(case, shared_workspace)
            reference_config = shared_workspace / case["files"][0]["path"]
            candidate_config = shared_workspace / case["files"][0]["path"]
            self.assertEqual(reference_config.resolve(), candidate_config.resolve())
            self.assertEqual(reference_config.stat().st_mode & 0o777, 0)
            archived_workspace = Path(temporary) / "archive"
            compare.copy_workspace_layout(case, shared_workspace, archived_workspace)
            self.assertEqual(reference_config.stat().st_mode & 0o777, 0)
            self.assertEqual((archived_workspace / case["files"][0]["path"]).stat().st_mode & 0o777, 0)
        case["files"][0]["mode"] = True
        with self.assertRaisesRegex(ValueError, "mode must be an integer from 0 through 511"):
            compare.validate_case(case)

    def test_checks_scalar_dump_config_registry_and_discriminators(self) -> None:
        scalar_values = {"null": None, "list": [], "string": "invalid", "bool": False, "int": 7, "float": 1.5}
        expected = {
            f"cli_config_checks_invalid_{kind}_dump_config_{fmt}"
            for kind in scalar_values for fmt in ("json", "full", "compact")
        }
        expected.update(
            f"cli_config_checks_invalid_{kind}_version_{fmt}"
            for kind in scalar_values for fmt in ("json", "full", "compact")
        )
        expected.update(f"cli_config_checks_valid_mapping_dump_config_{fmt}" for fmt in ("json", "full", "compact"))
        expected.add("cli_config_checks_list_per_file_null_version_json")
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "cli.config.consumed_special_values"]
        focused = [case for case in cases if case["id"] in expected]
        self.assertEqual(len(focused), 40)
        self.assertEqual({case["id"] for case in focused}, expected)
        for case in focused:
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            fmt = case["output_format"]
            self.assertEqual(case["argv"][-1], fmt)
            self.assertEqual(case["comparison"], {"stdout": "json" if fmt == "json" and case["expected_oracle_outcome"] == "cli_error" else "bytes", "stderr": "bytes", "exit_status": "exact"})
            if case["id"].startswith("cli_config_checks_invalid_"):
                kind = case["id"].split("_invalid_", 1)[1].split("_", 1)[0]
                self.assertEqual(config["checks"], {"null": None, "list": [], "string": "invalid", "bool": False, "int": 7, "float": 1.5}[kind])
                self.assertEqual(case["expected_oracle_outcome"], "cli_error")
            elif case["id"] == "cli_config_checks_list_per_file_null_version_json":
                self.assertEqual(config, {"checks": [], "per_file_checks": None})
                self.assertEqual(case["argv"], ["dump-config", "--config", "{file:config.json}", "--version", "--output-format", "json"])
                self.assertEqual(case["expected_oracle_outcome"], "cli_error")
            else:
                self.assertEqual(config, {"checks": {}})
                self.assertEqual(case["expected_oracle_outcome"], "clean")
            compare.validate_case(case)
        combined = next(item for item in focused if item["id"] == "cli_config_checks_list_per_file_null_version_json")
        self.assertEqual(json.loads(base64.b64decode(combined["files"][0]["content_base64"])), {"checks": [], "per_file_checks": None})
        self.assertEqual(combined["argv"], ["dump-config", "--config", "{file:config.json}", "--version", "--output-format", "json"])
        self.assertEqual(combined["comparison"], {"stdout": "json", "stderr": "bytes", "exit_status": "exact"})
        self.assertEqual(combined["expected_oracle_outcome"], "cli_error")
        compare.validate_case(combined)

    def test_per_file_file_config_scalar_registry_and_discriminators(self) -> None:
        values = {"null": None, "list": [], "string": "invalid", "bool": False, "int": 7, "float": 1.5}
        expected = {
            f"cli_per_file_file_config_invalid_{kind}_{scope}_{fmt}"
            for kind in values for scope in ("check", "dump_config", "version")
            for fmt in ("json", "full", "compact")
        }
        expected.update(f"cli_per_file_file_config_valid_mapping_dump_config_{fmt}" for fmt in ("json", "full", "compact"))
        expected.update({
            "cli_per_file_file_config_nested_invalid_leaf",
            "cli_per_file_file_config_duplicate_invalid_then_valid",
            "cli_per_file_file_config_duplicate_valid_then_invalid",
            "cli_per_file_checks_error_precedes_per_file",
        })
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "cli.config.per_file_checks"]
        focused = [case for case in cases if case["id"] in expected]
        self.assertEqual(len(focused), 61)
        self.assertEqual({case["id"] for case in focused}, expected)
        for kind, scalar in values.items():
            config_value = {"checks": {}, "per_file_checks": {"*.zzz": scalar}}
            for fmt in ("json", "full", "compact"):
                for scope in ("check", "dump_config", "version"):
                    case = next(item for item in focused if item["id"] == f"cli_per_file_file_config_invalid_{kind}_{scope}_{fmt}")
                    self.assertEqual(json.loads(base64.b64decode(case["files"][0]["content_base64"])), config_value)
                    argv = (["check", "--config", "{file:config.json}", "--output-format", fmt, "{file:input.txt}"]
                            if scope == "check" else ["dump-config", "--config", "{file:config.json}"])
                    if scope == "version": argv.append("--version")
                    if scope != "check": argv.extend(["--output-format", fmt])
                    self.assertEqual(case["argv"], argv)
                    self.assertEqual(case["comparison"], {"stdout": "json" if fmt == "json" else "bytes", "stderr": "bytes", "exit_status": "exact"})
                    self.assertEqual(case["expected_oracle_outcome"], "cli_error")
                    compare.validate_case(case)
        for fmt in ("json", "full", "compact"):
            valid = next(item for item in focused if item["id"] == f"cli_per_file_file_config_valid_mapping_dump_config_{fmt}")
            self.assertEqual(json.loads(base64.b64decode(valid["files"][0]["content_base64"])), {"checks": {}, "per_file_checks": {"*.zzz": {"misc": {"apologizing": True}}}})
            self.assertEqual(valid["expected_oracle_outcome"], "clean")
            compare.validate_case(valid)
        nested = next(item for item in focused if item["id"] == "cli_per_file_file_config_nested_invalid_leaf")
        self.assertEqual(json.loads(base64.b64decode(nested["files"][0]["content_base64"])), {"checks": {}, "per_file_checks": {"*.txt": {"misc": {"apologizing": None}}}})
        self.assertEqual(nested["expected_oracle_outcome"], "cli_error")
        compare.validate_case(nested)
        duplicates = {
            "cli_per_file_file_config_duplicate_invalid_then_valid": b'{"checks":{},"per_file_checks":{"*.txt":null,"*.txt":{"misc":{"apologizing":true}}}}',
            "cli_per_file_file_config_duplicate_valid_then_invalid": b'{"checks":{},"per_file_checks":{"*.txt":{"misc":{"apologizing":true}},"*.txt":null}}',
        }
        for case_id, raw in duplicates.items():
            case = next(item for item in focused if item["id"] == case_id)
            self.assertEqual(base64.b64decode(case["files"][0]["content_base64"]), raw)
            compare.validate_case(case)
        precedence = next(item for item in focused if item["id"] == "cli_per_file_checks_error_precedes_per_file")
        self.assertEqual(json.loads(base64.b64decode(precedence["files"][0]["content_base64"])), {"checks": [], "per_file_checks": {"*.txt": None}})
        self.assertEqual(precedence["expected_oracle_outcome"], "cli_error")
        compare.validate_case(precedence)

    def test_per_file_checks_null_registry(self) -> None:
        expected = {
            "per_file_checks_glob_disables_global_annotation",
            "per_file_parent_group_specific_nested_precedence",
            "per_file_bracket_character_class",
            "per_file_overlap_insertion_order_precedence",
            "per_file_checks_enable_globally_disabled_annotation",
            "per_file_checks_override_merges_global_checks",
            "per_file_checks_nonmatching_pattern_uses_global",
            "per_file_checks_null_error",
            "per_file_checks_null_error_full",
            "per_file_checks_null_error_compact",
            "per_file_checks_null_dump_config_json",
            "per_file_checks_null_dump_config_full",
            "per_file_checks_null_dump_config_compact",
            "per_file_checks_null_dump_config_version_json",
            "per_file_checks_null_dump_config_version_full",
            "per_file_checks_null_dump_config_version_compact",
        }
        scalar_types = {"list", "string", "bool", "int", "float"}
        expected.update(f"per_file_checks_invalid_{kind}_dump_config_{fmt}" for kind in scalar_types for fmt in ("json", "full", "compact"))
        expected.update(f"per_file_checks_invalid_{kind}_version_json" for kind in scalar_types)
        expected.add("per_file_checks_valid_empty_dump_config_json")
        file_config_types = {"null", "list", "string", "bool", "int", "float"}
        expected.update(f"cli_per_file_file_config_invalid_{kind}_{scope}_{fmt}" for kind in file_config_types for scope in ("check", "dump_config", "version") for fmt in ("json", "full", "compact"))
        expected.update(f"cli_per_file_file_config_valid_mapping_dump_config_{fmt}" for fmt in ("json", "full", "compact"))
        expected.update({"cli_per_file_file_config_nested_invalid_leaf", "cli_per_file_file_config_duplicate_invalid_then_valid", "cli_per_file_file_config_duplicate_valid_then_invalid", "cli_per_file_checks_error_precedes_per_file"})
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "cli.config.per_file_checks"]
        self.assertEqual(len(cases), 98)
        self.assertEqual({case["id"] for case in cases}, expected)
        case = next(case for case in cases if case["id"] == "per_file_checks_null_error")
        self.assertEqual(json.loads(base64.b64decode(case["files"][0]["content_base64"])), {"checks": {}, "per_file_checks": None})
        self.assertEqual(case["expected_oracle_outcome"], "cli_error")
        self.assertEqual(case["comparison"], {"stdout": "json", "stderr": "bytes", "exit_status": "exact"})
        compare.validate_case(case)
        for output_format in ("full", "compact"):
            formatted = next(item for item in cases if item["id"] == f"per_file_checks_null_error_{output_format}")
            self.assertEqual(json.loads(base64.b64decode(formatted["files"][0]["content_base64"])), {"checks": {}, "per_file_checks": None})
            self.assertEqual(formatted["argv"], ["check", "--config", "{file:config.json}", "--output-format", output_format])
            self.assertEqual(formatted["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})
            self.assertEqual(formatted["expected_oracle_outcome"], "cli_error")
            compare.validate_case(formatted)
        for output_format in ("json", "full", "compact"):
            dumped = next(item for item in cases if item["id"] == f"per_file_checks_null_dump_config_{output_format}")
            self.assertEqual(json.loads(base64.b64decode(dumped["files"][0]["content_base64"])), {"checks": {}, "per_file_checks": None})
            self.assertEqual(dumped["argv"], ["dump-config", "--output-format", output_format, "--config", "{file:config.json}"])
            self.assertEqual(dumped["comparison"], {"stdout": "json" if output_format == "json" else "bytes", "stderr": "bytes", "exit_status": "exact"})
            self.assertEqual(dumped["expected_oracle_outcome"], "cli_error")
            compare.validate_case(dumped)
        for output_format in ("json", "full", "compact"):
            versioned = next(item for item in cases if item["id"] == f"per_file_checks_null_dump_config_version_{output_format}")
            self.assertEqual(json.loads(base64.b64decode(versioned["files"][0]["content_base64"])), {"checks": {}, "per_file_checks": None})
            self.assertEqual(versioned["argv"], ["dump-config", "--config", "{file:config.json}", "--version", "--output-format", output_format])
            self.assertEqual(versioned["comparison"], {"stdout": "json" if output_format == "json" else "bytes", "stderr": "bytes", "exit_status": "exact"})
            self.assertEqual(versioned["expected_oracle_outcome"], "cli_error")
            compare.validate_case(versioned)
        scalar_values = {"list": [], "string": "invalid", "bool": False, "int": 7, "float": 1.5}
        for kind, scalar in scalar_values.items():
            for output_format in ("json", "full", "compact"):
                invalid = next(item for item in cases if item["id"] == f"per_file_checks_invalid_{kind}_dump_config_{output_format}")
                self.assertEqual(json.loads(base64.b64decode(invalid["files"][0]["content_base64"])), {"checks": {}, "per_file_checks": scalar})
                self.assertEqual(invalid["argv"], ["dump-config", "--config", "{file:config.json}", "--output-format", output_format])
                self.assertEqual(invalid["comparison"], {"stdout": "json" if output_format == "json" else "bytes", "stderr": "bytes", "exit_status": "exact"})
                self.assertEqual(invalid["expected_oracle_outcome"], "cli_error")
                compare.validate_case(invalid)
            versioned = next(item for item in cases if item["id"] == f"per_file_checks_invalid_{kind}_version_json")
            self.assertEqual(json.loads(base64.b64decode(versioned["files"][0]["content_base64"])), {"checks": {}, "per_file_checks": scalar})
            self.assertEqual(versioned["argv"], ["dump-config", "--config", "{file:config.json}", "--version", "--output-format", "json"])
            self.assertEqual(versioned["expected_oracle_outcome"], "cli_error")
            compare.validate_case(versioned)
        valid = next(item for item in cases if item["id"] == "per_file_checks_valid_empty_dump_config_json")
        self.assertEqual(json.loads(base64.b64decode(valid["files"][0]["content_base64"])), {"checks": {}, "per_file_checks": {}})
        self.assertEqual(valid["expected_oracle_outcome"], "clean")
        self.assertEqual(valid["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})
        compare.validate_case(valid)

    def test_user_discovery_xdg_unreadable_file_fixture(self) -> None:
        case = next(case for case in compare.load_manifest()["cases"] if case["id"] == "cli_user_discovery_xdg_unreadable_file")
        self.assertEqual(case["capability_id"], "cli.config.user_discovery")
        self.assertEqual(case["files"], [{"path": "xdg/proselint/config.json", "content_base64": "e30=", "mode": 0}])
        self.assertEqual(case["unused_files"], ["xdg/proselint/config.json"])
        self.assertEqual(case["environment"]["XDG_CONFIG_HOME"], "xdg")
        self.assertFalse(case["workspace_mutates"])
        self.assertEqual(case["expected_oracle_outcome"], "cli_error")
        compare.validate_case(case)
        for output_format in ("full", "compact"):
            formatted = next(item for item in compare.load_manifest()["cases"]
                             if item["id"] == f"cli_user_discovery_xdg_unreadable_{output_format}")
            self.assertEqual(formatted["capability_id"], case["capability_id"])
            self.assertEqual(formatted["files"], case["files"])
            self.assertEqual(formatted["environment"], case["environment"])
            self.assertEqual(formatted["argv"], ["check", "--output-format", output_format])
            self.assertEqual(formatted["output_format"], output_format)
            self.assertEqual(formatted["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})
            self.assertEqual(formatted["expected_oracle_outcome"], "cli_error")
            compare.validate_case(formatted)

    def test_user_discovery_skips_xdg_directory_candidate_without_home_fallback(self) -> None:
        case = next(
            case for case in compare.load_manifest()["cases"]
            if case["id"] == "cli_user_discovery_xdg_directory_does_not_fall_back_to_home"
        )
        self.assertEqual(case["capability_id"], "cli.config.user_discovery")
        self.assertEqual(case["stdin_base64"], "T3VyIHBvbGljeSBzdGF0ZXM6IE1vcmUgcmVzZWFyY2ggaXMgbmVlZGVkLg==")
        self.assertEqual(case["files"], [{"path": "xdg/proselint/config.json/marker", "content_base64": "eA=="}])
        self.assertEqual(case["unused_files"], ["xdg/proselint/config.json/marker"])
        self.assertEqual(case["home_files"], [{"path": ".config/proselint/config.json", "content_base64": "eyJjaGVja3MiOnt9fQ=="}])
        self.assertEqual(case["environment"]["XDG_CONFIG_HOME"], "xdg")
        self.assertEqual(case["argv"], ["check", "--output-format", "json"])
        self.assertEqual(case["expected_oracle_outcome"], "findings")
        self.assertEqual(case["comparison"], {"stdout": "json", "stderr": "bytes", "exit_status": "exact"})
        compare.validate_case(case)

    def test_empty_home_fallback_case_marks_passwd_home_precondition(self) -> None:
        case = next(case for case in compare.load_manifest()["cases"] if case["id"] == "cli_config_empty_home_fallback")
        self.assertEqual(case["capability_id"], "cli.config.empty_home_fallback")
        self.assertEqual(case["environment"], {"locale": "C.UTF-8", "HOME": "", "XDG_CONFIG_HOME": None})
        self.assertEqual(case["files"][0]["path"], ".config/proselint/config.json")
        self.assertEqual(case["comparison"]["stdout"], "bytes")
        self.assertIn("/home/dev/.config/proselint/config.json must be absent", case["preconditions"][0])

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
        mismatched_policy["comparison"]["stdout"] = "xml"
        with self.assertRaisesRegex(ValueError, "comparison.stdout"):
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

        missing_cli_error_value = json.loads(json.dumps(default_full))
        missing_cli_error_value["argv"] = ["check", "--config", "{file:" + fixture_path + "}", "--output-format"]
        missing_cli_error_value["expected_oracle_outcome"] = "cli_error"
        compare.validate_case(missing_cli_error_value)

        missing_alias_value = json.loads(json.dumps(missing_cli_error_value))
        missing_alias_value["argv"][-1] = "-o"
        compare.validate_case(missing_alias_value)

        missing_non_error_value = json.loads(json.dumps(missing_cli_error_value))
        missing_non_error_value["expected_oracle_outcome"] = "clean"
        with self.assertRaisesRegex(ValueError, "may omit its value only for a CLI error"):
            compare.validate_case(missing_non_error_value)

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
                "version_json_schema", "version_json_missing_python", "version_json_extra_field",
                "version_json_wrong_python_type", "version_json_wrong_version", "version_json_wrong_python_value",
            }.issubset(names)
        )

    def test_write_good_and_max_errors_live_case_contracts(self) -> None:
        cases = {case["id"]: case for case in compare.load_manifest()["cases"]}
        special = {
            "null": ("null", "TODO TODO", "findings"),
            "true": ("true", "TODO TODO", "findings"),
            "string": ('"2"', "TODO", "clean"),
            "array": ("[2]", "TODO", "clean"),
            "object": ('{"x":2}', "TODO", "clean"),
            "negative_integer": ("-1", "TODO", "clean"),
            "fractional_number": ("1.5", "TODO", "clean"),
        }
        for label, (value, text, outcome) in special.items():
            case = cases[f"cli_config_consumed_max_errors_{label}"]
            self.assertEqual(case["capability_id"], "cli.config.consumed_special_values")
            self.assertEqual(base64.b64decode(case["stdin_base64"]).decode(), text)
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["max_errors"], json.loads(value))
            self.assertEqual(config["checks"], {"annotations": {"misc": True}})
            self.assertEqual(case["expected_oracle_outcome"], outcome)

        write_good = [case for case in cases.values() if case["capability_id"] == "rule.cliches.misc.write_good"]
        self.assertEqual(len(write_good), 9)
        corpus = Path(compare.RUST_ROOT / "data/third_party/no_cliches_write_good.txt").read_bytes()
        all_items = cases["cliches_write_good_all_items"]
        self.assertEqual(base64.b64decode(all_items["stdin_base64"]), corpus)
        self.assertEqual(corpus.count(b"\n"), 697)
        for case in write_good:
            config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
            self.assertEqual(config["checks"], {"cliches": {"misc": {"write_good": True}}})
            self.assertEqual(case["argv"], ["check", "--config", "{file:config.json}", "--output-format", "json"])
            self.assertEqual(case["comparison"], {"stdout": "json", "stderr": "bytes", "exit_status": "exact"})
        self.assertEqual(cases["cliches_write_good_package_only_like_the_plague"]["expected_oracle_outcome"], "clean")
        regex_optional_e = cases["cliches_write_good_regex_optional_final_e"]
        self.assertEqual(base64.b64decode(regex_optional_e["stdin_base64"]), b"will wonders never ceas")
        self.assertEqual(regex_optional_e["expected_oracle_outcome"], "findings")

    def test_cli_stdin_invalid_utf8_registry(self) -> None:
        expected = {
            "fast_apologizing_before_ff", "fast_apologizing_inside_ff", "fast_apologizing_after_ff",
            "fancy_lexical_illusions_before_ff", "fancy_lexical_illusions_inside_ff", "fancy_lexical_illusions_after_ff",
            "clean_control_ufffd", "clean_control_private_use_e0ff", "clean_control_crlf",
            "fancy_lexical_illusions_multibyte_prefix_ff", "fast_apologizing_consecutive_invalid_bytes",
            "mixed_invalid_utf8_max_errors_zero", "mixed_invalid_utf8_max_errors_one",
        }
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "cli.stdin_invalid_utf8"]
        self.assertEqual(len(cases), 13)
        self.assertEqual({case["id"] for case in cases}, expected)

    def test_surrogate_key_equality_cases_preserve_live_python_contract(self) -> None:
        expected = {
            "surrogate_check_key_high_false": ('{"checks":{"annotations":{"\\ud800":false}}}', "clean"),
            "surrogate_check_key_low_false": ('{"checks":{"annotations":{"\\udfff":false}}}', "clean"),
            "surrogate_duplicate_key_high_invalid_then_false": ('{"checks":{"annotations":{"\\ud800":"invalid","\\uD800":false}}}', "clean"),
            "surrogate_duplicate_key_high_false_then_invalid": ('{"checks":{"annotations":{"\\uD800":false,"\\ud800":"invalid"}}}', "cli_error"),
            "surrogate_duplicate_astral_pair_literal_invalid_then_false": ('{"checks":{"annotations":{"\\ud83d\\ude00":"invalid","😀":false}}}', "clean"),
        }
        cases = {
            case["id"]: case for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.config.surrogate_key_equality"
        }
        self.assertEqual(set(cases), set(expected))
        for case_id, (config, outcome) in expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(base64.b64decode(case["files"][0]["content_base64"]).decode(), config)
                self.assertEqual(case["argv"], ["check", "--config", "{file:config.json}", "--output-format", "json"])
                self.assertEqual(case["expected_oracle_outcome"], outcome)
                self.assertEqual(case["comparison"], {"stdout": "json", "stderr": "bytes", "exit_status": "exact"})
                self.assertIn("Pinned Proselint 0.16.0 live CLI", case["provenance"])

    def test_real_manifest_schema_and_unique_ids(self) -> None:
        manifest = compare.load_manifest()
        self.assertEqual(manifest["schema_version"], 1)
        case_ids = [case["id"] for case in manifest["cases"]]
        self.assertEqual(len(case_ids), 1436)
        self.assertEqual(len(set(case_ids)), len(case_ids))
        self.assertEqual(
            hashlib.sha256("".join(
                f"{case_id}\0{capability_id}\n"
                for case_id, capability_id in sorted(
                    (case["id"], case["capability_id"]) for case in manifest["cases"]
                )
            ).encode()).hexdigest(),
            "2f5c79c308691fa1fa5ea2fa9af0acf4bceee69214c0b0c23454a0a38f6d6b26",
        )
        self.assertEqual(
            sum(case["capability_id"] == "cli.config.no_config_default" for case in manifest["cases"]),
            3,
        )
        self.assertEqual(
            sum(case["capability_id"] == "cli.config.missing_explicit_text_error" for case in manifest["cases"]),
            2,
        )
        archaism = [case for case in manifest["cases"] if case["capability_id"] == "rule.archaism"]
        archaism_by_id = {case["id"]: case for case in archaism}
        probe_ids = [
            "archaism_different_length_activation_order",
            "archaism_different_length_activation_order_max_errors_one",
        ]
        for case_id in probe_ids:
            case = archaism_by_id[case_id]
            self.assertEqual(case["stdin_base64"], "ZW5jbG9zZWQgcGxlYXNlIGZpbmQgYWxhY2s=")
            self.assertEqual(json.loads(base64.b64decode(case["files"][0]["content_base64"]))["checks"], {"archaism": True})
        self.assertEqual(
            [json.loads(base64.b64decode(archaism_by_id[case_id]["files"][0]["content_base64"]))["max_errors"] for case_id in probe_ids],
            [1000, 1],
        )

    def test_default_config_cross_rule_case_scope(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "cli.config.default_cross_rule"]
        self.assertEqual(len(cases), 1)
        case = cases[0]
        self.assertEqual(case["id"], "default_config_cross_rule_mixed_findings")
        self.assertEqual(case["input_mode"], "stdin")
        self.assertEqual(base64.b64decode(case["stdin_base64"]), b"More research is needed. Suddenly, the very unique result surprised everyone.\n")
        self.assertEqual(case["files"], [])
        self.assertEqual(case["argv"], ["check", "--output-format", "json"])
        self.assertEqual(case["config_scenario"], "no_config_default")
        self.assertEqual(case["comparison"]["stdout"], "json")
        compare.validate_case(case)

    def test_needless_variants_manifest_cases_cover_source_and_runtime_edges(self) -> None:
        cases = {case["id"]: case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.needless_variants"}
        self.assertEqual(set(cases), {
            "needless_variants_all_361_source_pairs", "needless_variants_case_surface_repetition_order",
            "needless_variants_ascii_boundaries", "needless_variants_quote_suppression_control",
            "needless_variants_non_ascii_boundaries", "needless_variants_default_dispatch",
            "needless_variants_max_errors_one_across_batches",
        })
        pairs = Path("proselint/checks/needless-variants").read_text(encoding="utf-8").splitlines()
        self.assertEqual(len(pairs), 361)
        all_pairs = cases["needless_variants_all_361_source_pairs"]
        self.assertEqual(base64.b64decode(all_pairs["files"][1]["content_base64"]).decode(), "; ".join(row.split(",", 1)[0] for row in pairs))
        for case in cases.values():
            self.assertEqual(case["provenance"].split("SHA-256 ", 1)[1].split(".", 1)[0].rstrip(")"), "3148f8a2f3ba09eee6a6a8284a2d709611d2f51b228afab74db37160adae3316")
            compare.validate_case(case)
        self.assertEqual(
            {case_id: case["expected_oracle_outcome"] for case_id, case in cases.items()},
            {case_id: "findings" for case_id in cases},
        )
        self.assertEqual(json.loads(base64.b64decode(cases["needless_variants_max_errors_one_across_batches"]["files"][0]["content_base64"]))["max_errors"], 1)

    def test_uncomparables_live_case_contracts(self) -> None:
        cases = {case["id"]: case for case in compare.load_manifest()["cases"] if case["capability_id"] == "rule.uncomparables"}
        self.assertEqual(set(cases), {
            "uncomparables_all_standard_combinations", "uncomparables_fancy_least_most",
            "uncomparables_fancy_at_least_suppression", "uncomparables_documented_exceptions",
            "uncomparables_case_and_surface", "uncomparables_word_boundaries",
            "uncomparables_quote_behavior", "uncomparables_order_limit_one",
            "uncomparables_default_dispatch", "uncomparables_fast_pattern_id_order_limit_one",
            "uncomparables_fancy_text_order_limit_one",
        })
        expected = {
            "uncomparables_all_standard_combinations": ("findings", 1000), "uncomparables_fancy_least_most": ("findings", 1000),
            "uncomparables_fancy_at_least_suppression": ("clean", 1000), "uncomparables_documented_exceptions": ("clean", 1000),
            "uncomparables_case_and_surface": ("findings", 1000), "uncomparables_word_boundaries": ("findings", 1000),
            "uncomparables_quote_behavior": ("findings", 1000), "uncomparables_order_limit_one": ("findings", 1),
            "uncomparables_default_dispatch": ("findings", None), "uncomparables_fast_pattern_id_order_limit_one": ("findings", 1),
            "uncomparables_fancy_text_order_limit_one": ("findings", 1),
        }
        for case_id, case in cases.items():
            with self.subTest(case=case_id):
                self.assertEqual(case["expected_oracle_outcome"], expected[case_id][0])
                self.assertEqual(case["provenance"].split("SHA-256 ", 1)[1].split(";", 1)[0].rstrip(")"), "e6842c6cf15f2ee47782fa9d48d04ad3a9e5b5f29b6d3c89db6d02cedfd61582")
                self.assertEqual(len(case["files"]), 1 if expected[case_id][1] is None else 2)
                self.assertEqual(case.get("config_scenario"), "isolated_default" if expected[case_id][1] is None else None)
                if expected[case_id][1] is not None:
                    config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
                    self.assertEqual(config["checks"], {"uncomparables": True})
                    self.assertEqual(config["max_errors"], expected[case_id][1])
                compare.validate_case(case)
        first_matches = {
            "uncomparables_fast_pattern_id_order_limit_one": ("very unique; extremely absolute", "very unique"),
            "uncomparables_fancy_text_order_limit_one": ("most unique; least absolute", "most unique"),
        }
        for case_id, (text, first) in first_matches.items():
            case = cases[case_id]
            self.assertEqual(base64.b64decode(case["files"][1]["content_base64"]).decode(), text)
            self.assertIn(first, text)
            self.assertEqual(json.loads(base64.b64decode(case["files"][0]["content_base64"])), {"checks": {"uncomparables": True}, "max_errors": 1})

    def test_restricted_vocabulary_live_case_contracts(self) -> None:
        all_cases = compare.load_manifest()["cases"]
        expected = {
            "elementary": (849, "7e793e661ec22f7f345254a5fbd6e16d4309f94a26673587f068594d681b340e", "7d7b49f39fd8afc4e01ce66b20d8d5d59889d6b7f4e08d3038a0b14b828bf8c0"),
            "top1000": (1000, "d9e05490d10f0426e5e3ce0a720892fbeb6b8c6c6df90432f8fb4b461b5f398f", "84d32b89cfe250e10cb278e923681a8b17e28ebe22ad60e10ab85730ad02a140"),
        }
        for name, (word_count, data_hash, module_hash) in expected.items():
            cases = {case["id"]: case for case in all_cases if case["capability_id"] == f"rule.restricted.{name}"}
            self.assertEqual(set(cases), {f"restricted_{name}_all_data_terms", f"restricted_{name}_boundaries_apostrophes_duplicates", f"restricted_{name}_default_disabled"})
            data = Path(f"proselint/checks/restricted/{name}").read_text(encoding="utf-8").splitlines()
            self.assertEqual(len(data), word_count)
            self.assertEqual(base64.b64decode(cases[f"restricted_{name}_all_data_terms"]["files"][1]["content_base64"]).decode(), " ".join(data))
            for case_id, case in cases.items():
                with self.subTest(case=case_id):
                    self.assertIn(f"module proselint/checks/restricted/{name}.py SHA-256 {module_hash}", case["provenance"])
                    self.assertIn(f"data proselint/checks/restricted/{name} SHA-256 {data_hash}", case["provenance"])
                    compare.validate_case(case)
            for kind in ("all_data_terms", "boundaries_apostrophes_duplicates"):
                case = cases[f"restricted_{name}_{kind}"]
                self.assertEqual(case["expected_oracle_outcome"], "findings")
                self.assertEqual(json.loads(base64.b64decode(case["files"][0]["content_base64"]))["checks"], {"restricted": {name: True}})
            default = cases[f"restricted_{name}_default_disabled"]
            self.assertEqual(default["expected_oracle_outcome"], "clean")
            self.assertEqual(default["config_scenario"], "isolated_default")
            self.assertEqual(base64.b64decode(default["files"][0]["content_base64"]), b"xylophone")
            self.assertNotIn("--config", default["argv"])

    def test_astra_low_error_ordering_regressions_are_interleaved_and_isolated(self) -> None:
        expected = {
            "astra_ordering_capitalization_seasons_limit_two": ("misc.capitalization.seasons", "Winter; Summer; Winter"),
            "astra_ordering_capitalization_seasons_reversed_limit_two": ("misc.capitalization.seasons", "Summer; Summer; Winter"),
            "astra_ordering_capitalization_months_limit_two": ("misc.capitalization.months", "january; february; january"),
            "astra_ordering_capitalization_months_reversed_limit_two": ("misc.capitalization.months", "february; february; january"),
            "astra_ordering_capitalization_days_limit_two": ("misc.capitalization.days", "monday; tuesday; monday"),
            "astra_ordering_capitalization_days_reversed_limit_two": ("misc.capitalization.days", "tuesday; tuesday; monday"),
            "astra_ordering_capitalization_terms_limit_two": ("misc.capitalization.terms", "mother nature; Space Age; mother nature"),
            "astra_ordering_capitalization_terms_reversed_limit_two": ("misc.capitalization.terms", "Space Age; Space Age; mother nature"),
            "astra_ordering_debased_limit_two": ("misc.debased", "leaves much to be desired; would serve no purpose; leaves much to be desired"),
            "astra_ordering_debased_reversed_limit_two": ("misc.debased", "would serve no purpose; would serve no purpose; leaves much to be desired"),
            "astra_ordering_metadiscourse_limit_two": ("misc.metadiscourse", "This chapter discusses; The rest of this article; This chapter discusses"),
            "astra_ordering_metadiscourse_reversed_limit_two": ("misc.metadiscourse", "The rest of this article; The rest of this article; This chapter discusses"),
            "astra_ordering_many_a_limit_two": ("misc.many_a", "are many a; were many a; are many a"),
            "astra_ordering_many_a_reversed_limit_two": ("misc.many_a", "were many a; were many a; are many a"),
            "astra_ordering_greylist_limit_two": ("misc.greylist", "utilize; obviously; obviously"),
        }
        by_id = {case["id"]: case for case in compare.load_manifest()["cases"]}
        self.assertTrue(set(expected).issubset(by_id))
        for case_id, (check_path, stdin) in expected.items():
            with self.subTest(case=case_id):
                case = by_id[case_id]
                self.assertEqual(case["capability_id"], "rule." + check_path)
                self.assertEqual(base64.b64decode(case["stdin_base64"]).decode(), stdin)
                self.assertEqual(len(case["files"]), 1)
                config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
                self.assertEqual(config["max_errors"], 2)
                enabled = []

                def collect(value: object, path: str = "") -> None:
                    if isinstance(value, dict):
                        for key, child in value.items():
                            collect(child, f"{path}.{key}" if path else key)
                    elif value is True:
                        enabled.append(path)

                collect(config["checks"])
                self.assertEqual(enabled, [check_path])
                compare.validate_case(case)

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

    def test_version_json_manifest_declares_upstream_schema(self) -> None:
        cases = [case for case in compare.load_manifest()["cases"] if case["capability_id"] == "cli.version.json_metadata"]
        self.assertEqual(len(cases), 5)
        for case in cases:
            if case["comparison"]["stdout"] == "json":
                self.assertEqual(case["comparison"]["json_schema"], "version")
            else:
                self.assertEqual(case["id"], "cli_version_json_metadata_precedence_control")
                self.assertNotIn("json_schema", case["comparison"])

    def test_version_json_schema_and_exact_values(self) -> None:
        upstream = {"version": "0.16.0", "python": [3, 12, 3]}
        expected = {"exit_code": 0, "stdout": json.dumps(upstream).encode(), "stderr": b""}
        self.assertIsNone(compare.validate_output_schema(upstream, schema="version"))
        for invalid in (
            {"version": "0.16.0"},
            {**upstream, "extra": True},
            {"version": 0.16, "python": [3, 12, 3]},
            {"version": "0.16.0", "python": [3, True, 3]},
            {"version": "0.16.0", "python": [3, 12]},
        ):
            with self.subTest(invalid=invalid):
                self.assertIsNotNone(compare.validate_output_schema(invalid, schema="version"))
        for altered in (
            {"version": "0.16.1", "python": [3, 12, 3]},
            {"version": "0.16.0", "python": [3, 12, 4]},
        ):
            with self.subTest(altered=altered):
                actual = {"exit_code": 0, "stdout": json.dumps(altered).encode(), "stderr": b""}
                self.assertTrue(compare.compare_outputs(expected, actual, "json", json_schema="version"))
        equal = {"exit_code": 0, "stdout": b'{ "python" : [3,12,3], "version" : "0.16.0" }', "stderr": b""}
        self.assertEqual([], compare.compare_outputs(expected, equal, "json", json_schema="version"))

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

    def test_worker_exception_produces_valid_blocked_report_and_keeps_original_error(self) -> None:
        report = self._valid_report()
        report.update({"status": "BLOCKED", "run_level_blocker": True, "capability_ids": ["cli.version"], "case_ids": ["case_one", "case_two"], "cases": []})
        error = RuntimeError("worker failed while writing artifacts")

        compare.record_run_error(report, error)
        report["counts"] = {status: 0 for status in {"PASS", "MISMATCH", "NOT_IMPLEMENTED", "ORACLE_ERROR", "CANDIDATE_ERROR", "TIMEOUT", "BLOCKED"}}
        report["counts"]["BLOCKED"] = 1
        compare.validate_report(report)
        self.assertEqual(report["errors"], [{"kind": "RuntimeError", "message": "worker failed while writing artifacts"}])
        self.assertEqual(report["case_ids"], [])
        self.assertEqual(report["capability_ids"], [])

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

    def test_illogic_and_tense_present_live_case_contracts(self) -> None:
        cases = {case["id"]: case for case in compare.load_manifest()["cases"]}
        illogic = {
            "illogic_all_nine_patterns_parent_only", "illogic_clean_and_near_miss_boundaries",
            "illogic_quote_suppression", "illogic_unicode_coordinates",
            "illogic_interleaved_pattern_group_limit_two",
            "illogic_parent_and_children_duplicate_path_coexistence",
        }
        tense = {
            "tense_present_all_active_patterns_and_both_unicode_dashes",
            "tense_present_clean_and_near_miss_boundaries", "tense_present_quote_suppression",
            "tense_present_unicode_coordinates", "tense_present_interleaved_pattern_group_limit_two",
            "tense_present_dash_forms_unicode_escape_edges",
        }
        self.assertTrue(illogic.issubset(cases))
        self.assertTrue(tense.issubset(cases))
        self.assertEqual({cases[item]["capability_id"] for item in illogic}, {"rule.misc.illogic"})
        self.assertEqual({cases[item]["capability_id"] for item in tense}, {"rule.misc.tense_present"})
        for case_id in illogic | tense:
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["argv"], ["check", "--config", "{file:config.json}", "--output-format", "json"])
                config = json.loads(base64.b64decode(case["files"][0]["content_base64"]))
                self.assertEqual(case["expected_oracle_outcome"], "findings" if case_id not in {
                    "illogic_clean_and_near_miss_boundaries", "illogic_quote_suppression",
                    "tense_present_clean_and_near_miss_boundaries", "tense_present_quote_suppression",
                } else "clean")
                self.assertEqual(config["max_errors"], 2 if "interleaved" in case_id else 1000)
        combined = json.loads(base64.b64decode(cases["illogic_parent_and_children_duplicate_path_coexistence"]["files"][0]["content_base64"]))
        self.assertEqual(combined["checks"], {"misc": {"illogic": True, "illogic.coin": True, "illogic.collusion": True}})
        self.assertEqual(cases["illogic_all_nine_patterns_parent_only"]["config_scenario"], "only_misc.illogic")
        tense_inputs = " ".join(base64.b64decode(cases[item]["stdin_base64"]).decode("utf-8") for item in (
            "tense_present_all_active_patterns_and_both_unicode_dashes", "tense_present_dash_forms_unicode_escape_edges"))
        self.assertIn("—", tense_inputs)
        self.assertIn("–", tense_inputs)

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


    def test_missing_explicit_config_error_cases_preserve_output_contract(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.config.missing_explicit_text_error"
        }
        self.assertEqual(set(cases), {
            "cli_config_missing_explicit_text_error_full",
            "cli_config_missing_explicit_text_error_compact",
        })
        for case_id, output_format, argv in (
            ("cli_config_missing_explicit_text_error_full", "full", ["check", "--config", "missing config.json"]),
            ("cli_config_missing_explicit_text_error_compact", "compact", ["check", "--config", "missing config.json", "--output-format", "compact"]),
        ):
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["files"], [])
                self.assertEqual(case["stdin_base64"], "")
                self.assertEqual(case["argv"], argv)
                self.assertEqual(case["output_format"], output_format)
                self.assertEqual(case["expected_oracle_outcome"], "cli_error")
                self.assertEqual(case["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})
                self.assertEqual(case["config_scenario"], "explicit_missing_config")

    def test_no_config_default_cases_have_no_discovery_config(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.config.no_config_default"
        }
        self.assertEqual(
            set(cases),
            {
                "cli_config_no_config_default_enabled_rule_finding",
                "cli_config_no_config_default_clean_input",
                "cli_config_no_config_default_disabled_curly_quotes_clean",
            },
        )
        for case in cases.values():
            with self.subTest(case=case["id"]):
                self.assertEqual(case["files"], [])
                self.assertEqual(case["argv"], ["check"])
                self.assertEqual(case["config_scenario"], "no_config_default")
                self.assertEqual(case["output_format"], "full")

    def test_plain_text_version_manifest_cases_are_exact(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] in {"cli.version_text", "cli.product_version_flag"}
        }
        expected = {
            "cli_version_subcommand_plain_text": ["version"],
            "cli_version_flag_plain_text": ["--version"],
        }
        self.assertEqual(set(cases), set(expected))
        self.assertEqual(len(cases), 2)
        self.assertEqual(cases["cli_version_subcommand_plain_text"]["capability_id"], "cli.version_text")
        self.assertEqual(cases["cli_version_flag_plain_text"]["capability_id"], "cli.product_version_flag")
        for case_id, argv in expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["argv"], argv)
                self.assertEqual(case["input_mode"], "stdin")
                self.assertEqual(case["stdin_base64"], "")
                self.assertEqual(case["files"], [])
                self.assertEqual(case["config_scenario"], "no_config")
                self.assertEqual(case["output_format"], "full")
                self.assertEqual(case["expected_oracle_outcome"], "clean")
                self.assertEqual(case["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})
        self.assertFalse(any("json" in case_id for case_id in cases))

    def test_version_json_metadata_manifest_cases_are_exact(self) -> None:
        expected = {
            "cli_version_json_metadata_subcommand": (["version", "--output-format", "json"], "json", "version"),
            "cli_version_json_metadata_short_attached": (["version", "-ojson"], "json", "version"),
            "cli_version_json_metadata_flag_before_format": (["--version", "--output-format", "json"], "json", "version"),
            "cli_version_json_metadata_format_before_flag": (["--output-format", "json", "--version"], "json", "version"),
            "cli_version_json_metadata_precedence_control": (["--output-format", "json", "version"], "bytes", None),
        }
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.version.json_metadata"
        }
        self.assertEqual(set(cases), set(expected))
        self.assertEqual(len(cases), 5)
        for case_id, (argv, stdout_policy, json_schema) in expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["argv"], argv)
                self.assertEqual(case["input_mode"], "stdin")
                self.assertEqual(case["stdin_base64"], "")
                self.assertEqual(case["files"], [])
                self.assertEqual(case["config_scenario"], "no_config")
                self.assertEqual(case["expected_oracle_outcome"], "clean")
                comparison = {
                    "stdout": stdout_policy, "stderr": "bytes", "exit_status": "exact",
                }
                if json_schema is not None:
                    comparison["json_schema"] = json_schema
                self.assertEqual(case["comparison"], comparison)

    def test_verbose_version_runtime_manifest_case_is_exact(self) -> None:
        cases = [
            case for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.version.verbose_runtime"
        ]
        self.assertEqual([case["id"] for case in cases], ["cli_version_subcommand_verbose_debug_text"])
        self.assertEqual(cases[0]["argv"], ["version", "--verbose"])
        self.assertEqual(cases[0]["comparison"], {
            "stdout": "bytes", "stderr": "bytes", "exit_status": "exact",
        })

    def test_dump_config_version_precedence_cases_are_exact(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.dump_config.version_precedence"
        }
        self.assertEqual(set(cases), {
            "cli_dump_config_version_flag",
            "cli_dump_config_version_compact",
            "cli_dump_config_default_version_flag",
        })
        for case in cases.values():
            with self.subTest(case=case["id"]):
                self.assertEqual(case["expected_oracle_outcome"], "clean")
                self.assertEqual(case["comparison"], {
                    "stdout": "bytes", "stderr": "bytes", "exit_status": "exact",
                })

    def test_check_version_invalid_config_precedence_cases_are_exact(self) -> None:
        cases = {
            case["id"]: case for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.config.check_version_invalid_config_precedence"
        }
        expected = {
            "check_version_checks_null_default": ("checks", "full", None),
            "check_version_checks_null_json": ("checks", "json"),
            "check_version_checks_null_compact": ("checks", "compact"),
            "check_version_per_file_checks_null_default": ("per_file_checks", "full", None),
            "check_version_per_file_checks_null_json": ("per_file_checks", "json"),
            "check_version_per_file_checks_null_compact": ("per_file_checks", "compact"),
        }
        self.assertEqual(set(cases), set(expected))
        for case_id, spec in expected.items():
            with self.subTest(case=case_id):
                field, output_format = spec[:2]
                format_args = [] if len(spec) == 3 else ["--output-format", output_format]
                case = cases[case_id]
                self.assertEqual(json.loads(base64.b64decode(case["files"][0]["content_base64"])), {field: None})
                self.assertEqual(case["argv"], ["check", "--config", "{file:config.json}", "--version", *format_args])
                self.assertEqual(case["output_format"], output_format)
                self.assertEqual(case["expected_oracle_outcome"], "cli_error")
                self.assertEqual(case["timeout_seconds"], 10)
                self.assertEqual(case["comparison"], {
                    "stdout": "json" if output_format == "json" else "bytes",
                    "stderr": "bytes",
                    "exit_status": "exact",
                })

    def test_root_config_precedence_cases_distinguish_option_position(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.root_config_argument_precedence"
        }
        expected = {
            "cli_root_config_before_dump_config_version": (["--config", "{file:bad.json}", "dump-config", "--version"], "clean"),
            "cli_root_config_before_version_subcommand": (["--config", "{file:bad.json}", "version"], "clean"),
            "cli_root_config_attached_before_version_subcommand": (["--config={file:bad.json}", "version"], "clean"),
            "cli_root_config_before_check_subcommand": (["--config", "{file:bad.json}", "check"], "clean"),
            "cli_dump_config_subcommand_config_control": (["dump-config", "--config", "missing.json", "--version"], "cli_error"),
            "cli_check_subcommand_config_control": (["check", "--config", "missing.json"], "cli_error"),
        }
        self.assertEqual(set(cases), set(expected))
        for case_id, (argv, outcome) in expected.items():
            with self.subTest(case=case_id):
                self.assertEqual(cases[case_id]["argv"], argv)
                self.assertEqual(cases[case_id]["expected_oracle_outcome"], outcome)
                expected_files = [] if case_id.endswith("_control") else [{"path": "bad.json", "content_base64": "ew=="}]
                self.assertEqual(cases[case_id]["files"], expected_files)

    def test_attached_output_format_forms_are_covered_by_command(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.output_format.attached_values"
        }
        expected = {
            "cli_check_attached_short_json": (["check", "-ojson"], "json", "json"),
            "cli_check_attached_long_json": (["check", "--output-format=json"], "json", "json"),
            "cli_check_attached_short_compact": (["check", "-ocompact"], "compact", "bytes"),
            "cli_check_attached_long_compact": (["check", "--output-format=compact"], "compact", "bytes"),
            "cli_dump_config_attached_short_json": (["dump-config", "-ojson"], "json", "bytes"),
            "cli_dump_config_attached_short_compact": (["dump-config", "-ocompact"], "compact", "bytes"),
            "cli_dump_config_attached_long_json": (["dump-config", "--output-format=json"], "json", "bytes"),
            "cli_dump_config_attached_long_compact": (["dump-config", "--output-format=compact"], "compact", "bytes"),
        }
        self.assertEqual(set(cases), set(expected))
        for case_id, (argv, fmt, stdout_policy) in expected.items():
            with self.subTest(case=case_id):
                self.assertEqual(cases[case_id]["argv"], argv)
                self.assertEqual(cases[case_id]["output_format"], fmt)
                self.assertEqual(cases[case_id]["comparison"], {
                    "stdout": stdout_policy, "stderr": "bytes", "exit_status": "exact",
                })

    def test_top_level_help_cases_use_exact_output_and_status_contract(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.top_level_help"
        }
        expected = {
            "cli_top_level_help_long": (["--help"], "clean"),
            "cli_top_level_help_short": (["-h"], "clean"),
            "cli_top_level_help_no_args": ([], "cli_error"),
        }
        self.assertEqual(set(cases), set(expected))
        for case_id, (argv, outcome) in expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["argv"], argv)
                self.assertEqual(case["expected_oracle_outcome"], outcome)
                self.assertEqual(case["stdin_base64"], "")
                self.assertEqual(case["files"], [])
                self.assertEqual(case["config_scenario"], "no_config")
                self.assertEqual(case["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})

    def test_help_argument_precedence_cases_pin_guard_behavior(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.help_argument_precedence"
        }
        expected = {
            "cli_check_missing_config_value_before_help": (["check", "--config", "--help"], "cli_error"),
            "cli_check_invalid_output_format_before_help": (["check", "--output-format", "invalid", "--help"], "cli_error"),
            "cli_top_level_config_value_check_before_help": (["--config", "check", "--help"], "clean"),
            "cli_check_delimiter_help_path": (["check", "--", "--help"], "clean"),
        }
        self.assertEqual(set(cases), set(expected))
        self.assertEqual(sum(case["capability_id"] == "cli.help_argument_precedence" for case in compare.load_manifest()["cases"]), 4)
        for case_id, (argv, outcome) in expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["argv"], argv)
                self.assertEqual(case["expected_oracle_outcome"], outcome)
                self.assertEqual(case["output_format"], "full")
                self.assertEqual(case["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})
                self.assertEqual(case["stdin_base64"], "")
        file_case = cases["cli_check_delimiter_help_path"]
        self.assertEqual(file_case["input_mode"], "files")
        self.assertEqual(file_case["files"], [{"path": "--help", "content_base64": ""}])

    def test_invalid_output_format_before_help_is_only_valid_as_cli_error(self) -> None:
        case = next(
            case for case in compare.load_manifest()["cases"]
            if case["id"] == "cli_check_invalid_output_format_before_help"
        )
        compare.validate_case(case)
        with self.assertRaises(ValueError):
            compare.validate_case({**case, "expected_oracle_outcome": "clean"})

    def test_invalid_output_format_error_cases_pin_argparse_failures(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.invalid_output_format_error"
        }
        expected = {
            "cli_invalid_output_format_long_form": ["check", "--output-format", "bogus"],
            "cli_invalid_output_format_short_alias": ["check", "-o", "bogus"],
            "cli_invalid_output_format_before_help": ["check", "--output-format", "bogus", "--help"],
            "cli_invalid_output_format_version_subcommand": ["version", "--output-format", "bogus"],
            "cli_invalid_output_format_dump_config_subcommand": ["dump-config", "--output-format", "bogus"],
            "cli_invalid_output_format_check_quote": ["check", "--output-format", 'bad"quote'],
            "cli_invalid_output_format_check_newline": ["check", "--output-format", "bad\nline"],
            "cli_invalid_output_format_check_tab": ["check", "--output-format", "bad\ttab"],
            "cli_invalid_output_format_version_quote": ["version", "--output-format", 'bad"quote'],
            "cli_invalid_output_format_version_newline": ["version", "--output-format", "bad\nline"],
            "cli_invalid_output_format_version_tab": ["version", "--output-format", "bad\ttab"],
            "cli_invalid_output_format_dump_config_quote": ["dump-config", "--output-format", 'bad"quote'],
            "cli_invalid_output_format_dump_config_newline": ["dump-config", "--output-format", "bad\nline"],
            "cli_invalid_output_format_dump_config_tab": ["dump-config", "--output-format", "bad\ttab"],
            "cli_invalid_output_format_check_u2028": ["check", "--output-format", "bad\u2028value"],
            "cli_invalid_output_format_check_u2029": ["check", "--output-format", "bad\u2029value"],
            "cli_invalid_output_format_check_u200b": ["check", "--output-format", "bad\u200bvalue"],
            "cli_invalid_output_format_check_00a0": ["check", "--output-format", "bad\u00a0value"],
            "cli_invalid_output_format_check_1c89": ["check", "--output-format", "bad\u1c89value"],
            "cli_invalid_output_format_version_u2028": ["version", "--output-format", "bad\u2028value"],
            "cli_invalid_output_format_version_u2029": ["version", "--output-format", "bad\u2029value"],
            "cli_invalid_output_format_version_u200b": ["version", "--output-format", "bad\u200bvalue"],
            "cli_invalid_output_format_version_00a0": ["version", "--output-format", "bad\u00a0value"],
            "cli_invalid_output_format_version_1c89": ["version", "--output-format", "bad\u1c89value"],
            "cli_invalid_output_format_dump_config_u2028": ["dump-config", "--output-format", "bad\u2028value"],
            "cli_invalid_output_format_dump_config_u2029": ["dump-config", "--output-format", "bad\u2029value"],
            "cli_invalid_output_format_dump_config_u200b": ["dump-config", "--output-format", "bad\u200bvalue"],
            "cli_invalid_output_format_dump_config_00a0": ["dump-config", "--output-format", "bad\u00a0value"],
            "cli_invalid_output_format_dump_config_1c89": ["dump-config", "--output-format", "bad\u1c89value"],
            "cli_invalid_output_format_config_value_version": ["--config", "version", "check", "--output-format", "bogus"],
            "cli_invalid_output_format_config_value_dump_config": ["--config", "dump-config", "check", "--output-format", "bogus"],
            "cli_invalid_output_format_config_value_check": ["--config", "check", "--output-format", "bogus"],
            "cli_invalid_output_format_check_short_attached": ["check", "-oBOGUS"],
            "cli_invalid_output_format_check_long_attached": ["check", "--output-format=BOGUS"],
            "cli_invalid_output_format_version_short_attached": ["version", "-oBOGUS"],
            "cli_invalid_output_format_version_long_attached": ["version", "--output-format=BOGUS"],
            "cli_invalid_output_format_dump_config_short_attached": ["dump-config", "-oBOGUS"],
            "cli_invalid_output_format_dump_config_long_attached": ["dump-config", "--output-format=BOGUS"],
            "cli_output_format_dash_value_check_short_dash_value": ["check", "-o", "--help"],
            "cli_output_format_dash_value_check_long_dash_value": ["check", "--output-format", "-x"],
            "cli_output_format_dash_value_version_short_dash_value": ["version", "-o", "--help"],
            "cli_output_format_dash_value_version_long_dash_value": ["version", "--output-format", "-x"],
            "cli_output_format_dash_value_dump_config_short_dash_value": ["dump-config", "-o", "--help"],
            "cli_output_format_dash_value_dump_config_long_dash_value": ["dump-config", "--output-format", "-x"],
        }
        self.assertEqual(set(cases), set(expected))
        for case_id, argv in expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                compare.validate_case(case)
                self.assertEqual(case["argv"], argv)
                self.assertEqual(case["expected_oracle_outcome"], "cli_error")
                self.assertEqual(case["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})
                self.assertEqual(case["stdin_base64"], "")
                self.assertEqual(case["files"], [])

    def test_invalid_utf8_file_cases_pin_full_and_json_per_file_errors(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.file_invalid_utf8_error"
        }
        self.assertEqual(
            set(cases),
            {
                "cli_invalid_utf8_file_full_per_file_error",
                "cli_invalid_utf8_file_json_per_file_error",
            },
        )
        for case_id, output_format in (
            ("cli_invalid_utf8_file_full_per_file_error", "full"),
            ("cli_invalid_utf8_file_json_per_file_error", "json"),
        ):
            with self.subTest(case=case_id):
                case = cases[case_id]
                compare.validate_case(case)
                self.assertEqual(case["output_format"], output_format)
                self.assertEqual(case["argv"][4], output_format)
                self.assertEqual(case["expected_oracle_outcome"], "clean")
                self.assertEqual(case["files"][1], {"path": "broken.txt", "content_base64": "/w=="})
                self.assertEqual(case["workspace_mutates"], False)
                self.assertEqual(case["comparison"]["exit_status"], "exact")
        self.assertFalse(cases["cli_invalid_utf8_file_full_per_file_error"]["comparison"].get("normalize_workspace_uri", False))
        self.assertTrue(cases["cli_invalid_utf8_file_json_per_file_error"]["comparison"]["normalize_workspace_uri"])

    def test_missing_path_after_double_dash_cases_pin_each_output_mode(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.missing_path_error_after_double_dash"
        }
        expected = {
            "cli_missing_path_after_double_dash_full": "full",
            "cli_missing_path_after_double_dash_compact": "compact",
            "cli_missing_path_after_double_dash_json": "json",
        }
        self.assertEqual(set(cases), set(expected))
        for case_id, output_format in expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["argv"], ["check", "--output-format", output_format, "--", "--help"])
                self.assertEqual(case["output_format"], output_format)
                self.assertEqual(case["expected_oracle_outcome"], "clean")
                self.assertEqual(case["input_mode"], "stdin")
                self.assertEqual(case["stdin_base64"], "")
                self.assertEqual(case["files"], [])
                self.assertEqual(case["config_scenario"], "no_config")
                self.assertEqual(case["workspace_mutates"], False)
                self.assertEqual(case["comparison"]["stdout"], "json" if output_format == "json" else "bytes")
                self.assertEqual(case["comparison"]["stderr"], "bytes")
                self.assertEqual(case["comparison"]["exit_status"], "exact")
                self.assertEqual(case["comparison"].get("normalize_workspace_uri", False), output_format == "json")
                compare.validate_case(case)

    def test_multi_file_cases_pin_order_per_file_limits_and_errors(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.multiple_file_paths"
        }
        formats = {
            "cli_multi_file_limits_per_file_full": "full",
            "cli_multi_file_limits_per_file_compact": "compact",
            "cli_multi_file_limits_per_file_json": "json",
        }
        duplicate_ids = {
            "cli_multi_file_duplicate_explicit_full",
            "cli_multi_file_overlap_explicit_directory_compact",
        }
        self.assertEqual(set(cases), set(formats) | {"cli_multi_file_json_continues_after_missing_path"} | duplicate_ids | {"cli_multi_file_per_file_override_json"})
        self.assertEqual(sum(case["capability_id"] == "cli.multiple_file_paths" for case in compare.load_manifest()["cases"]), 7)
        expected_files = [
            {"path": "config.json", "content_base64": "eyJtYXhfZXJyb3JzIjoxLCJjaGVja3MiOnsibWlzYyI6eyJhcG9sb2dpemluZyI6dHJ1ZX19fQ=="},
            {"path": "z.txt", "content_base64": "TW9yZSByZXNlYXJjaCBpcyBuZWVkZWQuIE1vcmUgcmVzZWFyY2ggaXMgbmVlZGVkLg=="},
            {"path": "a.txt", "content_base64": "TW9yZSByZXNlYXJjaCBpcyBuZWVkZWQuIE1vcmUgcmVzZWFyY2ggaXMgbmVlZGVkLg=="},
        ]
        for case_id, output_format in formats.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["output_format"], output_format)
                self.assertEqual(case["expected_oracle_outcome"], "findings")
                self.assertEqual(case["input_mode"], "files")
                self.assertEqual(case["files"], expected_files)
                self.assertEqual(case["argv"][-2:], ["{file:z.txt}", "{file:a.txt}"])
                self.assertEqual(case["config_scenario"], "explicit_selected_config")
                self.assertFalse(case["workspace_mutates"])
                self.assertEqual(case["comparison"], {
                    "stdout": "json" if output_format == "json" else "bytes",
                    "stderr": "bytes",
                    "exit_status": "exact",
                    **({"normalize_workspace_uri": True} if output_format == "json" else {}),
                })
                compare.validate_case(case)

        mixed = cases["cli_multi_file_json_continues_after_missing_path"]
        self.assertEqual(mixed["argv"][-2:], ["missing.txt", "{file:a.txt}"])
        self.assertEqual({item["path"] for item in mixed["files"]}, {"config.json", "a.txt"})
        self.assertEqual(mixed["output_format"], "json")
        self.assertEqual(mixed["expected_oracle_outcome"], "findings")
        self.assertEqual(mixed["comparison"]["stdout"], "json")
        self.assertTrue(mixed["comparison"]["normalize_workspace_uri"])
        compare.validate_case(mixed)

        duplicate_expected = {
            "cli_multi_file_duplicate_explicit_full": (
                "full",
                ["check", "--config", "{file:config.json}", "--output-format", "full", "{file:one.txt}", "{file:one.txt}"],
                {"config.json", "one.txt"},
            ),
            "cli_multi_file_overlap_explicit_directory_compact": (
                "compact",
                ["check", "--config", "{file:config.json}", "--output-format", "compact", "{file:tree/only.txt}", "tree"],
                {"config.json", "tree/only.txt"},
            ),
        }
        duplicate_config = "eyJtYXhfZXJyb3JzIjoxLCJjaGVja3MiOnsibWlzYyI6eyJhcG9sb2dpemluZyI6dHJ1ZX19fQ=="
        duplicate_content = "TW9yZSByZXNlYXJjaCBpcyBuZWVkZWQuIE1vcmUgcmVzZWFyY2ggaXMgbmVlZGVkLg=="
        for case_id, (output_format, argv, file_paths) in duplicate_expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["argv"], argv)
                self.assertEqual(case["output_format"], output_format)
                self.assertEqual(case["expected_oracle_outcome"], "findings")
                self.assertEqual({item["path"] for item in case["files"]}, file_paths)
                self.assertEqual(case["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})
                self.assertEqual(case["config_scenario"], "explicit_selected_config")
                self.assertEqual(case["files"][0]["content_base64"], duplicate_config)
                self.assertEqual(case["files"][1]["content_base64"], duplicate_content)
                compare.validate_case(case)

        per_file = cases["cli_multi_file_per_file_override_json"]
        self.assertEqual(per_file["argv"], ["check", "--config", "{file:config.json}", "--output-format", "json", "{file:selected.txt}", "{file:global.txt}"])
        self.assertEqual(per_file["output_format"], "json")
        self.assertEqual(per_file["expected_oracle_outcome"], "findings")
        self.assertEqual(per_file["comparison"], {"stdout": "json", "stderr": "bytes", "exit_status": "exact", "normalize_workspace_uri": True})
        self.assertEqual({item["path"] for item in per_file["files"]}, {"config.json", "selected.txt", "global.txt"})
        self.assertEqual(json.loads(base64.b64decode(per_file["files"][0]["content_base64"])), {"max_errors": 10, "checks": {"misc": {"apologizing": False}}, "per_file_checks": {"selected.txt": {"misc": {"apologizing": True}}}})
        self.assertEqual(base64.b64decode(per_file["files"][1]["content_base64"]), base64.b64decode(per_file["files"][2]["content_base64"]))
        self.assertEqual(per_file["config_scenario"], "per_file_selected_override_two_files")
        compare.validate_case(per_file)

    def test_directory_expansion_cases_pin_source_derived_contracts(self) -> None:
        cases = {case["id"]: case for case in compare.load_manifest()["cases"] if case["capability_id"] == "cli.directory_expansion"}
        self.assertEqual(set(cases), {
            "cli_directory_expansion_recursive", "cli_directory_expansion_case_sensitive_extensions",
            "cli_directory_expansion_ignored_extension", "cli_directory_expansion_per_file_max_errors",
            "cli_directory_expansion_among_explicit_paths", "cli_directory_expansion_suffix_rtf",
            "cli_directory_expansion_suffix_html", "cli_directory_expansion_suffix_tex",
            "cli_directory_expansion_suffix_markdown", "cli_directory_expansion_full_output",
            "cli_directory_expansion_compact_output", "cli_directory_expansion_root_symlink",
            "cli_directory_expansion_explicit_file_symlink_compact",
            "cli_directory_expansion_dangling_file_symlink_compact",
            "cli_directory_expansion_dangling_file_symlink_full",
            "cli_directory_expansion_dangling_file_symlink_json",
        })
        for case in cases.values():
            if case["id"] == "cli_directory_expansion_among_explicit_paths":
                self.assertEqual(case["argv"][-3:], ["before.txt", "tree", "after.txt"])
            elif case["id"] == "cli_directory_expansion_root_symlink":
                self.assertEqual(case["argv"][-1], "alias")
                self.assertEqual(case["symlinks"], [{"path": "alias", "target": "tree"}])
                self.assertEqual(case["expected_oracle_outcome"], "findings")
            elif case["id"] == "cli_directory_expansion_explicit_file_symlink_compact":
                self.assertEqual(case["argv"], ["check", "--config", "{file:config.json}", "--output-format", "compact", "alias.md"])
                self.assertEqual(case["symlinks"], [{"path": "alias.md", "target": "target.txt"}])
                self.assertEqual({item["path"] for item in case["files"]}, {"config.json", "target.txt"})
                self.assertEqual(case["output_format"], "compact")
                self.assertEqual(case["expected_oracle_outcome"], "findings")
            elif case["id"] == "cli_directory_expansion_dangling_file_symlink_compact":
                self.assertEqual(case["argv"], ["check", "--config", "{file:config.json}", "--output-format", "compact", "alias.md"])
                self.assertEqual(case["symlinks"], [{"path": "alias.md", "target": "target.txt", "dangling": True}])
                self.assertEqual({item["path"] for item in case["files"]}, {"config.json"})
                self.assertEqual(case["output_format"], "compact")
                self.assertEqual(case["expected_oracle_outcome"], "clean")
            elif case["id"] == "cli_directory_expansion_dangling_file_symlink_full":
                self.assertEqual(case["argv"], ["check", "--config", "{file:config.json}", "--output-format", "full", "alias.md"])
                self.assertEqual(case["symlinks"], [{"path": "alias.md", "target": "target.txt", "dangling": True}])
                self.assertEqual(case["output_format"], "full")
                self.assertEqual(case["expected_oracle_outcome"], "clean")
                self.assertEqual(case["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})
            elif case["id"] == "cli_directory_expansion_dangling_file_symlink_json":
                self.assertEqual(case["argv"], ["check", "--config", "{file:config.json}", "--output-format", "json", "alias.md"])
                self.assertEqual(case["symlinks"], [{"path": "alias.md", "target": "target.txt", "dangling": True}])
                self.assertEqual(case["output_format"], "json")
                self.assertEqual(case["expected_oracle_outcome"], "clean")
                self.assertEqual(case["comparison"], {"stdout": "json", "stderr": "bytes", "exit_status": "exact", "normalize_workspace_uri": True})
            else:
                self.assertEqual(case["argv"][-1], "tree")
            self.assertEqual(case["comparison"].get("normalize_workspace_uri", False), case["output_format"] == "json")
            compare.validate_case(case)
        self.assertEqual(sum(case["capability_id"] == "cli.directory_expansion" for case in compare.load_manifest()["cases"]), 16)
        for suffix in ("rtf", "html", "tex", "markdown"):
            case = cases[f"cli_directory_expansion_suffix_{suffix}"]
            self.assertEqual(case["expected_oracle_outcome"], "findings")
            self.assertEqual(case["argv"], ["check", "--config", "{file:config.json}", "--output-format", "json", "tree"])
            self.assertEqual({item["path"] for item in case["files"]}, {"config.json", f"tree/nested/only.{suffix}"})
            self.assertEqual(json.loads(base64.b64decode(case["files"][0]["content_base64"])), {"checks": {"misc": {"apologizing": True}}, "max_errors": 10})
        for output_format in ("full", "compact"):
            case = cases[f"cli_directory_expansion_{output_format}_output"]
            self.assertEqual(case["output_format"], output_format)
            self.assertEqual(case["expected_oracle_outcome"], "findings")
            self.assertEqual(case["argv"], ["check", "--config", "{file:config.json}", "--output-format", output_format, "tree"])
            self.assertEqual({item["path"] for item in case["files"]}, {"config.json", "tree/nested/only.txt"})
            self.assertEqual(case["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})
            self.assertEqual(json.loads(base64.b64decode(case["files"][0]["content_base64"])), {"checks": {"misc": {"apologizing": True}}, "max_errors": 10})
        self.assertEqual(cases["cli_directory_expansion_ignored_extension"]["expected_oracle_outcome"], "clean")
        limited = cases["cli_directory_expansion_per_file_max_errors"]
        self.assertEqual(json.loads(base64.b64decode(limited["files"][0]["content_base64"]))["max_errors"], 1)
        self.assertEqual({item["path"] for item in limited["files"]}, {"config.json", "tree/one.txt", "tree/two.txt"})

    def test_demo_compact_cases_pin_rule_selection_and_config_order(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.check_demo_compact_selected_rules"
        }
        expected = {
            "cli_demo_compact_selected_rule": ["check", "--demo", "--config", "{file:selected config.json}", "--output-format", "compact"],
            "cli_demo_ignores_extra_path": ["check", "--demo", "not-a-real-demo-input.md", "--config", "{file:selected config.json}", "--output-format", "compact"],
            "cli_demo_config_error_precedes_demo": ["check", "--demo", "--config", "missing config.json", "--output-format", "compact"],
        }
        self.assertEqual(set(cases), set(expected))
        for case_id, argv in expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["argv"], argv)
                self.assertEqual(case["stdin_base64"], "")
                self.assertEqual(case["output_format"], "compact")
                self.assertEqual(case["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})
                self.assertFalse(case["comparison"].get("normalize_workspace_uri", False))
        selected = cases["cli_demo_compact_selected_rule"]
        config = json.loads(base64.b64decode(selected["files"][0]["content_base64"]))
        self.assertEqual(config, {"max_errors": 1000, "checks": {"dates_times": {"am_pm": {"lowercase_periods": True}}}})
        self.assertEqual(selected["expected_oracle_outcome"], "findings")
        self.assertEqual(cases["cli_demo_ignores_extra_path"]["expected_oracle_outcome"], "findings")
        self.assertEqual(cases["cli_demo_config_error_precedes_demo"]["expected_oracle_outcome"], "cli_error")

    def test_demo_default_compact_uses_implicit_configuration(self) -> None:
        cases = [
            case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.demo.default_compact"
        ]
        self.assertEqual(len(cases), 1)
        case = cases[0]
        self.assertEqual(case["id"], "cli_demo_default_compact")
        self.assertEqual(case["argv"], ["check", "--demo", "--output-format", "compact"])
        self.assertEqual(case["config_scenario"], "no_config_default")
        self.assertEqual(case["files"], [])
        self.assertEqual(case["expected_oracle_outcome"], "findings")
        self.assertEqual(case["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})

    def test_annotations_default_dispatch_case_uses_implicit_configuration(self) -> None:
        cases = [
            case
            for case in compare.load_manifest()["cases"]
            if case["id"] == "annotations_misc_no_config_default_dispatch"
        ]
        self.assertEqual(len(cases), 1)
        case = cases[0]
        self.assertEqual(case["capability_id"], "rule.annotations.misc")
        self.assertEqual(base64.b64decode(case["stdin_base64"]), b"TODO")
        self.assertEqual(case["argv"], ["check", "--output-format", "json"])
        self.assertNotIn("--config", case["argv"])
        self.assertEqual(case["config_scenario"], "no_config_default")
        self.assertEqual(case["files"], [])
        self.assertEqual(case["expected_oracle_outcome"], "findings")
        self.assertEqual(case["comparison"], {"stdout": "json", "stderr": "bytes", "exit_status": "exact"})
        compare.validate_case(case)

    def test_missing_checks_configuration_cases_preserve_upstream_empty_mapping(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.config.missing_checks"
        }
        self.assertEqual(set(cases), {
            "cli_config_missing_checks_max_errors_one",
            "cli_config_missing_checks_per_file_override",
        })
        max_errors = cases["cli_config_missing_checks_max_errors_one"]
        self.assertEqual(max_errors["argv"], ["check", "--demo", "--config", "{file:config.json}", "--output-format", "compact"])
        self.assertEqual(json.loads(base64.b64decode(max_errors["files"][0]["content_base64"])), {"max_errors": 1})
        self.assertEqual(max_errors["expected_oracle_outcome"], "clean")
        per_file = cases["cli_config_missing_checks_per_file_override"]
        config = json.loads(base64.b64decode(per_file["files"][0]["content_base64"]))
        self.assertNotIn("checks", config)
        self.assertEqual(config, {"max_errors": 1, "per_file_checks": {"draft.txt": {"misc": {"suddenly": True}}}})
        self.assertEqual(base64.b64decode(per_file["files"][1]["content_base64"]), b"Suddenly, we launched.")
        self.assertEqual(per_file["expected_oracle_outcome"], "findings")
        for case in cases.values():
            compare.validate_case(case)

    def test_command_help_cases_cover_only_supported_subcommands(self) -> None:
        cases = {
            case["id"]: case
            for case in compare.load_manifest()["cases"]
            if case["capability_id"] == "cli.command_help"
        }
        expected = {
            "cli_command_help_version": ["version", "--help"],
            "cli_command_help_dump_config": ["dump-config", "--help"],
        }
        self.assertEqual(set(cases), set(expected))
        for case_id, argv in expected.items():
            with self.subTest(case=case_id):
                case = cases[case_id]
                self.assertEqual(case["argv"], argv)
                self.assertEqual(case["expected_oracle_outcome"], "clean")
                self.assertEqual(case["stdin_base64"], "")
                self.assertEqual(case["files"], [])
                self.assertEqual(case["config_scenario"], "no_config")
                self.assertEqual(case["comparison"], {"stdout": "bytes", "stderr": "bytes", "exit_status": "exact"})
        self.assertFalse(any(case["argv"] == ["check", "--help"] for case in cases.values()))


if __name__ == "__main__":
    unittest.main()
