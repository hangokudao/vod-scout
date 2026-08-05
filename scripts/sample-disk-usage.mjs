#!/usr/bin/env node
/**
 * Out-of-product disk usage sampler for YouTube download/merge peak temporary size.
 *
 * Recursively sums FileInfo/stat sizes without opening file contents, so files
 * currently open by yt-dlp/ffmpeg during merge are included. Never mutates the
 * measured tree. Samples are written incrementally as NDJSON; a final summary
 * is written on duration expiry, stop signal, or clean Ctrl+C termination.
 *
 * Usage:
 *   node scripts/sample-disk-usage.mjs --target <dir> --output <path>
 *     [--interval-ms <n>] [--duration-ms <n>] [--stop-file <path>]
 *     [--largest <n>]
 */

import {
  appendFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  writeFileSync
} from "node:fs";
import { dirname, join, normalize, resolve, sep } from "node:path";

const DEFAULT_INTERVAL_MS = 1000;
const MIN_INTERVAL_MS = 100;
const MAX_INTERVAL_MS = 60_000;
const MAX_DURATION_MS = 8 * 60 * 60 * 1000;
const DEFAULT_LARGEST = 8;
const MAX_LARGEST = 32;

function printHelp() {
  process.stdout.write(`sample-disk-usage.mjs — recursive disk usage sampler (read-only)

Required:
  --target <dir>       Directory to measure (must exist)
  --output <path>      Output path for NDJSON samples + .summary.json
                       Must NOT be inside --target

Options:
  --interval-ms <n>    Poll interval in ms (default ${DEFAULT_INTERVAL_MS},
                       min ${MIN_INTERVAL_MS}, max ${MAX_INTERVAL_MS})
  --duration-ms <n>    Stop after this many ms (max ${MAX_DURATION_MS}).
                       Required unless --stop-file is set.
  --stop-file <path>   Stop when this file exists. Required unless
                       --duration-ms is set. Must not be inside --target.
  --largest <n>        Keep top N largest files per sample
                       (default ${DEFAULT_LARGEST}, max ${MAX_LARGEST})
  --help               Show this help

Output:
  <output>             NDJSON lines, one sample object per poll
  <output>.summary.json
                       Final summary with peak/final/overhead fields

Each sample includes: timestamp, totalBytes, fileCount, dirCount,
skippedCount, disappearedCount, largestFiles[{path,bytes}].
`);
}

function fail(message) {
  process.stderr.write(`error: ${message}\n`);
  process.exit(2);
}

function parseArgs(argv) {
  const args = {
    target: null,
    output: null,
    intervalMs: DEFAULT_INTERVAL_MS,
    durationMs: null,
    stopFile: null,
    largest: DEFAULT_LARGEST,
    help: false
  };

  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i];
    const next = () => {
      const value = argv[i + 1];
      if (value === undefined || value.startsWith("--")) {
        fail(`missing value for ${token}`);
      }
      i += 1;
      return value;
    };

    switch (token) {
      case "--help":
      case "-h":
        args.help = true;
        break;
      case "--target":
        args.target = next();
        break;
      case "--output":
        args.output = next();
        break;
      case "--interval-ms":
        args.intervalMs = Number(next());
        break;
      case "--duration-ms":
        args.durationMs = Number(next());
        break;
      case "--stop-file":
        args.stopFile = next();
        break;
      case "--largest":
        args.largest = Number(next());
        break;
      default:
        fail(`unknown argument: ${token}`);
    }
  }

  return args;
}

function absPath(p) {
  return resolve(p);
}

/**
 * True if `inner` is the same as `outer` or a path under it.
 * Works on Windows drive letters and mixed separators.
 */
function isPathInsideOrEqual(outer, inner) {
  const a = normalize(absPath(outer)).replace(/[/\\]+$/, "");
  const b = normalize(absPath(inner)).replace(/[/\\]+$/, "");
  if (process.platform === "win32") {
    const al = a.toLowerCase();
    const bl = b.toLowerCase();
    if (al === bl) return true;
    const prefix = al.endsWith(sep) ? al : `${al}${sep}`;
    return bl.startsWith(prefix);
  }
  if (a === b) return true;
  const prefix = a.endsWith(sep) ? a : `${a}${sep}`;
  return b.startsWith(prefix);
}

