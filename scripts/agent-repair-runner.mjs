#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const designDataFile = "burr-design-data.json";
const receiptFile = "burr-receipt.json";
const runnerReceiptFile = "burr-agent-repair-receipt.json";

const options = parseArgs(process.argv.slice(2));
const targetDir = path.resolve(options.targetDir);
const receiptPath = path.join(targetDir, options.receipt ?? runnerReceiptFile);

try {
  const result = runRepairLoop({ targetDir, options });
  fs.writeFileSync(receiptPath, `${JSON.stringify(result, null, 2)}\n`);
  console.log(JSON.stringify(result, null, 2));
} catch (error) {
  const result = {
    schema_version: "burr.agent-repair-receipt.v1",
    status: "runner_error",
    target_dir: targetDir,
    reason: error instanceof Error ? error.message : String(error),
  };
  fs.writeFileSync(receiptPath, `${JSON.stringify(result, null, 2)}\n`);
  console.error(JSON.stringify(result, null, 2));
  process.exit(2);
}

function runRepairLoop({ targetDir, options }) {
  assertDirectory(targetDir);
  runGenerator(targetDir);

  const firstCheck = runBurrCheck(targetDir, { allowFailure: true });
  const beforeReceipt = readOptionalJson(path.join(targetDir, receiptFile));
  const beforeDesignData = readOptionalJson(path.join(targetDir, designDataFile));
  if (firstCheck.status === 0) {
    return baseReceipt({
      status: "already_passed",
      targetDir,
      beforeReceipt,
      repairPacket: null,
      appliedEdits: [],
      finalReceipt: beforeReceipt,
    });
  }

  const repairPacket = options.packet
    ? readJson(path.resolve(options.packet))
    : runExplainJson(targetDir);
  const exactActions = exactRepairActions(repairPacket);

  if (exactActions.length === 0) {
    return baseReceipt({
      status: "not_repairable",
      targetDir,
      beforeReceipt,
      repairPacket,
      appliedEdits: [],
      reason:
        "Burr explained the failure, but the packet did not contain exact source_hint.before_text/after_text edits with confidence exact_from_design_data.",
    });
  }

  const appliedEdits = applyExactActions({
    targetDir,
    designData: beforeDesignData,
    sourceFile: options.sourceFile,
    actions: exactActions,
  });

  runGenerator(targetDir);
  const repairedCheck = runBurrCheck(targetDir, { allowFailure: true });
  const finalReceipt = readOptionalJson(path.join(targetDir, receiptFile));
  const repaired = repairedCheck.status === 0 && finalReceipt?.status === "pass";

  return baseReceipt({
    status: repaired ? "repaired" : "repair_failed",
    targetDir,
    beforeReceipt,
    repairPacket,
    appliedEdits,
    finalReceipt,
    reason: repaired
      ? undefined
      : "Exact edits were applied, but the regenerated Burr check did not pass.",
  });
}

function baseReceipt({
  status,
  targetDir,
  beforeReceipt,
  repairPacket,
  appliedEdits,
  finalReceipt,
  reason,
}) {
  return {
    schema_version: "burr.agent-repair-receipt.v1",
    status,
    target_dir: targetDir,
    reason,
    before: {
      status: beforeReceipt?.status ?? null,
      receipt: path.join(targetDir, receiptFile),
      failures: beforeReceipt?.summary?.failures ?? null,
    },
    repair_packet: repairPacket
      ? {
          schema_version: repairPacket.schema_version,
          source_kind: repairPacket.source_kind,
          exact_source_edit_count:
            repairPacket.summary?.exact_source_edit_count ?? 0,
          exact_source_edits_available:
            repairPacket.summary?.exact_source_edits_available ?? false,
        }
      : null,
    applied_edits: appliedEdits,
    after: finalReceipt
      ? {
          status: finalReceipt.status,
          receipt: path.join(targetDir, receiptFile),
          failures: finalReceipt.summary?.failures ?? null,
        }
      : null,
  };
}

function exactRepairActions(packet) {
  const actions = Array.isArray(packet?.repair_actions) ? packet.repair_actions : [];
  return actions.filter((action) => {
    const hint = action?.source_hint;
    return (
      hint &&
      typeof hint === "object" &&
      typeof hint.before_text === "string" &&
      typeof hint.after_text === "string" &&
      hint.confidence === "exact_from_design_data"
    );
  });
}

function applyExactActions({ targetDir, designData, sourceFile, actions }) {
  const editsByFile = new Map();
  for (const action of actions) {
    const hint = action.source_hint;
    assertHintBeforeValue({ designData, hint, label: `${action.action}:${action.feature_id ?? hint.selector}` });
    const editPath = resolveSourcePath({
      targetDir,
      sourceFile,
      sourceFilePath: hint.source_file_path,
    });
    const existing = editsByFile.get(editPath) ?? {
      source: fs.readFileSync(editPath, "utf8"),
      edits: [],
    };
    existing.source = replaceOnce(
      existing.source,
      hint.before_text,
      hint.after_text,
      `${action.action}:${action.feature_id ?? hint.selector}`,
    );
    existing.edits.push({
      action: action.action,
      feature_id: action.feature_id ?? null,
      selector: hint.selector ?? null,
      source_file: path.relative(targetDir, editPath),
      before_text: hint.before_text,
      after_text: hint.after_text,
    });
    editsByFile.set(editPath, existing);
  }

  const edits = [];
  for (const [editPath, pending] of editsByFile) {
    fs.writeFileSync(editPath, pending.source);
    edits.push(...pending.edits);
  }
  return edits;
}

