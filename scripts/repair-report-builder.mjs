import fs from "node:fs";
import path from "node:path";

const actuatorEdgeDistanceReport = {
  slug: "actuator-housing-edge-distance",
  title: "Actuator Housing Edge-Distance Repair",
  focusRuleId: "actuator_mount:m3_loaded_hole_edge_distance",
  beforeExampleSlug: "bad-actuator-housing-edge-distance",
  afterExampleSlug: "fixed-actuator-housing",
};

export function buildRepairReports({ releaseDir, version, generatedAt, examples }) {
  const reportsDir = path.join(releaseDir, "repair-reports");
  fs.mkdirSync(reportsDir, { recursive: true });

  return [buildActuatorEdgeDistanceReport()]
    .map((report) => writeRepairReport({ reportsDir, report }))
    .map((written) => ({
      id: written.report.report_id,
      slug: written.report.report_id,
      title: written.report.title,
      status: written.report.status,
      focus_rule_id: written.report.focus_rule_id,
      before_example: written.report.before.example_slug,
      after_example: written.report.after.example_slug,
      before_slug: written.report.before.example_slug,
      after_slug: written.report.after.example_slug,
      report_json: `repair-reports/${written.jsonFile}`,
      report_markdown: `repair-reports/${written.markdownFile}`,
    }));

  function buildActuatorEdgeDistanceReport() {
    return buildBeforeAfterReport({
      releaseDir,
      version,
      generatedAt,
      examples,
      spec: actuatorEdgeDistanceReport,
    });
  }
}

function buildBeforeAfterReport({ releaseDir, version, generatedAt, examples, spec }) {
  const beforeExample = findExample(examples, spec.beforeExampleSlug);
  const afterExample = findExample(examples, spec.afterExampleSlug);
  const beforeReceipt = readJson(path.join(releaseDir, beforeExample.receipt));
  const afterReceipt = readJson(path.join(releaseDir, afterExample.receipt));
  const beforeDesign = readJson(path.join(releaseDir, beforeExample.design_data));
  const afterDesign = readJson(path.join(releaseDir, afterExample.design_data));

  if (beforeReceipt.status !== "fail") {
    throw new Error(
      `${spec.beforeExampleSlug} must be a failing receipt for repair report generation`,
    );
  }
  if (afterReceipt.status !== "pass") {
    throw new Error(
      `${spec.afterExampleSlug} must be a passing receipt for repair report generation`,
    );
  }

  const beforeChecks = checksForRule(beforeReceipt, spec.focusRuleId);
  const afterChecks = checksForRule(afterReceipt, spec.focusRuleId);
  const featureIds = uniqueSorted([
    ...beforeChecks.map((check) => check.feature_id),
    ...afterChecks.map((check) => check.feature_id),
  ]);
  if (featureIds.length === 0) {
    throw new Error(`No feature checks found for ${spec.focusRuleId}`);
  }

  const featureResults = featureIds.map((featureId) => {
    const before = findFeatureCheck(beforeChecks, featureId);
    const after = findFeatureCheck(afterChecks, featureId);
    return {
      feature_id: featureId,
      before: summarizeCheck(before),
      after: summarizeCheck(after),
      margin_delta_mm: roundMm((after.margin_mm ?? 0) - (before.margin_mm ?? 0)),
      measured_delta_mm: roundMm(
        (after.measured?.center_to_edge_mm ?? 0) -
          (before.measured?.center_to_edge_mm ?? 0),
      ),
      repaired: before.status === "fail" && after.status === "pass",
    };
  });

  const unrepaired = featureResults.filter((result) => !result.repaired);
  if (unrepaired.length > 0) {
    throw new Error(
      `Repair report has unrepaired feature checks: ${unrepaired
        .map((result) => result.feature_id)
        .join(", ")}`,
    );
  }

  const beforeFailures = beforeChecks.filter((check) => check.status === "fail");
  const afterFailures = afterChecks.filter((check) => check.status === "fail");
  const minBeforeMarginMm = Math.min(
    ...featureResults.map((result) => result.before.margin_mm),
  );
  const minAfterMarginMm = Math.min(
    ...featureResults.map((result) => result.after.margin_mm),
  );

  const firstFix = firstFixForReason(beforeFailures[0]?.reason);
  const beforeSourceFilePath = sourceFilePath({
    receipt: beforeReceipt,
    designData: beforeDesign,
  });
  const afterSourceFilePath = sourceFilePath({
    receipt: afterReceipt,
    designData: afterDesign,
  });
  const beforeSource = readText(beforeSourceFilePath);
  const afterSource = readText(afterSourceFilePath);
  const repairActions = [
    ...beforeFailures.map((check) =>
      buildRepairAction({
        check,
        beforeReceipt,
        beforeDesign,
        afterDesign,
        afterChecks,
        beforeSource,
        afterSource,
      }),
    ),
    ...buildEnvelopeRepairActions({
      beforeReceipt,
      beforeDesign,
      afterDesign,
      beforeSource,
      afterSource,
      ruleId: spec.focusRuleId,
    }),
  ];

  return {
    schema_version: "burr.repair-report.v1",
    id: spec.slug,
    report_id: spec.slug,
    title: spec.title,
    status: "pass",
    burr_version: version,
    generated_at: generatedAt,
    focus_rule_id: spec.focusRuleId,
    loop: "bad CAD -> Burr check -> explain fix order -> fixed CAD passes",
    first_fix: firstFix,
    summary: {
      repair_status: "repaired",
      status_transition: `${beforeReceipt.status}_to_${afterReceipt.status}`,
      repaired_features: featureResults.length,
      before_failures: beforeFailures.length,
      after_failures: afterFailures.length,
      failure_delta: afterFailures.length - beforeFailures.length,
      min_before_margin_mm: roundMm(minBeforeMarginMm),
      min_after_margin_mm: roundMm(minAfterMarginMm),
      min_margin_delta_mm: roundMm(minAfterMarginMm - minBeforeMarginMm),
    },
    before: summarizeReceipt({
      example: beforeExample,
      receipt: beforeReceipt,
      focusRuleId: spec.focusRuleId,
    }),
    after: summarizeReceipt({
      example: afterExample,
      receipt: afterReceipt,
      focusRuleId: spec.focusRuleId,
      proofChecks: afterChecks.filter((check) => check.status === "pass"),
    }),
    failures: beforeFailures.map((check) => summarizeFailure(check, firstFix)),
    repair_actions: repairActions,
    feature_results: featureResults,
  };
}

