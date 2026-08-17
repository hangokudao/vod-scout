use super::{mutate_job, record_stage_metric, AppState};
use crate::captions::{self, CaptionInterval, CaptionPlan, CaptionProvenance, VerificationState};
use crate::domain::{
    AnalysisMode, Candidate, CandidateDecision, ContextTranscriptEntry, JobStatus, SourceKind,
    TranscriptQualityStatus,
};
use crate::integrity::{
    format_bytes_for_message, free_disk_space_bytes, runtime_hashes, source_fingerprint,
    verify_runtime_bundle,
};
use crate::storage::{
    previous_generation_path, replace_file_preserving_previous,
};
use crate::whisper::{
    self, WhisperAttemptStatus, WhisperDeviceMode, WhisperRuntimeStatus, WhisperSettings,
    WhisperUnitState, MODEL_NAME,
};
use crate::resource::{ResourceSample, ResourceStage};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering as CmpOrdering;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

const CHUNK_SECONDS: f64 = 600.0;
const CHAT_SAMPLE_SECONDS: f64 = 5.0;
const QUICK_CHAT_SAMPLE_SECONDS: f64 = 15.0;
const CHAT_FRAME_SIDE: usize = 64;
const CONTEXT_PADDING_SECONDS: f64 = 15.0;
/// Media checkpoint schema for P0 compatibility fields (fingerprint/tools/ranker).
const MEDIA_CHECKPOINT_SCHEMA: u8 = 5;
/// Candidate scoring contract recorded in checkpoints and provenance.
const RANKER_VERSION: &str = "rules-v0.4.0-p0";
const TRANSCRIPTION_LANGUAGE: &str = "ko";
const MIB: u64 = 1024 * 1024;
/// Soft wait after kill before forcing the whole owned process tree.
pub(crate) const CHILD_TERMINATE_GRACE: Duration = Duration::from_secs(3);
/// Hard cap so cancel never blocks the pipeline on a stuck wait/join.
pub(crate) const CHILD_TERMINATE_HARD_CAP: Duration = Duration::from_secs(8);
pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
pub(crate) struct KillOnCloseJob(HANDLE);

#[cfg(windows)]
impl KillOnCloseJob {
    pub(crate) fn attach(child: &Child) -> Result<Self, std::io::Error> {
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            ) == 0
            {
                let error = std::io::Error::last_os_error();
                CloseHandle(job);
                return Err(error);
            }
            if AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE) == 0 {
                let error = std::io::Error::last_os_error();
                CloseHandle(job);
                return Err(error);
            }
            Ok(Self(job))
        }
    }

    /// Force-terminate every process currently assigned to this job object.
    pub(crate) fn terminate_all(&self) {
        unsafe {
            let _ = TerminateJobObject(self.0, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for KillOnCloseJob {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// Kill the direct child, wait with a grace period, then force the owned job tree.
/// Never blocks longer than [`CHILD_TERMINATE_HARD_CAP`].
///
/// On Windows, when a job object is provided, `TerminateJobObject` always runs
/// after the soft kill. A quick parent exit alone does not prove the tree is
/// gone — descendants can outlive `child.kill()`.
pub(crate) fn terminate_child_tree(
    child: &mut Child,
    #[cfg(windows)] job: Option<&KillOnCloseJob>,
) {
    let started = Instant::now();
    let _ = child.kill();

    let parent_reaped = wait_child_until(child, CHILD_TERMINATE_GRACE);

    #[cfg(windows)]
    if let Some(job) = job {
        job.terminate_all();
    } else if parent_reaped {
        return;
    }

    #[cfg(not(windows))]
    if parent_reaped {
        return;
    }

    let _ = child.kill();
    let remaining = CHILD_TERMINATE_HARD_CAP.saturating_sub(started.elapsed());
    let _ = wait_child_until(child, remaining);
}

/// Wait until the child is reaped or `limit` elapses.
/// Returns `true` only when `try_wait` reports an exited status. An I/O error
/// is **not** treated as reaped — callers must still run the force-terminate
/// path (job object / second kill) which remains capped by
/// [`CHILD_TERMINATE_HARD_CAP`].
fn wait_child_until(child: &mut Child, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) if Instant::now() >= deadline => return false,
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            // try_wait failure is not proof the process tree is gone.
            Err(_) => return false,
        }
    }
}

#[derive(Debug)]
enum PipelineError {
    Cancelled,
    ResourceLimit { stage: ResourceStage, reason: String },
    Message(String),
}

impl From<std::io::Error> for PipelineError {
    fn from(error: std::io::Error) -> Self {
        Self::Message(error.to_string())
    }
}

impl From<serde_json::Error> for PipelineError {
    fn from(error: serde_json::Error) -> Self {
        Self::Message(error.to_string())
    }
}

#[derive(Debug, Deserialize)]
struct ProbeOutput {
    format: ProbeFormat,
    #[serde(default)]
    streams: Vec<ProbeStream>,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranscriptSegment {
    start_seconds: f64,
    end_seconds: f64,
    text: String,
    #[serde(default)]
    quality_status: TranscriptQualityStatus,
    #[serde(default)]
    quality_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnergyPoint {
    start_seconds: f64,
    rms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatMotionPoint {
    start_seconds: f64,
    motion: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlannedChunk {
    offset_seconds: f64,
    length_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaCheckpoint {
    schema_version: u8,
    source_path: String,
    duration_seconds: f64,
    chunk_seconds: f64,
    analysis_mode: AnalysisMode,
    analysis_start_seconds: u32,
    analysis_end_seconds: u32,
    /// Input content fingerprint; required for resume compatibility (P0).
    #[serde(default)]
    input_fingerprint: String,
    #[serde(default)]
    input_bytes: u64,
    /// FFmpeg/Whisper/model (and other runtime) SHA-256 map from the integrity manifest.
    #[serde(default)]
    runtime_sha256: HashMap<String, String>,
    #[serde(default)]
    caption_source_url: String,
    #[serde(default)]
    caption_sha256: String,
    #[serde(default)]
    caption_revision: String,
    #[serde(default)]
    caption_schema_version: u8,
    #[serde(default)]
    caption_content_sha256: String,
    #[serde(default)]
    caption_verification_state: Option<VerificationState>,
    /// Missing fields deserialize empty and fail compatibility (must not invent defaults).
    #[serde(default)]
    language: String,
    #[serde(default)]
    ranker_version: String,
    planned_chunks: Vec<PlannedChunk>,
    completed_chunks: u32,
    segments: Vec<TranscriptSegment>,
    energy: Vec<EnergyPoint>,
    #[serde(default)]
    chat_motion_completed: bool,
    #[serde(default)]
    chat_motion: Vec<ChatMotionPoint>,
    #[serde(default)]
    whisper_settings: WhisperSettings,
    #[serde(default)]
    whisper_units: Vec<WhisperUnitState>,
}

impl MediaCheckpoint {
    fn fresh(
        source_path: &str,
        duration_seconds: f64,
        analysis_mode: AnalysisMode,
        analysis_start_seconds: u32,
        analysis_end_seconds: u32,
        planned_chunks: Vec<PlannedChunk>,
        input_fingerprint: String,
        input_bytes: u64,
        runtime_sha256: HashMap<String, String>,
    ) -> Self {
        Self {
            schema_version: MEDIA_CHECKPOINT_SCHEMA,
            source_path: source_path.into(),
            duration_seconds,
            chunk_seconds: CHUNK_SECONDS,
            analysis_mode,
            analysis_start_seconds,
            analysis_end_seconds,
            input_fingerprint,
            input_bytes,
            runtime_sha256,
            caption_source_url: String::new(),
            caption_sha256: String::new(),
            caption_revision: String::new(),
            caption_schema_version: 0,
            caption_content_sha256: String::new(),
            caption_verification_state: None,
            language: TRANSCRIPTION_LANGUAGE.into(),
            ranker_version: RANKER_VERSION.into(),
            planned_chunks,
            completed_chunks: 0,
            segments: Vec::new(),
            energy: Vec::new(),
            chat_motion_completed: false,
            chat_motion: Vec::new(),
            whisper_settings: WhisperSettings::default(),
            whisper_units: Vec::new(),
        }
    }
}

fn analysis_bounds(
    duration_seconds: f64,
    mode: AnalysisMode,
    requested_start: Option<u32>,
    requested_end: Option<u32>,
) -> Result<(u32, u32), PipelineError> {
    let duration = duration_seconds.ceil().max(1.0) as u32;
    if mode != AnalysisMode::Range {
        return Ok((0, duration));
    }
    let start = requested_start.unwrap_or(0);
    let end = requested_end.unwrap_or(duration);
    if start >= end || end > duration {
        return Err(PipelineError::Message(format!(
            "분석 구간은 00:00:00부터 영상 길이 {}초 사이여야 합니다.",
            duration
        )));
    }
    Ok((start, end))
}

fn build_analysis_plan(mode: AnalysisMode, start: u32, end: u32) -> Vec<PlannedChunk> {
    let start = start as f64;
    let end = end as f64;
    let duration = (end - start).max(0.1);
    if mode != AnalysisMode::Quick {
        let mut chunks = Vec::new();
        let mut offset = start;
        while offset < end {
            chunks.push(PlannedChunk {
                offset_seconds: offset,
                length_seconds: (end - offset).min(CHUNK_SECONDS),
            });
            offset += CHUNK_SECONDS;
        }
        return chunks;
    }

    let budget = (duration * 0.20)
        .clamp(30.0 * 60.0, 120.0 * 60.0)
        .min(duration);
    if budget >= duration - 0.5 {
        return build_analysis_plan(AnalysisMode::Full, start as u32, end as u32);
    }
    let chunk_count = (budget / CHUNK_SECONDS).ceil().max(1.0) as usize;
    let mut remaining = budget;
    let mut lengths = Vec::with_capacity(chunk_count);
    for _ in 0..chunk_count {
        let length = remaining.min(CHUNK_SECONDS);
        lengths.push(length);
        remaining = (remaining - length).max(0.0);
    }
    let occupied = lengths.iter().sum::<f64>();
    let gap = if chunk_count > 1 {
        (duration - occupied).max(0.0) / (chunk_count - 1) as f64
    } else {
        0.0
    };
    let mut offset = start;
    lengths
        .into_iter()
        .map(|length| {
            let chunk = PlannedChunk {
                offset_seconds: offset,
                length_seconds: length,
            };
            offset += length + gap;
            chunk
        })
        .collect()
}

fn is_in_completed_chunks(seconds: f64, chunks: &[PlannedChunk]) -> bool {
    chunks.iter().any(|chunk| {
        seconds >= chunk.offset_seconds
            && seconds < chunk.offset_seconds + chunk.length_seconds + 0.001
    })
}

fn apply_caption_identity(checkpoint: &mut MediaCheckpoint, provenance: &CaptionProvenance) {
    checkpoint.caption_source_url = provenance.source_url.clone();
    checkpoint.caption_sha256 = provenance.sha256.clone();
    checkpoint.caption_revision = provenance.revision.clone();
    checkpoint.caption_schema_version = provenance.schema_version;
    checkpoint.caption_content_sha256 = provenance.content_sha256.clone();
    checkpoint.caption_verification_state = Some(provenance.verification_state);
}

fn caption_plan_for_duration(
    artifacts: Option<&(CaptionProvenance, Vec<u8>)>,
    duration_seconds: f64,
) -> CaptionPlan {
    let Some((provenance, bytes)) = artifacts else {
        return captions::plan_fallbacks(
            &captions::validate_intervals(Vec::new(), duration_seconds, VerificationState::Failed),
            duration_seconds,
        );
    };
    let intervals = captions::parse_caption_text(&String::from_utf8_lossy(bytes));
    let validation = captions::validate_intervals(
        intervals,
        duration_seconds,
        provenance.verification_state,
    );
    captions::plan_fallbacks(&validation, duration_seconds)
}

/// Partition one analyzed chunk into disjoint verified-caption and Whisper ranges.
/// A caption crossing a chunk boundary belongs to the first analyzed chunk it touches;
/// cross-chunk cues are emitted once and their remainder is not Whisper fallback. This
/// also handles sparse quick-analysis chunks and requested-range boundaries without
/// uncovered spans.
fn partition_caption_chunk(
    plan: &CaptionPlan,
    chunks: &[PlannedChunk],
    chunk_index: usize,
    analysis_start_seconds: u32,
    analysis_end_seconds: u32,
) -> (Vec<CaptionInterval>, Vec<(f64, f64)>) {
    let chunk = &chunks[chunk_index];
    let start_seconds = chunk.offset_seconds;
    let end_seconds = chunk.offset_seconds + chunk.length_seconds;
    if plan.full_whisper {
        return (Vec::new(), vec![(start_seconds, end_seconds)]);
    }

    let analysis_start = analysis_start_seconds as f64;
    let analysis_end = analysis_end_seconds as f64;
    let mut trusted = plan
        .trusted
        .iter()
        .filter_map(|interval| {
            let owner = chunks.iter().position(|candidate| {
                interval.end_seconds > candidate.offset_seconds
                    && interval.start_seconds
                        < candidate.offset_seconds + candidate.length_seconds
            });
            if owner != Some(chunk_index) {
                return None;
            }
            let start = interval.start_seconds.max(analysis_start);
            let end = interval.end_seconds.min(analysis_end);
            (start < end).then(|| CaptionInterval {
                start_seconds: start,
                end_seconds: end,
                text: interval.text.clone(),
            })
        })
        .collect::<Vec<_>>();
    trusted.sort_by(|left, right| {
        left.start_seconds
            .total_cmp(&right.start_seconds)
            .then_with(|| left.end_seconds.total_cmp(&right.end_seconds))
    });

    let fallback = plan
        .fallback
        .iter()
        .filter_map(|range| {
            let start = range.start_seconds.max(start_seconds);
            let end = range.end_seconds.min(end_seconds);
            (start < end).then_some((start, end))
        })
        .collect();
    (trusted, fallback)
}

/// How media checkpoint progress was reconciled with the job snapshot's units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckpointAlignResult {
    /// Checkpoint and snapshot already agree on completed media chunks.
    Aligned,
    /// Checkpoint was ahead of the snapshot; intermediate media state was rewound.
    Rewound,
    /// Snapshot claimed more media progress than the checkpoint. Allowed only when
    /// incompatible media intermediates were discarded and rebuilt (schema/fingerprint/etc.).
    RestartMediaFromScratch,
}

/// Align checkpoint chunk progress with `job.completed_units`.
///
/// Job units encode media as: 0/1 acquire+tools, 2 probe, then one unit per completed chunk.
/// When media intermediates were rebuilt (`media_intermediates_rebuilt`), lagging behind the
/// snapshot must restart media work rather than hard-fail — the snapshot's advanced units are
/// stale relative to the discarded checkpoint, not proof of durable media progress.
fn align_checkpoint_with_job_units(
    checkpoint: &mut MediaCheckpoint,
    job_completed_units: u32,
    media_intermediates_rebuilt: bool,
) -> Result<CheckpointAlignResult, String> {
    let chunk_count = checkpoint.planned_chunks.len().max(1) as u32;
    let snapshot_chunks = job_completed_units.saturating_sub(2).min(chunk_count);
    if checkpoint.completed_chunks > snapshot_chunks {
        let completed = &checkpoint.planned_chunks[..snapshot_chunks as usize];
        checkpoint
            .segments
            .retain(|segment| is_in_completed_chunks(segment.start_seconds, completed));
        checkpoint
            .energy
            .retain(|point| is_in_completed_chunks(point.start_seconds, completed));
        checkpoint.completed_chunks = snapshot_chunks;
        checkpoint.chat_motion_completed = false;
        checkpoint.chat_motion.clear();
        Ok(CheckpointAlignResult::Rewound)
    } else if checkpoint.completed_chunks < snapshot_chunks {
        if media_intermediates_rebuilt {
            Ok(CheckpointAlignResult::RestartMediaFromScratch)
        } else {
            Err(
                "작업 스냅샷보다 미디어 체크포인트가 뒤에 있어 자동 재개할 수 없습니다."
                    .into(),
            )
        }
    } else {
        Ok(CheckpointAlignResult::Aligned)
    }
}

/// Job progress units after media intermediates were rebuilt (probe-complete + any chunks kept).
fn job_units_after_media_restart(checkpoint: &MediaCheckpoint) -> u32 {
    2 + checkpoint.completed_chunks
}

#[derive(Debug)]
struct WindowScore {
    start: f64,
    end: f64,
    audio_raw: f64,
    dialogue_raw: f64,
    chat_raw: Option<f64>,
    excerpt: String,
    transcript_quality_status: TranscriptQualityStatus,
    transcript_quality_reasons: Vec<String>,
    quality_warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewKind {
    Candidate,
    Context,
}

impl PreviewKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Context => "context",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewMedia {
    pub path: String,
    pub clip_start_seconds: f64,
    pub source_start_seconds: u32,
    pub source_end_seconds: u32,
    pub preview_kind: PreviewKind,
}

impl Serialize for PreviewKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

pub fn run_media_pipeline<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: Arc<AppState>,
    job_id: String,
) {
    let _heavy_tool_guard = match acquire_heavy_tool_gate(&state) {
        Ok(guard) => guard,
        Err(reason) => {
            let _ = terminalize_heavy_tool_gate_failure(&app, &state, reason);
            state.cancel_requested.store(false, Ordering::SeqCst);
            state.manual_running.store(false, Ordering::SeqCst);
            state.running.store(false, Ordering::SeqCst);
            crate::continue_queue(app, Arc::clone(&state));
            return;
        }
    };
    let result = run(&app, &state, &job_id);
    match result {
        Ok(candidates) => {
            let candidate_pool = candidates.clone();
            let count = candidates.len().min(30);
            let _ = mutate_job(&app, &state, |job| {
                let source_label = if job.source_kind == SourceKind::Youtube {
                    "YouTube 영상"
                } else {
                    "로컬 영상"
                };
                job.candidate_pool = candidate_pool;
                job.candidates = job.candidate_pool.clone();
                job.candidates.sort_by(|left, right| right.total_score.cmp(&left.total_score).then_with(|| left.start_seconds.cmp(&right.start_seconds)));
                job.candidates.truncate(job.candidate_count as usize);
                job.push_activity(
                    "candidates",
                    &format!("후보 구간 {count}개를 검토 목록에 추가했습니다."),
                );
                job.transition(JobStatus::ReviewReady)?;
                job.current_stage_label = "후보 검토 준비".into();
                job.error_message = None;
                job.error_detail = None;
                job.push_activity(
                    "complete",
                    &format!("{source_label} 분석을 마쳤습니다. 후보를 검토해 주세요."),
                );
                Ok(())
            });
        }
        Err(PipelineError::Cancelled) => {
            // Child terminate already finished inside run_command before Cancelled bubbles up.
            // Do not invent a wall-clock here; hard-cap/grace are enforced at terminate sites.
            let _ = mutate_job(&app, &state, |job| {
                if job.status != JobStatus::Cancelling && job.status.is_active() {
                    job.transition(JobStatus::Cancelling)?;
                    job.current_stage_label = "실행 중 도구 종료 중".into();
                    job.push_activity(
                        "cancel",
                        "취소 요청을 반영했습니다. 관련 도구 프로세스를 종료하는 중입니다.",
                    );
                }
                job.owned_child_processes = 0;
                job.transition(JobStatus::Cancelled)?;
                job.current_stage_label = "사용자가 취소함".into();
                job.error_message = None;
                job.error_detail = None;
                job.push_activity(
                    "cancel",
                    "실행 중인 도구를 종료했습니다. 완료된 청크부터 재개할 수 있습니다.",
                );
                Ok(())
            });
        }
        Err(PipelineError::ResourceLimit { stage, reason }) => {
            let _ = terminalize_resource_limit(&app, &state, stage, reason);
        }
        Err(PipelineError::Message(detail)) => {
            let _ = mutate_job(&app, &state, |job| {
                if job.status == JobStatus::Cancelling {
                    job.owned_child_processes = 0;
                    job.transition(JobStatus::Cancelled)?;
                    job.current_stage_label = "사용자가 취소함".into();
                    job.error_message = None;
                    job.error_detail = None;
                    job.push_activity(
                        "cancel",
                        "실행 중인 도구를 종료했습니다. 완료된 청크부터 재개할 수 있습니다.",
                    );
                } else {
                    job.transition(JobStatus::Failed)?;
                    job.error_message = Some(if job.source_kind == SourceKind::Youtube {
                        "YouTube 영상 분석을 완료하지 못했습니다.".into()
                    } else {
                        "로컬 영상 분석을 완료하지 못했습니다.".into()
                    });
                    job.error_detail = Some(detail);
                    job.push_activity("error", "현재 체크포인트를 보존하고 분석을 중지했습니다.");
                }
                Ok(())
            });
        }
    }
    state.cancel_requested.store(false, Ordering::SeqCst);
    let _ = mutate_job(&app, &state, |job| {
        job.owned_child_processes = 0;
        Ok(())
    });
    drop(_heavy_tool_guard);
    state.running.store(false, Ordering::SeqCst);
    crate::continue_queue(app, state);
}

pub fn run_candidate_recognition<R: tauri::Runtime>(app: tauri::AppHandle<R>, state: Arc<AppState>, job_id: String, candidate_id: String, run_id: String) {
    let result = match acquire_heavy_tool_gate(&state) {
        Ok(_heavy_tool_guard) => recognize_candidate(&state, &job_id, &candidate_id, &run_id),
        Err(error) => Err(CandidateRecognitionFailure {
            reason: error,
            evidence: "무거운 외부 도구 실행 잠금이 손상되어 실행하지 않았습니다.".into(),
        }),
    };
    let _ = mutate_job(&app, &state, |job| {
        let run = job.recognition_runs.iter_mut().find(|run| run.id == run_id)
            .ok_or_else(|| "음성 인식 실행 기록을 찾을 수 없습니다.".to_string())?;
        match result {
            Ok(output) => {
                run.complete(Utc::now(), output.raw_result.clone(), output.display_result.clone(), output.backend_evidence.clone())?;
                apply_candidate_recognition_output(&mut job.candidates, &candidate_id, &output);
                apply_candidate_recognition_output(&mut job.candidate_pool, &candidate_id, &output);
                job.current_stage_label = "선택 후보 음성 인식 완료".into();
                job.push_activity("recognition", "선택 후보 음성 인식을 완료했습니다. 기존 후보 판정은 유지했습니다.");
            }
            Err(error) => {
                run.fail(Utc::now(), error.reason.clone(), error.evidence.clone())?;
                job.current_stage_label = "선택 후보 음성 인식 실패".into();
                job.push_activity("recognition-error", &format!("선택 후보 음성 인식에 실패했습니다: {}", error.reason));
            }
        }
        Ok(())
    });
    state.cancel_requested.store(false, Ordering::SeqCst);
    state.manual_running.store(false, Ordering::SeqCst);
    let _ = mutate_job(&app, &state, |job| {
        job.owned_child_processes = 0;
        Ok(())
    });
    state.running.store(false, Ordering::SeqCst);
}

fn acquire_heavy_tool_gate(state: &AppState) -> Result<MutexGuard<'_, ()>, String> {
    state
        .heavy_tool_gate
        .lock()
        .map_err(|_| "무거운 외부 도구 실행 잠금이 손상됐습니다.".to_string())
}

fn apply_heavy_tool_gate_failure(
    job: &mut crate::domain::JobSnapshot,
    reason: String,
) -> Result<(), String> {
    job.transition(JobStatus::Failed)?;
    job.current_stage_label = "외부 도구 실행 잠금 실패".into();
    job.error_message = Some("외부 도구 실행 잠금 오류로 분석을 시작하지 못했습니다.".into());
    job.error_detail = Some(reason.clone());
    job.owned_child_processes = 0;
    job.push_activity(
        "error",
        &format!("외부 도구 실행 잠금 오류로 분석을 중지했습니다: {reason}"),
    );
    Ok(())
}

fn terminalize_heavy_tool_gate_failure<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &AppState,
    reason: String,
) -> Result<(), String> {
    mutate_job(app, state, |job| apply_heavy_tool_gate_failure(job, reason))?;
    Ok(())
}

fn apply_resource_limit_failure(
    job: &mut crate::domain::JobSnapshot,
    stage: ResourceStage,
    reason: String,
) -> Result<(), String> {
    job.transition(JobStatus::Failed)?;
    job.resource_failure = Some(crate::resource::ResourceLimitFailure {
        stage,
        reason: reason.clone(),
        last_completed_units: job.completed_units,
    });
    job.error_message = Some("자원 제한을 초과해 현재 작업을 중지했습니다.".into());
    job.error_detail = Some(reason.clone());
    job.owned_child_processes = 0;
    job.push_activity("resource-limit", &format!("{}: {reason}", stage.label()));
    Ok(())
}

fn terminalize_resource_limit<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &AppState,
    stage: ResourceStage,
    reason: String,
) -> Result<(), String> {
    mutate_job(app, state, |job| apply_resource_limit_failure(job, stage, reason))?;
    Ok(())
}

struct CandidateRecognitionOutput {
    raw_result: String,
    display_result: String,
    quality_status: TranscriptQualityStatus,
    quality_reasons: Vec<String>,
    backend_evidence: String,
}

fn apply_candidate_recognition_output(
    candidates: &mut [Candidate],
    candidate_id: &str,
    output: &CandidateRecognitionOutput,
) {
    if let Some(candidate) = candidates.iter_mut().find(|candidate| candidate.id == candidate_id) {
        candidate.transcript_excerpt = output.display_result.clone();
        candidate.transcript_quality_status = output.quality_status;
        candidate.transcript_quality_reasons = output.quality_reasons.clone();
        candidate.quality_status = if output.quality_reasons.is_empty() { "VALID" } else { "WARNING" }.into();
        candidate.quality_warnings = output.quality_reasons.clone();
        candidate.uncertainty_reasons = output.quality_reasons.clone();
    }
}

struct CandidateRecognitionFailure {
    reason: String,
    evidence: String,
}

fn recognition_failure(error: PipelineError, evidence: &str) -> CandidateRecognitionFailure {
    let cancelled = matches!(&error, PipelineError::Cancelled);
    let reason = match error {
        PipelineError::Cancelled => "사용자가 음성 인식을 취소했습니다.".into(),
        PipelineError::ResourceLimit { reason, .. } => format!("자원 제한 초과: {reason}"),
        PipelineError::Message(message) => message,
    };
    let evidence = if cancelled {
        format!("{evidence}; 취소 요청 확인")
    } else {
        evidence.into()
    };
    CandidateRecognitionFailure { reason, evidence }
}

fn recognize_candidate(state: &Arc<AppState>, job_id: &str, candidate_id: &str, run_id: &str) -> Result<CandidateRecognitionOutput, CandidateRecognitionFailure> {
    let (source_path, candidate, settings) = {
        let guard = state.job.lock().map_err(|_| CandidateRecognitionFailure { reason: "작업 상태 잠금이 손상됐습니다.".into(), evidence: "후보 음성 인식 시작 전 상태 잠금 확인 실패".into() })?;
        let job = guard.as_ref().ok_or_else(|| CandidateRecognitionFailure { reason: "현재 작업이 없습니다.".into(), evidence: "후보 음성 인식 시작 전 현재 작업 확인 실패".into() })?;
        if job.id != job_id || job.status != JobStatus::ReviewReady {
            return Err(CandidateRecognitionFailure { reason: "검토 준비가 끝난 현재 작업에서만 다시 음성 인식을 실행할 수 있습니다.".into(), evidence: "후보 음성 인식 시작 전 작업 상태 확인 실패".into() });
        }
        let candidate = job.candidates.iter().find(|candidate| candidate.id == candidate_id).cloned()
            .ok_or_else(|| CandidateRecognitionFailure { reason: "선택한 후보를 찾을 수 없습니다.".into(), evidence: "후보 음성 인식 시작 전 후보 확인 실패".into() })?;
        let source_path = job.acquired_media_path.clone().unwrap_or_else(|| job.source_label.clone());
        (source_path, candidate, job.whisper.clone())
    };
    let threads = whisper::effective_cpu_threads(&settings, thread::available_parallelism().map(|count| count.get()).unwrap_or(4));
    let mut evidence = format!("요청 장치={:?}; 프로필={:?}; CPU 스레드={threads}; 모델={MODEL_NAME}", settings.device_mode, settings.profile);
    if state.cancel_requested.load(Ordering::SeqCst) {
        evidence.push_str("; 취소 요청 확인");
        return Err(recognition_failure(PipelineError::Cancelled, &evidence));
    }
    let source = PathBuf::from(source_path);
    if !source.is_file() {
        evidence.push_str("; 원본 영상 파일 없음");
        return Err(CandidateRecognitionFailure { reason: "후보의 원본 영상 파일을 찾을 수 없습니다.".into(), evidence });
    }
    let tools = locate_tools(&state.resource_dir).map_err(|error| recognition_failure(error, &evidence))?;
    let run_dir = state.store.job_dir(job_id).join("recognition-runs").join(run_id);
    fs::create_dir_all(&run_dir).map_err(|error| CandidateRecognitionFailure { reason: error.to_string(), evidence: format!("{evidence}; 실행 디렉터리 생성 실패") })?;
    let wav = run_dir.join("candidate.wav");
    let prefix = run_dir.join("transcript");
    let log_prefix = run_dir.join("whisper");
    run_command(&state.cancel_requested, &tools.ffmpeg, &tools.ffmpeg_dir, [
        "-hide_banner".into(), "-loglevel".into(), "error".into(), "-y".into(),
        "-ss".into(), candidate.start_seconds.to_string().into(),
        "-t".into(), candidate.end_seconds.saturating_sub(candidate.start_seconds).max(1).to_string().into(),
        "-protocol_whitelist".into(), "file,crypto,data".into(), "-i".into(), source.as_os_str().into(),
        "-vn".into(), "-ac".into(), "1".into(), "-ar".into(), "16000".into(), "-c:a".into(), "pcm_s16le".into(), wav.as_os_str().into(),
    ], &log_prefix.with_extension("ffmpeg.stdout.log"), &log_prefix.with_extension("ffmpeg.stderr.log"))
        .map_err(|error| recognition_failure(error, &format!("{evidence}; 오디오 추출 시도")))?;
    let mut segments = None;
    if !matches!(settings.device_mode, WhisperDeviceMode::Cpu) {
        if let Some(gpu) = tools.whisper_gpu.as_ref() {
            let gpu_prefix = run_dir.join("gpu-probe");
            let gpu_probe = run_gpu_probe(&state.cancel_requested, &tools.ffmpeg, &tools.ffmpeg_dir, gpu, &tools.whisper_gpu_dir, &tools.model, &wav, &run_dir.join("gpu-probe.wav"), &gpu_prefix, threads, &settings, &run_dir.join("gpu-probe.stdout.log"), &run_dir.join("gpu-probe.stderr.log"));
            match gpu_probe.and_then(|_| run_whisper_attempt(&state.cancel_requested, gpu, &tools.whisper_gpu_dir, &tools.model, &wav, &prefix, candidate.start_seconds as f64, threads, &settings, true, &run_dir.join("gpu.stdout.log"), &run_dir.join("gpu.stderr.log"))) {
                Ok(value) => { evidence.push_str("; 실제 백엔드=whisper.cpp-gpu; GPU 시험·결과 확인"); segments = Some(value); }
                Err(error @ PipelineError::Cancelled) => {
                    evidence.push_str("; GPU 시도 중 취소 요청");
                    return Err(recognition_failure(error, &evidence));
                }
                Err(error) => evidence.push_str(&format!("; GPU 실패 후 CPU 대체={}", sanitize_gpu_failure_reason(&format!("{error:?}")))),
            }
        } else { evidence.push_str("; GPU 런타임 없음·CPU 대체"); }
    }
    if segments.is_none() {
        let value = run_whisper_attempt(&state.cancel_requested, &tools.whisper, &tools.whisper_dir, &tools.model, &wav, &prefix, candidate.start_seconds as f64, threads, &settings, false, &run_dir.join("cpu.stdout.log"), &run_dir.join("cpu.stderr.log"))
            .map_err(|error| recognition_failure(error, &format!("{evidence}; CPU 음성 인식 시도")))?;
        evidence.push_str("; 실제 백엔드=whisper.cpp-cpu");
        segments = Some(value);
    }
    let segments = sanitize_transcript_segments(segments.unwrap_or_default());
    let raw_result = segments.iter().map(|segment| segment.text.as_str()).collect::<Vec<_>>().join(" ");
    if raw_result.is_empty() {
        evidence.push_str("; 실제 백엔드 결과가 비어 있음");
        return Err(CandidateRecognitionFailure { reason: "Whisper 음성 인식 결과가 비어 있습니다.".into(), evidence });
    }
    let quality_reasons = segments.iter().flat_map(|segment| segment.quality_reasons.iter().cloned()).fold(Vec::new(), |mut reasons, reason| { if !reasons.contains(&reason) { reasons.push(reason); } reasons });
    let quality_status = if quality_reasons.is_empty() { TranscriptQualityStatus::Certain } else { TranscriptQualityStatus::Uncertain };
    let display_result = if quality_status == TranscriptQualityStatus::Uncertain || raw_result.contains('\u{fffd}') { UNCERTAIN_TRANSCRIPT_PLACEHOLDER.into() } else { raw_result.clone() };
    Ok(CandidateRecognitionOutput { raw_result, display_result, quality_status, quality_reasons, backend_evidence: evidence })
}

pub(crate) fn preview_cache_key(
    job_id: &str,
    candidate_id: &str,
    source_fingerprint: &str,
    context_start_seconds: f64,
    context_end_seconds: f64,
    preview_kind: PreviewKind,
) -> String {
    format!(
        "job={job_id}|candidate={candidate_id}|source={source_fingerprint}|contextStart={context_start_seconds:.3}|contextEnd={context_end_seconds:.3}|kind={}",
        preview_kind.as_str()
    )
}

fn preview_output_name(cache_key: &str, preview_kind: PreviewKind) -> String {
    let digest = Sha256::digest(cache_key.as_bytes());
    format!("{}-{digest:x}.mp4", preview_kind.as_str())
}

fn preview_temporary_path(output: &Path) -> PathBuf {
    output.with_extension("tmp.mp4")
}

pub(crate) fn prepare_candidate_preview<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &Arc<AppState>,
    job_id: &str,
    candidate_id: &str,
) -> Result<PreviewMedia, String> {
    prepare_preview(app, state, job_id, candidate_id, PreviewKind::Candidate)
}

pub(crate) fn prepare_candidate_context_preview<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &Arc<AppState>,
    job_id: &str,
    candidate_id: &str,
) -> Result<PreviewMedia, String> {
    prepare_preview(app, state, job_id, candidate_id, PreviewKind::Context)
}

