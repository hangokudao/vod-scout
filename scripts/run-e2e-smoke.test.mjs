import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

const script = await readFile(new URL("./run-e2e-smoke.ps1", import.meta.url), "utf8");

test("E2E data directory is accepted only through the com.vodscout.app e2e-* contract", () => {
  assert.match(script, /function Resolve-E2eDataDirectory/);
  assert.match(script, /Join-Path \$env:LOCALAPPDATA "com\.vodscout\.app"/);
  assert.match(script, /if \(-not \$safeLeaf\.StartsWith\("e2e-"/);
  assert.match(script, /\$env:VOD_SCOUT_E2E_DATA_DIR = \$resolvedDataDirectory/);
  assert.match(script, /IsNullOrWhiteSpace\(\$RequestedDataDirectory\)/);
  assert.match(script, /\$requestedLeaf -in @\("\."\, "\.\."\)/);
});

test("E2E runner no longer passes the caller's arbitrary parent path to the app", () => {
  assert.doesNotMatch(script, /\$env:VOD_SCOUT_E2E_DATA_DIR = \$DataDirectory/);
});

test("ReviewExisting keeps the current REVIEW_READY job without restarting analysis", () => {
  assert.match(script, /\[switch\]\$ReviewExisting/);
  assert.match(script, /if \(\$ReviewExisting\) \{\s*\$arguments \+= "--review-existing"/s);
  assert.match(script, /\[switch\]\$ReviewExisting/);
});
