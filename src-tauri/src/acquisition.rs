use super::{media, mutate_job, AppState};
use crate::domain::JobStatus;
use crate::integrity::{
    aggregate_required_bytes_by_volume, format_bytes_for_message, free_disk_space_bytes,
    verify_runtime_bundle, volume_identity,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;
use url::Url;

use crate::media::terminate_child_tree;
#[cfg(windows)]
use crate::media::{KillOnCloseJob, CREATE_NO_WINDOW};

/// Initial format expression for metadata probe only. The probe result is pinned by exact
/// `format_id`s for the actual media transfer (`run_yt_dlp`).
const YT_DLP_FORMAT_PROBE: &str = "bv*[height<=720]+ba/b[height<=720]/b";

/// Hard caps on metadata-only probe pipes (fail closed if exceeded).
const PROBE_STDOUT_BYTE_LIMIT: usize = 2 * 1024 * 1024;
const PROBE_STDERR_BYTE_LIMIT: usize = 256 * 1024;

#[derive(Debug)]
enum AcquisitionError {
    Cancelled,
    Message(String),
}

impl From<std::io::Error> for AcquisitionError {
    fn from(error: std::io::Error) -> Self {
        Self::Message(error.to_string())
    }
}

impl From<serde_json::Error> for AcquisitionError {
    fn from(error: serde_json::Error) -> Self {
        Self::Message(error.to_string())
    }
}

#[derive(Debug)]
struct DownloadTools {
    yt_dlp: PathBuf,
    deno: PathBuf,
    ffmpeg_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AcquisitionCheckpoint {
    schema_version: u8,
    source_url: String,
    media_path: String,
    title: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum OutputKind {
    Stdout,
    Stderr,
}

#[derive(Debug)]
struct OutputLine {
    kind: OutputKind,
    text: String,
}

#[derive(Default)]
struct ProgressTracker {
    format_id: Option<String>,
    stream_index: u8,
    last_reported: u8,
}

impl ProgressTracker {
    fn update(&mut self, format_id: String, stream_percent: u8) -> Option<u8> {
        match self.format_id.as_deref() {
            None => self.format_id = Some(format_id),
            Some(current) if current != format_id => {
                self.format_id = Some(format_id);
                self.stream_index = self.stream_index.saturating_add(1);
            }
            _ => {}
        }
        let base = if self.stream_index == 0 { 0 } else { 50 };
        let overall = (base + stream_percent / 2).min(99);
        if overall >= self.last_reported.saturating_add(2) {
            self.last_reported = overall;
            Some(overall)
        } else {
            None
        }
    }
}

pub(crate) fn validate_youtube_url(value: &str) -> Result<(), String> {
    let parsed = Url::parse(value).map_err(|_| "YouTube 영상 주소 형식을 확인해 주세요.")?;
    if parsed.scheme() != "https" || !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("HTTPS YouTube 영상 주소만 사용할 수 있습니다.".into());
    }
    if parsed.port().is_some() {
        return Err("포트가 포함된 YouTube 주소는 사용할 수 없습니다.".into());
    }

    let host = parsed
        .host_str()
        .map(|host| host.to_ascii_lowercase())
        .ok_or_else(|| "YouTube 영상 주소 형식을 확인해 주세요.".to_string())?;
    let segments = parsed
        .path_segments()
        .map(|segments| segments.filter(|part| !part.is_empty()).collect::<Vec<_>>())
        .unwrap_or_default();
    let valid = match host.as_str() {
        "youtu.be" => segments.first().is_some_and(|id| !id.is_empty()),
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" => {
            (parsed.path() == "/watch"
                && parsed
                    .query_pairs()
                    .any(|(key, value)| key == "v" && !value.is_empty()))
                || (matches!(segments.first().copied(), Some("shorts" | "live" | "embed"))
                    && segments.get(1).is_some_and(|id| !id.is_empty()))
        }
        _ => false,
    };

    if valid {
        Ok(())
    } else {
        Err(
            "지원되는 YouTube 영상 주소를 입력해 주세요. 채널·재생목록 주소는 지원하지 않습니다."
                .into(),
        )
    }
}

pub(crate) fn run_youtube_pipeline<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: Arc<AppState>,
    job_id: String,
) {
    match acquire(&app, &state, &job_id) {
        Ok(()) => media::run_media_pipeline(app, state, job_id),
        Err(AcquisitionError::Cancelled) => {
            let _ = mutate_job(&app, &state, |job| {
                if job.status != JobStatus::Cancelling && job.status.is_active() {
                    job.transition(JobStatus::Cancelling)?;
                }
                job.transition(JobStatus::Cancelled)?;
                job.current_stage_label = "다운로드 취소됨".into();
                job.error_message = None;
                job.error_detail = None;
                job.push_activity(
                    "cancel",
                    "YouTube 다운로드를 중단했습니다. 임시 파일을 보존해 다음 실행에서 이어받습니다.",
                );
                Ok(())
            });
            finish(&state);
        }
        Err(AcquisitionError::Message(detail)) => {
            let _ = mutate_job(&app, &state, |job| {
                if job.status == JobStatus::Cancelling {
                    job.transition(JobStatus::Cancelled)?;
                    job.current_stage_label = "다운로드 취소됨".into();
                    job.error_message = None;
                    job.error_detail = None;
                    job.push_activity("cancel", "YouTube 다운로드를 취소했습니다.");
                } else {
                    job.transition(JobStatus::Failed)?;
                    job.current_stage_label = "YouTube 다운로드 실패".into();
                    job.error_message = Some("YouTube 영상을 다운로드하지 못했습니다.".into());
                    job.error_detail = Some(detail);
                    job.push_activity(
                        "error",
                        "다운로드 로그와 임시 파일을 보존했습니다. 같은 작업에서 다시 시도할 수 있습니다.",
                    );
                }
                Ok(())
            });
            finish(&state);
        }
    }
}

fn finish(state: &Arc<AppState>) {
    state.cancel_requested.store(false, Ordering::SeqCst);
    state.running.store(false, Ordering::SeqCst);
}

