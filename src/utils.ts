import type { JobSnapshot, JobStatus } from "./types";

export function formatTime(totalSeconds: number): string {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = Math.floor(totalSeconds % 60);
  return [hours, minutes, seconds]
    .map((value) => String(value).padStart(2, "0"))
    .join(":");
}

export function parseTimeInput(value: string): number | null {
  const parts = value.trim().split(":");
  if (parts.length !== 3 || parts.some((part) => !/^\d{1,2}$/.test(part))) return null;
  const [hours, minutes, seconds] = parts.map(Number);
  if (minutes > 59 || seconds > 59) return null;
  return hours * 3600 + minutes * 60 + seconds;
}

export function shortSource(source: string, limit = 38): string {
  if (source.length <= limit) return source;
  return `${source.slice(0, Math.max(8, limit - 12))}…${source.slice(-10)}`;
}

export function formatDuration(totalSeconds: number): string {
  const seconds = Math.max(0, Math.round(totalSeconds));
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) return `${hours}시간 ${minutes}분`;
  if (minutes > 0) return `${minutes}분`;
  return `${seconds}초`;
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** exponent;
  return `${value.toFixed(exponent === 0 || value >= 10 ? 0 : 1)} ${units[exponent]}`;
}

export interface JobTiming {
  elapsedSeconds: number;
  remainingSeconds: number | null;
  expectedAt: Date | null;
}

export function estimateJobTiming(job: JobSnapshot, now = new Date()): JobTiming {
  const start = [...job.activity].reverse().find((event) => event.kind === "start");
  const startedAt = start ? new Date(start.timestamp) : new Date(job.createdAt);
  const elapsedSeconds = Math.max(0, (now.getTime() - startedAt.getTime()) / 1000);
  let remainingSeconds: number | null = null;

  if (job.status === "ACQUIRING" && (job.downloadPercent ?? 0) >= 4) {
    const percent = job.downloadPercent ?? 0;
    remainingSeconds = elapsedSeconds * (100 - percent) / percent;
  } else if (job.status === "TRANSCRIBING") {
    const chunks = job.activity.flatMap((event) => {
      const match = event.message.match(/오디오 청크 (\d+)\/(\d+)/);
      return match ? [{ current: Number(match[1]), total: Number(match[2]), time: new Date(event.timestamp).getTime() }] : [];
    });
    if (chunks.length >= 2) {
      const recent = chunks.slice(-4);
      const intervals = recent.slice(1).map((entry, index) => (entry.time - recent[index].time) / 1000).filter((value) => value > 0);
      const average = intervals.reduce((sum, value) => sum + value, 0) / intervals.length;
      const latest = chunks.at(-1)!;
      const timeInCurrentChunk = Math.max(0, (now.getTime() - latest.time) / 1000);
      remainingSeconds = Math.max(0, average * (latest.total - latest.current) - timeInCurrentChunk + 20);
    }
  } else if (["AUDIO_SIGNALS", "CHAT_SIGNALS", "FUSING", "RANKING"].includes(job.status)) {
    remainingSeconds = 20;
  }

  return {
    elapsedSeconds,
    remainingSeconds,
    expectedAt: remainingSeconds === null ? null : new Date(now.getTime() + remainingSeconds * 1000)
  };
}

export const ACTIVE_STATUSES: JobStatus[] = [
  "ACQUIRING",
  "PROBING",
  "EXTRACTING_AUDIO",
  "TRANSCRIBING",
  "AUDIO_SIGNALS",
  "CHAT_SIGNALS",
  "FUSING",
  "RANKING",
  "CANCELLING"
];

export const RESUMABLE_STATUSES: JobStatus[] = ["CANCELLED", "INTERRUPTED", "FAILED"];

export function statusLabel(status: JobStatus): string {
  const labels: Record<JobStatus, string> = {
    CREATED: "실행 대기",
    ACQUIRING: "입력 준비",
    PROBING: "미디어 확인",
    EXTRACTING_AUDIO: "오디오 준비",
    TRANSCRIBING: "전사",
    AUDIO_SIGNALS: "오디오 신호",
    CHAT_SIGNALS: "채팅 신호",
    FUSING: "신호 결합",
    RANKING: "후보 정렬",
    CANCELLING: "취소 중",
    CANCELLED: "취소됨",
    INTERRUPTED: "중단됨",
    FAILED: "확인 필요",
    NEEDS_INPUT: "입력 필요",
    REVIEW_READY: "검토 준비"
  };
  return labels[status];
}
