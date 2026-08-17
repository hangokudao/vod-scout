mod acquisition;
mod captions;
mod domain;
mod integrity;
mod media;
mod queue;
mod resource;
mod storage;
mod whisper;

use crate::domain::{
    normalize_candidate_count, AnalysisMode, Candidate, CandidateDecision, CandidateRecognitionRun,
    CandidateRevision, JobSnapshot, JobStatus, RecognitionRunStatus, Scenario, SourceKind,
    TranscriptQualityStatus,
};
use crate::whisper::WhisperSettings;
use crate::resource::{ResourceDecision, ResourceSample, ResourceStage, StageResourceMetric};
use crate::storage::JobStore;
use crate::queue::{InstanceLease, QueueIndex, QueueStore, QueueTransitionState};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use uuid::Uuid;

struct AppState {
    store: JobStore,
    queue_store: QueueStore,
    queue: Mutex<QueueIndex>,
    _instance_lease: InstanceLease,
    resource_dir: PathBuf,
    job: Mutex<Option<JobSnapshot>>,
    running: AtomicBool,
    cancel_requested: AtomicBool,
    manual_running: AtomicBool,
    heavy_tool_gate: std::sync::Mutex<()>,
}

impl AppState {
    fn new(data_dir: PathBuf, resource_dir: PathBuf) -> Result<Self, String> {
        let store = JobStore::new(data_dir.clone()).map_err(|error| error.to_string())?;
        let queue_store = QueueStore::new(data_dir.clone()).map_err(|error| error.to_string())?;
        let _instance_lease = InstanceLease::acquire(&data_dir)?;
        let ids = store.list_jobs().map_err(|error| error.to_string())?.into_iter().map(|job| job.id);
        let queue = queue_store.load_or_create(ids)?;
        Ok(Self {
            store,
            queue_store,
            queue: Mutex::new(queue),
            _instance_lease,
            resource_dir,
            job: Mutex::new(None),
            running: AtomicBool::new(false),
            cancel_requested: AtomicBool::new(false),
            manual_running: AtomicBool::new(false),
            heavy_tool_gate: std::sync::Mutex::new(()),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateJobInput {
    source_kind: SourceKind,
    source_label: String,
    scenario: Scenario,
    #[serde(default)]
    analysis_mode: AnalysisMode,
    #[serde(default)]
    analysis_start_seconds: Option<u32>,
    #[serde(default)]
    analysis_end_seconds: Option<u32>,
    #[serde(default)]
    whisper: WhisperSettings,
    #[serde(default = "default_candidate_count_input")]
    candidate_count: u8,
}

fn default_candidate_count_input() -> u8 { 20 }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeInfo {
    app_version: &'static str,
    data_directory: String,
    worker_source: &'static str,
    analysis_mode: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobStorageInfo {
    size_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredJobInfo {
    snapshot: JobSnapshot,
    size_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum WorkerEvent {
    Heartbeat {
        unit: u32,
    },
    Progress {
        unit: u32,
        status: JobStatus,
        stage_label: String,
        message: String,
    },
    Candidates {
        candidates: Vec<Candidate>,
    },
    Failed {
        message: String,
        detail: String,
    },
    Completed,
}

fn validate_source(input: &CreateJobInput) -> Result<(), String> {
    let value = input.source_label.trim();
    if value.is_empty() {
        return Err("입력 소스를 선택하거나 주소를 입력해 주세요.".into());
    }

    if input.analysis_mode == AnalysisMode::Range {
        let start = input.analysis_start_seconds.unwrap_or(0);
        let end = input
            .analysis_end_seconds
            .ok_or_else(|| "구간 분석의 종료 시간을 입력해 주세요.".to_string())?;
        if start >= end {
            return Err("구간 종료 시간은 시작 시간보다 뒤여야 합니다.".into());
        }
    }

    match input.source_kind {
        SourceKind::Local if !PathBuf::from(value).is_file() => {
            Err("선택한 로컬 영상 파일을 찾을 수 없습니다.".into())
        }
        SourceKind::Youtube => acquisition::validate_youtube_url(value),
        _ => Ok(()),
    }
}

fn last_sequence(job: &JobSnapshot) -> u64 {
    job.activity.last().map(|event| event.sequence).unwrap_or(0)
}

fn save_queue(state: &AppState, queue: &QueueIndex) -> Result<(), String> {
    state.queue_store.save(queue)
}

fn queue_snapshot(state: &AppState) -> Result<QueueIndex, String> {
    state.queue.lock().map(|queue| queue.clone()).map_err(|_| "작업 대기열 잠금이 손상됐습니다.".into())
}

fn set_queue_state(state: &AppState, transition_state: QueueTransitionState) -> Result<QueueIndex, String> {
    let queue = {
        let mut guard = state.queue.lock().map_err(|_| "작업 대기열 잠금이 손상됐습니다.".to_string())?;
        guard.transition_state = transition_state;
        guard.clone()
    };
    save_queue(state, &queue)?;
    Ok(queue)
}

fn register_job_in_queue(state: &AppState, id: String) -> Result<(), String> {
    let queue = {
        let mut guard = state.queue.lock().map_err(|_| "작업 대기열 잠금이 손상됐습니다.".to_string())?;
        guard.add(id)?;
        guard.clone()
    };
    save_queue(state, &queue)
}

fn remove_job_from_queue(state: &AppState, id: &str) -> Result<(), String> {
    let queue = {
        let mut guard = state.queue.lock().map_err(|_| "작업 대기열 잠금이 손상됐습니다.".to_string())?;
        guard.remove(id)?;
        guard.clone()
    };
    save_queue(state, &queue)
}

fn record_delete_failure(state: &AppState, id: &str, reason: &str) {
    let Ok(mut jobs) = state.store.list_jobs() else { return; };
    let Some(mut job) = jobs.drain(..).find(|job| job.id == id) else { return; };
    job.delete_failure_reason = Some(reason.into());
    job.error_message = Some("작업 폴더 삭제 실패".into());
    job.error_detail = Some(reason.into());
    job.push_activity("delete-error", &format!("작업 폴더를 삭제하지 못했습니다: {reason}"));
    let _ = state.store.save(&job);
}

fn mutate_job<R, F>(
    app: &tauri::AppHandle<R>,
    state: &AppState,
    mutation: F,
) -> Result<JobSnapshot, String>
where
    R: tauri::Runtime,
    F: FnOnce(&mut JobSnapshot) -> Result<(), String>,
{
    let (snapshot, new_events) = {
        let mut guard = state
            .job
            .lock()
            .map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())?;
        let job = guard
            .as_mut()
            .ok_or_else(|| "먼저 작업을 만들어 주세요.".to_string())?;
        let previous_sequence = last_sequence(job);
        mutation(job)?;
        let new_events = job
            .activity
            .iter()
            .filter(|event| event.sequence > previous_sequence)
            .cloned()
            .collect::<Vec<_>>();
        (job.clone(), new_events)
    };

    state
        .store
        .save(&snapshot)
        .map_err(|error| error.to_string())?;
    for event in new_events {
        state
            .store
            .append_event(&snapshot.id, &event)
            .map_err(|error| error.to_string())?;
    }
    app.emit("job-updated", &snapshot)
        .map_err(|error| format!("화면에 상태를 알리지 못했습니다: {error}"))?;
    Ok(snapshot)
}

fn resume_fixture_status(completed_units: u32) -> JobStatus {
    match completed_units {
        0 | 1 => JobStatus::Acquiring,
        2 => JobStatus::Probing,
        3 => JobStatus::ExtractingAudio,
        4..=6 => JobStatus::Transcribing,
        7 => JobStatus::AudioSignals,
        8 => JobStatus::ChatSignals,
        9..=10 => JobStatus::Fusing,
        _ => JobStatus::Ranking,
    }
}

fn resume_media_status(completed_units: u32, total_units: u32) -> JobStatus {
    if completed_units == 0 {
        JobStatus::Acquiring
    } else if completed_units == 1 {
        JobStatus::Probing
    } else if total_units > 6 && completed_units < total_units.saturating_sub(4) {
        JobStatus::Transcribing
    } else if completed_units == total_units.saturating_sub(4) {
        JobStatus::AudioSignals
    } else if completed_units == total_units.saturating_sub(3) {
        JobStatus::ChatSignals
    } else if completed_units == total_units.saturating_sub(2) {
        JobStatus::Fusing
    } else {
        JobStatus::Ranking
    }
}

fn migrate_snapshot(mut loaded: JobSnapshot) -> (JobSnapshot, bool) {
    if loaded.schema_version < 4 {
        loaded.schema_version = 4;
        loaded.analysis_mode = AnalysisMode::Full;
        loaded.analysis_start_seconds = None;
        loaded.analysis_end_seconds = None;
        if loaded.status != JobStatus::ReviewReady {
            loaded.completed_units = 0;
            loaded.total_units = 12;
            loaded.status = JobStatus::Interrupted;
            loaded.error_message =
                Some("이전 버전 작업은 분석 설정을 확인한 뒤 다시 시작해야 합니다.".into());
            loaded.error_detail = Some("v0.3.2 체크포인트 fingerprint 초기화".into());
            loaded.push_activity("migration", "이전 체크포인트를 안전하게 무효화했습니다.");
        }
        return (loaded, true);
    }

    if loaded.schema_version == 4 {
        loaded.schema_version = 5;
        loaded.push_activity("migration", "schema 4 작업을 새 음성 인식 설정으로 복원했습니다.");
        return (loaded, true);
    }
    (loaded, false)
}

const APP_INTERRUPTED_RECOGNITION_REASON: &str = "앱 종료로 음성 인식이 중단됐습니다.";

fn recover_started_recognition_runs(job: &mut JobSnapshot) -> bool {
    let mut recovered = false;
    for run in &mut job.recognition_runs {
        if run.status != RecognitionRunStatus::Started {
            continue;
        }
        let evidence = run.backend_evidence.clone();
        if run.fail(Utc::now(), APP_INTERRUPTED_RECOGNITION_REASON.into(), evidence).is_ok() {
            recovered = true;
        }
    }
    if recovered {
        job.push_activity("recovery", "앱 종료로 진행 중이던 후보 음성 인식을 실패 처리했습니다.");
    }
    recovered
}

#[tauri::command]
fn bootstrap(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<JobSnapshot>, String> {
    let jobs = state.store.list_jobs().map_err(|error| error.to_string())?;
    if jobs.is_empty() { return Ok(None); }
    let mut interrupted = false;
    for original in jobs {
        let (mut loaded, migrated) = migrate_snapshot(original);
        let recovered_runs = recover_started_recognition_runs(&mut loaded);
        let was_active = loaded.status.is_active();
        if was_active {
            loaded.status = JobStatus::Interrupted;
            loaded.error_message = Some("이전 실행이 끝나기 전에 앱이 종료됐습니다.".into());
            loaded.error_detail = Some("마지막 완료 단위 다음부터 재개할 수 있습니다.".into());
            loaded.push_activity("recovery", "중단된 작업을 복원했습니다. 사용자가 재개 또는 취소해야 합니다.");
            interrupted = true;
        }
        if migrated || recovered_runs || was_active {
            state.store.save(&loaded).map_err(|error| error.to_string())?;
        }
    }
    if interrupted {
        set_queue_state(&state, QueueTransitionState::Interrupted)?;
    } else if queue_snapshot(&state)?.transition_state == QueueTransitionState::Running {
        set_queue_state(&state, QueueTransitionState::Idle)?;
    }
    let queue = queue_snapshot(&state)?;
    let selected_id = state.store.load_latest().ok().map(|job| job.id)
        .filter(|id| queue.contains(id))
        .or_else(|| queue.ordered_job_ids.first().cloned());
    let Some(selected_id) = selected_id else { return Ok(None); };
    let loaded = state.store.list_jobs().map_err(|error| error.to_string())?.into_iter().find(|job| job.id == selected_id)
        .ok_or_else(|| "대기열 작업 폴더를 찾을 수 없습니다.".to_string())?;
    *state.job.lock().map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())? = Some(loaded.clone());
    app.emit("job-updated", &loaded)
        .map_err(|error| error.to_string())?;
    Ok(Some(loaded))
}

#[tauri::command]
fn create_job(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    input: CreateJobInput,
) -> Result<JobSnapshot, String> {
    validate_source(&input)?;
    let job = JobSnapshot::new(
        Uuid::new_v4().to_string(),
        input.source_kind,
        input.source_label.trim().into(),
        input.scenario,
        input.analysis_mode,
        input.analysis_start_seconds,
        input.analysis_end_seconds,
    );
    let mut job = job;
    job.candidate_count = normalize_candidate_count(input.candidate_count);
    job.whisper = input.whisper.normalized();
    state.store.save(&job).map_err(|error| error.to_string())?;
    register_job_in_queue(&state, job.id.clone())?;
    if let Some(event) = job.activity.last() {
        state
            .store
            .append_event(&job.id, event)
            .map_err(|error| error.to_string())?;
    }
    if !state.running.load(Ordering::SeqCst) {
        *state
            .job
            .lock()
            .map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())? = Some(job.clone());
        app.emit("job-updated", &job)
            .map_err(|error| error.to_string())?;
    }
    Ok(job)
}

fn preserve_candidate_revision(job: &mut JobSnapshot, reason: &str) {
    if job.candidates.is_empty() {
        return;
    }
    job.candidate_revisions.push(CandidateRevision {
        revision: job.candidate_revision,
        candidate_count: job.candidate_count,
        reason: reason.into(),
        created_at: Utc::now(),
        candidates: job.candidates.clone(),
    });
    job.candidate_revision = job.candidate_revision.saturating_add(1);
}

fn candidates_for_count(job: &JobSnapshot, count: u8) -> Vec<Candidate> {
    let mut candidates = if job.candidate_pool.is_empty() {
        job.candidates.clone()
    } else {
        job.candidate_pool.clone()
    };
    candidates.sort_by(|left, right| {
        right.total_score.cmp(&left.total_score)
            .then_with(|| left.start_seconds.cmp(&right.start_seconds))
            .then_with(|| left.id.cmp(&right.id))
    });
    let mut decisions = job.candidates.iter().map(|candidate| (candidate.id.as_str(), candidate.decision)).collect::<std::collections::HashMap<_, _>>();
    for candidate in &job.candidate_pool {
        decisions.entry(candidate.id.as_str()).or_insert(candidate.decision);
    }
    candidates.truncate(count as usize);
    for candidate in &mut candidates {
        candidate.decision = decisions.get(candidate.id.as_str()).copied().unwrap_or(CandidateDecision::Pending);
    }
    candidates
}

pub(crate) fn continue_queue<R: tauri::Runtime>(app: tauri::AppHandle<R>, state: Arc<AppState>) {
    if state.queue.lock().ok().map(|queue| queue.transition_state != QueueTransitionState::Running).unwrap_or(true) { return; }
    let next = state.store.list_jobs().ok().and_then(|jobs| {
        let queue = state.queue.lock().ok()?;
        queue.ordered_job_ids.iter().find_map(|id| jobs.iter().find(|job| &job.id == id && job.status == JobStatus::Created).cloned())
    });
    let Some(mut next) = next else {
        let _ = set_queue_state(&state, QueueTransitionState::Idle);
        return;
    };
    next.current_stage_label = "worker 시작".into();
    next.push_activity("start", "앞선 작업이 끝나 대기열의 다음 작업을 시작합니다.");
    if next.transition(JobStatus::Acquiring).is_err() { return; }
    if state.store.save(&next).is_err() { return; }
    if let Ok(mut guard) = state.job.lock() { *guard = Some(next.clone()); } else { return; }
    state.running.store(true, Ordering::SeqCst);
    let next_id = next.id.clone();
    match next.source_kind {
        SourceKind::Local => tauri::async_runtime::spawn_blocking(move || media::run_media_pipeline(app, state, next_id)),
        SourceKind::Youtube => tauri::async_runtime::spawn_blocking(move || acquisition::run_youtube_pipeline(app, state, next_id)),
        SourceKind::Demo => tauri::async_runtime::spawn(run_worker(app, state, next_id)),
    };
}

fn sync_candidate_decision(
    job: &mut JobSnapshot,
    candidate_id: &str,
    decision: CandidateDecision,
) -> Result<(), String> {
    let candidate = job
        .candidates
        .iter_mut()
        .find(|candidate| candidate.id == candidate_id)
        .ok_or_else(|| "후보를 찾을 수 없습니다.".to_string())?;
    candidate.decision = decision;
    if let Some(pool_candidate) = job.candidate_pool.iter_mut().find(|candidate| candidate.id == candidate_id) {
        pool_candidate.decision = decision;
    }
    Ok(())
}

#[tauri::command]
fn set_candidate_count(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    job_id: String,
    candidate_count: u8,
) -> Result<JobSnapshot, String> {
    if state.running.load(Ordering::SeqCst) {
        return Err("분석 또는 다시 음성 인식이 실행 중일 때 후보 수를 바꿀 수 없습니다.".into());
    }
    let candidate_count = normalize_candidate_count(candidate_count);
    mutate_job(&app, &state, |job| {
        if job.id != job_id || job.status != JobStatus::ReviewReady {
            return Err("검토 준비가 끝난 현재 작업에서만 후보 수를 바꿀 수 있습니다.".into());
        }
        if job.candidate_count == candidate_count {
            return Ok(());
        }
        preserve_candidate_revision(job, "후보 수 변경 전 기존 결과 보존");
        job.candidate_count = candidate_count;
        job.candidates = candidates_for_count(job, candidate_count);
        job.push_activity("candidates", &format!("후보 수를 {}개로 바꿔 새 개정으로 저장했습니다.", candidate_count));
        Ok(())
    })
}

async fn run_worker<R: tauri::Runtime>(app: tauri::AppHandle<R>, state: Arc<AppState>, job_id: String) {
    let (scenario, start_unit) = {
        let guard = match state.job.lock() {
            Ok(guard) => guard,
            Err(_) => {
                state.running.store(false, Ordering::SeqCst);
                return;
            }
        };
        let Some(job) = guard.as_ref() else {
            state.running.store(false, Ordering::SeqCst);
            return;
        };
        (job.scenario, job.completed_units)
    };

    let command = match app.shell().sidecar("fixture-worker") {
        Ok(command) => command.args([
            "--scenario",
            scenario.as_arg(),
            "--start-unit",
            &start_unit.to_string(),
        ]),
        Err(error) => {
            let _ = mutate_job(&app, &state, |job| {
                job.transition(JobStatus::Failed)?;
                job.error_message = Some("분석 worker를 찾지 못했습니다.".into());
                job.error_detail = Some(error.to_string());
                job.push_activity("error", "분석 worker 실행 준비에 실패했습니다.");
                Ok(())
            });
            state.running.store(false, Ordering::SeqCst);
            return;
        }
    };

    let (mut receiver, child) = match command.spawn() {
        Ok(result) => result,
        Err(error) => {
            let _ = mutate_job(&app, &state, |job| {
                job.transition(JobStatus::Failed)?;
                job.error_message = Some("분석 worker를 시작하지 못했습니다.".into());
                job.error_detail = Some(error.to_string());
                job.push_activity("error", "분석 worker 시작에 실패했습니다.");
                Ok(())
            });
            state.running.store(false, Ordering::SeqCst);
            return;
        }
    };
    let mut child = Some(child);

    let _ = mutate_job(&app, &state, |job| {
        job.owned_child_processes = 1;
        Ok(())
    });

    let mut last_heartbeat = Instant::now();
    let mut completed = false;
    let mut terminal_state_written = false;
    let mut cancel_completion = false;

    loop {
        if state.cancel_requested.load(Ordering::SeqCst) {
            if !cancel_completion {
                if let Some(child) = child.take() { let _ = child.kill(); }
                cancel_completion = true;
                let _ = mutate_job(&app, &state, |job| {
                    job.current_stage_label = "실행 중 도구 종료 중".into();
                    job.push_activity("cancel", "취소 요청을 반영했습니다. 자식 프로세스 종료를 기다립니다.");
                    Ok(())
                });
            }
        }

        match tokio::time::timeout(Duration::from_millis(250), receiver.recv()).await {
            Ok(Some(CommandEvent::Stdout(bytes))) => {
                let line = String::from_utf8_lossy(&bytes).trim().to_string();
                let event = match serde_json::from_str::<WorkerEvent>(&line) {
                    Ok(event) => event,
                    Err(error) => {
                        if let Some(child) = child.take() { let _ = child.kill(); }
                        let _ = mutate_job(&app, &state, |job| {
                            job.transition(JobStatus::Failed)?;
                            job.error_message =
                                Some("분석 worker가 이해할 수 없는 응답을 보냈습니다.".into());
                            job.error_detail = Some(format!("{error}: {line}"));
                            job.push_activity(
                                "error",
                                "잘못된 worker 이벤트 때문에 작업을 중지했습니다.",
                            );
                            Ok(())
                        });
                        terminal_state_written = true;
                        break;
                    }
                };

                match event {
                    WorkerEvent::Heartbeat { unit } => {
                        last_heartbeat = Instant::now();
                        if let Ok(mut guard) = state.job.lock() {
                            if let Some(job) = guard.as_mut() {
                                if job.id == job_id && unit >= job.completed_units {
                                    job.last_heartbeat_at = Some(Utc::now());
                                }
                            }
                        }
                    }
                    WorkerEvent::Progress {
                        unit,
                        status,
                        stage_label,
                        message,
                    } => {
                        last_heartbeat = Instant::now();
                        if mutate_job(&app, &state, |job| {
                            job.apply_progress(unit, status, stage_label, message)
                        })
                        .is_err()
                        {
                            if let Some(child) = child.take() { let _ = child.kill(); }
                            let _ = mutate_job(&app, &state, |job| {
                                job.transition(JobStatus::Failed)?;
                                job.error_message =
                                    Some("진행 순서가 올바르지 않아 작업을 중지했습니다.".into());
                                job.error_detail = Some(format!("수신 단위: {unit}"));
                                job.push_activity("error", "진행 이벤트 순서 검증에 실패했습니다.");
                                Ok(())
                            });
                            terminal_state_written = true;
                            break;
                        }
                    }
                    WorkerEvent::Candidates { candidates } => {
                        let _ = mutate_job(&app, &state, |job| {
                            job.candidate_pool = candidates;
                            job.candidates = job.candidate_pool.clone();
                            job.candidates.truncate(job.candidate_count as usize);
                            job.push_activity(
                                "candidates",
                                "후보 구간 3개를 검토 목록에 추가했습니다.",
                            );
                            Ok(())
                        });
                    }
                    WorkerEvent::Failed { message, detail } => {
                        let _ = mutate_job(&app, &state, |job| {
                            job.transition(JobStatus::Failed)?;
                            job.error_message = Some(message);
                            job.error_detail = Some(detail);
                            job.push_activity(
                                "error",
                                "분석 단계에서 복구 가능한 오류가 발생했습니다.",
                            );
                            Ok(())
                        });
                        terminal_state_written = true;
                        break;
                    }
                    WorkerEvent::Completed => {
                        let _ = mutate_job(&app, &state, |job| {
                            job.transition(JobStatus::ReviewReady)?;
                            job.current_stage_label = "후보 검토 준비".into();
                            job.error_message = None;
                            job.error_detail = None;
                            job.push_activity(
                                "complete",
                                "분석을 마쳤습니다. 후보를 검토해 주세요.",
                            );
                            Ok(())
                        });
                        completed = true;
                        terminal_state_written = true;
                        break;
                    }
                }
            }
            Ok(Some(CommandEvent::Stderr(bytes))) => {
                let detail = String::from_utf8_lossy(&bytes).trim().to_string();
                if !detail.is_empty() {
                    let _ = mutate_job(&app, &state, |job| {
                        job.push_activity("diagnostic", &format!("worker 진단: {detail}"));
                        Ok(())
                    });
                }
            }
            Ok(Some(CommandEvent::Terminated(payload))) => {
                if cancel_completion {
                    break;
                }
                if !completed && !terminal_state_written {
                    let _ = mutate_job(&app, &state, |job| {
                        job.transition(JobStatus::Interrupted)?;
                        job.error_message =
                            Some("분석 worker가 예상보다 일찍 종료됐습니다.".into());
                        job.error_detail = Some(format!("종료 코드: {:?}", payload.code));
                        job.push_activity(
                            "interrupted",
                            "마지막 완료 단위에서 작업이 중단됐습니다.",
                        );
                        Ok(())
                    });
                    terminal_state_written = true;
                }
                break;
            }
            Ok(Some(CommandEvent::Error(message))) => {
                let _ = mutate_job(&app, &state, |job| {
                    job.transition(JobStatus::Interrupted)?;
                    job.error_message = Some("worker 통신이 끊겼습니다.".into());
                    job.error_detail = Some(message);
                    job.push_activity("interrupted", "worker 통신 오류로 작업이 중단됐습니다.");
                    Ok(())
                });
                terminal_state_written = true;
                break;
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(_) if last_heartbeat.elapsed() >= Duration::from_secs(3) => {
                if let Some(child) = child.take() { let _ = child.kill(); }
                let _ = mutate_job(&app, &state, |job| {
                    job.transition(JobStatus::Interrupted)?;
                    job.error_message = Some("분석 worker의 응답이 멈췄습니다.".into());
                    job.error_detail = Some("3초 동안 heartbeat를 받지 못했습니다.".into());
                    job.push_activity("stalled", "응답 정지를 감지해 worker를 종료했습니다.");
                    Ok(())
                });
                terminal_state_written = true;
                break;
            }
            Err(_) => {}
        }
    }

    if !terminal_state_written && !completed && !cancel_completion {
        let _ = mutate_job(&app, &state, |job| {
            job.transition(JobStatus::Interrupted)?;
            job.error_message = Some("worker 이벤트 스트림이 예고 없이 끝났습니다.".into());
            job.error_detail = Some("event channel closed".into());
            job.push_activity(
                "interrupted",
                "worker 연결이 끝나 작업을 중단 상태로 저장했습니다.",
            );
            Ok(())
        });
    }

    let _ = mutate_job(&app, &state, |job| {
        job.owned_child_processes = 0;
        Ok(())
    });
    if cancel_completion {
        let _ = mutate_job(&app, &state, |job| {
            if job.status != JobStatus::Cancelled { job.transition(JobStatus::Cancelled)?; }
            job.current_stage_label = "사용자가 취소함".into();
            job.error_message = None;
            job.error_detail = None;
            job.push_activity("cancel", "자식 프로세스 0개를 확인하고 작업을 취소했습니다. 이어서 재개할 수 있습니다.");
            Ok(())
        });
    }
    state.cancel_requested.store(false, Ordering::SeqCst);
    state.running.store(false, Ordering::SeqCst);
    continue_queue(app, state);
}

#[tauri::command]
async fn start_job(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    job_id: String,
) -> Result<JobSnapshot, String> {
    if state
        .running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("이미 실행 중인 작업이 있습니다.".into());
    }

    state.cancel_requested.store(false, Ordering::SeqCst);
    let current_id = state.job.lock().map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())?.as_ref().map(|job| job.id.clone());
    if current_id.as_deref() != Some(job_id.as_str()) {
        let loaded = match state.store.list_jobs().map_err(|error| error.to_string())?.into_iter().find(|job| job.id == job_id) {
            Some(job) => job,
            None => {
                state.running.store(false, Ordering::SeqCst);
                return Err("대기열에서 작업을 찾을 수 없습니다.".into());
            }
        };
        *state.job.lock().map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())? = Some(loaded);
    }
    let queue_before_start = queue_snapshot(&state)?;
    let requested_status = state.job.lock().map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())?.as_ref().map(|job| job.status);
    if queue_before_start.transition_state == QueueTransitionState::Interrupted && requested_status != Some(JobStatus::Interrupted) {
        state.running.store(false, Ordering::SeqCst);
        return Err("중단된 작업을 먼저 재개하거나 취소해야 다음 작업을 시작할 수 있습니다.".into());
    }
    if requested_status == Some(JobStatus::Created) {
        let first_waiting = state.store.list_jobs().map_err(|error| error.to_string())?.into_iter()
            .find(|job| queue_before_start.contains(&job.id) && job.status == JobStatus::Created)
            .map(|job| job.id);
        if first_waiting.as_deref() != Some(job_id.as_str()) {
            state.running.store(false, Ordering::SeqCst);
            return Err("대기열 순서가 먼저인 작업부터 시작해야 합니다.".into());
        }
    }
    if let Err(error) = set_queue_state(&state, QueueTransitionState::Running) {
        state.running.store(false, Ordering::SeqCst);
        return Err(error);
    }
    let snapshot = mutate_job(&app, &state, |job| {
        if job.id != job_id {
            return Err("현재 작업과 요청한 작업이 다릅니다.".into());
        }
        if !matches!(
            job.status,
            JobStatus::Created | JobStatus::Cancelled | JobStatus::Interrupted | JobStatus::Failed
        ) {
            return Err("이 상태에서는 작업을 시작하거나 재개할 수 없습니다.".into());
        }
        let target = if job.source_kind != SourceKind::Demo {
            resume_media_status(job.completed_units, job.total_units)
        } else {
            resume_fixture_status(job.completed_units)
        };
        job.transition(target)?;
        job.current_stage_label = if job.completed_units == 0 {
            "worker 시작".into()
        } else {
            format!("{}단위 다음부터 재개", job.completed_units)
        };
        job.error_message = None;
        job.error_detail = None;
        job.push_activity(
            "start",
            if job.completed_units == 0 {
                "분석 worker를 시작합니다."
            } else {
                "저장된 체크포인트 다음부터 분석을 재개합니다."
            },
        );
        Ok(())
    });

    let snapshot = match snapshot {
        Ok(snapshot) => snapshot,
        Err(error) => {
            state.running.store(false, Ordering::SeqCst);
            let _ = set_queue_state(&state, QueueTransitionState::Idle);
            return Err(error);
        }
    };

    let source_kind = snapshot.source_kind;
    let state_arc = Arc::clone(state.inner());
    if source_kind == SourceKind::Local {
        tauri::async_runtime::spawn_blocking(move || {
            media::run_media_pipeline(app, state_arc, job_id)
        });
    } else if source_kind == SourceKind::Youtube {
        tauri::async_runtime::spawn_blocking(move || {
            acquisition::run_youtube_pipeline(app, state_arc, job_id)
        });
    } else {
        tauri::async_runtime::spawn(run_worker(app, state_arc, job_id));
    }
    Ok(snapshot)
}

#[tauri::command]
async fn rerun_candidate_transcription(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    job_id: String,
    candidate_id: String,
) -> Result<JobSnapshot, String> {
    if state.running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return Err("분석 또는 다른 음성 인식이 실행 중입니다.".into());
    }
    state.cancel_requested.store(false, Ordering::SeqCst);
    state.manual_running.store(true, Ordering::SeqCst);
    let run_id = Uuid::new_v4().to_string();
    let started = mutate_job(&app, &state, |job| {
        if job.id != job_id || job.status != JobStatus::ReviewReady {
            return Err("검토 준비가 끝난 현재 작업에서만 다시 음성 인식을 실행할 수 있습니다.".into());
        }
        let original_result = job.candidates.iter().find(|candidate| candidate.id == candidate_id)
            .map(|candidate| candidate.transcript_excerpt.clone())
            .ok_or_else(|| "선택한 후보를 찾을 수 없습니다.".to_string())?;
        preserve_candidate_revision(job, "수동 재음성 인식 전 기존 결과 보존");
        let revision = job.recognition_runs.iter().filter(|run| run.candidate_id == candidate_id)
            .map(|run| run.result_revision).max().unwrap_or(0).saturating_add(1);
        job.recognition_runs.push(CandidateRecognitionRun {
            id: run_id.clone(), candidate_id: candidate_id.clone(), status: RecognitionRunStatus::Started,
            started_at: Utc::now(), completed_at: None, result_revision: revision,
            original_result: Some(original_result), raw_result: None,
            display_result: None, failure_reason: None,
            backend_evidence: "요청됨 · 내장 G2 Whisper 런타임·모델 사용".into(),
        });
        job.current_stage_label = "선택 후보 음성 인식 중".into();
        job.push_activity("recognition", "선택한 후보의 음성을 다시 인식하기 시작했습니다.");
        Ok(())
    });
    let started = match started {
        Ok(snapshot) => snapshot,
        Err(error) => {
            state.manual_running.store(false, Ordering::SeqCst);
            state.running.store(false, Ordering::SeqCst);
            return Err(error);
        }
    };
    let state_arc = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || media::run_candidate_recognition(app, state_arc, job_id, candidate_id, run_id));
    Ok(started)
}