fn acquire<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &Arc<AppState>,
    job_id: &str,
) -> Result<(), AcquisitionError> {
    let (source_url, completed_units) = {
        let guard = state
            .job
            .lock()
            .map_err(|_| AcquisitionError::Message("작업 상태 잠금이 손상됐습니다.".into()))?;
        let job = guard
            .as_ref()
            .ok_or_else(|| AcquisitionError::Message("현재 작업이 없습니다.".into()))?;
        if job.id != job_id {
            return Err(AcquisitionError::Message(
                "실행할 작업이 현재 작업과 다릅니다.".into(),
            ));
        }
        (job.source_label.clone(), job.completed_units)
    };
    validate_youtube_url(&source_url).map_err(AcquisitionError::Message)?;

    let tools = locate_tools(&state.resource_dir)?;
    let job_dir = state.store.job_dir(job_id);
    let download_dir = job_dir.join("youtube-download");
    let log_dir = job_dir.join("tool-logs");
    fs::create_dir_all(&download_dir)?;
    fs::create_dir_all(&log_dir)?;
    let checkpoint_path = job_dir.join("acquisition.json");

    let checkpoint = load_checkpoint(&checkpoint_path, &source_url)?;
    let (media_path, title) = if let Some(checkpoint) = checkpoint {
        (PathBuf::from(checkpoint.media_path), checkpoint.title)
    } else if let Some(path) = find_downloaded_media(&download_dir)? {
        save_checkpoint(
            &checkpoint_path,
            &AcquisitionCheckpoint {
                schema_version: 1,
                source_url: source_url.clone(),
                media_path: path.display().to_string(),
                title: None,
            },
        )?;
        (path, None)
    } else {
        mutate_job(app, state, |job| {
            job.download_percent = Some(0);
            job.current_stage_label = "YouTube 정보 확인".into();
            job.push_activity(
                "download",
                "YouTube 영상 정보를 확인하고 저장 공간을 점검한 뒤 최대 720p로 다운로드합니다.",
            );
            Ok(())
        })
        .map_err(AcquisitionError::Message)?;

        // Metadata-only size lookup, exact format pin, and multi-phase free-space plan MUST pass
        // before any media transfer child.
        let format_plan = ensure_download_disk_space(
            &tools,
            &source_url,
            &download_dir,
            &job_dir,
            &log_dir,
            &state.cancel_requested,
        )?;

        let outcome = run_yt_dlp(
            app,
            state,
            &tools,
            &source_url,
            &download_dir,
            &log_dir,
            &format_plan.format_selector,
        )?;
        let path = outcome
            .media_path
            .filter(|path| path.is_file())
            .or(find_downloaded_media(&download_dir)?)
            .ok_or_else(|| {
                AcquisitionError::Message(
                    "yt-dlp가 성공했지만 완성된 영상 파일을 찾지 못했습니다.".into(),
                )
            })?;
        save_checkpoint(
            &checkpoint_path,
            &AcquisitionCheckpoint {
                schema_version: 1,
                source_url: source_url.clone(),
                media_path: path.display().to_string(),
                title: outcome.title.clone(),
            },
        )?;
        (path, outcome.title)
    };

    if state.cancel_requested.load(Ordering::SeqCst) {
        return Err(AcquisitionError::Cancelled);
    }
    let label = title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("YouTube 영상");
    mutate_job(app, state, |job| {
        job.acquired_media_path = Some(media_path.display().to_string());
        job.download_percent = Some(100);
        if completed_units == 0 {
            job.apply_progress(
                1,
                JobStatus::Acquiring,
                "YouTube 다운로드 완료".into(),
                format!("‘{label}’ 다운로드를 마치고 로컬 분석을 시작합니다."),
            )?;
        }
        Ok(())
    })
    .map_err(AcquisitionError::Message)?;
    Ok(())
}

struct DownloadOutcome {
    media_path: Option<PathBuf>,
    title: Option<String>,
}

fn overflow_plan_error() -> String {
    "예상 용량 계산 중 값이 너무 커서 내려받기를 시작하지 않습니다. 디스크 공간을 확인한 뒤 다시 시도해 주세요."
        .into()
}

/// Exact `filesize` missing or only `filesize_approx` — cannot plan space safely.
pub(crate) fn unsafe_size_error() -> String {
    "선택한 영상의 정확한 용량을 확인하지 못해 안전하게 내려받기를 시작할 수 없습니다. 잠시 후 다시 시도하거나 다른 영상을 선택해 주세요."
        .into()
}

/// Network / availability class (probe exit failure, empty response).
pub(crate) fn metadata_probe_network_message() -> String {
    "YouTube 영상 정보를 확인하지 못했습니다. 네트워크 연결과 영상 공개 여부를 확인한 뒤 다시 시도해 주세요."
        .into()
}

/// Local log folder / permissions / disk for diagnostics.
pub(crate) fn metadata_probe_local_environment_message() -> String {
    "작업 로그 폴더를 준비하지 못했습니다. 저장 공간과 폴더 권한을 확인한 뒤 다시 시도해 주세요."
        .into()
}

/// Spawn / pipe / Job Object / oversized probe output.
pub(crate) fn metadata_probe_tool_runtime_message() -> String {
    "영상 정보 확인을 실행하지 못했습니다. 앱을 다시 시작한 뒤 다시 시도해 주세요. 문제가 계속되면 앱을 다시 설치해 주세요."
        .into()
}

/// Probe response too large to process safely.
pub(crate) fn metadata_probe_oversized_message() -> String {
    "영상 정보가 비정상적으로 커서 안전하게 확인하지 못했습니다. 잠시 후 다시 시도해 주세요."
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MetadataProbeFailureClass {
    NetworkOrAvailability,
    LocalEnvironment,
    ToolRuntime,
    OversizedOutput,
    UnsafeSize,
}

pub(crate) fn metadata_probe_user_message(class: MetadataProbeFailureClass) -> String {
    match class {
        MetadataProbeFailureClass::NetworkOrAvailability => metadata_probe_network_message(),
        MetadataProbeFailureClass::LocalEnvironment => metadata_probe_local_environment_message(),
        MetadataProbeFailureClass::ToolRuntime => metadata_probe_tool_runtime_message(),
        MetadataProbeFailureClass::OversizedOutput => metadata_probe_oversized_message(),
        MetadataProbeFailureClass::UnsafeSize => unsafe_size_error(),
    }
}

/// Probe selection pinned for the actual media transfer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SelectedFormatPlan {
    pub format_selector: String,
    pub format_ids: Vec<String>,
    pub stream_sizes: Vec<u64>,
    pub duration_seconds: f64,
}

/// Checked sum of selected stream sizes (each must be > 0).
pub(crate) fn sum_stream_sizes(stream_sizes: &[u64]) -> Result<u64, String> {
    if stream_sizes.is_empty() {
        return Err(unsafe_size_error());
    }
    let mut total = 0u64;
    for &size in stream_sizes {
        if size == 0 {
            return Err(unsafe_size_error());
        }
        total = total.checked_add(size).ok_or_else(overflow_plan_error)?;
    }
    Ok(total)
}

/// Same-volume download peak: separate A/V + merge output (~2× streams) + 10% margin.
/// `peak = 2*S + floor((2*S)/10)` with checked arithmetic.
pub(crate) fn estimate_download_peak_bytes(stream_sizes: &[u64]) -> Result<u64, String> {
    let streams_total = sum_stream_sizes(stream_sizes)?;
    download_peak_from_streams_total(streams_total)
}

fn download_peak_from_streams_total(streams_total: u64) -> Result<u64, String> {
    let peak = streams_total
        .checked_mul(2)
        .ok_or_else(overflow_plan_error)?;
    let margin = peak / 10;
    peak.checked_add(margin).ok_or_else(overflow_plan_error)
}

/// Per-volume budget when home and temp are on distinct volumes: streams + 10% margin each.
fn stream_budget_with_margin(streams_total: u64) -> Result<u64, String> {
    let margin = streams_total / 10;
    streams_total
        .checked_add(margin)
        .ok_or_else(overflow_plan_error)
}

