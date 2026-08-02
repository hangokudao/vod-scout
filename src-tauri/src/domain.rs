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
pub struct JobSnapshot {
    pub schema_version: u8,
    pub id: String,
    pub source_kind: SourceKind,
    pub source_label: String,
    #[serde(default)]
    pub acquired_media_path: Option<String>,
    #[serde(default)]
    pub download_percent: Option<u8>,
    pub scenario: Scenario,
    #[serde(default)]
    pub analysis_mode: AnalysisMode,
    #[serde(default)]
    pub analysis_start_seconds: Option<u32>,
    #[serde(default)]
    pub analysis_end_seconds: Option<u32>,
    pub status: JobStatus,
    pub completed_units: u32,
    pub total_units: u32,
    #[serde(default)]
    pub media_duration_seconds: Option<f64>,
    pub current_stage_label: String,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub error_message: Option<String>,
    pub error_detail: Option<String>,
    pub candidates: Vec<Candidate>,
    pub activity: Vec<ActivityEvent>,
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
            schema_version: 4,
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
}
