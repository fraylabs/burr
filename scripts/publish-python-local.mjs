#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"
import { spawnSync } from "node:child_process"

const packageName = "burr-build123d"
const packageVersion = readPackageVersion(`packages/${packageName}/pyproject.toml`)
const distPrefix = `burr_build123d-${packageVersion}`
const confirm = process.argv.includes("--confirm")
const env = loadEnvLocal()
const token = process.env.UV_PUBLISH_TOKEN ?? env.UV_PUBLISH_TOKEN

if (!token || token === "pypi-replace-with-real-token" || !token.startsWith("pypi-")) {
  throw new Error(
    "Set UV_PUBLISH_TOKEN in .env.local to a real PyPI API token before publishing.",
  )
}

run("uv", ["build", "--package", packageName], { env: { ...process.env, ...env } })

const files = fs
  .readdirSync("dist")
  .filter((file) => file.startsWith(distPrefix))
  .map((file) => path.join("dist", file))
  .sort()

if (files.length === 0) {
  throw new Error(`No dist files found for ${distPrefix}`)
}

console.log("Python artifacts ready:")
for (const file of files) {
  console.log(`- ${file}`)
}

if (!confirm) {
  console.log("\nDry run only. Re-run with:")
  console.log("npm run publish:python:local -- --confirm")
  process.exit(0)
}

run("uv", ["publish", ...files], {
  env: {
    ...process.env,
    ...env,
    UV_PUBLISH_TOKEN: token,
  },
})

console.log("Published Python package to PyPI.")

function loadEnvLocal() {
  const file = ".env.local"
  if (!fs.existsSync(file)) {
    return {}
  }

  const env = {}
  for (const line of fs.readFileSync(file, "utf8").split(/\r?\n/)) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith("#")) {
      continue
    }
    const equals = trimmed.indexOf("=")
    if (equals === -1) {
      continue
    }
    const key = trimmed.slice(0, equals).trim()
    const value = trimmed.slice(equals + 1).trim()
    env[key] = unquote(value)
  }
  return env
}

function unquote(value) {
  if (
    (value.startsWith('"') && value.endsWith('"')) ||
    (value.startsWith("'") && value.endsWith("'"))
  ) {
    return value.slice(1, -1)
  }
  return value
}

function readPackageVersion(file) {
  const text = fs.readFileSync(file, "utf8")
  const match = text.match(/^version\s*=\s*"([^"]+)"$/m)
  if (!match) {
    throw new Error(`Could not read package version from ${file}`)
  }
  return match[1]
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    env: options.env ?? process.env,
    stdio: "inherit",
  })
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit ${result.status}`)
  }
}