/// Pure multi-phase planner (deterministic volume ids — no filesystem).
///
/// # Formulas
/// - `S = Σ stream_sizes` (checked; final source estimate = S)
/// - Download peak (home≡temp): `P = 2S + floor(2S/10)`
/// - Split budget (home≠temp, each volume): `B = S + floor(S/10)`
/// - Analysis workspace: `W = estimate_analysis_workspace_bytes(S, duration)` (existing media helper)
/// - Phase download (simultaneous → **sum** on a volume):
///   - home≡temp → home: P
///   - home≠temp → home: B, temp: B
/// - Phase analysis (simultaneous → **sum**):
///   - home: S (final source remains), job: W
/// - Across sequential phases → **max** per volume
pub(crate) fn plan_pre_download_volume_bytes(
    stream_sizes: &[u64],
    duration_seconds: f64,
    home_volume: &str,
    temp_volume: &str,
    job_volume: &str,
) -> Result<BTreeMap<String, u64>, String> {
    let streams_total = sum_stream_sizes(stream_sizes)?;
    let final_source = streams_total;
    let analysis_ws = media::estimate_analysis_workspace_bytes(final_source, duration_seconds);

    let mut download_phase: BTreeMap<String, u64> = BTreeMap::new();
    if home_volume == temp_volume {
        let peak = download_peak_from_streams_total(streams_total)?;
        phase_sum_insert(&mut download_phase, home_volume, peak)?;
    } else {
        let budget = stream_budget_with_margin(streams_total)?;
        phase_sum_insert(&mut download_phase, home_volume, budget)?;
        phase_sum_insert(&mut download_phase, temp_volume, budget)?;
    }

    let mut analysis_phase: BTreeMap<String, u64> = BTreeMap::new();
    phase_sum_insert(&mut analysis_phase, home_volume, final_source)?;
    phase_sum_insert(&mut analysis_phase, job_volume, analysis_ws)?;

    Ok(max_across_phases(
        [download_phase, analysis_phase].into_iter(),
    ))
}

fn phase_sum_insert(
    map: &mut BTreeMap<String, u64>,
    volume: &str,
    amount: u64,
) -> Result<(), String> {
    let entry = map.entry(volume.to_string()).or_insert(0);
    *entry = entry.checked_add(amount).ok_or_else(overflow_plan_error)?;
    Ok(())
}

fn max_across_phases<I>(phases: I) -> BTreeMap<String, u64>
where
    I: IntoIterator<Item = BTreeMap<String, u64>>,
{
    let mut out: BTreeMap<String, u64> = BTreeMap::new();
    for phase in phases {
        for (volume, need) in phase {
            out.entry(volume)
                .and_modify(|existing: &mut u64| *existing = (*existing).max(need))
                .or_insert(need);
        }
    }
    out
}

/// Production planner: same formulas as [`plan_pre_download_volume_bytes`], but
/// simultaneous needs are summed with [`aggregate_required_bytes_by_volume`]
/// (proves distinct-volume handling in the real path).
///
/// Returns `(volume_id, probe_path, required_free_bytes)`.
pub(crate) fn plan_pre_download_path_requirements(
    stream_sizes: &[u64],
    duration_seconds: f64,
    home: &Path,
    temp: &Path,
    job: &Path,
) -> Result<Vec<(String, PathBuf, u64)>, String> {
    let streams_total = sum_stream_sizes(stream_sizes)?;
    let final_source = streams_total;
    let analysis_ws = media::estimate_analysis_workspace_bytes(final_source, duration_seconds);
    let home_id = volume_identity(home)?;
    let temp_id = volume_identity(temp)?;

    // Phase download — simultaneous budgets aggregated (sum) per volume.
    let download_targets: Vec<(PathBuf, u64)> = if home_id == temp_id {
        vec![(
            home.to_path_buf(),
            download_peak_from_streams_total(streams_total)?,
        )]
    } else {
        let budget = stream_budget_with_margin(streams_total)?;
        vec![(home.to_path_buf(), budget), (temp.to_path_buf(), budget)]
    };
    let download_agg = aggregate_required_bytes_by_volume(&download_targets)?;

    // Phase analysis — final source on home + analysis workspace on job (sum if same volume).
    let analysis_targets = vec![
        (home.to_path_buf(), final_source),
        (job.to_path_buf(), analysis_ws),
    ];
    let analysis_agg = aggregate_required_bytes_by_volume(&analysis_targets)?;

    let mut by_volume: BTreeMap<String, (PathBuf, u64)> = BTreeMap::new();
    for (id, path, need) in download_agg {
        by_volume.insert(id, (path, need));
    }
    for (id, path, need) in analysis_agg {
        match by_volume.get_mut(&id) {
            Some((_probe, existing)) => {
                *existing = (*existing).max(need);
            }
            None => {
                by_volume.insert(id, (path, need));
            }
        }
    }
    Ok(by_volume
        .into_iter()
        .map(|(id, (path, need))| (id, path, need))
        .collect())
}

/// Check free space against the multi-phase plan. Shortage / free-space query failure block.
pub(crate) fn ensure_planned_download_space(
    stream_sizes: &[u64],
    duration_seconds: f64,
    home: &Path,
    temp: &Path,
    job: &Path,
) -> Result<u64, String> {
    let plan =
        plan_pre_download_path_requirements(stream_sizes, duration_seconds, home, temp, job)?;
    if plan.is_empty() {
        return Err("내려받기 대상 폴더를 확인하지 못했습니다.".into());
    }
    let mut max_need = 0u64;
    for (_id, probe, need) in plan {
        max_need = max_need.max(need);
        let available = free_disk_space_bytes(&probe)?;
        if available < need {
            let shortfall = need - available;
            return Err(format!(
                "저장 공간이 부족합니다. YouTube 내려받기와 이어지는 분석 준비에 이 디스크에서 약 {}이 필요하지만 현재 여유 공간은 {}입니다. 약 {}을 확보한 뒤 다시 시작해 주세요.",
                format_bytes_for_message(need),
                format_bytes_for_message(available),
                format_bytes_for_message(shortfall)
            ));
        }
    }
    Ok(max_need)
}

/// Build a transfer-pinned plan from metadata JSON: exact `filesize` only + `format_id` pin.
pub(crate) fn selected_format_plan_from_info(info: &Value) -> Result<SelectedFormatPlan, String> {
    let duration_seconds = duration_seconds_from_info(info)?;
    let (format_ids, stream_sizes) = if let Some(formats) = info
        .get("requested_formats")
        .and_then(|value| value.as_array())
    {
        if formats.is_empty() {
            return Err(unsafe_size_error());
        }
        let mut ids = Vec::with_capacity(formats.len());
        let mut sizes = Vec::with_capacity(formats.len());
        for format in formats {
            ids.push(extract_format_id(format)?);
            sizes.push(stream_exact_filesize(format)?);
        }
        (ids, sizes)
    } else {
        (
            vec![extract_format_id(info)?],
            vec![stream_exact_filesize(info)?],
        )
    };
    let format_selector = format_selector_from_ids(&format_ids)?;
    Ok(SelectedFormatPlan {
        format_selector,
        format_ids,
        stream_sizes,
        duration_seconds,
    })
}

/// Exact positive `filesize` only. `filesize_approx` alone is insufficient (fail closed).
fn stream_exact_filesize(format: &Value) -> Result<u64, String> {
    match format.get("filesize").and_then(json_positive_u64) {
        Some(size) => Ok(size),
        None => Err(unsafe_size_error()),
    }
}

fn extract_format_id(format: &Value) -> Result<String, String> {
    let raw = format
        .get("format_id")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(unsafe_size_error)?;
    validate_format_id(raw)?;
    Ok(raw.to_string())
}

