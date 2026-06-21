#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"
import { spawnSync } from "node:child_process"
import { galleryExamples } from "./gallery-examples.mjs"

const outputDir = "artifacts/gallery-previews"

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
  })
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n")
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit ${result.status}\n${output}`)
  }
  return output
}

fs.mkdirSync(outputDir, { recursive: true })

for (const example of galleryExamples) {
  run("uv", ["run", "--package", "burr-build123d", "python", example.design])
  run("cargo", ["run", "--quiet", "--", "check", path.dirname(example.design)])
  const png = path.join(outputDir, `${example.slug}.png`)
  run("uv", [
    "run",
    "--package",
    "burr-ocp",
    "python",
    "scripts/render-step-preview.py",
    example.step,
    png,
    "--title",
    example.title,
  ])
  console.log(`RENDER ${png}`)
}

console.log(`gallery previews written to ${outputDir}`)
