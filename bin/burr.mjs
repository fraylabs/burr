#!/usr/bin/env node
import path from "node:path"

import {
  defaultRulepackPath,
  findManifestPaths,
  lintTargets,
  stampTargets,
} from "../src/index.mjs"

function printHelp() {
  console.log(`Usage:
  burr check [--rulepack <file>] [--no-write-receipt] <folder|fray-cad.json>...
  burr stamp <folder|fray-cad.json>...
`)
}

function parseCheckArgs(argv) {
  const args = [...argv]
  const inputs = []
  let rulepackPath = defaultRulepackPath
  let writeReceipt = true

  while (args.length > 0) {
    const arg = args.shift()
    if (arg === "--rulepack") {
      const next = args.shift()
      if (!next) throw new Error("--rulepack requires a file path.")
      rulepackPath = path.resolve(next)
    } else if (arg === "--no-write-receipt") {
      writeReceipt = false
    } else if (arg === "--help" || arg === "-h") {
      return { help: true }
    } else if (arg?.startsWith("--")) {
      throw new Error(`Unknown argument: ${arg}`)
    } else if (arg) {
      inputs.push(arg)
    }
  }

  return { inputs, rulepackPath, writeReceipt }
}

try {
  const [command, ...args] = process.argv.slice(2)
  if (!command || command === "--help" || command === "-h") {
    printHelp()
    process.exit(command ? 0 : 2)
  }

  if (command === "check") {
    const options = parseCheckArgs(args)
    if (options.help) {
      printHelp()
      process.exit(0)
    }
    if (options.inputs.length === 0) {
      printHelp()
      process.exit(2)
    }

    const results = lintTargets(options.inputs, options)
    const failures = results.filter(({ receipt }) => receipt.status === "fail")
    for (const result of results) {
      const receiptLabel =
        options.writeReceipt === false
          ? "<not written>"
          : path.relative(process.cwd(), result.receiptPath)
      console.log(
        `${result.receipt.status.toUpperCase()} ${path.relative(process.cwd(), result.manifestPath)} -> ${receiptLabel}`,
      )
    }
    process.exit(failures.length === 0 ? 0 : 1)
  }

  if (command === "stamp") {
    if (args.length === 0 || args.includes("--help") || args.includes("-h")) {
      printHelp()
      process.exit(args.length === 0 ? 2 : 0)
    }
    const manifests = findManifestPaths(args)
    if (manifests.length === 0) throw new Error("No fray-cad.json manifests found.")
    for (const manifestPath of stampTargets(args)) {
      console.log(`STAMP ${path.relative(process.cwd(), manifestPath)}`)
    }
    process.exit(0)
  }

  throw new Error(`Unknown command: ${command}`)
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error))
  process.exit(2)
}