/// yt-dlp format ids used as CLI `--format` tokens must be conservative and shell-safe.
pub(crate) fn validate_format_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 64 {
        return Err(unsafe_size_error());
    }
    let ok = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if ok {
        Ok(())
    } else {
        Err(unsafe_size_error())
    }
}

pub(crate) fn format_selector_from_ids(ids: &[String]) -> Result<String, String> {
    if ids.is_empty() {
        return Err(unsafe_size_error());
    }
    for id in ids {
        validate_format_id(id)?;
    }
    Ok(ids.join("+"))
}

pub(crate) fn duration_seconds_from_info(info: &Value) -> Result<f64, String> {
    let raw = info
        .get("duration")
        .ok_or_else(metadata_probe_network_message)?;
    let seconds = raw
        .as_f64()
        .or_else(|| raw.as_u64().map(|value| value as f64))
        .or_else(|| {
            raw.as_i64()
                .filter(|value| *value > 0)
                .map(|value| value as f64)
        });
    match seconds {
        Some(value) if value.is_finite() && value > 0.0 => Ok(value),
        _ => Err(metadata_probe_network_message()),
    }
}

fn json_positive_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => {
            if let Some(unsigned) = number.as_u64() {
                return (unsigned > 0).then_some(unsigned);
            }
            if let Some(signed) = number.as_i64() {
                return (signed > 0).then_some(signed as u64);
            }
            // Exact filesize must be an integer field; reject fractional JSON numbers.
            None
        }
        _ => None,
    }
}

