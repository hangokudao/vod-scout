use serde::{Deserialize, Serialize};

pub const MODEL_NAME: &str = "ggml-base";
pub const MIN_CPU_THREADS: u16 = 1;
pub const MAX_CPU_THREADS: u16 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WhisperDeviceMode {
    #[default]
    Auto,
    Gpu,
    Cpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WhisperProfile {
    Fast,
    #[default]
    Balanced,
    Accurate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WhisperAttemptStatus {
    Pending,
    Started,
    Completed,
    Failed,
}

impl Default for WhisperAttemptStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperSettings {
    #[serde(default)]
    pub device_mode: WhisperDeviceMode,
    #[serde(default)]
    pub profile: WhisperProfile,
    #[serde(default)]
    pub cpu_threads: Option<u16>,
}

/// Runtime state shown to the user and persisted with the job snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum WhisperRuntimeStatus {
    #[default]
    Untested,
    Testing,
    Gpu,
    Cpu,
    CpuFallback,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperRuntimeState {
    #[serde(default)]
    pub status: WhisperRuntimeStatus,
    #[serde(default)]
    pub unit_index: Option<u32>,
    #[serde(default)]
    pub effective_cpu_threads: Option<u16>,
    #[serde(default)]
    pub gpu_failure_reason: Option<String>,
}

impl Default for WhisperRuntimeState {
    fn default() -> Self {
        Self {
            status: WhisperRuntimeStatus::Untested,
            unit_index: None,
            effective_cpu_threads: None,
            gpu_failure_reason: None,
        }
    }
}

impl Default for WhisperSettings {
    fn default() -> Self {
        Self {
            device_mode: WhisperDeviceMode::Auto,
            profile: WhisperProfile::Balanced,
            cpu_threads: None,
        }
    }
}

impl WhisperSettings {
    pub fn normalized(mut self) -> Self {
        self.cpu_threads = normalize_cpu_threads(self.cpu_threads);
        self
    }
}

pub fn normalize_cpu_threads(value: Option<u16>) -> Option<u16> {
    value.map(|threads| threads.clamp(MIN_CPU_THREADS, MAX_CPU_THREADS))
}

pub fn effective_cpu_threads(settings: &WhisperSettings, available: usize) -> usize {
    let available_bound = available.max(MIN_CPU_THREADS as usize).min(MAX_CPU_THREADS as usize);
    settings
        .cpu_threads
        .map(|threads| {
            normalize_cpu_threads(Some(threads))
                .unwrap_or(MIN_CPU_THREADS)
                .min(available_bound as u16) as usize
        })
        .unwrap_or_else(|| available.saturating_sub(1).clamp(1, MAX_CPU_THREADS as usize))
}

pub fn profile_args(profile: WhisperProfile) -> &'static [&'static str] {
    match profile {
        WhisperProfile::Fast => &["-bs", "1", "-bo", "1"],
        WhisperProfile::Balanced => &["-bs", "5", "-bo", "5"],
        WhisperProfile::Accurate => &["-bs", "8", "-bo", "8"],
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperAttempt {
    #[serde(default)]
    pub status: WhisperAttemptStatus,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub failure_reason: Option<String>,
}

impl Default for WhisperAttempt {
    fn default() -> Self {
        Self {
            status: WhisperAttemptStatus::Pending,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            failure_reason: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperUnitState {
    pub chunk_index: u32,
    pub fallback_index: u32,
    pub device: WhisperDeviceMode,
    pub model: String,
    pub profile: WhisperProfile,
    pub cpu_threads: Option<u16>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub gpu_failure_reason: Option<String>,
    #[serde(default)]
    pub gpu: WhisperAttempt,
    #[serde(default)]
    pub cpu_fallback: WhisperAttempt,
}

impl WhisperUnitState {
    pub fn legacy_cpu(chunk_index: u32, fallback_index: u32) -> Self {
        let settings = WhisperSettings::default();
        Self {
            chunk_index,
            fallback_index,
            device: WhisperDeviceMode::Cpu,
            model: MODEL_NAME.into(),
            profile: settings.profile,
            cpu_threads: settings.cpu_threads,
            duration_ms: None,
            gpu_failure_reason: None,
            gpu: WhisperAttempt::default(),
            cpu_fallback: WhisperAttempt::default(),
        }
    }
}

pub fn should_try_gpu(settings: &WhisperSettings, unit: &WhisperUnitState) -> bool {
    !matches!(settings.device_mode, WhisperDeviceMode::Cpu)
        && unit.gpu.status != WhisperAttemptStatus::Failed
        && unit.cpu_fallback.status != WhisperAttemptStatus::Failed
}

pub fn should_try_cpu(unit: &WhisperUnitState) -> bool {
    unit.cpu_fallback.status != WhisperAttemptStatus::Failed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_settings_deserialize_to_safe_cpu_compatible_defaults() {
        let settings: WhisperSettings = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(settings, WhisperSettings::default());
    }

    #[test]
    fn explicit_threads_are_clamped_and_auto_is_bounded() {
        assert_eq!(normalize_cpu_threads(Some(0)), Some(1));
        assert_eq!(normalize_cpu_threads(Some(99)), Some(32));
        assert_eq!(normalize_cpu_threads(None), None);
        assert_eq!(effective_cpu_threads(&WhisperSettings::default(), 128), 32);
        assert_eq!(
            effective_cpu_threads(
                &WhisperSettings {
                    cpu_threads: Some(32),
                    ..Default::default()
                },
                4
            ),
            4
        );
        assert_eq!(
            effective_cpu_threads(
                &WhisperSettings {
                    cpu_threads: Some(0),
                    ..Default::default()
                },
                0
            ),
            1
        );
    }

    #[test]
    fn profiles_use_minimal_documented_decoding_flags() {
        assert_eq!(profile_args(WhisperProfile::Fast), &["-bs", "1", "-bo", "1"]);
        assert_eq!(profile_args(WhisperProfile::Balanced), &["-bs", "5", "-bo", "5"]);
        assert_eq!(profile_args(WhisperProfile::Accurate), &["-bs", "8", "-bo", "8"]);
    }

    #[test]
    fn recorded_gpu_or_cpu_failures_close_the_retry_gate() {
        let settings = WhisperSettings::default();
        let mut unit = WhisperUnitState::legacy_cpu(0, 0);
        assert!(should_try_gpu(&settings, &unit));
        unit.gpu.status = WhisperAttemptStatus::Failed;
        assert!(!should_try_gpu(&settings, &unit));
        assert!(should_try_cpu(&unit));
        unit.cpu_fallback.status = WhisperAttemptStatus::Failed;
        assert!(!should_try_cpu(&unit));
        assert!(!should_try_gpu(&settings, &unit));
    }

    #[test]
    fn whisper_attempt_state_machine_allows_one_gpu_then_one_cpu_fallback() {
        let auto = WhisperSettings::default();
        let cpu = WhisperSettings {
            device_mode: WhisperDeviceMode::Cpu,
            ..auto.clone()
        };
        let mut unit = WhisperUnitState::legacy_cpu(3, 0);
        assert!(should_try_gpu(&auto, &unit));
        assert!(should_try_cpu(&unit));

        unit.gpu.status = WhisperAttemptStatus::Failed;
        assert!(!should_try_gpu(&auto, &unit));
        assert!(should_try_cpu(&unit));

        unit.cpu_fallback.status = WhisperAttemptStatus::Failed;
        assert!(!should_try_cpu(&unit));
        assert!(!should_try_gpu(&auto, &unit));
        assert!(!should_try_gpu(&cpu, &WhisperUnitState::legacy_cpu(3, 1)));
    }
}
