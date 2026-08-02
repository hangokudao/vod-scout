import { access, readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";

const source = process.argv[2];
const port = Number(process.argv[3] ?? 9225);
const verifyCancelResume = process.argv.includes("--cancel-resume");
const startOnly = process.argv.includes("--start-only");
const youtube = process.argv.includes("--youtube");
const expectDownloadFailure = process.argv.includes("--expect-download-failure");
const verifyDelete = process.argv.includes("--verify-delete");
const longRun = process.argv.includes("--long");
const resumeExisting = process.argv.includes("--resume-existing");
const screenshotFlagIndex = process.argv.indexOf("--screenshot");
const screenshotPath = screenshotFlagIndex >= 0 ? process.argv[screenshotFlagIndex + 1] : null;
const modeFlagIndex = process.argv.indexOf("--mode");
const analysisMode = modeFlagIndex >= 0 ? process.argv[modeFlagIndex + 1] : "full";
if (!["quick", "range", "full"].includes(analysisMode)) throw new Error("--mode는 quick, range, full 중 하나여야 합니다.");
const sourceKind = youtube ? "youtube" : "local";
if (!source) throw new Error("사용법: node scripts/e2e-local-cdp.mjs <영상 경로 또는 YouTube URL> [port] [--youtube]");

const targets = await fetch(`http://127.0.0.1:${port}/json`).then((response) => response.json());
const target = targets.find((item) => item.type === "page" && item.title === "VOD Scout");
if (!target) throw new Error("CDP에서 VOD Scout 창을 찾지 못했습니다.");

const socket = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  socket.addEventListener("open", resolve, { once: true });
  socket.addEventListener("error", reject, { once: true });
});

let nextId = 1;
const pending = new Map();
socket.addEventListener("message", (event) => {
  const message = JSON.parse(event.data);
  const waiter = pending.get(message.id);
  if (!waiter) return;
  pending.delete(message.id);
  if (message.error) waiter.reject(new Error(message.error.message));
  else waiter.resolve(message.result);
});

function cdp(method, params = {}) {
  const id = nextId++;
  socket.send(JSON.stringify({ id, method, params }));
  return new Promise((resolve, reject) => pending.set(id, { resolve, reject }));
}

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

await cdp("Runtime.enable");
const input = JSON.stringify({ sourceKind, sourceLabel: source, scenario: "normal", analysisMode });
const created = resumeExisting
  ? await evaluate("window.__TAURI_INTERNALS__.invoke('bootstrap')")
  : await evaluate(`window.__TAURI_INTERNALS__.invoke("create_job", { input: ${input} })`);
if (!created) throw new Error("재개할 기존 작업을 찾지 못했습니다.");
await evaluate(`window.__TAURI_INTERNALS__.invoke("start_job", { jobId: ${JSON.stringify(created.id)} })`);

if (startOnly) {
  await new Promise((resolve) => setTimeout(resolve, youtube ? 100 : 750));
  socket.close();
  process.stdout.write(JSON.stringify({ jobId: created.id, started: true }));
  process.exit(0);
}

let cancelVerified = false;
if (verifyCancelResume) {
  await new Promise((resolve) => setTimeout(resolve, 750));
  await evaluate(`window.__TAURI_INTERNALS__.invoke("cancel_job", { jobId: ${JSON.stringify(created.id)} })`);
  const cancelDeadline = Date.now() + 10_000;
  while (Date.now() < cancelDeadline) {
    await new Promise((resolve) => setTimeout(resolve, 200));
    const cancelBody = await evaluate("document.body.innerText");
    if (cancelBody.includes("취소됨")) {
      cancelVerified = true;
      break;
    }
  }
  if (!cancelVerified) throw new Error("10초 안에 실행 중인 미디어 도구가 취소되지 않았습니다.");
  await evaluate(`window.__TAURI_INTERNALS__.invoke("start_job", { jobId: ${JSON.stringify(created.id)} })`);
}

const timeoutMs = longRun ? 1_200_000 : youtube ? 180_000 : 90_000;
const deadline = Date.now() + timeoutMs;
let body = "";
let etaSeen = false;
while (Date.now() < deadline) {
  await new Promise((resolve) => setTimeout(resolve, 500));
  body = await evaluate("document.body.innerText");
  if (body.includes("예상 완료") && body.includes("남음")) etaSeen = true;
  if (body.includes("편집 후보를 검토하세요")) break;
  if (body.includes("영상 분석을 완료하지 못했습니다") || body.includes("YouTube 영상을 다운로드하지 못했습니다")) {
    const failed = await evaluate("window.__TAURI_INTERNALS__.invoke('bootstrap')");
    if (expectDownloadFailure && failed.status === "FAILED" && failed.errorMessage?.includes("YouTube")) {
      socket.close();
      process.stdout.write(JSON.stringify({
        status: failed.status,
        errorMessage: failed.errorMessage,
        errorDetail: failed.errorDetail
      }, null, 2));
      process.exit(0);
    }
    throw new Error(`${failed.errorMessage}: ${failed.errorDetail}`);
  }
}
if (expectDownloadFailure) throw new Error("실패해야 하는 YouTube 다운로드가 실패 상태가 되지 않았습니다.");
if (!body.includes("편집 후보를 검토하세요")) throw new Error(`${timeoutMs / 1000}초 안에 검토 화면이 열리지 않았습니다.`);
if (longRun && !etaSeen) throw new Error("장시간 전사 중 예상 남은 시간이 표시되지 않았습니다.");

