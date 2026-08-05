use super::{media, mutate_job, AppState};
use crate::domain::JobStatus;
use crate::integrity::verify_runtime_bundle;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;
use url::Url;

use crate::media::terminate_child_tree;
#[cfg(windows)]
use crate::media::{KillOnCloseJob, CREATE_NO_WINDOW};

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
                "YouTube 영상 정보를 확인하고 최대 720p로 다운로드합니다.",
            );
            Ok(())
        })
        .map_err(AcquisitionError::Message)?;

        let outcome = run_yt_dlp(app, state, &tools, &source_url, &download_dir, &log_dir)?;
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

fn run_yt_dlp<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &Arc<AppState>,
    tools: &DownloadTools,
    source_url: &str,
    download_dir: &Path,
    log_dir: &Path,
) -> Result<DownloadOutcome, AcquisitionError> {
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
            "bv*[height<=720]+ba/b[height<=720]/b",
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
    if !path.is_file() {
        return Ok(None);
    }
    let checkpoint: AcquisitionCheckpoint = serde_json::from_slice(&fs::read(path)?)?;
    if checkpoint.schema_version != 1 || checkpoint.source_url != source_url {
        return Ok(None);
    }
    let media = PathBuf::from(&checkpoint.media_path);
    if media.is_file() && fs::metadata(media)?.len() > 0 {
        Ok(Some(checkpoint))
    } else {
        Ok(None)
    }
}

fn save_checkpoint(
    path: &Path,
    checkpoint: &AcquisitionCheckpoint,
) -> Result<(), AcquisitionError> {
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(checkpoint)?)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)?;
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
}
