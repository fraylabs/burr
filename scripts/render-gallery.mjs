#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"
import { spawnSync } from "node:child_process"

const outputDir = "artifacts/gallery-previews"
const examples = [
  {
    slug: "shaft-bearing-bracket",
    step: "examples/gallery/shaft-bearing-bracket/shaft-bearing-bracket.step",
    design: "examples/gallery/shaft-bearing-bracket/design.py",
    title: "Shaft Bearing Bracket",
  },
  {
    slug: "slotted-motor-plate",
    step: "examples/gallery/slotted-motor-plate/slotted-motor-plate.step",
    design: "examples/gallery/slotted-motor-plate/design.py",
    title: "Slotted Motor Plate",
  },
  {
    slug: "electronics-standoff-deck",
    step: "examples/gallery/electronics-standoff-deck/electronics-standoff-deck.step",
    design: "examples/gallery/electronics-standoff-deck/design.py",
    title: "Electronics Standoff Deck",
  },
]

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

for (const example of examples) {
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