fn prepare_preview<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &Arc<AppState>,
    job_id: &str,
    candidate_id: &str,
    preview_kind: PreviewKind,
) -> Result<PreviewMedia, String> {
    let _heavy_tool_guard = acquire_heavy_tool_gate(state)?;
    if state.running.load(Ordering::SeqCst) {
        return Err("분석이 끝난 뒤 후보 영상을 준비할 수 있습니다.".into());
    }
    let (source_path, candidate, media_duration_seconds) = {
        let guard = state
            .job
            .lock()
            .map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())?;
        let job = guard
            .as_ref()
            .ok_or_else(|| "현재 작업이 없습니다.".to_string())?;
        if job.id != job_id || job.status != JobStatus::ReviewReady {
            return Err("검토 준비가 끝난 현재 작업에서만 영상을 재생할 수 있습니다.".into());
        }
        let candidate = job
            .candidates
            .iter()
            .find(|candidate| candidate.id == candidate_id)
            .cloned()
            .ok_or_else(|| "후보를 찾을 수 없습니다.".to_string())?;
        let source_path = job
            .acquired_media_path
            .clone()
            .unwrap_or_else(|| job.source_label.clone());
        (source_path, candidate, job.media_duration_seconds)
    };

    let source = PathBuf::from(source_path);
    if !source.is_file() {
        return Err("후보의 원본 영상 파일을 찾을 수 없습니다.".into());
    }
    let source_fingerprint = source_fingerprint(&source)
        .map_err(|error| format!("후보 원본 fingerprint를 계산하지 못했습니다: {error}"))?
        .0;
    let tools = locate_tools(&state.resource_dir).map_err(|error| match error {
        PipelineError::Cancelled => "미리보기 준비가 취소됐습니다.".to_string(),
        PipelineError::ResourceLimit { reason, .. } => format!("자원 제한 초과: {reason}"),
        PipelineError::Message(message) => message,
    })?;
    let preview_dir = state.store.job_dir(job_id).join("review-clips");
    fs::create_dir_all(&preview_dir).map_err(|error| error.to_string())?;
    let (context_start, context_end) = if preview_kind == PreviewKind::Context
        && candidate.context_end_seconds > candidate.context_start_seconds
    {
        (
            candidate.context_start_seconds,
            candidate.context_end_seconds,
        )
    } else {
        context_bounds(
            candidate.start_seconds,
            candidate.end_seconds,
            candidate
                .context_end_seconds
                .max(media_duration_seconds.unwrap_or(candidate.end_seconds as f64)),
        )
    };
    let (clip_start, clip_end, source_start, source_end) = match preview_kind {
        PreviewKind::Candidate => (
            candidate.start_seconds.saturating_sub(2) as f64,
            candidate.end_seconds.saturating_add(2) as f64,
            candidate.start_seconds,
            candidate.end_seconds,
        ),
        PreviewKind::Context => (
            context_start,
            context_end,
            context_start.round().max(0.0) as u32,
            context_end.round().max(0.0) as u32,
        ),
    };
    let cache_key = preview_cache_key(
        job_id,
        candidate_id,
        &source_fingerprint,
        context_start,
        context_end,
        preview_kind,
    );
    let output = preview_dir.join(preview_output_name(&cache_key, preview_kind));
    let clip_duration = (clip_end - clip_start).max(1.0);

    if !output.is_file() || fs::metadata(&output).map(|value| value.len()).unwrap_or(0) < 1024 {
        let temporary = preview_temporary_path(&output);
        fs::remove_file(&temporary).ok();
        let preview_started = Instant::now();
        // Preview FFmpeg must honor the same job cancel flag as analysis tools.
        let result = run_command(
            &state.cancel_requested,
            &tools.ffmpeg,
            tools.ffmpeg_dir.as_path(),
            [
                "-hide_banner".into(),
                "-loglevel".into(),
                "error".into(),
                "-y".into(),
                "-ss".into(),
                format!("{clip_start:.3}").into(),
                "-protocol_whitelist".into(),
                "file,crypto,data".into(),
                "-i".into(),
                source.as_os_str().into(),
                "-t".into(),
                format!("{clip_duration:.3}").into(),
                "-map".into(),
                "0:v:0".into(),
                "-map".into(),
                "0:a:0?".into(),
                "-sn".into(),
                "-dn".into(),
                "-vf".into(),
                "scale=1280:720:force_original_aspect_ratio=decrease:force_divisible_by=2".into(),
                "-r".into(),
                "30".into(),
                "-c:v".into(),
                "libopenh264".into(),
                "-b:v".into(),
                "2200k".into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                "128k".into(),
                "-movflags".into(),
                "+faststart".into(),
                temporary.as_os_str().into(),
            ],
            &preview_dir.join(format!("{}.stdout.log", cache_key_hash(&cache_key))),
            &preview_dir.join(format!("{}.stderr.log", cache_key_hash(&cache_key))),
        );
        if let Err(error) = result {
            fs::remove_file(&temporary).ok();
            return Err(match error {
                PipelineError::Cancelled => "미리보기 준비가 취소됐습니다.".into(),
                PipelineError::ResourceLimit { reason, .. } => format!("미리보기 준비를 중단했습니다: {reason}"),
                PipelineError::Message(message) => {
                    format!("후보 영상을 준비하지 못했습니다: {message}")
                }
            });
        }
        if let Err(error) = record_stage_metric(
            app,
            state,
            ResourceStage::Preview,
            preview_started,
            ResourceSample { external_tool_count: Some(1), ..Default::default() },
        ) {
            fs::remove_file(&temporary).ok();
            return Err(format!("후보 영상을 준비하지 못했습니다: {error}"));
        }
        fs::remove_file(&output).ok();
        fs::rename(&temporary, &output).map_err(|error| error.to_string())?;
    }

    Ok(PreviewMedia {
        path: output.display().to_string(),
        clip_start_seconds: clip_start,
        source_start_seconds: source_start,
        source_end_seconds: source_end,
        preview_kind,
    })
}

fn cache_key_hash(cache_key: &str) -> String {
    format!("{:x}", Sha256::digest(cache_key.as_bytes()))
}

