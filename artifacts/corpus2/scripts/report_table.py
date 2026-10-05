"""Render analysis.json as a reviewable Markdown measurement table.

Run summarize.py first. Missing values stay explicit; this does not classify
models or certify pair recall beyond the strict comparison's recorded scope.
"""
import argparse
import json
from pathlib import Path


def number(value):
    return f"{value:.3f}" if isinstance(value, (int, float)) else "—"


def cell(value):
    return str(value).replace("|", "\\|").replace("\n", " ")


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
args = parser.parse_args()
rows = json.loads((args.root / "analysis.json").read_text())
print("| Model | Class | Parts Burr / OCCT | Lost / declared faces | Burr verdict | OCCT verdict | Strict pairs matched / extra / missing | Viewer s | Check s | Burr MiB | OCCT import s | OCCT total s | OCCT MiB |")
print("|---|---|---:|---:|---|---|---|---:|---:|---:|---:|---:|---:|")
for row in rows:
    metric, reference = row["metric"], row["occt_metric"]
    burr, occt, comparison = row["burr"], row["occt"], row["comparison"]
    meta = reference.get("load_metadata", occt)
    pairs = "—"
    if comparison.get("refused"):
        pairs = "refused"
    elif comparison:
        pairs = " / ".join(str(len(comparison.get(key, []))) for key in
                           ("matched_pairs", "extra_pairs", "missing_pairs"))
        if not comparison.get("occurrence_mapping_complete"):
            pairs += " (partial mapping)"
        if comparison.get("reference_scope") != "all_pairs":
            pairs += " (reported only)"
    if row.get("independent_false_pairs") and not comparison.get("extra_pairs"):
        pairs += f'; {len(row["independent_false_pairs"])} independently proved extra pair(s)'
    verdict = burr.get("outcome", "—")
    if burr and not burr.get("pair_set_complete"):
        verdict += " (partial)"
    exact = occt.get("outcome", "—")
    if reference.get("cap") == "memory":
        exact = "reference cap"
    elif reference.get("timeout"):
        exact = "reference timeout"
    elif occt and not occt.get("pair_check_complete"):
        exact += " (partial)"
    values = [row["name"], row["status"],
              f'{metric.get("component_count", "—")} / {row.get("parts") if row.get("parts") is not None else "—"}',
              f'{metric.get("lost_faces", "—")} / {metric.get("declared_faces", "—")}',
              verdict, exact, pairs,
              number(metric.get("import_and_viewer_s")), number(metric.get("check_s")),
              number(metric.get("peak_rss_mib")), number(meta.get("import_s")),
              number(reference.get("elapsed_s")), number(reference.get("peak_rss_mib"))]
    print("| " + " | ".join(map(cell, values)) + " |")
