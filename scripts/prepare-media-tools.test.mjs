import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("./prepare-media-tools.mjs", import.meta.url), "utf8");

test("media preparation pins and prepares the CUDA 11.8 runtime", () => {
  assert.match(source, /autobuild-2026-08-17-13-05\/ffmpeg-n8\.1\.2-44-g7c533d0f86-win64-lgpl-shared-8\.1\.zip/);
  assert.match(source, /681b9ca6d8f9be1e01d8873ad16f8a632f8a22b9653f1044837de6d5979b0fd6/);
  assert.match(source, /whisperGpu:\s*\{[\s\S]*?whisper-cublas-11\.8\.0/);
  assert.match(source, /runtimeDirectories = \[[^\]]*"whisper-gpu"/);
  assert.match(source, /manifest\.schemaVersion === 6/);
  assert.doesNotMatch(source, /whisperGpu\.prepare|prepare:\s*false/);
});

test("GPU executable is required before a runtime manifest is generated", () => {
  assert.match(source, /whisperGpuExe/);
  assert.match(source, /!whisperGpuExe/);
  assert.match(source, /schemaVersion: 6/);
});
