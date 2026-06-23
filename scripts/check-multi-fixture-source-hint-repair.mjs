#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const designDataFile = "burr-design-data.json";
const receiptFile = "burr-receipt.json";

const fixtures = [
  {
    id: "printed-plate-cosmetic-relief-count",
    artifactType: "printed_plate",
    beforeDir: "examples/gallery/dense-random-hole-plate-too-few-reliefs",
    afterDir: "examples/gallery/dense-random-hole-plate",
    expectedBeforeStatus: "fail",
    expectedAfterStatus: "pass",
    expectedRepairedStatus: "pass",
    repairedStepFile: "dense-random-hole-plate-too-few-reliefs.step",
    focusRuleId: "printed_plate:cosmetic_relief_inventory",
    sourceHints: [
      {
        edit_kind: "replace_python_list",
        selector: "cosmetic_holes",
        before_text: `cosmetic_holes = [
    ("cosmetic_grid_a1", -36.0, -20.0, 2.6),
    ("cosmetic_grid_a2", -24.0, -20.0, 2.6),
    ("cosmetic_grid_a3", -12.0, -20.0, 2.6),
    ("cosmetic_grid_a4", 12.0, -20.0, 2.6),
    ("cosmetic_grid_a5", 24.0, -20.0, 2.6),
    ("cosmetic_grid_a6", 36.0, -20.0, 2.6),
    ("cosmetic_grid_b1", -30.0, 5.0, 3.2),
]`,
        after_text: `cosmetic_holes = [
    ("cosmetic_grid_a1", -36.0, -20.0, 2.6),
    ("cosmetic_grid_a2", -24.0, -20.0, 2.6),
    ("cosmetic_grid_a3", -12.0, -20.0, 2.6),
    ("cosmetic_grid_a4", 12.0, -20.0, 2.6),
    ("cosmetic_grid_a5", 24.0, -20.0, 2.6),
    ("cosmetic_grid_a6", 36.0, -20.0, 2.6),
    ("cosmetic_grid_b1", -30.0, 5.0, 3.2),
    ("cosmetic_grid_b2", -18.0, 5.0, 3.2),
    ("cosmetic_grid_b3", 18.0, 5.0, 3.2),
    ("cosmetic_grid_b4", 30.0, 5.0, 3.2),
]`,
      },
    ],
    repairedAssertions({ receipt }) {
      const check = expectCheck(receipt, "printed_plate:cosmetic_relief_inventory");
      expectEqual(check.status, "pass", "printed plate repaired check status");
      expectEqual(check.measured?.count, 10, "printed plate repaired cosmetic hole count");
    },
  },
  {
    id: "captured-slider-clearance-window",
    artifactType: "captured_slider",
    beforeDir: "examples/gallery/t-slot-linear-slider-loose-clearance",
    afterDir: "examples/gallery/t-slot-linear-slider",
    expectedBeforeStatus: "fail",
    expectedAfterStatus: "pass",
    expectedRepairedStatus: "pass",
    repairedStepFile: "t-slot-linear-slider-loose-clearance.step",
    focusRuleId: "captured_slider:head_side_clearance_window",
    sourceHints: [
      {
        edit_kind: "replace_python_assignments",
        selector: "clearance_variables",
        before_text: `head_clearance = 0.5
neck_clearance = 0.25`,
        after_text: `clearance = 0.25`,
      },
      {
        edit_kind: "replace_python_dict_entries",
        selector: "measurements_update.clearance",
        before_text: `        "head_side_clearance_mm": head_clearance,
        "neck_side_clearance_mm": neck_clearance,`,
        after_text: `        "head_side_clearance_mm": clearance,
        "neck_side_clearance_mm": clearance,`,
      },
      {
        edit_kind: "replace_python_call",
        selector: "neck_channel_box",
        before_text: `        Box(
            rail_neck_width + 2.0 * neck_clearance,
            carriage_length + 2.0,
            rail_neck_height + neck_clearance,
            mode=Mode.SUBTRACT,
        )`,
        after_text: `        Box(
            rail_neck_width + 2.0 * clearance,
            carriage_length + 2.0,
            rail_neck_height + clearance,
            mode=Mode.SUBTRACT,
        )`,
      },
      {
        edit_kind: "replace_python_call",
        selector: "head_channel_box",
        before_text: `        Box(
            rail_head_width + 2.0 * head_clearance,
            carriage_length + 2.0,
            rail_head_height + 2.0 * head_clearance,
            mode=Mode.SUBTRACT,
        )`,
        after_text: `        Box(
            rail_head_width + 2.0 * clearance,
            carriage_length + 2.0,
            rail_head_height + 2.0 * clearance,
            mode=Mode.SUBTRACT,
        )`,
      },
    ],
    repairedAssertions({ receipt, designData }) {
      const head = expectCheck(receipt, "captured_slider:head_side_clearance_window");
      const neck = expectCheck(receipt, "captured_slider:neck_side_clearance_window");
      expectEqual(head.status, "pass", "captured slider repaired head clearance status");
      expectEqual(neck.status, "pass", "captured slider repaired neck clearance status");
      expectEqual(
        designData.measurements?.head_side_clearance_mm,
        0.25,
        "captured slider repaired head clearance measurement",
      );
      expectEqual(
        designData.measurements?.neck_side_clearance_mm,
        0.25,
        "captured slider repaired neck clearance measurement",
      );
    },
  },
];

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "burr-multi-fixture-source-hint-repair-"));