pub(crate) fn record_stage_metric<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &AppState,
    stage: ResourceStage,
    started: Instant,
    sample: ResourceSample,
) -> Result<(), String> {
    let elapsed_ms = Some(started.elapsed().as_millis() as u64);
    let (decision, metric) = {
        let guard = state
            .job
            .lock()
            .map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())?;
        let job = guard
            .as_ref()
            .ok_or_else(|| "현재 작업이 없습니다.".to_string())?;
        let decision = job.resource_policy.evaluate(&sample);
        let disk_bytes = state.store.job_size_bytes(&job.id).ok();
        let mut unavailable_reasons = Vec::new();
        if sample.memory_bytes.is_none() { unavailable_reasons.push("memoryBytes: OS별 프로세스 메모리 측정을 사용할 수 없습니다.".into()); }
        if sample.temp_bytes.is_none() { unavailable_reasons.push("tempBytes: 임시 파일 표본을 이 단계에서 수집하지 않았습니다.".into()); }
        if disk_bytes.is_none() { unavailable_reasons.push("diskBytes: 작업 폴더 크기를 확인할 수 없습니다.".into()); }
        if sample.external_tool_count.is_none() { unavailable_reasons.push("ownedChildProcesses: 외부 도구 프로세스 수를 이 단계에서 관찰하지 않았습니다.".into()); }
        unavailable_reasons.push("cpuPercent: CPU 사용량 측정을 사용할 수 없습니다.".into());
        let policy_status = if job.resource_policy.is_configured() {
            decision.status()
        } else {
            crate::resource::ResourcePolicyStatus::Unconfigured
        };
        let metric = StageResourceMetric {
            stage,
            elapsed_ms,
            cpu_percent: None,
            memory_bytes: sample.memory_bytes,
            disk_bytes,
            temp_bytes: sample.temp_bytes,
            owned_child_processes: sample.external_tool_count,
            unavailable_reasons,
            policy_status,
            policy_reason: decision.reason().map(str::to_string),
        };
        (decision, metric)
    };
    mutate_job(app, state, |job| {
        if let Some(existing) = job.resource_metrics.iter_mut().find(|item| item.stage == stage) {
            let elapsed_ms = accumulate_elapsed_ms(existing.elapsed_ms, metric.elapsed_ms);
            *existing = metric.clone();
            existing.elapsed_ms = elapsed_ms;
        } else {
            job.resource_metrics.push(metric.clone());
        }
        if let Some(reason) = decision.reason() {
            job.push_activity(
                if matches!(decision, ResourceDecision::Warning(_)) { "resource-warning" } else { "resource-limit" },
                reason,
            );
        }
        Ok(())
    })?;
    if let ResourceDecision::HardLimit(reason) = decision {
        return Err(format!("자원 제한 초과: {reason}"));
    }
    Ok(())
}

