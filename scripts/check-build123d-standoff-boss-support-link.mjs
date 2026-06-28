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
    `examples/build123d-standoff-boss-support-link/${fixture}/design.py`,
  ]);
}

const bad = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-standoff-boss-support-link/bad"],
  { expectFailure: true },
);
if (!bad.output.includes("Standoff boss m3_standoff_boss is not aligned to m3_bossed_mount.")) {
  throw new Error(`Bad support-link fixture did not report the alignment problem.\n${bad.output}`);
}
if (!bad.output.includes("Measured centerline offset: 0.8 mm")) {
  throw new Error(`Bad support-link fixture did not report the measured offset.\n${bad.output}`);
}

const good = run("cargo", [
  "run",
  "--quiet",
  "--",
  "check",
  "examples/build123d-standoff-boss-support-link/good",
]);
if (
  !good.output.includes(
    "PASS examples/build123d-standoff-boss-support-link/good/burr-design-data.json",
  )
) {
  throw new Error(`Good support-link fixture did not pass.\n${good.output}`);
}

checkReceipt("examples/build123d-standoff-boss-support-link/good/burr-receipt.json", {
  status: "pass",
  linkReason: "ok",
  centerlineDistance: 0,
});
checkReceipt("examples/build123d-standoff-boss-support-link/bad/burr-receipt.json", {
  status: "fail",
  linkReason: "standoff_boss_support_mismatch",
  centerlineDistance: 0.8,
});

console.log("build123d standoff-boss support-link proof passed");

function checkReceipt(path, expected) {
  const receipt = JSON.parse(fs.readFileSync(path, "utf8"));
  expectEqual(receipt.status, expected.status, `${path} receipt status`);
  expectEqual(receipt.rulepack_version, "0.11.0", `${path} rulepack version`);

  const supportLink = receipt.checks.find(
    (item) =>
      item.rule_id === "actuator_mount:m3_standoff_boss_support_link" &&
      item.feature_id === "m3_standoff_boss",
  );
  if (!supportLink) {
    throw new Error(`${path} is missing the standoff-boss support-link check`);
  }
  expectEqual(supportLink.reason, expected.linkReason, `${path} support-link reason`);
  expectEqual(
    supportLink.related_feature_id,
    "m3_bossed_mount",
    `${path} support-link related feature`,
  );
  expectEqual(
    supportLink.measured.centerline_distance_mm,
    expected.centerlineDistance,
    `${path} support-link centerline distance`,
  );

  const bossPresence = receipt.checks.find(
    (item) =>
      item.rule_id === "actuator_mount:m3_standoff_boss_step_presence" &&
      item.feature_id === "m3_standoff_boss",
  );
  if (!bossPresence) {
    throw new Error(`${path} is missing the standoff-boss STEP-presence check`);
  }
  expectEqual(bossPresence.status, "pass", `${path} boss STEP presence status`);

  const supportWall = receipt.checks.find(
    (item) =>
      item.rule_id === "actuator_mount:m3_bossed_mount_support_wall_thickness" &&
      item.feature_id === "m3_bossed_mount",
  );
  if (!supportWall) {
    throw new Error(`${path} is missing the bossed mount support-wall check`);
  }
  expectEqual(supportWall.status, "pass", `${path} support-wall status`);
  expectEqual(
    supportWall.measured.support_wall_thickness_mm,
    2.3,
    `${path} support-wall thickness`,
  );
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`Unexpected ${label}: got ${actual}, expected ${expected}`);
  }
}
