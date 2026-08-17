use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const QUEUE_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueueTransitionState {
    #[default]
    Idle,
    Running,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueueExecutionMode {
    #[default]
    Sequential,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueueEvaluationStatus {
    /// No same-input performance or resource comparison has been measured.
    #[default]
    UnmeasuredPending,
    /// A fail-closed transition permanently keeps this queue sequential.
    SequentialFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueEvaluation {
    pub status: QueueEvaluationStatus,
    pub effective_execution_mode: QueueExecutionMode,
    pub max_concurrency: u8,
    pub parallel_available: bool,
    #[serde(default)]
    pub sequential_fallback_reason: Option<String>,
}

impl Default for QueueEvaluation {
    fn default() -> Self {
        Self {
            status: QueueEvaluationStatus::UnmeasuredPending,
            effective_execution_mode: QueueExecutionMode::Sequential,
            max_concurrency: 1,
            parallel_available: false,
            sequential_fallback_reason: None,
        }
    }
}

impl QueueEvaluation {
    /// Permanently disable parallel execution for this queue until a future
    /// explicit migration replaces the record. There is intentionally no
    /// automatic or user-facing enable transition.
    pub fn disable_parallel_with_fallback(&mut self, reason: impl Into<String>) -> Result<(), String> {
        let reason = reason.into().trim().to_string();
        if reason.is_empty() {
            return Err("순차 처리 전환 사유를 비워 둘 수 없습니다.".into());
        }
        if self.status == QueueEvaluationStatus::SequentialFallback {
            return Err("순차 처리 전환은 이미 영구 적용됐습니다.".into());
        }
        self.status = QueueEvaluationStatus::SequentialFallback;
        self.effective_execution_mode = QueueExecutionMode::Sequential;
        self.max_concurrency = 1;
        self.parallel_available = false;
        self.sequential_fallback_reason = Some(reason);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.effective_execution_mode != QueueExecutionMode::Sequential {
            return Err("측정되지 않은 대기열은 순차 처리만 사용할 수 있습니다.".into());
        }
        if self.max_concurrency != 1 {
            return Err("대기열의 최대 동시 실행 수는 1이어야 합니다.".into());
        }
        if self.parallel_available {
            return Err("병렬 처리는 실제 측정 전까지 사용할 수 없습니다.".into());
        }
        match (self.status, self.sequential_fallback_reason.as_deref()) {
            (QueueEvaluationStatus::UnmeasuredPending, None) => Ok(()),
            (QueueEvaluationStatus::SequentialFallback, Some(reason)) if !reason.trim().is_empty() => Ok(()),
            (QueueEvaluationStatus::UnmeasuredPending, Some(_)) => Err("미측정 대기 상태에는 순차 전환 사유를 기록할 수 없습니다.".into()),
            (QueueEvaluationStatus::SequentialFallback, None) => Err("순차 처리 전환 사유가 없습니다.".into()),
            _ => Err("순차 처리 전환 사유가 비어 있습니다.".into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueIndex {
    pub schema_version: u8,
    pub ordered_job_ids: Vec<String>,
    pub transition_state: QueueTransitionState,
    pub execution_mode: QueueExecutionMode,
    #[serde(default)]
    pub evaluation: QueueEvaluation,
}

impl Default for QueueIndex {
    fn default() -> Self {
        Self {
            schema_version: QUEUE_SCHEMA_VERSION,
            ordered_job_ids: Vec::new(),
            transition_state: QueueTransitionState::Idle,
            execution_mode: QueueExecutionMode::Sequential,
            evaluation: QueueEvaluation::default(),
        }
    }
}

impl QueueIndex {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != QUEUE_SCHEMA_VERSION {
            return Err(format!("지원하지 않는 작업 대기열 스키마입니다: {}", self.schema_version));
        }
        if self.execution_mode != QueueExecutionMode::Sequential {
            return Err("지원하지 않는 작업 대기열 실행 모드입니다.".into());
        }
        self.evaluation.validate()?;
        let mut seen = std::collections::HashSet::new();
        for id in &self.ordered_job_ids {
            validate_job_id(id)?;
            if !seen.insert(id) {
                return Err("작업 대기열에 중복된 작업 ID가 있습니다.".into());
            }
        }
        Ok(())
    }

    pub fn contains(&self, id: &str) -> bool {
        self.ordered_job_ids.iter().any(|item| item == id)
    }

    pub fn add(&mut self, id: String) -> Result<(), String> {
        validate_job_id(&id)?;
        if !self.contains(&id) {
            self.ordered_job_ids.push(id);
        }
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        validate_job_id(id)?;
        self.ordered_job_ids.retain(|item| item != id);
        Ok(())
    }

    /// Remove references to missing jobs and append newly stored jobs in a
    /// stable order. Existing queue order is preserved.
    pub fn reconcile(&mut self, ids: impl IntoIterator<Item = String>) -> Result<bool, String> {
        self.validate()?;
        let mut stored_ids = ids.into_iter().collect::<Vec<_>>();
        for id in &stored_ids {
            validate_job_id(id)?;
        }
        stored_ids.sort();
        stored_ids.dedup();

        let before = self.ordered_job_ids.clone();
        self.ordered_job_ids.retain(|id| stored_ids.binary_search(id).is_ok());
        for id in stored_ids {
            self.add(id)?;
        }
        Ok(self.ordered_job_ids != before)
    }

    pub fn move_job(&mut self, id: &str, new_index: usize) -> Result<(), String> {
        validate_job_id(id)?;
        let Some(old_index) = self.ordered_job_ids.iter().position(|item| item == id) else {
            return Err("대기열에서 작업을 찾을 수 없습니다.".into());
        };
        let bounded = new_index.min(self.ordered_job_ids.len().saturating_sub(1));
        let value = self.ordered_job_ids.remove(old_index);
        self.ordered_job_ids.insert(bounded, value);
        Ok(())
    }

    pub fn disable_parallel_with_fallback(&mut self, reason: impl Into<String>) -> Result<(), String> {
        self.evaluation.disable_parallel_with_fallback(reason)?;
        self.execution_mode = QueueExecutionMode::Sequential;
        Ok(())
    }
}

pub fn validate_job_id(id: &str) -> Result<(), String> {
    Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| "작업 ID가 UUID 형식이 아닙니다.".into())
}

#[derive(Debug)]
pub struct QueueStore {
    root: PathBuf,
}

impl QueueStore {
    pub fn new(root: PathBuf) -> Result<Self, std::io::Error> {
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn path(&self) -> PathBuf { self.root.join("queue.json") }

    pub fn previous_path(&self) -> PathBuf { self.root.join("queue.prev.json") }

    pub fn load(&self) -> Result<Option<QueueIndex>, String> {
        Ok(self.load_with_metadata()?.map(|(queue, _)| queue))
    }

    fn load_with_metadata(&self) -> Result<Option<(QueueIndex, bool)>, String> {
        let current = self.path();
        let previous = self.previous_path();
        for path in [&current, &previous] {
            if !path.is_file() { continue; }
            match fs::read(path).and_then(|bytes| {
                let has_evaluation = serde_json::from_slice::<serde_json::Value>(&bytes)
                    .ok()
                    .and_then(|value| value.get("evaluation").cloned())
                    .is_some();
                serde_json::from_slice::<QueueIndex>(&bytes)
                    .map(|queue| (queue, has_evaluation))
                    .map_err(|error| std::io::Error::new(ErrorKind::InvalidData, error))
            }) {
                Ok((queue, has_evaluation)) => {
                    queue.validate()?;
                    return Ok(Some((queue, has_evaluation)));
                }
                Err(error) if error.kind() == ErrorKind::NotFound => continue,
                Err(_) => continue,
            }
        }
        Ok(None)
    }

    pub fn save(&self, queue: &QueueIndex) -> Result<(), String> {
        queue.validate()?;
        let path = self.path();
        let temporary = self.root.join("queue.tmp.json");
        let previous = self.previous_path();
        let bytes = serde_json::to_vec_pretty(queue).map_err(|error| error.to_string())?;
        fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
        if path.exists() {
            if previous.exists() { fs::remove_file(&previous).map_err(|error| error.to_string())?; }
            fs::rename(&path, &previous).map_err(|error| error.to_string())?;
        }
        fs::rename(&temporary, &path).map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn load_or_reconcile(&self, ids: impl IntoIterator<Item = String>) -> Result<QueueIndex, String> {
        let ids = ids.into_iter().collect::<Vec<_>>();
        if let Some((mut queue, had_evaluation)) = self.load_with_metadata()? {
            if queue.reconcile(ids)? {
                self.save(&queue)?;
            } else if !had_evaluation {
                self.save(&queue)?;
            }
            return Ok(queue);
        }
        let mut queue = QueueIndex::default();
        for id in ids { queue.add(id)?; }
        queue.ordered_job_ids.sort();
        self.save(&queue)?;
        Ok(queue)
    }

    pub fn load_or_create(&self, ids: impl IntoIterator<Item = String>) -> Result<QueueIndex, String> {
        self.load_or_reconcile(ids)
    }
}

/// A process-local lease backed by an atomic create-new file. A second app
/// instance fails closed instead of touching the first instance's queue.
#[derive(Debug)]
pub struct InstanceLease { path: PathBuf }

impl InstanceLease {
    pub fn acquire(root: &Path) -> Result<Self, String> {
        let path = root.join("instance.lock");
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                let stale = fs::read_to_string(&path)
                    .ok()
                    .and_then(|value| value.strip_prefix("pid=")?.trim().parse::<u32>().ok())
                    .is_some_and(|pid| !process_is_alive(pid));
                if !stale { return Err("이미 실행 중인 VOD Scout 인스턴스가 있습니다.".into()); }
                let _ = fs::remove_file(&path);
                OpenOptions::new().write(true).create_new(true).open(&path)
                    .map_err(|_| "이미 실행 중인 VOD Scout 인스턴스가 있습니다.".to_string())?
            }
            Err(error) => return Err(format!("앱 실행권을 얻지 못했습니다: {error}")),
        };
        use std::io::Write;
        writeln!(file, "pid={}", std::process::id()).map_err(|error| error.to_string())?;
        Ok(Self { path })
    }
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM) }
}

#[cfg(windows)]
fn process_is_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() { return false; }
        let mut exit_code = 0;
        let result = GetExitCodeProcess(handle, &mut exit_code);
        CloseHandle(handle);
        result != 0 && exit_code == STILL_ACTIVE as u32
    }
}

impl Drop for InstanceLease {
    fn drop(&mut self) { let _ = fs::remove_file(&self.path); }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> String { Uuid::new_v4().to_string() }

    #[test]
    fn queue_round_trip_contains_only_index_fields() {
        let queue = QueueIndex { schema_version: 1, ordered_job_ids: vec![id()], transition_state: QueueTransitionState::Running, execution_mode: QueueExecutionMode::Sequential, evaluation: QueueEvaluation::default() };
        let value = serde_json::to_value(queue).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 5);
        assert!(value.get("orderedJobIds").is_some());
        assert!(value.get("jobs").is_none());
        assert_eq!(value["evaluation"]["maxConcurrency"], 1);
    }

    #[test]
    fn legacy_queue_defaults_to_unmeasured_sequential_policy() {
        let legacy = format!(
            r#"{{"schemaVersion":1,"orderedJobIds":["{}"],"transitionState":"IDLE","executionMode":"SEQUENTIAL"}}"#,
            id()
        );
        let queue: QueueIndex = serde_json::from_str(&legacy).unwrap();
        assert_eq!(queue.evaluation, QueueEvaluation::default());
        assert_eq!(queue.evaluation.status, QueueEvaluationStatus::UnmeasuredPending);
        assert!(!queue.evaluation.parallel_available);
        assert_eq!(queue.evaluation.max_concurrency, 1);
    }

    #[test]
    fn queue_policy_persists_and_legacy_file_is_upgraded() {
        let temp = tempfile::tempdir().unwrap();
        let store = QueueStore::new(temp.path().to_path_buf()).unwrap();
        let job_id = id();
        fs::write(
            store.path(),
            format!(
                r#"{{"schemaVersion":1,"orderedJobIds":["{}"],"transitionState":"IDLE","executionMode":"SEQUENTIAL"}}"#,
                job_id
            ),
        )
        .unwrap();
        let queue = store.load_or_reconcile(vec![job_id]).unwrap();
        assert_eq!(queue.evaluation.max_concurrency, 1);
        let persisted: serde_json::Value = serde_json::from_slice(&fs::read(store.path()).unwrap()).unwrap();
        assert_eq!(persisted["evaluation"]["status"], "UNMEASURED_PENDING");

        let reloaded = store.load().unwrap().unwrap();
        assert_eq!(reloaded.evaluation, queue.evaluation);
    }

    #[test]
    fn parallel_is_unavailable_and_fallback_transition_is_one_way() {
        let mut queue = QueueIndex::default();
        assert_eq!(queue.evaluation.max_concurrency, 1);
        assert!(!queue.evaluation.parallel_available);
        queue.disable_parallel_with_fallback("동일 입력의 GPU·자원 측정이 없어 안전하게 순차 처리합니다.").unwrap();
        assert_eq!(queue.evaluation.status, QueueEvaluationStatus::SequentialFallback);
        assert_eq!(queue.evaluation.sequential_fallback_reason.as_deref(), Some("동일 입력의 GPU·자원 측정이 없어 안전하게 순차 처리합니다."));
        assert!(!queue.evaluation.parallel_available);
        assert_eq!(queue.evaluation.max_concurrency, 1);
        assert!(queue.disable_parallel_with_fallback("다른 사유").is_err());
        assert!(QueueEvaluation::default().disable_parallel_with_fallback("   ").is_err());
    }

    #[test]
    fn fallback_record_stays_disabled_after_round_trip() {
        let mut queue = QueueIndex::default();
        queue.disable_parallel_with_fallback("실제 입력 비교가 아직 없어 순차 처리로 고정했습니다.").unwrap();
        let restored: QueueIndex = serde_json::from_value(serde_json::to_value(queue).unwrap()).unwrap();
        assert_eq!(restored.evaluation.status, QueueEvaluationStatus::SequentialFallback);
        assert!(!restored.evaluation.parallel_available);
        assert_eq!(restored.evaluation.max_concurrency, 1);
        assert!(restored.evaluation.sequential_fallback_reason.is_some());
    }

    #[test]
    fn atomic_save_recovers_previous_normal_generation() {
        let temp = tempfile::tempdir().unwrap();
        let store = QueueStore::new(temp.path().to_path_buf()).unwrap();
        let first_id = id();
        let second_id = id();
        let mut queue = QueueIndex::default();
        queue.add(first_id.clone()).unwrap();
        store.save(&queue).unwrap();
        queue.add(second_id).unwrap();
        store.save(&queue).unwrap();
        fs::write(store.path(), b"broken").unwrap();
        let recovered = store.load().unwrap().unwrap();
        assert_eq!(recovered.ordered_job_ids, vec![first_id]);
        assert!(store.previous_path().is_file());
    }

    #[test]
    fn reorder_and_remove_validate_uuid_targets() {
        let mut queue = QueueIndex::default();
        let first = id();
        let second = id();
        queue.add(first.clone()).unwrap();
        queue.add(second.clone()).unwrap();
        queue.move_job(&second, 0).unwrap();
        assert_eq!(queue.ordered_job_ids[0], second);
        queue.remove(&first).unwrap();
        assert!(queue.remove("../outside").is_err());
    }

    #[test]
    fn reconciliation_removes_missing_references_and_appends_untracked_jobs() {
        let missing = id();
        let first = id();
        let untracked = id();
        let mut queue = QueueIndex::default();
        queue.ordered_job_ids = vec![first.clone(), missing];

        assert!(queue.reconcile(vec![untracked.clone(), first.clone()]).unwrap());
        assert_eq!(queue.ordered_job_ids, vec![first, untracked]);
    }

    #[test]
    fn reconciliation_preserves_queue_order_and_rejects_duplicate_references() {
        let first = id();
        let second = id();
        let mut queue = QueueIndex::default();
        queue.ordered_job_ids = vec![second.clone(), first.clone()];
        assert!(queue.reconcile(vec![first.clone(), second.clone()]).is_ok());
        assert_eq!(queue.ordered_job_ids, vec![second, first.clone()]);

        queue.ordered_job_ids.push(first);
        assert!(queue.reconcile(Vec::<String>::new()).is_err());
    }

    #[test]
    fn second_instance_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let first = InstanceLease::acquire(temp.path()).unwrap();
        assert!(InstanceLease::acquire(temp.path()).is_err());
        drop(first);
        assert!(InstanceLease::acquire(temp.path()).is_ok());
    }
}
