export type SourceKind = "local" | "youtube" | "demo";
export type AnalysisMode = "quick" | "range" | "full";
export type Scenario = "normal" | "fail" | "crash" | "hang" | "malformed";
export type JobStatus =
  | "CREATED"
  | "ACQUIRING"
  | "PROBING"
  | "EXTRACTING_AUDIO"
  | "TRANSCRIBING"
  | "AUDIO_SIGNALS"
  | "CHAT_SIGNALS"
  | "FUSING"
  | "RANKING"
  | "CANCELLING"
  | "CANCELLED"
  | "INTERRUPTED"
  | "FAILED"
  | "NEEDS_INPUT"
  | "REVIEW_READY";

export type CandidateDecision = "PENDING" | "ACCEPTED" | "REJECTED";
export type CaptionSource = "creator" | "automatic";
export type CaptionVerificationState = "UNVERIFIED" | "VERIFIED" | "FAILED";
export type WhisperDeviceMode = "auto" | "gpu" | "cpu";
export type WhisperProfile = "fast" | "balanced" | "accurate";

export interface WhisperSettings {
  deviceMode: WhisperDeviceMode;
  profile: WhisperProfile;
  cpuThreads: number | null;
}

export type WhisperRuntimeStatus = "untested" | "testing" | "gpu" | "cpu" | "cpuFallback" | "failed";

export interface WhisperRuntimeState {
  status: WhisperRuntimeStatus;
  unitIndex: number | null;
  effectiveCpuThreads: number | null;
  gpuFailureReason: string | null;
}

export interface CaptionProvenanceSummary {
  originalFile: string;
  language: string | null;
  trackId: string;
  sha256: string;
  revision: string;
  verificationState: CaptionVerificationState;
}

export interface CaptionDiagnosticSummary {
  kind: string;
  intervalIndex: number | null;
  startSeconds: number | null;
  endSeconds: number | null;
  detail: string;
}

export interface CaptionSummary {
  source: CaptionSource | null;
  language: string | null;
  quality: string;
  fallbackIntervals: number;
  localWhisperFallback: boolean;
  diagnostics: CaptionDiagnosticSummary[];
  provenance: CaptionProvenanceSummary | null;
}

/** 후보 앞뒤 맥락을 이루는 음성 인식 문장 한 줄. 타임코드는 원본 기준이다. */
export interface ContextLine {
  startSeconds: number;
  endSeconds: number | null;
  text: string;
}

export interface Candidate {
  id: string;
  startSeconds: number;
  endSeconds: number;
  title: string;
  summary: string;
  transcriptExcerpt: string;
  audioScore: number;
  dialogueScore: number;
  chatScore: number | null;
  totalScore: number;
  decision: CandidateDecision;
  /**
   * 맥락 구간. worker가 아직 보내지 않는 작업도 열 수 있어야 하므로 선택 항목이다.
   * 값이 없으면 화면에서 기본 여유 구간을 계산해 사용한다.
   */
  contextStartSeconds?: number | null;
  contextEndSeconds?: number | null;
  contextTranscript?: ContextLine[] | null;
}

export interface ActivityEvent {
  sequence: number;
  timestamp: string;
  kind: string;
  message: string;
}

export interface JobSnapshot {
  schemaVersion: number;
  id: string;
  sourceKind: SourceKind;
  sourceLabel: string;
  acquiredMediaPath: string | null;
  downloadPercent: number | null;
  scenario: Scenario;
  analysisMode: AnalysisMode;
  analysisStartSeconds: number | null;
  analysisEndSeconds: number | null;
  status: JobStatus;
  completedUnits: number;
  totalUnits: number;
  mediaDurationSeconds: number | null;
  currentStageLabel: string;
  lastHeartbeatAt: string | null;
  createdAt: string;
  updatedAt: string;
  errorMessage: string | null;
  errorDetail: string | null;
  candidates: Candidate[];
  activity: ActivityEvent[];
  captions: CaptionSummary | null;
  whisper: WhisperSettings;
  whisperRuntime: WhisperRuntimeState;
}

export interface RuntimeInfo {
  appVersion: string;
  dataDirectory: string;
  workerSource: string;
  analysisMode: string;
}

export interface JobStorageInfo {
  sizeBytes: number;
}

export interface StoredJobInfo {
  snapshot: JobSnapshot;
  sizeBytes: number;
}

export interface PreviewMedia {
  path: string;
  clipStartSeconds: number;
  sourceStartSeconds: number;
  sourceEndSeconds: number;
  previewKind: "candidate" | "context";
}

export interface CreateJobInput {
  sourceKind: SourceKind;
  sourceLabel: string;
  scenario: Scenario;
  analysisMode: AnalysisMode;
  analysisStartSeconds: number | null;
  analysisEndSeconds: number | null;
  whisper: WhisperSettings;
}
