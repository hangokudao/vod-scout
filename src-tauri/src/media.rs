use super::{mutate_job, AppState};
use crate::domain::{
    AnalysisMode, Candidate, CandidateDecision, ContextTranscriptEntry, JobStatus, SourceKind,
};
use crate::integrity::{runtime_hashes, source_fingerprint, verify_runtime_bundle};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering as CmpOrdering;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

const CHUNK_SECONDS: f64 = 600.0;
const CHAT_SAMPLE_SECONDS: f64 = 5.0;
const QUICK_CHAT_SAMPLE_SECONDS: f64 = 15.0;
const CHAT_FRAME_SIDE: usize = 64;
const CONTEXT_PADDING_SECONDS: f64 = 15.0;
pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
pub(crate) struct KillOnCloseJob(HANDLE);

#[cfg(windows)]
impl KillOnCloseJob {
    pub(crate) fn attach(child: &std::process::Child) -> Result<Self, std::io::Error> {
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
}

#[cfg(windows)]
impl Drop for KillOnCloseJob {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[derive(Debug)]
enum PipelineError {
    Cancelled,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaCheckpoint {
    schema_version: u8,
    source_path: String,
    duration_seconds: f64,
    chunk_seconds: f64,
    analysis_mode: AnalysisMode,
    analysis_start_seconds: u32,
    analysis_end_seconds: u32,
    planned_chunks: Vec<PlannedChunk>,
    completed_chunks: u32,
    segments: Vec<TranscriptSegment>,
    energy: Vec<EnergyPoint>,
    #[serde(default)]
    chat_motion_completed: bool,
    #[serde(default)]
    chat_motion: Vec<ChatMotionPoint>,
}

impl MediaCheckpoint {
    fn fresh(
        source_path: &str,
        duration_seconds: f64,
        analysis_mode: AnalysisMode,
        analysis_start_seconds: u32,
        analysis_end_seconds: u32,
        planned_chunks: Vec<PlannedChunk>,
    ) -> Self {
        Self {
            schema_version: 3,
            source_path: source_path.into(),
            duration_seconds,
            chunk_seconds: CHUNK_SECONDS,
            analysis_mode,
            analysis_start_seconds,
            analysis_end_seconds,
            planned_chunks,
            completed_chunks: 0,
            segments: Vec::new(),
            energy: Vec::new(),
            chat_motion_completed: false,
            chat_motion: Vec::new(),
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

#[derive(Debug)]
struct WindowScore {
    start: f64,
    end: f64,
    audio_raw: f64,
    dialogue_raw: f64,
    chat_raw: Option<f64>,
    excerpt: String,
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
    let result = run(&app, &state, &job_id);
    match result {
        Ok(candidates) => {
            let count = candidates.len();
            let _ = mutate_job(&app, &state, |job| {
                let source_label = if job.source_kind == SourceKind::Youtube {
                    "YouTube 영상"
                } else {
                    "로컬 영상"
                };
                job.candidates = candidates;
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
            let _ = mutate_job(&app, &state, |job| {
                if job.status != JobStatus::Cancelling && job.status.is_active() {
                    job.transition(JobStatus::Cancelling)?;
                }
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
        Err(PipelineError::Message(detail)) => {
            let _ = mutate_job(&app, &state, |job| {
                if job.status == JobStatus::Cancelling {
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
    state.running.store(false, Ordering::SeqCst);
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

pub(crate) fn prepare_candidate_preview(
    state: &Arc<AppState>,
    job_id: &str,
    candidate_id: &str,
) -> Result<PreviewMedia, String> {
    prepare_preview(state, job_id, candidate_id, PreviewKind::Candidate)
}

pub(crate) fn prepare_candidate_context_preview(
    state: &Arc<AppState>,
    job_id: &str,
    candidate_id: &str,
) -> Result<PreviewMedia, String> {
    prepare_preview(state, job_id, candidate_id, PreviewKind::Context)
}

fn prepare_preview(
    state: &Arc<AppState>,
    job_id: &str,
    candidate_id: &str,
    preview_kind: PreviewKind,
) -> Result<PreviewMedia, String> {
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
        let temporary = preview_dir.join(format!(
            "{}.tmp",
            preview_output_name(&cache_key, preview_kind)
        ));
        fs::remove_file(&temporary).ok();
        let never_cancel = AtomicBool::new(false);
        let result = run_command(
            &never_cancel,
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
                PipelineError::Message(message) => {
                    format!("후보 영상을 준비하지 못했습니다: {message}")
                }
            });
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
    let (source_path, completed_units, analysis_mode, requested_start, requested_end) = {
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

    let checkpoint_path = job_dir.join("media-checkpoint.json");
    let mut checkpoint = load_checkpoint(
        &checkpoint_path,
        &source_path,
        analysis_mode,
        requested_start,
        requested_end,
    )?;
    if checkpoint.is_none() {
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
        ));
    }
    let mut checkpoint = checkpoint.expect("checkpoint initialized");
    let chunk_count = checkpoint.planned_chunks.len().max(1) as u32;
    let total_units = chunk_count + 6;

    let snapshot_chunks = completed_units.saturating_sub(2).min(chunk_count);
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
    } else if checkpoint.completed_chunks < snapshot_chunks {
        return Err(PipelineError::Message(
            "작업 스냅샷보다 미디어 체크포인트가 뒤에 있어 자동 재개할 수 없습니다.".into(),
        ));
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
                    "{} 범위 {}초를 전사 청크 {}개로 계획했습니다.",
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
    } else {
        let duration = checkpoint.duration_seconds;
        let _ = mutate_job(app, state, |job| {
            job.total_units = total_units;
            job.media_duration_seconds = Some(duration);
            Ok(())
        })
        .map_err(PipelineError::Message)?;
    }
    save_checkpoint(&checkpoint_path, &checkpoint)?;
    write_pipeline_provenance(&job_dir, &source, &checkpoint)?;

    for chunk_index in checkpoint.completed_chunks..chunk_count {
        check_cancel(state)?;
        let planned = checkpoint.planned_chunks[chunk_index as usize].clone();
        let offset = planned.offset_seconds;
        let length = planned.length_seconds.max(0.1);
        let wav = job_dir.join("active-chunk.wav");
        let output_prefix = job_dir.join("active-transcript");
        let srt = output_prefix.with_extension("srt");
        fs::remove_file(&wav).ok();
        fs::remove_file(&srt).ok();

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

        let mut energy = analyze_wav(&wav, offset)?;
        let threads = thread::available_parallelism()
            .map(|count| count.get().saturating_sub(1).clamp(1, 8))
            .unwrap_or(4);
        run_command(
            &state.cancel_requested,
            &tools.whisper,
            tools.whisper_dir.as_path(),
            [
                "-m".into(),
                tools.model.as_os_str().into(),
                "-f".into(),
                wav.as_os_str().into(),
                "-l".into(),
                "ko".into(),
                "-nth".into(),
                "0.72".into(),
                "-nf".into(),
                "-sns".into(),
                "-sow".into(),
                "-osrt".into(),
                "-of".into(),
                output_prefix.as_os_str().into(),
                "-np".into(),
                "-t".into(),
                threads.to_string().into(),
            ],
            &log_dir.join(format!("whisper-{chunk_index:04}.stdout.log")),
            &log_dir.join(format!("whisper-{chunk_index:04}.stderr.log")),
        )?;

        let mut segments = sanitize_transcript_segments(parse_srt(&srt, offset)?);
        checkpoint.segments.append(&mut segments);
        checkpoint.energy.append(&mut energy);
        checkpoint.completed_chunks = chunk_index + 1;
        save_checkpoint(&checkpoint_path, &checkpoint)?;
        write_transcript(&job_dir.join("transcript.json"), &checkpoint.segments)?;
        fs::remove_file(&wav).ok();
        fs::remove_file(&srt).ok();

        progress(
            app,
            state,
            chunk_index + 3,
            JobStatus::Transcribing,
            &format!("전사 {}/{}", chunk_index + 1, chunk_count),
            &format!(
                "오디오 청크 {}/{}를 추출하고 전사했습니다.",
                chunk_index + 1,
                chunk_count
            ),
        )?;
    }

    checkpoint.segments = sanitize_transcript_segments(std::mem::take(&mut checkpoint.segments));
    checkpoint.schema_version = 3;
    save_checkpoint(&checkpoint_path, &checkpoint)?;
    write_transcript(&job_dir.join("transcript.json"), &checkpoint.segments)?;

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
        match motion_result {
            Ok(points) => checkpoint.chat_motion = points,
            Err(PipelineError::Cancelled) => return Err(PipelineError::Cancelled),
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
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
    whisper: PathBuf,
    model: PathBuf,
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
        let paths = ToolPaths {
            ffmpeg: ffmpeg_dir.join("ffmpeg.exe"),
            ffprobe: ffmpeg_dir.join("ffprobe.exe"),
            whisper: whisper_dir.join("whisper-cli.exe"),
            model: root.join("models").join("ggml-base.bin"),
            ffmpeg_dir,
            whisper_dir,
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
    let threads = thread::available_parallelism()
        .map(|count| count.get().saturating_sub(1).clamp(1, 8))
        .unwrap_or(4);
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
            "backend": "whisper.cpp-cpu",
            "language": "ko",
            "threads": threads,
            "noFallback": true
        },
        "chatMotion": {
            "roi": "right-38-percent",
            "sampleSeconds": if checkpoint.analysis_mode == AnalysisMode::Quick {
                QUICK_CHAT_SAMPLE_SECONDS
            } else {
                CHAT_SAMPLE_SECONDS
            }
        },
        "rankerVersion": "rules-v0.3.2"
    });
    let path = job_dir.join("pipeline-provenance.json");
    let temporary = job_dir.join("pipeline-provenance.json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(&provenance)?)?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
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
    let _job_guard = match KillOnCloseJob::attach(&child) {
        Ok(job) => job,
        Err(error) => {
            child.kill().ok();
            child.wait().ok();
            return Err(PipelineError::Message(format!(
                "{}에 강제 종료 보호를 설정하지 못했습니다: {error}",
                executable.file_name().unwrap_or_default().to_string_lossy()
            )));
        }
    };
    loop {
        if cancel_requested.load(Ordering::SeqCst) {
            child.kill().ok();
            child.wait().ok();
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
) -> Result<Option<MediaCheckpoint>, PipelineError> {
    if !path.is_file() {
        return Ok(None);
    }
    let checkpoint: MediaCheckpoint = serde_json::from_slice(&fs::read(path)?)?;
    let requested_start = requested_start.unwrap_or(0);
    if checkpoint.schema_version != 3
        || checkpoint.source_path != source_path
        || checkpoint.analysis_mode != analysis_mode
        || checkpoint.analysis_start_seconds != requested_start
        || (analysis_mode == AnalysisMode::Range
            && Some(checkpoint.analysis_end_seconds) != requested_end)
    {
        return Ok(None);
    }
    Ok(Some(checkpoint))
}

fn save_checkpoint(path: &Path, checkpoint: &MediaCheckpoint) -> Result<(), PipelineError> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(checkpoint)?)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

fn write_transcript(path: &Path, segments: &[TranscriptSegment]) -> Result<(), PipelineError> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(segments)?)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

fn write_chat_motion(path: &Path, points: &[ChatMotionPoint]) -> Result<(), PipelineError> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(points)?)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)?;
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

fn is_transcript_hallucination(value: &str) -> bool {
    let normalized = normalize_transcript(value);
    if normalized.chars().count() < 2 {
        return true;
    }
    let lower = value.to_lowercase();
    if [
        "[music]",
        "(music)",
        "[음악]",
        "자막 제공",
        "시청해 주셔서 감사합니다",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        return true;
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
        return true;
    }

    let words = normalized.split_whitespace().collect::<Vec<_>>();
    if words.len() >= 6 {
        let unique = words
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        if unique.len() * 10 < words.len() * 5 {
            return true;
        }
        if words.len() >= 8 && words[..4] == words[4..8] {
            return true;
        }
    }
    false
}

fn sanitize_transcript_segments(segments: Vec<TranscriptSegment>) -> Vec<TranscriptSegment> {
    let mut kept: Vec<TranscriptSegment> = Vec::new();
    for segment in segments {
        if is_transcript_hallucination(&segment.text) {
            continue;
        }
        let normalized = normalize_transcript(&segment.text);
        let repeated_nearby = kept.iter().rev().take(6).any(|previous| {
            segment.start_seconds - previous.end_seconds < 120.0
                && normalize_transcript(&previous.text) == normalized
        });
        if !repeated_nearby {
            kept.push(segment);
        }
    }
    kept
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
            });
        }
    }
    Ok(segments)
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
    segments: &[TranscriptSegment],
    energy: &[EnergyPoint],
    chat_motion: &[ChatMotionPoint],
) -> Vec<Candidate> {
    let window_size = duration.clamp(1.0, 45.0);
    let mut windows = Vec::new();
    let mut start = 0.0;
    loop {
        let end = (start + window_size).min(duration);
        let points = energy
            .iter()
            .filter(|point| point.start_seconds >= start && point.start_seconds < end)
            .collect::<Vec<_>>();
        let audio_raw = if points.is_empty() {
            0.0
        } else {
            points.iter().map(|point| point.rms).sum::<f64>() / points.len() as f64
        };
        let spoken = segments
            .iter()
            .filter(|segment| segment.end_seconds > start && segment.start_seconds < end)
            .collect::<Vec<_>>();
        let characters = spoken
            .iter()
            .map(|segment| segment.text.chars().count())
            .sum::<usize>();
        let dialogue_raw = characters as f64 + spoken.len() as f64 * 12.0;
        let chat_points = chat_motion
            .iter()
            .filter(|point| point.start_seconds >= start && point.start_seconds < end)
            .collect::<Vec<_>>();
        let chat_raw = if chat_motion.is_empty() {
            None
        } else if chat_points.is_empty() {
            Some(0.0)
        } else {
            Some(
                chat_points.iter().map(|point| point.motion).sum::<f64>()
                    / chat_points.len() as f64,
            )
        };
        let excerpt = spoken
            .iter()
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
        });
        if end >= duration {
            break;
        }
        start += 15.0;
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
            overlaps || transcript_similarity(&selected.0.excerpt, &item.0.excerpt) >= 0.75
        });
        if overlaps_or_repeats {
            continue;
        }
        selected.push(item);
        if selected.len() == 8 {
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
            let title = if excerpt.is_empty() {
                "오디오 반응이 컸던 구간".into()
            } else {
                truncate_chars(&excerpt, 28)
            };
            let start_seconds = window.start.floor().max(0.0) as u32;
            let end_seconds = window.end.ceil().max(1.0) as u32;
            let (context_start_seconds, context_end_seconds) =
                context_bounds(start_seconds, end_seconds, duration);
            Candidate {
                id: stable_candidate_id(start_seconds, end_seconds),
                start_seconds,
                end_seconds,
                title,
                summary: if let Some(chat) = chat {
                    format!("오디오 반응 {audio} · 발화 밀도 {dialogue} · 채팅 움직임 {chat}")
                } else {
                    format!("오디오 반응 {audio} · 발화 밀도 {dialogue} · 채팅 움직임 없음")
                },
                transcript_excerpt: if excerpt.is_empty() {
                    "이 구간에서 인식된 발화가 없습니다.".into()
                } else {
                    excerpt
                },
                audio_score: audio,
                dialogue_score: dialogue,
                chat_score: chat,
                total_score: total,
                decision: CandidateDecision::Pending,
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

fn transcript_similarity(left: &str, right: &str) -> f64 {
    let left = normalize_transcript(left);
    let right = normalize_transcript(right);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    if left == right {
        return 1.0;
    }
    let left = left
        .split_whitespace()
        .collect::<std::collections::HashSet<_>>();
    let right = right
        .split_whitespace()
        .collect::<std::collections::HashSet<_>>();
    let union = left.union(&right).count();
    if union == 0 {
        0.0
    } else {
        left.intersection(&right).count() as f64 / union as f64
    }
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
            text: segment.text.clone(),
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
    use std::env;

    #[test]
    fn parses_srt_timestamp() {
        assert_eq!(parse_srt_time("01:02:03,500"), Some(3723.5));
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
        }];
        let energy = (0..60)
            .map(|second| EnergyPoint {
                start_seconds: second as f64,
                rms: if (15..45).contains(&second) { 0.8 } else { 0.1 },
            })
            .collect::<Vec<_>>();
        let first = build_candidates(60.0, &segments, &energy, &[]);
        let regenerated = build_candidates(60.0, &segments, &energy, &[]);
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
        }];
        let energy = (0..60)
            .map(|second| EnergyPoint {
                start_seconds: second as f64,
                rms: if second < 45 { 0.8 } else { 0.1 },
            })
            .collect::<Vec<_>>();
        let candidates = build_candidates(60.0, &segments, &energy, &[]);
        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].chat_score, None);
        assert!(candidates[0].total_score > 0);
    }

