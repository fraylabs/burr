#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"
import { spawnSync } from "node:child_process"

const examples = [
  {
    dir: "examples/gallery/shaft-bearing-bracket",
    artifact: "shaft-bearing-bracket.step",
    checked: [
      "bearing_608_primary",
      "m3_loaded_mount_left",
      "m3_loaded_mount_right",
    ],
    unchecked: ["cosmetic_relief_recess", "wire_passage"],
  },
  {
    dir: "examples/gallery/slotted-motor-plate",
    artifact: "slotted-motor-plate.step",
    checked: [
      "motor_tension_slot",
      "m3_socket_mount_left",
      "m3_socket_mount_right",
    ],
    unchecked: ["cosmetic_alignment_mark"],
  },
  {
    dir: "examples/gallery/electronics-standoff-deck",
    artifact: "electronics-standoff-deck.step",
    checked: [
      "m3_insert_socket_left",
      "m3_insert_socket_right",
      "m3_base_mount_left",
      "m3_base_mount_right",
    ],
    unchecked: ["cosmetic_label_socket"],
  },
]

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
    ...options,
  })
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n")
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit ${result.status}\n${output}`)
  }
  return { ...result, output }
}

for (const example of examples) {
  run("uv", [
    "run",
    "--package",
    "burr-build123d",
    "python",
    path.join(example.dir, "design.py"),
  ])

  run("cargo", ["run", "--quiet", "--", "check", example.dir])

  const dataPath = path.join(example.dir, "burr-design-data.json")
  const stepPath = path.join(example.dir, example.artifact)
  const receiptPath = path.join(example.dir, "burr-receipt.json")

  assertFile(dataPath)
  assertFile(stepPath)
  assertFile(receiptPath)

  const receipt = JSON.parse(fs.readFileSync(receiptPath, "utf8"))
  assertEqual(receipt.status, "pass", `${example.dir} receipt status`)
  assertEqual(receipt.rulepack_version, "0.8.0", `${example.dir} rulepack version`)
  assertEqual(receipt.summary.failures, 0, `${example.dir} failure count`)

  const checkedFeatureIds = new Set(receipt.summary.features.checked_feature_ids)
  const uncheckedFeatureIds = new Set(receipt.summary.features.unchecked_feature_ids)
  for (const featureId of example.checked) {
    if (!checkedFeatureIds.has(featureId)) {
      throw new Error(`${example.dir} did not check feature ${featureId}`)
    }
    const okCheck = receipt.checks.find(
      (check) => check.feature_id === featureId && check.reason === "ok" && check.status === "pass",
    )
    if (!okCheck) {
      throw new Error(`${example.dir} is missing a passing receipt check for ${featureId}`)
    }
  }
  for (const featureId of example.unchecked) {
    if (!uncheckedFeatureIds.has(featureId)) {
      throw new Error(`${example.dir} did not leave ${featureId} unchecked`)
    }
    if (checkedFeatureIds.has(featureId)) {
      throw new Error(`${example.dir} incorrectly linted unchecked feature ${featureId}`)
    }
  }

  const designData = JSON.parse(fs.readFileSync(dataPath, "utf8"))
  const stepRef = designData.artifacts.find((artifact) => artifact.path === example.artifact)
  if (!stepRef?.sha256 || !stepRef?.size_bytes) {
    throw new Error(`${dataPath} did not stamp ${example.artifact}`)
  }
}

console.log("gallery examples passed")

function assertFile(filePath) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`Expected file to exist: ${filePath}`)
  }
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`Unexpected ${label}: got ${actual}, expected ${expected}`)
  }
}
