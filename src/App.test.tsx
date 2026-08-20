import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import App, {
  CANDIDATE_SORTS,
  CANDIDATE_COUNTS,
  DEFAULT_CONTEXT_PADDING_SECONDS,
  DEFAULT_UI_SETTINGS,
  SETTINGS_STORAGE_KEY,
  readUiSettings,
  resolveCandidateContext,
  resolveTheme,
  resolveUpdateStatus,
  safeCandidateDerivedText,
  safeTranscriptText,
  hasStartedRecognitionRun,
  selectionStorageKey,
  sortCandidates,
  writeUiSettings,
  normalizeWhisperSettings,
  queueEvaluationExplanation,
  resourceMetricValue,
  type CandidateSortKey
} from "./App";
import type { Candidate } from "./types";

function candidate(overrides: Partial<Candidate> & { id: string }): Candidate {
  return {
    startSeconds: 0,
    endSeconds: 30,
    title: "후보",
    summary: "요약",
    transcriptExcerpt: "인용",
    audioScore: 50,
    dialogueScore: 50,
    chatScore: 50,
    totalScore: 50,
    decision: "PENDING",
    ...overrides
  };
}

const SAMPLE: Candidate[] = [
  candidate({ id: "c", startSeconds: 300, endSeconds: 340, totalScore: 80, audioScore: 40, dialogueScore: 91, chatScore: null, decision: "REJECTED" }),
  candidate({ id: "a", startSeconds: 100, endSeconds: 140, totalScore: 90, audioScore: 70, dialogueScore: 60, chatScore: 30, decision: "PENDING" }),
  candidate({ id: "b", startSeconds: 200, endSeconds: 240, totalScore: 90, audioScore: 95, dialogueScore: 60, chatScore: 88, decision: "ACCEPTED" })
];

const ids = (list: Candidate[]) => list.map((item) => item.id);

describe("sortCandidates", () => {
  it("puts the highest total score first by default", () => {
    expect(ids(sortCandidates(SAMPLE, "totalScore"))).toEqual(["a", "b", "c"]);
  });

  it("breaks ties by source time and then by candidate id", () => {
    const tied = [
      candidate({ id: "z", startSeconds: 500, totalScore: 70 }),
      candidate({ id: "m", startSeconds: 100, totalScore: 70 }),
      candidate({ id: "d", startSeconds: 100, totalScore: 70 })
    ];
    expect(ids(sortCandidates(tied, "totalScore"))).toEqual(["d", "m", "z"]);
  });

  it("orders by source time, audio, and dialogue", () => {
    expect(ids(sortCandidates(SAMPLE, "startSeconds"))).toEqual(["a", "b", "c"]);
    expect(ids(sortCandidates(SAMPLE, "audioScore"))).toEqual(["b", "a", "c"]);
    expect(ids(sortCandidates(SAMPLE, "dialogueScore"))).toEqual(["c", "a", "b"]);
  });

  it("sorts candidates without a chat signal last", () => {
    expect(ids(sortCandidates(SAMPLE, "chatScore"))).toEqual(["b", "a", "c"]);
  });

  it("orders decisions as accepted, pending, then rejected", () => {
    expect(ids(sortCandidates(SAMPLE, "decision"))).toEqual(["b", "a", "c"]);
  });

  it("is deterministic and never mutates the input", () => {
    const original = [...SAMPLE];
    for (const sort of CANDIDATE_SORTS) {
      const first = sortCandidates(SAMPLE, sort.value);
      const second = sortCandidates([...SAMPLE].reverse(), sort.value);
      expect(ids(first)).toEqual(ids(second));
    }
    expect(SAMPLE).toEqual(original);
  });

  it("keeps every candidate and its scores untouched across sorts", () => {
    const sorted = sortCandidates(SAMPLE, "audioScore");
    expect(sorted).toHaveLength(SAMPLE.length);
    for (const item of SAMPLE) {
      expect(sorted.find((entry) => entry.id === item.id)).toEqual(item);
    }
  });
});

