import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import type {
  Candidate,
  CandidateDecision,
  CreateJobInput,
  JobStorageInfo,
  JobSnapshot,
  JobStatus,
  PreviewMedia,
  RuntimeInfo,
  StoredJobInfo
} from "./types";

const tauriAvailable = "__TAURI_INTERNALS__" in window;
const mockSubscribers = new Set<(job: JobSnapshot) => void>();
let mockJob: JobSnapshot | null = null;
let mockTimer: number | null = null;

const mockCandidates: Candidate[] = [
  {
    id: "candidate-1",
    startSeconds: 754,
    endSeconds: 802,
    title: "예상 밖의 보스 역전",
    summary: "목소리 반응과 채팅 활동이 동시에 치솟은 48초 구간",
    transcriptExcerpt: "잠깐, 이게 된다고? 아니 진짜 잡았어!",
    audioScore: 92,
    dialogueScore: 81,
    chatScore: 95,
    totalScore: 91,
    decision: "PENDING",
    contextStartSeconds: 730,
    contextEndSeconds: 828,
    contextTranscript: [
      { startSeconds: 730, endSeconds: 742, text: "체력이 거의 안 남았는데 한 번만 더 해볼게요." },
      { startSeconds: 742, endSeconds: 754, text: "이 패턴만 피하면 되는데 계속 맞네." },
      { startSeconds: 754, endSeconds: 780, text: "잠깐, 이게 된다고? 아니 진짜 잡았어!" },
      { startSeconds: 780, endSeconds: 802, text: "손이 아직도 떨려요. 방금 그거 보셨죠?" },
      { startSeconds: 802, endSeconds: 828, text: "일단 저장부터 하고 다음 구역으로 갈게요." }
    ]
  },
  {
    id: "candidate-2",
    startSeconds: 1922,
    endSeconds: 1960,
    title: "시청자와 완벽한 티키타카",
    summary: "짧은 발화가 빠르게 오가고 채팅 반응이 이어진 38초 구간",
    transcriptExcerpt: "그건 칭찬이 아니잖아. 방금 누가 인정했어?",
    audioScore: 66,
    dialogueScore: 94,
    chatScore: 86,
    totalScore: 86,
    decision: "PENDING",
    contextStartSeconds: 1900,
    contextEndSeconds: 1984,
    contextTranscript: [
      { startSeconds: 1900, endSeconds: 1912, text: "오늘 채팅 반응이 유난히 빠르네요." },
      { startSeconds: 1912, endSeconds: 1922, text: "아니 그걸 그렇게 받아치면 어떡해요." },
      { startSeconds: 1922, endSeconds: 1946, text: "그건 칭찬이 아니잖아. 방금 누가 인정했어?" },
      { startSeconds: 1946, endSeconds: 1960, text: "지금 이거 다시 보기로 남는 거 아시죠?" },
      { startSeconds: 1960, endSeconds: 1984, text: "자, 진정하고 다음 판 준비할게요." }
    ]
  },
  {
    id: "candidate-3",
    startSeconds: 3288,
    endSeconds: 3349,
    title: "갑작스러운 웃음 붕괴",
    summary: "반복 웃음과 음량 변화가 길게 유지된 61초 구간",
    transcriptExcerpt: "아 그만, 그만해. 나 진짜 숨 못 쉬겠어.",
    audioScore: 96,
    dialogueScore: 72,
    chatScore: 79,
    totalScore: 84,
    decision: "PENDING",
    contextStartSeconds: 3266,
    contextEndSeconds: 3372,
    contextTranscript: [
      { startSeconds: 3266, endSeconds: 3278, text: "잠깐만요, 지금 이름을 뭐라고 지으셨어요?" },
      { startSeconds: 3278, endSeconds: 3288, text: "그걸 진짜로 등록했다고요?" },
      { startSeconds: 3288, endSeconds: 3320, text: "아 그만, 그만해. 나 진짜 숨 못 쉬겠어." },
      { startSeconds: 3320, endSeconds: 3349, text: "눈물 나서 화면이 안 보여요 지금." },
      { startSeconds: 3349, endSeconds: 3372, text: "휴, 정리하고 원래 하던 거 마저 할게요." }
    ]
  }
];

