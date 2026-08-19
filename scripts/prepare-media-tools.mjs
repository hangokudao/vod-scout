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
const cacheRoot = join(tmpdir(), "vod-scout-media-tools-v2");

const artifacts = {
  ffmpeg: {
    url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-08-17-13-05/ffmpeg-n8.1.2-44-g7c533d0f86-win64-lgpl-shared-8.1.zip",
    sha256: "681b9ca6d8f9be1e01d8873ad16f8a632f8a22b9653f1044837de6d5979b0fd6",
    archive: "ffmpeg-n8.1.2-44-g7c533d0f86-win64-lgpl-shared-8.1.zip"
  },
  whisper: {
    url: "https://github.com/ggml-org/whisper.cpp/releases/download/v1.9.1/whisper-bin-x64.zip",
    sha256: "7d8be46ecd31828e1eb7a2ecdd0d6b314feafd82163038ab6092594b0a063539",
    archive: "whisper-bin-x64-v1.9.1.zip"
  },
  whisperGpu: {
    url: "https://github.com/ggml-org/whisper.cpp/releases/download/v1.9.1/whisper-cublas-11.8.0-bin-x64.zip",
    sha256: "aecdce0e4d4bb758a7c72a31f3f9f19a7b6d861405fd2da743cd86398633c963",
    archive: "whisper-cublas-11.8.0-bin-x64-v1.9.1.zip"
  },
  whisperGpuCublas: {
    url: "https://developer.download.nvidia.com/compute/cuda/redist/libcublas/windows-x86_64/libcublas-windows-x86_64-11.11.3.6-archive.zip",
    version: "11.11.3.6",
    license: "CUDA Toolkit",
    licenseUrl: "https://docs.nvidia.com/cuda/archive/11.8.0/eula/index.html",
    licenseSha256: "17a280713a9cf1930d0f3a946935ca968d9726a64f1a41c9a589a959a673784f",
    sha256: "67b0934a6359e4ee26fff823c356021589d392c4fd49ca12624f570edc08e2b9",
    archive: "libcublas-windows-x86_64-11.11.3.6-archive.zip"
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
    url: "https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/download/2026.08.18.122307/yt-dlp.exe",
    sha256: "652e154bce7170070d0f26415c9a3c35c121f5a7903cb8cde6d31c4577517fb9",
    archive: "yt-dlp-nightly-2026.08.18.122307.exe",
    repo: "yt-dlp/yt-dlp-nightly-builds",
    channel: "nightly",
    sourceRepo: "yt-dlp/yt-dlp",
    sourceCommit: "5d5b634d8e6b41dc2891847a5ea7a5a3f569a28c",
    sourceCommitUrl: "https://github.com/yt-dlp/yt-dlp/commit/5d5b634d8e6b41dc2891847a5ea7a5a3f569a28c",
    checksumUrl: "https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/download/2026.08.18.122307/SHA2-256SUMS",
    checksumSha256: "e53fefb8bcec1b7bdbeaa77f662955528d530d76127ea42037c9fd1e6893c990",
    license: "Unlicense",
    executableLicenseNotice: "yt-dlp.exe is a GPL-3.0-or-later PyInstaller combined work; yt-dlp source is Unlicense and bundled components are listed in THIRD_PARTY_LICENSES.txt.",
    licenseUrl: "https://raw.githubusercontent.com/yt-dlp/yt-dlp/5d5b634d8e6b41dc2891847a5ea7a5a3f569a28c/LICENSE",
    licenseSha256: "7e12e5df4bae12cb21581ba157ced20e1986a0508dd10d0e8a4ab9a4cf94e85c"
  },
  deno: {
    url: "https://github.com/denoland/deno/releases/download/v2.9.4/deno-x86_64-pc-windows-msvc.zip",
    sha256: "68ed08b05c56cf887e9aa509947dc3f468f7e12f47a13e5c1abd51d46d1453ef",
    archive: "deno-x86_64-pc-windows-msvc-v2.9.4.zip"
  },
  ytDlpLicense: {
    url: "https://raw.githubusercontent.com/yt-dlp/yt-dlp/5d5b634d8e6b41dc2891847a5ea7a5a3f569a28c/LICENSE",
    sha256: "7e12e5df4bae12cb21581ba157ced20e1986a0508dd10d0e8a4ab9a4cf94e85c",
    archive: "yt-dlp-LICENSE.txt"
  },
  ytDlpThirdPartyLicenses: {
    url: "https://raw.githubusercontent.com/yt-dlp/yt-dlp/5d5b634d8e6b41dc2891847a5ea7a5a3f569a28c/THIRD_PARTY_LICENSES.txt",
    sha256: "472aefe951c7db35e1657c1d13fd337140511ed6f2b329205105ad441c5a02b7",
    sourceCommit: "5d5b634d8e6b41dc2891847a5ea7a5a3f569a28c",
    archive: "yt-dlp-THIRD_PARTY_LICENSES.txt"
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

const runtimeDirectories = ["ffmpeg", "whisper", "whisper-gpu", "models", "yt-dlp", "deno"];

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
      join(resourceRoot, "whisper-gpu", "whisper-cli.exe"),
      join(resourceRoot, "whisper-gpu", "cublas64_11.dll"),
      join(resourceRoot, "whisper-gpu", "cublasLt64_11.dll"),
      join(resourceRoot, "models", "ggml-base.bin"),
      join(resourceRoot, "yt-dlp", "yt-dlp.exe"),
      join(resourceRoot, "deno", "deno.exe"),
      join(resourceRoot, "licenses", "FFmpeg-LGPL-3.0.txt"),
      join(resourceRoot, "licenses", "whisper.cpp-MIT.txt"),
      join(resourceRoot, "licenses", "NVIDIA-CUDA-Toolkit.txt"),
      join(resourceRoot, "licenses", "OpenAI-Whisper-MIT.txt"),
      join(resourceRoot, "licenses", "yt-dlp-Unlicense.txt"),
      join(resourceRoot, "licenses", "yt-dlp-THIRD_PARTY_LICENSES.txt"),
      join(resourceRoot, "licenses", "Deno-MIT.md")
    ];
    const runtimeFiles = await collectRuntimeFiles();
    const manifestFiles = Object.keys(manifest.runtimeHashes ?? {}).sort();
    const runtimeHashesMatch = JSON.stringify(runtimeFiles) === JSON.stringify(manifestFiles)
      && (await Promise.all(runtimeFiles.map(async (relative) => manifest.runtimeHashes[relative] === await sha256(join(resourceRoot, relative))))).every(Boolean);
    return manifest.schemaVersion === 6
      && manifest.artifacts.ffmpeg.sha256 === artifacts.ffmpeg.sha256
      && manifest.artifacts.whisper.sha256 === artifacts.whisper.sha256
      && manifest.artifacts.whisperGpu.url === artifacts.whisperGpu.url
      && manifest.artifacts.whisperGpu.sha256 === artifacts.whisperGpu.sha256
      && manifest.artifacts.whisperGpuCublas.url === artifacts.whisperGpuCublas.url
      && manifest.artifacts.whisperGpuCublas.version === artifacts.whisperGpuCublas.version
      && manifest.artifacts.whisperGpuCublas.license === artifacts.whisperGpuCublas.license
      && manifest.artifacts.whisperGpuCublas.licenseUrl === artifacts.whisperGpuCublas.licenseUrl
      && manifest.artifacts.whisperGpuCublas.licenseSha256 === artifacts.whisperGpuCublas.licenseSha256
      && manifest.artifacts.whisperGpuCublas.sha256 === artifacts.whisperGpuCublas.sha256
      && manifest.artifacts.model.sha256 === artifacts.model.sha256
      && manifest.artifacts.whisperLicense.sha256 === artifacts.whisperLicense.sha256
      && manifest.artifacts.modelLicense.sha256 === artifacts.modelLicense.sha256
      && manifest.artifacts.ytDlp.url === artifacts.ytDlp.url
      && manifest.artifacts.ytDlp.repo === artifacts.ytDlp.repo
      && manifest.artifacts.ytDlp.channel === artifacts.ytDlp.channel
      && manifest.artifacts.ytDlp.sourceRepo === artifacts.ytDlp.sourceRepo
      && manifest.artifacts.ytDlp.sourceCommit === artifacts.ytDlp.sourceCommit
      && manifest.artifacts.ytDlp.sourceCommitUrl === artifacts.ytDlp.sourceCommitUrl
      && manifest.artifacts.ytDlp.checksumUrl === artifacts.ytDlp.checksumUrl
      && manifest.artifacts.ytDlp.checksumSha256 === artifacts.ytDlp.checksumSha256
      && manifest.artifacts.ytDlp.license === artifacts.ytDlp.license
      && manifest.artifacts.ytDlp.executableLicenseNotice === artifacts.ytDlp.executableLicenseNotice
      && manifest.artifacts.ytDlp.licenseUrl === artifacts.ytDlp.licenseUrl
      && manifest.artifacts.ytDlp.licenseSha256 === artifacts.ytDlp.licenseSha256
      && manifest.artifacts.ytDlp.sha256 === artifacts.ytDlp.sha256
      && manifest.artifacts.deno.sha256 === artifacts.deno.sha256
      && manifest.artifacts.ytDlpLicense.sha256 === artifacts.ytDlpLicense.sha256
      && manifest.artifacts.ytDlpLicense.url === artifacts.ytDlpLicense.url
      && manifest.artifacts.ytDlpThirdPartyLicenses.url === artifacts.ytDlpThirdPartyLicenses.url
      && manifest.artifacts.ytDlpThirdPartyLicenses.sha256 === artifacts.ytDlpThirdPartyLicenses.sha256
      && manifest.artifacts.ytDlpThirdPartyLicenses.sourceCommit === artifacts.ytDlpThirdPartyLicenses.sourceCommit
      && manifest.artifacts.denoLicense.sha256 === artifacts.denoLicense.sha256
      && manifest.licenseHashes?.["licenses/yt-dlp-THIRD_PARTY_LICENSES.txt"] === artifacts.ytDlpThirdPartyLicenses.sha256
      && (await sha256(join(resourceRoot, "licenses", "yt-dlp-THIRD_PARTY_LICENSES.txt"))) === artifacts.ytDlpThirdPartyLicenses.sha256
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
const [ffmpegArchive, whisperArchive, whisperGpuArchive, whisperGpuCublasArchive, modelFile, whisperLicense, modelLicense, ytDlpExe, denoArchive, ytDlpLicense, ytDlpThirdPartyLicenses, denoLicense] = await Promise.all([
  download(artifacts.ffmpeg),
  download(artifacts.whisper),
  download(artifacts.whisperGpu),
  download(artifacts.whisperGpuCublas),
  download(artifacts.model),
  download(artifacts.whisperLicense),
  download(artifacts.modelLicense),
  download(artifacts.ytDlp),
  download(artifacts.deno),
  download(artifacts.ytDlpLicense),
  download(artifacts.ytDlpThirdPartyLicenses),
  download(artifacts.denoLicense)
]);

const staging = join(cacheRoot, "staging");
const ffmpegExtracted = join(staging, "ffmpeg");
const whisperExtracted = join(staging, "whisper");
const whisperGpuExtracted = join(staging, "whisper-gpu");
const whisperGpuCublasExtracted = join(staging, "whisper-gpu-cublas");
const denoExtracted = join(staging, "deno");
await extractZip(ffmpegArchive, ffmpegExtracted);
await extractZip(whisperArchive, whisperExtracted);
await extractZip(whisperGpuArchive, whisperGpuExtracted);
await extractZip(whisperGpuCublasArchive, whisperGpuCublasExtracted);
await extractZip(denoArchive, denoExtracted);

const ffmpegExe = await findFile(ffmpegExtracted, "ffmpeg.exe");
const ffprobeExe = await findFile(ffmpegExtracted, "ffprobe.exe");
const ffmpegLicense = await findFile(ffmpegExtracted, "LICENSE.txt");
const whisperExe = await findFile(whisperExtracted, "whisper-cli.exe");
const whisperGpuExe = await findFile(whisperGpuExtracted, "whisper-cli.exe");
const cublasDll = await findFile(whisperGpuCublasExtracted, "cublas64_11.dll");
const cublasLtDll = await findFile(whisperGpuCublasExtracted, "cublasLt64_11.dll");
const cublasLicense = await findFile(whisperGpuCublasExtracted, "LICENSE");
const denoExe = await findFile(denoExtracted, "deno.exe");
if (!ffmpegExe || !ffprobeExe || !ffmpegLicense || !whisperExe || !whisperGpuExe || !cublasDll || !cublasLtDll || !cublasLicense || !denoExe) {
  throw new Error("압축 파일에서 필요한 실행 파일을 찾지 못했습니다.");
}

await rm(resourceRoot, { recursive: true, force: true });
await mkdir(join(resourceRoot, "ffmpeg"), { recursive: true });
await mkdir(join(resourceRoot, "whisper"), { recursive: true });
await mkdir(join(resourceRoot, "whisper-gpu"), { recursive: true });
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
for (const entry of await readdir(dirname(whisperGpuExe), { withFileTypes: true })) {
  if (entry.isFile() && (entry.name.toLowerCase().endsWith(".dll") || entry.name === "whisper-cli.exe")) {
    await copyFile(join(dirname(whisperGpuExe), entry.name), join(resourceRoot, "whisper-gpu", entry.name));
  }
}
await copyFile(cublasDll, join(resourceRoot, "whisper-gpu", "cublas64_11.dll"));
await copyFile(cublasLtDll, join(resourceRoot, "whisper-gpu", "cublasLt64_11.dll"));
await copyFile(modelFile, join(resourceRoot, "models", "ggml-base.bin"));
await copyFile(ytDlpExe, join(resourceRoot, "yt-dlp", "yt-dlp.exe"));
await copyFile(denoExe, join(resourceRoot, "deno", "deno.exe"));
await copyFile(ffmpegLicense, join(resourceRoot, "licenses", "FFmpeg-LGPL-3.0.txt"));
await copyFile(whisperLicense, join(resourceRoot, "licenses", "whisper.cpp-MIT.txt"));
const normalizedCublasLicense = (await readFile(cublasLicense, "utf8"))
  .replace(/\r\n/g, "\n")
  .replace(/[ \t]+$/gm, "");
await writeFile(join(resourceRoot, "licenses", "NVIDIA-CUDA-Toolkit.txt"), normalizedCublasLicense);
await copyFile(modelLicense, join(resourceRoot, "licenses", "OpenAI-Whisper-MIT.txt"));
await copyFile(ytDlpLicense, join(resourceRoot, "licenses", "yt-dlp-Unlicense.txt"));
await copyFile(ytDlpThirdPartyLicenses, join(resourceRoot, "licenses", "yt-dlp-THIRD_PARTY_LICENSES.txt"));
await copyFile(denoLicense, join(resourceRoot, "licenses", "Deno-MIT.md"));

const runtimeFiles = await collectRuntimeFiles();
const runtimeHashes = Object.fromEntries(await Promise.all(
  runtimeFiles.map(async (relative) => [relative, await sha256(join(resourceRoot, relative))])
));

await writeFile(join(resourceRoot, "manifest.json"), JSON.stringify({
  schemaVersion: 6,
  preparedAt: new Date().toISOString(),
  artifacts,
  licenseHashes: {
    "licenses/yt-dlp-THIRD_PARTY_LICENSES.txt": await sha256(join(resourceRoot, "licenses", "yt-dlp-THIRD_PARTY_LICENSES.txt"))
  },
  runtimeHashes
}, null, 2));

process.stdout.write(`media tools prepared at ${resourceRoot}\n`);
