mod acquisition;
mod domain;
mod integrity;
mod media;
mod storage;

use crate::domain::{
    AnalysisMode, Candidate, CandidateDecision, JobSnapshot, JobStatus, Scenario, SourceKind,
};
use crate::storage::JobStore;
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
    resource_dir: PathBuf,
    job: Mutex<Option<JobSnapshot>>,
    running: AtomicBool,
    cancel_requested: AtomicBool,
}

impl AppState {
    fn new(data_dir: PathBuf, resource_dir: PathBuf) -> Result<Self, String> {
        Ok(Self {
            store: JobStore::new(data_dir).map_err(|error| error.to_string())?,
            resource_dir,
            job: Mutex::new(None),
            running: AtomicBool::new(false),
            cancel_requested: AtomicBool::new(false),
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
}

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

#[tauri::command]
fn bootstrap(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<JobSnapshot>, String> {
    let mut loaded = match state.store.load_latest() {
        Ok(job) => job,
        Err(_) => return Ok(None),
    };

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
    }

    if loaded.status.is_active() {
        loaded.status = JobStatus::Interrupted;
        loaded.error_message = Some("이전 실행이 끝나기 전에 앱이 종료됐습니다.".into());
        loaded.error_detail = Some("마지막 완료 단위 다음부터 재개할 수 있습니다.".into());
        loaded.push_activity("recovery", "중단된 작업을 복원했습니다.");
        state
            .store
            .save(&loaded)
            .map_err(|error| error.to_string())?;
    }

    *state
        .job
        .lock()
        .map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())? = Some(loaded.clone());
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
    if state.running.load(Ordering::SeqCst) {
        return Err("실행 중인 작업을 먼저 취소해 주세요.".into());
    }
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
    state.store.save(&job).map_err(|error| error.to_string())?;
    if let Some(event) = job.activity.last() {
        state
            .store
            .append_event(&job.id, event)
            .map_err(|error| error.to_string())?;
    }
    *state
        .job
        .lock()
        .map_err(|_| "작업 상태 잠금이 손상됐습니다.".to_string())? = Some(job.clone());
    app.emit("job-updated", &job)
        .map_err(|error| error.to_string())?;
    Ok(job)
}

async fn run_worker(app: tauri::AppHandle, state: Arc<AppState>, job_id: String) {
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

    let mut last_heartbeat = Instant::now();
    let mut completed = false;
    let mut terminal_state_written = false;

    loop {
        if state.cancel_requested.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = mutate_job(&app, &state, |job| {
                job.transition(JobStatus::Cancelled)?;
                job.current_stage_label = "사용자가 취소함".into();
                job.error_message = None;
                job.error_detail = None;
                job.push_activity(
                    "cancel",
                    "작업을 안전하게 취소했습니다. 이어서 재개할 수 있습니다.",
                );
                Ok(())
            });
            terminal_state_written = true;
            break;
        }

        match tokio::time::timeout(Duration::from_millis(250), receiver.recv()).await {
            Ok(Some(CommandEvent::Stdout(bytes))) => {
                let line = String::from_utf8_lossy(&bytes).trim().to_string();
                let event = match serde_json::from_str::<WorkerEvent>(&line) {
                    Ok(event) => event,
                    Err(error) => {
                        let _ = child.kill();
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
                            let _ = child.kill();
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
                            job.candidates = candidates;
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
                let _ = child.kill();
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

    if !terminal_state_written && !completed {
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

    state.cancel_requested.store(false, Ordering::SeqCst);
    state.running.store(false, Ordering::SeqCst);
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
    arm_cancel_signal(&state, &job_id)?;
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
fn delete_job(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    job_id: String,
) -> Result<(), String> {
    if state.running.load(Ordering::SeqCst) {
        return Err("실행 중인 작업은 삭제할 수 없습니다. 먼저 안전하게 취소해 주세요.".into());
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
    state
        .store
        .delete_job(&job_id)
        .map_err(|error| error.to_string())?;
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
                csv_field(&candidate.title),
                csv_field(&candidate.transcript_excerpt),
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
    state
        .store
        .list_jobs()
        .map_err(|error| error.to_string())?
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
    if state.running.load(Ordering::SeqCst) {
        return Err("분석 중에는 저장된 작업을 삭제할 수 없습니다.".into());
    }
    Uuid::parse_str(&job_id).map_err(|_| "삭제할 작업 ID가 올바르지 않습니다.".to_string())?;
    state
        .store
        .delete_job(&job_id)
        .map_err(|error| error.to_string())?;
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
    state: State<'_, Arc<AppState>>,
    job_id: String,
    candidate_id: String,
) -> Result<media::PreviewMedia, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        media::prepare_candidate_preview(&state, &job_id, &candidate_id)
    })
    .await
    .map_err(|error| format!("미리보기 작업이 중단됐습니다: {error}"))?
}

#[tauri::command]
async fn prepare_candidate_context_preview(
    state: State<'_, Arc<AppState>>,
    job_id: String,
    candidate_id: String,
) -> Result<media::PreviewMedia, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        media::prepare_candidate_context_preview(&state, &job_id, &candidate_id)
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
    mutate_job(&app, &state, |job| {
        if job.id != job_id || job.status != JobStatus::ReviewReady {
            return Err("검토 준비가 끝난 현재 작업에서만 후보를 판정할 수 있습니다.".into());
        }
        let candidate = job
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == candidate_id)
            .ok_or_else(|| "후보를 찾을 수 없습니다.".to_string())?;
        candidate.decision = decision;
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
            delete_job,
            list_jobs,
            delete_stored_job,
            delete_all_jobs,
            export_candidates_csv,
            prepare_candidate_preview,
            prepare_candidate_context_preview,
            set_candidate_decision,
            get_runtime_info
        ])
        .run(tauri::generate_context!())
        .expect("VOD Scout를 실행하지 못했습니다");
}
