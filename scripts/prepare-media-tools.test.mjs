import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("./prepare-media-tools.mjs", import.meta.url), "utf8");

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