fn accumulate_elapsed_ms(previous: Option<u64>, current: Option<u64>) -> Option<u64> {
    match (previous, current) {
        (Some(previous), Some(current)) => Some(previous.saturating_add(current)),
        (None, current) => current,
        (previous, None) => previous,
    }
}

/// Validate `job_id` against the active job, then set `cancel_requested`.
/// Mismatched IDs must not arm the global cancel flag. The check+set stays
/// under the job lock and never touches disk, so tool loops can stop before
/// `mutate_job` persistence work.
fn arm_cancel_signal(state: &AppState, job_id: &str) -> Result<(), String> {
    if !state.running.load(Ordering::SeqCst) {
        return Err("현재 실행 중인 작업이 없습니다.".into());
    }
    let guard = state
        .job
        .lock()
        .map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())?;
    let job = guard
        .as_ref()
        .ok_or_else(|| "현재 작업이 없습니다.".to_string())?;
    if job.id != job_id {
        return Err("현재 작업과 요청한 작업이 다릅니다.".into());
    }
    // Signal tool loops before disk I/O so yt-dlp/ffmpeg/whisper can stop immediately.
    state.cancel_requested.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn cancel_job(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    job_id: String,
) -> Result<JobSnapshot, String> {
    if !state.running.load(Ordering::SeqCst) {
        let interrupted = state.job.lock().map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())?.as_ref().is_some_and(|job| job.id == job_id && job.status == JobStatus::Interrupted);
        if interrupted {
            let snapshot = mutate_job(&app, &state, |job| {
                job.transition(JobStatus::Cancelled)?;
                job.current_stage_label = "사용자가 중단 작업을 취소함".into();
                job.error_message = None;
                job.error_detail = None;
                job.push_activity("cancel", "중단된 작업을 사용자가 취소했습니다. 대기열은 다음 작업을 진행합니다.");
                Ok(())
            })?;
            set_queue_state(&state, QueueTransitionState::Running)?;
            continue_queue(app, Arc::clone(state.inner()));
            return Ok(snapshot);
        }
        return Err("현재 실행 중인 작업이 없습니다.".into());
    }
    arm_cancel_signal(&state, &job_id)?;
    if state.manual_running.load(Ordering::SeqCst) {
        return mutate_job(&app, &state, |job| {
            if job.id != job_id || job.status != JobStatus::ReviewReady {
                return Err("현재 선택 후보 음성 인식 상태가 아닙니다.".into());
            }
            job.current_stage_label = "선택 후보 음성 인식 취소 중".into();
            job.push_activity("cancel", "선택 후보 음성 인식 취소를 요청했습니다.");
            Ok(())
        });
    }
    let snapshot = mutate_job(&app, &state, |job| {
        if job.id != job_id {
            return Err("현재 작업과 요청한 작업이 다릅니다.".into());
        }
        if job.status != JobStatus::Cancelling {
            job.transition(JobStatus::Cancelling)?;
        }
        job.current_stage_label = "실행 중 도구 종료 중".into();
        job.push_activity(
            "cancel",
            "작업을 취소합니다. 관련 도구 프로세스를 종료하는 중입니다.",
        );
        Ok(())
    })?;
    Ok(snapshot)
}

