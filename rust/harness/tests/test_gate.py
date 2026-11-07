import importlib.util
import sys
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
HARNESS = ROOT / "harness"
if str(HARNESS) not in sys.path:
    sys.path.insert(0, str(HARNESS))

spec = importlib.util.spec_from_file_location("gate", HARNESS / "gate.py")
gate = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(gate)


class GateTests(unittest.TestCase):
    def test_reusable_attestation_must_prove_each_locked_gate_command(self):
        cargo = Path("/toolchain/bin/cargo")
        manifest = Path("/checkout/rust/Cargo.toml")
        commands = [
            [str(cargo), "fmt", "--manifest-path", str(manifest), "--check"],
            [str(cargo), "clippy", "--manifest-path", str(manifest), "--all-targets", "--", "-D", "warnings"],
            [str(cargo), "test", "--locked", "--manifest-path", str(manifest)],
            [str(cargo), "build", "--locked", "--manifest-path", str(manifest)],
        ]
        attestation = {"build_commands": [{"command": command, "exit_code": 0} for command in commands]}

        self.assertEqual(gate.missing_gate_attestation_checks(attestation, cargo, manifest), [])
        attestation["build_commands"].pop(1)
        self.assertEqual(gate.missing_gate_attestation_checks(attestation, cargo, manifest), ["clippy -D warnings"])

    def test_reusable_attestation_rejects_failed_build_record(self):
        with self.assertRaisesRegex(ValueError, "unsuccessful"):
            gate.missing_gate_attestation_checks(
                {"build_commands": [{"command": ["cargo", "test"], "exit_code": 1}]},
                Path("cargo"), Path("Cargo.toml"),
            )

    def test_gate_execution_summary_records_stage_duration_and_scope(self):
        report = {}
        stages = {
            "rust_checks_and_build": {"duration_seconds": 42.5},
            "cumulative_live_comparison": {"duration_seconds": 12.25, "case_count": 18},
        }

        gate.add_gate_execution(report, 55.0, stages, 3, 18)

        self.assertEqual(
            report["gate_execution"],
            {
                "duration_seconds": 55.0,
                "stages": stages,
                "accepted_capability_count": 3,
                "live_case_count": 18,
            },
        )


if __name__ == "__main__":
    unittest.main()
