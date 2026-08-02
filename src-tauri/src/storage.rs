use crate::domain::{ActivityEvent, JobSnapshot};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("작업 저장소를 읽거나 쓸 수 없습니다: {0}")]
    Io(#[from] std::io::Error),
    #[error("저장된 작업 데이터 형식이 올바르지 않습니다: {0}")]
    Json(#[from] serde_json::Error),
    #[error("복구할 작업이 없습니다")]
    Missing,
}

#[derive(Debug)]
pub struct JobStore {
    root: PathBuf,
}

impl JobStore {
    pub fn new(root: PathBuf) -> Result<Self, StoreError> {
        fs::create_dir_all(root.join("jobs"))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn job_dir(&self, id: &str) -> PathBuf {
        self.root.join("jobs").join(id)
    }

    pub fn job_size_bytes(&self, id: &str) -> Result<u64, StoreError> {
        validate_job_id(id)?;
        fn directory_size(path: &Path) -> Result<u64, std::io::Error> {
            let mut total = 0u64;
            if !path.exists() {
                return Ok(0);
            }
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let metadata = fs::symlink_metadata(entry.path())?;
                if metadata.file_type().is_symlink() {
                    continue;
                }
                total = total.saturating_add(if metadata.is_dir() {
                    directory_size(&entry.path())?
                } else {
                    metadata.len()
                });
            }
            Ok(total)
        }

        Ok(directory_size(&self.job_dir(id))?)
    }

    pub fn delete_job(&self, id: &str) -> Result<(), StoreError> {
        validate_job_id(id)?;
        let directory = self.job_dir(id);
        if directory
            .symlink_metadata()
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(StoreError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "심볼릭 링크 작업 폴더는 삭제하지 않습니다",
            )));
        }
        if directory.exists() {
            fs::remove_dir_all(directory)?;
        }
        let current = self.root.join("current-job.json");
        let points_to_deleted = current
            .is_file()
            .then(|| fs::read(&current).ok())
            .flatten()
            .and_then(|bytes| serde_json::from_slice::<String>(&bytes).ok())
            .is_some_and(|current_id| current_id == id);
        if points_to_deleted {
            fs::remove_file(current)?;
        }
        let temporary = self.root.join("current-job.tmp");
        if temporary.exists() {
            fs::remove_file(temporary)?;
        }
        Ok(())
    }

    pub fn list_jobs(&self) -> Result<Vec<JobSnapshot>, StoreError> {
        let jobs = self.root.join("jobs");
        let mut snapshots = Vec::new();
        for entry in fs::read_dir(jobs)? {
            let entry = entry?;
            let metadata = fs::symlink_metadata(entry.path())?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().to_string();
            if validate_job_id(&id).is_err() {
                continue;
            }
            let snapshot = entry.path().join("snapshot.json");
            let previous = entry.path().join("snapshot.prev.json");
            if let Ok(job) =
                Self::read_snapshot(&snapshot).or_else(|_| Self::read_snapshot(&previous))
            {
                if job.id == id {
                    snapshots.push(job);
                }
            }
        }
        snapshots.sort_by_key(|item| std::cmp::Reverse(item.updated_at));
        Ok(snapshots)
    }

    pub fn delete_all_jobs(&self) -> Result<(), StoreError> {
        for entry in fs::read_dir(self.root.join("jobs"))? {
            let entry = entry?;
            let id = entry.file_name().to_string_lossy().to_string();
            if validate_job_id(&id).is_ok() {
                self.delete_job(&id)?;
            }
        }
        for pointer in ["current-job.json", "current-job.tmp"] {
            let path = self.root.join(pointer);
            if path.is_file() {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    pub fn save(&self, job: &JobSnapshot) -> Result<(), StoreError> {
        let dir = self.job_dir(&job.id);
        fs::create_dir_all(&dir)?;
        let snapshot = dir.join("snapshot.json");
        let previous = dir.join("snapshot.prev.json");
        let temporary = dir.join("snapshot.tmp");
        let bytes = serde_json::to_vec_pretty(job)?;

        fs::write(&temporary, bytes)?;
        if snapshot.exists() {
            if previous.exists() {
                fs::remove_file(&previous)?;
            }
            fs::rename(&snapshot, &previous)?;
        }
        fs::rename(&temporary, &snapshot)?;
        self.write_current_id(&job.id)?;
        Ok(())
    }

    pub fn append_event(&self, job_id: &str, event: &ActivityEvent) -> Result<(), StoreError> {
        let dir = self.job_dir(job_id);
        fs::create_dir_all(&dir)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("events.jsonl"))?;
        serde_json::to_writer(&mut file, event)?;
        file.write_all(b"\n")?;
        file.flush()?;
        Ok(())
    }

    fn write_current_id(&self, id: &str) -> Result<(), StoreError> {
        let current = self.root.join("current-job.json");
        let temporary = self.root.join("current-job.tmp");
        fs::write(&temporary, serde_json::to_vec(&id)?)?;
        if current.exists() {
            fs::remove_file(&current)?;
        }
        fs::rename(temporary, current)?;
        Ok(())
    }

    pub fn load_latest(&self) -> Result<JobSnapshot, StoreError> {
        let id: String = serde_json::from_slice(&fs::read(self.root.join("current-job.json"))?)?;
        let dir = self.job_dir(&id);
        let snapshot = dir.join("snapshot.json");
        match Self::read_snapshot(&snapshot) {
            Ok(job) => Ok(job),
            Err(_) => Self::read_snapshot(&dir.join("snapshot.prev.json")),
        }
    }

    fn read_snapshot(path: &Path) -> Result<JobSnapshot, StoreError> {
        if !path.exists() {
            return Err(StoreError::Missing);
        }
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }
}

fn validate_job_id(id: &str) -> Result<(), StoreError> {
    Uuid::parse_str(id).map(|_| ()).map_err(|_| {
        StoreError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "작업 ID가 UUID 형식이 아닙니다",
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AnalysisMode, Scenario, SourceKind};

    #[test]
    fn recovers_previous_snapshot_when_latest_is_corrupt() {
        let temp = tempfile::tempdir().unwrap();
        let store = JobStore::new(temp.path().to_path_buf()).unwrap();
        let mut job = JobSnapshot::new(
            Uuid::new_v4().to_string(),
            SourceKind::Demo,
            "fixture".into(),
            Scenario::Normal,
            AnalysisMode::Full,
            None,
            None,
        );
        let id = job.id.clone();
        store.save(&job).unwrap();
        job.current_stage_label = "두 번째 저장".into();
        store.save(&job).unwrap();
        fs::write(store.job_dir(&id).join("snapshot.json"), b"{broken").unwrap();

        let recovered = store.load_latest().unwrap();
        assert_eq!(recovered.current_stage_label, "실행 대기");
    }

    #[test]
    fn measures_and_deletes_only_the_requested_job_directory() {
        let temp = tempfile::tempdir().unwrap();
        let store = JobStore::new(temp.path().to_path_buf()).unwrap();
        let job = JobSnapshot::new(
            Uuid::new_v4().to_string(),
            SourceKind::Demo,
            "fixture".into(),
            Scenario::Normal,
            AnalysisMode::Full,
            None,
            None,
        );
        let id = job.id.clone();
        let other_id = Uuid::new_v4().to_string();
        store.save(&job).unwrap();
        fs::write(store.job_dir(&id).join("media.bin"), vec![0u8; 128]).unwrap();
        fs::create_dir_all(store.job_dir(&other_id)).unwrap();
        fs::write(store.job_dir(&other_id).join("keep.bin"), b"keep").unwrap();

        assert!(store.job_size_bytes(&id).unwrap() >= 128);
        store.delete_job(&id).unwrap();
        assert!(!store.job_dir(&id).exists());
        assert!(store.job_dir(&other_id).join("keep.bin").is_file());
        assert!(store.load_latest().is_err());
    }

    #[test]
    fn refuses_non_uuid_storage_targets() {
        let temp = tempfile::tempdir().unwrap();
        let store = JobStore::new(temp.path().to_path_buf()).unwrap();
        let outside = temp.path().join("outside.txt");
        fs::write(&outside, b"keep").unwrap();
        assert!(store.delete_job("../outside.txt").is_err());
        assert!(store.job_size_bytes("../outside.txt").is_err());
        assert_eq!(fs::read(&outside).unwrap(), b"keep");
    }

    #[test]
    fn delete_all_removes_corrupt_orphan_job_directories() {
        let temp = tempfile::tempdir().unwrap();
        let store = JobStore::new(temp.path().to_path_buf()).unwrap();
        let orphan_id = Uuid::new_v4().to_string();
        fs::create_dir_all(store.job_dir(&orphan_id)).unwrap();
        fs::write(store.job_dir(&orphan_id).join("snapshot.json"), b"{broken").unwrap();
        fs::write(
            store.job_dir(&orphan_id).join("private-media.mp4"),
            b"private",
        )
        .unwrap();
        store.delete_all_jobs().unwrap();
        assert!(!store.job_dir(&orphan_id).exists());
    }
}