#[tauri::command]
fn get_job_storage_info(
    state: State<'_, Arc<AppState>>,
    job_id: String,
) -> Result<JobStorageInfo, String> {
    let guard = state
        .job
        .lock()
        .map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())?;
    let job = guard
        .as_ref()
        .ok_or_else(|| "현재 작업이 없습니다.".to_string())?;
    if job.id != job_id {
        return Err("현재 작업과 요청한 작업이 다릅니다.".into());
    }
    Ok(JobStorageInfo {
        size_bytes: state
            .store
            .job_size_bytes(&job_id)
            .map_err(|error| error.to_string())?,
    })
}

#[tauri::command]
fn get_queue(state: State<'_, Arc<AppState>>) -> Result<QueueIndex, String> {
    queue_snapshot(&state)
}

#[tauri::command]
fn reorder_job(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    job_id: String,
    new_index: usize,
) -> Result<QueueIndex, String> {
    queue::validate_job_id(&job_id)?;
    if state.running.load(Ordering::SeqCst) { return Err("분석 중에는 대기 순서를 바꿀 수 없습니다.".into()); }
    let target = state.store.list_jobs().map_err(|error| error.to_string())?.into_iter().find(|job| job.id == job_id)
        .ok_or_else(|| "대기열에서 작업을 찾을 수 없습니다.".to_string())?;
    if target.status != JobStatus::Created { return Err("실행 대기 상태인 작업만 순서를 바꿀 수 있습니다.".into()); }
    let queue = {
        let mut guard = state.queue.lock().map_err(|_| "작업 대기열 잠금이 손상됐습니다.".to_string())?;
        guard.move_job(&job_id, new_index)?;
        guard.clone()
    };
    save_queue(&state, &queue)?;
    app.emit("queue-updated", &queue).map_err(|error| error.to_string())?;
    Ok(queue)
}