function validatePositiveInt(name, value, min, max) {
  if (!Number.isInteger(value) || value < min || value > max) {
    fail(`${name} must be an integer in [${min}, ${max}], got ${value}`);
  }
}

/**
 * Recursively walk target, summing sizes from lstat only (no open/read of contents).
 * Open handles (yt-dlp/ffmpeg merge outputs) still report growing size via metadata.
 * Disappeared entries during walk are counted, not fatal. Symlinks are not followed.
 */
function measureTree(root, largestN) {
  let totalBytes = 0;
  let fileCount = 0;
  let dirCount = 0;
  let skippedCount = 0;
  let disappearedCount = 0;
  /** @type {{ path: string, bytes: number }[]} */
  const largest = [];

  function considerLargest(filePath, bytes) {
    if (largestN <= 0) return;
    if (largest.length < largestN) {
      largest.push({ path: filePath, bytes });
      largest.sort((a, b) => b.bytes - a.bytes);
      return;
    }
    if (bytes <= largest[largest.length - 1].bytes) return;
    largest[largest.length - 1] = { path: filePath, bytes };
    largest.sort((a, b) => b.bytes - a.bytes);
  }

  function walk(dir) {
    let entries;
    try {
      entries = readdirSync(dir, { withFileTypes: true });
    } catch (err) {
      const code = err && err.code;
      if (code === "ENOENT" || code === "ENOTDIR") {
        disappearedCount += 1;
        return;
      }
      skippedCount += 1;
      return;
    }

    for (const entry of entries) {
      const full = join(dir, entry.name);
      let st;
      try {
        st = lstatSync(full);
      } catch (err) {
        const code = err && err.code;
        if (code === "ENOENT") {
          disappearedCount += 1;
          continue;
        }
        skippedCount += 1;
        continue;
      }

      if (st.isSymbolicLink()) {
        // Do not follow; count link metadata size only so measurement cannot escape the tree.
        const bytes = Number(st.size) || 0;
        totalBytes += bytes;
        fileCount += 1;
        considerLargest(full, bytes);
        continue;
      }

      if (st.isDirectory()) {
        dirCount += 1;
        walk(full);
        continue;
      }

      if (st.isFile()) {
        const bytes = Number(st.size) || 0;
        totalBytes += bytes;
        fileCount += 1;
        considerLargest(full, bytes);
        continue;
      }

      skippedCount += 1;
    }
  }

  walk(root);

  return {
    totalBytes,
    fileCount,
    dirCount,
    skippedCount,
    disappearedCount,
    largestFiles: largest
  };
}

