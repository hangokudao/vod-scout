import { execFileSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const npmCli = process.env.npm_execpath;
if (!npmCli) throw new Error("npm CLI 경로를 찾지 못했습니다. npm run sbom으로 실행해 주세요.");
const document = JSON.parse(execFileSync(process.execPath, [npmCli, "sbom", "--sbom-format", "spdx", "--sbom-type", "application"], {
  cwd: projectRoot,
  encoding: "utf8",
  maxBuffer: 32 * 1024 * 1024,
  windowsHide: true
}));
const rootId = document.documentDescribes[0];
const cargoLock = await readFile(join(projectRoot, "src-tauri", "Cargo.lock"), "utf8");
const packages = cargoLock.split("[[package]]").slice(1);
packages.forEach((block, index) => {
  const name = block.match(/^name = "([^"]+)"/m)?.[1];
  const version = block.match(/^version = "([^"]+)"/m)?.[1];
  if (!name || !version) return;
  const checksum = block.match(/^checksum = "([a-f0-9]+)"/m)?.[1];
  const id = `SPDXRef-Cargo-${name.replace(/[^A-Za-z0-9.-]/g, "-")}-${version}-${index}`;
  const entry = {
    name,
    SPDXID: id,
    versionInfo: version,
    primaryPackagePurpose: "LIBRARY",
    downloadLocation: "NOASSERTION",
    filesAnalyzed: false,
    licenseDeclared: "NOASSERTION",
    externalRefs: [{
      referenceCategory: "PACKAGE-MANAGER",
      referenceType: "purl",
      referenceLocator: `pkg:cargo/${encodeURIComponent(name)}@${encodeURIComponent(version)}`
    }]
  };
  if (checksum) entry.checksums = [{ algorithm: "SHA256", checksumValue: checksum }];
  document.packages.push(entry);
  document.relationships.push({
    spdxElementId: id,
    relatedSpdxElement: rootId,
    relationshipType: "DEPENDENCY_OF"
  });
});
document.creationInfo.creators.push("Tool: VOD Scout scripts/generate-sbom.mjs (Cargo.lock augmentation)");
await writeFile(join(projectRoot, "SBOM.spdx.json"), `${JSON.stringify(document, null, 2)}\n`, "utf8");
process.stdout.write(`SBOM.spdx.json generated with ${document.packages.length} packages\n`);
