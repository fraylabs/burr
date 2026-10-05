"""Summarize bounded source rechecks without promoting sampling to a proof."""
import argparse
import collections
import json
import pathlib

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("directory", type=pathlib.Path)
parser.add_argument("--output", type=pathlib.Path, required=True)
args = parser.parse_args()
jobs = json.loads((args.directory / "jobs.json").read_text())
counts = collections.Counter()
rows = []
for model, pair in jobs:
    filename = model + "." + "-".join(map(str, pair)) + ".json"
    receipt = args.directory / filename
    data = json.loads(receipt.read_text()) if receipt.exists() else {}
    status = data.get("status", "pending") if data.get("phase") == "finished" else "pending"
    counts[status] += 1
    common = data.get("common", {})
    volume = common.get("volume_mm3", "—")
    exact_inside = len(data.get("classification", {}).get("both_in", []))
    winding_inside = len(data.get("winding_both_in_indices", []))
    certificates = data.get("interior_overlap_certificates", [])
    candidates = data.get("interior_candidates", [])
    clearances = [min(candidate["boundary_distances_mm"]) for candidate in candidates if candidate.get("winding_confirms_inside", True)]
    clearance = max(clearances) if clearances else "—"
    reported = data.get("reported_pair", {})
    method_version = data.get("method_version", 1) if data.get("phase") == "finished" else "—"
    normal = data.get("normal_probe", {})
    inward = sum(p.get("inward_verified", False) for p in normal.get("points", []))
    depth_bound = reported.get("maximum_overlap_depth", "—")
    volume_bound = reported.get("maximum_common_volume", "—")
    detail = data.get("refused") or data.get("error") or ""
    rows.append(f"| {model} | {pair[0]}/{pair[1]} | {status} | {method_version} | {volume} | {exact_inside} | {winding_inside} | {inward} | {len(certificates)} | {clearance} | {depth_bound} | {volume_bound} | {detail} |")
lines = [
    "Bounded source rechecks: " + ", ".join(f"{number} {status}" for status, number in sorted(counts.items())) + ".",
    "",
    "Each pair uses strict occurrence matching, valid source solids, OCCT Common, source point classification in bounded seed cubes, and independent solid-angle winding of fine source meshes. An overlap certificate also requires positive distance to both source boundaries. Negative sampling is bounded; it does not establish exhaustive separation. Boundary disagreements, invalid sources, mapping refusals and timeouts remain disputed.",
    "",
    "The maximum sampled clearance is the largest sampled distance to the nearer of the two source boundaries among points classified IN by both source solids and inside by both winding checks. It is local evidence, not a measured total overlap depth or volume. The declared depth and volume bounds come from the tested Burr report; a sampled clearance below them does not verify those global bounds. A dash means no candidate clearance was recorded, not zero clearance.",
    "",
    "Method 1 uses Common, seed cubes and winding. Methods 2–4 retain preliminary normal-probe receipts. Method 5 probes both face-normal signs and adjacent-face bisectors at 1e-6, 1e-4, 1e-3 and 1e-2 mm, verifies which steps enter the originating source solid, and checks those points against the other solid and winding. It records boundary distances for every source IN/IN point, including winding disagreements. Shared interior within 1e-6 mm of either boundary is treated as tolerance contact at the sampled witness; only deeper shared interior corroborated by winding yields an overlap certificate. These remain bounded local checks; earlier disputed rows without the completed tie-breaker need rechecking.",
    "",
    "| Model | Burr pair | Status | Method | Common mm³ | Source IN/IN samples | Winding IN/IN samples | Verified inward steps | Interior certificates | Max sampled clearance mm | Declared max depth mm | Declared max volume mm³ | Limitation |",
    "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|",
    *rows,
    "",
]
args.output.write_text("\n".join(lines))
print(json.dumps(dict(counts)))
