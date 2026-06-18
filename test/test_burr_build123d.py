from pathlib import Path
import json
import sys
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "python"))

from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, m3_clearance_hole


class BurrBuild123dTests(unittest.TestCase):
    def test_records_design_data_without_build123d_geometry(self):
        with tempfile.TemporaryDirectory() as temp:
            temp_path = Path(temp)
            (temp_path / "design.py").write_text("print('design')\n", encoding="utf-8")
            (temp_path / "actuator.step").write_text("STEP PLACEHOLDER\n", encoding="utf-8")

            design = BurrDesignData(
                artifact_id="unit-actuator",
                artifact_type="actuator_mount",
                process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
            )
            design.source("design.py")
            design.artifact("actuator.step")
            design.part(
                "housing",
                bbox_min=(-42, -16, 0),
                bbox_max=(42, 16, 26),
            )
            m3_clearance_hole(
                design,
                feature_id="m3_lower_left",
                part="housing",
                center=(39.5, -8, 8),
                axis=(1, 0, 0),
                role="loaded_mount",
                create_geometry=False,
            )

            output = design.write(temp_path / DESIGN_DATA_FILE)
            data = json.loads(output.read_text(encoding="utf-8"))

            self.assertEqual(data["schema_version"], "burr.design-data.v1")
            self.assertEqual(data["artifact_id"], "unit-actuator")
            self.assertEqual(data["source"]["path"], "design.py")
            self.assertIn("sha256", data["source"])
            self.assertEqual(data["features"][0]["fastener"], "M3")
            self.assertEqual(data["features"][0]["role"], "loaded_mount")


if __name__ == "__main__":
    unittest.main()
