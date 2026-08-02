import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(await readFile(join(projectRoot, "src-tauri", "resources", "media-tools", "manifest.json"), "utf8"));
const pinnedUrl = manifest.artifacts?.ytDlp?.url ?? "";
const pinnedVersion = pinnedUrl.match(/\/download\/([^/]+)\//)?.[1];
if (!pinnedVersion) throw new Error("manifest에서 yt-dlp 고정 버전을 찾지 못했습니다.");

const response = await fetch("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest", {
  headers: { Accept: "application/vnd.github+json", "User-Agent": "vod-scout-release-check" },
  signal: AbortSignal.timeout(15_000)
});
if (!response.ok) throw new Error(`yt-dlp latest 확인 실패: HTTP ${response.status}`);
const latest = await response.json();
if (latest.prerelease || latest.draft) throw new Error("yt-dlp latest가 안정 릴리스가 아닙니다.");

const executable = join(projectRoot, "src-tauri", "resources", "media-tools", "yt-dlp", "yt-dlp.exe");
const bundledVersion = execFileSync(executable, ["--version"], {
  encoding: "utf8",
  timeout: 15_000,
  windowsHide: true,
  env: { SystemRoot: process.env.SystemRoot, WINDIR: process.env.WINDIR }
}).trim();
const result = {
  pinnedVersion,
  bundledVersion,
  latestStableVersion: latest.tag_name,
  rollbackControl: pinnedVersion,
  status: pinnedVersion === latest.tag_name && bundledVersion === pinnedVersion ? "PASS" : "REVIEW_REQUIRED"
};
process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
if (result.status !== "PASS") process.exitCode = 2;