const stageForUnit = (unit: number): [JobStatus, string, string] => {
  const stages: Array<[JobStatus, string, string]> = [
    ["ACQUIRING", "입력 준비", "입력 소스를 작업 공간에 등록했습니다."],
    ["PROBING", "미디어 확인", "컨테이너와 재생 시간을 확인했습니다."],
    ["EXTRACTING_AUDIO", "오디오 준비", "분석용 오디오 청크를 준비했습니다."],
    ["TRANSCRIBING", "전사 1/3", "첫 번째 전사 청크를 처리했습니다."],
    ["TRANSCRIBING", "전사 2/3", "두 번째 전사 청크를 처리했습니다."],
    ["TRANSCRIBING", "전사 3/3", "마지막 전사 청크를 처리했습니다."],
    ["AUDIO_SIGNALS", "오디오 신호", "말 밀도와 반응 신호를 계산했습니다."],
    ["CHAT_SIGNALS", "채팅 신호", "채팅 영역의 활동량을 계산했습니다."],
    ["FUSING", "신호 결합 1/2", "겹치는 반응 구간을 하나로 묶었습니다."],
    ["FUSING", "신호 결합 2/2", "중복된 구간을 정리했습니다."],
    ["RANKING", "후보 점수", "로컬 규칙으로 후보 점수를 계산했습니다."],
    ["RANKING", "검토 목록 준비", "검토할 후보 목록을 만들었습니다."]
  ];
  return stages[unit - 1];
};

function notifyMock() {
  if (!mockJob) return;
  mockJob.updatedAt = new Date().toISOString();
  mockSubscribers.forEach((subscriber) => subscriber(structuredClone(mockJob!)));
}

function addMockActivity(kind: string, message: string) {
  if (!mockJob) return;
  mockJob.activity.push({
    sequence: (mockJob.activity.at(-1)?.sequence ?? 0) + 1,
    timestamp: new Date().toISOString(),
    kind,
    message
  });
}

export async function bootstrap(): Promise<JobSnapshot | null> {
  if (tauriAvailable) return invoke("bootstrap");
  return mockJob ? structuredClone(mockJob) : null;
}

export async function getRuntimeInfo(): Promise<RuntimeInfo> {
  if (tauriAvailable) return invoke("get_runtime_info");
  return {
    appVersion: "0.3.1-web-preview",
    dataDirectory: "브라우저 미리보기: 메모리에만 저장",
    workerSource: "deterministic web fixture",
    analysisMode: "web fixture preview (desktop build runs local media)"
  };
}

export async function createJob(input: CreateJobInput): Promise<JobSnapshot> {
  if (tauriAvailable) return invoke("create_job", { input });
  const now = new Date().toISOString();
  mockJob = {
    schemaVersion: 5,
    id: crypto.randomUUID(),
    sourceKind: input.sourceKind,
    sourceLabel: input.sourceLabel,
    acquiredMediaPath: null,
    downloadPercent: null,
    scenario: input.scenario,
    analysisMode: input.analysisMode,
    analysisStartSeconds: input.analysisStartSeconds,
    analysisEndSeconds: input.analysisEndSeconds,
    status: "CREATED",
    completedUnits: 0,
    totalUnits: 12,
    mediaDurationSeconds: null,
    currentStageLabel: "실행 대기",
    lastHeartbeatAt: null,
    createdAt: now,
    updatedAt: now,
    errorMessage: null,
    errorDetail: null,
    candidates: [],
    activity: [{ sequence: 1, timestamp: now, kind: "job", message: "새 분석 작업을 만들었습니다." }],
    captions: null,
    whisper: input.whisper
  };
  notifyMock();
  return structuredClone(mockJob);
}

