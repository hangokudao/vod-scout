use crate::captions::{CaptionSource, VerificationState};
use crate::whisper::{WhisperRuntimeState, WhisperSettings};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobStatus {
    Created,
    Acquiring,
    Probing,
    ExtractingAudio,
    Transcribing,
    AudioSignals,
    ChatSignals,
    Fusing,
    Ranking,
    Cancelling,
    Cancelled,
    Interrupted,
    Failed,
    NeedsInput,
    ReviewReady,
}

impl JobStatus {
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Acquiring
                | Self::Probing
                | Self::ExtractingAudio
                | Self::Transcribing
                | Self::AudioSignals
                | Self::ChatSignals
                | Self::Fusing
                | Self::Ranking
                | Self::Cancelling
        )
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        if self == next {
            return true;
        }

        if next == Self::Cancelling {
            return self.is_active() && self != Self::Cancelling;
        }

        if matches!(next, Self::Failed | Self::Interrupted) {
            return self.is_active();
        }

        matches!(
            (self, next),
            (Self::Created, Self::Acquiring)
                | (Self::Acquiring, Self::Probing)
                | (Self::Probing, Self::ExtractingAudio)
                | (Self::Probing, Self::Transcribing)
                | (Self::ExtractingAudio, Self::Transcribing)
                | (Self::Transcribing, Self::AudioSignals)
                | (Self::AudioSignals, Self::ChatSignals)
                | (Self::AudioSignals, Self::Fusing)
                | (Self::ChatSignals, Self::Fusing)
                | (Self::Fusing, Self::Ranking)
                | (Self::Ranking, Self::ReviewReady)
                | (Self::Cancelling, Self::Cancelled)
                | (Self::Cancelled, Self::Acquiring)
                | (Self::Cancelled, Self::Probing)
                | (Self::Cancelled, Self::ExtractingAudio)
                | (Self::Cancelled, Self::Transcribing)
                | (Self::Cancelled, Self::AudioSignals)
                | (Self::Cancelled, Self::ChatSignals)
                | (Self::Cancelled, Self::Fusing)
                | (Self::Cancelled, Self::Ranking)
                | (Self::Interrupted, Self::Acquiring)
                | (Self::Interrupted, Self::Probing)
                | (Self::Interrupted, Self::ExtractingAudio)
                | (Self::Interrupted, Self::Transcribing)
                | (Self::Interrupted, Self::AudioSignals)
                | (Self::Interrupted, Self::ChatSignals)
                | (Self::Interrupted, Self::Fusing)
                | (Self::Interrupted, Self::Ranking)
                | (Self::Failed, Self::Acquiring)
                | (Self::Failed, Self::Probing)
                | (Self::Failed, Self::ExtractingAudio)
                | (Self::Failed, Self::Transcribing)
                | (Self::Failed, Self::AudioSignals)
                | (Self::Failed, Self::ChatSignals)
                | (Self::Failed, Self::Fusing)
                | (Self::Failed, Self::Ranking)
                | (Self::NeedsInput, Self::Acquiring)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Local,
    Youtube,
    Demo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AnalysisMode {
    #[default]
    Full,
    Quick,
    Range,
}

impl AnalysisMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Quick => "빠른 분석",
            Self::Range => "구간 지정",
            Self::Full => "전체 정밀 분석",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scenario {
    Normal,
    Fail,
    Crash,
    Hang,
    Malformed,
}

impl Scenario {
    pub fn as_arg(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Fail => "fail",
            Self::Crash => "crash",
            Self::Hang => "hang",
            Self::Malformed => "malformed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CandidateDecision {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextTranscriptEntry {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub id: String,
    pub start_seconds: u32,
    pub end_seconds: u32,
    pub title: String,
    pub summary: String,
    pub transcript_excerpt: String,
    pub audio_score: u8,
    pub dialogue_score: u8,
    pub chat_score: Option<u8>,
    pub total_score: u8,
    pub decision: CandidateDecision,
    #[serde(default)]
    pub context_start_seconds: f64,
    #[serde(default)]
    pub context_end_seconds: f64,
    #[serde(default)]
    pub context_transcript: Vec<ContextTranscriptEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub sequence: u64,
    pub timestamp: DateTime<Utc>,
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionProvenanceSummary {
    pub original_file: String,
    #[serde(default)]
    pub language: Option<String>,
    pub track_id: String,
    pub sha256: String,
    pub revision: String,
    pub verification_state: VerificationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionDiagnosticSummary {
    pub kind: String,
    pub interval_index: Option<usize>,
    pub start_seconds: Option<f64>,
    pub end_seconds: Option<f64>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionSummary {
    pub source: Option<CaptionSource>,
    #[serde(default)]
    pub language: Option<String>,
    pub quality: String,
    pub fallback_intervals: u32,
    #[serde(default)]
    pub local_whisper_fallback: bool,
    #[serde(default)]
    pub diagnostics: Vec<CaptionDiagnosticSummary>,
    pub provenance: Option<CaptionProvenanceSummary>,
}

#[derive(Debug, Clone)]
pub struct JobSnapshot {
    pub schema_version: u8,
    pub id: String,
    pub source_kind: SourceKind,
    pub source_label: String,
    pub acquired_media_path: Option<String>,
    pub download_percent: Option<u8>,
    pub scenario: Scenario,
    pub analysis_mode: AnalysisMode,
    pub analysis_start_seconds: Option<u32>,
    pub analysis_end_seconds: Option<u32>,
    pub status: JobStatus,
    pub completed_units: u32,
    pub total_units: u32,
    pub media_duration_seconds: Option<f64>,
    pub current_stage_label: String,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub error_message: Option<String>,
    pub error_detail: Option<String>,
    pub candidates: Vec<Candidate>,
    pub activity: Vec<ActivityEvent>,
    pub captions: Option<CaptionSummary>,
    pub whisper: WhisperSettings,
    pub whisper_runtime: WhisperRuntimeState,
}

fn legacy_whisper_settings() -> WhisperSettings {
    WhisperSettings {
        device_mode: crate::whisper::WhisperDeviceMode::Cpu,
        profile: crate::whisper::WhisperProfile::Balanced,
        cpu_threads: None,
    }
}

impl<'de> serde::Deserialize<'de> for JobSnapshot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SnapshotFields {
            schema_version: u8,
            id: String,
            source_kind: SourceKind,
            source_label: String,
            #[serde(default)]
            acquired_media_path: Option<String>,
            #[serde(default)]
            download_percent: Option<u8>,
            scenario: Scenario,
            #[serde(default)]
            analysis_mode: AnalysisMode,
            #[serde(default)]
            analysis_start_seconds: Option<u32>,
            #[serde(default)]
            analysis_end_seconds: Option<u32>,
            status: JobStatus,
            completed_units: u32,
            total_units: u32,
            #[serde(default)]
            media_duration_seconds: Option<f64>,
            current_stage_label: String,
            last_heartbeat_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            error_message: Option<String>,
            error_detail: Option<String>,
            candidates: Vec<Candidate>,
            activity: Vec<ActivityEvent>,
            #[serde(default)]
            captions: Option<CaptionSummary>,
            #[serde(default = "legacy_whisper_settings")]
            whisper: WhisperSettings,
            #[serde(default)]
            whisper_runtime: WhisperRuntimeState,
        }
        let fields = SnapshotFields::deserialize(deserializer)?;
        Ok(Self {
            schema_version: fields.schema_version,
            id: fields.id,
            source_kind: fields.source_kind,
            source_label: fields.source_label,
            acquired_media_path: fields.acquired_media_path,
            download_percent: fields.download_percent,
            scenario: fields.scenario,
            analysis_mode: fields.analysis_mode,
            analysis_start_seconds: fields.analysis_start_seconds,
            analysis_end_seconds: fields.analysis_end_seconds,
            status: fields.status,
            completed_units: fields.completed_units,
            total_units: fields.total_units,
            media_duration_seconds: fields.media_duration_seconds,
            current_stage_label: fields.current_stage_label,
            last_heartbeat_at: fields.last_heartbeat_at,
            created_at: fields.created_at,
            updated_at: fields.updated_at,
            error_message: fields.error_message,
            error_detail: fields.error_detail,
            candidates: fields.candidates,
            activity: fields.activity,
            captions: fields.captions,
            whisper: fields.whisper,
            whisper_runtime: fields.whisper_runtime,
        })
    }
}

impl serde::Serialize for JobSnapshot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct SnapshotFields<'a> {
            schema_version: u8,
            id: &'a str,
            source_kind: SourceKind,
            source_label: &'a str,
            acquired_media_path: &'a Option<String>,
            download_percent: &'a Option<u8>,
            scenario: Scenario,
            analysis_mode: AnalysisMode,
            analysis_start_seconds: &'a Option<u32>,
            analysis_end_seconds: &'a Option<u32>,
            status: JobStatus,
            completed_units: u32,
            total_units: u32,
            media_duration_seconds: &'a Option<f64>,
            current_stage_label: &'a str,
            last_heartbeat_at: &'a Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            error_message: &'a Option<String>,
            error_detail: &'a Option<String>,
            candidates: &'a Vec<Candidate>,
            activity: &'a Vec<ActivityEvent>,
            captions: &'a Option<CaptionSummary>,
            whisper: &'a WhisperSettings,
            whisper_runtime: &'a WhisperRuntimeState,
        }
        SnapshotFields {
            schema_version: self.schema_version,
            id: &self.id,
            source_kind: self.source_kind,
            source_label: &self.source_label,
            acquired_media_path: &self.acquired_media_path,
            download_percent: &self.download_percent,
            scenario: self.scenario,
            analysis_mode: self.analysis_mode,
            analysis_start_seconds: &self.analysis_start_seconds,
            analysis_end_seconds: &self.analysis_end_seconds,
            status: self.status,
            completed_units: self.completed_units,
            total_units: self.total_units,
            media_duration_seconds: &self.media_duration_seconds,
            current_stage_label: &self.current_stage_label,
            last_heartbeat_at: &self.last_heartbeat_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
            error_message: &self.error_message,
            error_detail: &self.error_detail,
            candidates: &self.candidates,
            activity: &self.activity,
            captions: &self.captions,
            whisper: &self.whisper,
            whisper_runtime: &self.whisper_runtime,
        }
        .serialize(serializer)
    }
}