describe("candidate count contract", () => {
  it("offers only the persisted 8, 20, and 30 choices", () => {
    expect(CANDIDATE_COUNTS).toEqual([8, 20, 30]);
    expect(CANDIDATE_COUNTS).not.toContain(0);
    expect(CANDIDATE_COUNTS).not.toContain(10);
  });

  it("keeps quality evidence separate from ranking score", () => {
    const item = candidate({
      id: "quality",
      totalScore: 91,
      qualityStatus: "WARNING",
      selectionReasons: ["오디오 반응 91"],
      uncertaintyReasons: ["앞뒤 문장이 연결되지 않아 맥락 확인이 필요함"]
    });
    expect(item.totalScore).toBe(91);
    expect(item.qualityStatus).toBe("WARNING");
    expect(item.selectionReasons).not.toEqual(item.uncertaintyReasons);
  });
});

describe("queue parallel evaluation display", () => {
  it("explains that unmeasured parallel execution is unavailable", () => {
    expect(queueEvaluationExplanation({
      status: "UNMEASURED_PENDING",
      effectiveExecutionMode: "SEQUENTIAL",
      maxConcurrency: 1,
      parallelAvailable: false,
      sequentialFallbackReason: null
    })).toContain("승인된 하드웨어가 없고");
  });

  it("keeps a persisted fallback reason read-only in the explanation", () => {
    expect(queueEvaluationExplanation({
      status: "SEQUENTIAL_FALLBACK",
      effectiveExecutionMode: "SEQUENTIAL",
      maxConcurrency: 1,
      parallelAvailable: false,
      sequentialFallbackReason: "동일 입력 측정이 없어 순차 처리로 고정했습니다."
    })).toContain("동일 입력 측정이 없어 순차 처리로 고정했습니다.");
  });
});

describe("selection stays with the candidate id across a reorder", () => {
  it("finds the same candidate in every sort order", () => {
    const selectedId = "b";
    for (const sort of CANDIDATE_SORTS) {
      const sorted = sortCandidates(SAMPLE, sort.value);
      const selected = sorted.find((item) => item.id === selectedId);
      expect(selected?.id).toBe(selectedId);
      expect(selected?.startSeconds).toBe(200);
      expect(selected?.decision).toBe("ACCEPTED");
    }
  });

  it("changes which position holds the selection without changing the candidate", () => {
    const byTotal = sortCandidates(SAMPLE, "totalScore");
    const byDialogue = sortCandidates(SAMPLE, "dialogueScore");
    expect(byTotal.findIndex((item) => item.id === "b")).toBe(1);
    expect(byDialogue.findIndex((item) => item.id === "b")).toBe(2);
  });

  it("scopes the stored selection key to a job", () => {
    expect(selectionStorageKey("job-1")).toBe("vod-scout.selected-candidate.job-1");
    expect(selectionStorageKey("job-2")).not.toBe(selectionStorageKey("job-1"));
  });
});

describe("transcript quality display", () => {
  it("masks replacement characters and uncertain source text", () => {
    expect(safeTranscriptText("깨진 � 문장")).toContain("불확실");
    expect(safeTranscriptText("원문", "UNCERTAIN")).toContain("불확실");
    expect(safeTranscriptText("정상 문장", "CERTAIN")).toBe("정상 문장");
  });

  it("keeps audio evidence visible while masking uncertain candidate text", () => {
    const uncertain = candidate({
      id: "uncertain",
      title: "음성 인식 결과 불확실 · 오디오 근거 구간",
      summary: "음성 인식 결과 불확실 · 오디오 반응 91 · 발화 밀도 64",
      transcriptExcerpt: "음성 인식 결과가 불확실해 원문을 표시하지 않습니다.",
      transcriptQualityStatus: "UNCERTAIN"
    });
    expect(safeCandidateDerivedText(uncertain.title)).toContain("오디오 근거 구간");
    expect(safeCandidateDerivedText(uncertain.summary)).toContain("오디오 반응 91");
    expect(safeTranscriptText(uncertain.transcriptExcerpt, uncertain.transcriptQualityStatus)).toContain("불확실");
  });

  it("masks unsafe derived text even when its status is unavailable", () => {
    expect(safeCandidateDerivedText("제목 � 손상")).toContain("불확실");
    expect(safeCandidateDerivedText("오디오 반응 91 · 채팅 움직임 84")).toBe("오디오 반응 91 · 채팅 움직임 84");
  });

  it("masks only context lines that contain unsafe text", () => {
    const context = resolveCandidateContext({
      ...candidate({ id: "context" }),
      contextTranscript: [
        { startSeconds: 8, endSeconds: 10, text: "안전한 앞 맥락" },
        { startSeconds: 22, endSeconds: 24, text: "깨진 � 원문" },
        { startSeconds: 35, endSeconds: 37, text: "안전한 뒤 맥락" }
      ]
    }, 60);
    expect(context.lines.map((line) => line.text)).toEqual([
      "안전한 앞 맥락",
      "음성 인식 결과가 불확실해 원문을 표시하지 않습니다.",
      "안전한 뒤 맥락"
    ]);
  });
});

