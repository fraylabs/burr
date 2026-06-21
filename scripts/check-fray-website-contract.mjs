#!/usr/bin/env node
import fs from "node:fs";
import https from "node:https";
import { spawnSync } from "node:child_process";

const contract = fs.readFileSync("docs/fray-website-contract.md", "utf8");
const packageJson = JSON.parse(fs.readFileSync("package.json", "utf8"));
const version = packageJson.version;
const tag = `burr-v${version}`;
const artifact = `burr-gallery-v${version}`;
const assetName = `${artifact}.zip`;
const expectedUrl = `https://github.com/fraylabs/burr/releases/download/${tag}/${assetName}`;
const urlMatch = contract.match(
  /asset_url: (https:\/\/github\.com\/fraylabs\/burr\/releases\/download\/[^\s]+)/,
);
if (!urlMatch) {
  throw new Error("Contract is missing the expected Burr gallery asset URL.");
}
if (urlMatch[1] !== expectedUrl) {
  throw new Error(
    `Contract is not pointing at the current Burr gallery URL.\n${urlMatch[1]}\n${expectedUrl}`,
  );
}

const release = JSON.parse(
  spawnSync(
    "gh",
    ["release", "view", tag, "--repo", "fraylabs/burr", "--json", "assets"],
    { encoding: "utf8" },
  ).stdout,
);
const asset = release.assets.find((item) => item.name === assetName);
if (!asset) {
  throw new Error(`GitHub release is missing ${assetName}`);
}
if (asset.url !== urlMatch[1]) {
  throw new Error(
    `Contract URL does not match release asset URL.\n${urlMatch[1]}\n${asset.url}`,
  );
}

const zip = await download(urlMatch[1]);
const result = spawnSync(
  "python3",
  [
    "-c",
    `
import io
import json
import sys
import zipfile

data = sys.stdin.buffer.read()
with zipfile.ZipFile(io.BytesIO(data)) as archive:
    manifest = json.loads(archive.read("${artifact}/manifest.json"))

assert manifest["schema_version"] == "burr.gallery-artifact.v1"
assert manifest["burr_version"] == "${version}"
assert manifest["source"]["repository"] == "fraylabs/burr"
assert manifest["source"]["tag"] == "${tag}"
assert len(manifest["examples"]) >= 2
statuses = {example["status"] for example in manifest["examples"]}
assert "pass" in statuses
assert "fail" in statuses
for example in manifest["examples"]:
    assert example["status"] in ("pass", "fail")
    assert example["expectation"] == example["status"]
    assert example["preview"].endswith(".png")
    assert example["receipt"].endswith(".receipt.json")
    assert example["design_data"].endswith(".design-data.json")
    if example["status"] == "fail":
        assert len(example["failed_rules"]) >= 1
`,
  ],
  { input: zip, encoding: "buffer" },
);
if (result.status !== 0) {
  throw new Error(
    `Downloaded gallery asset failed manifest validation.\n${result.stderr}`,
  );
}

console.log("Fray website contract proof passed");

function download(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (response) => {
        if (
          response.statusCode &&
          response.statusCode >= 300 &&
          response.statusCode < 400
        ) {
          const location = response.headers.location;
          response.resume();
          if (!location) {
            reject(new Error(`Redirect without location for ${url}`));
            return;
          }
          download(location).then(resolve, reject);
          return;
        }
        if (response.statusCode !== 200) {
          response.resume();
          reject(
            new Error(
              `Download failed with status ${response.statusCode}: ${url}`,
            ),
          );
          return;
        }
        const chunks = [];
        response.on("data", (chunk) => chunks.push(chunk));
        response.on("end", () => resolve(Buffer.concat(chunks)));
      })
      .on("error", reject);
  });
}