fn run<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &Arc<AppState>,
    job_id: &str,
) -> Result<Vec<Candidate>, PipelineError> {
    let (
        source_path,
        completed_units,
        analysis_mode,
        requested_start,
        requested_end,
        source_kind,
        source_url,
        whisper_settings,
    ) = {
        let guard = state
            .job
            .lock()
            .map_err(|_| PipelineError::Message("작업 상태 잠금이 손상됐습니다.".into()))?;
        let job = guard
            .as_ref()
            .ok_or_else(|| PipelineError::Message("현재 작업이 없습니다.".into()))?;
        if job.id != job_id {
            return Err(PipelineError::Message(
                "실행할 작업이 현재 작업과 다릅니다.".into(),
            ));
        }
        (
            job.acquired_media_path
                .clone()
                .unwrap_or_else(|| job.source_label.clone()),
            job.completed_units,
            job.analysis_mode,
            job.analysis_start_seconds,
            job.analysis_end_seconds,
            job.source_kind,
            job.source_label.clone(),
            job.whisper.clone().normalized(),
        )
    };

    let source = PathBuf::from(&source_path);
    if !source.is_file() {
        return Err(PipelineError::Message(
            "선택한 영상 파일을 찾을 수 없습니다.".into(),
        ));
    }
    let tools = locate_tools(&state.resource_dir)?;
    let job_dir = state.store.job_dir(job_id);
    fs::create_dir_all(&job_dir)?;
    let log_dir = job_dir.join("tool-logs");
    fs::create_dir_all(&log_dir)?;
    let whisper_budget_path = job_dir.join("whisper-budget.json");

    if completed_units == 0 {
        progress(
            app,
            state,
            1,
            JobStatus::Acquiring,
            "로컬 도구 확인",
            "내장 FFmpeg와 Whisper 모델을 확인했습니다.",
        )?;
    }
    check_cancel(state)?;

    let (input_fingerprint, input_bytes) =
        source_fingerprint(&source).map_err(PipelineError::Message)?;
    let runtime_sha256 = runtime_hashes().map_err(PipelineError::Message)?;

    let checkpoint_path = job_dir.join("media-checkpoint.json");
    let mut caption_artifacts = if source_kind == SourceKind::Youtube {
        captions::read_provenance_with_diagnostics(&job_dir).map_err(PipelineError::Message)?
    } else {
        None
    };
    if source_kind == SourceKind::Youtube {
        if let Some((provenance, _)) = caption_artifacts.as_mut() {
            if provenance.source_url != source_url {
                provenance.verification_state = VerificationState::Failed;
                provenance.diagnostics.push(captions::CaptionDiagnostic {
                    kind: captions::CaptionDiagnosticKind::ProvenanceInvalid,
                    interval_index: None,
                    start_seconds: None,
                    end_seconds: None,
                    detail: "자막 provenance가 현재 YouTube 영상과 일치하지 않습니다.".into(),
                });
            }
        }
    }
    let caption_provenance = caption_artifacts.as_ref().map(|(provenance, _)| provenance);
    let mut checkpoint = load_checkpoint_with_caption(
        &checkpoint_path,
        &source_path,
        analysis_mode,
        requested_start,
        requested_end,
        &input_fingerprint,
        input_bytes,
        &runtime_sha256,
        caption_provenance,
        &whisper_settings,
    )?;
    // load_checkpoint returns None for missing, corrupt, or incompatible (schema/fingerprint/tools/ranker).
    // In those cases we rebuild a fresh media checkpoint and must recompute intermediates only.
    let mut media_intermediates_rebuilt = false;
    if checkpoint.is_none() {
        media_intermediates_rebuilt = true;
        let probe_json = job_dir.join("ffprobe.json");
        run_command(
            &state.cancel_requested,
            &tools.ffprobe,
            tools.ffmpeg_dir.as_path(),
            [
                "-v".into(),
                "error".into(),
                "-show_entries".into(),
                "format=duration:stream=codec_type".into(),
                "-of".into(),
                "json".into(),
                "-protocol_whitelist".into(),
                "file,crypto,data".into(),
                source.as_os_str().into(),
            ],
            &probe_json,
            &log_dir.join("ffprobe.stderr.log"),
        )?;
        let probe: ProbeOutput = serde_json::from_slice(&fs::read(&probe_json)?)?;
        if !probe
            .streams
            .iter()
            .any(|stream| stream.codec_type.as_deref() == Some("audio"))
        {
            return Err(PipelineError::Message(
                "영상에 분석할 오디오 스트림이 없습니다.".into(),
            ));
        }
        let duration = probe
            .format
            .duration
            .as_deref()
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or_else(|| {
                PipelineError::Message("ffprobe가 영상 길이를 반환하지 않았습니다.".into())
            })?;
        let (analysis_start, analysis_end) =
            analysis_bounds(duration, analysis_mode, requested_start, requested_end)?;
        let planned_chunks = build_analysis_plan(analysis_mode, analysis_start, analysis_end);
        checkpoint = Some(MediaCheckpoint::fresh(
            &source_path,
            duration,
            analysis_mode,
            analysis_start,
            analysis_end,
            planned_chunks,
            input_fingerprint.clone(),
            input_bytes,
            runtime_sha256.clone(),
        ));
        if let Some(checkpoint) = checkpoint.as_mut() {
            checkpoint.whisper_settings = whisper_settings.clone();
        }
        if let (Some(checkpoint), Some(provenance)) = (checkpoint.as_mut(), caption_provenance) {
            apply_caption_identity(checkpoint, provenance);
        }
    }
    let mut checkpoint = checkpoint.expect("checkpoint initialized");
    checkpoint.schema_version = MEDIA_CHECKPOINT_SCHEMA;
    checkpoint.whisper_settings = whisper_settings.clone();
    let chunk_count = checkpoint.planned_chunks.len().max(1) as u32;
    let total_units = chunk_count + 6;

    // Block analysis start when free space cannot cover active WAV, checkpoint growth,
    // chat-motion temp frames, and preview headroom for the selected source.
    ensure_analysis_disk_space(&job_dir, input_bytes, checkpoint.duration_seconds)?;

    let align = align_checkpoint_with_job_units(
        &mut checkpoint,
        completed_units,
        media_intermediates_rebuilt,
    )
    .map_err(PipelineError::Message)?;

    // After an incompatible discard, clamp job media progress to the rebuilt checkpoint so
    // sequential progress updates work and a later mid-recompute resume does not hard-fail.
    // Job id, source, and analysis settings are preserved; user data is not deleted.
    let mut completed_units = completed_units;
    if align == CheckpointAlignResult::RestartMediaFromScratch {
        completed_units = job_units_after_media_restart(&checkpoint);
        let duration = checkpoint.duration_seconds;
        let _ = mutate_job(app, state, |job| {
            job.total_units = total_units;
            job.media_duration_seconds = Some(duration);
            if job.completed_units > completed_units {
                job.completed_units = completed_units;
                job.candidates.clear();
                job.current_stage_label = format!("미디어 확인 · {}개 청크", chunk_count);
                job.error_message = None;
                job.error_detail = None;
                // Reverse media restart is not a normal forward transition (e.g. Transcribing→Probing).
                job.status = JobStatus::Probing;
                job.push_activity(
                    "progress",
                    "호환되지 않는 미디어 중간 결과를 버리고 음성 인식부터 다시 계산합니다.",
                );
            }
            Ok(())
        })
        .map_err(PipelineError::Message)?;
    }

    if completed_units < 2 {
        let duration = checkpoint.duration_seconds;
        let _ = mutate_job(app, state, |job| {
            job.total_units = total_units;
            job.media_duration_seconds = Some(duration);
            job.apply_progress(
                2,
                JobStatus::Probing,
                format!("미디어 확인 · {}개 청크", chunk_count),
                format!(
                    "{} 범위 {}초를 음성 인식 청크 {}개로 계획했습니다.",
                    analysis_mode.label(),
                    checkpoint
                        .planned_chunks
                        .iter()
                        .map(|chunk| chunk.length_seconds)
                        .sum::<f64>()
                        .round() as u32,
                    chunk_count,
                ),
            )
        })
        .map_err(PipelineError::Message)?;
    } else if align != CheckpointAlignResult::RestartMediaFromScratch {
        let duration = checkpoint.duration_seconds;
        let _ = mutate_job(app, state, |job| {
            job.total_units = total_units;
            job.media_duration_seconds = Some(duration);
            Ok(())
        })
        .map_err(PipelineError::Message)?;
    }
    persist_whisper_state(&checkpoint_path, &whisper_budget_path, &checkpoint)?;
    let caption_plan = caption_plan_for_duration(
        caption_artifacts.as_ref(),
        checkpoint.duration_seconds,
    );
    if let Some((provenance, _)) = caption_artifacts.as_ref() {
        let failed_provenance = provenance.verification_state == VerificationState::Failed;
        let quality = if failed_provenance {
            "failed"
        } else if caption_plan.full_whisper {
            "unverified"
        } else if caption_plan.fallback.is_empty() {
            "trusted"
        } else {
            "mixed"
        };
        let mut caption_diagnostics = provenance.diagnostics.clone();
        for diagnostic in &caption_plan.diagnostics {
            if !caption_diagnostics.contains(diagnostic) {
                caption_diagnostics.push(diagnostic.clone());
            }
        }
        let _ = mutate_job(app, state, |job| {
            job.captions = Some(crate::domain::CaptionSummary {
                source: Some(provenance.source),
                language: (!failed_provenance).then(|| provenance.language.clone()),
                quality: quality.into(),
                fallback_intervals: caption_plan.fallback.len() as u32,
                local_whisper_fallback: caption_plan.full_whisper || !caption_plan.fallback.is_empty(),
                diagnostics: caption_diagnostics
                    .iter()
                    .map(|diagnostic| crate::domain::CaptionDiagnosticSummary {
                        kind: format!("{:?}", diagnostic.kind),
                        interval_index: diagnostic.interval_index,
                        start_seconds: diagnostic.start_seconds,
                        end_seconds: diagnostic.end_seconds,
                        detail: diagnostic.detail.clone(),
                    })
                    .collect(),
                provenance: Some(crate::domain::CaptionProvenanceSummary {
                    original_file: provenance.original_file.clone(),
                    language: (!failed_provenance).then(|| provenance.language.clone()),
                    track_id: provenance.track_id.clone(),
                    sha256: provenance.sha256.clone(),
                    revision: provenance.revision.clone(),
                    verification_state: provenance.verification_state,
                }),
            });
            Ok(())
        });
    } else if source_kind == SourceKind::Youtube {
        let _ = mutate_job(app, state, |job| {
            job.captions = Some(crate::domain::CaptionSummary {
                source: None,
                language: Some(TRANSCRIPTION_LANGUAGE.into()),
                quality: "unavailable".into(),
                fallback_intervals: 1,
                local_whisper_fallback: true,
                diagnostics: vec![crate::domain::CaptionDiagnosticSummary {
                    kind: "CaptionUnavailable".into(),
                    interval_index: None,
                    start_seconds: None,
                    end_seconds: None,
                    detail: "한국어 자막을 사용할 수 없어 로컬 Whisper로 전체 구간을 확인합니다.".into(),
                }],
                provenance: None,
            });
            Ok(())
        });
    }

    for chunk_index in checkpoint.completed_chunks..chunk_count {
        check_cancel(state)?;
        let planned = checkpoint.planned_chunks[chunk_index as usize].clone();
        let offset = planned.offset_seconds;
        let length = planned.length_seconds.max(0.1);
        let wav = job_dir.join("active-chunk.wav");
        fs::remove_file(&wav).ok();

        let ffmpeg_audio_started = Instant::now();
        run_command(
            &state.cancel_requested,
            &tools.ffmpeg,
            tools.ffmpeg_dir.as_path(),
            [
                "-hide_banner".into(),
                "-loglevel".into(),
                "error".into(),
                "-y".into(),
                "-ss".into(),
                format!("{offset:.3}").into(),
                "-t".into(),
                format!("{length:.3}").into(),
                "-protocol_whitelist".into(),
                "file,crypto,data".into(),
                "-i".into(),
                source.as_os_str().into(),
                "-vn".into(),
                "-ac".into(),
                "1".into(),
                "-ar".into(),
                "16000".into(),
                "-c:a".into(),
                "pcm_s16le".into(),
                wav.as_os_str().into(),
            ],
            &log_dir.join(format!("ffmpeg-{chunk_index:04}.stdout.log")),
            &log_dir.join(format!("ffmpeg-{chunk_index:04}.stderr.log")),
        )?;
        persist_stage_metric(app, state, ResourceStage::FfmpegAudio, ffmpeg_audio_started)?;

        let mut energy = analyze_wav(&wav, offset)?;
        let (trusted_intervals, fallback_ranges) = partition_caption_chunk(
            &caption_plan,
            &checkpoint.planned_chunks,
            chunk_index as usize,
            checkpoint.analysis_start_seconds,
            checkpoint.analysis_end_seconds,
        );
        let trusted_segments = trusted_intervals
            .into_iter()
            .map(|interval| TranscriptSegment {
                start_seconds: interval.start_seconds,
                end_seconds: interval.end_seconds,
                text: interval.text,
                quality_status: TranscriptQualityStatus::Certain,
                quality_reasons: Vec::new(),
            })
            .collect::<Vec<_>>();
        let threads = whisper::effective_cpu_threads(
            &whisper_settings,
            thread::available_parallelism().map(|count| count.get()).unwrap_or(4),
        );
        let mut segments = trusted_segments;
        for (fallback_index, (fallback_start, fallback_end)) in fallback_ranges.iter().enumerate() {
            check_cancel(state)?;
            let fallback_length = (fallback_end - fallback_start).max(0.1);
            let uses_full_chunk = (*fallback_start - offset).abs() < 0.001
                && (*fallback_end - (offset + length)).abs() < 0.001;
            let whisper_wav = if uses_full_chunk {
                wav.clone()
            } else {
                let path = job_dir.join(format!("active-fallback-{chunk_index:04}-{fallback_index:02}.wav"));
                fs::remove_file(&path).ok();
                run_command(
                    &state.cancel_requested,
                    &tools.ffmpeg,
                    tools.ffmpeg_dir.as_path(),
                    [
                        "-hide_banner".into(),
                        "-loglevel".into(),
                        "error".into(),
                        "-y".into(),
                        "-ss".into(),
                        format!("{fallback_start:.3}").into(),
                        "-t".into(),
                        format!("{fallback_length:.3}").into(),
                        "-protocol_whitelist".into(),
                        "file,crypto,data".into(),
                        "-i".into(),
                        source.as_os_str().into(),
                        "-vn".into(),
                        "-ac".into(),
                        "1".into(),
                        "-ar".into(),
                        "16000".into(),
                        "-c:a".into(),
                        "pcm_s16le".into(),
                        path.as_os_str().into(),
                    ],
                    &log_dir.join(format!("ffmpeg-fallback-{chunk_index:04}-{fallback_index:02}.stdout.log")),
                    &log_dir.join(format!("ffmpeg-fallback-{chunk_index:04}-{fallback_index:02}.stderr.log")),
                )?;
                path
            };
            let whisper_prefix = job_dir.join(format!("active-transcript-{chunk_index:04}-{fallback_index:02}"));
            let whisper_srt = whisper_prefix.with_extension("srt");
            fs::remove_file(&whisper_srt).ok();
            let state_index = checkpoint
                .whisper_units
                .iter()
                .position(|unit| unit.chunk_index == chunk_index && unit.fallback_index == fallback_index as u32);
            let state_index = state_index.unwrap_or_else(|| {
                checkpoint.whisper_units.push(WhisperUnitState {
                    chunk_index: chunk_index,
                    fallback_index: fallback_index as u32,
                    device: WhisperDeviceMode::Cpu,
                    model: MODEL_NAME.into(),
                    profile: whisper_settings.profile,
                    cpu_threads: whisper_settings.cpu_threads,
                    duration_ms: None,
                    gpu_failure_reason: None,
                    gpu: Default::default(),
                    cpu_fallback: Default::default(),
                });
                checkpoint.whisper_units.len() - 1
            });
            if !whisper::should_try_cpu(&checkpoint.whisper_units[state_index]) {
                return Err(PipelineError::Message(
                    "이 구간의 CPU 음성 인식이 이미 실패해 자동 재시도하지 않습니다.".into(),
                ));
            }
            let gpu_allowed = whisper::should_try_gpu(
                &whisper_settings,
                &checkpoint.whisper_units[state_index],
            );
            let mut fallback_segments = None;
            let probe_wav = job_dir.join(format!("active-gpu-probe-{chunk_index:04}-{fallback_index:02}.wav"));
            let probe_prefix = job_dir.join(format!("active-gpu-probe-{chunk_index:04}-{fallback_index:02}"));
            if gpu_allowed {
                update_whisper_runtime(
                    app,
                    state,
                    WhisperRuntimeStatus::Testing,
                    chunk_index,
                    threads,
                    None,
                )?;
                mutate_job(app, state, |job| {
                    job.current_stage_label = "GPU 시험·음성 인식 중".into();
                    job.push_activity("whisper", "짧은 실제 Whisper 실행에서 GPU 백엔드와 음성 인식 결과를 확인합니다.");
                    Ok(())
                }).map_err(PipelineError::Message)?;
                let unit = &mut checkpoint.whisper_units[state_index];
                unit.device = WhisperDeviceMode::Gpu;
                unit.model = MODEL_NAME.into();
                unit.profile = whisper_settings.profile;
                unit.cpu_threads = Some(threads as u16);
                unit.gpu.status = WhisperAttemptStatus::Started;
                unit.gpu.started_at = Some(Utc::now().to_rfc3339());
                persist_whisper_state(&checkpoint_path, &whisper_budget_path, &checkpoint)?;
                let started = Instant::now();
                fs::remove_file(&probe_wav).ok();
                fs::remove_file(probe_prefix.with_extension("srt")).ok();
                let gpu_result = tools.whisper_gpu.as_ref().ok_or_else(|| {
                    PipelineError::Message("GPU 런타임을 찾지 못했습니다.".into())
                }).and_then(|gpu| {
                    run_gpu_probe(
                        &state.cancel_requested,
                        &tools.ffmpeg,
                        &tools.ffmpeg_dir,
                        gpu,
                        &tools.whisper_gpu_dir,
                        &tools.model,
                        &whisper_wav,
                        &probe_wav,
                        &probe_prefix,
                        threads,
                        &whisper_settings,
                        &log_dir.join(format!("whisper-gpu-probe-{chunk_index:04}-{fallback_index:02}.stdout.log")),
                        &log_dir.join(format!("whisper-gpu-probe-{chunk_index:04}-{fallback_index:02}.stderr.log")),
                    )?;
                    // The actual unit run remains the source of transcript output; the
                    // preceding three-second probe only establishes backend evidence.
                    run_whisper_attempt(
                        &state.cancel_requested,
                        gpu,
                        &tools.whisper_gpu_dir,
                        &tools.model,
                        &whisper_wav,
                        &whisper_prefix,
                        *fallback_start,
                        threads,
                        &whisper_settings,
                        true,
                        &log_dir.join(format!("whisper-gpu-{chunk_index:04}-{fallback_index:02}.stdout.log")),
                        &log_dir.join(format!("whisper-gpu-{chunk_index:04}-{fallback_index:02}.stderr.log")),
                    )
                });
                let duration_ms = started.elapsed().as_millis() as u64;
                persist_stage_metric(app, state, ResourceStage::Whisper, started)?;
                match gpu_result {
                    Ok(result) => {
                        let unit = &mut checkpoint.whisper_units[state_index];
                        unit.device = WhisperDeviceMode::Gpu;
                        unit.duration_ms = Some(duration_ms);
                        unit.gpu.duration_ms = Some(duration_ms);
                        unit.gpu.status = WhisperAttemptStatus::Completed;
                        unit.gpu.completed_at = Some(Utc::now().to_rfc3339());
                        persist_whisper_state(&checkpoint_path, &whisper_budget_path, &checkpoint)?;
                        update_whisper_runtime(
                            app,
                            state,
                            WhisperRuntimeStatus::Gpu,
                            chunk_index,
                            threads,
                            None,
                        )?;
                        mutate_job(app, state, |job| {
                            job.current_stage_label = "GPU 사용 중".into();
                            job.push_activity("whisper", "GPU 백엔드 로드와 비어 있지 않은 음성 인식 결과를 확인했습니다.");
                            Ok(())
                        }).map_err(PipelineError::Message)?;
                        fallback_segments = Some(result);
                    }
                    Err(PipelineError::Cancelled) => return Err(PipelineError::Cancelled),
                    Err(PipelineError::ResourceLimit { stage, reason }) => {
                        return Err(PipelineError::ResourceLimit { stage, reason });
                    }
                    Err(PipelineError::Message(reason)) => {
                        let unit = &mut checkpoint.whisper_units[state_index];
                        unit.duration_ms = Some(duration_ms);
                        unit.gpu.duration_ms = Some(duration_ms);
                        unit.gpu.status = WhisperAttemptStatus::Failed;
                        unit.gpu.completed_at = Some(Utc::now().to_rfc3339());
                        let safe_reason = sanitize_gpu_failure_reason(&reason);
                        unit.gpu.failure_reason = Some(safe_reason.clone());
                        unit.gpu_failure_reason = Some(safe_reason.clone());
                        persist_whisper_state(&checkpoint_path, &whisper_budget_path, &checkpoint)?;
                        update_whisper_runtime(
                            app,
                            state,
                            WhisperRuntimeStatus::CpuFallback,
                            chunk_index,
                            threads,
                            Some(safe_reason),
                        )?;
                        mutate_job(app, state, |job| {
                            job.current_stage_label = "CPU 대체 처리 중".into();
                            job.push_activity("whisper", "GPU 실행에 실패해 이 구간만 CPU로 한 번 대체합니다.");
                            Ok(())
                        }).map_err(PipelineError::Message)?;
                    }
                }
            }
            if fallback_segments.is_none() {
                let runtime_status = if matches!(whisper_settings.device_mode, WhisperDeviceMode::Cpu) {
                    WhisperRuntimeStatus::Cpu
                } else {
                    WhisperRuntimeStatus::CpuFallback
                };
                update_whisper_runtime(
                    app,
                    state,
                    runtime_status,
                    chunk_index,
                    threads,
                    checkpoint.whisper_units[state_index].gpu_failure_reason.clone(),
                )?;
                let unit = &mut checkpoint.whisper_units[state_index];
                unit.device = WhisperDeviceMode::Cpu;
                unit.model = MODEL_NAME.into();
                unit.profile = whisper_settings.profile;
                unit.cpu_threads = Some(threads as u16);
                unit.cpu_fallback.status = WhisperAttemptStatus::Started;
                unit.cpu_fallback.started_at = Some(Utc::now().to_rfc3339());
                persist_whisper_state(&checkpoint_path, &whisper_budget_path, &checkpoint)?;
                mutate_job(app, state, |job| {
                    if matches!(whisper_settings.device_mode, WhisperDeviceMode::Cpu) {
                        job.current_stage_label = "CPU 음성 인식 중".into();
                        job.push_activity("whisper", "CPU 모드로 실행합니다. GPU를 사용하지 않습니다.");
                    } else {
                        job.current_stage_label = "CPU 대체 처리 중".into();
                        job.push_activity("whisper", "GPU 실패 구간을 CPU로 처리합니다. 이 구간의 자동 전환은 한 번만 허용됩니다.");
                    }
                    Ok(())
                }).map_err(PipelineError::Message)?;
                fs::remove_file(&whisper_srt).ok();
                let started = Instant::now();
                let cpu_result = run_whisper_attempt(
                    &state.cancel_requested,
                    &tools.whisper,
                    &tools.whisper_dir,
                    &tools.model,
                    &whisper_wav,
                    &whisper_prefix,
                    *fallback_start,
                    threads,
                    &whisper_settings,
                    false,
                    &log_dir.join(format!("whisper-{chunk_index:04}-{fallback_index:02}.stdout.log")),
                    &log_dir.join(format!("whisper-{chunk_index:04}-{fallback_index:02}.stderr.log")),
                );
                let duration_ms = started.elapsed().as_millis() as u64;
                persist_stage_metric(app, state, ResourceStage::Whisper, started)?;
                match cpu_result {
                    Ok(result) => {
                        let unit = &mut checkpoint.whisper_units[state_index];
                        unit.duration_ms = Some(duration_ms);
                        unit.cpu_fallback.duration_ms = Some(duration_ms);
                        unit.cpu_fallback.status = WhisperAttemptStatus::Completed;
                        unit.cpu_fallback.completed_at = Some(Utc::now().to_rfc3339());
                        persist_whisper_state(&checkpoint_path, &whisper_budget_path, &checkpoint)?;
                        update_whisper_runtime(
                            app,
                            state,
                            runtime_status,
                            chunk_index,
                            threads,
                            checkpoint.whisper_units[state_index].gpu_failure_reason.clone(),
                        )?;
                        fallback_segments = Some(result);
                    }
                    Err(PipelineError::Cancelled) => return Err(PipelineError::Cancelled),
                    Err(PipelineError::ResourceLimit { stage, reason }) => {
                        return Err(PipelineError::ResourceLimit { stage, reason });
                    }
                    Err(PipelineError::Message(reason)) => {
                        let gpu_failure_reason = {
                            let unit = &mut checkpoint.whisper_units[state_index];
                            unit.duration_ms = Some(duration_ms);
                            unit.cpu_fallback.duration_ms = Some(duration_ms);
                            unit.cpu_fallback.status = WhisperAttemptStatus::Failed;
                            unit.cpu_fallback.completed_at = Some(Utc::now().to_rfc3339());
                            unit.cpu_fallback.failure_reason =
                                Some("CPU 음성 인식에 실패했습니다.".into());
                            unit.gpu_failure_reason.clone()
                        };
                        persist_whisper_state(&checkpoint_path, &whisper_budget_path, &checkpoint)?;
                        update_whisper_runtime(
                            app,
                            state,
                            WhisperRuntimeStatus::Failed,
                            chunk_index,
                            threads,
                            gpu_failure_reason,
                        )?;
                        return Err(PipelineError::Message(format!("CPU 음성 인식 실패: {reason}")));
                    }
                }
            }
            fs::remove_file(&probe_wav).ok();
            fs::remove_file(probe_prefix.with_extension("srt")).ok();
            let mut fallback_segments = fallback_segments.expect("Whisper attempt produced no result");
            clip_segments_to_range(&mut fallback_segments, *fallback_start, *fallback_end);
            segments.append(&mut fallback_segments);
            if !uses_full_chunk {
                fs::remove_file(&whisper_wav).ok();
            }
            fs::remove_file(&whisper_srt).ok();
        }
        segments.sort_by(|left, right| {
            left.start_seconds
                .total_cmp(&right.start_seconds)
                .then_with(|| left.end_seconds.total_cmp(&right.end_seconds))
        });
        let mut segments = sanitize_transcript_segments(segments);
        checkpoint.segments.append(&mut segments);
        checkpoint.segments = sanitize_transcript_segments(std::mem::take(&mut checkpoint.segments));
        checkpoint.energy.append(&mut energy);
        checkpoint.completed_chunks = chunk_index + 1;
        persist_whisper_state(&checkpoint_path, &whisper_budget_path, &checkpoint)?;
        write_transcript(&job_dir.join("transcript.json"), &checkpoint.segments)?;
        fs::remove_file(&wav).ok();

        progress(
            app,
            state,
            chunk_index + 3,
            JobStatus::Transcribing,
            &format!("음성 인식 {}/{}", chunk_index + 1, chunk_count),
            &format!(
                "오디오 청크 {}/{}를 추출하고 음성 인식했습니다.",
                chunk_index + 1,
                chunk_count
            ),
        )?;
    }

    checkpoint.segments = sanitize_transcript_segments(std::mem::take(&mut checkpoint.segments));
    checkpoint.schema_version = MEDIA_CHECKPOINT_SCHEMA;
    checkpoint.language = TRANSCRIPTION_LANGUAGE.into();
    checkpoint.ranker_version = RANKER_VERSION.into();
    persist_whisper_state(&checkpoint_path, &whisper_budget_path, &checkpoint)?;
    write_transcript(&job_dir.join("transcript.json"), &checkpoint.segments)?;
    write_pipeline_provenance(&job_dir, &source, &checkpoint)?;

    progress(
        app,
        state,
        chunk_count + 3,
        JobStatus::AudioSignals,
        "오디오 반응 계산",
        "1초 단위 음량과 발화 밀도를 계산했습니다.",
    )?;

    if !checkpoint.chat_motion_completed {
        let motion_raw = job_dir.join("chat-motion.raw");
        fs::remove_file(&motion_raw).ok();
        let (chat_start, chat_length, chat_sample_seconds) = match checkpoint.analysis_mode {
            AnalysisMode::Range => (
                checkpoint.analysis_start_seconds as f64,
                (checkpoint.analysis_end_seconds - checkpoint.analysis_start_seconds) as f64,
                CHAT_SAMPLE_SECONDS,
            ),
            AnalysisMode::Quick => (0.0, checkpoint.duration_seconds, QUICK_CHAT_SAMPLE_SECONDS),
            AnalysisMode::Full => (0.0, checkpoint.duration_seconds, CHAT_SAMPLE_SECONDS),
        };
        let chat_started = Instant::now();
        let motion_result = analyze_chat_motion(
            &state.cancel_requested,
            &tools.ffmpeg,
            tools.ffmpeg_dir.as_path(),
            &source,
            chat_start,
            chat_length,
            chat_sample_seconds,
            &motion_raw,
            &log_dir.join("chat-motion.stderr.log"),
        );
        persist_stage_metric(app, state, ResourceStage::ChatDecode, chat_started)?;
        match motion_result {
            Ok(points) => checkpoint.chat_motion = points,
            Err(PipelineError::Cancelled) => return Err(PipelineError::Cancelled),
            Err(PipelineError::ResourceLimit { stage, reason }) => {
                return Err(PipelineError::ResourceLimit { stage, reason });
            }
            Err(PipelineError::Message(detail)) => {
                checkpoint.chat_motion.clear();
                let _ = mutate_job(app, state, |job| {
                    job.push_activity(
                        "diagnostic",
                        "채팅 영역 움직임을 측정하지 못해 오디오·발화 신호만 사용합니다.",
                    );
                    job.error_detail = Some(detail.clone());
                    Ok(())
                });
            }
        }
        checkpoint.chat_motion_completed = true;
        save_checkpoint(&checkpoint_path, &checkpoint)?;
        write_chat_motion(&job_dir.join("chat-motion.json"), &checkpoint.chat_motion)?;
        fs::remove_file(&motion_raw).ok();
    }
    progress(
        app,
        state,
        chunk_count + 4,
        JobStatus::ChatSignals,
        "채팅 움직임 계산",
        if checkpoint.chat_motion.is_empty() {
            "채팅 움직임 신호 없이 오디오와 발화 밀도로 계속합니다."
        } else {
            if checkpoint.analysis_mode == AnalysisMode::Quick {
                "화면 오른쪽 영역을 15초 간격으로 빠르게 비교해 활발한 구간을 찾았습니다."
            } else {
                "화면 오른쪽 영역을 5초 간격으로 비교해 활발한 구간을 찾았습니다."
            }
        },
    )?;
    let candidates = build_candidates(
        checkpoint.duration_seconds,
        checkpoint.analysis_start_seconds as f64,
        checkpoint.analysis_end_seconds as f64,
        &checkpoint.segments,
        &checkpoint.energy,
        &checkpoint.chat_motion,
    );
    progress(
        app,
        state,
        chunk_count + 5,
        JobStatus::Fusing,
        "후보 구간 결합",
        "겹치는 반응 구간을 묶고 가까운 중복 후보를 제거했습니다.",
    )?;
    progress(
        app,
        state,
        chunk_count + 6,
        JobStatus::Ranking,
        "후보 순위 결정",
        if checkpoint.chat_motion.is_empty() {
            "오디오 55%, 발화 밀도 45% 규칙으로 후보 순위를 정했습니다."
        } else {
            "오디오 45%, 발화 밀도 35%, 채팅 움직임 20% 규칙으로 후보 순위를 정했습니다."
        },
    )?;
    Ok(candidates)
}