try {
  for (const fixture of fixtures) {
    proveFixture(fixture);
  }

  console.log(
    `multi-fixture source_hint repair proof passed for ${fixtures
      .map((fixture) => `${fixture.id} (${fixture.artifactType})`)
      .join(", ")}`,
  );
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}

function proveFixture(fixture) {
  const beforeSourcePath = path.join(fixture.beforeDir, "design.py");
  const afterSourcePath = path.join(fixture.afterDir, "design.py");
  const beforeSource = readText(beforeSourcePath);
  const afterSource = readText(afterSourcePath);

  expectExactSourceHints({
    fixture,
    beforeSource,
    afterSource,
    beforeSourcePath,
    afterSourcePath,
  });

  const beforeDir = copyFixtureSource({ fixtureDir: fixture.beforeDir, label: "before" });
  const afterDir = copyFixtureSource({ fixtureDir: fixture.afterDir, label: "after" });
  const repairedDir = copyFixtureSource({ fixtureDir: fixture.beforeDir, label: "repaired" });

  runGenerator(beforeDir);
  const beforeCheck = runBurrCheck(beforeDir, { expectFailure: true });
  expectIncludes(beforeCheck.output, "FAIL", `${fixture.id} before Burr output`);
  const beforeReceipt = readJson(path.join(beforeDir, receiptFile));
  expectEqual(beforeReceipt.status, fixture.expectedBeforeStatus, `${fixture.id} before receipt status`);
  expectEqual(beforeReceipt.artifact_type, fixture.artifactType, `${fixture.id} before artifact type`);
  expectCheck(beforeReceipt, fixture.focusRuleId, { status: "fail" });

  runGenerator(afterDir);
  const afterCheck = runBurrCheck(afterDir);
  expectIncludes(afterCheck.output, "PASS", `${fixture.id} after Burr output`);
  const afterReceipt = readJson(path.join(afterDir, receiptFile));
  expectEqual(afterReceipt.status, fixture.expectedAfterStatus, `${fixture.id} after receipt status`);
  expectEqual(afterReceipt.artifact_type, fixture.artifactType, `${fixture.id} after artifact type`);
  expectCheck(afterReceipt, fixture.focusRuleId, { status: "pass" });

  const repairedSourcePath = path.join(repairedDir, "design.py");
  const repairedSource = applySourceHints(beforeSource, fixture.sourceHints, fixture.id);
  if (repairedSource === beforeSource) {
    throw new Error(`${fixture.id}: source hints did not modify the before source`);
  }
  fs.writeFileSync(repairedSourcePath, repairedSource);

  runGenerator(repairedDir);
  const repairedCheck = runBurrCheck(repairedDir);
  expectIncludes(repairedCheck.output, "PASS", `${fixture.id} repaired Burr output`);
  const repairedReceipt = readJson(path.join(repairedDir, receiptFile));
  const repairedDesignData = readJson(path.join(repairedDir, designDataFile));
  expectEqual(repairedReceipt.status, fixture.expectedRepairedStatus, `${fixture.id} repaired receipt status`);
  expectEqual(repairedReceipt.artifact_type, fixture.artifactType, `${fixture.id} repaired artifact type`);
  expectFile(path.join(repairedDir, fixture.repairedStepFile), `${fixture.id} repaired STEP artifact`);
  expectPassingFileHashes(repairedReceipt);
  fixture.repairedAssertions({ receipt: repairedReceipt, designData: repairedDesignData });
}