/// Minimal structured probe log (no raw JSON, URLs, headers, or stderr bodies).
pub(crate) fn build_minimal_metadata_log(
    plan: Option<&SelectedFormatPlan>,
    probe_exit_code: Option<i32>,
    stdout_bytes_observed: usize,
    stderr_bytes_observed: usize,
    stdout_capped: bool,
    stderr_capped: bool,
    ok: bool,
    failure_class: Option<&str>,
) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("schemaVersion".into(), Value::from(1));
    map.insert("ok".into(), Value::from(ok));
    if let Some(code) = probe_exit_code {
        map.insert("probeExitCode".into(), Value::from(code));
    }
    map.insert(
        "stdoutBytesObserved".into(),
        Value::from(stdout_bytes_observed as u64),
    );
    map.insert(
        "stderrBytesObserved".into(),
        Value::from(stderr_bytes_observed as u64),
    );
    map.insert("stdoutCapped".into(), Value::from(stdout_capped));
    map.insert("stderrCapped".into(), Value::from(stderr_capped));
    if let Some(class) = failure_class {
        map.insert("failureClass".into(), Value::from(class));
    }
    if let Some(plan) = plan {
        map.insert(
            "durationSeconds".into(),
            Value::from(plan.duration_seconds),
        );
        map.insert(
            "formatIds".into(),
            Value::Array(
                plan.format_ids
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
        map.insert(
            "formatSelector".into(),
            Value::String(plan.format_selector.clone()),
        );
        map.insert(
            "streamFilesizes".into(),
            Value::Array(
                plan.stream_sizes
                    .iter()
                    .map(|size| Value::from(*size))
                    .collect(),
            ),
        );
    }
    Value::Object(map)
}

fn write_minimal_metadata_log(log_dir: &Path, value: &Value) {
    let _ = fs::create_dir_all(log_dir);
    if let Ok(bytes) = serde_json::to_vec_pretty(value) {
        let _ = fs::write(log_dir.join("yt-dlp.metadata.json"), bytes);
    }
}

fn probe_fail(
    log_dir: &Path,
    class: MetadataProbeFailureClass,
    exit_code: Option<i32>,
    stdout_len: usize,
    stderr_len: usize,
    stdout_capped: bool,
    stderr_capped: bool,
) -> AcquisitionError {
    let class_name = match class {
        MetadataProbeFailureClass::NetworkOrAvailability => "network_or_availability",
        MetadataProbeFailureClass::LocalEnvironment => "local_environment",
        MetadataProbeFailureClass::ToolRuntime => "tool_runtime",
        MetadataProbeFailureClass::OversizedOutput => "oversized_output",
        MetadataProbeFailureClass::UnsafeSize => "unsafe_size",
    };
    write_minimal_metadata_log(
        log_dir,
        &build_minimal_metadata_log(
            None,
            exit_code,
            stdout_len,
            stderr_len,
            stdout_capped,
            stderr_capped,
            false,
            Some(class_name),
        ),
    );
    AcquisitionError::Message(metadata_probe_user_message(class))
}

/// Read at most `limit` bytes; returns `(bytes, exceeded)` without retaining the overflow tail.
pub(crate) fn read_capped_bytes<R: Read>(reader: R, limit: usize) -> std::io::Result<(Vec<u8>, bool)> {
    let mut limited = reader.take(limit as u64 + 1);
    let mut buffer = Vec::new();
    limited.read_to_end(&mut buffer)?;
    let exceeded = buffer.len() > limit;
    if exceeded {
        buffer.truncate(limit);
    }
    Ok((buffer, exceeded))
}

fn ensure_download_disk_space(
    tools: &DownloadTools,
    source_url: &str,
    download_dir: &Path,
    job_dir: &Path,
    log_dir: &Path,
    cancel_requested: &AtomicBool,
) -> Result<SelectedFormatPlan, AcquisitionError> {
    if cancel_requested.load(Ordering::SeqCst) {
        return Err(AcquisitionError::Cancelled);
    }
    let plan = probe_download_metadata(tools, source_url, download_dir, log_dir, cancel_requested)?;
    // Product currently pins yt-dlp home and temp to download_dir; job_dir holds analysis workspace.
    ensure_planned_download_space(
        &plan.stream_sizes,
        plan.duration_seconds,
        download_dir,
        download_dir,
        job_dir,
    )
    .map_err(AcquisitionError::Message)?;
    Ok(plan)
}

/// Metadata-only probe (`--skip-download` + `--dump-single-json`). No media transfer.
/// Pins exact format_ids for transfer; exact filesize only; capped pipes; minimal structured log.
fn probe_download_metadata(
    tools: &DownloadTools,
    source_url: &str,
    download_dir: &Path,
    log_dir: &Path,
    cancel_requested: &AtomicBool,
) -> Result<SelectedFormatPlan, AcquisitionError> {
    if fs::create_dir_all(log_dir).is_err() {
        return Err(probe_fail(
            log_dir,
            MetadataProbeFailureClass::LocalEnvironment,
            None,
            0,
            0,
            false,
            false,
        ));
    }
    let mut command = Command::new(&tools.yt_dlp);
    command
        .args([
            "--ignore-config",
            "--no-playlist",
            "--skip-download",
            "--no-progress",
            "--socket-timeout",
            "30",
            "--match-filter",
            "!is_live",
            "--format",
            YT_DLP_FORMAT_PROBE,
            "--dump-single-json",
        ])
        .arg("--ffmpeg-location")
        .arg(&tools.ffmpeg_dir)
        .arg("--js-runtimes")
        .arg(format!("deno:{}", tools.deno.display()))
        // Keep metadata probe out of the media transfer paths; still pin cache away from user home.
        .arg("--cache-dir")
        .arg(download_dir.join("cache"))
        .arg(source_url)
        .current_dir(download_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("NO_COLOR", "1");
    crate::media::restrict_command_environment(&mut command);
    command.env("NO_COLOR", "1");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            return Err(probe_fail(
                log_dir,
                MetadataProbeFailureClass::ToolRuntime,
                None,
                0,
                0,
                false,
                false,
            ));
        }
    };
    #[cfg(windows)]
    let job_guard = match KillOnCloseJob::attach(&child) {
        Ok(job) => Some(job),
        Err(_) => {
            terminate_child_tree(&mut child, None);
            return Err(probe_fail(
                log_dir,
                MetadataProbeFailureClass::ToolRuntime,
                None,
                0,
                0,
                false,
                false,
            ));
        }
    };

    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            #[cfg(windows)]
            terminate_child_tree(&mut child, job_guard.as_ref());
            #[cfg(not(windows))]
            terminate_child_tree(&mut child);
            return Err(probe_fail(
                log_dir,
                MetadataProbeFailureClass::ToolRuntime,
                None,
                0,
                0,
                false,
                false,
            ));
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            #[cfg(windows)]
            terminate_child_tree(&mut child, job_guard.as_ref());
            #[cfg(not(windows))]
            terminate_child_tree(&mut child);
            return Err(probe_fail(
                log_dir,
                MetadataProbeFailureClass::ToolRuntime,
                None,
                0,
                0,
                false,
                false,
            ));
        }
    };

    let stdout_cap = Arc::new(AtomicBool::new(false));
    let stderr_cap = Arc::new(AtomicBool::new(false));
    let stdout_cap_thread = stdout_cap.clone();
    let stderr_cap_thread = stderr_cap.clone();
    // stdout: keep capped body only long enough to parse JSON.
    let stdout_thread = thread::spawn(move || -> (Vec<u8>, usize, bool) {
        match read_capped_bytes(stdout, PROBE_STDOUT_BYTE_LIMIT) {
            Ok((buf, exceeded)) => {
                if exceeded {
                    stdout_cap_thread.store(true, Ordering::SeqCst);
                }
                let observed = if exceeded {
                    PROBE_STDOUT_BYTE_LIMIT + 1
                } else {
                    buf.len()
                };
                (buf, observed, exceeded)
            }
            Err(_) => {
                stdout_cap_thread.store(true, Ordering::SeqCst);
                (Vec::new(), 0, true)
            }
        }
    });
    // stderr: measure with cap, discard body immediately (never persist).
    let stderr_thread = thread::spawn(move || -> (usize, bool) {
        match read_capped_bytes(stderr, PROBE_STDERR_BYTE_LIMIT) {
            Ok((buf, exceeded)) => {
                if exceeded {
                    stderr_cap_thread.store(true, Ordering::SeqCst);
                }
                let observed = if exceeded {
                    PROBE_STDERR_BYTE_LIMIT + 1
                } else {
                    buf.len()
                };
                drop(buf);
                (observed, exceeded)
            }
            Err(_) => {
                stderr_cap_thread.store(true, Ordering::SeqCst);
                (0, true)
            }
        }
    });

    let status = loop {
        if cancel_requested.load(Ordering::SeqCst) {
            #[cfg(windows)]
            terminate_child_tree(&mut child, job_guard.as_ref());
            #[cfg(not(windows))]
            terminate_child_tree(&mut child);
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            return Err(AcquisitionError::Cancelled);
        }
        if stdout_cap.load(Ordering::SeqCst) || stderr_cap.load(Ordering::SeqCst) {
            #[cfg(windows)]
            terminate_child_tree(&mut child, job_guard.as_ref());
            #[cfg(not(windows))]
            terminate_child_tree(&mut child);
            let (stdout_bytes, stdout_observed, stdout_capped) =
                stdout_thread.join().unwrap_or_else(|_| (Vec::new(), 0, true));
            let (stderr_observed, stderr_capped) =
                stderr_thread.join().unwrap_or((0, true));
            drop(stdout_bytes);
            return Err(probe_fail(
                log_dir,
                MetadataProbeFailureClass::OversizedOutput,
                None,
                stdout_observed,
                stderr_observed,
                stdout_capped,
                stderr_capped,
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(_) => {
                #[cfg(windows)]
                terminate_child_tree(&mut child, job_guard.as_ref());
                #[cfg(not(windows))]
                terminate_child_tree(&mut child);
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();
                return Err(probe_fail(
                    log_dir,
                    MetadataProbeFailureClass::ToolRuntime,
                    None,
                    0,
                    0,
                    false,
                    false,
                ));
            }
        }
        thread::sleep(Duration::from_millis(50));
    };
    let (stdout_bytes, stdout_observed, stdout_capped) =
        stdout_thread.join().unwrap_or_else(|_| (Vec::new(), 0, true));
    let (stderr_observed, stderr_capped) = stderr_thread.join().unwrap_or((0, true));
    if stdout_capped || stderr_capped {
        drop(stdout_bytes);
        return Err(probe_fail(
            log_dir,
            MetadataProbeFailureClass::OversizedOutput,
            status.code(),
            stdout_observed,
            stderr_observed,
            stdout_capped,
            stderr_capped,
        ));
    }

    if !status.success() {
        drop(stdout_bytes);
        return Err(probe_fail(
            log_dir,
            MetadataProbeFailureClass::NetworkOrAvailability,
            status.code(),
            stdout_observed,
            stderr_observed,
            false,
            false,
        ));
    }
    if stdout_bytes.is_empty() {
        return Err(probe_fail(
            log_dir,
            MetadataProbeFailureClass::NetworkOrAvailability,
            status.code(),
            0,
            stderr_observed,
            false,
            false,
        ));
    }
    let info: Value = match serde_json::from_slice(&stdout_bytes) {
        Ok(value) => value,
        Err(_) => {
            drop(stdout_bytes);
            return Err(probe_fail(
                log_dir,
                MetadataProbeFailureClass::NetworkOrAvailability,
                status.code(),
                stdout_observed,
                stderr_observed,
                false,
                false,
            ));
        }
    };
    // Drop raw stdout after parse — only structured plan fields are logged.
    drop(stdout_bytes);

    let plan = match selected_format_plan_from_info(&info) {
        Ok(plan) => plan,
        Err(message) => {
            let class = if message == unsafe_size_error() {
                MetadataProbeFailureClass::UnsafeSize
            } else {
                MetadataProbeFailureClass::NetworkOrAvailability
            };
            return Err(probe_fail(
                log_dir,
                class,
                status.code(),
                stdout_observed,
                stderr_observed,
                false,
                false,
            ));
        }
    };

    write_minimal_metadata_log(
        log_dir,
        &build_minimal_metadata_log(
            Some(&plan),
            status.code(),
            stdout_observed,
            stderr_observed,
            false,
            false,
            true,
            None,
        ),
    );
    Ok(plan)
}

fn run_yt_dlp<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &Arc<AppState>,
    tools: &DownloadTools,
    source_url: &str,
    download_dir: &Path,
    log_dir: &Path,
    format_selector: &str,
) -> Result<DownloadOutcome, AcquisitionError> {
    // Fail closed if selector was not pre-validated (should already be pinned from probe).
    for id in format_selector.split('+') {
        validate_format_id(id).map_err(AcquisitionError::Message)?;
    }
    let mut command = Command::new(&tools.yt_dlp);
    command
        .args([
            "--ignore-config",
            "--no-playlist",
            "--no-simulate",
            "--newline",
            "--progress",
            "--progress-delta",
            "1",
            "--progress-template",
            "download:VODSCOUT_PROGRESS=%(info.format_id)s|%(progress._percent_str)s",
            "--print",
            "before_dl:VODSCOUT_TITLE=%(title)j",
            "--print",
            "after_move:VODSCOUT_RESULT=%(filepath)j",
            "--format",
            format_selector,
            "--continue",
            "--part",
            "--retries",
            "3",
            "--fragment-retries",
            "3",
            "--socket-timeout",
            "30",
            "--match-filter",
            "!is_live",
            "--windows-filenames",
            "--output",
            "source.%(ext)s",
        ])
        .arg("--paths")
        .arg(format!("home:{}", download_dir.display()))
        .arg("--paths")
        .arg(format!("temp:{}", download_dir.display()))
        .arg("--cache-dir")
        .arg(download_dir.join("cache"))
        .arg("--ffmpeg-location")
        .arg(&tools.ffmpeg_dir)
        .arg("--js-runtimes")
        .arg(format!("deno:{}", tools.deno.display()))
        .arg(source_url)
        .current_dir(download_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("NO_COLOR", "1");
    crate::media::restrict_command_environment(&mut command);
    command.env("NO_COLOR", "1");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command
        .spawn()
        .map_err(|error| AcquisitionError::Message(format!("yt-dlp 실행 실패: {error}")))?;
    #[cfg(windows)]
    let job_guard = match KillOnCloseJob::attach(&child) {
        Ok(job) => Some(job),
        Err(error) => {
            terminate_child_tree(&mut child, None);
            return Err(AcquisitionError::Message(format!(
                "yt-dlp에 강제 종료 보호를 설정하지 못했습니다: {error}"
            )));
        }
    };

    let stdout = child.stdout.take().ok_or_else(|| {
        AcquisitionError::Message("yt-dlp 표준 출력을 연결하지 못했습니다.".into())
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        AcquisitionError::Message("yt-dlp 진단 출력을 연결하지 못했습니다.".into())
    })?;
    let (sender, receiver) = mpsc::channel();
    let stdout_reader = spawn_reader(stdout, OutputKind::Stdout, sender.clone());
    let stderr_reader = spawn_reader(stderr, OutputKind::Stderr, sender);
    let mut stdout_log = BufWriter::new(File::create(log_dir.join("yt-dlp.stdout.log"))?);
    let mut stderr_log = BufWriter::new(File::create(log_dir.join("yt-dlp.stderr.log"))?);
    let mut stderr_tail = VecDeque::new();
    let mut media_path = None;
    let mut title = None;
    let mut progress_tracker = ProgressTracker::default();

    let status = loop {
        drain_output(
            app,
            state,
            &receiver,
            &mut stdout_log,
            &mut stderr_log,
            &mut stderr_tail,
            &mut media_path,
            &mut title,
            &mut progress_tracker,
        )?;
        if state.cancel_requested.load(Ordering::SeqCst) {
            #[cfg(windows)]
            terminate_child_tree(&mut child, job_guard.as_ref());
            #[cfg(not(windows))]
            terminate_child_tree(&mut child);
            // Detach log readers so a stuck pipe cannot block Cancelled.
            drop(stdout_reader);
            drop(stderr_reader);
            return Err(AcquisitionError::Cancelled);
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        thread::sleep(Duration::from_millis(100));
    };
    let _ = stdout_reader.join();
    let _ = stderr_reader.join();
    drain_output(
        app,
        state,
        &receiver,
        &mut stdout_log,
        &mut stderr_log,
        &mut stderr_tail,
        &mut media_path,
        &mut title,
        &mut progress_tracker,
    )?;
    stdout_log.flush()?;
    stderr_log.flush()?;

    if !status.success() {
        return Err(AcquisitionError::Message(friendly_download_error(
            status,
            &stderr_tail.into_iter().collect::<Vec<_>>().join("\n"),
        )));
    }
    Ok(DownloadOutcome { media_path, title })
}

#[allow(clippy::too_many_arguments)]
fn drain_output<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &Arc<AppState>,
    receiver: &mpsc::Receiver<OutputLine>,
    stdout_log: &mut BufWriter<File>,
    stderr_log: &mut BufWriter<File>,
    stderr_tail: &mut VecDeque<String>,
    media_path: &mut Option<PathBuf>,
    title: &mut Option<String>,
    progress_tracker: &mut ProgressTracker,
) -> Result<(), AcquisitionError> {
    while let Ok(line) = receiver.try_recv() {
        match line.kind {
            OutputKind::Stdout => {
                writeln!(stdout_log, "{}", line.text)?;
                if let Some((format_id, stream_percent)) = parse_progress(&line.text) {
                    if let Some(percent) = progress_tracker.update(format_id, stream_percent) {
                        mutate_job(app, state, |job| {
                            job.download_percent = Some(percent);
                            job.current_stage_label = format!("YouTube 다운로드 · {percent}%");
                            job.last_heartbeat_at = Some(Utc::now());
                            Ok(())
                        })
                        .map_err(AcquisitionError::Message)?;
                    }
                } else if let Some(value) = line.text.strip_prefix("VODSCOUT_RESULT=") {
                    *media_path = Some(PathBuf::from(parse_json_string(value)));
                } else if let Some(value) = line.text.strip_prefix("VODSCOUT_TITLE=") {
                    *title = Some(parse_json_string(value));
                }
            }
            OutputKind::Stderr => {
                writeln!(stderr_log, "{}", line.text)?;
                if !line.text.trim().is_empty() {
                    stderr_tail.push_back(line.text);
                    while stderr_tail.len() > 30 {
                        stderr_tail.pop_front();
                    }
                }
            }
        }
    }
    Ok(())
}

fn spawn_reader<R: Read + Send + 'static>(
    reader: R,
    kind: OutputKind,
    sender: mpsc::Sender<OutputLine>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            match reader.read_until(b'\n', &mut buffer) {
                Ok(0) => break,
                Ok(_) => {
                    let text = String::from_utf8_lossy(&buffer)
                        .trim_end_matches(['\r', '\n'])
                        .to_string();
                    if sender.send(OutputLine { kind, text }).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    })
}

