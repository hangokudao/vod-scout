import { useEffect, useMemo, useRef, useState } from "react";
import {
  Activity,
  AlertTriangle,
  Bot,
  Check,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  Circle,
  Copy,
  Clock3,
  Download,
  FileVideo2,
  FolderOpen,
  Gauge,
  HardDrive,
  ListChecks,
  MessageSquareText,
  Mic2,
  Pause,
  Play,
  Radio,
  RefreshCw,
  RotateCcw,
  Settings2,
  Sparkles,
  Square,
  TerminalSquare,
  Timer,
  Trash2,
  Tv,
  Waves,
  X
} from "lucide-react";
import {
  bootstrap,
  cancelJob,
  chooseLocalVideo,
  confirmJobDeletion,
  createJob,
  deleteJob,
  deleteAllJobs,
  deleteStoredJob,
  getRuntimeInfo,
  getJobStorageInfo,
  listJobs,
  isDesktopRuntime,
  prepareCandidateContextPreview,
  previewMediaUrl,
  saveCandidatesCsv,
  setCandidateDecision,
  startJob,
  subscribeToJob
} from "./api";
import type {
  Candidate,
  CandidateDecision,
  CaptionSummary,
  ContextLine,
  AnalysisMode,
  JobSnapshot,
  JobStatus,
  RuntimeInfo,
  PreviewMedia,
  Scenario,
  SourceKind,
  StoredJobInfo,
  WhisperDeviceMode,
  WhisperProfile
} from "./types";

export function normalizeWhisperSettings(
  deviceMode: WhisperDeviceMode,
  profile: WhisperProfile,
  cpuThreads: string
) {
  const parsed = cpuThreads === "auto" ? null : Number(cpuThreads);
  return {
    deviceMode,
    profile,
    cpuThreads: parsed == null || !Number.isFinite(parsed) ? null : Math.min(Math.max(Math.trunc(parsed), 1), 32)
  };
}

function captionSummaryLabel(captions: CaptionSummary | null | undefined): string | null {
  if (!captions) return null;
  const source = captions.provenance?.verificationState === "FAILED"
    ? "출처 알 수 없는 자막"
    : captions.source === "creator" ? "제작자 한국어 자막" : captions.source === "automatic" ? "한국어 자동 자막" : "자막 없음";
  const quality = captions.quality === "failed" ? "검증 실패" : captions.quality === "trusted" ? "검증된 구간" : captions.quality === "mixed" ? "일부 구간 대체" : "검증 전";
  if (captions.quality === "failed") return `${source} · ${quality}`;
  if (captions.fallbackIntervals > 0) return `${source} · Whisper 대체 ${captions.fallbackIntervals}구간`;
  return `${source} · ${quality}`;
}

function captionDiagnosticLabel(kind: string): string {
  if (["StartAfterEnd", "OutOfRange", "Overlap", "Duplicate", "EmptyText", "QualityWarning"].includes(kind)) return "시간·내용 구조";
  if (kind === "OffsetUnverified") return "시간 오프셋";
  if (kind === "GapObserved") return "자막 공백";
  if (kind === "ProvenanceInvalid") return "근거 확인";
  return "음성 인식 대체";
}

function captionVerificationLabel(value: CaptionSummary["provenance"]): string {
  if (!value) return "확인 정보 없음";
  return value.verificationState === "VERIFIED" ? "검증됨" : value.verificationState === "FAILED" ? "검증 실패" : "검증 전";
}

function CaptionDetails({ captions }: { captions: CaptionSummary }) {
  const provenance = captions.provenance;
  const source = provenance?.verificationState === "FAILED"
    ? "알 수 없음"
    : captions.source === "creator" ? "제작자" : captions.source === "automatic" ? "자동" : "없음";
  return (
    <div className="caption-details" aria-label="YouTube 자막 근거">
      <div className="caption-detail-row">
        <span>자막 출처: {source}</span>
        <span>언어: {captions.language ?? provenance?.language ?? "알 수 없음"}</span>
        <span>트랙: {provenance?.trackId || "알 수 없음"}</span>
      </div>
      <div className="caption-detail-row">
        <span>원본 파일: {provenance?.originalFile || "없음"}</span>
        <span>SHA-256: {provenance?.sha256 || "없음"}</span>
        <span>검증: {captionVerificationLabel(provenance)}</span>
      </div>
      <div className="caption-detail-row">
        <span>로컬 음성 인식 대체: {captions.localWhisperFallback ? `${captions.fallbackIntervals}구간` : "없음"}</span>
      </div>
      {captions.diagnostics.length > 0 ? (
        <ul className="caption-diagnostics">
          {captions.diagnostics.map((diagnostic, index) => <li key={`${diagnostic.kind}-${index}`}><strong>{captionDiagnosticLabel(diagnostic.kind)}</strong> · {diagnostic.detail}</li>)}
        </ul>
      ) : null}
    </div>
  );
}
import {
  ACTIVE_STATUSES,
  estimateJobTiming,
  formatBytes,
  formatDuration,
  formatTime,
  parseTimeInput,
  RESUMABLE_STATUSES,
  shortSource,
  statusLabel
} from "./utils";
import { checkForAppUpdate, installPendingUpdate, type AppUpdateInfo } from "./updater";
import { CURRENT_RELEASE_NOTES } from "./releaseNotes";

const PIPELINE = [
  { label: "미디어 확인", statuses: ["ACQUIRING", "PROBING"], icon: FileVideo2 },
  { label: "오디오·음성 인식", statuses: ["EXTRACTING_AUDIO", "TRANSCRIBING"], icon: Mic2 },
  { label: "반응 신호", statuses: ["AUDIO_SIGNALS", "CHAT_SIGNALS"], icon: Activity },
  { label: "후보 조합", statuses: ["FUSING"], icon: Sparkles },
  { label: "순위 결정", statuses: ["RANKING"], icon: Gauge },
  { label: "검토 목록", statuses: ["REVIEW_READY"], icon: ListChecks }
] as const;

const PIPELINE_STATUS_ORDER: JobStatus[] = [
  "ACQUIRING", "PROBING", "EXTRACTING_AUDIO", "TRANSCRIBING", "AUDIO_SIGNALS",
  "CHAT_SIGNALS", "FUSING", "RANKING", "REVIEW_READY"
];

const SOURCE_META: Record<SourceKind, { label: string; hint: string; icon: typeof FileVideo2 }> = {
  demo: { label: "데모", hint: "영상 없이 실행기 흐름을 검증합니다.", icon: Radio },
  local: { label: "로컬 파일", hint: "FFmpeg와 Whisper로 PC 안에서 분석합니다.", icon: FileVideo2 },
  youtube: { label: "YouTube", hint: "최대 720p로 내려받아 PC 안에서 분석합니다.", icon: Tv }
};

const SCENARIOS: Array<{ value: Scenario; label: string; detail: string }> = [
  { value: "normal", label: "정상 완료", detail: "12단위 처리 후 후보 3개 생성" },
  { value: "fail", label: "제어된 실패", detail: "5단위에서 실패하고 재개 가능" },
  { value: "crash", label: "worker 충돌", detail: "6단위 뒤 종료, 7부터 재개" },
  { value: "hang", label: "응답 정지", detail: "heartbeat 만료 감지" },
  { value: "malformed", label: "잘못된 이벤트", detail: "프로토콜 오류를 사용자 실패로 전환" }
];

const ANALYSIS_MODES: Array<{ value: AnalysisMode; label: string; detail: string }> = [
  { value: "quick", label: "빠른 분석", detail: "전체를 훑고 최대 120분만 분산 음성 인식" },
  { value: "range", label: "구간 지정", detail: "선택한 시작·종료 범위만 정밀 분석" },
  { value: "full", label: "전체 정밀 분석", detail: "전체 오디오를 10분 단위로 음성 인식" }
];

const WHISPER_DEVICES: Array<{ value: WhisperDeviceMode; label: string; detail: string }> = [
  { value: "auto", label: "자동(GPU 우선)", detail: "짧은 실제 시험이 성공한 경우에만 GPU를 사용합니다." },
  { value: "gpu", label: "GPU", detail: "GPU 시험·실행 실패 시 해당 구간만 CPU로 한 번 대체합니다." },
  { value: "cpu", label: "CPU", detail: "GPU를 명시적으로 끄고 CPU에서만 실행합니다." }
];

