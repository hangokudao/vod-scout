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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueIndex {
    pub schema_version: u8,
    pub ordered_job_ids: Vec<String>,
    pub transition_state: QueueTransitionState,
    pub execution_mode: QueueExecutionMode,
}

impl Default for QueueIndex {
    fn default() -> Self {
        Self {
            schema_version: QUEUE_SCHEMA_VERSION,
            ordered_job_ids: Vec::new(),
            transition_state: QueueTransitionState::Idle,
            execution_mode: QueueExecutionMode::Sequential,
        }
    }
}

impl QueueIndex {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != QUEUE_SCHEMA_VERSION {
            return Err(format!("지원하지 않는 작업 대기열 스키마입니다: {}", self.schema_version));
        }
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
        let current = self.path();
        let previous = self.previous_path();
        for path in [&current, &previous] {
            if !path.is_file() { continue; }
            match fs::read(path).and_then(|bytes| serde_json::from_slice::<QueueIndex>(&bytes).map_err(|error| std::io::Error::new(ErrorKind::InvalidData, error))) {
                Ok(queue) => {
                    queue.validate()?;
                    return Ok(Some(queue));
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
        if let Some(mut queue) = self.load()? {
            if queue.reconcile(ids)? {
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
        let queue = QueueIndex { schema_version: 1, ordered_job_ids: vec![id()], transition_state: QueueTransitionState::Running, execution_mode: QueueExecutionMode::Sequential };
        let value = serde_json::to_value(queue).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 4);
        assert!(value.get("orderedJobIds").is_some());
        assert!(value.get("jobs").is_none());
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