#[tauri::command]
async fn delete_job(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    job_id: String,
) -> Result<(), String> {
    if state.running.load(Ordering::SeqCst) {
        let current_id = state.job.lock().map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())?.as_ref().map(|job| job.id.clone());
        if current_id.as_deref() != Some(job_id.as_str()) { return Err("현재 실행 중인 작업만 안전하게 삭제할 수 있습니다.".into()); }
        arm_cancel_signal(&state, &job_id)?;
        let _ = mutate_job(&app, &state, |job| {
            if job.status.is_active() { job.transition(JobStatus::Cancelling)?; }
            job.current_stage_label = "삭제 전 안전하게 종료하는 중".into();
            job.push_activity("delete", "삭제 요청을 반영하고 자식 프로세스 종료를 기다립니다.");
            Ok(())
        });
        for _ in 0..200 {
            if !state.running.load(Ordering::SeqCst) { break; }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        if state.running.load(Ordering::SeqCst) { return Err("삭제 전 자식 프로세스를 종료하지 못했습니다.".into()); }
    }
    {
        let guard = state
            .job
            .lock()
            .map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())?;
        let job = guard
            .as_ref()
            .ok_or_else(|| "현재 작업이 없습니다.".to_string())?;
        if job.id != job_id || Uuid::parse_str(&job_id).is_err() {
            return Err("삭제할 작업 ID가 현재 작업과 일치하지 않습니다.".into());
        }
    }
    if let Err(error) = state.store.delete_job(&job_id) {
        let reason = error.to_string();
        record_delete_failure(&state, &job_id, &reason);
        return Err(reason);
    }
    remove_job_from_queue(&state, &job_id)?;
    *state
        .job
        .lock()
        .map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())? = None;
    app.emit("job-deleted", &job_id)
        .map_err(|error| format!("화면에 삭제 결과를 알리지 못했습니다: {error}"))?;
    Ok(())
}

