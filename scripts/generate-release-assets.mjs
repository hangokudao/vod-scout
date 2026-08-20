import { createHash } from "node:crypto";
import { createReadStream, existsSync } from "node:fs";
import { copyFile, mkdir, readdir, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Prepare the v0.5.0-rc.1 unsigned GitHub Pre-release assets:
 * - VOD.Scout_0.5.0-rc.1_x64-setup.exe
 * - SBOM.spdx.json
 * - SHA256SUMS.txt
 *
 * This release intentionally does not generate or publish updater packages,
 * signatures, or latest.json. Bundle lookup is exact and version-scoped.
 */
const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const version = "0.5.0-rc.1";
const bundleDir = join(projectRoot, "src-tauri", "target", "release", "bundle", "nsis");
const releaseDir = join(projectRoot, "release", `v${version}`);
const publicInstallerName = `VOD.Scout_${version}_x64-setup.exe`;
const installerCandidates = [publicInstallerName, `VOD Scout_${version}_x64-setup.exe`];

function pickExactOne(present, candidates, label) {
  const matches = candidates.filter((name) => present.has(name));
  if (matches.length === 0) return null;
  if (matches.length > 1) {
    throw new Error(
      `Ambiguous ${label} for v${version}: found ${matches.join(", ")}. Expected at most one of: ${candidates.join(", ")}`
    );
  }
  return matches[0];
}

if (!existsSync(bundleDir)) {
  throw new Error(`NSIS bundle directory not found: ${bundleDir}`);
}

const bundleFiles = await readdir(bundleDir);
const present = new Set(bundleFiles);
const builtInstallerName = pickExactOne(present, installerCandidates, "NSIS installer");
if (!builtInstallerName) {
  throw new Error(
    `Tauri NSIS installer not found for v${version}. Expected exactly one of: ${installerCandidates.join(", ")}`
  );
}

const forbiddenUpdaterAssets = bundleFiles.filter(
  (name) => name.includes(version) && (name.endsWith(".sig") || name.endsWith(".nsis.zip"))
);
if (forbiddenUpdaterAssets.length > 0) {
  throw new Error(`Updater assets are not allowed for v${version}: ${forbiddenUpdaterAssets.join(", ")}`);
}

await mkdir(releaseDir, { recursive: true });
await copyFile(join(bundleDir, builtInstallerName), join(releaseDir, publicInstallerName));
await copyFile(join(projectRoot, "SBOM.spdx.json"), join(releaseDir, "SBOM.spdx.json"));

async function sha256(path) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest("hex").toUpperCase();
}

const checksumFiles = [publicInstallerName, "SBOM.spdx.json"];
const sums = [];
for (const name of checksumFiles) sums.push(`${await sha256(join(releaseDir, name))}  ${name}`);
await writeFile(join(releaseDir, "SHA256SUMS.txt"), `${sums.join("\n")}\n`, "utf8");

process.stdout.write(
  `unsigned pre-release assets prepared in ${releaseDir}\n${[
    ...checksumFiles,
    "SHA256SUMS.txt"
  ].map((name) => basename(name)).join("\n")}\n`
);
