import { describe, expect, it } from "vitest";
import { estimateJobTiming, formatBytes, formatDuration, formatTime, parseTimeInput, shortSource, statusLabel } from "./utils";
import type { JobSnapshot } from "./types";

describe("formatTime", () => {
  it("formats hour based timestamps", () => {
    expect(formatTime(754)).toBe("00:12:34");
    expect(formatTime(3661)).toBe("01:01:01");
  });
});

describe("parseTimeInput", () => {
  it("accepts strict hour minute second values", () => {
    expect(parseTimeInput("01:02:03")).toBe(3723);
    expect(parseTimeInput("1:02:03")).toBe(3723);
    expect(parseTimeInput("00:60:00")).toBeNull();
    expect(parseTimeInput("1:2")).toBeNull();
  });
});

describe("shortSource", () => {
  it("keeps a useful beginning and ending", () => {
    const result = shortSource("C:\\very-long-folder-name\\stream-recording-2026-08-02.mkv", 32);
    expect(result.length).toBeLessThanOrEqual(32);
    expect(result.endsWith("-08-02.mkv")).toBe(true);
  });
});

describe("statusLabel", () => {
  it("uses user-facing Korean", () => {
    expect(statusLabel("INTERRUPTED")).toBe("중단됨");
    expect(statusLabel("REVIEW_READY")).toBe("검토 준비");
  });
});

describe("progress helpers", () => {
  it("formats storage and elapsed time for people", () => {
    expect(formatBytes(1_572_864)).toBe("1.5 MB");
    expect(formatDuration(3670)).toBe("1시간 1분");
  });

  it("uses recent transcript chunks for an honest ETA", () => {
    const job = {
      status: "TRANSCRIBING",
      downloadPercent: 100,
      createdAt: "2026-08-02T00:00:00.000Z",
      activity: [
        { sequence: 1, kind: "start", message: "분석 시작", timestamp: "2026-08-02T00:00:00.000Z" },
        { sequence: 2, kind: "progress", message: "오디오 청크 1/4를 추출하고 전사했습니다.", timestamp: "2026-08-02T00:01:00.000Z" },
        { sequence: 3, kind: "progress", message: "오디오 청크 2/4를 추출하고 전사했습니다.", timestamp: "2026-08-02T00:02:00.000Z" }
      ]
    } as JobSnapshot;
    const timing = estimateJobTiming(job, new Date("2026-08-02T00:02:10.000Z"));
    expect(timing.elapsedSeconds).toBe(130);
    expect(timing.remainingSeconds).toBe(130);
    expect(timing.expectedAt?.toISOString()).toBe("2026-08-02T00:04:20.000Z");
  });
});
