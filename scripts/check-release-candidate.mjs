#!/usr/bin/env node
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { spawnSync } from "node:child_process"
import { fileURLToPath } from "node:url"

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const packageJson = JSON.parse(fs.readFileSync(path.join(repoRoot, "package.json"), "utf8"))
const burrVersion = packageJson.version
const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "burr-release-candidate-"))
const cargoTarget = path.join(tempRoot, "cargo-package-target")
const installTarget = path.join(tempRoot, "cargo-install-target")
const toolRoot = path.join(tempRoot, "tools")
const starterDir = path.join(tempRoot, "starter")
const venvDir = path.join(tempRoot, "venv")

try {
  run("cargo", [
    "package",
    "--locked",
    "--allow-dirty",
    "--target-dir",
    cargoTarget,
  ])

  const packagedCrate = path.join(cargoTarget, "package", `burr-${burrVersion}`)
  assertExists(path.join(packagedCrate, "Cargo.toml"))
  run(
    "cargo",
    ["install", "--locked", "--path", packagedCrate, "--root", toolRoot],
    { env: { CARGO_TARGET_DIR: installTarget } },
  )

  const burrBinary = path.join(toolRoot, "bin", process.platform === "win32" ? "burr.exe" : "burr")
  assertExists(burrBinary)
  const installedVersion = run(burrBinary, ["--version"], { capture: true }).trim()
  if (installedVersion !== burrVersion) {
    throw new Error(`Expected packaged Burr ${burrVersion}, got ${installedVersion}`)
  }

  run(burrBinary, ["init", starterDir])
  for (const file of ["design.py", "pyproject.toml", ".gitignore"]) {
    assertExists(path.join(starterDir, file))
  }
  const goodExample = path.join(repoRoot, "examples", "linear-actuator-good")
  const checkOutput = run(
    burrBinary,
    ["check", "--no-write-receipt", goodExample],
    { capture: true },
  )
  if (!checkOutput.includes("PASS")) {
    throw new Error(`Packaged Burr did not pass the known-good fixture.\n${checkOutput}`)
  }

  const wheelSpecs = [
    {
      distribution: "burr-build123d",
      module: "burr_build123d",
      version: readPythonVersion("packages/burr-build123d/pyproject.toml"),
    },
    {
      distribution: "burr-ocp",
      module: "burr_ocp",
      version: readPythonVersion("packages/burr-ocp/pyproject.toml"),
    },
  ]
  const wheels = []
  for (const spec of wheelSpecs) {
    const outDir = path.join(tempRoot, "dist", spec.distribution)
    run("uv", ["build", "--package", spec.distribution, "--out-dir", outDir], {
      env: { UV_NO_CACHE: "1" },
    })
    const artifacts = fs.readdirSync(outDir).sort()
    const wheel = artifacts.find((file) => file.endsWith(".whl"))
    const source = artifacts.find((file) => file.endsWith(".tar.gz"))
    if (!wheel || !source) {
      throw new Error(`${spec.distribution} did not build both wheel and source artifacts`)
    }
    wheels.push(path.join(outDir, wheel))
  }

  run("uv", ["venv", "--python", "3.11", venvDir])
  const venvPython = path.join(
    venvDir,
    process.platform === "win32" ? "Scripts/python.exe" : "bin/python",
  )
  run("uv", ["pip", "install", "--python", venvPython, "--no-deps", ...wheels], {
    env: { UV_NO_CACHE: "1" },
  })
  const imports = wheelSpecs
    .map(
      ({ module, version }) =>
        `import ${module}; assert ${module}.__version__ == ${JSON.stringify(version)}`,
    )
    .join("; ")
  run(venvPython, ["-c", imports])

  console.log(
    `release candidate passed for burr ${burrVersion} and ${wheelSpecs.length} Python wheels`,
  )
} finally {
  fs.rmSync(tempRoot, { recursive: true, force: true })
}

function readPythonVersion(relativePath) {
  const text = fs.readFileSync(path.join(repoRoot, relativePath), "utf8")
  const match = text.match(/^version\s*=\s*"([^"]+)"$/m)
  if (!match) {
    throw new Error(`Could not read version from ${relativePath}`)
  }
  return match[1]
}

function assertExists(file) {
  if (!fs.existsSync(file)) {
    throw new Error(`Expected release candidate artifact: ${file}`)
  }
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    env: { ...process.env, ...(options.env ?? {}) },
    encoding: "utf8",
    stdio: options.capture ? "pipe" : "inherit",
    maxBuffer: 1024 * 1024 * 32,
  })
  if (result.error) {
    throw result.error
  }
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n")
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit ${result.status}\n${output}`)
  }
  return output
}