function writeRepairReport({ reportsDir, report }) {
  const jsonFile = `${report.report_id}.json`;
  const markdownFile = `${report.report_id}.md`;
  fs.writeFileSync(
    path.join(reportsDir, jsonFile),
    JSON.stringify(report, null, 2) + "\n",
  );
  fs.writeFileSync(path.join(reportsDir, markdownFile), renderMarkdown(report));
  return { report, jsonFile, markdownFile };
}

function summarizeReceipt({ example, receipt, focusRuleId, proofChecks = [] }) {
  const focusChecks = checksForRule(receipt, focusRuleId);
  const summary = {
    slug: example.slug,
    example_slug: example.slug,
    title: example.title,
    status: receipt.status,
    receipt: example.receipt,
    design_data: example.design_data,
    artifact_id: receipt.artifact_id,
    artifact_type: receipt.artifact_type,
    rulepack_id: receipt.rulepack_id,
    rulepack_version: receipt.rulepack_version,
    checked_features: receipt.summary?.features?.checked_feature_ids ?? [],
    focus_failures: focusChecks
      .filter((check) => check.status === "fail")
      .map(summarizeCheck),
  };
  if (proofChecks.length > 0) {
    summary.proof_checks = proofChecks.map(summarizeProofCheck);
  }
  return summary;
}

function summarizeCheck(check) {
  return {
    status: check.status,
    reason: check.reason,
    message: check.message,
    measured: {
      center_to_edge_mm: check.measured?.center_to_edge_mm ?? null,
      wall_to_edge_mm: check.measured?.wall_to_edge_mm ?? null,
      hole_diameter_mm: check.measured?.hole_diameter_mm ?? null,
    },
    required: {
      center_to_edge_mm: check.required?.center_to_edge_mm ?? null,
      wall_to_edge_mm: check.required?.wall_to_edge_mm ?? null,
      center_to_edge_diameter_multiple:
        check.required?.center_to_edge_diameter_multiple ?? null,
    },
    margin_mm: check.margin_mm ?? null,
  };
}

function summarizeFailure(check, firstFix) {
  return {
    feature_id: check.feature_id ?? null,
    rule_id: check.rule_id,
    reason: check.reason,
    message: check.message,
    measured: check.measured ?? {},
    required: check.required ?? {},
    margin_mm: check.margin_mm ?? null,
    first_fix: firstFixForReason(check.reason) ?? firstFix,
  };
}

