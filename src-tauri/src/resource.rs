use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceStage {
    FfmpegAudio,
    Whisper,
    ChatDecode,
    Preview,
    UiResponsiveness,
}

impl ResourceStage {
    pub fn label(self) -> &'static str {
        match self {
            Self::FfmpegAudio => "FFmpeg 오디오",
            Self::Whisper => "Whisper 음성 인식",
            Self::ChatDecode => "채팅 영역 디코딩",
            Self::Preview => "후보 미리보기",
            Self::UiResponsiveness => "UI 반응성",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResourcePolicyStatus {
    #[default]
    Unconfigured,
    Ok,
    Warning,
    HardLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSample {
    pub memory_bytes: Option<u64>,
    pub temp_bytes: Option<u64>,
    pub external_tool_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePolicy {
    #[serde(default)]
    pub warning_memory_bytes: Option<u64>,
    #[serde(default)]
    pub hard_memory_bytes: Option<u64>,
    #[serde(default)]
    pub warning_temp_bytes: Option<u64>,
    #[serde(default)]
    pub hard_temp_bytes: Option<u64>,
    #[serde(default)]
    pub warning_external_tool_count: Option<u32>,
    #[serde(default)]
    pub hard_external_tool_count: Option<u32>,
}

impl Default for ResourcePolicy {
    fn default() -> Self {
        Self {
            warning_memory_bytes: None,
            hard_memory_bytes: None,
            warning_temp_bytes: None,
            hard_temp_bytes: None,
            warning_external_tool_count: None,
            hard_external_tool_count: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceDecision {
    Ok,
    Warning(String),
    HardLimit(String),
}

impl ResourceDecision {
    pub fn status(&self) -> ResourcePolicyStatus {
        match self {
            Self::Ok => ResourcePolicyStatus::Ok,
            Self::Warning(_) => ResourcePolicyStatus::Warning,
            Self::HardLimit(_) => ResourcePolicyStatus::HardLimit,
        }
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Ok => None,
            Self::Warning(reason) | Self::HardLimit(reason) => Some(reason),
        }
    }
}

impl ResourcePolicy {
    pub fn is_configured(&self) -> bool {
        self.warning_memory_bytes.is_some()
            || self.hard_memory_bytes.is_some()
            || self.warning_temp_bytes.is_some()
            || self.hard_temp_bytes.is_some()
            || self.warning_external_tool_count.is_some()
            || self.hard_external_tool_count.is_some()
    }

    /// Evaluate only measured values. An unset threshold or unavailable sample
    /// never becomes an invented PASS/FAIL value.
    pub fn evaluate(&self, sample: &ResourceSample) -> ResourceDecision {
        if let (Some(limit), Some(value)) = (self.hard_memory_bytes, sample.memory_bytes) {
            if value > limit {
                return ResourceDecision::HardLimit(format!(
                    "메모리 사용량 {value}바이트가 강제 중단 기준 {limit}바이트를 초과했습니다."
                ));
            }
        }
        if let (Some(limit), Some(value)) = (self.hard_temp_bytes, sample.temp_bytes) {
            if value > limit {
                return ResourceDecision::HardLimit(format!(
                    "임시 파일 {value}바이트가 강제 중단 기준 {limit}바이트를 초과했습니다."
                ));
            }
        }
        if let (Some(limit), Some(value)) = (
            self.hard_external_tool_count,
            sample.external_tool_count,
        ) {
            if value > limit {
                return ResourceDecision::HardLimit(format!(
                    "외부 도구 {value}개가 강제 중단 기준 {limit}개를 초과했습니다."
                ));
            }
        }

        if let (Some(limit), Some(value)) = (self.warning_memory_bytes, sample.memory_bytes) {
            if value > limit {
                return ResourceDecision::Warning(format!(
                    "메모리 사용량 {value}바이트가 경고 기준 {limit}바이트를 초과했습니다."
                ));
            }
        }
        if let (Some(limit), Some(value)) = (self.warning_temp_bytes, sample.temp_bytes) {
            if value > limit {
                return ResourceDecision::Warning(format!(
                    "임시 파일 {value}바이트가 경고 기준 {limit}바이트를 초과했습니다."
                ));
            }
        }
        if let (Some(limit), Some(value)) = (
            self.warning_external_tool_count,
            sample.external_tool_count,
        ) {
            if value > limit {
                return ResourceDecision::Warning(format!(
                    "외부 도구 {value}개가 경고 기준 {limit}개를 초과했습니다."
                ));
            }
        }
        ResourceDecision::Ok
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageResourceMetric {
    pub stage: ResourceStage,
    pub elapsed_ms: Option<u64>,
    pub cpu_percent: Option<f32>,
    pub memory_bytes: Option<u64>,
    pub disk_bytes: Option<u64>,
    pub temp_bytes: Option<u64>,
    pub owned_child_processes: Option<u32>,
    #[serde(default)]
    pub unavailable_reasons: Vec<String>,
    #[serde(default)]
    pub policy_status: ResourcePolicyStatus,
    #[serde(default)]
    pub policy_reason: Option<String>,
}

impl StageResourceMetric {
    pub fn unavailable(stage: ResourceStage) -> Self {
        Self {
            stage,
            elapsed_ms: None,
            cpu_percent: None,
            memory_bytes: None,
            disk_bytes: None,
            temp_bytes: None,
            owned_child_processes: None,
            unavailable_reasons: vec!["이 단계의 측정값을 아직 수집하지 않았습니다.".into()],
            policy_status: ResourcePolicyStatus::Unconfigured,
            policy_reason: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceLimitFailure {
    pub stage: ResourceStage,
    pub reason: String,
    pub last_completed_units: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_has_no_unmeasured_thresholds() {
        let policy = ResourcePolicy::default();
        let decision = policy.evaluate(&ResourceSample {
            memory_bytes: None,
            temp_bytes: None,
            external_tool_count: None,
        });
        assert_eq!(decision, ResourceDecision::Ok);
        assert!(!policy.is_configured());
        assert!(policy.hard_memory_bytes.is_none());
        assert!(policy.hard_temp_bytes.is_none());
    }

    #[test]
    fn injected_hard_limit_is_deterministic_and_warning_is_non_terminal() {
        let policy = ResourcePolicy {
            warning_memory_bytes: Some(10),
            hard_memory_bytes: Some(20),
            ..Default::default()
        };
        assert_eq!(
            policy.evaluate(&ResourceSample { memory_bytes: Some(11), ..Default::default() }),
            ResourceDecision::Warning("메모리 사용량 11바이트가 경고 기준 10바이트를 초과했습니다.".into())
        );
        assert!(matches!(
            policy.evaluate(&ResourceSample { memory_bytes: Some(21), ..Default::default() }),
            ResourceDecision::HardLimit(_)
        ));
    }

    #[test]
    fn stage_metric_serializes_unavailable_values_as_null() {
        let value = serde_json::to_value(StageResourceMetric::unavailable(ResourceStage::UiResponsiveness)).unwrap();
        assert!(value["elapsedMs"].is_null());
        assert!(value["cpuPercent"].is_null());
        assert!(value["memoryBytes"].is_null());
        assert!(value["unavailableReasons"].as_array().is_some_and(|items| !items.is_empty()));
    }
}