const WHISPER_PROFILES: Array<{ value: WhisperProfile; label: string; detail: string }> = [
  { value: "fast", label: "빠르게", detail: "낮은 탐색 폭으로 빠르게 처리" },
  { value: "balanced", label: "균형", detail: "기본 설정" },
  { value: "accurate", label: "정확하게", detail: "더 넓은 탐색 폭으로 처리" }
];

export type CandidateSortKey =
  | "totalScore"
  | "startSeconds"
  | "audioScore"
  | "dialogueScore"
  | "chatScore"
  | "decision";

export const CANDIDATE_SORTS: Array<{ value: CandidateSortKey; label: string }> = [
  { value: "totalScore", label: "종합 점수 높은 순" },
  { value: "startSeconds", label: "원본 영상 시간순" },
  { value: "audioScore", label: "오디오 반응 높은 순" },
  { value: "dialogueScore", label: "대화 밀도 높은 순" },
  { value: "chatScore", label: "채팅 움직임 높은 순" },
  { value: "decision", label: "채택·보류·제외 상태" }
];

const DECISION_SORT_RANK: Record<CandidateDecision, number> = { ACCEPTED: 0, PENDING: 1, REJECTED: 2 };

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function compareByPrimaryKey(a: Candidate, b: Candidate, key: CandidateSortKey) {
  if (key === "startSeconds") return a.startSeconds - b.startSeconds;
  if (key === "decision") return DECISION_SORT_RANK[a.decision] - DECISION_SORT_RANK[b.decision];
  if (key === "chatScore") {
    // 채팅 신호를 계산하지 못한 후보는 항상 목록 끝에 둔다.
    if (a.chatScore === null || b.chatScore === null) {
      if (a.chatScore === b.chatScore) return 0;
      return a.chatScore === null ? 1 : -1;
    }
    return b.chatScore - a.chatScore;
  }
  return b[key] - a[key];
}

/**
 * 화면에 보이는 순서만 바꾼다. 점수와 판정은 읽기만 하고 원본 배열도 건드리지 않는다.
 * 같은 입력은 언제나 같은 순서를 만든다. 값이 같으면 원본 시간, 그다음 후보 식별자 순이다.
 */
export function sortCandidates(candidates: readonly Candidate[], key: CandidateSortKey): Candidate[] {
  return [...candidates].sort((a, b) => {
    const primary = compareByPrimaryKey(a, b, key);
    if (primary !== 0) return primary;
    if (a.startSeconds !== b.startSeconds) return a.startSeconds - b.startSeconds;
    return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
  });
}

export type ThemePreference = "system" | "light" | "dark";

export const THEME_OPTIONS: Array<{ value: ThemePreference; label: string }> = [
  { value: "system", label: "시스템 설정 사용" },
  { value: "light", label: "밝게" },
  { value: "dark", label: "어둡게" }
];

export interface UiSettings {
  theme: ThemePreference;
  sortKey: CandidateSortKey;
}

export const DEFAULT_UI_SETTINGS: UiSettings = { theme: "system", sortKey: "totalScore" };

/** 화면 설정은 앱 전체에서 이 키 하나만 사용한다. */
export const SETTINGS_STORAGE_KEY = "vod-scout.settings.v1";

/** 선택한 후보는 작업마다 따로 기억한다. 순서가 아니라 후보 식별자를 저장한다. */
export function selectionStorageKey(jobId: string) {
  return `vod-scout.selected-candidate.${jobId}`;
}

function safeStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export function readUiSettings(storage: Storage | null = safeStorage()): UiSettings {
  try {
    const raw = storage?.getItem(SETTINGS_STORAGE_KEY);
    if (!raw) return DEFAULT_UI_SETTINGS;
    const parsed = JSON.parse(raw) as Partial<UiSettings>;
    return {
      theme: THEME_OPTIONS.some((item) => item.value === parsed.theme) ? parsed.theme! : DEFAULT_UI_SETTINGS.theme,
      sortKey: CANDIDATE_SORTS.some((item) => item.value === parsed.sortKey) ? parsed.sortKey! : DEFAULT_UI_SETTINGS.sortKey
    };
  } catch {
    return DEFAULT_UI_SETTINGS;
  }
}

export function writeUiSettings(settings: UiSettings, storage: Storage | null = safeStorage()) {
  try {
    storage?.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // 저장 공간을 쓸 수 없어도 이번 실행 동안의 화면 설정은 그대로 동작해야 한다.
  }
}

export function resolveTheme(preference: ThemePreference, prefersDark: boolean): "light" | "dark" {
  return preference === "system" ? (prefersDark ? "dark" : "light") : preference;
}

/** worker가 맥락 구간을 보내지 않는 작업에서 사용할 기본 여유 시간. */
export const DEFAULT_CONTEXT_PADDING_SECONDS = 15;

export interface CandidateContext {
  startSeconds: number;
  endSeconds: number;
  lines: ContextLine[];
  fromWorker: boolean;
}

/**
 * 맥락 구간을 정한다. 원본 시작·끝을 벗어나지 않고 후보 구간을 항상 포함한다.
 * worker가 보낸 값이 있으면 그 값을 쓰고, 없으면 기본 여유 시간을 적용한다.
 */
export function resolveCandidateContext(
  candidate: Candidate,
  mediaDurationSeconds: number | null
): CandidateContext {
  const requestedStart = candidate.contextStartSeconds ?? candidate.startSeconds - DEFAULT_CONTEXT_PADDING_SECONDS;
  const requestedEnd = candidate.contextEndSeconds ?? candidate.endSeconds + DEFAULT_CONTEXT_PADDING_SECONDS;
  const sourceEnd = mediaDurationSeconds && mediaDurationSeconds > 0
    ? Math.max(mediaDurationSeconds, candidate.endSeconds)
    : Math.max(requestedEnd, candidate.endSeconds);
  return {
    startSeconds: clamp(requestedStart, 0, candidate.startSeconds),
    endSeconds: clamp(requestedEnd, candidate.endSeconds, sourceEnd),
    lines: [...(candidate.contextTranscript ?? [])].sort((a, b) => a.startSeconds - b.startSeconds),
    fromWorker: candidate.contextStartSeconds != null || candidate.contextEndSeconds != null
  };
}

export type UpdateStatusKind = "unknown" | "checking" | "current" | "available" | "waiting" | "error";

export interface UpdateStatusView {
  kind: UpdateStatusKind;
  label: string;
  detail: string;
}

/** 최신 상태, 새 버전, 확인 실패, 분석 종료 후 설치 대기를 서로 다른 상태로 구분한다. */
export function resolveUpdateStatus(input: {
  checking: boolean;
  available: boolean;
  checkedAt: string | null;
  failed: boolean;
  analysisActive: boolean;
}): UpdateStatusView {
  if (input.checking) {
    return { kind: "checking", label: "확인 중", detail: "GitHub Releases에서 최신 안정 버전을 확인하고 있습니다." };
  }
  if (input.failed) {
    return {
      kind: "error",
      label: "확인 실패",
      detail: "업데이트 서버에 연결하지 못했습니다. 로컬 영상 분석과 저장된 작업 검토는 그대로 사용할 수 있습니다."
    };
  }
  if (input.available && input.analysisActive) {
    return {
      kind: "waiting",
      label: "분석 종료 후 설치 대기",
      detail: "새 버전을 설치할 준비가 됐습니다. 실행 중인 분석을 마치거나 안전하게 취소한 뒤 설치합니다."
    };
  }
  if (input.available) {
    return { kind: "available", label: "새 버전 있음", detail: "서명된 안정 버전을 지금 설치할 수 있습니다." };
  }
  if (input.checkedAt) {
    return { kind: "current", label: "최신 상태", detail: "현재 최신 안정 버전을 사용 중입니다." };
  }
  return {
    kind: "unknown",
    label: "확인 전",
    detail: "앱을 실행하면 자동으로 확인합니다. 연결하지 못해도 로컬 분석은 그대로 사용할 수 있습니다."
  };
}

function statusTone(status: JobStatus) {
  if (status === "REVIEW_READY") return "success";
  if (status === "FAILED" || status === "INTERRUPTED") return "warning";
  if (status === "CANCELLED") return "muted";
  if (ACTIVE_STATUSES.includes(status)) return "active";
  return "neutral";
}