function summarizeProofCheck(check) {
  return {
    feature_id: check.feature_id ?? null,
    rule_id: check.rule_id,
    reason: check.reason,
    measured: check.measured ?? {},
    required: check.required ?? {},
    margin_mm: check.margin_mm ?? null,
  };
}

function buildRepairAction({
  check,
  beforeReceipt,
  beforeDesign,
  afterDesign,
  afterChecks,
  beforeSource,
  afterSource,
}) {
  const featureId = check.feature_id;
  const beforeFeature = findDesignFeature(beforeDesign, featureId);
  const afterFeature = findDesignFeature(afterDesign, featureId);
  const afterCheck = findFeatureCheck(afterChecks, featureId);
  const featureCenterDeltaMm = vectorDelta(afterFeature.center_mm, beforeFeature.center_mm);
  const parameter = "center_mm";
  const valuePath = `features[id=${featureId}].${parameter}`;
  const beforeText = sourceLineForFeature({ source: beforeSource, featureId });
  const afterText = sourceLineForFeature({ source: afterSource, featureId });

  return {
    feature_id: featureId,
    action: "move_feature",
    parameter,
    before_value_mm: beforeFeature.center_mm,
    after_value_mm: afterFeature.center_mm,
    suggested_delta_mm: featureCenterDeltaMm,
    source_hint: {
      source_file_path: sourceFilePath({ receipt: beforeReceipt, designData: beforeDesign }),
      edit_kind: "replace_python_dict_entry",
      selector: `mount_holes[${JSON.stringify(featureId)}]`,
      before_text: beforeText,
      after_text: afterText,
      feature_id: featureId,
      parameter,
      value_path: valuePath,
      before_value_mm: beforeFeature.center_mm,
      after_value_mm: afterFeature.center_mm,
      confidence: "exact_from_design_data",
      rationale: `The before and after design data declare ${valuePath}; updating that editable value moves the failed clearance-hole center used by this edge-distance check.`,
    },
    rule_id: check.rule_id,
    failure_reason: check.reason,
    reason: `Move ${featureId} from [${beforeFeature.center_mm.join(", ")}] mm to [${afterFeature.center_mm.join(", ")}] mm so center-to-edge increases from ${formatMm(check.measured?.center_to_edge_mm)} to at least ${formatMm(check.required?.center_to_edge_mm)}.`,
    measured_delta_mm: roundMm(
      (afterCheck.measured?.center_to_edge_mm ?? 0) -
        (check.measured?.center_to_edge_mm ?? 0),
    ),
    margin_delta_mm: roundMm((afterCheck.margin_mm ?? 0) - (check.margin_mm ?? 0)),
    measured: check.measured ?? {},
    required: check.required ?? {},
    margin_mm: check.margin_mm ?? null,
    verifies_against_after_feature: {
      feature_id: afterCheck.feature_id ?? featureId,
      status: afterCheck.status,
      reason: afterCheck.reason,
      measured: afterCheck.measured ?? {},
      required: afterCheck.required ?? {},
      margin_mm: afterCheck.margin_mm ?? null,
    },
  };
}

function buildEnvelopeRepairActions({
  beforeReceipt,
  beforeDesign,
  afterDesign,
  beforeSource,
  afterSource,
  ruleId,
}) {
  const beforePart = findDesignPart(beforeDesign, "housing");
  const afterPart = findDesignPart(afterDesign, "housing");
  const beforeSize = bboxSize(beforePart);
  const afterSize = bboxSize(afterPart);
  const dimensions = [
    { selector: "housing_length", axis: 0 },
    { selector: "housing_width", axis: 1 },
    { selector: "housing_height", axis: 2 },
  ];

  return dimensions
    .filter(({ axis }) => beforeSize[axis] !== afterSize[axis])
    .map(({ selector, axis }) => {
      const beforeValue = beforeSize[axis];
      const afterValue = afterSize[axis];
      return {
        feature_id: beforePart.id,
        action: "resize_part_envelope",
        parameter: `bbox_mm.size[${axis}]`,
        before_value_mm: beforeValue,
        after_value_mm: afterValue,
        suggested_delta_mm: roundMm(afterValue - beforeValue),
        source_hint: {
          source_file_path: sourceFilePath({
            receipt: beforeReceipt,
            designData: beforeDesign,
          }),
          edit_kind: "replace_python_assignment",
          selector,
          feature_id: beforePart.id,
          parameter: `bbox_mm.size[${axis}]`,
          value_path: `parts[id=${beforePart.id}].bbox_mm.size[${axis}]`,
          before_value_mm: beforeValue,
          after_value_mm: afterValue,
          before_text: sourceLineForAssignment({ source: beforeSource, selector }),
          after_text: sourceLineForAssignment({ source: afterSource, selector }),
          confidence: "exact_from_design_data",
          rationale: `The before and after design data declare parts[id=${beforePart.id}].bbox_mm; updating ${selector} changes the housing envelope needed by the moved mounting holes.`,
        },
        rule_id: ruleId,
        failure_reason: "supporting_envelope_change",
        reason: `Resize ${beforePart.id} ${selector} from ${formatMm(beforeValue)} to ${formatMm(afterValue)} so the repaired hole centers keep positive edge-distance margin.`,
        verifies_against_after_part: {
          part_id: afterPart.id,
          bbox_size_mm: afterSize,
        },
      };
    });
}

