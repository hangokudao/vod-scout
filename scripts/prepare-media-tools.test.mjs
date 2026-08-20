import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("./prepare-media-tools.mjs", import.meta.url), "utf8");
const manifest = JSON.parse(await readFile(new URL("../src-tauri/resources/media-tools/manifest.json", import.meta.url), "utf8"));

test("media preparation pins and prepares the CUDA 11.8 runtime", () => {
  assert.match(source, /autobuild-2026-08-17-13-05\/ffmpeg-n8\.1\.2-44-g7c533d0f86-win64-lgpl-shared-8\.1\.zip/);
  assert.match(source, /681b9ca6d8f9be1e01d8873ad16f8a632f8a22b9653f1044837de6d5979b0fd6/);
  assert.match(source, /whisperGpu:\s*\{[\s\S]*?whisper-cublas-11\.8\.0/);
  assert.match(source, /whisperGpuCublas:\s*\{[\s\S]*?developer\.download\.nvidia\.com[\s\S]*?libcublas-windows-x86_64-11\.11\.3\.6-archive\.zip/);
  assert.match(source, /whisperGpuCublas:[\s\S]*?version: "11\.11\.3\.6"[\s\S]*?license: "CUDA Toolkit"[\s\S]*?licenseUrl: "https:\/\/docs\.nvidia\.com\/cuda\/archive\/11\.8\.0\/eula\/index\.html"[\s\S]*?licenseSha256: "17a280713a9cf1930d0f3a946935ca968d9726a64f1a41c9a589a959a673784f"[\s\S]*?67b0934a6359e4ee26fff823c356021589d392c4fd49ca12624f570edc08e2b9/);
  assert.match(source, /runtimeDirectories = \[[^\]]*"whisper-gpu"/);
  assert.match(source, /manifest\.schemaVersion === 6/);
  assert.doesNotMatch(source, /whisperGpu\.prepare|prepare:\s*false/);
});

test("GPU executable is required before a runtime manifest is generated", () => {
  assert.match(source, /whisperGpuExe/);
  assert.match(source, /!whisperGpuExe/);
  assert.match(source, /cublasDll/);
  assert.match(source, /cublasLtDll/);
  assert.match(source, /cublasLicense/);
  assert.match(source, /NVIDIA-CUDA-Toolkit\.txt/);
  assert.match(source, /normalizedCublasLicense/);
  assert.match(source, /schemaVersion: 6/);
});

test("yt-dlp nightly provenance and checksum remain pinned", () => {
  const ytDlp = manifest.artifacts.ytDlp;
  assert.equal(ytDlp.channel, "nightly");
  assert.equal(ytDlp.repo, "yt-dlp/yt-dlp-nightly-builds");
  assert.equal(ytDlp.sourceRepo, "yt-dlp/yt-dlp");
  assert.equal(ytDlp.sourceCommit, "594bd50c2c78ac432f81600d309fdc4e0a92d82c");
  assert.equal(ytDlp.sha256, "02bcc69a2a65a2af5da81a79356763522b611edc028c476e78c282735e28d442");
  assert.equal(ytDlp.checksumSha256, "6b2471fa596aaa588446fb0dfcf6025f8533d9dc931fa034b21c40c431473ce6");
  assert.equal(ytDlp.license, "Unlicense");
  assert.match(ytDlp.executableLicenseNotice, /GPL-3\.0-or-later PyInstaller combined work/);
  assert.equal(ytDlp.licenseSha256, "7e12e5df4bae12cb21581ba157ced20e1986a0508dd10d0e8a4ab9a4cf94e85c");
  assert.equal(manifest.artifacts.ytDlpThirdPartyLicenses.sourceCommit, ytDlp.sourceCommit);
  assert.equal(manifest.artifacts.ytDlpThirdPartyLicenses.sha256, "472aefe951c7db35e1657c1d13fd337140511ed6f2b329205105ad441c5a02b7");
  assert.equal(manifest.licenseHashes["licenses/yt-dlp-THIRD_PARTY_LICENSES.txt"], manifest.artifacts.ytDlpThirdPartyLicenses.sha256);
  assert.equal(manifest.runtimeHashes["yt-dlp/yt-dlp.exe"], ytDlp.sha256);
  assert.match(source, /yt-dlp-nightly-2026\.08\.19\.233000\.exe/);
  assert.match(source, /sourceCommit: "594bd50c2c78ac432f81600d309fdc4e0a92d82c"/);
  assert.match(source, /checksumUrl: "https:\/\/github\.com\/yt-dlp\/yt-dlp-nightly-builds/);
  assert.match(source, /ytDlpThirdPartyLicenses:[\s\S]*THIRD_PARTY_LICENSES\.txt/);
  assert.match(source, /licenseHashes/);
});
