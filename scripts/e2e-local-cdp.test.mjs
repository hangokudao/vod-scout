import assert from "node:assert/strict";
import test from "node:test";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  assertCandidateSignals,
  readPersistedSnapshot,
  waitForCancelled,
  writeFailureEvidence
} from "./e2e-local-cdp.mjs";

test("voice-centric candidate checks allow a missing chat score", () => {
  assert.doesNotThrow(() => assertCandidateSignals({ candidates: [{ chatScore: null }] }));
});

test("chat-only checks require at least one numeric chat score", () => {
  assert.throws(
    () => assertCandidateSignals({ candidates: [{ chatScore: null }, { chatScore: "88" }] }, { requireChatScore: true }),
    /숫자인 chatScore/
  );
  assert.doesNotThrow(() => assertCandidateSignals({ candidates: [{ chatScore: null }, { chatScore: 88 }] }, { requireChatScore: true }));
});

test("cancel polling uses the persisted snapshot state", async () => {
  const snapshots = [{ status: "CANCELLING" }, { status: "CANCELLED" }];
  const result = await waitForCancelled(async () => snapshots.shift(), { timeoutMs: 100, intervalMs: 1 });
  assert.equal(result.snapshot.status, "CANCELLED");
});

test("cancel polling exposes the last snapshot when the deadline expires", async () => {
  await assert.rejects(
    () => waitForCancelled(async () => ({ status: "CANCELLING" }), { timeoutMs: 5, intervalMs: 1 }),
    (error) => error.lastSnapshot?.status === "CANCELLING" && /CANCELLED/.test(error.message)
  );
});

test("snapshot polling reads the persisted snapshot without invoking bootstrap recovery", async () => {
  const directory = await mkdtemp(join(tmpdir(), "vod-scout-e2e-snapshot-"));
  const jobs = join(directory, "jobs", "job-1");
  await mkdir(jobs, { recursive: true });
  await writeFile(join(jobs, "snapshot.json"), JSON.stringify({ status: "CANCELLED", id: "job-1" }), "utf8");
  assert.deepEqual(await readPersistedSnapshot(jobs), { status: "CANCELLED", id: "job-1" });
});

test("CDP failure evidence includes arguments, exception, snapshot, and full log", async () => {
  const directory = await mkdtemp(join(tmpdir(), "vod-scout-e2e-evidence-"));
  const evidence = await writeFailureEvidence({
    directory,
    args: ["video.mp4", "9225", "--mode", "full"],
    error: new Error("CDP 평가 실패"),
    lastSnapshot: { status: "CANCELLING", id: "job-1" },
    logEntries: [{ event: "cancel_requested" }, { event: "snapshot", status: "CANCELLING" }]
  });
  const payload = JSON.parse(await readFile(evidence.jsonPath, "utf8"));
  const log = await readFile(evidence.logPath, "utf8");
  assert.deepEqual(payload.args, ["video.mp4", "9225", "--mode", "full"]);
  assert.equal(payload.error.message, "CDP 평가 실패");
  assert.deepEqual(payload.lastSnapshot, { status: "CANCELLING", id: "job-1" });
  assert.match(log, /cancel_requested/);
  assert.match(log, /snapshot/);
});