function sourceFilePath({ receipt, designData }) {
  const sourcePath = designData.source?.path ?? designData.sources?.[0]?.path;
  if (!sourcePath) {
    throw new Error("Missing design-data source path for repair action source_hint.");
  }
  const sourceDesignData = receipt.source_design_data ?? receipt.source_manifest;
  if (!sourceDesignData) {
    return sourcePath;
  }
  return path.posix.join(path.posix.dirname(sourceDesignData), sourcePath);
}

function findDesignFeature(designData, featureId) {
  const feature = designData.features?.find((item) => item.id === featureId);
  if (!feature) {
    throw new Error(`Missing design-data feature for repair action: ${featureId}`);
  }
  if (!Array.isArray(feature.center_mm) || feature.center_mm.length !== 3) {
    throw new Error(`Feature ${featureId} is missing center_mm for repair action.`);
  }
  return feature;
}

function findDesignPart(designData, partId) {
  const part = designData.parts?.find((item) => item.id === partId);
  if (!part) {
    throw new Error(`Missing design-data part for repair action: ${partId}`);
  }
  if (!Array.isArray(part.bbox_mm?.min) || !Array.isArray(part.bbox_mm?.max)) {
    throw new Error(`Part ${partId} is missing bbox_mm for repair action.`);
  }
  return part;
}

function bboxSize(part) {
  return part.bbox_mm.max.map((value, index) =>
    roundMm(value - part.bbox_mm.min[index]),
  );
}

function sourceLineForFeature({ source, featureId }) {
  return sourceLineForPattern({
    source,
    pattern: new RegExp(
      `^\\s*["']${escapeRegExp(featureId)}["']\\s*:\\s*\\([^)]+\\),\\s*$`,
      "m",
    ),
    label: `mount_holes entry for ${featureId}`,
  });
}

function sourceLineForAssignment({ source, selector }) {
  return sourceLineForPattern({
    source,
    pattern: new RegExp(
      `^\\s*${escapeRegExp(selector)}\\s*=\\s*[-+]?\\d+(?:\\.\\d+)?\\s*$`,
      "m",
    ),
    label: `assignment for ${selector}`,
  });
}

function sourceLineForPattern({ source, pattern, label }) {
  const globalPattern = pattern.global
    ? pattern
    : new RegExp(pattern.source, `${pattern.flags}g`);
  const matches = [...source.matchAll(globalPattern)];
  if (matches.length !== 1) {
    throw new Error(`Expected one ${label}, found ${matches.length}`);
  }
  return matches[0][0];
}

function vectorDelta(after, before) {
  return after.map((value, index) => roundMm(value - before[index]));
}