fn csv_field(value: &str) -> String {
    let cleaned = value.replace('\0', "");
    let trimmed = cleaned.trim_start();
    let safe = if matches!(trimmed.chars().next(), Some('=' | '+' | '-' | '@')) {
        format!("'{cleaned}")
    } else {
        cleaned
    };
    format!("\"{}\"", safe.replace('"', "\"\""))
}

fn safe_candidate_text(value: &str, status: TranscriptQualityStatus) -> String {
    if status == TranscriptQualityStatus::Uncertain || value.contains('\u{fffd}') {
        "음성 인식 결과가 불확실해 원문을 표시하지 않습니다.".into()
    } else { value.into() }
}

fn safe_candidate_derived_text(value: &str) -> String {
    if value.contains('\u{fffd}') {
        "음성 인식 결과가 불확실해 원문을 표시하지 않습니다.".into()
    } else {
        value.into()
    }
}

#[tauri::command]
fn export_candidates_csv(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    job_id: String,
) -> Result<Option<String>, String> {
    let rows = {
        let guard = state
            .job
            .lock()
            .map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())?;
        let job = guard
            .as_ref()
            .ok_or_else(|| "현재 작업이 없습니다.".to_string())?;
        if job.id != job_id || job.status != JobStatus::ReviewReady {
            return Err("검토 준비가 끝난 현재 작업만 CSV로 내보낼 수 있습니다.".into());
        }
        let mut rows = vec![
            "rank,start,end,start_seconds,end_seconds,total,audio,dialogue,chat,decision,title,transcript"
                .to_string(),
        ];
        for (index, candidate) in job.candidates.iter().enumerate() {
            rows.push(format!(
                "{},{},{},{},{},{},{},{},{},{:?},{},{}",
                index + 1,
                format_timecode(candidate.start_seconds),
                format_timecode(candidate.end_seconds),
                candidate.start_seconds,
                candidate.end_seconds,
                candidate.total_score,
                candidate.audio_score,
                candidate.dialogue_score,
                candidate
                    .chat_score
                    .map(|score| score.to_string())
                    .unwrap_or_default(),
                candidate.decision,
                csv_field(&safe_candidate_derived_text(&candidate.title)),
                csv_field(&safe_candidate_text(&candidate.transcript_excerpt, candidate.transcript_quality_status)),
            ));
        }
        rows
    };
    let Some(selected_path) = app
        .dialog()
        .file()
        .set_title("편집 후보 CSV 저장")
        .set_file_name("vod-scout-candidates.csv")
        .add_filter("CSV", &["csv"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let mut path = selected_path
        .into_path()
        .map_err(|_| "로컬 파일 경로만 선택할 수 있습니다.".to_string())?;
    if path.extension().is_none() {
        path.set_extension("csv");
    }
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"))
        || path.file_name().is_none()
        || path.parent().is_none()
    {
        return Err("저장할 CSV 경로가 올바르지 않습니다.".into());
    }
    let parent = path
        .parent()
        .expect("validated parent")
        .canonicalize()
        .map_err(|_| "CSV를 저장할 폴더를 찾을 수 없습니다.".to_string())?;
    let file_name = path.file_name().expect("validated filename");
    path = parent.join(file_name);
    if path
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err("심볼릭 링크에는 CSV를 저장하지 않습니다.".into());
    }
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(rows.join("\r\n").as_bytes());
    fs::write(&path, bytes).map_err(|error| format!("CSV를 저장하지 못했습니다: {error}"))?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
fn list_jobs(state: State<'_, Arc<AppState>>) -> Result<Vec<StoredJobInfo>, String> {
    let jobs = state.store.list_jobs().map_err(|error| error.to_string())?;
    let ordered = queue_snapshot(&state)?.ordered_job_ids;
    let mut jobs = jobs;
    jobs.sort_by_key(|job| ordered.iter().position(|id| id == &job.id).unwrap_or(usize::MAX));
    jobs
        .into_iter()
        .map(|snapshot| {
            let size_bytes = state
                .store
                .job_size_bytes(&snapshot.id)
                .map_err(|error| error.to_string())?;
            Ok(StoredJobInfo {
                snapshot,
                size_bytes,
            })
        })
        .collect()
}

#[tauri::command]
fn delete_stored_job(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    job_id: String,
) -> Result<(), String> {
    let active_id = state.job.lock().map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())?.as_ref().filter(|_| state.running.load(Ordering::SeqCst)).map(|job| job.id.clone());
    if active_id.as_deref() == Some(job_id.as_str()) { return Err("실행 중인 작업은 안전하게 종료한 뒤 삭제해 주세요.".into()); }
    Uuid::parse_str(&job_id).map_err(|_| "삭제할 작업 ID가 올바르지 않습니다.".to_string())?;
    if let Err(error) = state.store.delete_job(&job_id) {
        let reason = error.to_string();
        record_delete_failure(&state, &job_id, &reason);
        return Err(reason);
    }
    remove_job_from_queue(&state, &job_id)?;
    let cleared_current = {
        let mut guard = state
            .job
            .lock()
            .map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())?;
        if guard.as_ref().is_some_and(|job| job.id == job_id) {
            *guard = None;
            true
        } else {
            false
        }
    };
    if cleared_current {
        app.emit("job-deleted", &job_id)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn delete_all_jobs(app: tauri::AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    if state.running.load(Ordering::SeqCst) {
        return Err("분석 중에는 저장된 작업을 삭제할 수 없습니다.".into());
    }
    state
        .store
        .delete_all_jobs()
        .map_err(|error| error.to_string())?;
    {
        let queue = QueueIndex::default();
        *state.queue.lock().map_err(|_| "작업 대기열 잠금이 손상됐습니다.".to_string())? = queue.clone();
        save_queue(&state, &queue)?;
    }
    *state
        .job
        .lock()
        .map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())? = None;
    app.emit("jobs-deleted", ())
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn format_timecode(seconds: u32) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60
    )
}

