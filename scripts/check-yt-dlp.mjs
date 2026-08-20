import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(await readFile(join(projectRoot, "src-tauri", "resources", "media-tools", "manifest.json"), "utf8"));
const ytDlp = manifest.artifacts?.ytDlp ?? {};
const pinnedVersion = ytDlp.url?.match(/\/download\/([^/]+)\/yt-dlp\.exe$/)?.[1];
if (!pinnedVersion || ytDlp.channel !== "nightly" || !ytDlp.repo || !ytDlp.sourceRepo || !ytDlp.sourceCommit
  || !ytDlp.executableLicenseNotice?.includes("GPL-3.0-or-later")) {
  throw new Error("manifest에 yt-dlp nightly provenance pin이 없습니다.");
}

const headers = { Accept: "application/vnd.github+json", "User-Agent": "vod-scout-release-check" };
async function fetchJson(url) {
  const response = await fetch(url, { headers, signal: AbortSignal.timeout(15_000) });
  if (!response.ok) throw new Error(`yt-dlp provenance 확인 실패: HTTP ${response.status} (${url})`);
  return response.json();
}
async function fetchBytes(url) {
  const response = await fetch(url, { headers, signal: AbortSignal.timeout(15_000) });
  if (!response.ok) throw new Error(`yt-dlp provenance asset 확인 실패: HTTP ${response.status} (${url})`);
  return Buffer.from(await response.arrayBuffer());
}
function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}
function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

const release = await fetchJson(`https://api.github.com/repos/${ytDlp.repo}/releases/tags/${pinnedVersion}`);
const latest = await fetchJson(`https://api.github.com/repos/${ytDlp.repo}/releases/latest`);
const latestNightlyVerified = latest.tag_name === pinnedVersion && !latest.draft && !latest.prerelease;
if (!latestNightlyVerified) throw new Error(`yt-dlp nightly pin ${pinnedVersion}이 current latest ${latest.tag_name ?? "unknown"}과 다릅니다.`);
const releaseAsset = release.assets?.find((asset) => asset.name === "yt-dlp.exe");
const checksumAsset = release.assets?.find((asset) => asset.name === "SHA2-256SUMS");
if (release.tag_name !== pinnedVersion || release.draft || !releaseAsset || !checksumAsset
  || releaseAsset.browser_download_url !== ytDlp.url
  || checksumAsset.browser_download_url !== ytDlp.checksumUrl) {
  throw new Error("yt-dlp nightly release asset/provenance가 예상과 다릅니다.");
}
const checksumBytes = await fetchBytes(ytDlp.checksumUrl);
const checksumText = checksumBytes.toString("utf8");
const checksumMatch = checksumText.match(new RegExp(`^([0-9a-f]{64})\\s+${escapeRegExp(releaseAsset.name)}$`, "m"));
const checksumSha256 = sha256(checksumBytes);
if (!checksumMatch || checksumMatch[1] !== ytDlp.sha256 || checksumSha256 !== ytDlp.checksumSha256) {
  throw new Error("yt-dlp nightly SHA2-256SUMS 또는 checksum 해시가 manifest와 다릅니다.");
}
if (releaseAsset.digest && releaseAsset.digest !== `sha256:${ytDlp.sha256}`) {
  throw new Error("GitHub release asset digest가 manifest와 다릅니다.");
}
const sourceCommit = await fetchJson(`https://api.github.com/repos/${ytDlp.sourceRepo}/commits/${ytDlp.sourceCommit}`);
const sourceCommitVerified = sourceCommit.sha === ytDlp.sourceCommit
  && release.body?.includes(ytDlp.sourceCommitUrl)
  && release.body?.includes(`Generated from: ${ytDlp.sourceCommitUrl}`);
if (!sourceCommitVerified) throw new Error("yt-dlp source commit provenance가 release와 다릅니다.");
const remoteLicenseSha256 = sha256(await fetchBytes(ytDlp.licenseUrl));
if (remoteLicenseSha256 !== ytDlp.licenseSha256) throw new Error("yt-dlp LICENSE 해시가 manifest와 다릅니다.");
const thirdPartyLicense = manifest.artifacts?.ytDlpThirdPartyLicenses ?? {};
const remoteThirdPartyLicenseSha256 = sha256(await fetchBytes(thirdPartyLicense.url));
if (thirdPartyLicense.sourceCommit !== ytDlp.sourceCommit
  || remoteThirdPartyLicenseSha256 !== thirdPartyLicense.sha256) {
  throw new Error("yt-dlp THIRD_PARTY_LICENSES provenance 또는 해시가 manifest와 다릅니다.");
}

const executable = join(projectRoot, "src-tauri", "resources", "media-tools", "yt-dlp", "yt-dlp.exe");
const licenseFile = join(projectRoot, "src-tauri", "resources", "media-tools", "licenses", "yt-dlp-Unlicense.txt");
const thirdPartyLicenseFile = join(projectRoot, "src-tauri", "resources", "media-tools", "licenses", "yt-dlp-THIRD_PARTY_LICENSES.txt");
const bundledVersion = execFileSync(executable, ["--ignore-config", "--version"], {
  encoding: "utf8",
  timeout: 15_000,
  windowsHide: true,
  env: { SystemRoot: process.env.SystemRoot, WINDIR: process.env.WINDIR }
}).trim();
const binarySha256 = sha256(await readFile(executable));
const localLicenseSha256 = sha256(await readFile(licenseFile));
const localThirdPartyLicenseSha256 = sha256(await readFile(thirdPartyLicenseFile));
const status = bundledVersion === pinnedVersion
  && binarySha256 === ytDlp.sha256
  && localLicenseSha256 === ytDlp.licenseSha256
  && manifest.licenseHashes?.["licenses/yt-dlp-THIRD_PARTY_LICENSES.txt"] === thirdPartyLicense.sha256
  && localThirdPartyLicenseSha256 === thirdPartyLicense.sha256
  ? "PASS"
  : "REVIEW_REQUIRED";
const result = {
  status,
  channel: ytDlp.channel,
  repo: ytDlp.repo,
  pinnedVersion,
  latestNightlyVersion: latest.tag_name,
  latestNightlyVerified,
  bundledVersion,
  binarySha256,
  checksumSha256,
  licenseSha256: localLicenseSha256,
  thirdPartyLicenseSha256: localThirdPartyLicenseSha256,
  sourceCommit: ytDlp.sourceCommit,
  sourceCommitVerified,
  releaseAssetSize: releaseAsset.size,
  releaseAssetDigest: releaseAsset.digest ?? null,
  rollbackControl: pinnedVersion
};
process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
if (status !== "PASS") process.exitCode = 2;