function StatusPill({ status }: { status: JobStatus }) {
  const active = ACTIVE_STATUSES.includes(status) && status !== "CANCELLING";
  return (
    <span className={`status-pill ${statusTone(status)}`}>
      {active ? <span className="live-dot" aria-hidden="true" /> : null}
      {statusLabel(status)}
    </span>
  );
}

function SignalRail({ candidate }: { candidate: Candidate }) {
  const signals = [
    { label: "오디오", score: candidate.audioScore, className: "audio" },
    { label: "대화", score: candidate.dialogueScore, className: "dialogue" },
    { label: "채팅", score: candidate.chatScore, className: "chat" }
  ];
  return (
    <div className="signal-rail" aria-label="후보 선택 근거 점수">
      {signals.map((signal) => (
        <div className={`signal-row ${signal.score === null ? "unavailable" : ""}`} key={signal.label}>
          <span>{signal.label}</span>
          <div className="signal-track" aria-hidden="true">
            <div className={`signal-fill ${signal.className}`} style={{ width: `${signal.score ?? 0}%` }} />
          </div>
          <strong>{signal.score ?? "—"}</strong>
        </div>
      ))}
    </div>
  );
}

function DecisionMark({ decision }: { decision: CandidateDecision }) {
  if (decision === "ACCEPTED") return <span className="decision accepted"><Check size={13} /> 채택</span>;
  if (decision === "REJECTED") return <span className="decision rejected"><X size={13} /> 제외</span>;
  return <span className="decision pending"><Circle size={10} /> 미검토</span>;
}