#[tauri::command]
async fn prepare_candidate_preview(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    job_id: String,
    candidate_id: String,
) -> Result<media::PreviewMedia, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        media::prepare_candidate_preview(&app, &state, &job_id, &candidate_id)
    })
    .await
    .map_err(|error| format!("미리보기 작업이 중단됐습니다: {error}"))?
}

#[tauri::command]
async fn prepare_candidate_context_preview(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    job_id: String,
    candidate_id: String,
) -> Result<media::PreviewMedia, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        media::prepare_candidate_context_preview(&app, &state, &job_id, &candidate_id)
    })
    .await
    .map_err(|error| format!("맥락 미리보기 작업이 중단됐습니다: {error}"))?
}

#[tauri::command]
fn set_candidate_decision(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    job_id: String,
    candidate_id: String,
    decision: CandidateDecision,
) -> Result<JobSnapshot, String> {
    if state.manual_running.load(Ordering::SeqCst) {
        return Err("선택 후보 음성 인식이 끝난 뒤 후보를 판정할 수 있습니다.".into());
    }
    mutate_job(&app, &state, |job| {
        if job.id != job_id || job.status != JobStatus::ReviewReady {
            return Err("검토 준비가 끝난 현재 작업에서만 후보를 판정할 수 있습니다.".into());
        }
        sync_candidate_decision(job, &candidate_id, decision)?;
        let label = match decision {
            CandidateDecision::Pending => "보류",
            CandidateDecision::Accepted => "채택",
            CandidateDecision::Rejected => "제외",
        };
        job.push_activity("review", &format!("후보를 {label} 처리했습니다."));
        Ok(())
    })
}

