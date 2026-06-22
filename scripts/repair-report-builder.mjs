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

function markdownCell(value) {
  return String(value).replaceAll("|", "\\|");
}

function firstFixForReason(reason) {
  if (reason === "insufficient_edge_distance") {
    return "Move the loaded M3 holes inward or increase the surrounding housing size.";
  }
  return "Fix the failed Burr check, then rerun burr check.";
}