fn parse_progress(line: &str) -> Option<(String, u8)> {
    let value = line.strip_prefix("VODSCOUT_PROGRESS=")?;
    let (format_id, raw_percent) = value.split_once('|')?;
    let number = raw_percent.split('%').next()?.trim().parse::<f64>().ok()?;
    Some((
        format_id.trim().to_string(),
        number.round().clamp(0.0, 100.0) as u8,
    ))
}

fn parse_json_string(value: &str) -> String {
    serde_json::from_str::<String>(value).unwrap_or_else(|_| value.trim().to_string())
}

fn friendly_download_error(status: ExitStatus, detail: &str) -> String {
    let lower = detail.to_ascii_lowercase();
    let reason = if lower.contains("private video") || lower.contains("members-only") {
        "비공개 또는 멤버십 전용 영상입니다. v0.3.0은 로그인이 필요한 영상을 지원하지 않습니다."
    } else if lower.contains("sign in to confirm") || lower.contains("not a bot") {
        "YouTube가 봇 확인을 요구했습니다. 잠시 후 다른 네트워크에서 다시 시도해 주세요."
    } else if lower.contains("video unavailable") || lower.contains("this video is unavailable") {
        "삭제됐거나 현재 지역에서 볼 수 없는 영상입니다."
    } else if lower.contains("is live") || lower.contains("premieres in") {
        "진행 중인 라이브·예약 영상은 지원하지 않습니다. 방송 종료 후 다시 시도해 주세요."
    } else {
        "네트워크 상태와 영상 공개 여부를 확인해 주세요."
    };
    let tail = detail
        .chars()
        .rev()
        .take(3000)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{reason}\nyt-dlp 종료 코드: {:?}\n{tail}", status.code())
}