function expectExactSourceHints({
  fixture,
  beforeSource,
  afterSource,
  beforeSourcePath,
  afterSourcePath,
}) {
  for (const hint of fixture.sourceHints) {
    const label = `${fixture.id} ${hint.selector}`;
    expectString(hint.before_text, `${label} before_text`);
    expectString(hint.after_text, `${label} after_text`);
    replaceOnce(beforeSource, hint.before_text, hint.after_text, `${label} before_text in ${beforeSourcePath}`);
    expectOccursOnce(afterSource, hint.after_text, `${label} after_text in ${afterSourcePath}`);
  }
}

function applySourceHints(source, hints, fixtureId) {
  return hints.reduce(
    (current, hint) =>
      replaceOnce(
        current,
        hint.before_text,
        hint.after_text,
        `${fixtureId} source_hint ${hint.selector}`,
      ),
    source,
  );
}

function copyFixtureSource({ fixtureDir, label }) {
  const targetRoot = path.join(tmp, label);
  const targetDir = path.join(targetRoot, fixtureDir);
  copyRequired("rules/printed_plate.rulepack.json", path.join(targetRoot, "rules/printed_plate.rulepack.json"));
  copyRequired("rules/captured_slider.rulepack.json", path.join(targetRoot, "rules/captured_slider.rulepack.json"));
  fs.mkdirSync(targetDir, { recursive: true });
  copyRequired(path.join(fixtureDir, "design.py"), path.join(targetDir, "design.py"));
  return targetDir;
}

function runGenerator(dir) {
  run("uv", ["run", "--package", "burr-build123d", "python", path.join(dir, "design.py")]);
  expectFile(path.join(dir, designDataFile), "generated design data");
}

function runBurrCheck(dir, options = {}) {
  const result = run("cargo", ["run", "--quiet", "--", "check", dir], options);
  expectFile(path.join(dir, receiptFile), "Burr receipt");
  return result;
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
  });
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
  if (options.expectFailure) {
    if (result.status === 0) {
      throw new Error(`${command} ${args.join(" ")} unexpectedly passed\n${output}`);
    }
  } else if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit ${result.status}\n${output}`);
  }
  return { ...result, output };
}

function expectPassingFileHashes(receipt) {
  for (const ruleId of [
    "burr_design_data:source_sha256_matches",
    "burr_design_data:artifact_sha256_matches",
  ]) {
    expectCheck(receipt, ruleId, { status: "pass" });
  }
}

function expectCheck(receipt, ruleId, options = {}) {
  const check = receipt.checks.find((item) => item.rule_id === ruleId);
  if (!check) {
    throw new Error(`Missing check ${ruleId}`);
  }
  if (options.status !== undefined) {
    expectEqual(check.status, options.status, `${ruleId} status`);
  }
  return check;
}

function replaceOnce(source, before, after, label) {
  const first = source.indexOf(before);
  if (first < 0) {
    throw new Error(`Could not find ${label}`);
  }
  const second = source.indexOf(before, first + before.length);
  if (second >= 0) {
    throw new Error(`Expected one ${label}, found multiple`);
  }
  return `${source.slice(0, first)}${after}${source.slice(first + before.length)}`;
}

function expectOccursOnce(source, needle, label) {
  const first = source.indexOf(needle);
  if (first < 0) {
    throw new Error(`Could not find ${label}`);
  }
  const second = source.indexOf(needle, first + needle.length);
  if (second >= 0) {
    throw new Error(`Expected one ${label}, found multiple`);
  }
}

function copyRequired(source, target) {
  if (!fs.existsSync(source)) {
    throw new Error(`Missing required file: ${source}`);
  }
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(source, target);
}

function readJson(file) {
  return JSON.parse(readText(file));
}

function readText(file) {
  if (!fs.existsSync(file)) {
    throw new Error(`Missing file: ${file}`);
  }
  return fs.readFileSync(file, "utf8");
}

function expectFile(file, label) {
  if (!fs.existsSync(file)) {
    throw new Error(`Missing ${label}: ${file}`);
  }
  if (fs.statSync(file).size === 0) {
    throw new Error(`${label} is empty: ${file}`);
  }
}

function expectString(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`Expected non-empty string for ${label}`);
  }
}

function expectIncludes(value, needle, label) {
  if (!value.includes(needle)) {
    throw new Error(`${label} did not include ${JSON.stringify(needle)}\n${value}`);
  }
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}
