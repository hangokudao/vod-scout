use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const EMBEDDED_MANIFEST: &str = include_str!("../resources/media-tools/manifest.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IntegrityManifest {
    schema_version: u8,
    runtime_hashes: HashMap<String, String>,
}

fn embedded_manifest() -> Result<IntegrityManifest, String> {
    let manifest: IntegrityManifest = serde_json::from_str(EMBEDDED_MANIFEST)
        .map_err(|error| format!("내장 도구 무결성 정보가 올바르지 않습니다: {error}"))?;
    if manifest.schema_version != 5 {
        return Err("내장 도구 무결성 정보 버전이 올바르지 않습니다.".into());
    }
    Ok(manifest)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let file = File::open(path)
        .map_err(|error| format!("{} 파일을 열 수 없습니다: {error}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| format!("{} 해시 계산 실패: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn runtime_relative_paths(root: &Path) -> Result<HashSet<String>, String> {
    let mut result = HashSet::new();
    let mut pending = ["ffmpeg", "whisper", "models", "yt-dlp", "deno"]
        .into_iter()
        .map(|directory| root.join(directory))
        .collect::<Vec<_>>();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("{} 폴더를 읽을 수 없습니다: {error}", directory.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let metadata = fs::symlink_metadata(entry.path()).map_err(|error| error.to_string())?;
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "runtime 링크 파일을 허용하지 않습니다: {}",
                    entry.path().display()
                ));
            }
            if metadata.is_dir() {
                pending.push(entry.path());
            } else if metadata.is_file() {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|error| error.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                result.insert(relative);
            }
        }
    }
    Ok(result)
}

fn verify_runtime_bundle_uncached(root: &Path) -> Result<(), String> {
    let manifest = embedded_manifest()?;
    let actual_paths = runtime_relative_paths(root)?;
    let expected_paths = manifest
        .runtime_hashes
        .keys()
        .cloned()
        .collect::<HashSet<_>>();
    if actual_paths != expected_paths {
        return Err(
            "내장 runtime 파일 목록이 빌드 manifest와 다릅니다. 앱을 다시 설치해 주세요.".into(),
        );
    }
    for (relative_path, expected) in &manifest.runtime_hashes {
        let path = root.join(relative_path);
        let actual = sha256_file(&path)?;
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(format!(
                "{} 무결성 검증에 실패했습니다. 앱을 다시 설치해 주세요.",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
    }
    Ok(())
}

pub(crate) fn verify_runtime_bundle(root: &Path) -> Result<(), String> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Result<(), String>>>> = OnceLock::new();
    let key = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(result) = cache
        .lock()
        .map_err(|_| "runtime 검증 캐시 잠금 오류")?
        .get(&key)
        .cloned()
    {
        return result;
    }
    let result = verify_runtime_bundle_uncached(root);
    cache
        .lock()
        .map_err(|_| "runtime 검증 캐시 잠금 오류")?
        .insert(key, result.clone());
    result
}

pub(crate) fn runtime_hashes() -> Result<HashMap<String, String>, String> {
    Ok(embedded_manifest()?.runtime_hashes)
}

pub(crate) fn source_fingerprint(path: &Path) -> Result<(String, u64), String> {
    const SAMPLE_BYTES: u64 = 1024 * 1024;
    let mut file = File::open(path)
        .map_err(|error| format!("입력 파일 fingerprint를 열 수 없습니다: {error}"))?;
    let length = file
        .metadata()
        .map_err(|error| format!("입력 파일 정보를 읽을 수 없습니다: {error}"))?
        .len();
    let mut hasher = Sha256::new();
    hasher.update(length.to_le_bytes());

    let first_length = length.min(SAMPLE_BYTES) as usize;
    let mut buffer = vec![0_u8; first_length];
    file.read_exact(&mut buffer)
        .map_err(|error| format!("입력 파일 앞부분 fingerprint 실패: {error}"))?;
    hasher.update(&buffer);

    if length > SAMPLE_BYTES {
        let last_length = length.min(SAMPLE_BYTES);
        file.seek(SeekFrom::Start(length - last_length))
            .map_err(|error| format!("입력 파일 끝부분 탐색 실패: {error}"))?;
        buffer.resize(last_length as usize, 0);
        file.read_exact(&mut buffer)
            .map_err(|error| format!("입력 파일 끝부분 fingerprint 실패: {error}"))?;
        hasher.update(&buffer);
    }
    Ok((format!("{:x}", hasher.finalize()), length))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn rejects_a_modified_runtime_file() {
        let directory = tempdir().unwrap();
        for relative in embedded_manifest().unwrap().runtime_hashes.keys() {
            let path = directory.path().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"modified").unwrap();
        }
        let error = verify_runtime_bundle_uncached(directory.path()).unwrap_err();
        assert!(error.contains("무결성"));
    }

    #[test]
    fn source_fingerprint_changes_with_content() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("source.mp4");
        fs::write(&path, b"one").unwrap();
        let first = source_fingerprint(&path).unwrap();
        fs::write(&path, b"two").unwrap();
        let second = source_fingerprint(&path).unwrap();
        assert_ne!(first.0, second.0);
        assert_eq!(second.1, 3);
    }
}