function assertHintBeforeValue({ designData, hint, label }) {
  if (!hint.value_path || !Object.hasOwn(hint, "before_value_mm")) {
    return;
  }
  if (!designData) {
    throw new Error(`Cannot validate source_hint value_path for ${label}: missing ${designDataFile}`);
  }

  const actual = valueAtPath(designData, hint.value_path);
  if (!sameJsonValue(actual, hint.before_value_mm)) {
    throw new Error(
      `source_hint before_value_mm does not match ${hint.value_path} for ${label}: got ${JSON.stringify(actual)}, expected ${JSON.stringify(hint.before_value_mm)}`,
    );
  }
}

function valueAtPath(designData, valuePath) {
  const featureMatch = /^features\[id=([^\]]+)\]\.([A-Za-z0-9_]+)$/.exec(valuePath);
  if (featureMatch) {
    const feature = designData.features?.find((item) => item.id === featureMatch[1]);
    if (!feature || !(featureMatch[2] in feature)) {
      throw new Error(`Missing design-data value_path: ${valuePath}`);
    }
    return feature[featureMatch[2]];
  }

  const partSizeMatch = /^parts\[id=([^\]]+)\]\.bbox_mm\.size\[(\d+)\]$/.exec(valuePath);
  if (partSizeMatch) {
    const part = designData.parts?.find((item) => item.id === partSizeMatch[1]);
    const axis = Number(partSizeMatch[2]);
    if (!part?.bbox_mm?.min || !part?.bbox_mm?.max || !Number.isInteger(axis)) {
      throw new Error(`Missing design-data value_path: ${valuePath}`);
    }
    return roundMm(part.bbox_mm.max[axis] - part.bbox_mm.min[axis]);
  }

  throw new Error(`Unsupported source_hint value_path: ${valuePath}`);
}

function sameJsonValue(actual, expected) {
  return JSON.stringify(roundJsonValue(actual)) === JSON.stringify(roundJsonValue(expected));
}

function roundJsonValue(value) {
  if (typeof value === "number") {
    return roundMm(value);
  }
  if (Array.isArray(value)) {
    return value.map(roundJsonValue);
  }
  return value;
}

function roundMm(value) {
  return Math.round(value * 1000) / 1000;
}

function resolveSourcePath({ targetDir, sourceFile, sourceFilePath }) {
  const candidates = [];
  if (sourceFile) {
    candidates.push(path.resolve(targetDir, sourceFile));
  }
  if (sourceFilePath) {
    candidates.push(path.resolve(targetDir, sourceFilePath));
    candidates.push(path.resolve(targetDir, path.basename(sourceFilePath)));
  }

  for (const candidate of candidates) {
    if (candidate.startsWith(`${targetDir}${path.sep}`) && fs.existsSync(candidate)) {
      return candidate;
    }
  }

  throw new Error(
    `Could not resolve source file for exact edit. Provide --source-file or a source_hint.source_file_path inside ${targetDir}.`,
  );
}

function runGenerator(dir) {
  const design = path.join(dir, "design.py");
  if (!fs.existsSync(design)) {
    throw new Error(`Missing CAD generator: ${design}`);
  }
  run("uv", ["run", "--package", "burr-build123d", "python", design]);
  expectFile(path.join(dir, designDataFile), "generated design data");
}

function runBurrCheck(dir, options = {}) {
  return run("cargo", ["run", "--quiet", "--", "check", dir], options);
}

function runExplainJson(dir) {
  const result = run("cargo", ["run", "--quiet", "--", "explain", "--json", dir]);
  return JSON.parse(result.output);
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
  });
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
  if (!options.allowFailure && result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit ${result.status}\n${output}`);
  }
  return { ...result, output };
}

function replaceOnce(source, before, after, label) {
  const first = source.indexOf(before);
  if (first < 0) {
    throw new Error(`Could not find exact before_text for ${label}`);
  }
  const second = source.indexOf(before, first + before.length);
  if (second >= 0) {
    throw new Error(`Expected one exact before_text for ${label}, found multiple`);
  }
  return `${source.slice(0, first)}${after}${source.slice(first + before.length)}`;
}

function parseArgs(args) {
  const options = {
    packet: null,
    receipt: null,
    sourceFile: null,
    targetDir: null,
  };
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--packet") {
      options.packet = requiredValue(args, ++index, arg);
    } else if (arg === "--receipt") {
      options.receipt = requiredValue(args, ++index, arg);
    } else if (arg === "--source-file") {
      options.sourceFile = requiredValue(args, ++index, arg);
    } else if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    } else if (arg.startsWith("--")) {
      throw new Error(`Unknown argument: ${arg}`);
    } else if (!options.targetDir) {
      options.targetDir = arg;
    } else {
      throw new Error(`Unexpected extra argument: ${arg}`);
    }
  }
  if (!options.targetDir) {
    printHelp();
    process.exit(2);
  }
  return options;
}

function requiredValue(args, index, flag) {
  const value = args[index];
  if (!value || value.startsWith("--")) {
    throw new Error(`${flag} requires a value`);
  }
  return value;
}

function printHelp() {
  console.log(`Usage:
  node scripts/agent-repair-runner.mjs <cad-folder> [--packet repair-packet.json] [--source-file design.py] [--receipt burr-agent-repair-receipt.json]

The runner applies only exact source_hint.before_text -> after_text edits with
confidence exact_from_design_data. If Burr only explains the failure, it writes
status not_repairable and does not edit source.`);
}

function assertDirectory(dir) {
  if (!fs.existsSync(dir) || !fs.statSync(dir).isDirectory()) {
    throw new Error(`Missing target directory: ${dir}`);
  }
}

function expectFile(file, label) {
  if (!fs.existsSync(file) || fs.statSync(file).size === 0) {
    throw new Error(`Missing ${label}: ${file}`);
  }
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function readOptionalJson(file) {
  if (!fs.existsSync(file)) {
    return null;
  }
  return readJson(file);
}
