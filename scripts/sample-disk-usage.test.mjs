/**
 * Focused smoke tests for scripts/sample-disk-usage.mjs.
 * Proves: read-only under --target, output-outside-target rejection, and
 * no interference with checkpoint replacement (live → .prev → new live).
 */
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";

const script = fileURLToPath(new URL("./sample-disk-usage.mjs", import.meta.url));

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function runSampler(args, timeoutMs = 15_000) {
  return spawnSync(process.execPath, [script, ...args], {
    encoding: "utf8",
    timeout: timeoutMs
  });
}

/** Mirror product replace_file_preserving_previous without importing Rust. */
function replacePreservingPrevious(path, bytes) {
  const temporary = `${path}.tmp`;
  const previous = `${path}.prev`;
  writeFileSync(temporary, bytes);
  if (existsSync(path)) {
    if (existsSync(previous)) {
      rmSync(previous);
    }
    renameSync(path, previous);
  }
  renameSync(temporary, path);
}

test("sampler rejects output inside target", () => {
  const root = join(tmpdir(), `vod-scout-sampler-reject-${process.pid}`);
  mkdirSync(root, { recursive: true });
  try {
    const result = runSampler([
      "--target",
      root,
      "--output",
      join(root, "inside.ndjson"),
      "--duration-ms",
      "200"
    ]);
    assert.equal(result.status, 2);
    assert.match(result.stderr, /must not be inside --target/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("sampler does not mutate target while checkpoint replacement runs", async () => {
  const root = join(tmpdir(), `vod-scout-sampler-ckp-${process.pid}-${Date.now()}`);
  const target = join(root, "job");
  const outDir = join(root, "out");
  mkdirSync(target, { recursive: true });
  mkdirSync(outDir, { recursive: true });

  const checkpoint = join(target, "media-checkpoint.json");
  const acquisition = join(target, "acquisition.json");
  const generation1 = Buffer.from('{"schemaVersion":4,"completedChunks":0,"marker":"gen1"}\n');
  const generation2 = Buffer.from('{"schemaVersion":4,"completedChunks":1,"marker":"gen2"}\n');
  writeFileSync(checkpoint, generation1);
  writeFileSync(
    acquisition,
    Buffer.from('{"schemaVersion":1,"sourceUrl":"https://youtu.be/x"}\n')
  );
  const acqHashBefore = sha256(acquisition);
  writeFileSync(join(target, "source.part"), Buffer.alloc(4096));

  const output = join(outDir, "samples.ndjson");
  const stopFile = join(outDir, "stop.flag");

  const child = spawn(
    process.execPath,
    [
      script,
      "--target",
      target,
      "--output",
      output,
      "--interval-ms",
      "100",
      "--stop-file",
      stopFile,
      "--duration-ms",
      "8000"
    ],
    { stdio: ["ignore", "pipe", "pipe"] }
  );

  let stderr = "";
  child.stderr.on("data", (chunk) => {
    stderr += chunk.toString();
  });

  try {
    await delay(150);
    replacePreservingPrevious(checkpoint, generation2);
    writeFileSync(stopFile, "1");

    const code = await new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        child.kill("SIGTERM");
        reject(new Error(`sampler timed out; stderr=${stderr}`));
      }, 12_000);
      child.on("error", reject);
      child.on("close", (exitCode) => {
        clearTimeout(timer);
        resolve(exitCode);
      });
    });

    assert.equal(code, 0, `sampler exit ${code}; stderr=${stderr}`);
    assert.equal(readFileSync(checkpoint, "utf8"), generation2.toString("utf8"));
    assert.equal(readFileSync(`${checkpoint}.prev`, "utf8"), generation1.toString("utf8"));
    assert.equal(
      sha256(acquisition),
      acqHashBefore,
      "unrelated checkpoint file must stay byte-identical"
    );
    assert.ok(existsSync(output), "NDJSON samples written");
    assert.ok(existsSync(`${output}.summary.json`), "summary written outside target");
    const summary = JSON.parse(readFileSync(`${output}.summary.json`, "utf8"));
    assert.ok(summary.sampleCount >= 1);
    assert.ok(summary.peak.totalBytes >= generation2.length);
    assert.equal(summary.stopReason === "stop-file" || summary.stopReason === "duration", true);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("standalone replace_preserving_previous helper stays coherent", () => {
  const root = join(tmpdir(), `vod-scout-sampler-helper-${process.pid}`);
  mkdirSync(root, { recursive: true });
  const path = join(root, "media-checkpoint.json");
  replacePreservingPrevious(path, Buffer.from('{"v":1}'));
  replacePreservingPrevious(path, Buffer.from('{"v":2}'));
  assert.equal(readFileSync(path, "utf8"), '{"v":2}');
  assert.equal(readFileSync(`${path}.prev`, "utf8"), '{"v":1}');
  rmSync(root, { recursive: true, force: true });
});