impl JobSnapshot {
    pub fn new(
        id: String,
        source_kind: SourceKind,
        source_label: String,
        scenario: Scenario,
        analysis_mode: AnalysisMode,
        analysis_start_seconds: Option<u32>,
        analysis_end_seconds: Option<u32>,
    ) -> Self {
        let now = Utc::now();
        let mut job = Self {
            schema_version: 5,
            id,
            source_kind,
            source_label,
            acquired_media_path: None,
            download_percent: None,
            scenario,
            analysis_mode,
            analysis_start_seconds,
            analysis_end_seconds,
            status: JobStatus::Created,
            completed_units: 0,
            total_units: 12,
            media_duration_seconds: None,
            current_stage_label: "실행 대기".into(),
            last_heartbeat_at: None,
            created_at: now,
            updated_at: now,
            error_message: None,
            error_detail: None,
            candidates: Vec::new(),
            activity: Vec::new(),
            captions: None,
            whisper: WhisperSettings::default(),
            whisper_runtime: WhisperRuntimeState::default(),
        };
        job.push_activity("job", "새 분석 작업을 만들었습니다.");
        job
    }

    pub fn transition(&mut self, next: JobStatus) -> Result<(), String> {
        if !self.status.can_transition_to(next) {
            return Err(format!(
                "허용되지 않은 상태 전이입니다: {:?} → {:?}",
                self.status, next
            ));
        }
        self.status = next;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn apply_progress(
        &mut self,
        unit: u32,
        status: JobStatus,
        stage_label: String,
        message: String,
    ) -> Result<(), String> {
        if unit != self.completed_units.saturating_add(1) || unit > self.total_units {
            return Err(format!(
                "진행 단위가 연속적이지 않습니다: 현재 {}, 수신 {}",
                self.completed_units, unit
            ));
        }
        self.transition(status)?;
        self.completed_units = unit;
        self.current_stage_label = stage_label;
        self.last_heartbeat_at = Some(Utc::now());
        self.error_message = None;
        self.error_detail = None;
        self.push_activity("progress", &message);
        Ok(())
    }

    pub fn push_activity(&mut self, kind: &str, message: &str) {
        let sequence = self
            .activity
            .last()
            .map(|event| event.sequence + 1)
            .unwrap_or(1);
        self.activity.push(ActivityEvent {
            sequence,
            timestamp: Utc::now(),
            kind: kind.into(),
            message: message.into(),
        });
        if self.activity.len() > 80 {
            let remove_count = self.activity.len() - 80;
            self.activity.drain(0..remove_count);
        }
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job() -> JobSnapshot {
        JobSnapshot::new(
            "test".into(),
            SourceKind::Demo,
            "fixture".into(),
            Scenario::Normal,
            AnalysisMode::Full,
            None,
            None,
        )
    }

    #[test]
    fn rejects_skipping_directly_to_review() {
        let mut job = job();
        assert!(job.transition(JobStatus::ReviewReady).is_err());
        assert_eq!(job.status, JobStatus::Created);
    }

    #[test]
    fn accepts_expected_pipeline_order() {
        let mut job = job();
        let states = [
            JobStatus::Acquiring,
            JobStatus::Probing,
            JobStatus::ExtractingAudio,
            JobStatus::Transcribing,
            JobStatus::AudioSignals,
            JobStatus::ChatSignals,
            JobStatus::Fusing,
            JobStatus::Ranking,
            JobStatus::ReviewReady,
        ];
        for state in states {
            job.transition(state).expect("expected transition");
        }
        assert_eq!(job.status, JobStatus::ReviewReady);
    }

    #[test]
    fn progress_must_be_monotonic() {
        let mut job = job();
        job.apply_progress(1, JobStatus::Acquiring, "입력 준비".into(), "완료".into())
            .unwrap();
        assert!(job
            .apply_progress(3, JobStatus::Probing, "미디어 확인".into(), "건너뜀".into())
            .is_err());
        assert_eq!(job.completed_units, 1);
    }

    #[test]
    fn candidate_context_fields_round_trip_and_old_snapshots_still_deserialize() {
        let candidate = Candidate {
            id: "candidate-1".into(),
            start_seconds: 20,
            end_seconds: 40,
            title: "title".into(),
            summary: "summary".into(),
            transcript_excerpt: "excerpt".into(),
            audio_score: 80,
            dialogue_score: 70,
            chat_score: None,
            total_score: 75,
            decision: CandidateDecision::Pending,
            context_start_seconds: 5.0,
            context_end_seconds: 55.0,
            context_transcript: vec![ContextTranscriptEntry {
                start_seconds: 21.5,
                end_seconds: 24.0,
                text: "timestamped".into(),
            }],
        };
        let encoded = serde_json::to_value(&candidate).unwrap();
        assert_eq!(encoded["contextStartSeconds"], 5.0);
        assert_eq!(encoded["contextEndSeconds"], 55.0);
        assert_eq!(encoded["contextTranscript"][0]["startSeconds"], 21.5);
        assert_eq!(
            serde_json::from_value::<Candidate>(encoded).unwrap().id,
            "candidate-1"
        );

        let old = serde_json::json!({
            "id": "legacy",
            "startSeconds": 1,
            "endSeconds": 2,
            "title": "title",
            "summary": "summary",
            "transcriptExcerpt": "excerpt",
            "audioScore": 1,
            "dialogueScore": 2,
            "chatScore": null,
            "totalScore": 2,
            "decision": "PENDING"
        });
        let legacy = serde_json::from_value::<Candidate>(old).unwrap();
        assert_eq!(legacy.context_start_seconds, 0.0);
        assert!(legacy.context_transcript.is_empty());
    }
}
