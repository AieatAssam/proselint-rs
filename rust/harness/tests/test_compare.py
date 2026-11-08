from __future__ import annotations

import json
import base64
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

    def test_case_workers_overlap_are_bounded_and_preserve_input_order(self) -> None:
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

        cases = [{"id": str(index)} for index in range(6)]
        results = compare.run_cases_concurrently(cases, worker, max_workers=2)
        self.assertEqual(results, [case["id"] for case in cases])
        self.assertEqual(peak, 2)

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
        probe = [sys.executable, "-c", "import json;print(json.dumps({'result':{'<stdin>':{'diagnostics':[]}}}))"]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = compare.run_case(case, probe, probe, root, None)
            second_case = dict(case, id="cache_probe_2")
            second = compare.run_case(second_case, probe, probe, root, None)
        self.assertEqual(first["status"], "PASS", first)
        self.assertEqual(second["status"], "PASS", second)
        first_cache = first["effective_environment"]["reference"]["PYTHONPYCACHEPREFIX"]
        second_cache = second["effective_environment"]["reference"]["PYTHONPYCACHEPREFIX"]
        self.assertEqual(first_cache, str(root / "cases" / "cache_probe" / "reference-pycache"))
        self.assertEqual(second_cache, str(root / "cases" / "cache_probe_2" / "reference-pycache"))
        self.assertNotEqual(first_cache, second_cache)
        self.assertNotEqual(first["effective_environment"]["candidate"]["PYTHONPYCACHEPREFIX"], first_cache)

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
        empty_home = dict(case, environment={"HOME": "", "XDG_CONFIG_HOME": None})
        compare.validate_case(empty_home)
        env = compare.fresh_env(Path("/isolated"), empty_home["environment"])
        self.assertEqual(env["HOME"], "")
        self.assertNotIn("XDG_CONFIG_HOME", env)
        for value in ("/home/dev", "relative-home"):
            invalid = dict(case, environment={"HOME": value})
            with self.assertRaisesRegex(ValueError, "HOME may only be overridden with an empty string"):
                compare.validate_case(invalid)
        for item in (dict(case["home_files"][0], path="../outside"), dict(case["home_files"][0], content_base64="not-base64")):
            invalid = json.loads(json.dumps(case))
            invalid["home_files"] = [item]
            with self.assertRaises(ValueError):
                compare.validate_case(invalid)
        invalid = json.loads(json.dumps(case))
        invalid["environment"] = {"PATH": None}
        with self.assertRaisesRegex(ValueError, "only XDG_CONFIG_HOME may be unset"):
            compare.validate_case(invalid)
        invalid = json.loads(json.dumps(case))
        invalid["environment"] = {"PYTHONPYCACHEPREFIX": "/tmp/shared-pycache"}
        with self.assertRaisesRegex(ValueError, "reserved for oracle isolation"):
            compare.validate_case(invalid)

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

    def test_real_manifest_schema_and_unique_ids(self) -> None:
        manifest = compare.load_manifest()
        self.assertEqual(manifest["schema_version"], 1)
        expected_case_ids = {
            "cli_version_subcommand_plain_text", "cli_version_flag_plain_text",
            "cli_user_discovery_xdg_path", "cli_user_discovery_home_fallback_unset", "cli_user_discovery_home_fallback_empty",
            "cli_user_discovery_cwd_precedes_user", "cli_user_discovery_ancestor_precedes_user",
            "cli_user_discovery_malformed_first_user_errors", "cli_user_discovery_explicit_overrides_user", "cli_user_discovery_xdg_relative_path",
            "cli_config_cwd_discovers_finding",
            "cli_config_no_config_default_enabled_rule_finding",
            "cli_config_no_config_default_clean_input",
            "cli_config_no_config_default_disabled_curly_quotes_clean",
            "cli_config_missing_explicit_text_error_full",
            "cli_config_missing_explicit_text_error_compact",
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
            "cli_config_consumed_max_errors_null", "cli_config_consumed_max_errors_true",
            "cli_config_consumed_max_errors_string", "cli_config_consumed_max_errors_array",
            "cli_config_consumed_max_errors_object", "cli_config_consumed_max_errors_negative_integer",
            "cli_config_consumed_max_errors_fractional_number",
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
        expected_case_ids.update({
            "archaism_all_47_active_forms",
            "archaism_commented_out_terms_and_clean_controls",
            "archaism_case_insensitive_repetition",
            "archaism_boundary_without_activation",
            "archaism_raw_embedded_after_activation",
            "archaism_paired_quote_suppression",
            "archaism_unicode_ascii_boundaries_and_positions",
            "archaism_multiword_spacing_and_controls",
            "archaism_embedded_multiword_raw_matches",
            "archaism_activation_order_full",
            "archaism_activation_order_max_errors_two",
            "archaism_different_length_activation_order",
            "archaism_different_length_activation_order_max_errors_one",
            "month_year_comma_all_twelve_months",
            "month_year_comma_case_and_repetition",
            "month_year_comma_digit_threshold",
            "month_year_comma_spacing_and_punctuation_controls",
            "month_year_comma_raw_substring_match",
            "month_year_comma_quote_suppression",
            "month_year_comma_unicode_positions_and_decimal_digits",
            "month_year_comma_max_errors",
            "dash_and_from_standard_ranges",
            "dash_and_from_valid_date_time_and_prose_controls",
            "dash_and_from_case_and_punctuation",
            "dash_and_from_raw_substring",
            "dash_and_from_quote_suppression",
            "dash_and_from_unicode_positions",
            "dash_and_from_multiple_findings_order",
            "dash_and_from_max_errors",
            "dash_and_from_unicode_decimal_digits",
            "dash_and_from_unicode_word_separator",
            "dash_and_from_combining_mark_separator",
            "dash_and_from_undertie_separator",
            "dash_and_from_zero_width_non_joiner_separator",
        })
        expected_case_ids.update(f"spelling_athletes_form_{index:02d}" for index in range(1, 18))
        expected_case_ids.update({
            "month_of_year_all_months", "month_of_year_digits_quotes_unicode_positions", "month_of_year_ascii_digit_boundary",
            "scare_quotes_positive_and_boundary_controls", "scare_quotes_unicode_word_boundaries", "scare_quotes_quote_suppression",
            "hyperbole_raw_greedy_mixed_punctuation", "hyperbole_single_punctuation_negative", "hyperbole_unicode_word_match",
            "misplaced_punctuation_ordered_fast_patterns", "misplaced_punctuation_word_boundaries", "misplaced_punctuation_unicode_positions",
            "exclamation_two_match_threshold", "exclamation_padded_length_threshold_lower_edge",
            "exclamation_padded_length_threshold_upper_edge", "exclamation_quote_selected_then_suppressed",
        })
        expected_case_ids.update({
            "spelling_athletes_negative_correct_forms",
            "spelling_athletes_case_insensitive_repetition",
            "spelling_athletes_ascii_prefix_suffix_boundaries",
            "spelling_athletes_punctuation_boundaries",
            "spelling_athletes_unicode_left_boundary",
            "spelling_athletes_unicode_right_boundary",
            "spelling_athletes_quoted_suppression",
            "spelling_athletes_j_escaped_source_entry",
            "spelling_athletes_j_regex_wildcard_negative",
            "spelling_athletes_j_correct_form_negative",
            "spelling_athletes_order_all_forms",
            "spelling_athletes_max_errors_truncates",
        })
        expected_case_ids.update({
            "social_awareness_nword_upstream_example",
            "social_awareness_nword_without_hyphen",
            "social_awareness_nword_case_insensitive",
            "social_awareness_nword_unanchored_inside_word",
            "social_awareness_nword_space_is_near_miss",
            "social_awareness_nword_unicode_codepoint_position",
            "social_awareness_nword_quoted_suppressed",
            "social_awareness_nword_max_errors_first_match",
        })
        expected_case_ids.update({
            "curly_quotes_ascii_word_positive",
            "curly_quotes_ascii_digits_and_space",
            "curly_quotes_multiline_whitespace",
            "curly_quotes_punctuation_near_miss",
            "curly_quotes_empty_near_miss",
            "curly_quotes_allow_quotes_true",
            "curly_quotes_unicode_positions_and_content",
            "curly_quotes_max_errors_one",
            "curly_quotes_adjacent_quote_boundary_match",
        })
        expected_case_ids.update({"weasel_very_positive_ordinary_case", "weasel_very_well_exclusions", "weasel_very_tabs_and_newlines", "weasel_very_unicode_letter_boundary", "weasel_very_quote_suppression", "weasel_very_max_errors_one", "weasel_very_combining_mark_and_ascii_boundaries", "weasel_very_multiline_position", "weasel_very_u1c89_leading", "weasel_very_u1c89_trailing", "weasel_very_u0345_leading", "weasel_very_u0345_trailing", "weasel_very_u05b0_leading", "weasel_very_u05b0_trailing"})
        expected_case_ids.update({"jargon_interleave_limit_two", "hedging_interleave_limit_two"})
        expected_case_ids.update({"waxed_all_64_form_modifier_pairs", "waxed_correct_adjectives_clean", "waxed_raw_activation_and_case_boundaries", "waxed_quote_suppression", "waxed_max_errors_one", "waxed_pattern_group_truncation", "waxed_unicode_coordinates", "bottleneck_all_seven_phrases", "bottleneck_clean_and_near_boundary", "bottleneck_case_duplicates_and_overlap", "bottleneck_quote_suppression", "bottleneck_max_errors_one", "bottleneck_max_errors_interleaved_pattern_order", "bottleneck_pattern_group_truncation", "bottleneck_unicode_coordinates"})
        expected_case_ids.update({"mixed_misc_all_five_items_and_regex_alternatives", "mixed_misc_correct_forms_and_near_misses", "mixed_misc_case_and_duplicate_occurrences", "mixed_misc_quote_suppression", "mixed_misc_unicode_coordinates", "mixed_misc_max_errors_interleaved_pattern_groups"})
        expected_case_ids.update({"ras_syndrome_all_twenty_mappings", "ras_syndrome_correct_forms_clean", "ras_syndrome_case_and_duplicate_occurrences", "ras_syndrome_mixed_case_source_key_lookup", "ras_syndrome_quote_suppression", "ras_syndrome_unicode_coordinates", "ras_syndrome_interleaved_pattern_group_limit_two", "ras_syndrome_plural_overlap_when_only_plural_activates"})
        expected_case_ids.update({"illogic_all_nine_patterns_parent_only", "illogic_clean_and_near_miss_boundaries", "illogic_quote_suppression", "illogic_unicode_coordinates", "illogic_interleaved_pattern_group_limit_two", "illogic_parent_and_children_duplicate_path_coexistence"})
        expected_case_ids.update({"tense_present_all_active_patterns_and_both_unicode_dashes", "tense_present_clean_and_near_miss_boundaries", "tense_present_quote_suppression", "tense_present_unicode_coordinates", "tense_present_interleaved_pattern_group_limit_two", "tense_present_dash_forms_unicode_escape_edges"})
        expected_case_ids.update({"lexical_illusions_single_repeat", "lexical_illusions_exact_exceptions", "lexical_illusions_triple_repeats", "lexical_illusions_phrase_lengths_one_to_four", "lexical_illusions_more_than_four_words", "lexical_illusions_whitespace_backreference_exact", "lexical_illusions_whitespace_backreference_mismatch", "lexical_illusions_apostrophe_hyphen_boundaries", "lexical_illusions_unicode_casefold_and_word_semantics", "lexical_illusions_quote_suppression", "lexical_illusions_unicode_codepoint_offsets", "lexical_illusions_max_errors_one"})
        expected_case_ids.update({"punctuation_spacing_only_one_space", "punctuation_spacing_only_two_spaces", "punctuation_spacing_one_space_majority", "punctuation_spacing_two_space_majority", "punctuation_spacing_tie", "punctuation_spacing_interleaved_occurrences", "punctuation_spacing_quoted_minority_suppressed", "punctuation_spacing_unicode_coordinates", "punctuation_spacing_max_errors_one"})
        expected_case_ids.update({
            "astra_ordering_capitalization_seasons_limit_two", "astra_ordering_capitalization_seasons_reversed_limit_two",
            "astra_ordering_capitalization_months_limit_two", "astra_ordering_capitalization_months_reversed_limit_two",
            "astra_ordering_capitalization_days_limit_two", "astra_ordering_capitalization_days_reversed_limit_two",
            "astra_ordering_capitalization_terms_limit_two", "astra_ordering_capitalization_terms_reversed_limit_two",
            "astra_ordering_debased_limit_two", "astra_ordering_debased_reversed_limit_two",
            "astra_ordering_metadiscourse_limit_two", "astra_ordering_metadiscourse_reversed_limit_two",
            "astra_ordering_many_a_limit_two", "astra_ordering_many_a_reversed_limit_two",
            "astra_ordering_greylist_limit_two",
        })
        expected_case_ids.update({
            "cli_output_compact_clean_stdin", "cli_output_compact_stdin_finding",
            "cli_output_compact_nested_file_basename", "cli_output_compact_invalid_utf8_file_error",
            "per_file_checks_enable_globally_disabled_annotation",
            "per_file_checks_override_merges_global_checks",
            "per_file_checks_nonmatching_pattern_uses_global",
        })
        expected_case_ids.update({
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
        })
        expected_case_ids.update({
            "cliches_write_good_all_items", "cliches_write_good_case",
            "cliches_write_good_quoted_suppression", "cliches_write_good_word_boundary",
            "cliches_write_good_unicode_position", "cliches_write_good_package_only_like_the_plague",
            "cliches_write_good_pinned_whitespace_youre_the_boss", "cliches_write_good_max_errors_ordering",
            "cliches_write_good_regex_optional_final_e",
        })
        expected_case_ids.update({
            "cli_dump_config_default",
            "cli_dump_config_default_ignores_explicit_config",
            "cli_dump_config_explicit_flattened",
            "cli_dump_config_unicode_ensure_ascii",
            "cli_dump_config_invalid_json",
            "cli_dump_config_invalid_json_with_default",
        })
        expected_case_ids.update({
            "cli_top_level_help_long",
            "cli_top_level_help_short",
            "cli_top_level_help_no_args",
        })
        expected_case_ids.update({
            "cli_command_help_version",
            "cli_command_help_dump_config",
        })
        expected_case_ids.update({
            "cli_check_help",
            "cli_check_help_short_alias",
        })
        expected_case_ids.update({
            "cli_check_help_with_extra_path",
            "cli_check_help_config_before_help",
            "cli_check_help_global_config_before_subcommand",
        })
        expected_case_ids.update({
            "cli_check_missing_config_value_before_help",
            "cli_check_invalid_output_format_before_help",
            "cli_top_level_config_value_check_before_help",
            "cli_check_delimiter_help_path",
        })
        expected_case_ids.update({
            "cli_invalid_output_format_long_form",
            "cli_invalid_output_format_short_alias",
            "cli_invalid_output_format_before_help",
            "cli_invalid_output_format_version_subcommand",
            "cli_invalid_output_format_dump_config_subcommand",
            "cli_invalid_output_format_config_value_version",
            "cli_invalid_output_format_config_value_dump_config",
            "cli_invalid_output_format_config_value_check",
            "cli_invalid_utf8_file_full_per_file_error",
            "cli_invalid_utf8_file_json_per_file_error",
            "cli_version_before_check_no_output",
            "cli_version_before_check_json",
            "cli_output_format_json_version_before_check",
            "cli_root_output_format_missing_value_version",
            "cli_root_short_output_format_missing_value_version",
            "cli_check_version_flag_control",
            "cli_version_before_check_missing_path",
            "cli_xdg_missing_does_not_fall_back_to_home_check",
            "cli_xdg_missing_does_not_fall_back_to_home_dump_config",
            "cli_unknown_argument_root",
            "cli_unknown_argument_check",
            "cli_unknown_argument_version",
            "cli_unknown_argument_dump_config",
            "cli_unknown_argument_short",
            "cli_unknown_argument_attached_value",
            "cli_unknown_argument_long_o_prefix",
            "cli_unknown_after_double_dash_is_path",
            "cli_unknown_argument_multiple",
            "cli_unknown_argument_wrong_subcommand_demo",
            "cli_unknown_argument_wrong_subcommand_default",
            "cli_unknown_argument_help_precedence",
            "cli_missing_config_before_option_at_root",
            "cli_missing_config_before_option_in_check",
            "cli_missing_output_format_before_option",
            "cli_missing_short_output_format_before_option",
            "cli_missing_output_format_before_help",
            "cli_missing_short_output_format_before_help",
            "cli_config_single_dash_value",
            "cli_config_negative_integer_value",
            "cli_config_negative_decimal_value",
            "cli_output_format_single_dash_value",
            "cli_output_format_negative_integer_value",
            "cli_output_format_negative_decimal_without_integer",
            "cli_config_unicode_negative_decimal_value",
            "cli_output_format_unicode_negative_decimal_value",
        })
        expected_case_ids.update({
            "cli_config_missing_value_at_root_argv_end",
            "cli_config_missing_value_at_check_argv_end",
            "cli_output_format_missing_value_at_root_argv_end",
            "cli_output_format_missing_value_at_check_argv_end",
            "cli_short_output_format_missing_value_at_root_argv_end",
            "cli_short_output_format_missing_value_at_check_argv_end",
        })
        expected_case_ids.update({
            "cli_missing_path_after_double_dash_full",
            "cli_missing_path_after_double_dash_compact",
            "cli_missing_path_after_double_dash_json",
        })
        expected_case_ids.update({
            "cli_multi_file_limits_per_file_full",
            "cli_multi_file_limits_per_file_compact",
            "cli_multi_file_limits_per_file_json",
            "cli_multi_file_json_continues_after_missing_path",
        })
        expected_case_ids.update({
            "cli_directory_expansion_recursive",
            "cli_directory_expansion_case_sensitive_extensions",
            "cli_directory_expansion_ignored_extension",
            "cli_directory_expansion_per_file_max_errors",
            "cli_directory_expansion_among_explicit_paths",
        })
        expected_case_ids.add("cli_config_empty_home_fallback")
        expected_case_ids.update({
            "cli_demo_compact_selected_rule",
            "cli_demo_ignores_extra_path",
            "cli_demo_config_error_precedes_demo",
        })
        expected_case_ids.add("cli_demo_json_selected_rule")
        self.assertEqual({item["id"] for item in manifest["cases"]}, expected_case_ids)
        self.assertEqual(len(manifest["cases"]), len(expected_case_ids))
        self.assertEqual(
            sum(case["capability_id"] == "cli.config.no_config_default" for case in manifest["cases"]),
            3,
        )
        self.assertEqual(
            sum(case["capability_id"] == "cli.config.missing_explicit_text_error" for case in manifest["cases"]),
            2,
        )

        self.assertEqual(
            {capability: sum(case["capability_id"] == capability for case in manifest["cases"])
             for capability in {case["capability_id"] for case in manifest["cases"]}},
            {'rule.misc.debased': 15, 'rule.misc.capitalization.days': 12, 'rule.cliches.hell': 7, 'harness.oracle_smoke': 2, 'rule.dates_times.am_pm.lowercase_periods': 9, 'rule.dates_times.am_pm.midnight_noon': 10, 'rule.dates_times.dates.decade_apostrophes': 10, 'rule.misc.narcissism': 9, 'rule.misc.latin': 10, 'rule.misc.back_formations': 8, 'rule.misc.capitalization.terms': 12, 'rule.industrial_language.airlinese': 12, 'cli.json_no_findings': 8, 'rule.misc.phrasal_adjectives.ly': 7, 'rule.misc.greylist': 10, 'rule.misc.many_a': 12, 'cli.output_format.full': 3, 'cli.output.compact': 4, 'cli.config.ignored_values': 5, 'rule.misc.professions': 8, 'cli.config.cwd_discovery': 3, 'cli.config.empty_home_fallback': 1, 'cli.config.no_config_default': 3, 'cli.config.missing_explicit_text_error': 2, 'cli.config.ancestor_discovery': 5, 'rule.misc.illogic.coin': 7, 'rule.hedging': 14, 'cli.config.invalid_utf8': 5, 'rule.misc.whence': 6, 'rule.annotations.misc': 10, 'rule.typography.symbols.sentence_spacing': 7, 'cli.config.consumed_special_values': 19, 'rule.cliches.misc.write_good': 9, 'cli.config.malformed_json': 22, 'rule.typography.symbols.trademark': 7, 'rule.typography.symbols.copyright': 6, 'rule.misc.pretension': 9, 'cli.config.recursion_depth': 12, 'rule.industrial_language.bureaucratese': 11, 'rule.misc.institution_name': 8, 'rule.misc.currency': 12, 'rule.misc.suddenly': 7, 'cli.coordinates': 6, 'rule.misc.but': 8, 'rule.psychology.mental_telepathy': 6, 'rule.misc.illogic.collusion': 7, 'rule.misc.false_plurals.kudos': 7, 'rule.typography.symbols.multiplication': 7, 'rule.misc.capitalization.seasons': 12, 'cli.config.directory_path': 4, 'rule.misc.apologizing': 7, 'rule.misc.metadiscourse': 12, 'rule.misc.inferior_superior': 12, 'rule.misc.false_plurals.misc': 9, 'cli.config.per_file_checks': 7, 'rule.redundancy.misc.wallace': 10, 'rule.dates_times.am_pm.spacing': 8, 'rule.typography.symbols.ellipsis': 6, 'rule.psychology.lie_detector': 12, 'rule.industrial_language.jargon': 11, 'cli.config.user_discovery': 10, 'rule.misc.capitalization.months': 8, 'rule.psychology.p_equals_zero': 8, 'spelling.ance_ence': 10, 'rule.misc.composition': 38, 'cli.combined_checks': 3, 'rule.terms.eponymous_adjectives': 4, 'rule.spelling.ally_ly': 10, 'rule.spelling.ely_ly': 9, 'rule.spelling.ve_of': 11, 'rule.malapropisms': 10, 'rule.spelling.in_un': 10, 'rule.mondegreens': 10, 'rule.skunked_terms': 11, 'rule.oxymorons': 16, 'rule.redundancy.misc.nordquist': 11, 'rule.misc.not_guilty': 4, 'spelling.er_or': 8, 'spelling.able_ible': 9, 'spelling.able_atable': 8, 'spelling.em_im_en_in': 8, 'spelling.athletes': 29, 'rule.archaism': 13, 'rule.dates_times.dates.month_year_comma': 8, 'rule.dates_times.dates.dash_and_from': 13, 'rule.dates_times.dates.month_of_year': 3, 'rule.misc.scare_quotes': 3, 'rule.typography.punctuation.hyperbole': 3, 'rule.typography.punctuation.misplaced': 3, 'rule.typography.punctuation.exclamation': 4, 'rule.social_awareness.nword': 8, 'rule.typography.symbols.curly_quotes': 9, 'rule.weasel_words.very': 14, 'rule.misc.waxed': 7, 'rule.mixed_metaphors.bottleneck': 8, 'rule.mixed_metaphors.misc': 6, 'rule.redundancy.ras_syndrome': 8, 'rule.misc.illogic': 6, 'rule.misc.tense_present': 6, 'rule.lexical_illusions': 12, 'rule.typography.punctuation.spacing': 9, 'rule.typography.diacritical_marks': 22, 'cli.dump_config_default': 6, 'cli.version_text': 2, 'cli.top_level_help': 3, 'cli.command_help': 2, 'cli.check_help': 2, 'cli.help_combined_arguments': 3, 'cli.help_argument_precedence': 4, 'cli.invalid_output_format_error': 8, 'cli.file_invalid_utf8_error': 2, 'cli.missing_path_error_after_double_dash': 3, 'cli.multiple_file_paths': 4, 'cli.check_demo_compact_selected_rules': 3, 'cli.demo.json': 1, 'cli.directory_expansion': 5, 'cli.version_argument_precedence': 7, 'cli.unrecognized_arguments': 12, 'cli.missing_option_value_before_option': 14, 'cli.missing_option_value_at_argv_end': 6},
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
            if case["capability_id"] == "cli.version_text"
        }
        expected = {
            "cli_version_subcommand_plain_text": ["version"],
            "cli_version_flag_plain_text": ["--version"],
        }
        self.assertEqual(set(cases), set(expected))
        self.assertEqual(len(cases), 2)
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
            "cli_invalid_output_format_config_value_version": ["--config", "version", "check", "--output-format", "bogus"],
            "cli_invalid_output_format_config_value_dump_config": ["--config", "dump-config", "check", "--output-format", "bogus"],
            "cli_invalid_output_format_config_value_check": ["--config", "check", "--output-format", "bogus"],
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
        self.assertEqual(set(cases), set(formats) | {"cli_multi_file_json_continues_after_missing_path"})
        self.assertEqual(sum(case["capability_id"] == "cli.multiple_file_paths" for case in compare.load_manifest()["cases"]), 4)
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

    def test_directory_expansion_cases_pin_source_derived_contracts(self) -> None:
        cases = {case["id"]: case for case in compare.load_manifest()["cases"] if case["capability_id"] == "cli.directory_expansion"}
        self.assertEqual(set(cases), {
            "cli_directory_expansion_recursive", "cli_directory_expansion_case_sensitive_extensions",
            "cli_directory_expansion_ignored_extension", "cli_directory_expansion_per_file_max_errors",
            "cli_directory_expansion_among_explicit_paths",
        })
        for case in cases.values():
            self.assertEqual(case["argv"][-1], "tree") if case["id"] != "cli_directory_expansion_among_explicit_paths" else self.assertEqual(case["argv"][-3:], ["before.txt", "tree", "after.txt"])
            self.assertTrue(case["comparison"]["normalize_workspace_uri"])
            compare.validate_case(case)
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