#[tauri::command]
fn get_runtime_info(state: State<'_, Arc<AppState>>) -> RuntimeInfo {
    RuntimeInfo {
        app_version: env!("CARGO_PKG_VERSION"),
        data_directory: state.store.root().display().to_string(),
        worker_source: "yt-dlp + Deno / local FFmpeg + whisper.cpp / deterministic demo worker",
        analysis_mode: "local media analysis with direct YouTube download (no API token)",
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::domain::{AnalysisMode, Scenario, SourceKind};

    #[test]
    fn csv_fields_neutralize_spreadsheet_formulas() {
        for dangerous in ["=2+2", "+cmd", "-1+1", "@SUM(A1:A2)", "  =HYPERLINK(\"x\")"] {
            let field = csv_field(dangerous);
            assert!(field.starts_with("\"'"));
        }
        assert_eq!(csv_field("normal"), "\"normal\"");
        assert!(!csv_field("a\0b").contains('\0'));
    }

    #[test]
    fn csv_hides_uncertain_transcript_text_and_replacement_characters() {
        let safe = safe_candidate_text("원문", TranscriptQualityStatus::Uncertain);
        assert!(safe.contains("불확실"));
        let safe = safe_candidate_text("깨진 � 원문", TranscriptQualityStatus::Certain);
        assert!(!safe.contains('�'));
        assert!(safe.contains("불확실"));
        assert_eq!(safe_candidate_derived_text("오디오 근거 구간"), "오디오 근거 구간");
        assert!(safe_candidate_derived_text("깨진 � 제목").contains("불확실"));
    }

    #[test]
    fn first_stage_elapsed_is_preserved_and_later_measurements_saturate() {
        assert_eq!(accumulate_elapsed_ms(None, Some(17)), Some(17));
        assert_eq!(accumulate_elapsed_ms(Some(17), Some(23)), Some(40));
        assert_eq!(accumulate_elapsed_ms(Some(u64::MAX - 1), Some(23)), Some(u64::MAX));
    }

    /// Holds AppState and its temp data dir. Field order matters: Rust drops
    /// struct fields in declaration order, so `state` is declared first and
    /// drops before `_temp` is removed.
    struct TestAppState {
        state: AppState,
        _temp: tempfile::TempDir,
    }

    impl std::ops::Deref for TestAppState {
        type Target = AppState;

        fn deref(&self) -> &Self::Target {
            &self.state
        }
    }

    fn test_state_with_running_job(job_id: &str) -> TestAppState {
        let temp = tempfile::tempdir().expect("tempdir");
        let data_dir = temp.path().to_path_buf();
        let state = AppState::new(data_dir.clone(), data_dir).expect("AppState");
        let mut job = JobSnapshot::new(
            job_id.to_string(),
            SourceKind::Demo,
            "fixture".into(),
            Scenario::Normal,
            AnalysisMode::Full,
            None,
            None,
        );
        job.transition(JobStatus::Acquiring).expect("active status");
        *state.job.lock().expect("job lock") = Some(job);
        state.running.store(true, Ordering::SeqCst);
        TestAppState { state, _temp: temp }
    }

    fn test_candidate(id: &str, total_score: u8) -> Candidate {
        Candidate {
            id: id.into(), start_seconds: total_score as u32, end_seconds: total_score as u32 + 10,
            title: id.into(), summary: "요약".into(), transcript_excerpt: "기존 결과".into(),
            audio_score: total_score, dialogue_score: total_score, chat_score: None, total_score,
            decision: CandidateDecision::Pending, quality_status: "VALID".into(), quality_warnings: Vec::new(), selection_reasons: Vec::new(), uncertainty_reasons: Vec::new(), transcript_quality_status: TranscriptQualityStatus::Certain,
            transcript_quality_reasons: Vec::new(), context_start_seconds: 0.0, context_end_seconds: 10.0,
            context_transcript: Vec::new(),
        }
    }

    #[test]
    fn preserves_candidate_decision_when_count_round_trips_through_pool() {
        let mut job = JobSnapshot::new(
            "job-candidate-pool".into(), SourceKind::Demo, "fixture".into(), Scenario::Normal,
            AnalysisMode::Full, None, None,
        );
        job.candidate_pool = (0..30).map(|index| test_candidate(&format!("candidate-{index:02}"), 100 - index)).collect();
        job.candidates = job.candidate_pool[..8].to_vec();

        sync_candidate_decision(&mut job, "candidate-20", CandidateDecision::Accepted)
            .expect_err("hidden candidates are not directly reviewable");
        sync_candidate_decision(&mut job, "candidate-03", CandidateDecision::Accepted)
            .expect("visible candidate decision should update");
        job.candidates = candidates_for_count(&job, 8);
        assert_eq!(job.candidates.iter().find(|candidate| candidate.id == "candidate-03").unwrap().decision, CandidateDecision::Accepted);

        job.candidate_count = 30;
        job.candidates = candidates_for_count(&job, 30);
        assert_eq!(job.candidates.iter().find(|candidate| candidate.id == "candidate-03").unwrap().decision, CandidateDecision::Accepted);
        assert_eq!(job.candidates.iter().find(|candidate| candidate.id == "candidate-20").unwrap().decision, CandidateDecision::Pending);
    }

    #[test]
    fn mismatched_cancel_job_id_leaves_cancel_requested_false() {
        let active_id = Uuid::new_v4().to_string();
        let state = test_state_with_running_job(&active_id);
        let other_id = Uuid::new_v4().to_string();

        let err = arm_cancel_signal(&state, &other_id).expect_err("mismatch must fail");
        assert!(err.contains("다릅니다"), "unexpected error message: {err}");
        assert!(
            !state.cancel_requested.load(Ordering::SeqCst),
            "wrong job id must not arm global cancel_requested"
        );
    }

    #[test]
    fn matching_cancel_job_id_arms_cancel_before_disk_work() {
        let active_id = Uuid::new_v4().to_string();
        let state = test_state_with_running_job(&active_id);

        let started = Instant::now();
        arm_cancel_signal(&state, &active_id).expect("matching id must arm cancel");
        let elapsed = started.elapsed();

        assert!(
            state.cancel_requested.load(Ordering::SeqCst),
            "matching job id must set cancel_requested"
        );
        assert!(
            elapsed < Duration::from_millis(500),
            "arm_cancel_signal must stay in-memory and return quickly, took {elapsed:?}"
        );
    }

    #[test]
    fn recovery_fails_each_started_recognition_run_once_and_preserves_evidence() {
        let mut job = JobSnapshot::new(
            "job-recovery".into(), SourceKind::Demo, "fixture".into(), Scenario::Normal,
            AnalysisMode::Full, None, None,
        );
        job.status = JobStatus::ReviewReady;
        job.candidates.push(Candidate {
            id: "candidate-other".into(), start_seconds: 10, end_seconds: 20,
            title: "제목".into(), summary: "요약".into(), transcript_excerpt: "기존 결과".into(),
            audio_score: 80, dialogue_score: 70, chat_score: Some(60), total_score: 75,
            decision: CandidateDecision::Accepted, quality_status: "VALID".into(), quality_warnings: Vec::new(), selection_reasons: Vec::new(), uncertainty_reasons: Vec::new(), transcript_quality_status: TranscriptQualityStatus::Certain,
            transcript_quality_reasons: Vec::new(), context_start_seconds: 0.0, context_end_seconds: 30.0,
            context_transcript: Vec::new(),
        });
        let candidate_before = job.candidates[0].clone();
        job.recognition_runs.push(CandidateRecognitionRun {
            id: "run-started".into(), candidate_id: "candidate-other".into(), status: RecognitionRunStatus::Started,
            started_at: Utc::now(), completed_at: None, result_revision: 1, original_result: Some("old".into()),
            raw_result: None, display_result: None, failure_reason: None, backend_evidence: "CPU 시도; 실제 백엔드=whisper.cpp-cpu".into(),
        });
        job.recognition_runs.push(CandidateRecognitionRun {
            id: "run-failed".into(), candidate_id: "candidate-done".into(), status: RecognitionRunStatus::Failed,
            started_at: Utc::now(), completed_at: Some(Utc::now()), result_revision: 1, original_result: Some("done".into()),
            raw_result: None, display_result: None, failure_reason: Some("기존 실패".into()), backend_evidence: "기존 증거".into(),
        });
        let original_activity = job.activity.len();
        assert!(recover_started_recognition_runs(&mut job));
        assert_eq!(job.status, JobStatus::ReviewReady);
        let recovered = &job.recognition_runs[0];
        assert_eq!(recovered.status, RecognitionRunStatus::Failed);
        assert!(recovered.completed_at.is_some());
        assert_eq!(recovered.failure_reason.as_deref(), Some(APP_INTERRUPTED_RECOGNITION_REASON));
        assert_eq!(recovered.backend_evidence, "CPU 시도; 실제 백엔드=whisper.cpp-cpu");
        assert_eq!(job.candidates[0].id, candidate_before.id);
        assert_eq!(job.candidates[0].decision, candidate_before.decision);
        assert!(job.activity.len() > original_activity);
        assert!(!recover_started_recognition_runs(&mut job));
        assert_eq!(job.activity.len(), original_activity + 1);
    }

    #[test]
    fn bootstrap_migration_invalidates_unfinished_schema3_jobs_like_v04() {
        let mut job = JobSnapshot::new(
            Uuid::new_v4().to_string(),
            SourceKind::Demo,
            "fixture".into(),
            Scenario::Normal,
            AnalysisMode::Range,
            Some(10),
            Some(20),
        );
        job.schema_version = 3;
        job.completed_units = 7;
        job.status = JobStatus::Transcribing;

        let (migrated, changed) = migrate_snapshot(job);
        assert!(changed);
        assert_eq!(migrated.schema_version, 4);
        assert_eq!(migrated.analysis_mode, AnalysisMode::Full);
        assert_eq!(migrated.analysis_start_seconds, None);
        assert_eq!(migrated.analysis_end_seconds, None);
        assert_eq!(migrated.completed_units, 0);
        assert_eq!(migrated.total_units, 12);
        assert_eq!(migrated.status, JobStatus::Interrupted);
    }

    #[test]
    fn schema4_snapshot_migration_preserves_completed_units_and_uses_legacy_whisper_defaults() {
        let job = JobSnapshot::new(
            Uuid::new_v4().to_string(),
            SourceKind::Demo,
            "fixture".into(),
            Scenario::Normal,
            AnalysisMode::Full,
            None,
            None,
        );
        let mut json = serde_json::to_value(job).unwrap();
        json["schemaVersion"] = 4.into();
        json["completedUnits"] = 5.into();
        json.as_object_mut().unwrap().remove("whisper");
        let loaded: JobSnapshot = serde_json::from_value(json).unwrap();
        assert_eq!(loaded.whisper.device_mode, crate::whisper::WhisperDeviceMode::Cpu);
        assert_eq!(loaded.whisper.profile, crate::whisper::WhisperProfile::Balanced);
        assert_eq!(loaded.whisper.cpu_threads, None);

        let (migrated, changed) = migrate_snapshot(loaded);
        assert!(changed);
        assert_eq!(migrated.schema_version, 5);
        assert_eq!(migrated.completed_units, 5);
    }

    #[test]
    fn new_jobs_keep_auto_gpu_first_whisper_default() {
        let job = JobSnapshot::new(
            Uuid::new_v4().to_string(),
            SourceKind::Demo,
            "fixture".into(),
            Scenario::Normal,
            AnalysisMode::Full,
            None,
            None,
        );
        assert_eq!(job.whisper, WhisperSettings::default());
        assert_eq!(job.whisper.device_mode, crate::whisper::WhisperDeviceMode::Auto);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let headless_e2e = std::env::var_os("VOD_SCOUT_HEADLESS_E2E").is_some();
            let data_dir =
                if headless_e2e {
                    std::env::var_os("VOD_SCOUT_E2E_DATA_DIR")
                        .map(PathBuf::from)
                        .unwrap_or(app.path().app_local_data_dir().map_err(|error| {
                            format!("앱 데이터 경로를 만들 수 없습니다: {error}")
                        })?)
                } else {
                    app.path()
                        .app_local_data_dir()
                        .map_err(|error| format!("앱 데이터 경로를 만들 수 없습니다: {error}"))?
                };
            let resource_dir = app
                .path()
                .resource_dir()
                .map_err(|error| format!("앱 리소스 경로를 찾을 수 없습니다: {error}"))?;
            app.manage(Arc::new(AppState::new(data_dir, resource_dir)?));
            if !headless_e2e {
                app.get_webview_window("main")
                    .ok_or_else(|| "메인 창을 찾을 수 없습니다.".to_string())?
                    .show()
                    .map_err(|error| format!("메인 창을 표시할 수 없습니다: {error}"))?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            create_job,
            start_job,
            cancel_job,
            get_job_storage_info,
            get_queue,
            reorder_job,
            delete_job,
            list_jobs,
            delete_stored_job,
            delete_all_jobs,
            export_candidates_csv,
            prepare_candidate_preview,
            prepare_candidate_context_preview,
            rerun_candidate_transcription,
            set_candidate_count,
            set_candidate_decision,
            get_runtime_info
        ])
        .run(tauri::generate_context!())
        .expect("VOD Scout를 실행하지 못했습니다");
}
