import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const snapshotPath = process.argv[2];
if (!snapshotPath) throw new Error("작업 snapshot 경로가 필요합니다.");
const snapshot = JSON.parse(await readFile(snapshotPath, "utf8"));
const source = new URL(snapshot.sourceLabel);
if (source.protocol !== "https:" || !["youtube.com", "www.youtube.com", "youtu.be", "m.youtube.com"].includes(source.hostname)) {
  throw new Error("검증된 YouTube 단일 영상 주소가 아닙니다.");
}
const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const tools = join(projectRoot, "src-tauri", "resources", "media-tools");
const ytDlp = join(tools, "yt-dlp", "yt-dlp.exe");
const deno = join(tools, "deno", "deno.exe");
const ffmpeg = join(tools, "ffmpeg");
const result = JSON.parse(execFileSync(ytDlp, [
  "--simulate",
  "--dump-single-json",
  "--no-playlist",
  "--no-warnings",
  "--js-runtimes", `deno:${deno}`,
  "--ffmpeg-location", ffmpeg,
  source.href
], {
  encoding: "utf8",
  timeout: 60_000,
  windowsHide: true,
  maxBuffer: 16 * 1024 * 1024,
  env: { SystemRoot: process.env.SystemRoot, WINDIR: process.env.WINDIR, TEMP: process.env.TEMP, TMP: process.env.TMP }
}));
if (!result.id || !Number.isFinite(result.duration) || result.duration < 3600) {
  throw new Error("1시간 이상 공개 영상 metadata를 확인하지 못했습니다.");
}
process.stdout.write(`${JSON.stringify({ status: "PASS", durationSeconds: result.duration, extractor: result.extractor_key })}\n`);
