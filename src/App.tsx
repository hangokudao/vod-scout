import { useEffect, useMemo, useRef, useState } from "react";
import {
  Activity,
  AlertTriangle,
  Bot,
  Check,
  CheckCircle2,
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
  prepareCandidatePreview,
  previewMediaUrl,
  saveCandidatesCsv,
  setCandidateDecision,
  startJob,
  subscribeToJob
} from "./api";
import type {
  Candidate,
  CandidateDecision,
  AnalysisMode,
  JobSnapshot,
  JobStatus,
  RuntimeInfo,
  PreviewMedia,
  Scenario,
  SourceKind,
  StoredJobInfo
} from "./types";
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
  { label: "오디오·전사", statuses: ["EXTRACTING_AUDIO", "TRANSCRIBING"], icon: Mic2 },
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
  { value: "quick", label: "빠른 분석", detail: "전체를 훑고 최대 120분만 분산 전사" },
  { value: "range", label: "구간 지정", detail: "선택한 시작·종료 범위만 정밀 분석" },
  { value: "full", label: "전체 정밀 분석", detail: "전체 오디오를 10분 청크로 전사" }
];

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
  const [selectedCandidate, setSelectedCandidate] = useState(0);
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
    if (!job?.candidates.length) return;
    setSelectedCandidate((current) => Math.min(current, job.candidates.length - 1));
  }, [job?.candidates.length]);

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
  const selected = job?.candidates[selectedCandidate] ?? null;
  const checkpointPercent = job?.totalUnits ? Math.round((job.completedUnits / job.totalUnits) * 100) : 0;
  const percent = job?.sourceKind === "youtube" && job.status === "ACQUIRING" && job.completedUnits === 0
    ? (job.downloadPercent ?? 0)
    : checkpointPercent;
  const reviewedCount = job?.candidates.filter((candidate) => candidate.decision !== "PENDING").length ?? 0;
  const timing = job ? estimateJobTiming(job, new Date(clock)) : null;
  const audioSignalsReady = !!job && PIPELINE_STATUS_ORDER.indexOf(job.status) >= PIPELINE_STATUS_ORDER.indexOf("AUDIO_SIGNALS");
  const chatSignalsReady = !!job && PIPELINE_STATUS_ORDER.indexOf(job.status) >= PIPELINE_STATUS_ORDER.indexOf("CHAT_SIGNALS");

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
    void prepareCandidatePreview(job.id, selected.id)
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
          analysisEndSeconds: end
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

  function chooseCandidate(index: number, play = false) {
    if (!job?.candidates[index] || previewLoading) return;
    autoplayPreview.current = play;
    if (index === selectedCandidate && preview && videoRef.current) {
      videoRef.current.currentTime = Math.max(0, job.candidates[index].startSeconds - preview.clipStartSeconds);
      setPlayheadSeconds(job.candidates[index].startSeconds);
      if (play) void videoRef.current.play().catch(() => undefined);
      autoplayPreview.current = false;
      return;
    }
    setSelectedCandidate(index);
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
    if (active || !window.confirm("선택한 작업 폴더와 저장된 영상·전사를 삭제할까요?")) return;
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
    if (active || !window.confirm("저장된 모든 작업과 다운로드 영상·전사를 삭제할까요? 이 작업은 되돌릴 수 없습니다.")) return;
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
      if (manual && !info.available) setNotice("현재 최신 안정 버전을 사용 중입니다.");
      if (info.available) setSettingsOpen(true);
    } catch (error) {
      if (manual) setUpdateError(`업데이트 서버에 연결하지 못했습니다. 기존 오프라인 기능은 계속 사용할 수 있습니다. ${messageFrom(error)}`);
    } finally {
      setUpdateChecking(false);
    }
  }

  async function installUpdate() {
    if (active) {
      setUpdateError("분석 작업을 먼저 완료하거나 안전하게 취소한 뒤 업데이트해 주세요.");
      return;
    }
    setUpdateInstalling(true);
    setUpdateError(null);
    try {
      await installPendingUpdate(setUpdateProgress);
    } catch (error) {
      setUpdateError(`업데이트 설치에 실패했습니다. 기존 버전은 유지됩니다. ${messageFrom(error)}`);
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
      if (event.key.toLowerCase() === "j" && job?.candidates.length) {
        chooseCandidate(Math.min(selectedCandidate + 1, job.candidates.length - 1));
      }
      if (event.key.toLowerCase() === "k" && job?.candidates.length) {
        chooseCandidate(Math.max(selectedCandidate - 1, 0));
      }
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
          <span><strong>오프라인 로컬 분석</strong> · 영상과 전사는 이 PC 밖으로 전송되지 않습니다.</span>
        </div>
        <div className="runtime-state">
          <span className="runtime-dot" aria-hidden="true" />
          {isDesktopRuntime ? "Windows 로컬" : "브라우저 미리보기"}
          <button className="version version-button" onClick={() => void openSettings()} title="설정·업데이트 내역 열기">
            <Settings2 size={13} /> v{runtime?.appVersion ?? "0.1.0"}
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
                <p>로컬 영상을 10분 청크로 나눠 전사하고, 오디오 반응과 발화 밀도로 쇼츠 후보를 찾습니다.</p>
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
                <div className="candidate-list" aria-label="편집 후보 목록">
                  <div className="panel-heading">
                    <span><ListChecks size={17} /> 후보 큐</span>
                    <strong>{job.candidates.length}</strong>
                  </div>
                  {job.candidates.map((candidate, index) => (
                    <button
                      className={`candidate-row ${selectedCandidate === index ? "selected" : ""}`}
                      key={candidate.id}
                      disabled={previewLoading}
                      onClick={() => chooseCandidate(index, true)}
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
                  ))}
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
                        <span className="eyebrow">CANDIDATE {String(selectedCandidate + 1).padStart(2, "0")}</span>
                        <h2>{selected.title}</h2>
                      </div>
                      <span className="total-score"><small>TOTAL</small>{selected.totalScore}</span>
                    </div>
                    <blockquote>“{selected.transcriptExcerpt}”</blockquote>
                    <SignalRail candidate={selected} />
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
                  <p>{job.sourceKind === "local" ? "각 10분 청크의 전사와 오디오 신호를 저장합니다. 취소하거나 앱을 닫아도 완료된 청크 다음부터 이어집니다." : job.sourceKind === "youtube" ? "다운로드 임시 파일과 완료 영상, 10분 전사 청크를 저장합니다. 취소하거나 앱을 닫아도 가능한 지점부터 이어집니다." : "각 단위가 끝날 때 작업 상태를 원자적으로 저장합니다. 실패하거나 앱을 닫아도 완료 지점 다음부터 이어집니다."}</p>
                  <div className="checkpoint-meta">
                    <span><Clock3 size={14} /> {new Date(job.updatedAt).toLocaleTimeString("ko-KR", { hour: "2-digit", minute: "2-digit", second: "2-digit" })}</span>
                    <span><TerminalSquare size={14} /> {job.scenario}</span>
                  </div>
                </article>

                <article className="signal-preview">
                  <div className="panel-heading"><span><Activity size={17} /> Signal Rail 준비 상태</span><span>{audioSignalsReady ? (chatSignalsReady ? "3 / 3" : "2 / 3") : "0 / 3"}</span></div>
                  <div className={`preview-signal ${audioSignalsReady ? "ready" : ""}`}><span>오디오 반응</span><i /><strong>{audioSignalsReady ? "READY" : "WAIT"}</strong></div>
                  <div className={`preview-signal ${audioSignalsReady ? "ready" : ""}`}><span>발화 밀도</span><i /><strong>{audioSignalsReady ? "READY" : "WAIT"}</strong></div>
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
            <div className="settings-title-row"><div><h3>자동 업데이트</h3><p>GitHub Releases의 서명된 안정 버전만 설치합니다.</p></div><button className="button ghost compact" disabled={updateChecking || updateInstalling} onClick={() => void refreshUpdate(true)}><RefreshCw size={14} /> {updateChecking ? "확인 중…" : "업데이트 확인"}</button></div>
            {updateInfo?.available ? <div className="update-available">
              <strong>v{updateInfo.version} 사용 가능</strong>
              <pre>{updateInfo.notes}</pre>
              <button className="button primary" disabled={active || updateInstalling} onClick={() => void installUpdate()}><Download size={16} /> {updateInstalling ? (updateProgress === null ? "다운로드 중…" : `업데이트 ${updateProgress}%`) : active ? "분석 종료 후 업데이트" : "지금 업데이트"}</button>
            </div> : <p className="quiet-copy">{updateChecking ? "최신 안정 버전을 확인하고 있습니다." : "앱 실행 시 자동으로 확인하며, 연결 실패는 로컬 분석을 막지 않습니다."}</p>}
            {updateError ? <p className="settings-error">{updateError}</p> : null}
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
            <div className="settings-title-row"><div><h3>저장 공간 정리</h3><p>현재 PC에 저장된 다운로드 영상·전사·미리보기입니다.</p></div>{storedJobs.length ? <button className="button danger compact" disabled={active} onClick={() => void removeAllStoredJobs()}><Trash2 size={14} /> 전체 삭제</button> : null}</div>
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
