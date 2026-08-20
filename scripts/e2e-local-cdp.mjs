import { access, mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { setTimeout as delay } from "node:timers/promises";
import { pathToFileURL } from "node:url";

export const CANCEL_TIMEOUT_MS = 10_000;
export const CANCEL_POLL_INTERVAL_MS = 200;

export function formatPlayerReadinessFailure({
  hasVideo = false,
  readyState = 0,
  networkState = 0,
  errorCode = null,
  errorMessage = null
} = {}) {
  return `검토 화면의 원본 구간 플레이어가 준비되지 않았습니다. video=${hasVideo ? "있음" : "없음"}, readyState=${readyState}, networkState=${networkState}, errorCode=${errorCode ?? "없음"}${errorMessage ? `, error=${errorMessage}` : ""}`;
}

function argumentValue(args, flag) {
  const index = args.indexOf(flag);
  return index >= 0 ? args[index + 1] : null;
}

export function assertCandidateSignals(snapshot, { requireChatScore = false } = {}) {
  if (!snapshot?.candidates?.length) throw new Error("실제 후보가 생성되지 않았습니다.");
  if (requireChatScore && !snapshot.candidates.some((candidate) => typeof candidate.chatScore === "number" && Number.isFinite(candidate.chatScore))) {
    throw new Error("채팅 전용 검사를 통과할 숫자인 chatScore가 없습니다.");
  }
}

export async function waitForCancelled(readSnapshot, {
  timeoutMs = CANCEL_TIMEOUT_MS,
  intervalMs = CANCEL_POLL_INTERVAL_MS
} = {}) {
  const startedAt = Date.now();
  const deadline = startedAt + timeoutMs;
  let lastSnapshot = null;
  while (Date.now() <= deadline) {
    lastSnapshot = await readSnapshot();
    if (lastSnapshot?.status === "CANCELLED") {
      return { snapshot: lastSnapshot, elapsedMs: Date.now() - startedAt };
    }
    const remainingMs = deadline - Date.now();
    if (remainingMs <= 0) break;
    await delay(Math.min(intervalMs, remainingMs));
  }
  const error = new Error(`10초 안에 CANCELLED 상태가 되지 않았습니다. 마지막 상태: ${lastSnapshot?.status ?? "없음"}`);
  error.lastSnapshot = lastSnapshot;
  throw error;
}

export async function readPersistedSnapshot(jobDirectory, { retries = 5, retryDelayMs = 50 } = {}) {
  let lastError;
  for (let attempt = 0; attempt <= retries; attempt += 1) {
    try {
      return JSON.parse(await readFile(join(jobDirectory, "snapshot.json"), "utf8"));
    } catch (error) {
      lastError = error;
      if (attempt < retries) await delay(retryDelayMs);
    }
  }
  throw lastError;
}

export async function writeFailureEvidence({ directory, args, error, lastSnapshot, logEntries }) {
  await mkdir(directory, { recursive: true });
  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  const base = join(directory, `e2e-failure-${stamp}-${process.pid}`);
  const jsonPath = `${base}.json`;
  const logPath = `${base}.log`;
  const payload = {
    args,
    error: {
      name: error?.name ?? "Error",
      message: error?.message ?? String(error),
      stack: error?.stack ?? null
    },
    lastSnapshot: lastSnapshot ?? null,
    log: logEntries
  };
  await writeFile(jsonPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
  await writeFile(logPath, `${logEntries.map((entry) => JSON.stringify(entry)).join("\n")}\n`, "utf8");
  return { jsonPath, logPath };
}

function parseOptions(args) {
  const source = args[0];
  const port = Number(args[1] ?? 9225);
  const mode = argumentValue(args, "--mode") ?? "full";
  if (!source) throw new Error("사용법: node scripts/e2e-local-cdp.mjs <영상 경로 또는 YouTube URL> [port] [--youtube]");
  if (!Number.isInteger(port) || port <= 0) throw new Error("CDP 포트가 올바르지 않습니다.");
  if (!["quick", "range", "full"].includes(mode)) throw new Error("--mode는 quick, range, full 중 하나여야 합니다.");
  return {
    args,
    source,
    port,
    analysisMode: mode,
    verifyCancelResume: args.includes("--cancel-resume"),
    startOnly: args.includes("--start-only"),
    youtube: args.includes("--youtube"),
    requireChatScore: args.includes("--require-chat-score"),
    expectDownloadFailure: args.includes("--expect-download-failure"),
    verifyDelete: args.includes("--verify-delete"),
    longRun: args.includes("--long"),
    resumeExisting: args.includes("--resume-existing"),
    screenshotPath: argumentValue(args, "--screenshot"),
    evidenceDirectory: argumentValue(args, "--evidence-dir") ?? process.env.VOD_SCOUT_E2E_EVIDENCE_DIR ?? join(tmpdir(), "vod-scout-e2e-evidence")
  };
}

export async function runE2E(args = process.argv.slice(2)) {
  const options = parseOptions(args);
  const sourceKind = options.youtube ? "youtube" : "local";
  const logEntries = [];
  let lastSnapshot = null;
  let socket = null;
  const log = (event, details = {}) => {
    logEntries.push({ at: new Date().toISOString(), event, ...details });
  };

  try {
    log("start", { args });
    const response = await fetch(`http://127.0.0.1:${options.port}/json`);
    if (!response.ok) throw new Error(`CDP 대상 조회 실패: HTTP ${response.status}`);
    const targets = await response.json();
    const target = targets.find((item) => item.type === "page" && item.title === "VOD Scout");
    if (!target) throw new Error("CDP에서 VOD Scout 창을 찾지 못했습니다.");

    socket = new WebSocket(target.webSocketDebuggerUrl);
    const pending = new Map();
    socket.addEventListener("message", (event) => {
      let message;
      try {
        message = JSON.parse(event.data);
      } catch (error) {
        for (const waiter of pending.values()) waiter.reject(error);
        pending.clear();
        return;
      }
      const waiter = pending.get(message.id);
      if (!waiter) return;
      pending.delete(message.id);
      if (message.error) waiter.reject(new Error(message.error.message));
      else waiter.resolve(message.result);
    });
    const socketError = (event) => {
      const error = new Error(`CDP WebSocket 오류${event?.message ? `: ${event.message}` : ""}`);
      for (const waiter of pending.values()) waiter.reject(error);
      pending.clear();
    };
    socket.addEventListener("error", socketError);
    socket.addEventListener("close", () => {
      const error = new Error("CDP WebSocket 연결이 닫혔습니다.");
      for (const waiter of pending.values()) waiter.reject(error);
      pending.clear();
    });
    await new Promise((resolve, reject) => {
      socket.addEventListener("open", resolve, { once: true });
      socket.addEventListener("error", reject, { once: true });
    });

    let nextId = 1;
    const cdp = (method, params = {}) => {
      const id = nextId++;
      log("cdp_request", { method });
      socket.send(JSON.stringify({ id, method, params }));
      return new Promise((resolve, reject) => pending.set(id, { resolve, reject }));
    };
    async function evaluate(expression) {
      const response = await cdp("Runtime.evaluate", {
        expression,
        awaitPromise: true,
        returnByValue: true
      });
      if (response.exceptionDetails) {
        throw new Error(response.exceptionDetails.exception?.description ?? "WebView 평가 실패");
      }
      return response.result.value;
    }
    const runtime = await evaluate("window.__TAURI_INTERNALS__.invoke('get_runtime_info')");
    let jobDirectory = null;
    async function readSnapshot(reason) {
      const snapshot = await readPersistedSnapshot(jobDirectory);
      lastSnapshot = snapshot;
      log("snapshot", { reason, status: snapshot?.status ?? null, jobId: snapshot?.id ?? null });
      return snapshot;
    }

    await cdp("Runtime.enable");
    const input = JSON.stringify({ sourceKind, sourceLabel: options.source, scenario: "normal", analysisMode: options.analysisMode });
    const created = options.resumeExisting
      ? await evaluate("window.__TAURI_INTERNALS__.invoke('bootstrap')")
      : await evaluate(`window.__TAURI_INTERNALS__.invoke("create_job", { input: ${input} })`);
    if (!created) throw new Error("재개할 기존 작업을 찾지 못했습니다.");
    lastSnapshot = created;
    jobDirectory = join(runtime.dataDirectory, "jobs", created.id);
    log("job_created", { jobId: created.id, sourceKind });
    await evaluate(`window.__TAURI_INTERNALS__.invoke("start_job", { jobId: ${JSON.stringify(created.id)} })`);
    log("job_started", { jobId: created.id });

    if (options.startOnly) {
      await delay(options.youtube ? 100 : 750);
      return { jobId: created.id, started: true };
    }

    let cancelVerified = false;
    if (options.verifyCancelResume) {
      await delay(750);
      await evaluate(`window.__TAURI_INTERNALS__.invoke("cancel_job", { jobId: ${JSON.stringify(created.id)} })`);
      log("cancel_requested", { jobId: created.id });
      const cancelled = await waitForCancelled(() => readSnapshot("cancel_poll"));
      cancelVerified = true;
      log("cancel_confirmed", { jobId: created.id, elapsedMs: cancelled.elapsedMs });
      await evaluate(`window.__TAURI_INTERNALS__.invoke("start_job", { jobId: ${JSON.stringify(created.id)} })`);
      log("job_resumed", { jobId: created.id });
    }

    const timeoutMs = options.longRun ? 1_200_000 : options.youtube ? 180_000 : 90_000;
    const deadline = Date.now() + timeoutMs;
    let body = "";
    let etaSeen = false;
    while (Date.now() < deadline) {
      await delay(500);
      body = await evaluate("document.body.innerText");
      const progress = await readSnapshot("progress");
      if (body.includes("예상 완료") && body.includes("남음")) etaSeen = true;
      const failedMessage = body.includes("영상 분석을 완료하지 못했습니다") || body.includes("YouTube 영상을 다운로드하지 못했습니다");
      if (failedMessage || progress?.status === "FAILED") {
        if (options.expectDownloadFailure && progress?.status === "FAILED" && progress.errorMessage?.includes("YouTube")) {
          return {
            status: progress.status,
            errorMessage: progress.errorMessage,
            errorDetail: progress.errorDetail
          };
        }
        throw new Error(`${progress?.errorMessage ?? "분석 실패"}: ${progress?.errorDetail ?? ""}`);
      }
      if (body.includes("편집 후보를 검토하세요") || progress?.status === "REVIEW_READY") break;
    }
    if (options.expectDownloadFailure) throw new Error("실패해야 하는 YouTube 다운로드가 실패 상태가 되지 않았습니다.");
    if (!body.includes("편집 후보를 검토하세요") && lastSnapshot?.status !== "REVIEW_READY") throw new Error(`${timeoutMs / 1000}초 안에 검토 화면이 열리지 않았습니다.`);
    if (options.longRun && !etaSeen) throw new Error("장시간 음성 인식 중 예상 남은 시간이 표시되지 않았습니다.");

    const snapshot = await readSnapshot("review_ready");
    if (snapshot.status !== "REVIEW_READY") throw new Error(`예상하지 못한 상태: ${snapshot.status}`);
    if (snapshot.sourceKind !== sourceKind) throw new Error(`예상하지 못한 입력 종류: ${snapshot.sourceKind}`);
    assertCandidateSignals(snapshot, { requireChatScore: options.requireChatScore });
    for (let index = 0; index < snapshot.candidates.length; index += 1) {
      const left = snapshot.candidates[index];
      if (/1\/2 of the cream cheese/i.test(left.transcriptExcerpt)) throw new Error("영어 반복 환각이 후보에 남았습니다.");
      for (const right of snapshot.candidates.slice(index + 1)) {
        if (left.startSeconds < right.endSeconds && right.startSeconds < left.endSeconds) {
          throw new Error("겹치는 후보가 검토 목록에 남았습니다.");
        }
      }
    }
    if (options.longRun && !options.youtube && !snapshot.candidates.some((candidate) => /[가-힣]/.test(candidate.transcriptExcerpt) && !candidate.transcriptExcerpt.includes("인식된 발화가 없습니다"))) {
      throw new Error("한국어 음성 인식 문장이 후보에 반영되지 않았습니다.");
    }
    if (options.verifyCancelResume && !snapshot.activity.some((event) => event.kind === "cancel")) {
      throw new Error("취소 후 재개 활동 기록이 보존되지 않았습니다.");
    }

    jobDirectory = join(runtime.dataDirectory, "jobs", snapshot.id);
    const checkpoint = JSON.parse(await readFile(join(jobDirectory, "media-checkpoint.json"), "utf8"));
    const provenance = JSON.parse(await readFile(join(jobDirectory, "pipeline-provenance.json"), "utf8"));
    const transcript = JSON.parse(await readFile(join(jobDirectory, "transcript.json"), "utf8"));
    const chatMotion = JSON.parse(await readFile(join(jobDirectory, "chat-motion.json"), "utf8"));
    const acquisition = options.youtube ? JSON.parse(await readFile(join(jobDirectory, "acquisition.json"), "utf8")) : null;
    const expectedChunks = checkpoint.plannedChunks.length;
    if (checkpoint.completedChunks !== expectedChunks) throw new Error(`청크 체크포인트가 완성되지 않았습니다: ${checkpoint.completedChunks}/${expectedChunks}`);
    if (checkpoint.analysisMode !== options.analysisMode || provenance.analysis.mode !== options.analysisMode) throw new Error("분석 모드 provenance가 일치하지 않습니다.");
    if (provenance.inputFingerprint?.value?.length !== 64 || provenance.inputFingerprint?.bytes < 1) throw new Error("입력 fingerprint provenance가 올바르지 않습니다.");
    if (options.longRun && !transcript.length) throw new Error("장시간 음성 인식 산출물이 저장되지 않았습니다.");
    if (transcript.some((segment) => /1\/2 of the cream cheese/i.test(segment.text))) throw new Error("영어 반복 환각이 음성 인식 산출물에 남았습니다.");
    if (!checkpoint.chatMotionCompleted || !chatMotion.length) throw new Error("채팅 움직임 산출물이 저장되지 않았습니다.");
    if (options.youtube && (!acquisition?.mediaPath || snapshot.downloadPercent !== 100)) {
      throw new Error("YouTube 다운로드 체크포인트가 완성되지 않았습니다.");
    }

    const playerDeadline = Date.now() + 60_000;
    let playerReady = false;
    let playerState = null;
    while (Date.now() < playerDeadline) {
      await delay(250);
      playerState = await evaluate(`(() => {
        const video = document.querySelector('video');
        return {
          hasVideo: Boolean(video),
          readyState: video?.readyState ?? 0,
          networkState: video?.networkState ?? 0,
          errorCode: video?.error?.code ?? null,
          errorMessage: video?.error?.message ?? null
        };
      })()`);
      playerReady = Boolean(playerState?.hasVideo && playerState.readyState >= 1);
      if (playerReady) break;
    }
    if (!playerReady) throw new Error(formatPlayerReadinessFailure(playerState));

    const preview = await evaluate(`window.__TAURI_INTERNALS__.invoke("prepare_candidate_preview", { jobId: ${JSON.stringify(snapshot.id)}, candidateId: ${JSON.stringify(snapshot.candidates[0].id)} })`);
    if ((await stat(preview.path)).size < 1024) throw new Error("후보 영상 미리보기가 생성되지 않았습니다.");
    const storage = await evaluate(`window.__TAURI_INTERNALS__.invoke("get_job_storage_info", { jobId: ${JSON.stringify(snapshot.id)} })`);
    if (storage.sizeBytes < 1024) throw new Error("작업 저장 용량을 계산하지 못했습니다.");
    if (options.screenshotPath) {
      await mkdir(dirname(options.screenshotPath), { recursive: true });
      const screenshot = await cdp("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
      await writeFile(options.screenshotPath, Buffer.from(screenshot.data, "base64"));
    }

    let deleteVerified = false;
    if (options.verifyDelete) {
      await evaluate(`window.__TAURI_INTERNALS__.invoke("delete_job", { jobId: ${JSON.stringify(snapshot.id)} })`);
      const restored = await evaluate("window.__TAURI_INTERNALS__.invoke('bootstrap')");
      if (restored !== null) throw new Error("삭제한 작업이 다시 복원됐습니다.");
      try {
        await access(jobDirectory);
      } catch {
        deleteVerified = true;
      }
      if (!deleteVerified) throw new Error("작업 폴더가 삭제되지 않았습니다.");
    }

    return {
      status: snapshot.status,
      candidateCount: snapshot.candidates.length,
      firstCandidate: snapshot.candidates[0],
      completedUnits: snapshot.completedUnits,
      totalUnits: snapshot.totalUnits,
      checkpointChunks: checkpoint.completedChunks,
      transcriptSegments: transcript.length,
      chatMotionPoints: chatMotion.length,
      previewPath: preview.path,
      previewPlayerReady: playerReady,
      storageBytes: storage.sizeBytes,
      csvPath: null,
      csvBoundary: "native backend save dialog; formula escaping covered by Rust unit test",
      screenshotPath: options.screenshotPath,
      acquisitionPath: acquisition?.mediaPath ?? null,
      bodyVerified: body.includes("채팅 움직임"),
      etaSeen,
      cancelVerified,
      deleteVerified
    };
  } catch (error) {
    log("failure", { message: error?.message ?? String(error), status: lastSnapshot?.status ?? null });
    const evidence = await writeFailureEvidence({
      directory: options.evidenceDirectory,
      args: options.args,
      error,
      lastSnapshot: error.lastSnapshot ?? lastSnapshot,
      logEntries
    });
    error.evidence = evidence;
    throw error;
  } finally {
    if (socket) socket.close();
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const result = await runE2E();
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  } catch (error) {
    process.stderr.write(`${error.stack ?? error}\n`);
    if (error.evidence) process.stderr.write(`실패 증거: ${error.evidence.jsonPath}\n전체 로그: ${error.evidence.logPath}\n`);
    process.exitCode = 1;
  }
}