describe("manual recognition busy state", () => {
  it("stays busy when a started run belongs to a non-selected candidate", () => {
    expect(hasStartedRecognitionRun([
      {
        id: "run-other",
        candidateId: "candidate-other",
        status: "STARTED",
        startedAt: "2026-08-18T00:00:00.000Z",
        completedAt: null,
        resultRevision: 1,
        originalResult: "기존 결과",
        rawResult: null,
        displayResult: null,
        failureReason: null,
        backendEvidence: "CPU 시도"
      }
    ])).toBe(true);
  });
});

describe("ui settings persistence", () => {
  beforeEach(() => window.localStorage.clear());

  it("falls back to the system theme and the total score sort", () => {
    expect(readUiSettings()).toEqual(DEFAULT_UI_SETTINGS);
    expect(DEFAULT_UI_SETTINGS.theme).toBe("system");
  });

  it("stores every setting under a single namespaced key", () => {
    writeUiSettings({ theme: "dark", sortKey: "startSeconds" });
    expect(window.localStorage.getItem(SETTINGS_STORAGE_KEY)).toContain("dark");
    expect(Object.keys(window.localStorage)).toEqual([SETTINGS_STORAGE_KEY]);
  });

  it("reads back a saved theme so it survives a restart", () => {
    writeUiSettings({ theme: "light", sortKey: "chatScore" });
    expect(readUiSettings()).toEqual({ theme: "light", sortKey: "chatScore" });
  });

  it("ignores damaged or unknown stored values", () => {
    window.localStorage.setItem(SETTINGS_STORAGE_KEY, "{not json");
    expect(readUiSettings()).toEqual(DEFAULT_UI_SETTINGS);
    writeUiSettings({ theme: "sepia" as never, sortKey: "loudness" as unknown as CandidateSortKey });
    expect(readUiSettings()).toEqual(DEFAULT_UI_SETTINGS);
  });
});

describe("resolveTheme", () => {
  it("follows the system setting by default", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
  });

  it("lets an explicit choice win over the system setting", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });
});

describe("resolveCandidateContext", () => {
  const base = candidate({ id: "x", startSeconds: 600, endSeconds: 660 });

  it("uses the range the worker sent", () => {
    const context = resolveCandidateContext({ ...base, contextStartSeconds: 560, contextEndSeconds: 700 }, 7200);
    expect(context.startSeconds).toBe(560);
    expect(context.endSeconds).toBe(700);
    expect(context.fromWorker).toBe(true);
  });

  it("falls back to a default padding when the worker sent nothing", () => {
    const context = resolveCandidateContext(base, 7200);
    expect(context.startSeconds).toBe(600 - DEFAULT_CONTEXT_PADDING_SECONDS);
    expect(context.endSeconds).toBe(660 + DEFAULT_CONTEXT_PADDING_SECONDS);
    expect(context.fromWorker).toBe(false);
  });

  it("never runs past the start or the end of the source", () => {
    const early = resolveCandidateContext(candidate({ id: "e", startSeconds: 5, endSeconds: 40 }), 3600);
    expect(early.startSeconds).toBe(0);

    const late = resolveCandidateContext(candidate({ id: "l", startSeconds: 3500, endSeconds: 3590 }), 3600);
    expect(late.endSeconds).toBe(3600);
  });

  it("always contains the candidate itself", () => {
    const context = resolveCandidateContext({ ...base, contextStartSeconds: 640, contextEndSeconds: 610 }, 7200);
    expect(context.startSeconds).toBeLessThanOrEqual(base.startSeconds);
    expect(context.endSeconds).toBeGreaterThanOrEqual(base.endSeconds);
  });

  it("returns context lines in source time order", () => {
    const context = resolveCandidateContext(
      {
        ...base,
        contextTranscript: [
          { startSeconds: 700, endSeconds: 720, text: "뒤" },
          { startSeconds: 580, endSeconds: 600, text: "앞" }
        ]
      },
      7200
    );
    expect(context.lines.map((line) => line.text)).toEqual(["앞", "뒤"]);
  });

  it("returns no lines when the job has none stored", () => {
    expect(resolveCandidateContext(base, 7200).lines).toEqual([]);
  });
});

