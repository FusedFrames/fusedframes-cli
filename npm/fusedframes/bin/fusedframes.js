#!/usr/bin/env node
"use strict";

// Thin npm launcher for the native FusedFrames CLI. npm installs exactly one
// of the optionalDependencies platform packages (matched on os/cpu); this shim
// resolves its binary and hands over argv, stdio and the exit code. All CLI
// behaviour — including the JSON output contract — lives in the binary.

const { spawnSync } = require("node:child_process");

const PLATFORM_PACKAGES = {
  "darwin arm64": "@fusedframes/cli-darwin-arm64",
  "darwin x64": "@fusedframes/cli-darwin-x64",
  "linux arm64": "@fusedframes/cli-linux-arm64",
  "linux x64": "@fusedframes/cli-linux-x64",
  "win32 x64": "@fusedframes/cli-win32-x64",
};

const RELEASES_URL = "https://github.com/fusedframes/fusedframes-cli/releases";

const key = `${process.platform} ${process.arch}`;
const pkg = PLATFORM_PACKAGES[key];
if (!pkg) {
  console.error(
    `fusedframes: unsupported platform (${key}). ` +
      `Prebuilt binaries for other platforms are at ${RELEASES_URL}.`
  );
  process.exit(1);
}

const binName = process.platform === "win32" ? "fusedframes.exe" : "fusedframes";
let binPath;
try {
  binPath = require.resolve(`${pkg}/bin/${binName}`);
} catch {
  console.error(
    `fusedframes: the platform package ${pkg} is not installed. ` +
      "It is installed automatically as an optional dependency — reinstall " +
      "without --no-optional/--omit=optional, or download a binary from " +
      RELEASES_URL + "."
  );
  process.exit(1);
}

const result = spawnSync(binPath, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`fusedframes: failed to launch the native binary: ${result.error.message}`);
  process.exit(1);
}
if (result.signal) {
  // Died by signal (e.g. Ctrl-C): re-raise so the shell sees the same status.
  process.kill(process.pid, result.signal);
}
process.exit(result.status === null ? 1 : result.status);