struct ToolPaths {
    ffmpeg_dir: PathBuf,
    whisper_dir: PathBuf,
    whisper_gpu_dir: PathBuf,
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
    whisper: PathBuf,
    whisper_gpu: Option<PathBuf>,
    model: PathBuf,
}

fn whisper_command_args(
    settings: &WhisperSettings,
    model: &Path,
    wav: &Path,
    prefix: &Path,
    threads: usize,
    gpu: bool,
) -> Vec<std::ffi::OsString> {
    let mut args = vec![
        "-m".into(), model.as_os_str().into(),
        "-f".into(), wav.as_os_str().into(),
        "-l".into(), "ko".into(),
        "-nth".into(), "0.72".into(),
        "-nf".into(), "-sns".into(), "-sow".into(), "-osrt".into(),
        "-of".into(), prefix.as_os_str().into(), "-np".into(),
    ];
    if !gpu {
        // CPU mode must explicitly disable CUDA even when a GPU runtime exists.
        args.push("-ng".into());
    }
    for flag in whisper::profile_args(settings.profile) {
        args.push((*flag).into());
    }
    args.extend(["-t".into(), threads.to_string().into()]);
    args
}

fn has_gpu_backend_evidence(stdout: &str, stderr: &str) -> bool {
    let logs = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    if logs.contains("cuda error")
        || logs.contains("no cuda")
        || logs.contains("cuda device count: 0")
        || logs.contains("failed to")
    {
        return false;
    }
    let tokens = logs
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    if tokens
        .iter()
        .any(|token| matches!(*token, "error" | "errors" | "failed" | "failure"))
    {
        return false;
    }
    let found_positive_device = tokens.windows(4).any(|window| {
        window[0] == "found"
            && window[1].parse::<u32>().is_ok_and(|count| count > 0)
            && window[2] == "cuda"
            && window[3].starts_with("device")
    });
    let using_cuda_backend = tokens.windows(3).any(|window| {
        window[0] == "using"
            && window[1].strip_prefix("cuda").is_some_and(|suffix| {
                suffix.is_empty() || suffix.chars().all(|character| character.is_ascii_digit())
            })
            && window[2] == "backend"
    }) || logs.contains("cuda backend in use");
    found_positive_device && using_cuda_backend
}

fn sanitize_gpu_failure_reason(reason: &str) -> String {
    let lower = reason.to_ascii_lowercase();
    if lower.contains("런타임") || lower.contains("runtime") {
        "GPU 런타임을 사용할 수 없습니다.".into()
    } else if lower.contains("백엔드") || lower.contains("backend") {
        "GPU 백엔드 확인에 실패했습니다.".into()
    } else if lower.contains("비어") || lower.contains("empty") {
        "GPU 음성 인식 결과가 비어 있습니다.".into()
    } else {
        "GPU 실행에 실패했습니다.".into()
    }
}

fn update_whisper_runtime<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &AppState,
    status: WhisperRuntimeStatus,
    unit_index: u32,
    effective_cpu_threads: usize,
    gpu_failure_reason: Option<String>,
) -> Result<(), PipelineError> {
    mutate_job(app, state, |job| {
        job.whisper_runtime.status = status;
        job.whisper_runtime.unit_index = Some(unit_index);
        job.whisper_runtime.effective_cpu_threads = Some(effective_cpu_threads as u16);
        job.whisper_runtime.gpu_failure_reason = gpu_failure_reason;
        Ok(())
    })
    .map(|_| ())
    .map_err(PipelineError::Message)
}

fn run_whisper_attempt(
    cancel_requested: &AtomicBool,
    executable: &Path,
    current_dir: &Path,
    model: &Path,
    wav: &Path,
    prefix: &Path,
    offset: f64,
    threads: usize,
    settings: &WhisperSettings,
    gpu: bool,
    stdout_path: &Path,
    stderr_path: &Path,
) -> Result<Vec<TranscriptSegment>, PipelineError> {
    run_command(
        cancel_requested,
        executable,
        current_dir,
        whisper_command_args(settings, model, wav, prefix, threads, gpu),
        stdout_path,
        stderr_path,
    )?;
    let stdout = fs::read_to_string(stdout_path).unwrap_or_default();
    let stderr = fs::read_to_string(stderr_path).unwrap_or_default();
    if gpu && !has_gpu_backend_evidence(&stdout, &stderr) {
        return Err(PipelineError::Message("GPU 백엔드 로드 증거를 확인하지 못했습니다.".into()));
    }
    let srt = prefix.with_extension("srt");
    let mut segments = parse_srt(&srt, offset)?;
    if segments.is_empty() {
        return Err(PipelineError::Message("Whisper 음성 인식 결과가 비어 있습니다.".into()));
    }
    Ok(std::mem::take(&mut segments))
}

fn run_gpu_probe(
    cancel_requested: &AtomicBool,
    ffmpeg: &Path,
    ffmpeg_dir: &Path,
    gpu: &Path,
    gpu_dir: &Path,
    model: &Path,
    wav: &Path,
    probe_wav: &Path,
    prefix: &Path,
    threads: usize,
    settings: &WhisperSettings,
    stdout_path: &Path,
    stderr_path: &Path,
) -> Result<(), PipelineError> {
    run_command(
        cancel_requested,
        ffmpeg,
        ffmpeg_dir,
        [
            "-hide_banner".into(), "-loglevel".into(), "error".into(), "-y".into(),
            "-t".into(), "3".into(), "-i".into(), wav.as_os_str().into(),
            "-ac".into(), "1".into(), "-ar".into(), "16000".into(),
            "-c:a".into(), "pcm_s16le".into(), probe_wav.as_os_str().into(),
        ],
        &stdout_path.with_extension("ffmpeg.stdout.log"),
        &stderr_path.with_extension("ffmpeg.stderr.log"),
    )?;
    let probe_segments = run_whisper_attempt(
        cancel_requested,
        gpu,
        gpu_dir,
        model,
        probe_wav,
        prefix,
        0.0,
        threads,
        settings,
        true,
        stdout_path,
        stderr_path,
    )?;
    if probe_segments.is_empty() {
        return Err(PipelineError::Message("GPU 시험 결과가 비어 있습니다.".into()));
    }
    Ok(())
}

fn locate_tools(resource_dir: &Path) -> Result<ToolPaths, PipelineError> {
    let packaged = [
        resource_dir.join("resources").join("media-tools"),
        resource_dir.join("media-tools"),
    ];
    #[cfg(debug_assertions)]
    let candidates = packaged
        .into_iter()
        .chain(std::iter::once(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join("media-tools"),
        ))
        .collect::<Vec<_>>();
    #[cfg(not(debug_assertions))]
    let candidates = packaged;
    for root in candidates {
        let ffmpeg_dir = root.join("ffmpeg");
        let whisper_dir = root.join("whisper");
        let whisper_gpu_dir = root.join("whisper-gpu");
        let paths = ToolPaths {
            ffmpeg: ffmpeg_dir.join("ffmpeg.exe"),
            ffprobe: ffmpeg_dir.join("ffprobe.exe"),
            whisper: whisper_dir.join("whisper-cli.exe"),
            whisper_gpu: Some(whisper_gpu_dir.join("whisper-cli.exe"))
                .filter(|path| path.is_file()),
            model: root.join("models").join("ggml-base.bin"),
            ffmpeg_dir,
            whisper_dir,
            whisper_gpu_dir,
        };
        if paths.ffmpeg.is_file()
            && paths.ffprobe.is_file()
            && paths.whisper.is_file()
            && paths.model.is_file()
        {
            verify_runtime_bundle(&root).map_err(PipelineError::Message)?;
            return Ok(paths);
        }
    }
    Err(PipelineError::Message(
        "내장 FFmpeg, ffprobe, Whisper 또는 ggml-base.bin을 찾지 못했습니다. npm.cmd run media-tools를 실행해 주세요.".into(),
    ))
}

fn write_pipeline_provenance(
    job_dir: &Path,
    source: &Path,
    checkpoint: &MediaCheckpoint,
) -> Result<(), PipelineError> {
    let (input_fingerprint, input_bytes) =
        source_fingerprint(source).map_err(PipelineError::Message)?;
    let hashes = runtime_hashes().map_err(PipelineError::Message)?;
    let transcription_seconds = checkpoint
        .planned_chunks
        .iter()
        .map(|chunk| chunk.length_seconds)
        .sum::<f64>();
    let completed_gpu = checkpoint
        .whisper_units
        .iter()
        .filter(|unit| unit.gpu.status == WhisperAttemptStatus::Completed)
        .count();
    let completed_cpu = checkpoint
        .whisper_units
        .iter()
        .filter(|unit| unit.cpu_fallback.status == WhisperAttemptStatus::Completed)
        .count();
    let actual_backend = match (completed_gpu > 0, completed_cpu > 0) {
        (true, true) => "whisper.cpp-gpu-and-cpu-fallback",
        (true, false) => "whisper.cpp-gpu",
        (false, true) => {
            if matches!(checkpoint.whisper_settings.device_mode, WhisperDeviceMode::Cpu) {
                "whisper.cpp-cpu"
            } else {
                "whisper.cpp-cpu-fallback"
            }
        }
        (false, false) => "whisper.cpp-no-units",
    };
    let provenance = serde_json::json!({
        "schemaVersion": 1,
        "appVersion": env!("CARGO_PKG_VERSION"),
        "inputFingerprint": {
            "algorithm": "sha256(size+first1MiB+last1MiB)",
            "value": input_fingerprint,
            "bytes": input_bytes
        },
        "analysis": {
            "mode": checkpoint.analysis_mode,
            "startSeconds": checkpoint.analysis_start_seconds,
            "endSeconds": checkpoint.analysis_end_seconds,
            "transcriptionSeconds": transcription_seconds,
            "chunkCount": checkpoint.planned_chunks.len()
        },
        "runtimeSha256": hashes,
        "transcription": {
            "requestedDevice": checkpoint.whisper_settings.device_mode,
            "backend": actual_backend,
            "completedGpuUnits": completed_gpu,
            "completedCpuUnits": completed_cpu,
            "language": "ko",
            "model": MODEL_NAME,
            "profile": checkpoint.whisper_settings.profile,
            "cpuThreads": checkpoint.whisper_settings.cpu_threads,
            "units": checkpoint.whisper_units,
            "noGpuInCpuMode": matches!(checkpoint.whisper_settings.device_mode, WhisperDeviceMode::Cpu)
        },
        "chatMotion": {
            "roi": "right-38-percent",
            "sampleSeconds": if checkpoint.analysis_mode == AnalysisMode::Quick {
                QUICK_CHAT_SAMPLE_SECONDS
            } else {
                CHAT_SAMPLE_SECONDS
            }
        },
        "rankerVersion": RANKER_VERSION
    });
    let path = job_dir.join("pipeline-provenance.json");
    replace_file_preserving_previous(&path, &serde_json::to_vec_pretty(&provenance)?)?;
    Ok(())
}

/// Working-set estimate for media analysis after the source file is already on disk.
/// Includes one active WAV chunk, checkpoint/transcript growth, chat-motion temp frames,
/// and preview headroom. Source bytes are not re-counted.
pub(crate) fn estimate_analysis_workspace_bytes(
    source_bytes: u64,
    duration_seconds: f64,
) -> u64 {
    let _ = source_bytes; // retained for call-site clarity and future stream-size fusion
    const ACTIVE_WAV: u64 = 20 * MIB; // ~10 min mono PCM + margin
    const CHECKPOINT_HEADROOM: u64 = 256 * MIB;
    const PREVIEW_HEADROOM: u64 = 512 * MIB;
    const CHAT_MOTION_TEMP: u64 = 512 * MIB;
    // Finite positive duration only; non-finite/negative collapse to one hour floor.
    let hours = if duration_seconds.is_finite() && duration_seconds > 0.0 {
        (duration_seconds / 3600.0).ceil().max(1.0) as u64
    } else {
        1
    };
    let transcript_growth = hours.saturating_mul(32 * MIB);
    let base = ACTIVE_WAV
        .saturating_add(CHECKPOINT_HEADROOM)
        .saturating_add(PREVIEW_HEADROOM)
        .saturating_add(CHAT_MOTION_TEMP)
        .saturating_add(transcript_growth);
    // ~10% safety margin (filesystem TOCTOU / fragmentation); not a YouTube stream estimate.
    base.saturating_add(base / 10)
}

pub(crate) fn ensure_sufficient_disk_space(
    available_bytes: u64,
    required_bytes: u64,
) -> Result<(), String> {
    if available_bytes < required_bytes {
        let shortfall = required_bytes - available_bytes;
        return Err(format!(
            "저장 공간이 부족합니다. 분석에 약 {}이 필요하지만 현재 여유 공간은 {}입니다. 약 {}을 확보한 뒤 다시 시작해 주세요.",
            format_bytes_for_message(required_bytes),
            format_bytes_for_message(available_bytes),
            format_bytes_for_message(shortfall)
        ));
    }
    Ok(())
}

fn ensure_analysis_disk_space(
    job_dir: &Path,
    source_bytes: u64,
    duration_seconds: f64,
) -> Result<(), PipelineError> {
    let required = estimate_analysis_workspace_bytes(source_bytes, duration_seconds);
    let available = free_disk_space_bytes(job_dir).map_err(PipelineError::Message)?;
    ensure_sufficient_disk_space(available, required).map_err(PipelineError::Message)
}

fn progress<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &Arc<AppState>,
    unit: u32,
    status: JobStatus,
    stage: &str,
    message: &str,
) -> Result<(), PipelineError> {
    check_cancel(state)?;
    let current = state
        .job
        .lock()
        .map_err(|_| PipelineError::Message("작업 상태 잠금이 손상됐습니다.".into()))?
        .as_ref()
        .map(|job| job.completed_units)
        .unwrap_or(0);
    if current >= unit {
        return Ok(());
    }
    mutate_job(app, state, |job| {
        job.apply_progress(unit, status, stage.into(), message.into())
    })
    .map(|_| ())
    .map_err(PipelineError::Message)
}

fn check_cancel(state: &Arc<AppState>) -> Result<(), PipelineError> {
    if state.cancel_requested.load(Ordering::SeqCst) {
        Err(PipelineError::Cancelled)
    } else {
        Ok(())
    }
}

