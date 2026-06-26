#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const temp = fs.mkdtempSync(path.join(os.tmpdir(), "burr-breadth-rules-"));
const repoRoot = process.cwd();
const capturedSliderRulepackPath = path.join(repoRoot, "rules", "captured_slider.rulepack.json");
const printedPlateRulepackPath = path.join(repoRoot, "rules", "printed_plate.rulepack.json");

try {
  writeFixture("captured-slider-good", {
    head_side_clearance_mm: 0.25,
    neck_side_clearance_mm: 0.25,
    carriage_lip_each_side_mm: 3.5,
    captureLipCount: 2,
    expect: "pass",
  });
  writeFixture("captured-slider-loose", {
    head_side_clearance_mm: 0.5,
    neck_side_clearance_mm: 0.25,
    carriage_lip_each_side_mm: 3.5,
    captureLipCount: 2,
    expect: "fail",
    reason: "numeric_value_out_of_range",
  });
  writeFixture("captured-slider-missing-lip", {
    head_side_clearance_mm: 0.25,
    neck_side_clearance_mm: 0.25,
    carriage_lip_each_side_mm: 3.5,
    captureLipCount: 1,
    expect: "fail",
    reason: "feature_count_out_of_range",
  });
  writePrintedPlateFixture("printed-plate-relief-spacing-good", {
    holes: [
      ["relief_a1", -32, -16, 3.0],
      ["relief_a2", -20, -16, 3.0],
      ["relief_a3", -8, -16, 3.0],
      ["relief_a4", 8, -16, 3.0],
      ["relief_a5", 20, -16, 3.0],
      ["relief_a6", 32, -16, 3.0],
      ["relief_b1", -24, 12, 3.2],
      ["relief_b2", 24, 12, 3.2],
    ],
    expect: "pass",
  });
  writePrintedPlateFixture("printed-plate-relief-spacing-tight", {
    holes: [
      ["relief_a1", -32, -16, 3.0],
      ["relief_a2", -28.6, -16, 3.0],
      ["relief_a3", -8, -16, 3.0],
      ["relief_a4", 8, -16, 3.0],
      ["relief_a5", 20, -16, 3.0],
      ["relief_a6", 32, -16, 3.0],
      ["relief_b1", -24, 12, 3.2],
      ["relief_b2", 24, 12, 3.2],
    ],
    expect: "fail",
    reason: "insufficient_feature_pair_spacing",
  });
  writePrintedPlateFixture("printed-plate-hole-slot-spacing-good", {
    holes: [
      ["nearby_cutout", 0, 6.2, 4.0],
      ["relief_a2", -40, 30, 3.0],
      ["relief_a3", -28, 30, 3.0],
      ["relief_a4", -16, 30, 3.0],
      ["relief_a5", -4, 30, 3.0],
      ["relief_a6", 16, 30, 3.0],
      ["relief_a7", 28, 30, 3.0],
      ["relief_a8", 40, 30, 3.0],
    ],
    slots: [["relief_slot", 0, 0, 5.0, 30.0, [0, 1, 0]]],
    expect: "pass",
    expectedPair: {
      featureIds: ["nearby_cutout", "relief_slot"],
      featureShapes: ["circle", "capsule"],
      shapeDistanceMm: 6.2,
      clearanceMm: 1.7,
      marginMm: 0.5,
    },
  });
  writePrintedPlateFixture("printed-plate-hole-slot-spacing-tight", {
    holes: [
      ["nearby_cutout", 0, 5.0, 4.0],
      ["relief_a2", -40, 30, 3.0],
      ["relief_a3", -28, 30, 3.0],
      ["relief_a4", -16, 30, 3.0],
      ["relief_a5", -4, 30, 3.0],
      ["relief_a6", 16, 30, 3.0],
      ["relief_a7", 28, 30, 3.0],
      ["relief_a8", 40, 30, 3.0],
    ],
    slots: [["relief_slot", 0, 0, 5.0, 30.0, [0, 1, 0]]],
    expect: "fail",
    reason: "insufficient_feature_pair_spacing",
    expectedPair: {
      featureIds: ["nearby_cutout", "relief_slot"],
      featureShapes: ["circle", "capsule"],
      shapeDistanceMm: 5.0,
      clearanceMm: 0.5,
      marginMm: -0.7,
    },
  });
  console.log("breadth rule fixtures passed");
} finally {
  fs.rmSync(temp, { recursive: true, force: true });
}