fn locate_tools(resource_dir: &Path) -> Result<DownloadTools, AcquisitionError> {
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
        let tools = DownloadTools {
            yt_dlp: root.join("yt-dlp").join("yt-dlp.exe"),
            deno: root.join("deno").join("deno.exe"),
            ffmpeg_dir: root.join("ffmpeg"),
        };
        if tools.yt_dlp.is_file()
            && tools.deno.is_file()
            && tools.ffmpeg_dir.join("ffmpeg.exe").is_file()
        {
            verify_runtime_bundle(&root).map_err(AcquisitionError::Message)?;
            return Ok(tools);
        }
    }
    Err(AcquisitionError::Message(
        "내장 yt-dlp, Deno 또는 FFmpeg를 찾지 못했습니다. npm.cmd run media-tools를 실행해 주세요."
            .into(),
    ))
}

fn load_checkpoint(
    path: &Path,
    source_url: &str,
) -> Result<Option<AcquisitionCheckpoint>, AcquisitionError> {
    let previous = crate::storage::previous_generation_path(path);
    // Corrupt live falls through to .prev; valid live with wrong URL/schema does not.
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
        let checkpoint: AcquisitionCheckpoint = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if checkpoint.schema_version != 1 || checkpoint.source_url != source_url {
            if is_live {
                return Ok(None);
            }
            continue;
        }
        let media = PathBuf::from(&checkpoint.media_path);
        if media.is_file() && fs::metadata(&media)?.len() > 0 {
            return Ok(Some(checkpoint));
        }
        // Media missing: try previous generation if any.
    }
    Ok(None)
}

fn save_checkpoint(
    path: &Path,
    checkpoint: &AcquisitionCheckpoint,
) -> Result<(), AcquisitionError> {
    crate::storage::replace_file_preserving_previous(
        path,
        &serde_json::to_vec_pretty(checkpoint)?,
    )?;
    Ok(())
}

