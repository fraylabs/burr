#!/usr/bin/env node
import fs from "node:fs";
import { spawnSync } from "node:child_process";

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    env: { ...process.env, ...(options.env ?? {}) },
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

for (const fixture of ["bad", "good"]) {
  run("uv", [
    "run",
    "--package",
    "burr-build123d",
    "python",
    `examples/build123d-standoff-boss-edge-distance/${fixture}/design.py`,
  ]);
}

const bad = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-standoff-boss-edge-distance/bad"],
  { expectFailure: true },
);
if (!bad.output.includes("Feature m3_standoff_boss is too close to the edge.")) {
  throw new Error(`Bad standoff-boss edge fixture did not report edge material.\n${bad.output}`);
}
if (!bad.output.includes("Measured feature-to-edge: 2.5 mm")) {
  throw new Error(`Bad standoff-boss edge fixture did not report measured edge material.\n${bad.output}`);
}
if (!bad.output.includes("Short by: 0.5 mm")) {
  throw new Error(`Bad standoff-boss edge fixture did not report expected shortage.\n${bad.output}`);
}

const good = run("cargo", [
  "run",
  "--quiet",
  "--",
  "check",
  "examples/build123d-standoff-boss-edge-distance/good",
]);
if (!good.output.includes("PASS examples/build123d-standoff-boss-edge-distance/good/burr-design-data.json")) {
  throw new Error(`Good standoff-boss edge fixture did not pass.\n${good.output}`);
}

checkReceipt("examples/build123d-standoff-boss-edge-distance/good/burr-receipt.json", {
  status: "pass",
  edgeStatus: "pass",
  edgeReason: "ok",
  wallToEdge: 12,
  margin: 9,
});
checkReceipt("examples/build123d-standoff-boss-edge-distance/bad/burr-receipt.json", {
  status: "fail",
  edgeStatus: "fail",
  edgeReason: "insufficient_feature_edge_distance",
  wallToEdge: 2.5,
  margin: -0.5,
});

console.log("build123d standoff-boss edge-distance proof passed");

function checkReceipt(path, expected) {
  const receipt = JSON.parse(fs.readFileSync(path, "utf8"));
  expectEqual(receipt.status, expected.status, `${path} receipt status`);
  expectEqual(receipt.rulepack_version, "0.14.0", `${path} rulepack version`);
  const edgeCheck = receipt.checks.find(
    (check) =>
      check.rule_id === "actuator_mount:m3_standoff_boss_edge_distance" &&
      check.feature_id === "m3_standoff_boss",
  );
  if (!edgeCheck) {
    throw new Error(`${path} is missing the standoff-boss edge-distance check`);
  }
  expectEqual(edgeCheck.status, expected.edgeStatus, `${path} edge status`);
  expectEqual(edgeCheck.reason, expected.edgeReason, `${path} edge reason`);
  expectEqual(edgeCheck.measured.wall_to_edge_mm, expected.wallToEdge, `${path} wall-to-edge`);
  expectEqual(edgeCheck.required.center_field, "boss_center_mm", `${path} center field`);
  expectEqual(edgeCheck.required.diameter_field, "boss_diameter_mm", `${path} diameter field`);
  expectEqual(edgeCheck.required.min_wall_to_edge_mm, 3, `${path} required wall-to-edge`);
  expectEqual(edgeCheck.margin_mm, expected.margin, `${path} edge margin`);
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`Unexpected ${label}: got ${actual}, expected ${expected}`);
  }
}
