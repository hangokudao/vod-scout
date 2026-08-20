import assert from "node:assert/strict";
import test from "node:test";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  assertCandidateSignals,
  CDP_TARGET_POLL_INTERVAL_MS,
  CDP_TARGET_TIMEOUT_MS,
  formatPlayerReadinessFailure,
  readPersistedSnapshot,
  waitForCancelled,
  waitForCdpTarget,
  waitForRecognitionCompletion,
  waitForRecognitionDomCompletion,
  writeFailureEvidence
} from "./e2e-local-cdp.mjs";

const runner = await readFile(new URL("./e2e-local-cdp.mjs", import.meta.url), "utf8");

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

test("CDP target polling waits through a startup race until VOD Scout is available", async () => {
  const responses = [
    [],
    [{ type: "page", title: "Other" }],
    [{ type: "page", title: "VOD Scout", webSocketDebuggerUrl: "ws://127.0.0.1/devtools" }]
  ];
  const result = await waitForCdpTarget(async () => responses.shift(), { timeoutMs: 100, intervalMs: 1 });
  assert.equal(result.target.title, "VOD Scout");
});

test("CDP target polling is bounded when the target never appears", async () => {
  await assert.rejects(
    () => waitForCdpTarget(async () => [{ type: "page", title: "Other", url: "app://other" }], { timeoutMs: 5, intervalMs: 1 }),
    (error) => /VOD Scout 창을 5ms 안에 찾지 못했습니다/.test(error.message)
      && error.lastTargets?.[0]?.title === "Other"
  );
});

test("CDP timeout preserves the last target title/url and read error on the main error", async () => {
  let reads = 0;
  await assert.rejects(
    () => waitForCdpTarget(async () => {
      reads += 1;
      if (reads === 1) return [{ type: "page", title: "Other", url: "app://other" }];
      throw new Error("CDP socket read failed");
    }, { timeoutMs: 20, intervalMs: 1 }),
    (error) => error.lastTargets?.[0]?.title === "Other"
      && error.lastTargets?.[0]?.url === "app://other"
      && error.lastReadError === "CDP socket read failed"
      && error.message.includes("Other")
      && error.message.includes("app://other")
      && error.message.includes("CDP socket read failed")
  );
});

test("CDP target polling keeps the exact production bounds", () => {
  assert.equal(CDP_TARGET_TIMEOUT_MS, 10_000);
  assert.equal(CDP_TARGET_POLL_INTERVAL_MS, 250);
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

test("recognition polling requires a new completed revision with backend evidence", async () => {
  const snapshots = [
    { recognitionRuns: [{ id: "old", candidateId: "candidate-1", status: "COMPLETED", resultRevision: 1 }] },
    { recognitionRuns: [{
      id: "new",
      candidateId: "candidate-1",
      status: "COMPLETED",
      resultRevision: 2,
      backendEvidence: "실제 백엔드=whisper.cpp",
      failureReason: null
    }] }
  ];
  const result = await waitForRecognitionCompletion(async () => snapshots.shift(), {
    candidateId: "candidate-1",
    previousRunIds: ["old"],
    previousRevision: 1,
    timeoutMs: 100,
    intervalMs: 1
  });
  assert.equal(result.run.id, "new");
  assert.equal(result.run.resultRevision, 2);
});

test("recognition polling surfaces a failed run and persisted snapshot", async () => {
  await assert.rejects(
    () => waitForRecognitionCompletion(async () => ({ recognitionRuns: [{
      id: "failed",
      candidateId: "candidate-1",
      status: "FAILED",
      resultRevision: 1,
      failureReason: "GPU unavailable"
    }] }), { candidateId: "candidate-1", timeoutMs: 20, intervalMs: 1 }),
    (error) => error.lastSnapshot?.recognitionRuns?.[0]?.status === "FAILED" && /GPU unavailable/.test(error.message)
  );
});

test("recognition polling is bounded and retains the last persisted snapshot", async () => {
  await assert.rejects(
    () => waitForRecognitionCompletion(async () => ({ recognitionRuns: [{
      id: "started",
      candidateId: "candidate-1",
      status: "STARTED",
      resultRevision: 1
    }] }), { candidateId: "candidate-1", timeoutMs: 5, intervalMs: 1 }),
    (error) => error.lastSnapshot?.recognitionRuns?.[0]?.status === "STARTED" && /안에 완료되지 않았습니다/.test(error.message)
  );
});

test("recognition DOM polling requires the completion status", async () => {
  const bodies = ["다시 음성 인식: 진행 중", "다시 음성 인식: 완료 · 실행 ID run-1 · 개정 2"];
  const result = await waitForRecognitionDomCompletion(async () => bodies.shift(), { timeoutMs: 100, intervalMs: 1 });
  assert.match(result.body, /다시 음성 인식: 완료/);
});

test("recognition DOM polling rejects a visible failure", async () => {
  await assert.rejects(
    () => waitForRecognitionDomCompletion(async () => "다시 음성 인식: 실패", { timeoutMs: 20, intervalMs: 1 }),
    /음성 인식 실패가 표시되었습니다/
  );
});

test("review-existing bootstraps REVIEW_READY without starting a job", () => {
  assert.match(runner, /options\.resumeExisting \|\| options\.reviewExisting/);
  const reviewStart = runner.indexOf("if (options.reviewExisting)");
  const reviewBootstrap = runner.slice(reviewStart, runner.indexOf("} else {", reviewStart));
  assert.match(reviewBootstrap, /review_existing_ready/);
  assert.doesNotMatch(reviewBootstrap, /start_job/);
  assert.match(runner, /if \(options\.reviewExisting\) \{[\s\S]*?else \{[\s\S]*?start_job/);
});

test("normal REVIEW_READY flows also run the actual candidate re-recognition UI path", () => {
  const recognitionBlock = runner.slice(runner.indexOf("let recognitionEvidence"), runner.indexOf("if (options.screenshotPath)"));
  assert.match(recognitionBlock, /if \(snapshot\.status === "REVIEW_READY"\) \{[\s\S]*clickFirstCandidateRow/);
  assert.match(recognitionBlock, /clickVisibleEnabledButton\(evaluate, "다시 음성 인식"\)/);
  assert.match(recognitionBlock, /waitForRecognitionCompletion\(readSnapshot/);
  assert.match(recognitionBlock, /waitForRecognitionDomCompletion/);
  assert.doesNotMatch(recognitionBlock, /options\.reviewExisting/);
  assert.match(runner, /Page\.captureScreenshot/);
});

test("snapshot polling reads the persisted snapshot without invoking bootstrap recovery", async () => {
  const directory = await mkdtemp(join(tmpdir(), "vod-scout-e2e-snapshot-"));
  const jobs = join(directory, "jobs", "job-1");
  await mkdir(jobs, { recursive: true });
  await writeFile(join(jobs, "snapshot.json"), JSON.stringify({ status: "CANCELLED", id: "job-1" }), "utf8");
  assert.deepEqual(await readPersistedSnapshot(jobs), { status: "CANCELLED", id: "job-1" });
});

test("player readiness diagnostics include actionable media state", () => {
  assert.equal(
    formatPlayerReadinessFailure({ hasVideo: true, readyState: 0, networkState: 3, errorCode: 4 }),
    "검토 화면의 원본 구간 플레이어가 준비되지 않았습니다. video=있음, readyState=0, networkState=3, errorCode=4"
  );
  assert.match(formatPlayerReadinessFailure(), /video=없음, readyState=0, networkState=0, errorCode=없음/);
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
