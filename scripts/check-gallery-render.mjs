#!/usr/bin/env node
import fs from "node:fs"
import { spawnSync } from "node:child_process"

const expected = [
  "artifacts/gallery-previews/shaft-bearing-bracket.png",
  "artifacts/gallery-previews/slotted-motor-plate.png",
  "artifacts/gallery-previews/electronics-standoff-deck.png",
]

const result = spawnSync("node", ["scripts/render-gallery.mjs"], {
  encoding: "utf8",
  maxBuffer: 1024 * 1024 * 32,
})
const output = [result.stdout, result.stderr].filter(Boolean).join("\n")
if (result.status !== 0) {
  throw new Error(`render-gallery failed with exit ${result.status}\n${output}`)
}

for (const png of expected) {
  if (!fs.existsSync(png)) {
    throw new Error(`Missing preview: ${png}`)
  }
  const data = fs.readFileSync(png)
  if (data.length < 4096) {
    throw new Error(`Preview is too small to be useful: ${png} (${data.length} bytes)`)
  }
  if (!data.subarray(0, 8).equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]))) {
    throw new Error(`Preview is not a PNG: ${png}`)
  }
}

console.log("gallery render proof passed")