    #[test]
    fn removes_english_repetition_and_nearby_duplicate_hallucinations() {
        let sanitized = sanitize_transcript_segments(vec![
            TranscriptSegment {
                start_seconds: 0.0,
                end_seconds: 4.0,
                text: "1/2 of the cream cheese. 1/2 of the cream cheese. 1/2 of the cream cheese."
                    .into(),
            },
            TranscriptSegment {
                start_seconds: 10.0,
                end_seconds: 13.0,
                text: "이건 진짜 말도 안 되잖아".into(),
            },
            TranscriptSegment {
                start_seconds: 20.0,
                end_seconds: 23.0,
                text: "이건 진짜 말도 안 되잖아".into(),
            },
        ]);
        assert_eq!(sanitized.len(), 1);
        assert_eq!(sanitized[0].text, "이건 진짜 말도 안 되잖아");
    }

    #[test]
    fn produces_non_overlapping_candidates_with_real_chat_motion_scores() {
        let segments = (0..12)
            .map(|index| TranscriptSegment {
                start_seconds: index as f64 * 10.0,
                end_seconds: index as f64 * 10.0 + 4.0,
                text: format!("서로 다른 한국어 발화 구간 {index}"),
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
        let candidates = build_candidates(120.0, &segments, &energy, &motion);
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
        let candidates = build_candidates(duration, &segments, &energy, &[]);
        assert!(!candidates.is_empty());
        assert!(candidates
            .iter()
            .all(|candidate| candidate.chat_score.is_none()));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.transcript_excerpt.contains("fellow Americans")));

        let checkpoint = MediaCheckpoint {
            schema_version: 3,
            source_path: source.display().to_string(),
            duration_seconds: duration,
            chunk_seconds: CHUNK_SECONDS,
            analysis_mode: AnalysisMode::Full,
            analysis_start_seconds: 0,
            analysis_end_seconds: duration.ceil() as u32,
            planned_chunks: vec![PlannedChunk {
                offset_seconds: 0.0,
                length_seconds: duration,
            }],
            completed_chunks: 1,
            segments,
            energy,
            chat_motion_completed: false,
            chat_motion: Vec::new(),
        };
        let checkpoint_path = temp.path().join("media-checkpoint.json");
        save_checkpoint(&checkpoint_path, &checkpoint).unwrap();
        assert!(checkpoint_path.is_file());
    }
}