describe("resolveUpdateStatus", () => {
  const base = { checking: false, available: false, checkedAt: null, failed: false, analysisActive: false };

  it("separates the states a person needs to tell apart", () => {
    expect(resolveUpdateStatus({ ...base }).kind).toBe("unknown");
    expect(resolveUpdateStatus({ ...base, checking: true }).kind).toBe("checking");
    expect(resolveUpdateStatus({ ...base, checkedAt: "2026-08-04T00:00:00.000Z" }).kind).toBe("current");
    expect(resolveUpdateStatus({ ...base, available: true }).kind).toBe("available");
    expect(resolveUpdateStatus({ ...base, available: true, analysisActive: true }).kind).toBe("waiting");
    expect(resolveUpdateStatus({ ...base, failed: true }).kind).toBe("error");
  });

  it("does not describe a connection failure as an analysis failure", () => {
    const status = resolveUpdateStatus({ ...base, failed: true });
    expect(status.label).toBe("확인 실패");
    expect(status.detail).toContain("로컬 영상 분석");
    expect(status.detail).not.toContain("분석에 실패");
  });

  it("explains that an install waits for the running analysis", () => {
    expect(resolveUpdateStatus({ ...base, available: true, analysisActive: true }).label).toBe("분석 종료 후 설치 대기");
  });
});

describe("user facing wording", () => {
  it("avoids the wording the project bans for people-facing text", () => {
    const wording = [
      ...CANDIDATE_SORTS.map((item) => item.label),
      ...["unknown", "checking", "current", "available", "waiting", "error"].flatMap((kind) => {
        const status = resolveUpdateStatus({
          checking: kind === "checking",
          available: kind === "available" || kind === "waiting",
          checkedAt: kind === "current" ? "2026-08-04T00:00:00.000Z" : null,
          failed: kind === "error",
          analysisActive: kind === "waiting"
        });
        return [status.label, status.detail];
      })
    ].join(" ");
    expect(wording).not.toMatch(/고아/);
    expect(wording).not.toMatch(/전사/);
  });
});

describe("settings entry visibility", () => {
  afterEach(() => {
    cleanup();
    window.localStorage.clear();
  });

  it("keeps a labelled settings control reachable after the app loads", async () => {
    render(<App />);
    const settings = await screen.findByRole("button", { name: "설정·업데이트 열기" });
    expect(settings).toBeVisible();
    expect(settings).toHaveClass("settings-entry");
    expect(settings.textContent).toContain("설정");
    expect(settings.textContent).toMatch(/v\d/);
    expect(settings.querySelector(".settings-entry-label")?.textContent).toBe("설정");
    expect(settings.querySelector(".settings-entry-version")?.textContent).toMatch(/^v/);
  });

  it("opens the settings dialog from the topbar control", async () => {
    render(<App />);
    const settings = await screen.findByRole("button", { name: "설정·업데이트 열기" });
    fireEvent.click(settings);
    expect(await screen.findByRole("dialog", { name: "설정·업데이트" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "설정·업데이트" })).toBeVisible();
  });

  it("shows GPU device, profile, and bounded CPU controls before starting", async () => {
    render(<App />);
    expect(await screen.findByRole("button", { name: /자동\(GPU 우선\)/ })).toBeVisible();
    expect(screen.getByRole("button", { name: /빠르게/ })).toBeVisible();
    expect(screen.getByRole("button", { name: /정확하게/ })).toBeVisible();
    expect(screen.getByRole("combobox")).toHaveValue("auto");
  });
});

describe("Whisper settings payload", () => {
  it("uses auto CPU control and clamps explicit threads to the safe range", () => {
    expect(normalizeWhisperSettings("gpu", "accurate", "auto")).toEqual({
      deviceMode: "gpu",
      profile: "accurate",
      cpuThreads: null
    });
    expect(normalizeWhisperSettings("cpu", "fast", "99").cpuThreads).toBe(32);
    expect(normalizeWhisperSettings("cpu", "fast", "0").cpuThreads).toBe(1);
  });
});

describe("resource metric labels", () => {
  it("keeps unavailable measurements visibly distinct from zero", () => {
    expect(resourceMetricValue(null, "B")).toBe("측정 불가");
    expect(resourceMetricValue(0, "개")).toBe("0개");
  });
});