function sleep(ms) {
  return new Promise((resolvePromise) => {
    setTimeout(resolvePromise, ms);
  });
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    printHelp();
    process.exit(0);
  }

  if (!args.target) fail("--target is required");
  if (!args.output) fail("--output is required");

  validatePositiveInt("--interval-ms", args.intervalMs, MIN_INTERVAL_MS, MAX_INTERVAL_MS);
  validatePositiveInt("--largest", args.largest, 1, MAX_LARGEST);

  if (args.durationMs === null && !args.stopFile) {
    fail("either --duration-ms or --stop-file is required (bounded run)");
  }
  if (args.durationMs !== null) {
    validatePositiveInt("--duration-ms", args.durationMs, 1, MAX_DURATION_MS);
  }

  const target = absPath(args.target);
  const output = absPath(args.output);
  const stopFile = args.stopFile ? absPath(args.stopFile) : null;

  if (!existsSync(target)) fail(`target does not exist: ${target}`);
  let targetStat;
  try {
    targetStat = lstatSync(target);
  } catch (err) {
    fail(`cannot stat target: ${err.message}`);
  }
  if (!targetStat.isDirectory()) fail(`target is not a directory: ${target}`);

  if (isPathInsideOrEqual(target, output)) {
    fail(`--output must not be inside --target (would inflate measurements): ${output}`);
  }
  if (stopFile && isPathInsideOrEqual(target, stopFile)) {
    fail(`--stop-file must not be inside --target: ${stopFile}`);
  }

  mkdirSync(dirname(output), { recursive: true });
  writeFileSync(output, "", "utf8");

  const summaryPath = `${output}.summary.json`;
  const startedAt = Date.now();
  const deadline = args.durationMs !== null ? startedAt + args.durationMs : null;

  // Peak tracking only — do not retain all samples in memory (8h-safe).
  let peakTotalBytes = 0;
  let peakAt = null;
  let peakFileCount = 0;
  let peakLargestFiles = [];
  let sampleCount = 0;
  let lastSample = null;
  let finalized = false;
  // Blocks re-entrant finalization (e.g. second SIGINT during final sample).
  let closing = false;

  function writeSummary(reason) {
    if (finalized) return;
    finalized = true;
    const endedAt = Date.now();
    const finalTotal = lastSample ? lastSample.totalBytes : 0;
    const summary = {
      schemaVersion: 1,
      target,
      output,
      startedAt: new Date(startedAt).toISOString(),
      endedAt: new Date(endedAt).toISOString(),
      durationMs: endedAt - startedAt,
      intervalMs: args.intervalMs,
      sampleCount,
      stopReason: reason,
      peak: {
        totalBytes: peakTotalBytes,
        at: peakAt,
        fileCount: peakFileCount,
        largestFiles: peakLargestFiles
      },
      final: lastSample
        ? {
            totalBytes: lastSample.totalBytes,
            at: lastSample.timestamp,
            fileCount: lastSample.fileCount,
            largestFiles: lastSample.largestFiles
          }
        : {
            totalBytes: 0,
            at: null,
            fileCount: 0,
            largestFiles: []
          },
      peakMinusFinalTemporaryOverheadBytes: Math.max(0, peakTotalBytes - finalTotal)
    };
    writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
    process.stdout.write(
      `${JSON.stringify({
        status: "done",
        summaryPath,
        peakTotalBytes,
        finalTotalBytes: finalTotal,
        peakMinusFinalTemporaryOverheadBytes: summary.peakMinusFinalTemporaryOverheadBytes,
        sampleCount,
        stopReason: reason
      })}\n`
    );
  }

  function takeSample() {
    const measured = measureTree(target, args.largest);
    const sample = {
      timestamp: new Date().toISOString(),
      epochMs: Date.now(),
      totalBytes: measured.totalBytes,
      fileCount: measured.fileCount,
      dirCount: measured.dirCount,
      skippedCount: measured.skippedCount,
      disappearedCount: measured.disappearedCount,
      largestFiles: measured.largestFiles
    };
    appendFileSync(output, `${JSON.stringify(sample)}\n`, "utf8");
    sampleCount += 1;
    lastSample = sample;
    if (peakAt === null || sample.totalBytes > peakTotalBytes) {
      peakTotalBytes = sample.totalBytes;
      peakAt = sample.timestamp;
      peakFileCount = sample.fileCount;
      peakLargestFiles = sample.largestFiles;
    }
    return sample;
  }

  /**
   * Clean stop paths take exactly one fresh sample at stop time so
   * summary.final matches the tree at termination (not the prior interval).
   * On sample failure, keep the best previous sample and still write summary.
   */
  function finalize(reason) {
    if (finalized || closing) return;
    closing = true;
    try {
      takeSample();
    } catch (err) {
      process.stderr.write(
        `error taking final sample: ${err && err.message ? err.message : String(err)}\n`
      );
    }
    writeSummary(reason);
  }

  const onSignal = (signal) => {
    try {
      finalize(`signal:${signal}`);
    } catch (err) {
      process.stderr.write(`error writing summary on ${signal}: ${err.message}\n`);
    }
    process.exit(130);
  };
  process.on("SIGINT", () => onSignal("SIGINT"));
  process.on("SIGTERM", () => onSignal("SIGTERM"));

  try {
    takeSample();

    while (true) {
      if (deadline !== null && Date.now() >= deadline) {
        finalize("duration");
        break;
      }
      if (stopFile && existsSync(stopFile)) {
        finalize("stop-file");
        break;
      }

      if (deadline !== null) {
        const remaining = deadline - Date.now();
        if (remaining <= 0) {
          finalize("duration");
          break;
        }
        await sleep(Math.min(args.intervalMs, remaining));
      } else {
        await sleep(args.intervalMs);
      }

      if (deadline !== null && Date.now() >= deadline) {
        finalize("duration");
        break;
      }
      if (stopFile && existsSync(stopFile)) {
        finalize("stop-file");
        break;
      }

      takeSample();
    }
  } catch (err) {
    try {
      writeSummary(`error:${err && err.message ? err.message : String(err)}`);
    } catch {
      // ignore secondary errors
    }
    process.stderr.write(`error: ${err && err.stack ? err.stack : String(err)}\n`);
    process.exit(1);
  }
}

main();
