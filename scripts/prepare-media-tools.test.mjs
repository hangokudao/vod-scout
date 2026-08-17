import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("./prepare-media-tools.mjs", import.meta.url), "utf8");

test("media preparation pins and prepares the CUDA 11.8 runtime", () => {
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