export async function startJob(jobId: string): Promise<JobSnapshot> {
  if (tauriAvailable) return invoke("start_job", { jobId });
  if (!mockJob || mockJob.id !== jobId) throw new Error("현재 작업을 찾을 수 없습니다.");
  if (mockTimer) window.clearInterval(mockTimer);
  mockJob.status = mockJob.completedUnits ? stageForUnit(mockJob.completedUnits)[0] : "ACQUIRING";
  mockJob.errorMessage = null;
  mockJob.errorDetail = null;
  addMockActivity("start", mockJob.completedUnits ? "저장된 지점 다음부터 재개합니다." : "분석 worker를 시작합니다.");
  notifyMock();

  mockTimer = window.setInterval(() => {
    if (!mockJob) return;
    const next = mockJob.completedUnits + 1;
    if (mockJob.scenario === "fail" && next === 5) {
      mockJob.status = "FAILED";
      mockJob.errorMessage = "전사 도구가 응답하지 않았습니다.";
      mockJob.errorDetail = "브라우저 fixture: unit 5 controlled failure";
      addMockActivity("error", "분석 단계에서 복구 가능한 오류가 발생했습니다.");
      mockJob.scenario = "normal";
      window.clearInterval(mockTimer!);
      mockTimer = null;
      notifyMock();
      return;
    }
    if (mockJob.scenario === "crash" && next === 7 && mockJob.completedUnits === 6) {
      mockJob.status = "INTERRUPTED";
      mockJob.errorMessage = "분석 worker가 예상보다 일찍 종료됐습니다.";
      mockJob.errorDetail = "브라우저 fixture: crash after unit 6";
      addMockActivity("interrupted", "마지막 완료 단위에서 작업이 중단됐습니다.");
      mockJob.scenario = "normal";
      window.clearInterval(mockTimer!);
      mockTimer = null;
      notifyMock();
      return;
    }
    if (mockJob.scenario === "hang" && next === 5) {
      mockJob.status = "INTERRUPTED";
      mockJob.errorMessage = "분석 worker의 응답이 멈췄습니다.";
      mockJob.errorDetail = "3초 동안 heartbeat를 받지 못했습니다.";
      addMockActivity("stalled", "응답 정지를 감지해 worker를 종료했습니다.");
      mockJob.scenario = "normal";
      window.clearInterval(mockTimer!);
      mockTimer = null;
      notifyMock();
      return;
    }
    if (mockJob.scenario === "malformed" && next === 3) {
      mockJob.status = "FAILED";
      mockJob.errorMessage = "분석 worker가 이해할 수 없는 응답을 보냈습니다.";
      mockJob.errorDetail = "브라우저 fixture: invalid JSON event";
      addMockActivity("error", "잘못된 worker 이벤트 때문에 작업을 중지했습니다.");
      mockJob.scenario = "normal";
      window.clearInterval(mockTimer!);
      mockTimer = null;
      notifyMock();
      return;
    }
    const [status, label, message] = stageForUnit(next);
    mockJob.completedUnits = next;
    mockJob.status = status;
    mockJob.currentStageLabel = label;
    mockJob.lastHeartbeatAt = new Date().toISOString();
    addMockActivity("progress", message);
    if (next === 12) {
      mockJob.status = "REVIEW_READY";
      mockJob.currentStageLabel = "후보 검토 준비";
      mockJob.candidates = structuredClone(mockCandidates);
      addMockActivity("complete", "분석을 마쳤습니다. 후보를 검토해 주세요.");
      window.clearInterval(mockTimer!);
      mockTimer = null;
    }
    notifyMock();
  }, 320);
  return structuredClone(mockJob);
}

export async function cancelJob(jobId: string): Promise<JobSnapshot> {
  if (tauriAvailable) return invoke("cancel_job", { jobId });
  if (!mockJob || mockJob.id !== jobId) throw new Error("현재 작업을 찾을 수 없습니다.");
  if (mockTimer) window.clearInterval(mockTimer);
  mockTimer = null;
  mockJob.status = "CANCELLED";
  mockJob.currentStageLabel = "사용자가 취소함";
  addMockActivity("cancel", "작업을 안전하게 취소했습니다. 이어서 재개할 수 있습니다.");
  notifyMock();
  return structuredClone(mockJob);
}

export async function setCandidateDecision(
  jobId: string,
  candidateId: string,
  decision: CandidateDecision
): Promise<JobSnapshot> {
  if (tauriAvailable) return invoke("set_candidate_decision", { jobId, candidateId, decision });
  if (!mockJob || mockJob.id !== jobId) throw new Error("현재 작업을 찾을 수 없습니다.");
  const candidate = mockJob.candidates.find((item) => item.id === candidateId);
  if (!candidate) throw new Error("후보를 찾을 수 없습니다.");
  candidate.decision = decision;
  addMockActivity("review", `후보를 ${decision === "ACCEPTED" ? "채택" : decision === "REJECTED" ? "제외" : "보류"} 처리했습니다.`);
  notifyMock();
  return structuredClone(mockJob);
}