fn persist_stage_metric<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &AppState,
    stage: ResourceStage,
    started: Instant,
) -> Result<(), PipelineError> {
    match record_stage_metric(
        app,
        state,
        stage,
        started,
        ResourceSample {
            external_tool_count: Some(1),
            ..Default::default()
        },
    ) {
        Ok(()) => Ok(()),
        Err(detail) => {
            let reason = detail
                .strip_prefix("자원 제한 초과: ")
                .unwrap_or(&detail)
                .to_string();
            if detail.starts_with("자원 제한 초과: ") {
                Err(PipelineError::ResourceLimit { stage, reason })
            } else {
                Err(PipelineError::Message(detail))
            }
        }
    }
}

fn run_command<I>(
    cancel_requested: &AtomicBool,
    executable: &Path,
    current_dir: &Path,
    args: I,
    stdout_path: &Path,
    stderr_path: &Path,
) -> Result<(), PipelineError>
where
    I: IntoIterator<Item = std::ffi::OsString>,
{
    if cancel_requested.load(Ordering::SeqCst) {
        return Err(PipelineError::Cancelled);
    }
    let stdout = File::create(stdout_path)?;
    let stderr = File::create(stderr_path)?;
    let mut command = Command::new(executable);
    command
        .args(args)
        .current_dir(current_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    restrict_command_environment(&mut command);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|error| {
        PipelineError::Message(format!("{} 실행 실패: {error}", executable.display()))
    })?;
    #[cfg(windows)]
    let job_guard = match KillOnCloseJob::attach(&child) {
        Ok(job) => Some(job),
        Err(error) => {
            terminate_child_tree(&mut child, None);
            return Err(PipelineError::Message(format!(
                "{}에 강제 종료 보호를 설정하지 못했습니다: {error}",
                executable.file_name().unwrap_or_default().to_string_lossy()
            )));
        }
    };
    loop {
        if cancel_requested.load(Ordering::SeqCst) {
            #[cfg(windows)]
            terminate_child_tree(&mut child, job_guard.as_ref());
            #[cfg(not(windows))]
            terminate_child_tree(&mut child);
            return Err(PipelineError::Cancelled);
        }
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(());
            }
            let detail = read_tail(stderr_path, 4000);
            return Err(PipelineError::Message(format!(
                "{} 종료 코드 {:?}: {}",
                executable.file_name().unwrap_or_default().to_string_lossy(),
                status.code(),
                detail
            )));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

pub(crate) fn restrict_command_environment(command: &mut Command) {
    const ALLOWED: &[&str] = &[
        "SystemRoot",
        "WINDIR",
        "TEMP",
        "TMP",
        "LOCALAPPDATA",
        "APPDATA",
        "USERPROFILE",
        "COMSPEC",
        "NUMBER_OF_PROCESSORS",
        "PATHEXT",
    ];
    let values = ALLOWED
        .iter()
        .filter_map(|name| std::env::var_os(name).map(|value| (*name, value)))
        .collect::<Vec<_>>();
    command.env_clear();
    for (name, value) in values {
        command.env(name, value);
    }
}

fn read_tail(path: &Path, max_chars: usize) -> String {
    let mut value = String::new();
    if File::open(path)
        .and_then(|mut file| file.read_to_string(&mut value))
        .is_err()
    {
        return "진단 로그를 읽지 못했습니다.".into();
    }
    value
        .chars()
        .rev()
        .take(max_chars)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

fn load_checkpoint(
    path: &Path,
    source_path: &str,
    analysis_mode: AnalysisMode,
    requested_start: Option<u32>,
    requested_end: Option<u32>,
    expected_fingerprint: &str,
    expected_input_bytes: u64,
    expected_runtime: &HashMap<String, String>,
) -> Result<Option<MediaCheckpoint>, PipelineError> {
    load_checkpoint_with_caption(
        path,
        source_path,
        analysis_mode,
        requested_start,
        requested_end,
        expected_fingerprint,
        expected_input_bytes,
        expected_runtime,
        None,
        &WhisperSettings::default(),
    )
}

fn load_checkpoint_with_caption(
    path: &Path,
    source_path: &str,
    analysis_mode: AnalysisMode,
    requested_start: Option<u32>,
    requested_end: Option<u32>,
    expected_fingerprint: &str,
    expected_input_bytes: u64,
    expected_runtime: &HashMap<String, String>,
    expected_caption: Option<&CaptionProvenance>,
    expected_whisper: &WhisperSettings,
) -> Result<Option<MediaCheckpoint>, PipelineError> {
    let requested_start = requested_start.unwrap_or(0);
    let previous = previous_generation_path(path);
    // Live first, then .prev. Corrupt/unreadable live falls through; valid-but-incompatible
    // live must not resume from a prior generation (different fingerprint/tools/ranker run).
    for (is_live, candidate) in [(true, path), (false, previous.as_path())] {
        if !candidate.is_file() {
            continue;
        }
        let bytes = match fs::read(candidate) {
            Ok(bytes) if !bytes.is_empty() => bytes,
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        let checkpoint: MediaCheckpoint = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if checkpoint_is_compatible_with_whisper(
            &checkpoint,
            source_path,
            analysis_mode,
            requested_start,
            requested_end,
            expected_fingerprint,
            expected_input_bytes,
            expected_runtime,
            expected_caption,
            expected_whisper,
        ) {
            return Ok(Some(checkpoint));
        }
        if is_live {
            return Ok(None);
        }
    }
    Ok(None)
}

fn checkpoint_is_compatible(
    checkpoint: &MediaCheckpoint,
    source_path: &str,
    analysis_mode: AnalysisMode,
    requested_start: u32,
    requested_end: Option<u32>,
    expected_fingerprint: &str,
    expected_input_bytes: u64,
    expected_runtime: &HashMap<String, String>,
) -> bool {
    checkpoint_is_compatible_with_caption(
        checkpoint,
        source_path,
        analysis_mode,
        requested_start,
        requested_end,
        expected_fingerprint,
        expected_input_bytes,
        expected_runtime,
        None,
    )
}

fn checkpoint_is_compatible_with_caption(
    checkpoint: &MediaCheckpoint,
    source_path: &str,
    analysis_mode: AnalysisMode,
    requested_start: u32,
    requested_end: Option<u32>,
    expected_fingerprint: &str,
    expected_input_bytes: u64,
    expected_runtime: &HashMap<String, String>,
    expected_caption: Option<&CaptionProvenance>,
) -> bool {
    if !(4..=MEDIA_CHECKPOINT_SCHEMA).contains(&checkpoint.schema_version) {
        return false;
    }
    let caption_matches = match expected_caption {
        Some(expected) => {
            checkpoint.caption_source_url == expected.source_url
                && checkpoint.caption_sha256 == expected.sha256
                && checkpoint.caption_revision == expected.revision
                && checkpoint.caption_schema_version == expected.schema_version
                && checkpoint.caption_content_sha256 == expected.content_sha256
                && checkpoint.caption_verification_state == Some(expected.verification_state)
        }
        None => {
            checkpoint.caption_source_url.is_empty()
                && checkpoint.caption_sha256.is_empty()
                && checkpoint.caption_revision.is_empty()
                && checkpoint.caption_schema_version == 0
                && checkpoint.caption_content_sha256.is_empty()
                && checkpoint.caption_verification_state.is_none()
        }
    };
    if !caption_matches {
        return false;
    }
    if checkpoint.source_path != source_path
        || checkpoint.analysis_mode != analysis_mode
        || checkpoint.analysis_start_seconds != requested_start
        || (analysis_mode == AnalysisMode::Range
            && Some(checkpoint.analysis_end_seconds) != requested_end)
    {
        return false;
    }
    if checkpoint.schema_version >= 4 {
        if checkpoint.input_fingerprint.is_empty()
            || checkpoint.input_fingerprint != expected_fingerprint
            || checkpoint.input_bytes != expected_input_bytes
        {
            return false;
        }
    }
    if checkpoint.schema_version >= 4 {
        if checkpoint.language != TRANSCRIPTION_LANGUAGE
            || checkpoint.ranker_version != RANKER_VERSION
            || checkpoint.runtime_sha256 != *expected_runtime
        {
            return false;
        }
    }
    if !checkpoint.duration_seconds.is_finite() || checkpoint.duration_seconds <= 0.0 {
        return false;
    }
    true
}

fn checkpoint_is_compatible_with_whisper(
    checkpoint: &MediaCheckpoint,
    source_path: &str,
    analysis_mode: AnalysisMode,
    requested_start: u32,
    requested_end: Option<u32>,
    expected_fingerprint: &str,
    expected_input_bytes: u64,
    expected_runtime: &HashMap<String, String>,
    expected_caption: Option<&CaptionProvenance>,
    expected_whisper: &WhisperSettings,
) -> bool {
    if !checkpoint_is_compatible_with_caption(
        checkpoint,
        source_path,
        analysis_mode,
        requested_start,
        requested_end,
        expected_fingerprint,
        expected_input_bytes,
        expected_runtime,
        expected_caption,
    ) {
        return false;
    }
    // Schema 4 has no G2 settings and can resume only for its explicit legacy
    // CPU/Balanced/auto-threads configuration. Schema 5 is tied to the
    // selected settings.
    if checkpoint.schema_version == 4 {
        return *expected_whisper == legacy_cpu_whisper_settings();
    }
    checkpoint.whisper_settings == expected_whisper.clone().normalized()
}

fn legacy_cpu_whisper_settings() -> WhisperSettings {
    WhisperSettings {
        device_mode: WhisperDeviceMode::Cpu,
        profile: whisper::WhisperProfile::Balanced,
        cpu_threads: None,
    }
}

fn save_checkpoint(path: &Path, checkpoint: &MediaCheckpoint) -> Result<(), PipelineError> {
    replace_file_preserving_previous(path, &serde_json::to_vec_pretty(checkpoint)?)?;
    Ok(())
}

fn write_whisper_budget(path: &Path, checkpoint: &MediaCheckpoint) -> Result<(), PipelineError> {
    let budget = serde_json::json!({
        "schemaVersion": 1,
        "updatedAt": Utc::now().to_rfc3339(),
        "requestedDevice": checkpoint.whisper_settings.device_mode,
        "units": checkpoint.whisper_units,
    });
    replace_file_preserving_previous(path, &serde_json::to_vec_pretty(&budget)?)?;
    Ok(())
}

fn persist_whisper_state(
    checkpoint_path: &Path,
    budget_path: &Path,
    checkpoint: &MediaCheckpoint,
) -> Result<(), PipelineError> {
    save_checkpoint(checkpoint_path, checkpoint)?;
    write_whisper_budget(budget_path, checkpoint)
}

fn write_transcript(path: &Path, segments: &[TranscriptSegment]) -> Result<(), PipelineError> {
    replace_file_preserving_previous(path, &serde_json::to_vec_pretty(segments)?)?;
    Ok(())
}

fn write_chat_motion(path: &Path, points: &[ChatMotionPoint]) -> Result<(), PipelineError> {
    replace_file_preserving_previous(path, &serde_json::to_vec_pretty(points)?)?;
    Ok(())
}

fn normalize_transcript(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || ('가'..='힣').contains(&character) {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn transcript_quality_reasons(value: &str) -> Vec<String> {
    let normalized = normalize_transcript(value);
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    let mut reasons = Vec::new();
    if value.contains('\u{fffd}') { reasons.push("U+FFFD 깨진 문자가 포함됨".into()); }
    let repeated_short_word = words.len() >= 3 && words.iter().all(|word| *word == words[0]) && words[0].chars().count() <= 6;
    let repeated_short_sentence = words.len() >= 4 && words.len() % 2 == 0 && words[..words.len() / 2] == words[words.len() / 2..] && words[..words.len() / 2].len() <= 4;
    let compact = normalized.chars().filter(|character| !character.is_whitespace()).collect::<Vec<_>>();
    let repeated_short_syllable = compact.len() >= 3 && compact.iter().all(|character| *character == compact[0]);
    if repeated_short_syllable || repeated_short_word { reasons.push("짧은 단어 또는 음절이 비정상적으로 반복됨".into()); }
    if repeated_short_sentence { reasons.push("짧은 문장이 비정상적으로 반복됨".into()); }
    let lower = value.to_lowercase();
    if [
        "[music]",
        "(music)",
        "[음악]",
        "자막 제공",
        "시청해 주셔서 감사합니다",
    ]
    .iter()
    .any(|marker| lower.contains(marker)) {
        reasons.push("음악 또는 자막 안내 문구로 의심됨".into());
    }

    let hangul = normalized
        .chars()
        .filter(|character| ('가'..='힣').contains(character))
        .count();
    let ascii_letters = normalized
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .count();
    if hangul == 0 && ascii_letters >= 8 {
        reasons.push("한국어 음성 인식 문장으로 보기 어려운 결과".into());
    }

    let words = normalized.split_whitespace().collect::<Vec<_>>();
    if words.len() >= 6 && words[..3] == words[3..6] {
        reasons.push("Whisper 출력에서 반복 문구가 감지됨".into());
    }
    reasons
}

fn sanitize_transcript_segments(segments: Vec<TranscriptSegment>) -> Vec<TranscriptSegment> {
    let mut annotated = segments.into_iter().filter(|segment| segment.start_seconds.is_finite() && segment.end_seconds.is_finite() && segment.start_seconds < segment.end_seconds).map(|mut segment| {
        segment.quality_reasons = transcript_quality_reasons(&segment.text);
        segment.quality_status = if segment.quality_reasons.is_empty() { TranscriptQualityStatus::Certain } else { TranscriptQualityStatus::Uncertain };
        segment
    }).collect::<Vec<_>>();
    for index in 0..annotated.len() {
        let normalized = normalize_transcript(&annotated[index].text);
        if normalized.is_empty() || normalized.chars().count() > 12 { continue; }
        let repeated_index = (0..index).rev().find(|previous| annotated[index].start_seconds - annotated[*previous].end_seconds < 120.0 && normalize_transcript(&annotated[*previous].text) == normalized);
        let Some(previous) = repeated_index else { continue };
        let reason = "짧은 단어 또는 음절이 비정상적으로 반복됨".to_string();
        if !annotated[index].quality_reasons.contains(&reason) { annotated[index].quality_reasons.push(reason.clone()); }
        if !annotated[previous].quality_reasons.contains(&reason) { annotated[previous].quality_reasons.push(reason); }
        annotated[index].quality_status = TranscriptQualityStatus::Uncertain;
        annotated[previous].quality_status = TranscriptQualityStatus::Uncertain;
    }
    annotated
}

#[allow(clippy::too_many_arguments)]
fn analyze_chat_motion(
    cancel_requested: &AtomicBool,
    ffmpeg: &Path,
    ffmpeg_dir: &Path,
    source: &Path,
    start_seconds: f64,
    length_seconds: f64,
    sample_seconds: f64,
    output: &Path,
    stderr: &Path,
) -> Result<Vec<ChatMotionPoint>, PipelineError> {
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
    ];
    if start_seconds > 0.0 {
        args.extend(["-ss".into(), format!("{start_seconds:.3}").into()]);
    }
    args.extend([
        "-t".into(),
        format!("{length_seconds:.3}").into(),
        "-skip_frame".into(),
        "nokey".into(),
        "-protocol_whitelist".into(),
        "file,crypto,data".into(),
        "-i".into(),
        source.as_os_str().into(),
        "-an".into(),
        "-vf".into(),
        format!(
            "fps=1/{sample_seconds},crop=iw*0.38:ih:iw*0.62:0,scale={CHAT_FRAME_SIDE}:{CHAT_FRAME_SIDE}:flags=area,format=gray"
        )
        .into(),
        "-f".into(),
        "rawvideo".into(),
        output.as_os_str().into(),
    ]);
    run_command(
        cancel_requested,
        ffmpeg,
        ffmpeg_dir,
        args,
        &output.with_extension("stdout.log"),
        stderr,
    )?;

    let frame_size = CHAT_FRAME_SIDE * CHAT_FRAME_SIDE;
    let mut reader = BufReader::new(File::open(output)?);
    let mut previous = vec![0u8; frame_size];
    if let Err(error) = reader.read_exact(&mut previous) {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            return Ok(Vec::new());
        }
        return Err(error.into());
    }
    let mut frame = vec![0u8; frame_size];
    let mut index = 0usize;
    let mut points = Vec::new();
    loop {
        match reader.read_exact(&mut frame) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.into()),
        }
        let difference = previous
            .iter()
            .zip(&frame)
            .map(|(left, right)| (*left as f64 - *right as f64).abs())
            .sum::<f64>()
            / frame_size as f64
            / 255.0;
        index += 1;
        points.push(ChatMotionPoint {
            start_seconds: start_seconds + index as f64 * sample_seconds,
            motion: difference,
        });
        std::mem::swap(&mut previous, &mut frame);
    }
    if points.is_empty() {
        return Ok(Vec::new());
    }
    Ok(points)
}

fn analyze_wav(path: &Path, offset: f64) -> Result<Vec<EnergyPoint>, PipelineError> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|error| PipelineError::Message(format!("WAV 분석 실패: {error}")))?;
    let sample_rate = reader.spec().sample_rate.max(1) as usize;
    let mut points = Vec::new();
    let mut sum = 0.0;
    let mut count = 0usize;
    let mut window = 0usize;
    for sample in reader.samples::<i16>() {
        let value = sample
            .map_err(|error| PipelineError::Message(format!("WAV 샘플 읽기 실패: {error}")))?
            as f64
            / i16::MAX as f64;
        sum += value * value;
        count += 1;
        if count == sample_rate {
            points.push(EnergyPoint {
                start_seconds: offset + window as f64,
                rms: (sum / count as f64).sqrt(),
            });
            sum = 0.0;
            count = 0;
            window += 1;
        }
    }
    if count > 0 {
        points.push(EnergyPoint {
            start_seconds: offset + window as f64,
            rms: (sum / count as f64).sqrt(),
        });
    }
    Ok(points)
}

fn parse_srt(path: &Path, offset: f64) -> Result<Vec<TranscriptSegment>, PipelineError> {
    let bytes = fs::read(path).map_err(|error| {
        PipelineError::Message(format!("Whisper SRT 결과를 읽지 못했습니다: {error}"))
    })?;
    let text = String::from_utf8_lossy(&bytes);
    let normalized = text.replace("\r\n", "\n");
    let mut segments = Vec::new();
    for block in normalized.split("\n\n") {
        let mut lines = block.lines();
        let _index = lines.next();
        let Some(range) = lines.next() else { continue };
        let Some((start, end)) = range.split_once(" --> ") else {
            continue;
        };
        let body = lines.collect::<Vec<_>>().join(" ").trim().to_string();
        if body.is_empty() {
            continue;
        }
        if let (Some(start), Some(end)) = (parse_srt_time(start), parse_srt_time(end)) {
            segments.push(TranscriptSegment {
                start_seconds: offset + start,
                end_seconds: offset + end,
                text: body,
                quality_status: TranscriptQualityStatus::Certain,
                quality_reasons: Vec::new(),
            });
        }
    }
    Ok(segments)
}

fn clip_segments_to_range(segments: &mut Vec<TranscriptSegment>, start: f64, end: f64) {
    let mut clipped = Vec::with_capacity(segments.len());
    for mut segment in segments.drain(..) {
        if segment.end_seconds <= start || segment.start_seconds >= end {
            continue;
        }
        segment.start_seconds = segment.start_seconds.max(start);
        segment.end_seconds = segment.end_seconds.min(end);
        if segment.start_seconds < segment.end_seconds {
            clipped.push(segment);
        }
    }
    *segments = clipped;
}

fn parse_srt_time(value: &str) -> Option<f64> {
    let mut parts = value.trim().split([':', ',']);
    let hours = parts.next()?.parse::<f64>().ok()?;
    let minutes = parts.next()?.parse::<f64>().ok()?;
    let seconds = parts.next()?.parse::<f64>().ok()?;
    let millis = parts.next()?.parse::<f64>().ok()?;
    Some(hours * 3600.0 + minutes * 60.0 + seconds + millis / 1000.0)
}