function writeFixture(slug, options) {
  const dir = path.join(temp, slug);
  fs.mkdirSync(dir, { recursive: true });
  const sourcePath = path.join(dir, "design.py");
  const stepPath = path.join(dir, "part.step");
  fs.writeFileSync(sourcePath, "print('fixture')\n");
  fs.writeFileSync(stepPath, "ISO-10303-21;\nEND-ISO-10303-21;\n");

  const features = Array.from({ length: options.captureLipCount }, (_, index) => ({
    id: `${index === 0 ? "left" : "right"}_capture_lip`,
    part: "carriage",
    kind: "capture_lip",
    intent: "mechanical_interface",
    role: "lift_off_blocker",
  }));

  fs.writeFileSync(
    path.join(dir, "burr-design-data.json"),
    JSON.stringify(
      {
        schema_version: "burr.design-data.v1",
        artifact_id: slug,
        artifact_version: "0.1.0",
        artifact_type: "captured_slider",
        units: "mm",
        rulepack: { path: capturedSliderRulepackPath },
        source: {
          path: "design.py",
          sha256: sha256(sourcePath),
          size_bytes: fs.statSync(sourcePath).size,
        },
        artifacts: [
          {
            kind: "step",
            path: "part.step",
            sha256: sha256(stepPath),
            size_bytes: fs.statSync(stepPath).size,
          },
        ],
        measurements: {
          head_side_clearance_mm: options.head_side_clearance_mm,
          neck_side_clearance_mm: options.neck_side_clearance_mm,
          carriage_lip_each_side_mm: options.carriage_lip_each_side_mm,
        },
        features,
      },
      null,
      2,
    ) + "\n",
  );

  const result = spawnSync("cargo", ["run", "--quiet", "--", "check", dir], {
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
  });
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
  if (options.expect === "pass" && result.status !== 0) {
    throw new Error(`${slug} should pass\n${output}`);
  }
  if (options.expect === "fail" && result.status === 0) {
    throw new Error(`${slug} should fail\n${output}`);
  }

  const receipt = JSON.parse(fs.readFileSync(path.join(dir, "burr-receipt.json"), "utf8"));
  if (receipt.status !== options.expect) {
    throw new Error(`${slug} receipt status ${receipt.status}, expected ${options.expect}`);
  }
  if (options.reason && !receipt.checks.some((check) => check.reason === options.reason)) {
    throw new Error(`${slug} did not report ${options.reason}`);
  }
}

function writePrintedPlateFixture(slug, options) {
  const dir = path.join(temp, slug);
  fs.mkdirSync(dir, { recursive: true });
  const sourcePath = path.join(dir, "design.py");
  const stepPath = path.join(dir, "plate.step");
  fs.writeFileSync(sourcePath, "print('fixture')\n");
  fs.writeFileSync(stepPath, "ISO-10303-21;\nEND-ISO-10303-21;\n");

  const holeFeatures = options.holes.map(([id, y, z, diameter]) => ({
    id,
    part: "plate",
    kind: "clearance_hole",
    intent: "cosmetic",
    fastener: "none",
    diameter_mm: diameter,
    center_mm: [0, y, z],
    axis: [1, 0, 0],
    role: "visual_lightening",
  }));
  const slotFeatures = (options.slots ?? []).map(([id, y, z, width, length, spanAxis]) => ({
    id,
    part: "plate",
    kind: "straight_slot",
    intent: "cosmetic",
    width_mm: width,
    length_mm: length,
    center_mm: [0, y, z],
    axis: [1, 0, 0],
    span_axis: spanAxis,
    role: "relief_slot",
  }));
  const features = [...holeFeatures, ...slotFeatures];

  fs.writeFileSync(
    path.join(dir, "burr-design-data.json"),
    JSON.stringify(
      {
        schema_version: "burr.design-data.v1",
        artifact_id: slug,
        artifact_version: "0.1.0",
        artifact_type: "printed_plate",
        units: "mm",
        rulepack: { path: printedPlateRulepackPath },
        source: {
          path: "design.py",
          sha256: sha256(sourcePath),
          size_bytes: fs.statSync(sourcePath).size,
        },
        artifacts: [
          {
            kind: "step",
            path: "plate.step",
            sha256: sha256(stepPath),
            size_bytes: fs.statSync(stepPath).size,
          },
        ],
        features,
      },
      null,
      2,
    ) + "\n",
  );

  const result = spawnSync("cargo", ["run", "--quiet", "--", "check", dir], {
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
  });
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
  if (options.expect === "pass" && result.status !== 0) {
    throw new Error(`${slug} should pass\n${output}`);
  }
  if (options.expect === "fail" && result.status === 0) {
    throw new Error(`${slug} should fail\n${output}`);
  }

  const receipt = JSON.parse(fs.readFileSync(path.join(dir, "burr-receipt.json"), "utf8"));
  if (receipt.status !== options.expect) {
    throw new Error(`${slug} receipt status ${receipt.status}, expected ${options.expect}`);
  }
  if (options.reason && !receipt.checks.some((check) => check.reason === options.reason)) {
    throw new Error(`${slug} did not report ${options.reason}`);
  }
  if (options.expectedPair) {
    const check = receipt.checks.find((check) => check.rule_id === "printed_plate:cosmetic_relief_ligament");
    if (!check) {
      throw new Error(`${slug} did not write printed_plate:cosmetic_relief_ligament`);
    }
    expectArrayEqual(check.measured.closest_pair.feature_ids, options.expectedPair.featureIds, `${slug} closest pair ids`);
    expectArrayEqual(
      check.measured.closest_pair.feature_shapes,
      options.expectedPair.featureShapes,
      `${slug} closest pair shapes`,
    );
    expectEqual(
      check.measured.closest_pair.shape_distance_mm,
      options.expectedPair.shapeDistanceMm,
      `${slug} shape distance`,
    );
    expectEqual(check.measured.closest_pair.clearance_mm, options.expectedPair.clearanceMm, `${slug} clearance`);
    expectEqual(check.margin_mm, options.expectedPair.marginMm, `${slug} margin`);
    if (options.expect === "fail") {
      expectArrayEqual(
        check.measured.violating_pairs[0].feature_ids,
        options.expectedPair.featureIds,
        `${slug} violating pair ids`,
      );
    }
  }
}

function sha256(filePath) {
  return crypto.createHash("sha256").update(fs.readFileSync(filePath)).digest("hex");
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: got ${actual}, expected ${expected}`);
  }
}

function expectArrayEqual(actual, expected, label) {
  if (!Array.isArray(actual) || actual.length !== expected.length || actual.some((value, index) => value !== expected[index])) {
    throw new Error(`${label}: got ${JSON.stringify(actual)}, expected ${JSON.stringify(expected)}`);
  }
}