fn find_downloaded_media(download_dir: &Path) -> Result<Option<PathBuf>, AcquisitionError> {
    if !download_dir.is_dir() {
        return Ok(None);
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(download_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if path.is_file()
            && path
                .file_stem()
                .is_some_and(|stem| stem.eq_ignore_ascii_case("source"))
            && !name.ends_with(".part")
            && !name.ends_with(".ytdl")
            && !name.ends_with(".json")
            && fs::metadata(&path)?.len() > 0
        {
            candidates.push(path);
        }
    }
    candidates.sort();
    Ok(candidates.into_iter().next())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_only_supported_youtube_video_urls() {
        for valid in [
            "https://www.youtube.com/watch?v=BaW_jenozKc",
            "https://youtu.be/BaW_jenozKc?t=1",
            "https://www.youtube.com/shorts/BaW_jenozKc",
            "https://m.youtube.com/live/BaW_jenozKc",
        ] {
            assert!(validate_youtube_url(valid).is_ok(), "{valid}");
        }
        for invalid in [
            "http://www.youtube.com/watch?v=BaW_jenozKc",
            "https://www.youtube.com.evil.test/watch?v=BaW_jenozKc",
            "https://www.youtube.com/playlist?list=abc",
            "https://example.com/watch?v=BaW_jenozKc",
        ] {
            assert!(validate_youtube_url(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn parses_machine_progress_lines() {
        assert_eq!(
            parse_progress("VODSCOUT_PROGRESS=395| 42.6%"),
            Some(("395".into(), 43))
        );
        assert_eq!(parse_progress("[download] 42%"), None);
    }

    #[test]
    fn combines_video_and_audio_progress_without_early_completion() {
        let mut tracker = ProgressTracker::default();
        assert_eq!(tracker.update("video".into(), 50), Some(25));
        assert_eq!(tracker.update("video".into(), 100), Some(50));
        assert_eq!(tracker.update("audio".into(), 20), Some(60));
        assert_eq!(tracker.update("audio".into(), 100), Some(99));
    }

    #[test]
    fn ignores_partial_downloads() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("source.webm.part"), b"partial").unwrap();
        fs::write(temp.path().join("source.f395.webm"), b"video only").unwrap();
        assert!(find_downloaded_media(temp.path()).unwrap().is_none());
        let complete = temp.path().join("source.webm");
        fs::write(&complete, b"complete").unwrap();
        assert_eq!(find_downloaded_media(temp.path()).unwrap(), Some(complete));
    }

    #[test]
    fn download_peak_estimate_is_conservative_with_margin() {
        // video 6_000 + audio 500 → streams 6_500 → peak 13_000 → +10% = 14_300
        let peak = estimate_download_peak_bytes(&[6_000, 500]).unwrap();
        assert_eq!(peak, 14_300);
        assert!(estimate_download_peak_bytes(&[]).is_err());
        assert!(estimate_download_peak_bytes(&[0]).is_err());
        assert!(estimate_download_peak_bytes(&[u64::MAX, 1]).is_err());
        assert!(estimate_download_peak_bytes(&[u64::MAX / 2 + 1]).is_err());
    }

    #[test]
    fn selected_format_plan_requires_exact_filesize_and_pins_format_ids() {
        let multi = serde_json::json!({
            "duration": 120.0,
            "requested_formats": [
                {"format_id": "298", "filesize": 100u64, "filesize_approx": 150u64},
                {"format_id": "251", "filesize": 40u64, "filesize_approx": 999u64}
            ]
        });
        let plan = selected_format_plan_from_info(&multi).unwrap();
        // Exact filesize only — approx is ignored.
        assert_eq!(plan.stream_sizes, vec![100, 40]);
        assert_eq!(plan.format_ids, vec!["298".to_string(), "251".to_string()]);
        assert_eq!(plan.format_selector, "298+251");
        assert_eq!(plan.duration_seconds, 120.0);

        let approx_only = serde_json::json!({
            "duration": 10.0,
            "format_id": "22",
            "filesize_approx": 1_024u64
        });
        let err = selected_format_plan_from_info(&approx_only).unwrap_err();
        assert_eq!(err, unsafe_size_error());
        assert!(err.contains("정확한 용량"));

        let bad_id = serde_json::json!({
            "duration": 10.0,
            "requested_formats": [{"format_id": "298;rm", "filesize": 10u64}]
        });
        assert_eq!(
            selected_format_plan_from_info(&bad_id).unwrap_err(),
            unsafe_size_error()
        );
    }

    #[test]
    fn format_selector_from_ids_fail_closed_on_empty_or_invalid() {
        assert!(format_selector_from_ids(&[]).is_err());
        assert!(validate_format_id("").is_err());
        assert!(validate_format_id("a/b").is_err());
        assert_eq!(
            format_selector_from_ids(&["137".into(), "140".into()]).unwrap(),
            "137+140"
        );
    }

    #[test]
    fn read_capped_bytes_truncates_and_flags_overflow() {
        let (buf, exceeded) = read_capped_bytes(std::io::Cursor::new(vec![1u8; 10]), 4).unwrap();
        assert!(exceeded);
        assert_eq!(buf, vec![1, 1, 1, 1]);
        let (buf2, exceeded2) = read_capped_bytes(std::io::Cursor::new(vec![9u8; 3]), 8).unwrap();
        assert!(!exceeded2);
        assert_eq!(buf2, vec![9, 9, 9]);
    }

    #[test]
    fn minimal_metadata_log_has_no_urls_or_raw_stderr() {
        let plan = SelectedFormatPlan {
            format_selector: "298+251".into(),
            format_ids: vec!["298".into(), "251".into()],
            stream_sizes: vec![10, 20],
            duration_seconds: 30.0,
        };
        let log = build_minimal_metadata_log(
            Some(&plan),
            Some(0),
            100,
            0,
            false,
            false,
            true,
            None,
        );
        let text = log.to_string();
        assert!(!text.contains("http"));
        assert!(!text.contains("youtube"));
        assert!(!text.contains("Authorization"));
        assert_eq!(log["formatSelector"], "298+251");
        assert_eq!(log["streamFilesizes"][0], 10);
        assert_eq!(log["durationSeconds"], 30.0);
    }

    #[test]
    fn pure_plan_same_volume_takes_max_of_download_peak_and_analysis() {
        // S=10_000 → P=22_000; W = estimate_analysis_workspace_bytes(10000, 3600)
        let streams = [7_000u64, 3_000];
        let duration = 3600.0;
        let s = sum_stream_sizes(&streams).unwrap();
        let peak = download_peak_from_streams_total(s).unwrap();
        let analysis = media::estimate_analysis_workspace_bytes(s, duration);
        let analysis_phase = s.checked_add(analysis).unwrap(); // home+job same volume sums
        let expected = peak.max(analysis_phase);

        let plan =
            plan_pre_download_volume_bytes(&streams, duration, "volA", "volA", "volA").unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan.get("volA").copied().unwrap(), expected);
        // Download peak alone must not ignore analysis when analysis phase is larger.
        assert!(expected >= peak);
        assert!(expected >= analysis_phase);
    }

    #[test]
    fn pure_plan_distinct_home_temp_job_volumes() {
        let streams = [5_000u64, 1_000];
        let duration = 7200.0;
        let s = sum_stream_sizes(&streams).unwrap();
        let budget = stream_budget_with_margin(s).unwrap(); // S + S/10
        let analysis = media::estimate_analysis_workspace_bytes(s, duration);

        let plan =
            plan_pre_download_volume_bytes(&streams, duration, "homeVol", "tempVol", "jobVol")
                .unwrap();
        assert_eq!(plan.len(), 3);
        // home: max(download B, analysis S)
        assert_eq!(plan["homeVol"], budget.max(s));
        // temp: download B only
        assert_eq!(plan["tempVol"], budget);
        // job: analysis W only
        assert_eq!(plan["jobVol"], analysis);
    }

    #[test]
    fn pure_plan_overflow_and_unknown_sizes_fail_closed() {
        assert!(plan_pre_download_volume_bytes(&[], 1.0, "a", "a", "a").is_err());
        assert!(plan_pre_download_volume_bytes(&[0], 1.0, "a", "a", "a").is_err());
        assert!(plan_pre_download_volume_bytes(&[u64::MAX, 1], 1.0, "a", "a", "a").is_err());
        assert!(plan_pre_download_volume_bytes(&[u64::MAX / 2 + 1], 1.0, "a", "a", "a").is_err());
        let err = duration_seconds_from_info(&serde_json::json!({})).unwrap_err();
        assert_eq!(err, metadata_probe_network_message());
    }

    #[test]
    fn metadata_probe_messages_are_separated_and_hide_tool_names() {
        let classes = [
            MetadataProbeFailureClass::NetworkOrAvailability,
            MetadataProbeFailureClass::LocalEnvironment,
            MetadataProbeFailureClass::ToolRuntime,
            MetadataProbeFailureClass::OversizedOutput,
            MetadataProbeFailureClass::UnsafeSize,
        ];
        let mut bodies = std::collections::BTreeSet::new();
        for class in classes {
            let msg = metadata_probe_user_message(class);
            let lower = msg.to_ascii_lowercase();
            assert!(!lower.contains("yt-dlp") && !lower.contains("ffmpeg"), "{msg}");
            assert!(!msg.contains("종료 코드"), "{msg}");
            assert!(msg.contains("다시 시도") || msg.contains("선택"), "{msg}");
            bodies.insert(msg);
        }
        // Distinct recovery guidance for the four Oracle-required classes (+ oversized).
        assert!(bodies.len() >= 4);
        assert!(metadata_probe_network_message().contains("네트워크"));
        assert!(metadata_probe_local_environment_message().contains("권한"));
        assert!(metadata_probe_tool_runtime_message().contains("다시 설치") || metadata_probe_tool_runtime_message().contains("다시 시작"));
        assert!(unsafe_size_error().contains("정확한 용량"));
    }

    #[test]
    fn production_path_plan_uses_aggregate_and_matches_pure_same_volume() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("youtube-download");
        let job = root.path().to_path_buf();
        fs::create_dir_all(&home).unwrap();
        let streams = [64u64, 32];
        let duration = 60.0;
        let pure = plan_pre_download_volume_bytes(
            &streams,
            duration,
            &volume_identity(&home).unwrap(),
            &volume_identity(&home).unwrap(),
            &volume_identity(&job).unwrap(),
        )
        .unwrap();
        let path_plan =
            plan_pre_download_path_requirements(&streams, duration, &home, &home, &job).unwrap();
        assert_eq!(path_plan.len(), pure.len());
        for (id, _path, need) in &path_plan {
            assert_eq!(pure.get(id).copied(), Some(*need));
        }
        // Tiny streams on a real temp volume must pass free-space check.
        let need = ensure_planned_download_space(&streams, duration, &home, &home, &job).unwrap();
        assert_eq!(need, *pure.values().max().unwrap());
    }

    #[test]
    fn production_path_plan_shortage_message_is_actionable() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("dl");
        let job = root.path().to_path_buf();
        fs::create_dir_all(&home).unwrap();
        let free = free_disk_space_bytes(&home).unwrap();
        // Construct S so download peak P = 2S + floor(2S/10) strictly exceeds free without overflow.
        // S = free/2 + 1 ⇒ 2S >= free + 1 > free ⇒ P > free. Overflow paths are covered elsewhere.
        let stream = free
            .checked_div(2)
            .and_then(|half| half.checked_add(1))
            .filter(|&s| s > 0 && s <= u64::MAX / 3)
            .expect("free space too large to build a non-overflowing shortage fixture");
        let peak = download_peak_from_streams_total(stream).expect("peak must compute");
        assert!(
            peak > free,
            "fixture must require more free space than available (peak={peak}, free={free})"
        );
        let err = ensure_planned_download_space(&[stream], 3600.0, &home, &home, &job)
            .expect_err("shortage guard must return Err when required peak exceeds free space");
        assert!(
            err.contains("저장 공간이 부족합니다"),
            "expected shortage message, got: {err}"
        );
        assert!(err.contains("확보"), "{err}");
        assert!(err.contains("내려받기"), "{err}");
    }
}
