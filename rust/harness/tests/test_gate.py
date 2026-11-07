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