fn build_candidates(
    duration: f64,
    range_start_seconds: f64,
    range_end_seconds: f64,
    segments: &[TranscriptSegment],
    energy: &[EnergyPoint],
    chat_motion: &[ChatMotionPoint],
) -> Vec<Candidate> {
    let segments = sanitize_transcript_segments(segments.to_vec());
    let segments = segments.as_slice();
    let range_start = range_start_seconds.clamp(0.0, duration.max(0.0));
    let range_end = range_end_seconds
        .max(range_start)
        .min(if duration.is_finite() {
            duration.max(range_start)
        } else {
            range_start
        });
    let span = (range_end - range_start).max(0.0);
    if span <= f64::EPSILON {
        return Vec::new();
    }
    let window_size = span.clamp(1.0, 45.0).min(span.max(1.0));
    let mut windows = Vec::new();
    let mut start = range_start;
    loop {
        let end = (start + window_size).min(range_end);
        if end - start <= f64::EPSILON {
            break;
        }
        let points = energy
            .iter()
            .filter(|point| point.start_seconds >= start && point.start_seconds < end)
            .collect::<Vec<_>>();
        let spoken = segments
            .iter()
            .filter(|segment| segment.end_seconds > start && segment.start_seconds < end && !segment.text.trim().is_empty())
            .collect::<Vec<_>>();
        let has_audio_evidence = points.iter().any(|point| point.rms.is_finite() && point.rms > f64::EPSILON);
        // P0: drop windows with neither audible energy nor dialogue text.
        // Chat-motion alone, or zero-valued audio samples, is not enough evidence
        // to rank a candidate or fill the requested count.
        if !has_audio_evidence && spoken.is_empty() {
            if end >= range_end - f64::EPSILON {
                break;
            }
            start += 15.0;
            if start >= range_end {
                break;
            }
            continue;
        }
        let audio_raw = if points.is_empty() {
            0.0
        } else {
            points.iter().map(|point| point.rms).sum::<f64>() / points.len() as f64
        };
        let characters = spoken
            .iter()
            .map(|segment| segment.text.chars().count())
            .sum::<usize>();
        let dialogue_raw = characters as f64 + spoken.len() as f64 * 12.0;
        let chat_points = chat_motion
            .iter()
            .filter(|point| point.start_seconds >= start && point.start_seconds < end)
            .collect::<Vec<_>>();
        let chat_baseline = if chat_motion.len() >= 2 {
            let mut values = chat_motion.iter().map(|point| point.motion).collect::<Vec<_>>();
            values.sort_by(|left, right| left.total_cmp(right));
            Some(values[values.len() / 2])
        } else {
            None
        };
        let chat_raw = if chat_baseline.is_none() {
            None
        } else if chat_points.is_empty() {
            Some(0.0)
        } else {
            Some(
                chat_points.iter().map(|point| (point.motion - chat_baseline.unwrap()).max(0.0)).sum::<f64>()
                    / chat_points.len() as f64,
            )
        };
        let mut transcript_quality_reasons = Vec::new();
        for segment in &spoken {
            for reason in &segment.quality_reasons {
                if !transcript_quality_reasons.contains(reason) { transcript_quality_reasons.push(reason.clone()); }
            }
        }
        let transcript_quality_status = if transcript_quality_reasons.is_empty() { TranscriptQualityStatus::Certain } else { TranscriptQualityStatus::Uncertain };
        let mut quality_warnings = transcript_quality_reasons.clone();
        if spoken.windows(2).any(|pair| pair[1].start_seconds - pair[0].end_seconds > 8.0) {
            quality_warnings.push("앞뒤 문장이 연결되지 않아 맥락 확인이 필요함".into());
        }
        let excerpt = spoken
            .iter()
            .filter(|segment| segment.quality_status == TranscriptQualityStatus::Certain)
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        windows.push(WindowScore {
            start,
            end,
            audio_raw,
            dialogue_raw,
            chat_raw,
            excerpt,
            transcript_quality_status,
            transcript_quality_reasons,
            quality_warnings,
        });
        if end >= range_end - f64::EPSILON {
            break;
        }
        start += 15.0;
        if start >= range_end {
            break;
        }
    }

    let max_audio = windows
        .iter()
        .map(|window| window.audio_raw)
        .fold(0.0, f64::max);
    let max_dialogue = windows
        .iter()
        .map(|window| window.dialogue_raw)
        .fold(0.0, f64::max);
    let max_chat = windows
        .iter()
        .filter_map(|window| window.chat_raw)
        .fold(0.0, f64::max);
    let mut scored = windows
        .into_iter()
        .map(|window| {
            let audio = normalized_score(window.audio_raw, max_audio);
            let dialogue = normalized_score(window.dialogue_raw, max_dialogue);
            let chat = window
                .chat_raw
                .map(|value| normalized_score(value, max_chat));
            let total = if let Some(chat) = chat {
                (audio as f64 * 0.45 + dialogue as f64 * 0.35 + chat as f64 * 0.20).round() as u8
            } else {
                (audio as f64 * 0.55 + dialogue as f64 * 0.45).round() as u8
            };
            (window, audio, dialogue, chat, total)
        })
        .collect::<Vec<_>>();
    scored.sort_by_key(|item| std::cmp::Reverse(item.4));

    let mut selected: Vec<(WindowScore, u8, u8, Option<u8>, u8)> = Vec::new();
    for item in scored {
        let overlaps_or_repeats = selected.iter().any(|selected| {
            let overlaps = item.0.start < selected.0.end && selected.0.start < item.0.end;
            // Identical text at different source times is kept: only the same
            // time/event overlap is a duplicate candidate.
            overlaps
        });
        if overlaps_or_repeats {
            continue;
        }
        selected.push(item);
        // Keep the full bounded pool so a later 8/20/30 setting change can
        // create a revision without inventing filler candidates.
        if selected.len() == 30 {
            break;
        }
    }
    selected.sort_by(|left, right| {
        right.4.cmp(&left.4).then_with(|| {
            left.0
                .start
                .partial_cmp(&right.0.start)
                .unwrap_or(CmpOrdering::Equal)
        })
    });
    selected
        .into_iter()
        .map(|(window, audio, dialogue, chat, total)| {
            let excerpt = truncate_chars(window.excerpt.trim(), 140);
            let uncertain = window.transcript_quality_status == TranscriptQualityStatus::Uncertain;
            let title = if uncertain {
                if excerpt.is_empty() { "음성 인식 결과 불확실 · 오디오 근거 구간".into() }
                else { format!("{} · 음성 인식 결과 불확실", truncate_chars(&excerpt, 20)) }
            } else if excerpt.is_empty() {
                "오디오 반응이 컸던 구간".into()
            } else {
                truncate_chars(&excerpt, 28)
            };
            let start_seconds = window.start.floor().max(0.0) as u32;
            let end_seconds = window.end.ceil().max(1.0) as u32;
            let (context_start_seconds, context_end_seconds) =
                context_bounds(start_seconds, end_seconds, duration);
            let mut selection_reasons = vec![format!("오디오 반응 {audio}")];
            if dialogue > 0 { selection_reasons.push(format!("말하기 변화 {dialogue}")); }
            if audio == 0 && dialogue > 0 {
                selection_reasons.push("오디오가 조용해도 이어지는 말하기 근거 유지".into());
            } else if audio > 0 && dialogue == 0 {
                selection_reasons.push("음성 인식 문장 없이 오디오 근거 유지".into());
            }
            if let Some(chat) = chat { selection_reasons.push(format!("채팅 영역 움직임 {chat}")); }
            let quality_status = if window.quality_warnings.is_empty() { "VALID" } else { "WARNING" };
            Candidate {
                id: stable_candidate_id(start_seconds, end_seconds),
                start_seconds,
                end_seconds,
                title,
                summary: if uncertain {
                    if let Some(chat) = chat { format!("음성 인식 결과 불확실 · 오디오 반응 {audio} · 발화 밀도 {dialogue} · 채팅 움직임 {chat}") }
                    else { format!("음성 인식 결과 불확실 · 오디오 반응 {audio} · 발화 밀도 {dialogue} · 확인 가능한 채팅 영역 움직임 없음") }
                } else if let Some(chat) = chat {
                    format!("오디오 반응 {audio} · 발화 밀도 {dialogue} · 채팅 움직임 {chat}")
                } else {
                    format!("오디오 반응 {audio} · 발화 밀도 {dialogue} · 확인 가능한 채팅 영역 움직임 없음")
                },
                transcript_excerpt: if uncertain {
                    "음성 인식 결과가 불확실해 원문을 표시하지 않습니다.".into()
                } else if excerpt.is_empty() {
                    "이 구간에서 인식된 발화가 없습니다.".into()
                } else {
                    excerpt
                },
                audio_score: audio,
                dialogue_score: dialogue,
                chat_score: chat,
                total_score: total,
                decision: CandidateDecision::Pending,
                quality_status: quality_status.into(),
                quality_warnings: window.quality_warnings.clone(),
                selection_reasons,
                uncertainty_reasons: window.transcript_quality_reasons.clone(),
                transcript_quality_status: window.transcript_quality_status,
                transcript_quality_reasons: window.transcript_quality_reasons,
                context_start_seconds,
                context_end_seconds,
                context_transcript: context_transcript(
                    segments,
                    context_start_seconds,
                    context_end_seconds,
                ),
            }
        })
        .collect()
}

fn stable_candidate_id(start_seconds: u32, end_seconds: u32) -> String {
    format!("local-candidate-{start_seconds:06}-{end_seconds:06}")
}

pub(crate) fn context_bounds(
    candidate_start_seconds: u32,
    candidate_end_seconds: u32,
    media_duration_seconds: f64,
) -> (f64, f64) {
    let duration = if media_duration_seconds.is_finite() {
        media_duration_seconds.max(0.0)
    } else {
        0.0
    };
    let start = (candidate_start_seconds as f64 - CONTEXT_PADDING_SECONDS)
        .max(0.0)
        .min(duration);
    let end = (candidate_end_seconds as f64 + CONTEXT_PADDING_SECONDS)
        .max(start)
        .min(duration);
    (start, end)
}

fn context_transcript(
    segments: &[TranscriptSegment],
    context_start_seconds: f64,
    context_end_seconds: f64,
) -> Vec<ContextTranscriptEntry> {
    let mut entries = segments
        .iter()
        .filter(|segment| {
            segment.start_seconds < context_end_seconds
                && segment.end_seconds > context_start_seconds
        })
        .map(|segment| ContextTranscriptEntry {
            start_seconds: segment.start_seconds,
            end_seconds: segment.end_seconds,
            text: safe_transcript_text(segment),
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        left.start_seconds
            .total_cmp(&right.start_seconds)
            .then_with(|| left.end_seconds.total_cmp(&right.end_seconds))
            .then_with(|| left.text.cmp(&right.text))
    });
    entries
}

const UNCERTAIN_TRANSCRIPT_PLACEHOLDER: &str = "음성 인식 결과가 불확실해 원문을 표시하지 않습니다.";

fn safe_transcript_text(segment: &TranscriptSegment) -> String {
    if segment.quality_status == TranscriptQualityStatus::Uncertain || segment.text.contains('\u{fffd}') {
        UNCERTAIN_TRANSCRIPT_PLACEHOLDER.into()
    } else { segment.text.clone() }
}

fn normalized_score(value: f64, max: f64) -> u8 {
    if max <= f64::EPSILON {
        0
    } else {
        (value / max * 100.0).round().clamp(0.0, 100.0) as u8
    }
}

