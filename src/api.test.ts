import { describe, expect, it } from "vitest";
import {
  createJob,
  getQueue,
  selectCandidatesForCount,
  syncCandidateDecision,
  syncCandidateTranscript
} from "./api";
import type { Candidate, CreateJobInput } from "./types";

function candidate(id: string): Candidate {
  return {
    id,
    startSeconds: 0,
    endSeconds: 30,
    title: "후보",
    summary: "요약",
    transcriptExcerpt: "기존 결과",
    audioScore: 50,
    dialogueScore: 50,
    chatScore: null,
    totalScore: 50,
    decision: "PENDING"
  };
}

describe("mock candidate pool synchronization", () => {
  it("preserves a decision across 30 to 8 and back to 30 candidates", () => {
    const pool = Array.from({ length: 30 }, (_, index) => candidate(`candidate-${index}`));
    let visible = pool.slice(0, 30);

    expect(syncCandidateDecision(visible, pool, "candidate-20", "ACCEPTED")).toBe(true);
    visible = selectCandidatesForCount(visible, pool, 8);
    visible = selectCandidatesForCount(visible, pool, 30);

    expect(visible.find((item) => item.id === "candidate-20")?.decision).toBe("ACCEPTED");
  });

  it("updates transcript quality fields in the visible candidate and pool", () => {
    const pool = [candidate("candidate-1")];
    const visible = pool.map((item) => ({ ...item }));
    const update = {
      transcriptExcerpt: "음성 인식 결과가 불확실해 원문을 표시하지 않습니다.",
      transcriptQualityStatus: "UNCERTAIN" as const,
      transcriptQualityReasons: ["반복 문구"],
      qualityStatus: "WARNING" as const,
      qualityWarnings: ["반복 문구"],
      uncertaintyReasons: ["반복 문구"]
    };

    expect(syncCandidateTranscript(visible, pool, "candidate-1", update)).toBe(true);
    expect(visible[0]).toMatchObject(update);
    expect(pool[0]).toMatchObject(update);
  });
});

describe("mock queue evaluation contract", () => {
  it("exposes only unavailable sequential execution with max concurrency one", async () => {
    const queue = await getQueue();
    expect(queue.executionMode).toBe("SEQUENTIAL");
    expect(queue.evaluation).toMatchObject({
      status: "UNMEASURED_PENDING",
      effectiveExecutionMode: "SEQUENTIAL",
      maxConcurrency: 1,
      parallelAvailable: false,
      sequentialFallbackReason: null
    });
  });
});

describe("Whisper approval policy", () => {
  it("rejects a local file job until the user explicitly approves its settings", async () => {
    const input = {
      sourceKind: "local",
      sourceLabel: "fixture.mp4",
      scenario: "normal",
      analysisMode: "full",
      analysisStartSeconds: null,
      analysisEndSeconds: null,
      whisper: { deviceMode: "auto", profile: "balanced", cpuThreads: null },
      whisperApproved: false,
      candidateCount: 20
    } satisfies CreateJobInput;
    await expect(createJob(input)).rejects.toThrow("사용을 승인한 뒤에만");
  });

  it("rejects creation-time Whisper approval for a YouTube job", async () => {
    const input = {
      sourceKind: "youtube",
      sourceLabel: "https://www.youtube.com/watch?v=fixture",
      scenario: "normal",
      analysisMode: "full",
      analysisStartSeconds: null,
      analysisEndSeconds: null,
      whisper: { deviceMode: "gpu", profile: "accurate", cpuThreads: 6 },
      whisperApproved: true,
      candidateCount: 20
    } satisfies CreateJobInput;
    await expect(createJob(input)).rejects.toThrow("NEEDS_INPUT 상태에서만");
  });
});