export async function subscribeToJob(callback: (job: JobSnapshot) => void): Promise<() => void> {
  if (tauriAvailable) {
    return listen<JobSnapshot>("job-updated", (event) => callback(event.payload));
  }
  mockSubscribers.add(callback);
  return () => mockSubscribers.delete(callback);
}

export async function chooseLocalVideo(): Promise<string | null> {
  if (!tauriAvailable) return null;
  const result = await open({
    multiple: false,
    directory: false,
    title: "분석할 VOD 선택",
    filters: [{ name: "영상", extensions: ["mp4", "mkv", "webm", "mov", "avi", "flv"] }]
  });
  return typeof result === "string" ? result : null;
}

export async function getJobStorageInfo(jobId: string): Promise<JobStorageInfo> {
  if (tauriAvailable) return invoke("get_job_storage_info", { jobId });
  return { sizeBytes: new Blob([JSON.stringify(mockJob)]).size };
}

export async function confirmJobDeletion(sizeLabel: string): Promise<boolean> {
  const message = `현재 작업과 저장된 영상·전사·미리보기 ${sizeLabel}를 삭제합니다. 로컬에서 직접 선택한 원본 파일은 삭제하지 않습니다.`;
  if (tauriAvailable) {
    return confirm(message, { title: "작업 삭제", kind: "warning" });
  }
  return window.confirm(message);
}

export async function deleteJob(jobId: string): Promise<void> {
  if (tauriAvailable) {
    await invoke("delete_job", { jobId });
    return;
  }
  if (!mockJob || mockJob.id !== jobId) throw new Error("현재 작업을 찾을 수 없습니다.");
  mockJob = null;
}

export async function prepareCandidatePreview(jobId: string, candidateId: string): Promise<PreviewMedia> {
  if (!tauriAvailable) throw new Error("브라우저 미리보기에는 로컬 영상이 연결되지 않습니다.");
  return invoke("prepare_candidate_preview", { jobId, candidateId });
}

export async function prepareCandidateContextPreview(jobId: string, candidateId: string): Promise<PreviewMedia> {
  if (!tauriAvailable) throw new Error("브라우저 미리보기에는 로컬 영상이 연결되지 않습니다.");
  return invoke("prepare_candidate_context_preview", { jobId, candidateId });
}

export function previewMediaUrl(path: string): string {
  return tauriAvailable ? convertFileSrc(path) : path;
}

function mockCsv(): string {
  if (!mockJob) return "";
  const escape = (value: string) => {
    const cleaned = value.replaceAll("\0", "");
    const safe = /^[\s]*[=+\-@]/.test(cleaned) ? `'${cleaned}` : cleaned;
    return `"${safe.replaceAll('"', '""')}"`;
  };
  const rows = ["rank,start,end,total,audio,dialogue,chat,decision,title,transcript"];
  mockJob.candidates.forEach((candidate, index) => {
    rows.push([
      index + 1,
      candidate.startSeconds,
      candidate.endSeconds,
      candidate.totalScore,
      candidate.audioScore,
      candidate.dialogueScore,
      candidate.chatScore ?? "",
      candidate.decision,
      escape(candidate.title),
      escape(candidate.transcriptExcerpt)
    ].join(","));
  });
  return rows.join("\r\n");
}

export async function saveCandidatesCsv(jobId: string): Promise<string | null> {
  if (tauriAvailable) {
    return invoke("export_candidates_csv", { jobId });
  }
  const url = URL.createObjectURL(new Blob(["\uFEFF", mockCsv()], { type: "text/csv;charset=utf-8" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = "vod-scout-candidates.csv";
  anchor.click();
  URL.revokeObjectURL(url);
  return "vod-scout-candidates.csv";
}

export async function listJobs(): Promise<StoredJobInfo[]> {
  if (tauriAvailable) return invoke("list_jobs");
  return mockJob ? [{ snapshot: structuredClone(mockJob), sizeBytes: new Blob([JSON.stringify(mockJob)]).size }] : [];
}

export async function deleteStoredJob(jobId: string): Promise<void> {
  if (tauriAvailable) {
    await invoke("delete_stored_job", { jobId });
    return;
  }
  if (mockJob?.id === jobId) mockJob = null;
}

export async function deleteAllJobs(): Promise<void> {
  if (tauriAvailable) {
    await invoke("delete_all_jobs");
    return;
  }
  mockJob = null;
}

export const isDesktopRuntime = tauriAvailable;