fn truncate_chars(value: &str, maximum: usize) -> String {
    let mut result = value.chars().take(maximum).collect::<String>();
    if value.chars().count() > maximum {
        result.push('…');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{JobSnapshot, Scenario};
    use crate::resource::ResourceDecision;
    use std::env;
    use std::process::Stdio as ProcessStdio;

    #[test]
    fn parses_srt_timestamp() {
        assert_eq!(parse_srt_time("01:02:03,500"), Some(3723.5));
    }

    #[test]
    fn poisoned_heavy_tool_gate_fails_closed_before_external_execution() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state = crate::AppState::new(temp.path().to_path_buf(), temp.path().to_path_buf())
            .expect("AppState");
        let gate = &state.heavy_tool_gate;
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = gate.lock().expect("unpoisoned gate");
            panic!("poison test");
        }));
        assert!(acquire_heavy_tool_gate(&state).is_err());
    }

    #[test]
    fn heavy_tool_gate_failure_terminalization_preserves_checkpoint_and_candidate_decision() {
        let mut job = JobSnapshot::new(
            "gate-job".into(),
            SourceKind::Local,
            "source.mp4".into(),
            Scenario::Normal,
            AnalysisMode::Full,
            None,
            None,
        );
        job.status = JobStatus::Transcribing;
        job.completed_units = 6;
        job.owned_child_processes = 1;
        job.acquired_media_path = Some("job/media-checkpoint.json".into());
        job.candidates.push(Candidate {
            id: "candidate-keep".into(),
            start_seconds: 10,
            end_seconds: 20,
            title: "기존 제목".into(),
            summary: "기존 요약".into(),
            transcript_excerpt: "기존 결과".into(),
            audio_score: 80,
            dialogue_score: 70,
            chat_score: Some(60),
            total_score: 75,
            decision: CandidateDecision::Accepted,
            quality_status: "VALID".into(),
            quality_warnings: Vec::new(),
            selection_reasons: Vec::new(),
            uncertainty_reasons: Vec::new(),
            transcript_quality_status: TranscriptQualityStatus::Certain,
            transcript_quality_reasons: Vec::new(),
            context_start_seconds: 0.0,
            context_end_seconds: 30.0,
            context_transcript: Vec::new(),
        });
        let reason = "무거운 외부 도구 실행 잠금이 손상됐습니다.".to_string();
        let checkpoint_identity = job.acquired_media_path.clone();

        apply_heavy_tool_gate_failure(&mut job, reason.clone()).expect("terminalization");

        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.completed_units, 6);
        assert_eq!(job.acquired_media_path, checkpoint_identity);
        assert_eq!(job.error_detail.as_deref(), Some(reason.as_str()));
        assert_eq!(job.owned_child_processes, 0);
        assert_eq!(job.candidates[0].decision, CandidateDecision::Accepted);
    }

    #[test]
    fn manual_recognition_updates_candidate_pool_quality_metadata() {
        let candidate = Candidate {
            id: "candidate-sync".into(), start_seconds: 10, end_seconds: 20,
            title: "제목".into(), summary: "요약".into(), transcript_excerpt: "기존 결과".into(),
            audio_score: 80, dialogue_score: 70, chat_score: None, total_score: 75,
            decision: CandidateDecision::Accepted, quality_status: "VALID".into(), quality_warnings: Vec::new(), selection_reasons: Vec::new(), uncertainty_reasons: Vec::new(), transcript_quality_status: TranscriptQualityStatus::Certain,
            transcript_quality_reasons: Vec::new(), context_start_seconds: 0.0, context_end_seconds: 30.0, context_transcript: Vec::new(),
        };
        let output = CandidateRecognitionOutput {
            raw_result: "새 음성 인식 결과".into(), display_result: "음성 인식 결과가 불확실해 원문을 표시하지 않습니다.".into(),
            quality_status: TranscriptQualityStatus::Uncertain, quality_reasons: vec!["짧은 문장이 비정상적으로 반복됨".into()], backend_evidence: "fixture".into(),
        };
        let mut candidates = vec![candidate.clone()];
        let mut candidate_pool = vec![candidate];
        apply_candidate_recognition_output(&mut candidates, "candidate-sync", &output);
        apply_candidate_recognition_output(&mut candidate_pool, "candidate-sync", &output);

        for list in [&candidates, &candidate_pool] {
            let updated = &list[0];
            assert_eq!(updated.decision, CandidateDecision::Accepted);
            assert_eq!(updated.transcript_excerpt, output.display_result);
            assert_eq!(updated.transcript_quality_status, TranscriptQualityStatus::Uncertain);
            assert_eq!(updated.quality_status, "WARNING");
            assert_eq!(updated.quality_warnings, output.quality_reasons);
            assert_eq!(updated.uncertainty_reasons, output.quality_reasons);
        }
    }

    #[test]
    fn hard_limit_terminalization_preserves_completed_review_data_and_checkpoint_identity() {
        let mut job = JobSnapshot::new(
            "resource-job".into(),
            SourceKind::Local,
            "source.mp4".into(),
            Scenario::Normal,
            AnalysisMode::Full,
            None,
            None,
        );
        job.status = JobStatus::Transcribing;
        job.completed_units = 6;
        job.owned_child_processes = 1;
        job.acquired_media_path = Some("job/media-checkpoint.json".into());
        job.resource_policy.hard_external_tool_count = Some(0);
        job.candidates.push(Candidate {
            id: "candidate-keep".into(),
            start_seconds: 10,
            end_seconds: 20,
            title: "기존 제목".into(),
            summary: "기존 요약".into(),
            transcript_excerpt: "기존 결과".into(),
            audio_score: 80,
            dialogue_score: 70,
            chat_score: Some(60),
            total_score: 75,
            decision: CandidateDecision::Accepted,
            quality_status: "VALID".into(),
            quality_warnings: Vec::new(),
            selection_reasons: Vec::new(),
            uncertainty_reasons: Vec::new(),
            transcript_quality_status: TranscriptQualityStatus::Certain,
            transcript_quality_reasons: Vec::new(),
            context_start_seconds: 0.0,
            context_end_seconds: 30.0,
            context_transcript: Vec::new(),
        });
        let candidate_ids_and_decisions = job
            .candidates
            .iter()
            .map(|candidate| (candidate.id.clone(), candidate.decision))
            .collect::<Vec<_>>();
        let checkpoint_identity = job.acquired_media_path.clone();
        let reason = match job.resource_policy.evaluate(&ResourceSample {
            external_tool_count: Some(1),
            ..Default::default()
        }) {
            ResourceDecision::HardLimit(reason) => reason,
            decision => panic!("expected deterministic hard limit, got {decision:?}"),
        };

        apply_resource_limit_failure(&mut job, ResourceStage::FfmpegAudio, reason.clone())
            .expect("terminalization");

        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.resource_failure.as_ref().unwrap().reason, reason);
        assert_eq!(job.resource_failure.as_ref().unwrap().last_completed_units, 6);
        assert_eq!(job.owned_child_processes, 0);
        assert_eq!(job.acquired_media_path, checkpoint_identity);
        assert_eq!(
            job.candidates
                .iter()
                .map(|candidate| (candidate.id.clone(), candidate.decision))
                .collect::<Vec<_>>(),
            candidate_ids_and_decisions
        );
        assert_eq!(job.resource_policy.hard_external_tool_count, Some(0));
    }

    #[test]
    fn caption_partition_owns_cross_chunk_interval_once_and_covers_each_boundary() {
        let validation = captions::validate_intervals(
            vec![CaptionInterval {
                start_seconds: 9.0,
                end_seconds: 11.0,
                text: "crosses".into(),
            }],
            20.0,
            VerificationState::Verified,
        );
        let plan = captions::plan_fallbacks(&validation, 20.0);
        let chunks = vec![
            PlannedChunk {
                offset_seconds: 0.0,
                length_seconds: 10.0,
            },
            PlannedChunk {
                offset_seconds: 10.0,
                length_seconds: 10.0,
            },
        ];
        let (first_trusted, first_fallback) = partition_caption_chunk(&plan, &chunks, 0, 0, 20);
        let (second_trusted, second_fallback) = partition_caption_chunk(&plan, &chunks, 1, 0, 20);
        assert_eq!(first_trusted.len(), 1);
        assert_eq!(first_trusted[0].start_seconds, 9.0);
        assert_eq!(first_trusted[0].end_seconds, 11.0);
        assert!(second_trusted.is_empty());
        assert!(first_fallback.iter().any(|range| *range == (0.0, 9.0)));
        assert!(second_fallback.iter().any(|range| *range == (11.0, 20.0)));
    }

    #[test]
    fn caption_partition_uses_full_whisper_for_requested_range_and_invalid_caption() {
        let validation = captions::validate_intervals(
            vec![CaptionInterval {
                start_seconds: 14.0,
                end_seconds: 12.0,
                text: "invalid".into(),
            }],
            20.0,
            VerificationState::Verified,
        );
        let plan = captions::plan_fallbacks(&validation, 20.0);
        let chunks = vec![PlannedChunk {
            offset_seconds: 10.0,
            length_seconds: 5.0,
        }];
        let (trusted, fallback) = partition_caption_chunk(&plan, &chunks, 0, 10, 15);
        assert!(trusted.is_empty());
        assert_eq!(fallback, vec![(10.0, 15.0)]);
    }

    #[test]
    fn whisper_output_is_clipped_to_each_requested_fallback_boundary() {
        let mut segments = vec![
            TranscriptSegment {
                start_seconds: 9.0,
                end_seconds: 11.0,
                text: "before and inside".into(),
                quality_status: TranscriptQualityStatus::Certain,
                quality_reasons: Vec::new(),
            },
            TranscriptSegment {
                start_seconds: 11.0,
                end_seconds: 12.0,
                text: "outside".into(),
                quality_status: TranscriptQualityStatus::Certain,
                quality_reasons: Vec::new(),
            },
        ];
        clip_segments_to_range(&mut segments, 10.0, 11.0);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].start_seconds, 10.0);
        assert_eq!(segments[0].end_seconds, 11.0);
    }

    fn long_running_command() -> (PathBuf, Vec<std::ffi::OsString>) {
        #[cfg(windows)]
        {
            (
                PathBuf::from("ping.exe"),
                vec!["-n".into(), "60".into(), "127.0.0.1".into()],
            )
        }
        #[cfg(not(windows))]
        {
            (PathBuf::from("sleep"), vec!["60".into()])
        }
    }

    #[test]
    fn run_command_cancel_reaches_cancelled_under_five_seconds() {
        let temp = tempfile::tempdir().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_flag = Arc::clone(&cancel);
        let stdout = temp.path().join("cancel-stdout.log");
        let stderr = temp.path().join("cancel-stderr.log");
        let (executable, args) = long_running_command();
        let cwd = temp.path().to_path_buf();
        let worker = thread::spawn(move || {
            run_command(&cancel_flag, &executable, &cwd, args, &stdout, &stderr)
        });

        thread::sleep(Duration::from_millis(400));
        let started = Instant::now();
        cancel.store(true, Ordering::SeqCst);
        let result = worker.join().expect("cancel worker join");
        let elapsed = started.elapsed();
        assert!(
            matches!(result, Err(PipelineError::Cancelled)),
            "expected Cancelled, got {result:?}"
        );
        assert!(
            elapsed < Duration::from_secs(5),
            "cancel took {elapsed:?}, expected under 5s"
        );
    }

    #[test]
    fn terminate_child_tree_reaps_hanging_child_under_five_seconds() {
        let (executable, args) = long_running_command();
        let mut command = Command::new(&executable);
        command
            .args(args)
            .stdin(ProcessStdio::null())
            .stdout(ProcessStdio::null())
            .stderr(ProcessStdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = command.spawn().expect("spawn hanging child");
        #[cfg(windows)]
        let job = KillOnCloseJob::attach(&child).ok();
        thread::sleep(Duration::from_millis(200));
        let started = Instant::now();
        #[cfg(windows)]
        terminate_child_tree(&mut child, job.as_ref());
        #[cfg(not(windows))]
        terminate_child_tree(&mut child);
        let elapsed = started.elapsed();
        assert!(
            matches!(child.try_wait(), Ok(Some(_))),
            "child should be reaped after terminate_child_tree"
        );
        assert!(
            elapsed < Duration::from_secs(5),
            "terminate_child_tree took {elapsed:?}"
        );
    }

    #[cfg(windows)]
    fn windows_process_still_active(pid: u32) -> bool {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        const STILL_ACTIVE: u32 = 259;
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return false;
            }
            let mut code = 0u32;
            let ok = GetExitCodeProcess(handle, &mut code);
            CloseHandle(handle);
            ok != 0 && code == STILL_ACTIVE
        }
    }

    #[cfg(windows)]
    fn windows_child_pids(parent_pid: u32) -> Vec<u32> {
        let output = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Get-CimInstance Win32_Process -Filter \"ParentProcessId={parent_pid}\" | ForEach-Object {{ $_.ProcessId }}"
                ),
            ])
            .output()
            .expect("query child processes");
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .collect()
    }

    /// Owns the exact job object and direct child. On Drop, terminates via the
    /// job + direct child only — never mutates a numeric PID (no taskkill).
    #[cfg(windows)]
    struct WindowsTreeCleanup {
        // Drop impl terminates the job tree then reaps/takes parent; remaining fields follow declaration-order drop.
        job: Option<KillOnCloseJob>,
        parent: Option<Child>,
    }

    #[cfg(windows)]
    impl Drop for WindowsTreeCleanup {
        fn drop(&mut self) {
            if let Some(job) = self.job.as_ref() {
                job.terminate_all();
            }
            if let Some(mut parent) = self.parent.take() {
                let _ = parent.kill();
                let _ = parent.wait();
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn terminate_child_tree_kills_owned_cmd_ping_process_tree() {
        // Pre-spawn delay so KillOnCloseJob::attach assigns the parent before
        // the long-lived ping descendant is created (job membership race).
        let mut command = Command::new("cmd.exe");
        command
            .args([
                "/C",
                "ping.exe -n 3 127.0.0.1 >NUL && ping.exe -n 60 127.0.0.1 >NUL",
            ])
            .stdin(ProcessStdio::null())
            .stdout(ProcessStdio::null())
            .stderr(ProcessStdio::null());
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let child = command.spawn().expect("spawn tree parent");
        let parent_pid = child.id();
        let job = match KillOnCloseJob::attach(&child) {
            Ok(job) => job,
            Err(error) => {
                let mut child = child;
                let _ = child.kill();
                let _ = child.wait();
                panic!("attach job object: {error}");
            }
        };

        // Cleanup owns job + child; Drop uses terminate_all + kill/wait only.
        let mut cleanup = WindowsTreeCleanup {
            job: Some(job),
            parent: Some(child),
        };

        // Wait out the short delay ping so discovery targets the long-lived
        // ping that started after job assignment (read-only PID use only).
        thread::sleep(Duration::from_millis(2500));
        let mut descendant_pid = None;
        for _ in 0..40 {
            thread::sleep(Duration::from_millis(50));
            if let Some(pid) = windows_child_pids(parent_pid)
                .into_iter()
                .find(|pid| *pid != parent_pid && windows_process_still_active(*pid))
            {
                descendant_pid = Some(pid);
                break;
            }
        }
        let descendant_pid =
            descendant_pid.expect("cmd should have spawned a live descendant (ping)");
        assert!(
            windows_process_still_active(descendant_pid),
            "precondition: descendant {descendant_pid} must be alive before terminate"
        );

        let started = Instant::now();
        let mut parent = cleanup.parent.take().expect("parent child");
        let job = cleanup.job.as_ref().expect("job object");
        terminate_child_tree(&mut parent, Some(job));
        let elapsed = started.elapsed();
        cleanup.parent = Some(parent);

        let parent_reaped = matches!(
            cleanup
                .parent
                .as_mut()
                .map(|c| c.try_wait())
                .transpose()
                .expect("try_wait parent"),
            Some(Some(_))
        );
        assert!(
            parent_reaped || !windows_process_still_active(parent_pid),
            "parent should exit after job termination"
        );
        assert!(
            !windows_process_still_active(descendant_pid),
            "descendant pid {descendant_pid} must not remain after terminate_child_tree"
        );
        // Descendant-zero: no live children of the parent after tree terminate.
        let live_descendants: Vec<u32> = windows_child_pids(parent_pid)
            .into_iter()
            .filter(|pid| windows_process_still_active(*pid))
            .collect();
        assert!(
            live_descendants.is_empty(),
            "expected zero live descendants after terminate, found {live_descendants:?}"
        );
        assert!(
            elapsed < Duration::from_secs(5),
            "tree terminate took {elapsed:?}"
        );

        // Success path: reap parent; Drop still owns job for any residual.
        if let Some(mut parent) = cleanup.parent.take() {
            let _ = parent.wait();
        }
        let _ = cleanup.job.take();
    }

    #[test]
    fn context_bounds_are_deterministic_and_clamped_to_media_duration() {
        assert_eq!(context_bounds(4, 20, 120.0), (0.0, 35.0));
        assert_eq!(context_bounds(100, 119, 120.0), (85.0, 120.0));
        assert_eq!(context_bounds(30, 40, f64::NAN), (0.0, 0.0));
        assert_eq!(context_bounds(30, 40, 35.0), (15.0, 35.0));
    }

    #[test]
    fn candidate_ids_are_stable_for_same_input_and_distinguish_end_times() {
        let segments = vec![TranscriptSegment {
            start_seconds: 20.0,
            end_seconds: 24.0,
            text: "같은 입력".into(),
            quality_status: TranscriptQualityStatus::Certain,
            quality_reasons: Vec::new(),
        }];
        let energy = (0..60)
            .map(|second| EnergyPoint {
                start_seconds: second as f64,
                rms: if (15..45).contains(&second) { 0.8 } else { 0.1 },
            })
            .collect::<Vec<_>>();
        let first = build_candidates(60.0, 0.0, 60.0, &segments, &energy, &[]);
        let regenerated = build_candidates(60.0, 0.0, 60.0, &segments, &energy, &[]);
        let first_ids = first
            .iter()
            .map(|candidate| &candidate.id)
            .collect::<Vec<_>>();
        let regenerated_ids = regenerated
            .iter()
            .map(|candidate| &candidate.id)
            .collect::<Vec<_>>();
        assert_eq!(first_ids, regenerated_ids);
        assert_ne!(stable_candidate_id(15, 45), stable_candidate_id(15, 46));
    }

    #[test]
    fn preview_cache_key_distinguishes_every_context_dimension() {
        let base = preview_cache_key(
            "job-1",
            "candidate-1",
            "source-a",
            10.0,
            50.0,
            PreviewKind::Context,
        );
        for variant in [
            preview_cache_key(
                "job-2",
                "candidate-1",
                "source-a",
                10.0,
                50.0,
                PreviewKind::Context,
            ),
            preview_cache_key(
                "job-1",
                "candidate-2",
                "source-a",
                10.0,
                50.0,
                PreviewKind::Context,
            ),
            preview_cache_key(
                "job-1",
                "candidate-1",
                "source-b",
                10.0,
                50.0,
                PreviewKind::Context,
            ),
            preview_cache_key(
                "job-1",
                "candidate-1",
                "source-a",
                11.0,
                50.0,
                PreviewKind::Context,
            ),
            preview_cache_key(
                "job-1",
                "candidate-1",
                "source-a",
                10.0,
                51.0,
                PreviewKind::Context,
            ),
            preview_cache_key(
                "job-1",
                "candidate-1",
                "source-a",
                10.0,
                50.0,
                PreviewKind::Candidate,
            ),
        ] {
            assert_ne!(base, variant);
        }
    }

    #[test]
    fn preview_temporary_path_keeps_an_mp4_extension_for_ffmpeg() {
        let output = PathBuf::from("context-cache.mp4");
        assert_eq!(
            preview_temporary_path(&output),
            PathBuf::from("context-cache.tmp.mp4")
        );
    }

    #[test]
    fn quick_plan_uses_twenty_percent_for_an_eight_hour_video() {
        let plan = build_analysis_plan(AnalysisMode::Quick, 0, 8 * 60 * 60);
        let seconds = plan.iter().map(|chunk| chunk.length_seconds).sum::<f64>();
        assert_eq!(seconds, 8.0 * 60.0 * 60.0 * 0.20);
        assert_eq!(plan.len(), 10);
        assert!(plan.windows(2).all(|pair| {
            pair[0].offset_seconds + pair[0].length_seconds <= pair[1].offset_seconds
        }));
        assert!(
            plan.last().unwrap().offset_seconds + plan.last().unwrap().length_seconds
                <= 8.0 * 60.0 * 60.0 + 0.001
        );
    }

    #[test]
    fn range_plan_keeps_absolute_source_timecodes() {
        let bounds = analysis_bounds(7200.0, AnalysisMode::Range, Some(1800), Some(2700)).unwrap();
        let plan = build_analysis_plan(AnalysisMode::Range, bounds.0, bounds.1);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].offset_seconds, 1800.0);
        assert_eq!(plan[0].length_seconds, 600.0);
        assert_eq!(plan[1].offset_seconds, 2400.0);
        assert_eq!(plan[1].length_seconds, 300.0);
    }

    #[test]
    fn restricted_tool_environment_does_not_forward_parent_secrets() {
        let mut command = Command::new("fixture.exe");
        command.env("OPENAI_API_KEY", "must-not-pass");
        restrict_command_environment(&mut command);
        assert!(!command
            .get_envs()
            .any(|(name, value)| { name == "OPENAI_API_KEY" && value.is_some() }));
    }

    #[test]
    fn tolerates_invalid_utf8_bytes_in_whisper_srt() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("whisper.srt");
        let mut bytes = b"1\n00:00:01,000 --> 00:00:03,000\n\xed\x95\x9c\xea\xb8\x80 ".to_vec();
        bytes.push(0xff);
        bytes.extend_from_slice(b"\n");
        fs::write(&path, bytes).unwrap();
        let segments = parse_srt(&path, 10.0).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].start_seconds, 11.0);
        assert!(segments[0].text.contains("한글"));
    }

    #[test]
    fn ranks_audio_and_dialogue_without_fake_chat_score() {
        let segments = vec![TranscriptSegment {
            start_seconds: 10.0,
            end_seconds: 14.0,
            text: "이 구간은 발화가 많습니다 정말 많이 말합니다".into(),
            quality_status: TranscriptQualityStatus::Certain,
            quality_reasons: Vec::new(),
        }];
        let energy = (0..60)
            .map(|second| EnergyPoint {
                start_seconds: second as f64,
                rms: if second < 45 { 0.8 } else { 0.1 },
            })
            .collect::<Vec<_>>();
        let candidates = build_candidates(60.0, 0.0, 60.0, &segments, &energy, &[]);
        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].chat_score, None);
        assert!(candidates[0].total_score > 0);
        assert!(candidates[0].summary.contains("확인 가능한 채팅 영역 움직임 없음"));
    }

    #[test]
    fn preserves_suspect_repetition_with_diagnostic_quality_metadata() {
        let sanitized = sanitize_transcript_segments(vec![
            TranscriptSegment {
                start_seconds: 0.0,
                end_seconds: 4.0,
                text: "1/2 of the cream cheese. 1/2 of the cream cheese. 1/2 of the cream cheese."
                    .into(),
                quality_status: TranscriptQualityStatus::Certain,
                quality_reasons: Vec::new(),
            },
            TranscriptSegment {
                start_seconds: 10.0,
                end_seconds: 13.0,
                text: "이건 진짜 말도 안 되잖아".into(),
                quality_status: TranscriptQualityStatus::Certain,
                quality_reasons: Vec::new(),
            },
            TranscriptSegment {
                start_seconds: 20.0,
                end_seconds: 23.0,
                text: "이건 진짜 말도 안 되잖아".into(),
                quality_status: TranscriptQualityStatus::Certain,
                quality_reasons: Vec::new(),
            },
        ]);
        assert_eq!(sanitized.len(), 3);
        assert_eq!(sanitized[0].quality_status, TranscriptQualityStatus::Uncertain);
        assert_eq!(sanitized[1].quality_status, TranscriptQualityStatus::Certain);
        assert_eq!(sanitized[2].quality_status, TranscriptQualityStatus::Certain);
    }

    #[test]
    fn marks_short_repetition_and_replacement_character_without_deleting_evidence() {
        let sanitized = sanitize_transcript_segments(vec![
            TranscriptSegment { start_seconds: 0.0, end_seconds: 2.0, text: "너 너 너".into(), quality_status: TranscriptQualityStatus::Certain, quality_reasons: Vec::new() },
            TranscriptSegment { start_seconds: 4.0, end_seconds: 6.0, text: "노래 노래 노래".into(), quality_status: TranscriptQualityStatus::Certain, quality_reasons: Vec::new() },
            TranscriptSegment { start_seconds: 8.0, end_seconds: 10.0, text: "깨진 � 문장".into(), quality_status: TranscriptQualityStatus::Certain, quality_reasons: Vec::new() },
        ]);
        assert_eq!(sanitized.len(), 3);
        assert!(sanitized.iter().all(|segment| segment.quality_status == TranscriptQualityStatus::Uncertain));
        assert!(sanitized[2].quality_reasons.iter().any(|reason| reason.contains("U+FFFD")));
        let energy = (0..20).map(|second| EnergyPoint { start_seconds: second as f64, rms: 0.9 }).collect::<Vec<_>>();
        let candidates = build_candidates(20.0, 0.0, 20.0, &sanitized, &energy, &[]);
        assert!(!candidates.is_empty());
        assert!(candidates.iter().any(|candidate| candidate.transcript_quality_status == TranscriptQualityStatus::Uncertain && !candidate.title.contains('�') && !candidate.transcript_excerpt.contains('�')));
    }

    #[test]
    fn distinguishes_short_repetition_from_an_intentional_sentence() {
        for (text, uncertain) in [("아 아 아", true), ("가 가 가 가", true), ("라라라라", true), ("오늘은 정말 재미있는 방송입니다", false)] {
            let segment = sanitize_transcript_segments(vec![TranscriptSegment { start_seconds: 0.0, end_seconds: 2.0, text: text.into(), quality_status: TranscriptQualityStatus::Certain, quality_reasons: Vec::new() }]);
            assert_eq!(segment[0].quality_status == TranscriptQualityStatus::Uncertain, uncertain, "{text}");
        }
    }

    #[test]
    fn preserves_laughter_exclamation_and_song_raw_text_but_masks_display() {
        let raw = ["ㅋㅋㅋㅋ", "!!!", "라라라라"];
        let sanitized = sanitize_transcript_segments(raw.iter().enumerate().map(|(index, text)| TranscriptSegment {
            start_seconds: index as f64 * 3.0, end_seconds: index as f64 * 3.0 + 2.0, text: (*text).into(), quality_status: TranscriptQualityStatus::Certain, quality_reasons: Vec::new(),
        }).collect());
        assert_eq!(sanitized.iter().map(|segment| segment.text.as_str()).collect::<Vec<_>>(), raw);
        assert_eq!(sanitized.iter().map(|segment| segment.quality_status).collect::<Vec<_>>(), vec![TranscriptQualityStatus::Uncertain, TranscriptQualityStatus::Certain, TranscriptQualityStatus::Uncertain]);
        let context = context_transcript(&sanitized, 0.0, 10.0);
        assert!(context.iter().all(|line| !line.text.contains('�')));
    }

    #[test]
    fn produces_non_overlapping_candidates_with_real_chat_motion_scores() {
        let segments = (0..12)
            .map(|index| TranscriptSegment {
                start_seconds: index as f64 * 10.0,
                end_seconds: index as f64 * 10.0 + 4.0,
                text: format!("서로 다른 한국어 발화 구간 {index}"),
                quality_status: TranscriptQualityStatus::Certain,
                quality_reasons: Vec::new(),
            })
            .collect::<Vec<_>>();
        let energy = (0..120)
            .map(|second| EnergyPoint {
                start_seconds: second as f64,
                rms: (second % 30) as f64 / 30.0,
            })
            .collect::<Vec<_>>();
        let motion = (1..24)
            .map(|index| ChatMotionPoint {
                start_seconds: index as f64 * CHAT_SAMPLE_SECONDS,
                motion: if index > 12 { 0.8 } else { 0.1 },
            })
            .collect::<Vec<_>>();
        let candidates = build_candidates(120.0, 0.0, 120.0, &segments, &energy, &motion);
        assert!(candidates
            .iter()
            .all(|candidate| candidate.chat_score.is_some()));
        for (index, left) in candidates.iter().enumerate() {
            for right in candidates.iter().skip(index + 1) {
                assert!(
                    left.end_seconds <= right.start_seconds
                        || right.end_seconds <= left.start_seconds
                );
            }
        }
    }

    #[test]
    fn range_candidates_stay_inside_requested_bounds() {
        let segments = (0..40)
            .map(|index| TranscriptSegment {
                start_seconds: index as f64 * 30.0,
                end_seconds: index as f64 * 30.0 + 4.0,
                text: format!("범위 검증 발화 {index}"),
                quality_status: TranscriptQualityStatus::Certain,
                quality_reasons: Vec::new(),
            })
            .collect::<Vec<_>>();
        let energy = (0..1200)
            .map(|second| EnergyPoint {
                start_seconds: second as f64,
                rms: 0.5,
            })
            .collect::<Vec<_>>();
        let candidates = build_candidates(1200.0, 180.0, 360.0, &segments, &energy, &[]);
        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|candidate| {
            candidate.start_seconds as f64 >= 180.0 - 0.001
                && candidate.end_seconds as f64 <= 360.0 + 0.001
        }));
    }

    #[test]
    fn empty_or_inverted_range_yields_no_candidates() {
        let segments = vec![TranscriptSegment {
            start_seconds: 1.0,
            end_seconds: 3.0,
            text: "범위 밖 발화".into(),
            quality_status: TranscriptQualityStatus::Certain,
            quality_reasons: Vec::new(),
        }];
        let energy = vec![EnergyPoint {
            start_seconds: 1.0,
            rms: 0.9,
        }];
        assert!(build_candidates(60.0, 30.0, 30.0, &segments, &energy, &[]).is_empty());
        assert!(build_candidates(60.0, 50.0, 10.0, &segments, &energy, &[]).is_empty());
        assert!(build_candidates(60.0, 100.0, 120.0, &segments, &energy, &[]).is_empty());
    }

    #[test]
    fn excludes_windows_without_audio_or_dialogue_evidence() {
        // Chat motion spans the full timeline, but energy/dialogue only cover 0–60s.
        let segments = vec![TranscriptSegment {
            start_seconds: 10.0,
            end_seconds: 14.0,
            text: "근거가 있는 구간만 후보가 됩니다".into(),
            quality_status: TranscriptQualityStatus::Certain,
            quality_reasons: Vec::new(),
        }];
        let energy = (0..60)
            .map(|second| EnergyPoint {
                start_seconds: second as f64,
                rms: 0.7,
            })
            .collect::<Vec<_>>();
        let motion = (1..40)
            .map(|index| ChatMotionPoint {
                start_seconds: index as f64 * 15.0,
                motion: 0.9,
            })
            .collect::<Vec<_>>();
        let candidates = build_candidates(600.0, 0.0, 600.0, &segments, &energy, &motion);
        assert!(!candidates.is_empty());
        assert!(candidates
            .iter()
            .all(|candidate| candidate.end_seconds <= 60 + 45));
        assert!(candidates
            .iter()
            .all(|candidate| candidate.total_score > 0));
    }

    #[test]
    fn excludes_chat_only_and_all_evidence_free_windows() {
        let motion = (1..20)
            .map(|index| ChatMotionPoint { start_seconds: index as f64 * CHAT_SAMPLE_SECONDS, motion: 0.9 })
            .collect::<Vec<_>>();
        let silent_energy = (0..120)
            .map(|second| EnergyPoint { start_seconds: second as f64, rms: 0.0 })
            .collect::<Vec<_>>();

        assert!(build_candidates(120.0, 0.0, 120.0, &[], &silent_energy, &motion).is_empty());
        assert!(build_candidates(120.0, 0.0, 120.0, &[], &[], &[]).is_empty());
    }

    #[test]
    fn keeps_audio_only_and_speaking_only_windows_with_reasons() {
        let audio_only = build_candidates(60.0, 0.0, 60.0, &[], &(0..60).map(|second| EnergyPoint { start_seconds: second as f64, rms: 0.5 }).collect::<Vec<_>>(), &[]);
        assert!(!audio_only.is_empty());
        assert!(audio_only.iter().all(|candidate| candidate.chat_score.is_none()));
        assert!(audio_only.iter().any(|candidate| candidate.selection_reasons.iter().any(|reason| reason.contains("오디오 근거 유지"))));

        let speaking_only = build_candidates(
            60.0, 0.0, 60.0,
            &[TranscriptSegment { start_seconds: 10.0, end_seconds: 14.0, text: "조용하지만 이어지는 말하기 근거".into(), quality_status: TranscriptQualityStatus::Certain, quality_reasons: Vec::new() }],
            &(0..60).map(|second| EnergyPoint { start_seconds: second as f64, rms: 0.0 }).collect::<Vec<_>>(), &[],
        );
        assert!(!speaking_only.is_empty());
        assert!(speaking_only.iter().any(|candidate| candidate.selection_reasons.iter().any(|reason| reason.contains("말하기 근거 유지"))));
    }

    #[test]
    fn checkpoint_compatibility_requires_fingerprint_tools_and_ranker() {
        let runtime = HashMap::from([
            ("ffmpeg/ffmpeg.exe".into(), "aaa".into()),
            ("whisper/whisper-cli.exe".into(), "bbb".into()),
        ]);
        let checkpoint = MediaCheckpoint::fresh(
            "C:/media/source.mp4",
            120.0,
            AnalysisMode::Range,
            10,
            90,
            vec![PlannedChunk {
                offset_seconds: 10.0,
                length_seconds: 80.0,
            }],
            "fingerprint-a".into(),
            1024,
            runtime.clone(),
        );
        assert!(checkpoint_is_compatible(
            &checkpoint,
            "C:/media/source.mp4",
            AnalysisMode::Range,
            10,
            Some(90),
            "fingerprint-a",
            1024,
            &runtime,
        ));
        assert!(!checkpoint_is_compatible(
            &checkpoint,
            "C:/media/source.mp4",
            AnalysisMode::Range,
            10,
            Some(90),
            "fingerprint-b",
            1024,
            &runtime,
        ));
        let mut stale_ranker = checkpoint.clone();
        stale_ranker.ranker_version = "rules-v0.3.2".into();
        assert!(!checkpoint_is_compatible(
            &stale_ranker,
            "C:/media/source.mp4",
            AnalysisMode::Range,
            10,
            Some(90),
            "fingerprint-a",
            1024,
            &runtime,
        ));
        let mut stale_tools = checkpoint.clone();
        stale_tools
            .runtime_sha256
            .insert("ffmpeg/ffmpeg.exe".into(), "changed".into());
        assert!(!checkpoint_is_compatible(
            &stale_tools,
            "C:/media/source.mp4",
            AnalysisMode::Range,
            10,
            Some(90),
            "fingerprint-a",
            1024,
            &runtime,
        ));
        let mut schema_v3 = checkpoint.clone();
        schema_v3.schema_version = 3;
        assert!(!checkpoint_is_compatible(
            &schema_v3,
            "C:/media/source.mp4",
            AnalysisMode::Range,
            10,
            Some(90),
            "fingerprint-a",
            1024,
            &runtime,
        ));
        let mut missing_language = checkpoint.clone();
        missing_language.language.clear();
        assert!(!checkpoint_is_compatible(
            &missing_language,
            "C:/media/source.mp4",
            AnalysisMode::Range,
            10,
            Some(90),
            "fingerprint-a",
            1024,
            &runtime,
        ));
        // Schema-4 JSON without language/ranker must not invent matching defaults.
        let partial_json = serde_json::json!({
            "schemaVersion": 4,
            "sourcePath": "C:/media/source.mp4",
            "durationSeconds": 120.0,
            "chunkSeconds": 600.0,
            "analysisMode": "range",
            "analysisStartSeconds": 10,
            "analysisEndSeconds": 90,
            "inputFingerprint": "fingerprint-a",
            "inputBytes": 1024,
            "runtimeSha256": runtime,
            "plannedChunks": [],
            "completedChunks": 0,
            "segments": [],
            "energy": [],
        });
        let partial: MediaCheckpoint = serde_json::from_value(partial_json).unwrap();
        assert!(partial.language.is_empty());
        assert!(partial.ranker_version.is_empty());
        assert!(!checkpoint_is_compatible(
            &partial,
            "C:/media/source.mp4",
            AnalysisMode::Range,
            10,
            Some(90),
            "fingerprint-a",
            1024,
            &runtime,
        ));
    }

    #[test]
    fn checkpoint_caption_identity_includes_content_hash_and_verification_state() {
        let runtime = HashMap::new();
        let mut checkpoint = MediaCheckpoint::fresh(
            "source.mp4",
            60.0,
            AnalysisMode::Full,
            0,
            60,
            vec![PlannedChunk {
                offset_seconds: 0.0,
                length_seconds: 60.0,
            }],
            "fingerprint".into(),
            10,
            runtime.clone(),
        );
        let provenance = CaptionProvenance {
            schema_version: captions::CAPTION_SCHEMA_VERSION,
            source_url: "video".into(),
            source: captions::CaptionSource::Creator,
            language: "ko".into(),
            track_id: "ko".into(),
            revision: "r1".into(),
            original_file: "captions/ko.vtt".into(),
            sha256: "metadata-hash".into(),
            verification_state: VerificationState::Failed,
            diagnostics: Vec::new(),
            content_sha256: "actual-hash".into(),
        };
        apply_caption_identity(&mut checkpoint, &provenance);
        assert!(checkpoint_is_compatible_with_caption(
            &checkpoint,
            "source.mp4",
            AnalysisMode::Full,
            0,
            None,
            "fingerprint",
            10,
            &runtime,
            Some(&provenance),
        ));
        let mut tampered = provenance.clone();
        tampered.content_sha256 = "tampered-hash".into();
        assert!(!checkpoint_is_compatible_with_caption(
            &checkpoint,
            "source.mp4",
            AnalysisMode::Full,
            0,
            None,
            "fingerprint",
            10,
            &runtime,
            Some(&tampered),
        ));
        let mut reverified = provenance.clone();
        reverified.verification_state = VerificationState::Verified;
        assert!(!checkpoint_is_compatible_with_caption(
            &checkpoint,
            "source.mp4",
            AnalysisMode::Full,
            0,
            None,
            "fingerprint",
            10,
            &runtime,
            Some(&reverified),
        ));
    }

    #[test]
    fn discarded_incompatible_checkpoint_restarts_media_when_job_units_advanced() {
        // P0 F1: schema-3 / fingerprint-mismatched live is discarded → fresh CP with
        // completed_chunks=0. An interrupted job snapshot with completed_units > 2 must
        // recompute media intermediates, not hard-fail resume.
        let runtime = HashMap::from([("ffmpeg/ffmpeg.exe".into(), "aaa".into())]);
        let planned = vec![
            PlannedChunk {
                offset_seconds: 0.0,
                length_seconds: 600.0,
            },
            PlannedChunk {
                offset_seconds: 600.0,
                length_seconds: 600.0,
            },
            PlannedChunk {
                offset_seconds: 1200.0,
                length_seconds: 600.0,
            },
            PlannedChunk {
                offset_seconds: 1800.0,
                length_seconds: 600.0,
            },
        ];
        let mut schema3 = MediaCheckpoint::fresh(
            "C:/media/source.mp4",
            2400.0,
            AnalysisMode::Full,
            0,
            2400,
            planned.clone(),
            "fp-old".into(),
            4096,
            runtime.clone(),
        );
        schema3.schema_version = 3;
        schema3.completed_chunks = 3;
        schema3.segments.push(TranscriptSegment {
            start_seconds: 10.0,
            end_seconds: 12.0,
            text: "이전 스키마 결과".into(),
            quality_status: TranscriptQualityStatus::Certain,
            quality_reasons: Vec::new(),
        });
        assert!(!checkpoint_is_compatible(
            &schema3,
            "C:/media/source.mp4",
            AnalysisMode::Full,
            0,
            None,
            "fp-new",
            4096,
            &runtime,
        ));

        // Simulate load_checkpoint → None then MediaCheckpoint::fresh (completed_chunks = 0).
        let mut rebuilt = MediaCheckpoint::fresh(
            "C:/media/source.mp4",
            2400.0,
            AnalysisMode::Full,
            0,
            2400,
            planned,
            "fp-new".into(),
            4096,
            runtime.clone(),
        );
        // Job snapshot advanced past probe + 3 chunks (units = 2 + 3 = 5).
        let outcome =
            align_checkpoint_with_job_units(&mut rebuilt, 5, true).expect("must restart, not fail");
        assert_eq!(outcome, CheckpointAlignResult::RestartMediaFromScratch);
        assert_eq!(rebuilt.completed_chunks, 0);
        assert!(rebuilt.segments.is_empty());
        assert_eq!(job_units_after_media_restart(&rebuilt), 2);

        // Fingerprint mismatch rebuild path is the same: allow restart with advanced units.
        let mut fp_mismatch_fresh = MediaCheckpoint::fresh(
            "C:/media/source.mp4",
            2400.0,
            AnalysisMode::Full,
            0,
            2400,
            vec![PlannedChunk {
                offset_seconds: 0.0,
                length_seconds: 600.0,
            }],
            "fp-current".into(),
            4096,
            runtime.clone(),
        );
        assert_eq!(
            align_checkpoint_with_job_units(&mut fp_mismatch_fresh, 4, true).unwrap(),
            CheckpointAlignResult::RestartMediaFromScratch
        );

        // Compatible checkpoint lagging the snapshot remains a hard integrity error.
        let mut partial = MediaCheckpoint::fresh(
            "C:/media/source.mp4",
            2400.0,
            AnalysisMode::Full,
            0,
            2400,
            vec![
                PlannedChunk {
                    offset_seconds: 0.0,
                    length_seconds: 600.0,
                },
                PlannedChunk {
                    offset_seconds: 600.0,
                    length_seconds: 600.0,
                },
                PlannedChunk {
                    offset_seconds: 1200.0,
                    length_seconds: 600.0,
                },
            ],
            "fp-current".into(),
            4096,
            runtime,
        );
        partial.completed_chunks = 1;
        partial.segments.push(TranscriptSegment {
            start_seconds: 1.0,
            end_seconds: 2.0,
            text: "부분 완료".into(),
            quality_status: TranscriptQualityStatus::Certain,
            quality_reasons: Vec::new(),
        });
        let err = align_checkpoint_with_job_units(&mut partial, 5, false).unwrap_err();
        assert!(err.contains("작업 스냅샷보다 미디어 체크포인트가 뒤에"));
        assert_eq!(partial.completed_chunks, 1);
        assert_eq!(partial.segments.len(), 1);

        // Compatible rewind path still trims intermediate media to the snapshot.
        let mut ahead = partial.clone();
        ahead.completed_chunks = 3;
        ahead.segments.push(TranscriptSegment {
            start_seconds: 700.0,
            end_seconds: 702.0,
            text: "스냅샷 이후".into(),
            quality_status: TranscriptQualityStatus::Certain,
            quality_reasons: Vec::new(),
        });
        assert_eq!(
            align_checkpoint_with_job_units(&mut ahead, 3, false).unwrap(),
            CheckpointAlignResult::Rewound
        );
        assert_eq!(ahead.completed_chunks, 1);
        assert!(ahead
            .segments
            .iter()
            .all(|segment| segment.start_seconds < 600.0));
    }

    #[test]
    fn load_legacy_schema4_resumes_completed_cpu_chunks() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("media-checkpoint.json");
        let runtime = HashMap::from([("ffmpeg/ffmpeg.exe".into(), "aaa".into())]);
        let mut schema4 = MediaCheckpoint::fresh(
            "source.mp4",
            1200.0,
            AnalysisMode::Full,
            0,
            1200,
            vec![
                PlannedChunk {
                    offset_seconds: 0.0,
                    length_seconds: 600.0,
                },
                PlannedChunk {
                    offset_seconds: 600.0,
                    length_seconds: 600.0,
                },
            ],
            "fp".into(),
            2048,
            runtime.clone(),
        );
        schema4.schema_version = 4;
        schema4.completed_chunks = 2;
        save_checkpoint(&path, &schema4).unwrap();

        let legacy_cpu = legacy_cpu_whisper_settings();
        let loaded = load_checkpoint_with_caption(
            &path,
            "source.mp4",
            AnalysisMode::Full,
            None,
            None,
            "fp",
            2048,
            &runtime,
            None,
            &legacy_cpu,
        )
        .unwrap();
        let loaded = loaded.expect("schema4 legacy CPU checkpoint remains resumable");
        assert_eq!(loaded.completed_chunks, 2);

        let auto = WhisperSettings::default();
        assert!(load_checkpoint_with_caption(
            &path,
            "source.mp4",
            AnalysisMode::Full,
            None,
            None,
            "fp",
            2048,
            &runtime,
            None,
            &auto,
        )
        .unwrap()
        .is_none());
        let gpu = WhisperSettings {
            device_mode: WhisperDeviceMode::Gpu,
            ..legacy_cpu.clone()
        };
        assert!(load_checkpoint_with_caption(
            &path,
            "source.mp4",
            AnalysisMode::Full,
            None,
            None,
            "fp",
            2048,
            &runtime,
            None,
            &gpu,
        )
        .unwrap()
        .is_none());

        let mut schema5 = schema4.clone();
        schema5.schema_version = 5;
        schema5.whisper_settings = auto.clone();
        assert!(checkpoint_is_compatible_with_whisper(
            &schema5,
            "source.mp4",
            AnalysisMode::Full,
            0,
            None,
            "fp",
            2048,
            &runtime,
            None,
            &auto,
        ));
        assert!(!checkpoint_is_compatible_with_whisper(
            &schema5,
            "source.mp4",
            AnalysisMode::Full,
            0,
            None,
            "fp",
            2048,
            &runtime,
            None,
            &gpu,
        ));

        // Fresh rebuild + advanced job units must restart media chunks from 0.
        let mut fresh = MediaCheckpoint::fresh(
            "source.mp4",
            1200.0,
            AnalysisMode::Full,
            0,
            1200,
            vec![
                PlannedChunk {
                    offset_seconds: 0.0,
                    length_seconds: 600.0,
                },
                PlannedChunk {
                    offset_seconds: 600.0,
                    length_seconds: 600.0,
                },
            ],
            "fp".into(),
            2048,
            runtime,
        );
        let outcome = align_checkpoint_with_job_units(&mut fresh, 5, true).unwrap();
        assert_eq!(outcome, CheckpointAlignResult::RestartMediaFromScratch);
        assert_eq!(fresh.completed_chunks, 0);
        // Job identity/source fields stay on the job snapshot; media units clamp to probe.
        assert_eq!(job_units_after_media_restart(&fresh), 2);
    }

    #[test]
    fn gpu_evidence_requires_backend_marker_and_nonempty_output_is_checked_by_runner() {
        assert!(has_gpu_backend_evidence(
            "ggml_cuda_init: found 1 CUDA device(s)",
            "using CUDA backend"
        ));
        assert!(has_gpu_backend_evidence(
            "ggml_cuda_init: found 2 CUDA devices",
            "using CUDA0 backend"
        ));
        assert!(!has_gpu_backend_evidence(
            "ggml_cuda_init: found 0 CUDA device(s)",
            "using CUDA backend"
        ));
        assert!(!has_gpu_backend_evidence(
            "ggml_cuda_init: found 1 CUDA device(s)",
            "CUDA error: initialization failed"
        ));
        assert!(!has_gpu_backend_evidence(
            "ggml_cuda_init: found 1 CUDA devices",
            "using CUDA0 backend; CUDA error: initialization failed"
        ));
        assert!(!has_gpu_backend_evidence(
            "ggml_cuda_init: found 1 CUDA devices error",
            "using CUDA0 backend"
        ));
        assert!(!has_gpu_backend_evidence("", "cuBLAS loaded"));
        assert!(!has_gpu_backend_evidence("NVIDIA GeForce present", ""));
    }

    #[test]
    fn cpu_whisper_args_explicitly_disable_gpu() {
        let settings = WhisperSettings::default();
        let args = whisper_command_args(
            &settings,
            Path::new("model.bin"),
            Path::new("input.wav"),
            Path::new("output"),
            4,
            false,
        );
        assert!(args.iter().any(|arg| arg == "-ng"));
    }

    #[test]
    fn load_checkpoint_rejects_stale_and_recovers_previous_generation() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("media-checkpoint.json");
        let runtime = HashMap::from([("ffmpeg/ffmpeg.exe".into(), "aaa".into())]);
        let good = MediaCheckpoint::fresh(
            "source.mp4",
            90.0,
            AnalysisMode::Full,
            0,
            90,
            vec![PlannedChunk {
                offset_seconds: 0.0,
                length_seconds: 90.0,
            }],
            "fp-good".into(),
            2048,
            runtime.clone(),
        );
        save_checkpoint(&path, &good).unwrap();
        let mut stale = good.clone();
        stale.input_fingerprint = "fp-stale".into();
        save_checkpoint(&path, &stale).unwrap();
        // Live file is stale for the current fingerprint; must not resume from .prev either.
        let rejected = load_checkpoint(
            &path,
            "source.mp4",
            AnalysisMode::Full,
            None,
            None,
            "fp-good",
            2048,
            &runtime,
        )
        .unwrap();
        assert!(rejected.is_none());
        // Drop live file so reader falls back to the previous good generation.
        fs::remove_file(&path).unwrap();
        let recovered = load_checkpoint(
            &path,
            "source.mp4",
            AnalysisMode::Full,
            None,
            None,
            "fp-good",
            2048,
            &runtime,
        )
        .unwrap()
        .expect("previous generation should resume");
        assert_eq!(recovered.input_fingerprint, "fp-good");

        // Corrupt (non-empty garbage) live must recover .prev, not silently recompute.
        save_checkpoint(&path, &good).unwrap();
        let mut next = good.clone();
        next.completed_chunks = 1;
        save_checkpoint(&path, &next).unwrap();
        fs::write(&path, b"{not-json").unwrap();
        let from_corrupt = load_checkpoint(
            &path,
            "source.mp4",
            AnalysisMode::Full,
            None,
            None,
            "fp-good",
            2048,
            &runtime,
        )
        .unwrap()
        .expect("corrupt live should fall back to previous good generation");
        assert_eq!(from_corrupt.completed_chunks, 0);
        assert_eq!(from_corrupt.input_fingerprint, "fp-good");
    }

    #[test]
    fn disk_space_guard_explains_shortage_before_start() {
        let required = estimate_analysis_workspace_bytes(7_000_000_000, 8.0 * 3600.0);
        assert!(required > 512 * MIB);
        // Estimate is independent of source_bytes (local source already on disk).
        assert_eq!(
            estimate_analysis_workspace_bytes(1, 8.0 * 3600.0),
            estimate_analysis_workspace_bytes(u64::MAX / 2, 8.0 * 3600.0)
        );
        let err = ensure_sufficient_disk_space(1_000, required).unwrap_err();
        assert!(err.contains("저장 공간이 부족합니다"));
        assert!(err.contains("확보"));
        assert!(err.contains("GB") || err.contains("MB"));
        assert!(ensure_sufficient_disk_space(required, required).is_ok());
        assert!(ensure_sufficient_disk_space(required - 1, required).is_err());
        // Non-finite duration must not panic or wrap wildly.
        let finiteish = estimate_analysis_workspace_bytes(0, f64::NAN);
        assert!(finiteish > 512 * MIB);
        assert!(finiteish < 8 * 1024 * MIB);
    }

    #[test]
    #[ignore = "bundled FFmpeg, Whisper model, and VOD_SCOUT_SMOKE_VIDEO are required"]
    fn bundled_pipeline_reaches_review_ready_without_a_window() {
        let source = env::var("VOD_SCOUT_SMOKE_VIDEO")
            .expect("VOD_SCOUT_SMOKE_VIDEO must be an absolute media path");
        let temp = tempfile::tempdir().unwrap();
        let tools = locate_tools(temp.path()).unwrap();
        let source = PathBuf::from(source);
        let cancel = AtomicBool::new(false);
        let probe_json = temp.path().join("ffprobe.json");
        run_command(
            &cancel,
            &tools.ffprobe,
            &tools.ffmpeg_dir,
            [
                "-v".into(),
                "error".into(),
                "-show_entries".into(),
                "format=duration:stream=codec_type".into(),
                "-of".into(),
                "json".into(),
                source.as_os_str().into(),
            ],
            &probe_json,
            &temp.path().join("ffprobe.stderr.log"),
        )
        .unwrap();
        let probe: ProbeOutput = serde_json::from_slice(&fs::read(&probe_json).unwrap()).unwrap();
        let duration = probe.format.duration.unwrap().parse::<f64>().unwrap();
        assert!(probe
            .streams
            .iter()
            .any(|stream| stream.codec_type.as_deref() == Some("audio")));

        let wav = temp.path().join("chunk.wav");
        run_command(
            &cancel,
            &tools.ffmpeg,
            &tools.ffmpeg_dir,
            [
                "-hide_banner".into(),
                "-loglevel".into(),
                "error".into(),
                "-y".into(),
                "-i".into(),
                source.as_os_str().into(),
                "-vn".into(),
                "-ac".into(),
                "1".into(),
                "-ar".into(),
                "16000".into(),
                "-c:a".into(),
                "pcm_s16le".into(),
                wav.as_os_str().into(),
            ],
            &temp.path().join("ffmpeg.stdout.log"),
            &temp.path().join("ffmpeg.stderr.log"),
        )
        .unwrap();
        let energy = analyze_wav(&wav, 0.0).unwrap();

        let output_prefix = temp.path().join("transcript");
        run_command(
            &cancel,
            &tools.whisper,
            &tools.whisper_dir,
            [
                "-m".into(),
                tools.model.as_os_str().into(),
                "-f".into(),
                wav.as_os_str().into(),
                "-l".into(),
                "auto".into(),
                "-osrt".into(),
                "-of".into(),
                output_prefix.as_os_str().into(),
                "-np".into(),
                "-t".into(),
                "4".into(),
            ],
            &temp.path().join("whisper.stdout.log"),
            &temp.path().join("whisper.stderr.log"),
        )
        .unwrap();
        let segments = parse_srt(&output_prefix.with_extension("srt"), 0.0).unwrap();
        let candidates = build_candidates(duration, 0.0, duration, &segments, &energy, &[]);
        assert!(!candidates.is_empty());
        assert!(candidates
            .iter()
            .all(|candidate| candidate.chat_score.is_none()));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.transcript_excerpt.contains("fellow Americans")));

        let (fingerprint, bytes) = source_fingerprint(&source).unwrap();
        let checkpoint = MediaCheckpoint::fresh(
            &source.display().to_string(),
            duration,
            AnalysisMode::Full,
            0,
            duration.ceil() as u32,
            vec![PlannedChunk {
                offset_seconds: 0.0,
                length_seconds: duration,
            }],
            fingerprint,
            bytes,
            runtime_hashes().unwrap_or_default(),
        );
        let mut checkpoint = checkpoint;
        checkpoint.completed_chunks = 1;
        checkpoint.segments = segments;
        checkpoint.energy = energy;
        let checkpoint_path = temp.path().join("media-checkpoint.json");
        save_checkpoint(&checkpoint_path, &checkpoint).unwrap();
        assert!(checkpoint_path.is_file());
    }
}
