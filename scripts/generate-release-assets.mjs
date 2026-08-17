import { createHash } from "node:crypto";
import { createReadStream, existsSync } from "node:fs";
import { copyFile, mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Local packaging helper aligned with the v0.5.0 candidate artifact contract:
 * - Installer / updater: VOD.Scout_<version>_x64-setup.exe (+ .sig)
 * - Optional NSIS updater zip public name: VOD.Scout_<version>_x64-setup.nsis.zip
 * - latest.json, SHA256SUMS.txt, SBOM.spdx.json
 *
 * Bundle lookup is exact and version-scoped. Accepts only the known Tauri
 * product basename variants (dotted public name and space-containing local
 * productName) for 0.5.0. Never picks an arbitrary *.nsis.zip.
 * Fails closed on missing or ambiguous expected assets.
 * Preserves updater signature embedding and checksum/SBOM copies.
 * Does not invent Authenticode signing or delete files.
 */
const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const version = "0.5.0";
const bundleDir = join(projectRoot, "src-tauri", "target", "release", "bundle", "nsis");
const releaseDir = join(projectRoot, "release", `v${version}`);
const publicInstallerName = `VOD.Scout_${version}_x64-setup.exe`;
const publicUpdaterZipName = `VOD.Scout_${version}_x64-setup.nsis.zip`;

/** Exact Tauri NSIS basenames for this version only (public dotted + local productName). */
const installerCandidates = [
  publicInstallerName,
  `VOD Scout_${version}_x64-setup.exe`
];
const nsisZipCandidates = [
  publicUpdaterZipName,
  `VOD Scout_${version}_x64-setup.nsis.zip`
];

/**
 * @param {Set<string>} present
 * @param {string[]} candidates
 * @param {string} label
 * @returns {string | null}
 */
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
const builtInstaller = join(bundleDir, builtInstallerName);

// Version-scoped updater body: exact .nsis.zip for this version, else installer + .sig
// (public v0.3.3 shipped setup.exe + .sig without a separate .nsis.zip).
const builtNsisZipName = pickExactOne(present, nsisZipCandidates, "NSIS updater zip");
let updaterName;
if (builtNsisZipName) {
  updaterName = builtNsisZipName;
} else if (present.has(`${builtInstallerName}.sig`)) {
  updaterName = builtInstallerName;
} else {
  throw new Error(
    `Tauri updater package or signature not found for v${version}. ` +
      `Expected one of: ${nsisZipCandidates.join(", ")} (+ .sig), ` +
      `or ${builtInstallerName}.sig when no version-scoped .nsis.zip is present.`
  );
}

const builtUpdater = join(bundleDir, updaterName);
const builtSignature = `${builtUpdater}.sig`;
if (!existsSync(builtSignature)) {
  throw new Error(`Updater signature missing: ${builtSignature}`);
}

await mkdir(releaseDir, { recursive: true });
const installerName = publicInstallerName;
const updaterAssetName = updaterName.endsWith(".nsis.zip")
  ? publicUpdaterZipName
  : installerName;
const signatureName = `${updaterAssetName}.sig`;
await copyFile(builtInstaller, join(releaseDir, installerName));
if (resolve(builtUpdater) !== resolve(builtInstaller)) {
  await copyFile(builtUpdater, join(releaseDir, updaterAssetName));
}
await copyFile(builtSignature, join(releaseDir, signatureName));
await copyFile(join(projectRoot, "SBOM.spdx.json"), join(releaseDir, "SBOM.spdx.json"));

const signature = (await readFile(builtSignature, "utf8")).trim();
const notes = await readFile(join(projectRoot, "docs", "V0.5.0-UPDATER-NOTES.md"), "utf8");
const latest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature,
      url: `https://github.com/hangokudao/vod-scout/releases/download/v${version}/${updaterAssetName}`
    }
  }
};
await writeFile(join(releaseDir, "latest.json"), `${JSON.stringify(latest, null, 2)}\n`, "utf8");

async function sha256(path) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest("hex").toUpperCase();
}

const checksumFiles = [installerName, updaterAssetName, signatureName, "latest.json", "SBOM.spdx.json"]
  .filter((name, index, values) => values.indexOf(name) === index);
const sums = [];
for (const name of checksumFiles) sums.push(`${await sha256(join(releaseDir, name))}  ${name}`);
await writeFile(join(releaseDir, "SHA256SUMS.txt"), `${sums.join("\n")}\n`, "utf8");
process.stdout.write(`release assets prepared in ${releaseDir}\n${checksumFiles.map((name) => basename(name)).join("\n")}\n`);