const snapshot = await evaluate("window.__TAURI_INTERNALS__.invoke('bootstrap')");
if (snapshot.status !== "REVIEW_READY") throw new Error(`예상하지 못한 상태: ${snapshot.status}`);
if (snapshot.sourceKind !== sourceKind) throw new Error(`예상하지 못한 입력 종류: ${snapshot.sourceKind}`);
if (!snapshot.candidates?.length) throw new Error("실제 후보가 생성되지 않았습니다.");
for (let index = 0; index < snapshot.candidates.length; index += 1) {
  const left = snapshot.candidates[index];
  if (/1\/2 of the cream cheese/i.test(left.transcriptExcerpt)) throw new Error("영어 반복 환각이 후보에 남았습니다.");
  for (const right of snapshot.candidates.slice(index + 1)) {
    if (left.startSeconds < right.endSeconds && right.startSeconds < left.endSeconds) {
      throw new Error("겹치는 후보가 검토 목록에 남았습니다.");
    }
  }
}
if (longRun && !youtube && !snapshot.candidates.some((candidate) => /[가-힣]/.test(candidate.transcriptExcerpt) && !candidate.transcriptExcerpt.includes("인식된 발화가 없습니다"))) {
  throw new Error("한국어 Whisper 전사 문장이 후보에 반영되지 않았습니다.");
}
if (snapshot.candidates.some((candidate) => candidate.chatScore === null)) {
  throw new Error("채팅 움직임 점수가 후보에 반영되지 않았습니다.");
}
if (verifyCancelResume && !snapshot.activity.some((event) => event.kind === "cancel")) {
  throw new Error("취소 후 재개 활동 기록이 보존되지 않았습니다.");
}

const runtime = await evaluate("window.__TAURI_INTERNALS__.invoke('get_runtime_info')");
const jobDirectory = join(runtime.dataDirectory, "jobs", snapshot.id);
const checkpoint = JSON.parse(await readFile(join(jobDirectory, "media-checkpoint.json"), "utf8"));
const provenance = JSON.parse(await readFile(join(jobDirectory, "pipeline-provenance.json"), "utf8"));
const transcript = JSON.parse(await readFile(join(jobDirectory, "transcript.json"), "utf8"));
const chatMotion = JSON.parse(await readFile(join(jobDirectory, "chat-motion.json"), "utf8"));
const acquisition = youtube ? JSON.parse(await readFile(join(jobDirectory, "acquisition.json"), "utf8")) : null;
const expectedChunks = checkpoint.plannedChunks.length;
if (checkpoint.completedChunks !== expectedChunks) throw new Error(`청크 체크포인트가 완성되지 않았습니다: ${checkpoint.completedChunks}/${expectedChunks}`);
if (checkpoint.analysisMode !== analysisMode || provenance.analysis.mode !== analysisMode) throw new Error("분석 모드 provenance가 일치하지 않습니다.");
if (provenance.inputFingerprint?.value?.length !== 64 || provenance.inputFingerprint?.bytes < 1) throw new Error("입력 fingerprint provenance가 올바르지 않습니다.");
if (longRun && !transcript.length) throw new Error("장시간 전사 산출물이 저장되지 않았습니다.");
if (transcript.some((segment) => /1\/2 of the cream cheese/i.test(segment.text))) throw new Error("영어 반복 환각이 전사 산출물에 남았습니다.");
if (!checkpoint.chatMotionCompleted || !chatMotion.length) throw new Error("채팅 움직임 산출물이 저장되지 않았습니다.");
if (youtube && (!acquisition?.mediaPath || snapshot.downloadPercent !== 100)) {
  throw new Error("YouTube 다운로드 체크포인트가 완성되지 않았습니다.");
}

const playerDeadline = Date.now() + 60_000;
let playerReady = false;
while (Date.now() < playerDeadline) {
  await new Promise((resolve) => setTimeout(resolve, 250));
  playerReady = await evaluate("Boolean(document.querySelector('video') && document.querySelector('video').readyState >= 1)");
  if (playerReady) break;
}
if (!playerReady) throw new Error("검토 화면의 원본 구간 플레이어가 준비되지 않았습니다.");

const preview = await evaluate(`window.__TAURI_INTERNALS__.invoke("prepare_candidate_preview", { jobId: ${JSON.stringify(snapshot.id)}, candidateId: ${JSON.stringify(snapshot.candidates[0].id)} })`);
if ((await stat(preview.path)).size < 1024) throw new Error("후보 영상 미리보기가 생성되지 않았습니다.");
const storage = await evaluate(`window.__TAURI_INTERNALS__.invoke("get_job_storage_info", { jobId: ${JSON.stringify(snapshot.id)} })`);
if (storage.sizeBytes < 1024) throw new Error("작업 저장 용량을 계산하지 못했습니다.");
if (screenshotPath) {
  const screenshot = await cdp("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
  await writeFile(screenshotPath, Buffer.from(screenshot.data, "base64"));
}

let deleteVerified = false;
if (verifyDelete) {
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

socket.close();
process.stdout.write(JSON.stringify({
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
  screenshotPath,
  acquisitionPath: acquisition?.mediaPath ?? null,
  bodyVerified: body.includes("채팅 움직임"),
  etaSeen,
  cancelVerified,
  deleteVerified
}, null, 2));
