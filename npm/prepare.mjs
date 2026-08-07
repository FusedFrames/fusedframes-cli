#!/usr/bin/env node
// Assemble the publishable npm packages for a release: one package per
// platform containing the native binary extracted from that target's release
// archive, then the @fusedframes/cli meta package (Node launcher) whose
// optionalDependencies pin every platform package at the same version.
//
// Usage: node npm/prepare.mjs <version> <artifact-dir>
//
// Writes to npm/out/platforms/* and npm/out/meta. Publish the platform
// packages first and the meta package last, so the meta package never goes
// live pointing at versions that don't exist yet.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const PLATFORMS = [
  {
    target: "aarch64-apple-darwin",
    pkg: "@fusedframes/cli-darwin-arm64",
    os: "darwin",
    cpu: "arm64",
    archive: "tar.gz",
    label: "macOS Apple Silicon",
  },
  {
    target: "x86_64-apple-darwin",
    pkg: "@fusedframes/cli-darwin-x64",
    os: "darwin",
    cpu: "x64",
    archive: "tar.gz",
    label: "macOS Intel",
  },
  {
    target: "aarch64-unknown-linux-musl",
    pkg: "@fusedframes/cli-linux-arm64",
    os: "linux",
    cpu: "arm64",
    archive: "tar.gz",
    label: "Linux arm64 (static)",
  },
  {
    target: "x86_64-unknown-linux-musl",
    pkg: "@fusedframes/cli-linux-x64",
    os: "linux",
    cpu: "x64",
    archive: "tar.gz",
    label: "Linux x86_64 (static)",
  },
  {
    target: "x86_64-pc-windows-msvc",
    pkg: "@fusedframes/cli-win32-x64",
    os: "win32",
    cpu: "x64",
    archive: "zip",
    label: "Windows x86_64",
  },
];

const [version, artifactDir] = process.argv.slice(2);
if (!version || !artifactDir) {
  console.error("Usage: node npm/prepare.mjs <version> <artifact-dir>");
  process.exit(1);
}

const npmDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.dirname(npmDir);
const outDir = path.join(npmDir, "out");
fs.rmSync(outDir, { recursive: true, force: true });

const shared = {
  license: "MIT",
  author: "FusedFrames <hello@fusedframes.com> (https://www.fusedframes.com)",
  homepage: "https://www.fusedframes.com",
  repository: {
    type: "git",
    url: "git+https://github.com/FusedFrames/fusedframes-cli.git",
  },
  bugs: { url: "https://github.com/FusedFrames/fusedframes-cli/issues" },
};

for (const platform of PLATFORMS) {
  const dir = path.join(outDir, "platforms", platform.pkg.split("/")[1]);
  const binDir = path.join(dir, "bin");
  fs.mkdirSync(binDir, { recursive: true });

  const archive = path.join(
    artifactDir,
    `fusedframes-v${version}-${platform.target}.${platform.archive}`
  );
  const binName = platform.os === "win32" ? "fusedframes.exe" : "fusedframes";
  if (platform.archive === "zip") {
    execFileSync("unzip", ["-o", "-q", archive, binName, "-d", binDir]);
  } else {
    execFileSync("tar", ["-xzf", archive, "-C", binDir, binName]);
  }
  fs.chmodSync(path.join(binDir, binName), 0o755);

  const manifest = {
    name: platform.pkg,
    version,
    description: `FusedFrames CLI native binary for ${platform.label}`,
    ...shared,
    os: [platform.os],
    cpu: [platform.cpu],
    files: ["bin"],
    preferUnplugged: true,
  };
  fs.writeFileSync(
    path.join(dir, "package.json"),
    JSON.stringify(manifest, null, 2) + "\n"
  );
  fs.copyFileSync(path.join(repoRoot, "LICENSE"), path.join(dir, "LICENSE"));
}

const metaDir = path.join(outDir, "meta");
fs.mkdirSync(path.join(metaDir, "bin"), { recursive: true });
const meta = JSON.parse(
  fs.readFileSync(path.join(npmDir, "fusedframes", "package.json"), "utf8")
);
meta.version = version;
// The template in npm/fusedframes is marked private so its placeholder
// version can never be published by accident; the real package is not.
delete meta.private;
meta.optionalDependencies = Object.fromEntries(
  PLATFORMS.map((platform) => [platform.pkg, version])
);
fs.writeFileSync(
  path.join(metaDir, "package.json"),
  JSON.stringify(meta, null, 2) + "\n"
);
fs.copyFileSync(
  path.join(npmDir, "fusedframes", "bin", "fusedframes.js"),
  path.join(metaDir, "bin", "fusedframes.js")
);
for (const file of ["README.md", "LICENSE"]) {
  fs.copyFileSync(path.join(repoRoot, file), path.join(metaDir, file));
}

console.log(
  `Prepared ${PLATFORMS.length} platform packages and the meta package in ${outDir}`
);
