import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { copyFile, mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const version = "0.3.2";
const bundleDir = join(projectRoot, "src-tauri", "target", "release", "bundle", "nsis");
const releaseDir = join(projectRoot, "release", `v${version}`);
const builtInstaller = join(bundleDir, `VOD Scout_${version}_x64-setup.exe`);
const bundleFiles = await readdir(bundleDir);
const updaterName = bundleFiles.find((name) => name.endsWith(".nsis.zip"))
  ?? bundleFiles.find((name) => name === `VOD Scout_${version}_x64-setup.exe` && bundleFiles.includes(`${name}.sig`));
if (!updaterName) throw new Error("Tauri updater 패키지를 찾지 못했습니다.");
const builtUpdater = join(bundleDir, updaterName);
const builtSignature = `${builtUpdater}.sig`;

await mkdir(releaseDir, { recursive: true });
const installerName = `VOD-Scout-${version}-windows-x64-setup.exe`;
const updaterAssetName = updaterName.endsWith(".nsis.zip")
  ? `VOD-Scout-${version}-windows-x64-setup.nsis.zip`
  : installerName;
const signatureName = `${updaterAssetName}.sig`;
await copyFile(builtInstaller, join(releaseDir, installerName));
if (resolve(builtUpdater) !== resolve(builtInstaller)) {
  await copyFile(builtUpdater, join(releaseDir, updaterAssetName));
}
await copyFile(builtSignature, join(releaseDir, signatureName));
await copyFile(join(projectRoot, "SBOM.spdx.json"), join(releaseDir, "SBOM.spdx.json"));

const signature = (await readFile(builtSignature, "utf8")).trim();
const notes = await readFile(join(projectRoot, "docs", "V0.3.2-UPDATER-NOTES.md"), "utf8");
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
