#!/usr/bin/env node
import fs from "node:fs"
import https from "node:https"
import { spawnSync } from "node:child_process"

const contract = fs.readFileSync("docs/fray-website-contract.md", "utf8")
const urlMatch = contract.match(
  /asset_url: (https:\/\/github\.com\/fraylabs\/burr\/releases\/download\/burr-v0\.10\.0\/burr-gallery-v0\.10\.0\.zip)/,
)
if (!urlMatch) {
  throw new Error("Contract is missing the expected Burr gallery asset URL.")
}

const release = JSON.parse(
  spawnSync(
    "gh",
    ["release", "view", "burr-v0.10.0", "--repo", "fraylabs/burr", "--json", "assets"],
    { encoding: "utf8" },
  ).stdout,
)
const asset = release.assets.find((item) => item.name === "burr-gallery-v0.10.0.zip")
if (!asset) {
  throw new Error("GitHub release is missing burr-gallery-v0.10.0.zip")
}
if (asset.url !== urlMatch[1]) {
  throw new Error(`Contract URL does not match release asset URL.\n${urlMatch[1]}\n${asset.url}`)
}

const zip = await download(urlMatch[1])
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
    manifest = json.loads(archive.read("burr-gallery-v0.10.0/manifest.json"))

assert manifest["schema_version"] == "burr.gallery-artifact.v1"
assert manifest["burr_version"] == "0.10.0"
assert manifest["source"]["repository"] == "fraylabs/burr"
assert manifest["source"]["tag"] == "burr-v0.10.0"
assert len(manifest["examples"]) == 3
for example in manifest["examples"]:
    assert example["status"] == "pass"
    assert example["preview"].endswith(".png")
    assert example["receipt"].endswith(".receipt.json")
    assert example["design_data"].endswith(".design-data.json")
`,
  ],
  { input: zip, encoding: "buffer" },
)
if (result.status !== 0) {
  throw new Error(`Downloaded gallery asset failed manifest validation.\n${result.stderr}`)
}

console.log("Fray website contract proof passed")

function download(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (response) => {
        if (response.statusCode && response.statusCode >= 300 && response.statusCode < 400) {
          const location = response.headers.location
          response.resume()
          if (!location) {
            reject(new Error(`Redirect without location for ${url}`))
            return
          }
          download(location).then(resolve, reject)
          return
        }
        if (response.statusCode !== 200) {
          response.resume()
          reject(new Error(`Download failed with status ${response.statusCode}: ${url}`))
          return
        }
        const chunks = []
        response.on("data", (chunk) => chunks.push(chunk))
        response.on("end", () => resolve(Buffer.concat(chunks)))
      })
      .on("error", reject)
  })
}