function renderMarkdown(report) {
  return [
    `# ${report.title}`,
    "",
    `Burr ${report.burr_version} repair report generated from release artifact receipts.`,
    "",
    `Loop: ${report.loop}.`,
    "",
    `Status: ${report.status}`,
    `First fix: ${report.first_fix}`,
    "",
    "## Summary",
    "",
    `- Status: ${report.summary.repair_status}`,
    `- Transition: ${report.summary.status_transition}`,
    `- Focus rule: ${report.focus_rule_id}`,
    `- Failed focus checks: ${report.summary.before_failures} before, ${report.summary.after_failures} after`,
    `- Minimum margin: ${formatMm(report.summary.min_before_margin_mm)} before, ${formatMm(report.summary.min_after_margin_mm)} after`,
    "",
    "## Receipts",
    "",
    `- Before: ${report.before.receipt}`,
    `- Before design data: ${report.before.design_data}`,
    `- After: ${report.after.receipt}`,
    `- After design data: ${report.after.design_data}`,
    "",
    "## Feature Evidence",
    "",
    [
      "| Feature | Before center-to-edge | Required | Before margin | After center-to-edge | After margin | Result |",
      "| --- | ---: | ---: | ---: | ---: | ---: | --- |",
      ...report.feature_results.map((result) =>
        `| ${[
          markdownCell(result.feature_id),
          formatMm(result.before.measured.center_to_edge_mm),
          formatMm(result.before.required.center_to_edge_mm),
          formatMm(result.before.margin_mm),
          formatMm(result.after.measured.center_to_edge_mm),
          formatMm(result.after.margin_mm),
          result.repaired ? "repaired" : "not repaired",
        ].join(" | ")} |`,
      ),
    ].join("\n"),
    "",
    "## Failed Checks",
    "",
    ...report.failures.flatMap((failure) => [
      `### ${failure.feature_id}`,
      "",
      `- Rule: ${failure.rule_id}`,
      `- Reason: ${failure.reason}`,
      `- Measured center-to-edge: ${formatMm(failure.measured.center_to_edge_mm)}`,
      `- Required center-to-edge: ${formatMm(failure.required.center_to_edge_mm)}`,
      `- Margin: ${formatMm(failure.margin_mm)}`,
      `- First fix: ${failure.first_fix}`,
      "",
    ]),
    "## Repair Actions",
    "",
    [
      "| Feature | Action | Source | Value path | Confidence | Suggested delta | Reason |",
      "| --- | --- | --- | --- | --- | ---: | --- |",
      ...report.repair_actions.map((action) =>
        `| ${[
          markdownCell(action.feature_id),
          markdownCell(action.action),
          markdownCell(action.source_hint.source_file_path),
          markdownCell(action.source_hint.value_path),
          markdownCell(action.source_hint.confidence),
          formatDeltaMm(action.suggested_delta_mm),
          markdownCell(action.reason),
        ].join(" | ")} |`,
      ),
    ].join("\n"),
    "",
    "## After Pass",
    "",
    ...(report.after.proof_checks ?? []).flatMap((check) => [
      `### ${check.feature_id}`,
      "",
      `- Rule: ${check.rule_id}`,
      "- Status: pass",
      `- Measured center-to-edge: ${formatMm(check.measured.center_to_edge_mm)}`,
      `- Required center-to-edge: ${formatMm(check.required.center_to_edge_mm)}`,
      `- Margin: ${formatMm(check.margin_mm)}`,
      "",
    ]),
    "The JSON report is the machine-readable artifact. This Markdown file is a human review view of the same receipt-derived evidence.",
    "",
  ].join("\n");
}

function findExample(examples, slug) {
  const example = examples.find((item) => item.slug === slug);
  if (!example) {
    throw new Error(`Missing repair report example in manifest: ${slug}`);
  }
  return example;
}

function checksForRule(receipt, ruleId) {
  return receipt.checks.filter((check) => check.rule_id === ruleId);
}

function findFeatureCheck(checks, featureId) {
  const check = checks.find((item) => item.feature_id === featureId);
  if (!check) {
    throw new Error(`Missing ${featureId} check in repair report receipts`);
  }
  return check;
}

function readJson(file) {
  if (!fs.existsSync(file)) {
    throw new Error(`Missing repair report source JSON: ${file}`);
  }
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function readText(file) {
  if (!fs.existsSync(file)) {
    throw new Error(`Missing repair report source text: ${file}`);
  }
  return fs.readFileSync(file, "utf8");
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function uniqueSorted(values) {
  return [...new Set(values.filter(Boolean))].sort();
}

function roundMm(value) {
  return Math.round(value * 1000) / 1000;
}

function formatMm(value) {
  if (value === null || value === undefined) {
    return "n/a";
  }
  return `${value} mm`;
}

function formatDeltaMm(value) {
  if (Array.isArray(value)) {
    return `[${value.join(", ")}] mm`;
  }
  return formatMm(value);
}

function markdownCell(value) {
  return String(value).replaceAll("|", "\\|");
}

function firstFixForReason(reason) {
  if (reason === "insufficient_edge_distance") {
    return "Move the loaded M3 holes inward or increase the surrounding housing size.";
  }
  return "Fix the failed Burr check, then rerun burr check.";
}
