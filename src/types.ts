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
}

export interface CreateJobInput {
  sourceKind: SourceKind;
  sourceLabel: string;
  scenario: Scenario;
  analysisMode: AnalysisMode;
  analysisStartSeconds: number | null;
  analysisEndSeconds: number | null;
}
