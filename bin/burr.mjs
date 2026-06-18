#!/usr/bin/env node
import path from "node:path"

import {
  burrVersion,
  designDataFileName,
  defaultRulepackPath,
  findDesignDataPaths,
  formatReceiptDiagnostics,
  lintTargets,
  stampTargets,
} from "../src/index.mjs"

function printHelp() {
  console.log(`Usage:
  burr check [--rulepack <file>] [--no-write-receipt] <folder|${designDataFileName}>...
  burr stamp <folder|${designDataFileName}>...
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
  if (command === "--version" || command === "-v" || command === "version") {
    console.log(burrVersion)
    process.exit(0)
  }

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
        `${result.receipt.status.toUpperCase()} ${path.relative(process.cwd(), result.designDataPath)} -> ${receiptLabel}`,
      )
      const diagnostics = formatReceiptDiagnostics(result.receipt)
      if (diagnostics.length > 0) {
        console.log("")
        console.log(`${diagnostics.length} problem${diagnostics.length === 1 ? "" : "s"}:`)
        for (const [index, diagnostic] of diagnostics.entries()) {
          console.log(`${index + 1}. ${diagnostic.lines[0]}`)
          for (const line of diagnostic.lines.slice(1)) {
            console.log(`   ${line}`)
          }
        }
        console.log("")
      }
    }
    process.exit(failures.length === 0 ? 0 : 1)
  }

  if (command === "stamp") {
    if (args.length === 0 || args.includes("--help") || args.includes("-h")) {
      printHelp()
      process.exit(args.length === 0 ? 2 : 0)
    }
    const designDataFiles = findDesignDataPaths(args)
    if (designDataFiles.length === 0) throw new Error(`No ${designDataFileName} files found.`)
    for (const designDataPath of stampTargets(args)) {
      console.log(`STAMP ${path.relative(process.cwd(), designDataPath)}`)
    }
    process.exit(0)
  }

  throw new Error(`Unknown command: ${command}`)
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error))
  process.exit(2)
}