function App() {
  const [job, setJob] = useState<JobSnapshot | null>(null);
  const [runtime, setRuntime] = useState<RuntimeInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [actionBusy, setActionBusy] = useState(false);
  const [uiError, setUiError] = useState<string | null>(null);
  const [newJobMode, setNewJobMode] = useState(true);
  const [sourceKind, setSourceKind] = useState<SourceKind>("demo");
  const [sourceLabel, setSourceLabel] = useState("8시간 샘플 방송 · fixture");
  const [scenario, setScenario] = useState<Scenario>("normal");
  const [analysisMode, setAnalysisMode] = useState<AnalysisMode>("quick");
  const [rangeStart, setRangeStart] = useState("00:00:00");
  const [rangeEnd, setRangeEnd] = useState("01:00:00");
  const [whisperDevice, setWhisperDevice] = useState<WhisperDeviceMode>("auto");
  const [whisperProfile, setWhisperProfile] = useState<WhisperProfile>("balanced");
  const [cpuThreads, setCpuThreads] = useState("auto");
  const [selectedCandidateId, setSelectedCandidateId] = useState<string | null>(null);
  const [settings, setSettings] = useState<UiSettings>(() => readUiSettings());
  const [systemPrefersDark, setSystemPrefersDark] = useState(
    () => window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false
  );
  const [storageBytes, setStorageBytes] = useState<number | null>(null);
  const [clock, setClock] = useState(Date.now());
  const [preview, setPreview] = useState<PreviewMedia | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [playheadSeconds, setPlayheadSeconds] = useState(0);
  const [notice, setNotice] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [storedJobs, setStoredJobs] = useState<StoredJobInfo[]>([]);
  const [updateInfo, setUpdateInfo] = useState<AppUpdateInfo | null>(null);
  const [updateChecking, setUpdateChecking] = useState(false);
  const [updateInstalling, setUpdateInstalling] = useState(false);
  const [updateProgress, setUpdateProgress] = useState<number | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [updateCheckedAt, setUpdateCheckedAt] = useState<string | null>(null);
  const [updateInstallError, setUpdateInstallError] = useState<string | null>(null);
  const browserFileInput = useRef<HTMLInputElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const autoplayPreview = useRef(false);

  useEffect(() => {
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    (async () => {
      try {
        unsubscribe = await subscribeToJob((nextJob) => {
          if (!disposed) {
            setJob(nextJob);
            setNewJobMode(false);
          }
        });
        const [restored, runtimeInfo] = await Promise.all([bootstrap(), getRuntimeInfo()]);
        if (!disposed) {
          setRuntime(runtimeInfo);
          if (restored) {
            setJob(restored);
            setWhisperDevice(restored.whisper?.deviceMode ?? "auto");
            setWhisperProfile(restored.whisper?.profile ?? "balanced");
            setCpuThreads(restored.whisper?.cpuThreads == null ? "auto" : String(restored.whisper.cpuThreads));
            setNewJobMode(false);
          }
        }
      } catch (error) {
        if (!disposed) setUiError(messageFrom(error));
      } finally {
        if (!disposed) setLoading(false);
      }
    })();
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, []);

  useEffect(() => {
    if (loading || !isDesktopRuntime) return;
    void refreshUpdate(false);
  }, [loading]);

  useEffect(() => {
    const query = window.matchMedia?.("(prefers-color-scheme: dark)");
    if (!query) return;
    const onChange = (event: MediaQueryListEvent) => setSystemPrefersDark(event.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);

  const resolvedTheme = resolveTheme(settings.theme, systemPrefersDark);

  useEffect(() => {
    document.documentElement.dataset.theme = resolvedTheme;
    document.documentElement.style.colorScheme = resolvedTheme;
  }, [resolvedTheme]);

  useEffect(() => {
    writeUiSettings(settings);
  }, [settings]);

  // 작업을 열 때 저장해 둔 후보 식별자를 되살린다. 목록에 없으면 조용히 버린다.
  useEffect(() => {
    if (!job?.id) {
      setSelectedCandidateId(null);
      return;
    }
    setSelectedCandidateId(safeStorage()?.getItem(selectionStorageKey(job.id)) ?? null);
  }, [job?.id]);

  useEffect(() => {
    if (!job || !ACTIVE_STATUSES.includes(job.status)) return;
    const timer = window.setInterval(() => setClock(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [job?.id, job?.status]);

  useEffect(() => {
    if (!job) {
      setStorageBytes(null);
      return;
    }
    let disposed = false;
    void getJobStorageInfo(job.id)
      .then((info) => { if (!disposed) setStorageBytes(info.sizeBytes); })
      .catch(() => { if (!disposed) setStorageBytes(null); });
    return () => { disposed = true; };
  }, [job?.id, job?.status, job?.completedUnits]);

  const active = job ? ACTIVE_STATUSES.includes(job.status) : false;
  const resumable = job ? RESUMABLE_STATUSES.includes(job.status) : false;
  const sortedCandidates = useMemo(
    () => sortCandidates(job?.candidates ?? [], settings.sortKey),
    [job?.candidates, settings.sortKey]
  );
  // 선택은 순서가 아니라 후보 식별자를 따른다. 정렬을 바꿔도 같은 후보가 남는다.
  const selected = sortedCandidates.find((candidate) => candidate.id === selectedCandidateId) ?? sortedCandidates[0] ?? null;
  const selectedIndex = selected ? sortedCandidates.findIndex((candidate) => candidate.id === selected.id) : -1;
  const context = selected ? resolveCandidateContext(selected, job?.mediaDurationSeconds ?? null) : null;
  const updateStatus = resolveUpdateStatus({
    checking: updateChecking,
    available: !!updateInfo?.available,
    checkedAt: updateCheckedAt,
    failed: !!updateError && !updateInstalling,
    analysisActive: active
  });
  const checkpointPercent = job?.totalUnits ? Math.round((job.completedUnits / job.totalUnits) * 100) : 0;
  const percent = job?.sourceKind === "youtube" && job.status === "ACQUIRING" && job.completedUnits === 0
    ? (job.downloadPercent ?? 0)
    : checkpointPercent;
  const reviewedCount = job?.candidates.filter((candidate) => candidate.decision !== "PENDING").length ?? 0;
  const timing = job ? estimateJobTiming(job, new Date(clock)) : null;
  const audioSignalsReady = !!job && PIPELINE_STATUS_ORDER.indexOf(job.status) >= PIPELINE_STATUS_ORDER.indexOf("AUDIO_SIGNALS");
  const chatSignalsReady = !!job && PIPELINE_STATUS_ORDER.indexOf(job.status) >= PIPELINE_STATUS_ORDER.indexOf("CHAT_SIGNALS");

  useEffect(() => {
    if (!job?.id || !selected) return;
    try {
      safeStorage()?.setItem(selectionStorageKey(job.id), selected.id);
    } catch {
      // 저장에 실패해도 이번 실행 동안의 선택은 그대로 유지된다.
    }
  }, [job?.id, selected?.id]);

  useEffect(() => {
    if (!job || job.status !== "REVIEW_READY" || !selected || !isDesktopRuntime) {
      setPreview(null);
      setPreviewUrl(null);
      setPreviewError(null);
      return;
    }
    let disposed = false;
    setPreviewLoading(true);
    setPreviewError(null);
    void prepareCandidateContextPreview(job.id, selected.id)
      .then((media) => {
        if (disposed) return;
        setPreview(media);
        setPreviewUrl(previewMediaUrl(media.path));
        setPlayheadSeconds(selected.startSeconds);
        void getJobStorageInfo(job.id).then((info) => setStorageBytes(info.sizeBytes));
      })
      .catch((error) => {
        if (!disposed) setPreviewError(messageFrom(error));
      })
      .finally(() => {
        if (!disposed) setPreviewLoading(false);
      });
    return () => { disposed = true; };
  }, [job?.id, job?.status, selected?.id]);

  const primaryLabel = useMemo(() => {
    if (actionBusy) return "처리 중…";
    if (resumable) return job?.sourceKind === "youtube" ? "다운로드·분석 재개" : `${job?.completedUnits ?? 0}단위 다음부터 재개`;
    if (job?.status === "CREATED" && !newJobMode) return "분석 시작";
    return "작업 만들고 시작";
  }, [actionBusy, job?.completedUnits, job?.status, newJobMode, resumable]);

  function messageFrom(error: unknown) {
    if (typeof error === "string") return error;
    if (error instanceof Error) return error.message;
    return "요청을 처리하지 못했습니다.";
  }

  async function runPrimary() {
    if (actionBusy || active) return;
    setActionBusy(true);
    setUiError(null);
    try {
      let target = job;
      if (newJobMode || !target || target.status === "REVIEW_READY") {
        if (!sourceLabel.trim()) throw new Error("입력 소스를 선택하거나 주소를 입력해 주세요.");
        const start = analysisMode === "range" ? parseTimeInput(rangeStart) : null;
        const end = analysisMode === "range" ? parseTimeInput(rangeEnd) : null;
        if (analysisMode === "range" && (start === null || end === null || start >= end)) {
          throw new Error("분석 구간을 HH:MM:SS 형식으로 올바르게 입력해 주세요.");
        }
        target = await createJob({
          sourceKind,
          sourceLabel: sourceLabel.trim(),
          scenario,
          analysisMode,
          analysisStartSeconds: start,
          analysisEndSeconds: end,
          whisper: normalizeWhisperSettings(whisperDevice, whisperProfile, cpuThreads)
        });
        setJob(target);
        setNewJobMode(false);
      }
      const started = await startJob(target.id);
      setJob(started);
    } catch (error) {
      setUiError(messageFrom(error));
    } finally {
      setActionBusy(false);
    }
  }

  async function requestCancel() {
    if (!job || !active || job.status === "CANCELLING") return;
    setActionBusy(true);
    setUiError(null);
    try {
      setJob(await cancelJob(job.id));
    } catch (error) {
      setUiError(messageFrom(error));
    } finally {
      setActionBusy(false);
    }
  }

  async function decide(decision: CandidateDecision) {
    if (!job || !selected) return;
    setActionBusy(true);
    try {
      setJob(await setCandidateDecision(job.id, selected.id, decision));
    } catch (error) {
      setUiError(messageFrom(error));
    } finally {
      setActionBusy(false);
    }
  }

  function chooseCandidate(candidateId: string | undefined, play = false) {
    const target = sortedCandidates.find((candidate) => candidate.id === candidateId);
    if (!target || previewLoading) return;
    autoplayPreview.current = play;
    if (target.id === selected?.id && preview && videoRef.current) {
      videoRef.current.currentTime = Math.max(0, target.startSeconds - preview.clipStartSeconds);
      setPlayheadSeconds(target.startSeconds);
      if (play) void videoRef.current.play().catch(() => undefined);
      autoplayPreview.current = false;
      return;
    }
    setSelectedCandidateId(target.id);
  }

  function moveSelection(step: number) {
    if (!sortedCandidates.length) return;
    const next = clamp(selectedIndex + step, 0, sortedCandidates.length - 1);
    chooseCandidate(sortedCandidates[next].id);
  }

  /** 이미 만들어 둔 검토 프록시 안에서만 이동한다. 같은 구간을 다시 만들지 않는다. */
  function jumpToSource(targetSeconds: number, label: string) {
    const video = videoRef.current;
    if (!video || !preview) return;
    const bounded = clamp(targetSeconds, preview.sourceStartSeconds, preview.sourceEndSeconds);
    video.currentTime = Math.max(0, bounded - preview.clipStartSeconds);
    setPlayheadSeconds(bounded);
    setNotice(
      Math.abs(bounded - targetSeconds) < 0.5
        ? `${label} ${formatTime(Math.round(bounded))}로 이동했습니다.`
        : `${label}가 준비된 재생 구간을 벗어나 ${formatTime(Math.round(bounded))}로 이동했습니다.`
    );
  }

  async function copyTimecode() {
    if (!selected) return;
    const value = `${formatTime(selected.startSeconds)} — ${formatTime(selected.endSeconds)}`;
    try {
      await navigator.clipboard.writeText(value);
    } catch {
      const input = document.createElement("textarea");
      input.value = value;
      input.style.position = "fixed";
      input.style.opacity = "0";
      document.body.appendChild(input);
      input.select();
      document.execCommand("copy");
      input.remove();
    }
    setNotice(`타임코드 ${value}를 복사했습니다.`);
  }

  async function exportCsv() {
    if (!job) return;
    setActionBusy(true);
    setUiError(null);
    try {
      const path = await saveCandidatesCsv(job.id);
      if (path) setNotice(`CSV를 저장했습니다: ${path}`);
    } catch (error) {
      setUiError(messageFrom(error));
    } finally {
      setActionBusy(false);
    }
  }

  async function removeCurrentJob() {
    if (!job || active || previewLoading) return;
    const size = formatBytes(storageBytes ?? 0);
    if (!await confirmJobDeletion(size)) return;
    setActionBusy(true);
    setUiError(null);
    try {
      await deleteJob(job.id);
      setJob(null);
      setNewJobMode(true);
      setStorageBytes(null);
      setPreview(null);
      setPreviewUrl(null);
      setNotice("작업과 작업 폴더를 삭제했습니다.");
    } catch (error) {
      setUiError(messageFrom(error));
    } finally {
      setActionBusy(false);
    }
  }

  async function refreshStoredJobs() {
    try {
      setStoredJobs(await listJobs());
    } catch (error) {
      setUiError(messageFrom(error));
    }
  }

  async function openSettings() {
    setSettingsOpen(true);
    await refreshStoredJobs();
  }

  async function removeStoredJob(jobId: string) {
    if (active || !window.confirm("선택한 작업 폴더와 저장된 영상·음성 인식 결과를 삭제할까요?")) return;
    try {
      await deleteStoredJob(jobId);
      if (job?.id === jobId) {
        setJob(null);
        setNewJobMode(true);
      }
      await refreshStoredJobs();
    } catch (error) {
      setUiError(messageFrom(error));
    }
  }

  async function removeAllStoredJobs() {
    if (active || !window.confirm("저장된 모든 작업과 다운로드 영상·음성 인식 결과를 삭제할까요? 이 작업은 되돌릴 수 없습니다.")) return;
    try {
      await deleteAllJobs();
      setJob(null);
      setNewJobMode(true);
      setStoredJobs([]);
      setNotice("저장된 모든 작업을 삭제했습니다.");
    } catch (error) {
      setUiError(messageFrom(error));
    }
  }

  async function refreshUpdate(manual: boolean) {
    if (!isDesktopRuntime || updateChecking || updateInstalling) return;
    setUpdateChecking(true);
    setUpdateError(null);
    try {
      const info = await checkForAppUpdate();
      setUpdateInfo(info);
      setUpdateCheckedAt(new Date().toISOString());
      if (manual && !info.available) setNotice("현재 최신 안정 버전을 사용 중입니다.");
      if (info.available) setSettingsOpen(true);
    } catch (error) {
      // 업데이트 연결 실패는 분석 실패가 아니다. 작업 화면이 아니라 설정 화면에서만 알린다.
      setUpdateError(messageFrom(error));
      setUpdateCheckedAt(new Date().toISOString());
    } finally {
      setUpdateChecking(false);
    }
  }

  async function installUpdate() {
    if (active) {
      setUpdateInstallError("분석 작업을 먼저 완료하거나 안전하게 취소한 뒤 업데이트해 주세요.");
      return;
    }
    setUpdateInstalling(true);
    setUpdateInstallError(null);
    try {
      await installPendingUpdate(setUpdateProgress);
    } catch (error) {
      setUpdateInstallError(`업데이트 설치에 실패했습니다. 기존 버전은 그대로 유지됩니다. ${messageFrom(error)}`);
      setUpdateInstalling(false);
    }
  }

  async function pickLocalFile() {
    setSourceKind("local");
    if (isDesktopRuntime) {
      const path = await chooseLocalVideo();
      if (path) setSourceLabel(path);
    } else {
      browserFileInput.current?.click();
    }
  }

  function changeSource(kind: SourceKind) {
    setSourceKind(kind);
    if (kind === "demo") setSourceLabel("8시간 샘플 방송 · fixture");
    if (kind === "local") setSourceLabel("");
    if (kind === "youtube") setSourceLabel("https://www.youtube.com/watch?v=");
  }

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const target = event.target as HTMLElement;
      const typing = ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
      if (event.ctrlKey && event.key.toLowerCase() === "o") {
        event.preventDefault();
        void pickLocalFile();
        return;
      }
      if (event.ctrlKey && event.key === "Enter") {
        event.preventDefault();
        void runPrimary();
        return;
      }
      if (typing) return;
      if (event.key === "Escape" && active) void requestCancel();
      if (event.key.toLowerCase() === "r" && resumable) void runPrimary();
      if (event.key.toLowerCase() === "j") moveSelection(1);
      if (event.key.toLowerCase() === "k") moveSelection(-1);
      if (event.key.toLowerCase() === "a" && selected) void decide("ACCEPTED");
      if (event.key.toLowerCase() === "x" && selected) void decide("REJECTED");
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  if (loading) {
    return (
      <main className="loading-screen">
        <div className="brand-mark"><Waves size={24} /></div>
        <p>작업 기록을 불러오는 중…</p>
      </main>
    );
  }

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand-lockup">
          <div className="brand-mark" aria-hidden="true"><Waves size={21} /></div>
          <div>
            <strong>VOD SCOUT</strong>
            <span>LOCAL EDITING AGENT</span>
          </div>
        </div>
        <div className="verification-banner">
          <Bot size={15} />
          <span><strong>오프라인 로컬 분석</strong> · 영상과 음성 인식 결과는 이 PC 밖으로 전송되지 않습니다.</span>
        </div>
        <div className="runtime-state">
          <span className="runtime-dot" aria-hidden="true" />
          {isDesktopRuntime ? "Windows 로컬" : "브라우저 미리보기"}
          <button
            type="button"
            className="settings-entry"
            onClick={() => void openSettings()}
            title="설정·업데이트 내역 열기"
            aria-label="설정·업데이트 열기"
          >
            <Settings2 size={14} aria-hidden="true" />
            <span className="settings-entry-label">설정</span>
            <span className="settings-entry-version">v{runtime?.appVersion ?? "0.1.0"}</span>
          </button>
        </div>
      </header>

      <aside className="stage-sidebar">
        <section className="job-identity">
          <span className="eyebrow">CURRENT JOB</span>
          {job && !newJobMode ? (
            <>
              <div className="source-icon">{job.sourceKind === "youtube" ? <Tv /> : job.sourceKind === "local" ? <FileVideo2 /> : <Radio />}</div>
              <strong title={job.sourceLabel}>{shortSource(job.sourceLabel, 34)}</strong>
              <span className="job-id">#{job.id.slice(0, 8)}</span>
            </>
          ) : (
            <>
              <div className="source-icon empty"><FolderOpen /></div>
              <strong>새 작업</strong>
              <span>입력을 선택해 주세요.</span>
            </>
          )}
        </section>

        <nav className="pipeline-nav" aria-label="분석 단계">
          <span className="eyebrow">PIPELINE</span>
          {PIPELINE.map((step, index) => {
            const Icon = step.icon;
            const currentRank = job ? PIPELINE_STATUS_ORDER.indexOf(job.status) : -1;
            const stepRanks = step.statuses.map((status) => PIPELINE_STATUS_ORDER.indexOf(status));
            const current = !!job && active && step.statuses.some((status) => status === job.status);
            const finished = !!job && (job.status === "REVIEW_READY" || (currentRank >= 0 && currentRank > Math.max(...stepRanks)));
            return (
              <div className={`pipeline-step ${finished ? "finished" : ""} ${current ? "current" : ""}`} key={step.label}>
                <span className="step-node">{finished ? <Check size={13} /> : <Icon size={15} />}</span>
                <span>{step.label}</span>
                {current ? <ChevronRight size={14} /> : null}
                {index < PIPELINE.length - 1 ? <span className="step-line" /> : null}
              </div>
            );
          })}
        </nav>

        <section className="shortcut-legend">
          <span className="eyebrow">SHORTCUTS</span>
          <div><kbd>Ctrl</kbd><kbd>↵</kbd><span>시작</span></div>
          <div><kbd>J</kbd><kbd>K</kbd><span>후보 이동</span></div>
          <div><kbd>A</kbd><kbd>X</kbd><span>채택 / 제외</span></div>
        </section>
      </aside>

      <main className="workspace">
        {uiError ? (
          <div className="inline-alert" role="alert">
            <AlertTriangle size={18} />
            <span>{uiError}</span>
            <button aria-label="오류 닫기" onClick={() => setUiError(null)}><X size={16} /></button>
          </div>
        ) : null}
        {notice ? (
          <div className="inline-notice" role="status">
            <CheckCircle2 size={17} />
            <span>{notice}</span>
            <button aria-label="알림 닫기" onClick={() => setNotice(null)}><X size={15} /></button>
          </div>
        ) : null}

        {newJobMode || !job ? (
          <section className="source-workspace" aria-labelledby="source-title">
            <div className="section-heading wide-heading">
              <div>
                <span className="eyebrow">NEW ANALYSIS</span>
                <h1 id="source-title">어떤 방송을 살펴볼까요?</h1>
                <p>로컬 영상을 10분 단위로 나눠 음성 인식하고, 오디오 반응과 대화 밀도로 쇼츠 후보를 찾습니다.</p>
              </div>
              <span className="step-count">01 / 03</span>
            </div>

            <div className="source-tabs" role="tablist" aria-label="입력 방식">
              {(Object.keys(SOURCE_META) as SourceKind[]).map((kind) => {
                const Icon = SOURCE_META[kind].icon;
                return (
                  <button
                    key={kind}
                    role="tab"
                    aria-selected={sourceKind === kind}
                    className={sourceKind === kind ? "selected" : ""}
                    onClick={() => changeSource(kind)}
                  >
                    <Icon size={19} />
                    <span><strong>{SOURCE_META[kind].label}</strong><small>{SOURCE_META[kind].hint}</small></span>
                  </button>
                );
              })}
            </div>

            <div className="source-entry">
              <label htmlFor="source-value">{sourceKind === "youtube" ? "YouTube 영상 주소" : sourceKind === "local" ? "영상 파일" : "데모 작업 이름"}</label>
              <div className="source-input-row">
                <input
                  id="source-value"
                  value={sourceLabel}
                  readOnly={sourceKind === "local"}
                  onChange={(event) => setSourceLabel(event.target.value)}
                  placeholder={sourceKind === "youtube" ? "https://www.youtube.com/watch?v=…" : "입력을 선택하세요"}
                />
                {sourceKind === "local" ? (
                  <button className="button secondary" onClick={() => void pickLocalFile()}><FolderOpen size={17} /> 파일 선택</button>
                ) : null}
              </div>
              <input
                ref={browserFileInput}
                className="visually-hidden"
                type="file"
                accept="video/*"
                onChange={(event) => setSourceLabel(event.target.files?.[0]?.name ?? "")}
              />
              <p className="field-note"><AlertTriangle size={14} /> {sourceKind === "local" ? "FFmpeg·Whisper base가 로컬에서 실행됩니다. 긴 영상은 10분 청크마다 저장됩니다." : sourceKind === "youtube" ? "공개된 단일 영상만 지원합니다. yt-dlp로 최대 720p까지 내려받은 뒤 FFmpeg·Whisper를 PC 안에서 실행합니다." : "데모는 실제 영상을 읽지 않고 취소·실패·재개 흐름을 검증합니다."}</p>
            </div>

            {sourceKind !== "demo" ? <section className="analysis-mode-panel" aria-label="분석 방식">
              <div className="panel-heading"><span><Gauge size={17} /> 분석 방식</span><strong>{ANALYSIS_MODES.find((item) => item.value === analysisMode)?.label}</strong></div>
              <div className="analysis-mode-options">
                {ANALYSIS_MODES.map((item) => (
                  <button key={item.value} className={analysisMode === item.value ? "selected" : ""} onClick={() => setAnalysisMode(item.value)}>
                    <strong>{item.label}</strong><small>{item.detail}</small>
                  </button>
                ))}
              </div>
              {analysisMode === "range" ? <div className="range-fields">
                <label>시작<input value={rangeStart} onChange={(event) => setRangeStart(event.target.value)} placeholder="00:00:00" /></label>
                <span>→</span>
                <label>종료<input value={rangeEnd} onChange={(event) => setRangeEnd(event.target.value)} placeholder="01:00:00" /></label>
              </div> : null}
            </section> : null}

            <section className="analysis-mode-panel whisper-settings-panel" aria-label="음성 인식 설정">
              <div className="panel-heading"><span><Mic2 size={17} /> 음성 인식 장치</span><strong>{WHISPER_DEVICES.find((item) => item.value === whisperDevice)?.label}</strong></div>
              <div className="analysis-mode-options">
                {WHISPER_DEVICES.map((item) => (
                  <button key={item.value} type="button" className={whisperDevice === item.value ? "selected" : ""} onClick={() => setWhisperDevice(item.value)}>
                    <strong>{item.label}</strong><small>{item.detail}</small>
                  </button>
                ))}
              </div>
              <div className="analysis-mode-options">
                {WHISPER_PROFILES.map((item) => (
                  <button key={item.value} type="button" className={whisperProfile === item.value ? "selected" : ""} onClick={() => setWhisperProfile(item.value)}>
                    <strong>{item.label}</strong><small>{item.detail}</small>
                  </button>
                ))}
              </div>
              <label className="cpu-thread-control">CPU 사용량
                <select value={cpuThreads} onChange={(event) => setCpuThreads(event.target.value)}>
                  <option value="auto">자동</option>
                  {[1, 2, 4, 8, 16, 32].map((value) => <option key={value} value={value}>{value}개 스레드</option>)}
                </select>
              </label>
            </section>

            {sourceKind === "demo" ? <details className="scenario-panel">
              <summary><TerminalSquare size={16} /> 복구 시나리오 선택 <span>{SCENARIOS.find((item) => item.value === scenario)?.label}</span></summary>
              <div className="scenario-options">
                {SCENARIOS.map((item) => (
                  <label key={item.value} className={scenario === item.value ? "selected" : ""}>
                    <input type="radio" name="scenario" value={item.value} checked={scenario === item.value} onChange={() => setScenario(item.value)} />
                    <span><strong>{item.label}</strong><small>{item.detail}</small></span>
                  </label>
                ))}
              </div>
            </details> : null}

            <div className="source-actions">
              {job ? <button className="button ghost" onClick={() => setNewJobMode(false)}>기존 작업으로 돌아가기</button> : <span />}
              <button className="button primary" disabled={actionBusy || !sourceLabel.trim()} onClick={() => void runPrimary()}>
                <Play size={17} fill="currentColor" /> {primaryLabel}
              </button>
            </div>
          </section>
        ) : (
          <>
            <section className="job-header">
              <div>
                <div className="job-title-row">
                  <StatusPill status={job.status} />
                  <span className="mode-tag">{job.sourceKind === "local" ? `LOCAL · ${job.analysisMode.toUpperCase()}` : job.sourceKind === "youtube" ? `YOUTUBE · 720P · ${job.analysisMode.toUpperCase()}` : `FIXTURE · ${job.scenario.toUpperCase()}`}</span>
                </div>
                <h1>{job.status === "REVIEW_READY" ? "편집 후보를 검토하세요" : job.currentStageLabel}</h1>
                <p>{job.status === "REVIEW_READY" ? `후보 ${job.candidates.length}개 중 ${reviewedCount}개를 판정했습니다.` : `${job.completedUnits} / ${job.totalUnits} 체크포인트 완료${job.mediaDurationSeconds ? ` · 원본 ${formatTime(Math.round(job.mediaDurationSeconds))}` : ""}${active && timing ? ` · 경과 ${formatDuration(timing.elapsedSeconds)}${timing.remainingSeconds === null ? " · 남은 시간 계산 중" : ` · 약 ${formatDuration(timing.remainingSeconds)} 남음`}` : ""}`}</p>
                {job.sourceKind === "youtube" && captionSummaryLabel(job.captions) ? <small className="caption-summary">{captionSummaryLabel(job.captions)}</small> : null}
                {job.sourceKind === "youtube" && job.captions ? <CaptionDetails captions={job.captions} /> : null}
                <small className="whisper-status">음성 인식: {WHISPER_DEVICES.find((item) => item.value === (job.whisper?.deviceMode ?? "auto"))?.label} · {WHISPER_PROFILES.find((item) => item.value === (job.whisper?.profile ?? "balanced"))?.label} · CPU {job.whisper?.cpuThreads == null ? "자동" : `${job.whisper.cpuThreads}개 스레드`}</small>
              </div>
              <div className="job-actions">
                {storageBytes !== null ? <span className="storage-label"><HardDrive size={14} /> {formatBytes(storageBytes)}</span> : null}
                {job.status === "REVIEW_READY" ? <button className="button ghost" disabled={actionBusy} onClick={() => void exportCsv()}><Download size={15} /> CSV</button> : null}
                {!active ? <button className="button ghost" onClick={() => setNewJobMode(true)}><Square size={15} /> 새 작업</button> : null}
                {!active ? <button className="button danger" disabled={actionBusy || previewLoading} onClick={() => void removeCurrentJob()}><Trash2 size={15} /> 작업 삭제</button> : null}
                {active ? (
                  <button className="button danger" disabled={job.status === "CANCELLING" || actionBusy} onClick={() => void requestCancel()}>
                    <Pause size={16} /> {job.status === "CANCELLING" ? "취소 중…" : "안전하게 취소"}
                  </button>
                ) : job.status !== "REVIEW_READY" ? (
                  <button className="button primary" disabled={actionBusy} onClick={() => void runPrimary()}>
                    {resumable ? <RotateCcw size={16} /> : <Play size={16} fill="currentColor" />} {primaryLabel}
                  </button>
                ) : null}
              </div>
            </section>

            {job.errorMessage ? (
              <section className="error-panel" role="alert">
                <div className="error-symbol"><AlertTriangle /></div>
                <div>
                  <span className="eyebrow">RECOVERABLE STATE</span>
                  <h2>{job.errorMessage}</h2>
                  <p>완료된 {job.completedUnits}단위는 저장돼 있습니다. 같은 작업을 이어서 실행할 수 있습니다.</p>
                  {job.errorDetail ? <details><summary>진단 상세</summary><code>{job.errorDetail}</code></details> : null}
                </div>
              </section>
            ) : null}

            {job.status === "REVIEW_READY" && selected ? (
              <section className="review-layout">
                <div className="candidate-list">
                  <div className="panel-heading">
                    <span><ListChecks size={17} /> 후보 큐</span>
                    <strong>{job.candidates.length}</strong>
                  </div>
                  <div className="candidate-sort">
                    <label htmlFor="candidate-sort">정렬</label>
                    <select
                      id="candidate-sort"
                      value={settings.sortKey}
                      onChange={(event) => setSettings((current) => ({ ...current, sortKey: event.target.value as CandidateSortKey }))}
                    >
                      {CANDIDATE_SORTS.map((option) => (
                        <option key={option.value} value={option.value}>{option.label}</option>
                      ))}
                    </select>
                  </div>
                  <p className="candidate-sort-note" role="status">
                    보이는 순서만 바꿉니다. 점수와 채택·보류·제외 상태는 그대로입니다.
                  </p>
                  <ul className="candidate-rows" aria-label="편집 후보 목록">
                    {sortedCandidates.map((candidate, index) => (
                      <li key={candidate.id}>
                        <button
                          className={`candidate-row ${candidate.id === selected.id ? "selected" : ""}`}
                          disabled={previewLoading}
                          aria-current={candidate.id === selected.id ? "true" : undefined}
                          onClick={() => chooseCandidate(candidate.id, true)}
                        >
                          <span className="candidate-rank">{String(index + 1).padStart(2, "0")}</span>
                          <span className="candidate-copy">
                            <span className="candidate-time">{formatTime(candidate.startSeconds)} — {formatTime(candidate.endSeconds)}</span>
                            <strong>{candidate.title}</strong>
                            <small>{candidate.summary}</small>
                          </span>
                          <span className="candidate-score">{candidate.totalScore}</span>
                          <DecisionMark decision={candidate.decision} />
                        </button>
                      </li>
                    ))}
                  </ul>
                </div>

                <article className="candidate-detail">
                  <div className="video-preview">
                    <div className="preview-topline"><span>SOURCE TIMECODE · {formatTime(Math.round(playheadSeconds || selected.startSeconds))}</span><span>{isDesktopRuntime ? "로컬 원본 구간" : "브라우저 미리보기"}</span></div>
                    {previewUrl ? (
                      <video
                        ref={videoRef}
                        src={previewUrl}
                        controls
                        preload="metadata"
                        onLoadedMetadata={(event) => {
                          if (!preview) return;
                          event.currentTarget.currentTime = Math.max(0, selected.startSeconds - preview.clipStartSeconds);
                          if (autoplayPreview.current) void event.currentTarget.play().catch(() => undefined);
                          autoplayPreview.current = false;
                        }}
                        onTimeUpdate={(event) => {
                          if (preview) setPlayheadSeconds(preview.clipStartSeconds + event.currentTarget.currentTime);
                        }}
                        onError={() => setPreviewError("이 후보 영상을 재생하지 못했습니다. 다른 후보를 선택하거나 작업을 다시 열어 주세요.")}
                      />
                    ) : (
                      <div className="preview-center" role="status">
                        <span className="play-disc">{previewLoading ? <Gauge size={25} className="pulse-icon" /> : <Play size={25} fill="currentColor" />}</span>
                        <strong>{formatTime(selected.startSeconds)}</strong>
                        <small>{previewLoading ? "원본에서 재생 구간을 준비하고 있습니다…" : previewError ?? "데스크톱 앱에서 원본 구간을 재생할 수 있습니다."}</small>
                      </div>
                    )}
                  </div>
                  <div className="detail-content">
                    <div className="detail-title">
                      <div>
                        <span className="eyebrow">CANDIDATE {String(selectedIndex + 1).padStart(2, "0")}</span>
                        <h2>{selected.title}</h2>
                      </div>
                      <span className="total-score"><small>TOTAL</small>{selected.totalScore}</span>
                    </div>
                    <blockquote>“{selected.transcriptExcerpt}”</blockquote>
                    <SignalRail candidate={selected} />
                    {context ? (
                      <section className="context-panel" aria-labelledby="context-title">
                        <div className="context-heading">
                          <h3 id="context-title">앞뒤 맥락</h3>
                          <span className="context-range">
                            {formatTime(Math.round(context.startSeconds))} — {formatTime(Math.round(context.endSeconds))}
                          </span>
                        </div>
                        <div className="context-jumps" role="group" aria-label="원본 이동">
                          <button
                            className="button ghost compact"
                            disabled={!preview}
                            onClick={() => jumpToSource(selected.startSeconds, "후보 시작")}
                          >
                            <Play size={14} fill="currentColor" /> 후보 시작
                          </button>
                          <button
                            className="button ghost compact"
                            disabled={!preview}
                            onClick={() => jumpToSource(context.startSeconds, "맥락 시작")}
                          >
                            <ChevronLeft size={14} /> 맥락 시작
                          </button>
                          <button
                            className="button ghost compact"
                            disabled={!preview}
                            onClick={() => jumpToSource(context.endSeconds, "맥락 끝")}
                          >
                            <ChevronRight size={14} /> 맥락 끝
                          </button>
                        </div>
                        {context.lines.length ? (
                          <ol className="context-lines">
                            {context.lines.map((line) => {
                              const inside = line.startSeconds >= selected.startSeconds && line.startSeconds < selected.endSeconds;
                              return (
                                <li className={inside ? "inside" : "outside"} key={`${line.startSeconds}-${line.text}`}>
                                  <button
                                    className="context-jump-time"
                                    disabled={!preview}
                                    onClick={() => jumpToSource(line.startSeconds, "선택한 문장")}
                                  >
                                    {formatTime(Math.round(line.startSeconds))}
                                  </button>
                                  <span>{inside ? <span className="context-flag">후보 구간</span> : null}{line.text}</span>
                                </li>
                              );
                            })}
                          </ol>
                        ) : (
                          <p className="context-empty">
                            이 작업에는 앞뒤 음성 인식 문장이 저장돼 있지 않습니다. 위 이동 버튼으로 원본 앞뒤를 직접 확인할 수 있습니다.
                          </p>
                        )}
                        {!preview ? (
                          <p className="context-empty">데스크톱 앱에서 원본 구간을 열면 이동 버튼을 사용할 수 있습니다.</p>
                        ) : null}
                      </section>
                    ) : null}
                    <div className="candidate-tools">
                      <button className="button ghost compact" onClick={() => void copyTimecode()}><Copy size={15} /> 타임코드 복사</button>
                      <button className="button ghost compact" disabled={actionBusy} onClick={() => void exportCsv()}><Download size={15} /> CSV 내보내기</button>
                    </div>
                    <div className="review-actions">
                      <button className={`button reject ${selected.decision === "REJECTED" ? "selected" : ""}`} onClick={() => void decide("REJECTED")}><X size={17} /> 제외 <kbd>X</kbd></button>
                      <button className="button ghost compact" onClick={() => void decide("PENDING")}><RefreshCw size={15} /> 보류</button>
                      <button className={`button accept ${selected.decision === "ACCEPTED" ? "selected" : ""}`} onClick={() => void decide("ACCEPTED")}><Check size={17} /> 채택 <kbd>A</kbd></button>
                    </div>
                  </div>
                </article>
              </section>
            ) : (
              <section className="run-dashboard">
                <article className="progress-board">
                  <div className="progress-copy">
                    <span className="eyebrow">AGENT PROGRESS</span>
                    <strong>{percent}<small>%</small></strong>
                    <p>{job.status === "CANCELLED" ? "체크포인트가 보존됐습니다." : job.status === "INTERRUPTED" ? "마지막 완료 지점에서 멈췄습니다." : "worker가 보고한 완료 단위만 표시합니다."}</p>
                    {active && timing ? <div className="timing-line"><Timer size={15} /><span>경과 <b>{formatDuration(timing.elapsedSeconds)}</b></span><span>{timing.remainingSeconds === null ? "남은 시간 계산 중" : `약 ${formatDuration(timing.remainingSeconds)} 남음`}</span>{timing.expectedAt ? <span>예상 완료 {timing.expectedAt.toLocaleTimeString("ko-KR", { hour: "2-digit", minute: "2-digit" })}</span> : null}</div> : null}
                  </div>
                  <div className="progress-track" role="progressbar" aria-label="작업 진행률" aria-valuenow={percent} aria-valuemin={0} aria-valuemax={100}>
                    {Array.from({ length: 20 }).map((_, index) => (
                      <span key={index} className={index < Math.floor(percent / 5) ? "done" : index === Math.floor(percent / 5) && active ? "current" : ""} />
                    ))}
                  </div>
                  <div className="agent-status-line">
                    <span className="agent-orb"><Bot size={20} /></span>
                    <div><strong>{job.currentStageLabel}</strong><small>{active ? (job.sourceKind === "youtube" && job.status === "ACQUIRING" ? "yt-dlp 다운로드 중" : "로컬 worker 실행 중") : "worker 정지"}</small></div>
                    {active ? <Gauge size={18} className="pulse-icon" /> : <Pause size={18} />}
                  </div>
                </article>

                <article className="checkpoint-board">
                  <div className="panel-heading"><span><CheckCircle2 size={17} /> 저장된 체크포인트</span><span className="safe-label">LOCAL</span></div>
                  <strong>{job.completedUnits}<small> / {job.totalUnits} units</small></strong>
                  <p>{job.sourceKind === "local" ? "각 10분 단위의 음성 인식 결과와 오디오 신호를 저장합니다. 취소하거나 앱을 닫아도 완료된 청크 다음부터 이어집니다." : job.sourceKind === "youtube" ? "다운로드 임시 파일과 완료 영상, 10분 단위 음성 인식 결과를 저장합니다. 취소하거나 앱을 닫아도 가능한 지점부터 이어집니다." : "각 단위가 끝날 때 작업 상태를 원자적으로 저장합니다. 실패하거나 앱을 닫아도 완료 지점 다음부터 이어집니다."}</p>
                  <div className="checkpoint-meta">
                    <span><Clock3 size={14} /> {new Date(job.updatedAt).toLocaleTimeString("ko-KR", { hour: "2-digit", minute: "2-digit", second: "2-digit" })}</span>
                    <span><TerminalSquare size={14} /> {job.scenario}</span>
                  </div>
                </article>

                <article className="signal-preview">
                  <div className="panel-heading"><span><Activity size={17} /> Signal Rail 준비 상태</span><span>{audioSignalsReady ? (chatSignalsReady ? "3 / 3" : "2 / 3") : "0 / 3"}</span></div>
                  <div className={`preview-signal ${audioSignalsReady ? "ready" : ""}`}><span>오디오 반응</span><i /><strong>{audioSignalsReady ? "READY" : "WAIT"}</strong></div>
                  <div className={`preview-signal ${audioSignalsReady ? "ready" : ""}`}><span>대화 밀도</span><i /><strong>{audioSignalsReady ? "READY" : "WAIT"}</strong></div>
                  <div className={`preview-signal ${chatSignalsReady ? "ready" : ""}`}><span>채팅 움직임</span><i /><strong>{chatSignalsReady ? "READY" : "WAIT"}</strong></div>
                </article>
              </section>
            )}
          </>
        )}
      </main>

      <aside className="activity-panel">
        <div className="activity-heading">
          <div><span className="eyebrow">AGENT ACTIVITY</span><h2>실행 기록</h2></div>
          {active ? <span className="listening"><span /> LIVE</span> : null}
        </div>
        <div className="activity-list" aria-live="polite">
          {job?.activity.length ? [...job.activity].reverse().map((event, index) => (
            <div className={`activity-item ${index === 0 ? "latest" : ""}`} key={event.sequence}>
              <span className={`activity-node ${event.kind}`} />
              <div>
                <span>{new Date(event.timestamp).toLocaleTimeString("ko-KR", { hour: "2-digit", minute: "2-digit", second: "2-digit" })}</span>
                <p>{event.message}</p>
              </div>
            </div>
          )) : (
            <div className="activity-empty"><MessageSquareText size={28} /><p>작업을 시작하면 agent의 판단과 체크포인트가 여기에 쌓입니다.</p></div>
          )}
        </div>
        <details className="runtime-details">
          <summary>런타임 정보</summary>
          <dl>
            <div><dt>Worker</dt><dd>{runtime?.workerSource}</dd></div>
            <div><dt>Mode</dt><dd>{runtime?.analysisMode}</dd></div>
            <div><dt>Data</dt><dd title={runtime?.dataDirectory}>{shortSource(runtime?.dataDirectory ?? "-", 34)}</dd></div>
          </dl>
        </details>
      </aside>

      {settingsOpen ? <div className="settings-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setSettingsOpen(false); }}>
        <section className="settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title">
          <header>
            <div><span className="eyebrow">VOD SCOUT</span><h2 id="settings-title">설정·업데이트</h2></div>
            <button className="icon-button" aria-label="닫기" onClick={() => setSettingsOpen(false)}><X size={18} /></button>
          </header>

          <div className="settings-section">
            <div className="settings-title-row">
              <div><h3>화면 밝기</h3><p>기본값은 Windows의 밝은 화면·어두운 화면 설정을 따릅니다.</p></div>
            </div>
            <div className="theme-options" role="radiogroup" aria-label="화면 밝기">
              {THEME_OPTIONS.map((option) => (
                <button
                  key={option.value}
                  role="radio"
                  aria-checked={settings.theme === option.value}
                  className={settings.theme === option.value ? "selected" : ""}
                  onClick={() => setSettings((current) => ({ ...current, theme: option.value }))}
                >
                  {settings.theme === option.value ? <Check size={14} /> : <Circle size={10} />}
                  {option.label}
                </button>
              ))}
            </div>
            <p className="quiet-copy">
              지금 적용된 화면: {resolvedTheme === "dark" ? "어둡게" : "밝게"}
              {settings.theme === "system" ? " · 시스템 설정을 따르는 중" : ""}
            </p>
          </div>

          <div className="settings-section">
            <div className="settings-title-row"><div><h3>자동 업데이트</h3><p>GitHub Releases의 서명된 안정 버전만 설치합니다.</p></div><button className="button ghost compact" disabled={updateChecking || updateInstalling} onClick={() => void refreshUpdate(true)}><RefreshCw size={14} /> {updateChecking ? "확인 중…" : "업데이트 확인"}</button></div>
            <div className={`update-status ${updateStatus.kind}`} role="status">
              <span className="update-status-label">
                {updateStatus.kind === "error" ? <AlertTriangle size={14} /> : updateStatus.kind === "current" ? <CheckCircle2 size={14} /> : updateStatus.kind === "checking" ? <RefreshCw size={14} /> : <Download size={14} />}
                {updateStatus.label}
              </span>
              <p>{updateStatus.detail}</p>
              <dl>
                <div><dt>현재 버전</dt><dd>v{runtime?.appVersion ?? "-"}</dd></div>
                <div><dt>마지막 확인</dt><dd>{updateCheckedAt ? new Date(updateCheckedAt).toLocaleString("ko-KR") : "아직 확인하지 않음"}</dd></div>
              </dl>
            </div>
            {updateInfo?.available ? <div className="update-available">
              <strong>v{updateInfo.version} 사용 가능</strong>
              <pre>{updateInfo.notes}</pre>
              <button className="button primary" disabled={active || updateInstalling} onClick={() => void installUpdate()}><Download size={16} /> {updateInstalling ? (updateProgress === null ? "다운로드 중…" : `업데이트 ${updateProgress}%`) : active ? "분석 종료 후 업데이트" : "지금 업데이트"}</button>
            </div> : null}
            {updateError ? <p className="settings-error">연결 오류 상세: {updateError}</p> : null}
            {updateInstallError ? <p className="settings-error">{updateInstallError}</p> : null}
          </div>

          <div className="settings-section release-notes">
            <h3>v{CURRENT_RELEASE_NOTES.version} 업데이트 내역</h3>
            {([
              ["기능 추가", CURRENT_RELEASE_NOTES.added],
              ["변경", CURRENT_RELEASE_NOTES.changed],
              ["버그 수정", CURRENT_RELEASE_NOTES.fixed],
              ["보안", CURRENT_RELEASE_NOTES.security],
              ["알려진 문제", CURRENT_RELEASE_NOTES.knownIssues]
            ] as const).map(([label, items]) => <div key={label}><strong>{label}</strong><ul>{items.map((item) => <li key={item}>{item}</li>)}</ul></div>)}
          </div>

          <div className="settings-section">
            <div className="settings-title-row"><div><h3>저장 공간 정리</h3><p>현재 PC에 저장된 다운로드 영상·음성 인식 결과·미리보기입니다.</p></div>{storedJobs.length ? <button className="button danger compact" disabled={active} onClick={() => void removeAllStoredJobs()}><Trash2 size={14} /> 전체 삭제</button> : null}</div>
            <div className="stored-job-list">
              {storedJobs.length ? storedJobs.map((item) => <div key={item.snapshot.id}><span><strong>{shortSource(item.snapshot.sourceLabel, 42)}</strong><small>{new Date(item.snapshot.updatedAt).toLocaleString("ko-KR")} · {formatBytes(item.sizeBytes)}</small></span><button className="icon-button danger-icon" disabled={active} aria-label="작업 삭제" onClick={() => void removeStoredJob(item.snapshot.id)}><Trash2 size={15} /></button></div>) : <p className="quiet-copy">저장된 작업이 없습니다.</p>}
            </div>
          </div>
        </section>
      </div> : null}
    </div>
  );
}

export default App;
