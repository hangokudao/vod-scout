import { describe, expect, it } from "vitest";
import {
  getQueue,
  selectCandidatesForCount,
  syncCandidateDecision,
  syncCandidateTranscript
} from "./api";
import type { Candidate } from "./types";

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
