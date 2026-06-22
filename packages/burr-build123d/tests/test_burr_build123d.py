from pathlib import Path
import json
import tempfile
import unittest

from burr_build123d import (
    BurrDesignData,
    DESIGN_DATA_FILE,
    bearing_seat,
    counterbore,
    heat_set_insert_pocket,
    m3_clearance_hole,
    straight_slot,
)


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
            self.assertEqual(data["features"][0]["intent"], "mechanical_interface")

    def test_records_non_mechanical_intent(self):
        design = BurrDesignData(
            artifact_id="unit-actuator",
            artifact_type="actuator_mount",
        )
        m3_clearance_hole(
            design,
            feature_id="m3_lightening",
            part="housing",
            center=(0, 0, 0),
            axis=(1, 0, 0),
            role="lightening",
            intent="weight_reduction",
            create_geometry=False,
        )

        self.assertEqual(design.features[0]["intent"], "weight_reduction")

    def test_records_rulepack_measurements_and_generic_feature(self):
        design = BurrDesignData(
            artifact_id="unit-slider",
            artifact_type="captured_slider",
        )
        design.rulepack("../rules/captured_slider.rulepack.json")
        design.measurement("head_side_clearance_mm", 0.25)
        design.measurements_update({"carriage_lip_each_side_mm": 3.5})
        design.feature(
            feature_id="left_capture_lip",
            kind="capture_lip",
            part="carriage",
            role="lift_off_blocker",
            engagement_mm=3.5,
        )

        data = design.to_dict()

        self.assertEqual(data["rulepack"]["path"], "../rules/captured_slider.rulepack.json")
        self.assertEqual(data["measurements"]["head_side_clearance_mm"], 0.25)
        self.assertEqual(data["measurements"]["carriage_lip_each_side_mm"], 3.5)
        self.assertEqual(data["features"][0]["id"], "left_capture_lip")
        self.assertEqual(data["features"][0]["kind"], "capture_lip")
        self.assertEqual(data["features"][0]["role"], "lift_off_blocker")
        self.assertEqual(data["features"][0]["intent"], "mechanical_interface")

    def test_rejects_invalid_rulepack_measurement_and_generic_feature(self):
        design = BurrDesignData(
            artifact_id="unit-slider",
            artifact_type="captured_slider",
        )

        with self.assertRaises(ValueError):
            design.rulepack("")
        with self.assertRaises(ValueError):
            design.measurement("", 0.25)
        with self.assertRaises(ValueError):
            design.measurement("bad", float("nan"))
        with self.assertRaises(ValueError):
            design.feature(feature_id="", kind="capture_lip")
        with self.assertRaises(ValueError):
            design.feature(feature_id="capture", kind="", intent="mechanical_interface")
        with self.assertRaises(ValueError):
            design.feature(feature_id="capture", kind="capture_lip", intent="")
        with self.assertRaises(ValueError):
            design.feature(feature_id="capture", kind="capture_lip", id="override")

    def test_rejects_empty_intent(self):
        design = BurrDesignData(
            artifact_id="unit-actuator",
            artifact_type="actuator_mount",
        )
        with self.assertRaises(ValueError):
            m3_clearance_hole(
                design,
                feature_id="m3_bad",
                part="housing",
                center=(0, 0, 0),
                axis=(1, 0, 0),
                role="mount",
                intent="",
                create_geometry=False,
            )

    def test_records_straight_slot_without_build123d_geometry(self):
        design = BurrDesignData(
            artifact_id="unit-actuator",
            artifact_type="actuator_mount",
        )
        straight_slot(
            design,
            feature_id="motor_adjust_slot",
            part="housing",
            center=(0, -8, 10),
            axis=(1, 0, 0),
            span_axis=(0, 1, 0),
            role="loaded_mount",
            width_mm=4.0,
            length_mm=18.0,
            cut_depth_mm=70.0,
            create_geometry=False,
        )

        feature = design.features[0]
        self.assertEqual(feature["kind"], "straight_slot")
        self.assertEqual(feature["intent"], "mechanical_interface")
        self.assertEqual(feature["width_mm"], 4.0)
        self.assertEqual(feature["length_mm"], 18.0)
        self.assertEqual(feature["center_mm"], [0.0, -8.0, 10.0])
        self.assertEqual(feature["axis"], [1.0, 0.0, 0.0])
        self.assertEqual(feature["span_axis"], [0.0, 1.0, 0.0])

    def test_rejects_invalid_straight_slot(self):
        design = BurrDesignData(
            artifact_id="unit-actuator",
            artifact_type="actuator_mount",
        )
        with self.assertRaises(ValueError):
            straight_slot(
                design,
                feature_id="bad_slot",
                part="housing",
                center=(0, 0, 0),
                axis=(1, 0, 0),
                span_axis=(0, 1, 0),
                role="loaded_mount",
                width_mm=4.0,
                length_mm=4.0,
                cut_depth_mm=70.0,
                create_geometry=False,
            )

    def test_records_counterbore_without_build123d_geometry(self):
        design = BurrDesignData(
            artifact_id="unit-actuator",
            artifact_type="actuator_mount",
        )
        counterbore(
            design,
            feature_id="m3_counterbore",
            part="housing",
            center=(0, 0, 10),
            axis=(1, 0, 0),
            role="loaded_mount",
            bore_diameter_mm=3.4,
            counterbore_diameter_mm=6.8,
            counterbore_depth_mm=4.0,
            through_depth_mm=20.0,
            create_geometry=False,
        )

        feature = design.features[0]
        self.assertEqual(feature["kind"], "counterbore")
        self.assertEqual(feature["intent"], "mechanical_interface")
        self.assertEqual(feature["bore_diameter_mm"], 3.4)
        self.assertEqual(feature["counterbore_diameter_mm"], 6.8)
        self.assertEqual(feature["counterbore_depth_mm"], 4.0)
        self.assertEqual(feature["center_mm"], [0.0, 0.0, 10.0])
        self.assertEqual(feature["counterbore_center_mm"], [-8.0, 0.0, 10.0])
        self.assertEqual(feature["shoulder_center_mm"], [-6.0, 0.0, 10.0])

    def test_rejects_invalid_counterbore(self):
        design = BurrDesignData(
            artifact_id="unit-actuator",
            artifact_type="actuator_mount",
        )
        with self.assertRaises(ValueError):
            counterbore(
                design,
                feature_id="bad_counterbore",
                part="housing",
                center=(0, 0, 0),
                axis=(1, 0, 0),
                role="loaded_mount",
                bore_diameter_mm=3.4,
                counterbore_diameter_mm=3.4,
                counterbore_depth_mm=4.0,
                through_depth_mm=20.0,
                create_geometry=False,
            )

    def test_records_heat_set_insert_pocket_without_build123d_geometry(self):
        design = BurrDesignData(
            artifact_id="unit-actuator",
            artifact_type="actuator_mount",
        )
        heat_set_insert_pocket(
            design,
            feature_id="m3_insert_pocket",
            part="housing",
            center=(0, 0, 10),
            axis=(1, 0, 0),
            role="threaded_mount",
            insert="M3x5.7",
            pocket_diameter_mm=4.6,
            pocket_depth_mm=5.7,
            host_depth_mm=20.0,
            create_geometry=False,
        )

        feature = design.features[0]
        self.assertEqual(feature["kind"], "heat_set_insert_pocket")
        self.assertEqual(feature["intent"], "mechanical_interface")
        self.assertEqual(feature["insert"], "M3x5.7")
        self.assertEqual(feature["pocket_diameter_mm"], 4.6)
        self.assertEqual(feature["pocket_depth_mm"], 5.7)
        self.assertEqual(feature["center_mm"], [0.0, 0.0, 10.0])
        self.assertEqual(feature["pocket_center_mm"], [-7.15, 0.0, 10.0])
        self.assertEqual(feature["bottom_center_mm"], [-4.3, 0.0, 10.0])

    def test_rejects_invalid_heat_set_insert_pocket(self):
        design = BurrDesignData(
            artifact_id="unit-actuator",
            artifact_type="actuator_mount",
        )
        with self.assertRaises(ValueError):
            heat_set_insert_pocket(
                design,
                feature_id="bad_insert_pocket",
                part="housing",
                center=(0, 0, 0),
                axis=(1, 0, 0),
                role="threaded_mount",
                insert="M3x5.7",
                pocket_diameter_mm=4.6,
                pocket_depth_mm=5.7,
                host_depth_mm=5.7,
                create_geometry=False,
            )

    def test_records_bearing_seat_without_build123d_geometry(self):
        design = BurrDesignData(
            artifact_id="unit-actuator",
            artifact_type="actuator_mount",
        )
        bearing_seat(
            design,
            feature_id="608_bearing_seat",
            part="housing",
            center=(0, 0, 10),
            axis=(1, 0, 0),
            role="bearing_support",
            bearing="608",
            seat_diameter_mm=22.0,
            seat_depth_mm=7.0,
            host_depth_mm=24.0,
            create_geometry=False,
        )

        feature = design.features[0]
        self.assertEqual(feature["kind"], "bearing_seat")
        self.assertEqual(feature["intent"], "mechanical_interface")
        self.assertEqual(feature["bearing"], "608")
        self.assertEqual(feature["seat_diameter_mm"], 22.0)
        self.assertEqual(feature["seat_depth_mm"], 7.0)
        self.assertEqual(feature["center_mm"], [0.0, 0.0, 10.0])
        self.assertEqual(feature["seat_center_mm"], [-8.5, 0.0, 10.0])
        self.assertEqual(feature["shoulder_center_mm"], [-5.0, 0.0, 10.0])

    def test_rejects_invalid_bearing_seat(self):
        design = BurrDesignData(
            artifact_id="unit-actuator",
            artifact_type="actuator_mount",
        )
        with self.assertRaises(ValueError):
            bearing_seat(
                design,
                feature_id="bad_bearing_seat",
                part="housing",
                center=(0, 0, 0),
                axis=(1, 0, 0),
                role="bearing_support",
                bearing="608",
                seat_diameter_mm=22.0,
                seat_depth_mm=7.0,
                host_depth_mm=7.0,
                create_geometry=False,
            )


if __name__ == "__main__":
    unittest.main()
