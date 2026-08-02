import { createHash } from "node:crypto";
import { createReadStream, createWriteStream } from "node:fs";
import { copyFile, cp, mkdir, readdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { validateArchiveEntries } from "./archive-safety.mjs";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const resourceRoot = join(projectRoot, "src-tauri", "resources", "media-tools");
const cacheRoot = join(tmpdir(), "vod-scout-media-tools-v1");

const artifacts = {
  ffmpeg: {
    url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n8.1-latest-win64-lgpl-shared-8.1.zip",
    sha256: "d0e27b409599525395a0982c88a4597d0f764ef038aeb20ed5dbca74b3654fbf",
    archive: "ffmpeg-n8.1-win64-lgpl-shared.zip"
  },
  whisper: {
    url: "https://github.com/ggml-org/whisper.cpp/releases/download/v1.9.1/whisper-bin-x64.zip",
    sha256: "7d8be46ecd31828e1eb7a2ecdd0d6b314feafd82163038ab6092594b0a063539",
    archive: "whisper-bin-x64-v1.9.1.zip"
  },
  model: {
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin?download=true",
    sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
    archive: "ggml-base.bin"
  },
  whisperLicense: {
    url: "https://raw.githubusercontent.com/ggml-org/whisper.cpp/v1.9.1/LICENSE",
    sha256: "94f29bbed6a22c35b992c5c6ebf0e7c92f13b836b90f36f461c9cf2f0f1d010d",
    archive: "whisper.cpp-LICENSE.txt"
  },
  modelLicense: {
    url: "https://raw.githubusercontent.com/openai/whisper/6e3be77e1a105e59086e3e21ff5f609fd6fa89a5/LICENSE",
    sha256: "b5d65a59060e68c4ff940e1eddfa6f94b2d68fdf58ed7f4dd57721c997e35e9d",
    archive: "OpenAI-Whisper-LICENSE.txt"
  },
  ytDlp: {
    url: "https://github.com/yt-dlp/yt-dlp/releases/download/2026.07.04/yt-dlp.exe",
    sha256: "52fe3c26dcf71fbdc85b528589020bb0b8e383155cfa81b64dd447bbe35e24b8",
    archive: "yt-dlp-2026.07.04.exe"
  },
  deno: {
    url: "https://github.com/denoland/deno/releases/download/v2.9.4/deno-x86_64-pc-windows-msvc.zip",
    sha256: "68ed08b05c56cf887e9aa509947dc3f468f7e12f47a13e5c1abd51d46d1453ef",
    archive: "deno-x86_64-pc-windows-msvc-v2.9.4.zip"
  },
  ytDlpLicense: {
    url: "https://raw.githubusercontent.com/yt-dlp/yt-dlp/2026.07.04/LICENSE",
    sha256: "7e12e5df4bae12cb21581ba157ced20e1986a0508dd10d0e8a4ab9a4cf94e85c",
    archive: "yt-dlp-LICENSE.txt"
  },
  denoLicense: {
    url: "https://raw.githubusercontent.com/denoland/deno/v2.9.4/LICENSE.md",
    sha256: "f62497fffecc0852960c8d3e6934b9db86d16396e9b604072e923892cae3a588",
    archive: "Deno-LICENSE.md"
  }
};

async function sha256(path) {
  const hash = createHash("sha256");
  await pipeline(createReadStream(path), hash);
  return hash.digest("hex");
}

async function download(spec) {
  const destination = join(cacheRoot, spec.archive);
  try {
    if ((await sha256(destination)) === spec.sha256) return destination;
  } catch {
    // Cache miss: download below.
  }

  const response = await fetch(spec.url, { redirect: "follow" });
  if (!response.ok || !response.body) {
    throw new Error(`다운로드 실패 (${response.status}): ${spec.url}`);
  }
  const temporary = `${destination}.part`;
  await pipeline(response.body, createWriteStream(temporary));
  const actual = await sha256(temporary);
  if (actual !== spec.sha256) {
    await rm(temporary, { force: true });
    throw new Error(`${basename(destination)} SHA-256 불일치: ${actual}`);
  }
  await rm(destination, { force: true });
  await copyFile(temporary, destination);
  await rm(temporary, { force: true });
  return destination;
}

async function findFile(root, target) {
  for (const entry of await readdir(root, { withFileTypes: true })) {
    const path = join(root, entry.name);
    if (entry.isFile() && entry.name.toLowerCase() === target.toLowerCase()) return path;
    if (entry.isDirectory()) {
      const found = await findFile(path, target);
      if (found) return found;
    }
  }
  return null;
}

async function extractZip(archive, destination) {
  const entries = execFileSync("tar.exe", ["-tf", archive], { encoding: "utf8", maxBuffer: 16 * 1024 * 1024 })
    .split(/\r?\n/)
    .filter(Boolean);
  validateArchiveEntries(entries);
  await rm(destination, { recursive: true, force: true });
  await mkdir(destination, { recursive: true });
  execFileSync("tar.exe", ["-xf", archive, "-C", destination], { stdio: "inherit" });
  const pending = [destination];
  while (pending.length) {
    const current = pending.pop();
    for (const entry of await readdir(current, { withFileTypes: true })) {
      if (entry.isSymbolicLink()) throw new Error(`압축 파일의 링크 항목을 허용하지 않습니다: ${entry.name}`);
      if (entry.isDirectory()) pending.push(join(current, entry.name));
    }
  }
}

const runtimeDirectories = ["ffmpeg", "whisper", "models", "yt-dlp", "deno"];

async function collectRuntimeFiles() {
  const files = [];
  async function visit(relativeDirectory) {
    const absoluteDirectory = join(resourceRoot, relativeDirectory);
    for (const entry of await readdir(absoluteDirectory, { withFileTypes: true })) {
      const relative = join(relativeDirectory, entry.name);
      if (entry.isSymbolicLink()) throw new Error(`runtime 링크 파일을 허용하지 않습니다: ${relative}`);
      if (entry.isDirectory()) await visit(relative);
      else if (entry.isFile()) files.push(relative.replaceAll("\\", "/"));
    }
  }
  for (const directory of runtimeDirectories) await visit(directory);
  return files.sort();
}

async function ready() {
  try {
    const manifest = JSON.parse(await readFile(join(resourceRoot, "manifest.json"), "utf8"));
    const required = [
      join(resourceRoot, "ffmpeg", "ffmpeg.exe"),
      join(resourceRoot, "ffmpeg", "ffprobe.exe"),
      join(resourceRoot, "whisper", "whisper-cli.exe"),
      join(resourceRoot, "models", "ggml-base.bin"),
      join(resourceRoot, "yt-dlp", "yt-dlp.exe"),
      join(resourceRoot, "deno", "deno.exe"),
      join(resourceRoot, "licenses", "FFmpeg-LGPL-3.0.txt"),
      join(resourceRoot, "licenses", "whisper.cpp-MIT.txt"),
      join(resourceRoot, "licenses", "OpenAI-Whisper-MIT.txt"),
      join(resourceRoot, "licenses", "yt-dlp-Unlicense.txt"),
      join(resourceRoot, "licenses", "Deno-MIT.md")
    ];
    const runtimeFiles = await collectRuntimeFiles();
    const manifestFiles = Object.keys(manifest.runtimeHashes ?? {}).sort();
    const runtimeHashesMatch = JSON.stringify(runtimeFiles) === JSON.stringify(manifestFiles)
      && (await Promise.all(runtimeFiles.map(async (relative) => manifest.runtimeHashes[relative] === await sha256(join(resourceRoot, relative))))).every(Boolean);
    return manifest.schemaVersion === 5
      && manifest.artifacts.ffmpeg.sha256 === artifacts.ffmpeg.sha256
      && manifest.artifacts.whisper.sha256 === artifacts.whisper.sha256
      && manifest.artifacts.model.sha256 === artifacts.model.sha256
      && manifest.artifacts.whisperLicense.sha256 === artifacts.whisperLicense.sha256
      && manifest.artifacts.modelLicense.sha256 === artifacts.modelLicense.sha256
      && manifest.artifacts.ytDlp.sha256 === artifacts.ytDlp.sha256
      && manifest.artifacts.deno.sha256 === artifacts.deno.sha256
      && manifest.artifacts.ytDlpLicense.sha256 === artifacts.ytDlpLicense.sha256
      && manifest.artifacts.denoLicense.sha256 === artifacts.denoLicense.sha256
      && runtimeHashesMatch
      && (await Promise.all(required.map(async (path) => (await stat(path)).isFile()))).every(Boolean);
  } catch {
    return false;
  }
}

if (await ready()) {
  process.stdout.write("media tools already prepared\n");
  process.exit(0);
}

await mkdir(cacheRoot, { recursive: true });
const [ffmpegArchive, whisperArchive, modelFile, whisperLicense, modelLicense, ytDlpExe, denoArchive, ytDlpLicense, denoLicense] = await Promise.all([
  download(artifacts.ffmpeg),
  download(artifacts.whisper),
  download(artifacts.model),
  download(artifacts.whisperLicense),
  download(artifacts.modelLicense),
  download(artifacts.ytDlp),
  download(artifacts.deno),
  download(artifacts.ytDlpLicense),
  download(artifacts.denoLicense)
]);

const staging = join(cacheRoot, "staging");
const ffmpegExtracted = join(staging, "ffmpeg");
const whisperExtracted = join(staging, "whisper");
const denoExtracted = join(staging, "deno");
await extractZip(ffmpegArchive, ffmpegExtracted);
await extractZip(whisperArchive, whisperExtracted);
await extractZip(denoArchive, denoExtracted);

const ffmpegExe = await findFile(ffmpegExtracted, "ffmpeg.exe");
const ffprobeExe = await findFile(ffmpegExtracted, "ffprobe.exe");
const ffmpegLicense = await findFile(ffmpegExtracted, "LICENSE.txt");
const whisperExe = await findFile(whisperExtracted, "whisper-cli.exe");
const denoExe = await findFile(denoExtracted, "deno.exe");
if (!ffmpegExe || !ffprobeExe || !ffmpegLicense || !whisperExe || !denoExe) {
  throw new Error("압축 파일에서 필요한 실행 파일을 찾지 못했습니다.");
}

await rm(resourceRoot, { recursive: true, force: true });
await mkdir(join(resourceRoot, "ffmpeg"), { recursive: true });
await mkdir(join(resourceRoot, "whisper"), { recursive: true });
await mkdir(join(resourceRoot, "models"), { recursive: true });
await mkdir(join(resourceRoot, "yt-dlp"), { recursive: true });
await mkdir(join(resourceRoot, "deno"), { recursive: true });
await mkdir(join(resourceRoot, "licenses"), { recursive: true });

await cp(dirname(ffmpegExe), join(resourceRoot, "ffmpeg"), { recursive: true });
for (const entry of await readdir(dirname(whisperExe), { withFileTypes: true })) {
  if (entry.isFile() && (entry.name.toLowerCase().endsWith(".dll") || entry.name === "whisper-cli.exe")) {
    await copyFile(join(dirname(whisperExe), entry.name), join(resourceRoot, "whisper", entry.name));
  }
}
await copyFile(modelFile, join(resourceRoot, "models", "ggml-base.bin"));
await copyFile(ytDlpExe, join(resourceRoot, "yt-dlp", "yt-dlp.exe"));
await copyFile(denoExe, join(resourceRoot, "deno", "deno.exe"));
await copyFile(ffmpegLicense, join(resourceRoot, "licenses", "FFmpeg-LGPL-3.0.txt"));
await copyFile(whisperLicense, join(resourceRoot, "licenses", "whisper.cpp-MIT.txt"));
await copyFile(modelLicense, join(resourceRoot, "licenses", "OpenAI-Whisper-MIT.txt"));
await copyFile(ytDlpLicense, join(resourceRoot, "licenses", "yt-dlp-Unlicense.txt"));
await copyFile(denoLicense, join(resourceRoot, "licenses", "Deno-MIT.md"));

const runtimeFiles = await collectRuntimeFiles();
const runtimeHashes = Object.fromEntries(await Promise.all(
  runtimeFiles.map(async (relative) => [relative, await sha256(join(resourceRoot, relative))])
));

await writeFile(join(resourceRoot, "manifest.json"), JSON.stringify({
  schemaVersion: 5,
  preparedAt: new Date().toISOString(),
  artifacts,
  runtimeHashes
}, null, 2));

process.stdout.write(`media tools prepared at ${resourceRoot}\n`);
