//! YouTube caption selection, validation, provenance, and local fallback planning.
//!
//! This module deliberately does not assign a timing tolerance. A real video and
//! a human-checked reference are required before an offset can become verified.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use url::Url;

pub const CAPTION_SCHEMA_VERSION: u8 = 2;
const MAX_LANGUAGE_TAG_LENGTH: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptionSource {
    Creator,
    Automatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationState {
    Unverified,
    Verified,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CaptionDiagnosticKind {
    StartAfterEnd,
    OutOfRange,
    Overlap,
    Duplicate,
    EmptyText,
    GapObserved,
    OffsetUnverified,
    QualityWarning,
    ProvenanceInvalid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionDiagnostic {
    pub kind: CaptionDiagnosticKind,
    pub interval_index: Option<usize>,
    pub start_seconds: Option<f64>,
    pub end_seconds: Option<f64>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionInterval {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionTrack {
    pub track_id: String,
    pub language: String,
    pub source: CaptionSource,
    pub url: Option<String>,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionProvenance {
    pub schema_version: u8,
    pub source_url: String,
    pub source: CaptionSource,
    pub language: String,
    pub track_id: String,
    pub revision: String,
    pub original_file: String,
    pub sha256: String,
    pub verification_state: VerificationState,
    pub diagnostics: Vec<CaptionDiagnostic>,
    #[serde(skip)]
    pub content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionValidation {
    pub intervals: Vec<CaptionInterval>,
    pub diagnostics: Vec<CaptionDiagnostic>,
    pub verification_state: VerificationState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackInterval {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionPlan {
    pub trusted: Vec<CaptionInterval>,
    pub fallback: Vec<FallbackInterval>,
    pub diagnostics: Vec<CaptionDiagnostic>,
    pub verification_state: VerificationState,
    pub full_whisper: bool,
}

/// Select Korean automatic captions first, then creator Korean captions.
/// The metadata shape is the one emitted by yt-dlp (`subtitles` and
/// `automatic_captions`). Translations, other languages, and live chat are
/// rejected before the priority comparison.
pub fn select_track(info: &Value) -> Option<CaptionTrack> {
    select_tracks(info).into_iter().next()
}

/// Return every Korean automatic track before every creator track. The
/// acquisition stage advances to creator tracks only after every automatic
/// track contains no usable intervals.
pub fn select_tracks(info: &Value) -> Vec<CaptionTrack> {
    let mut tracks = select_track_group(
        info.get("automatic_captions"),
        CaptionSource::Automatic,
    );
    tracks.extend(select_track_group(
        info.get("subtitles"),
        CaptionSource::Creator,
    ));
    tracks
}

fn select_track_group(value: Option<&Value>, source: CaptionSource) -> Vec<CaptionTrack> {
    let Some(object) = value.and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut candidates = object
        .iter()
        .filter_map(|(language, formats)| {
            if !is_korean_language(language) || is_rejected_track(language, formats) {
                return None;
            }
            let format = formats.as_array()?.iter().find(|format| {
                format
                    .get("url")
                    .and_then(Value::as_str)
                    .is_some_and(|url| !url.trim().is_empty() && !is_rejected_url(url))
            });
            let format = format?;
            let track_id = format
                .get("vss_id")
                .or_else(|| format.get("name"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim();
            if is_rejected_text(track_id) {
                return None;
            }
            let url = format
                .get("url")
                .and_then(Value::as_str)
                .map(str::to_string);
            let revision = format
                .get("vss_id")
                .or_else(|| format.get("name"))
                .and_then(Value::as_str)
                .unwrap_or(track_id)
                .to_string();
            Some(CaptionTrack {
                track_id: track_id.to_string(),
                language: language.to_string(),
                source,
                url,
                revision,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.language
            .cmp(&right.language)
            .then_with(|| left.track_id.cmp(&right.track_id))
    });
    candidates
}

fn is_korean_language(language: &str) -> bool {
    is_safe_korean_language_tag(language)
}

/// Caption language metadata is untrusted input because it becomes part of a
/// filename and a child-process argument.
pub fn is_safe_korean_language_tag(language: &str) -> bool {
    if language.is_empty()
        || language.len() > MAX_LANGUAGE_TAG_LENGTH
        || !language
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return false;
    }
    let components = language.split(['-', '_']).collect::<Vec<_>>();
    components
        .first()
        .is_some_and(|component| component.eq_ignore_ascii_case("ko"))
        && components
            .iter()
            .skip(1)
            .all(|component| !component.is_empty())
}

fn is_rejected_track(language: &str, formats: &Value) -> bool {
    is_rejected_text(language) || is_rejected_text(&formats.to_string())
}

fn is_rejected_url(value: &str) -> bool {
    Url::parse(value)
        .map(|url| url.query_pairs().any(|(key, _)| key.eq_ignore_ascii_case("tlang")))
        .unwrap_or_else(|_| value.split('?').nth(1).is_some_and(|query| {
            query.split('&').any(|pair| {
                pair.split('=').next().is_some_and(|key| key.eq_ignore_ascii_case("tlang"))
            })
        }))
}

fn is_rejected_text(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "live_chat",
        "live chat",
        "translation",
        "translated",
        "translate",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

/// Parse WebVTT or SRT into original-video absolute seconds.
pub fn parse_caption_text(text: &str) -> Vec<CaptionInterval> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut parsed = Vec::new();
    for block in normalized.split("\n\n") {
        let mut lines = block.lines().map(str::trim);
        let first = lines.next().unwrap_or_default();
        let range = if first.contains(" --> ") {
            first
        } else {
            lines.next().unwrap_or_default()
        };
        let Some((start, end)) = range.split_once(" --> ") else {
            continue;
        };
        let body = lines.collect::<Vec<_>>();
        let Some(start_seconds) = parse_caption_time(start) else {
            continue;
        };
        let Some(end_seconds) = parse_caption_time(end.split_whitespace().next().unwrap_or(end))
        else {
            continue;
        };
        parsed.push((
            start_seconds,
            end_seconds,
            body.iter().any(|line| line.contains("<c>")),
            body.into_iter()
                .map(strip_vtt_tags)
                .map(|line| normalize_caption_whitespace(&line))
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>(),
        ));
    }

    // YouTube automatic VTT uses two rolling rows plus 0.01-second clearing
    // cues. A normal creator VTT can also contain two independent lines, so
    // enable rolling-row cleanup only when the document actually contains the
    // short clearing-cue pattern.
    let short_cue_count = parsed
        .iter()
        .filter(|(start, end, _, _)| end - start <= 0.011)
        .count();
    let rolling_vtt = short_cue_count >= 2
        && parsed
            .iter()
            .any(|(_, _, has_internal_timing, _)| *has_internal_timing);
    let mut previous_row: Option<String> = None;
    let mut intervals = Vec::new();
    for (start_seconds, end_seconds, has_internal_timing, mut rows) in parsed {
        if rolling_vtt {
            if end_seconds - start_seconds <= 0.011 {
                continue;
            }
            let last_raw_row = rows.last().cloned();
            if let Some(previous) = previous_row.as_ref() {
                if rows.first().is_some_and(|row| row == previous) {
                    rows.remove(0);
                } else if rows.len() == 1 {
                    if let Some(suffix) = rows[0].strip_prefix(previous) {
                        rows[0] = suffix.trim().to_string();
                    }
                }
            }
            if let Some(last) = last_raw_row.or(previous_row.clone()) {
                previous_row = Some(last);
            }
        }
        let text = normalize_caption_whitespace(&rows.join(" "));
        if text.is_empty() {
            continue;
        }
        // Inline <timestamp><c>text</c> spans belong to the outer cue. Tags are
        // removed only after their text has been retained, so the original
        // outer time range and every timed word remain represented.
        let _ = has_internal_timing;
        intervals.push(CaptionInterval {
            start_seconds,
            end_seconds,
            text,
        });
    }
    intervals
}

fn normalize_caption_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_vtt_tags(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;
    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }
    output.trim().to_string()
}

fn parse_caption_time(value: &str) -> Option<f64> {
    let value = value.trim().replace(',', ".");
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() == 2 {
        let minutes = parts[0].parse::<f64>().ok()?;
        let seconds = parts[1].parse::<f64>().ok()?;
        return finite_nonnegative(minutes * 60.0 + seconds);
    }
    if parts.len() == 3 {
        let hours = parts[0].parse::<f64>().ok()?;
        let minutes = parts[1].parse::<f64>().ok()?;
        let seconds = parts[2].parse::<f64>().ok()?;
        return finite_nonnegative(hours * 3600.0 + minutes * 60.0 + seconds);
    }
    None
}

fn finite_nonnegative(value: f64) -> Option<f64> {
    value
        .is_finite()
        .then_some(value)
        .filter(|value| *value >= 0.0)
}

/// Validate structural timing without inventing an offset tolerance or a gap
/// pass threshold. Every observed gap is recorded for later human review.
pub fn validate_intervals(
    intervals: Vec<CaptionInterval>,
    duration_seconds: f64,
    verification_state: VerificationState,
) -> CaptionValidation {
    let mut diagnostics = Vec::new();
    for (index, interval) in intervals.iter().enumerate() {
        if !interval.start_seconds.is_finite()
            || !interval.end_seconds.is_finite()
            || interval.start_seconds >= interval.end_seconds
        {
            diagnostics.push(CaptionDiagnostic {
                kind: CaptionDiagnosticKind::StartAfterEnd,
                interval_index: Some(index),
                start_seconds: Some(interval.start_seconds),
                end_seconds: Some(interval.end_seconds),
                detail: "시작 시각이 끝 시각보다 앞서지 않습니다.".into(),
            });
        }
        if interval.start_seconds < 0.0 || interval.end_seconds > duration_seconds {
            diagnostics.push(CaptionDiagnostic {
                kind: CaptionDiagnosticKind::OutOfRange,
                interval_index: Some(index),
                start_seconds: Some(interval.start_seconds),
                end_seconds: Some(interval.end_seconds),
                detail: "원본 영상 길이 범위를 벗어났습니다.".into(),
            });
        }
        if interval.text.trim().is_empty() {
            diagnostics.push(CaptionDiagnostic {
                kind: CaptionDiagnosticKind::EmptyText,
                interval_index: Some(index),
                start_seconds: Some(interval.start_seconds),
                end_seconds: Some(interval.end_seconds),
                detail: "자막 내용이 비어 있습니다.".into(),
            });
        } else if interval.text.contains('\u{fffd}') {
            diagnostics.push(CaptionDiagnostic {
                kind: CaptionDiagnosticKind::QualityWarning,
                interval_index: Some(index),
                start_seconds: Some(interval.start_seconds),
                end_seconds: Some(interval.end_seconds),
                detail: "자막에 깨진 문자가 포함되어 있습니다.".into(),
            });
        }
    }

    let mut indexed = intervals.iter().enumerate().collect::<Vec<_>>();
    indexed.sort_by(|(left_index, left), (right_index, right)| {
        left.start_seconds
            .total_cmp(&right.start_seconds)
            .then_with(|| left.end_seconds.total_cmp(&right.end_seconds))
            .then_with(|| left_index.cmp(right_index))
    });

    // For structurally valid intervals sorted by start, an active maximum-end
    // sweep marks every interval that participates in at least one overlap.
    // Intervals with non-finite or inverted bounds are already invalid above
    // and do not need to participate in this sweep.
    let mut overlap_partner: Vec<Option<usize>> = vec![None; indexed.len()];
    let mut active_max: Option<(usize, f64)> = None;
    for (index, interval) in indexed.iter().copied() {
        if !interval.start_seconds.is_finite()
            || !interval.end_seconds.is_finite()
            || interval.start_seconds >= interval.end_seconds
        {
            continue;
        }
        if let Some((active_index, active_end)) = active_max {
            if active_end > interval.start_seconds {
                overlap_partner[index].get_or_insert(active_index);
                overlap_partner[active_index].get_or_insert(index);
            }
        }
        if active_max.map_or(true, |(_, active_end)| interval.end_seconds > active_end) {
            active_max = Some((index, interval.end_seconds));
        }
    }
    for (index, partner) in overlap_partner.into_iter().enumerate() {
        if let Some(partner) = partner {
            let interval = &intervals[index];
            diagnostics.push(CaptionDiagnostic {
                kind: CaptionDiagnosticKind::Overlap,
                interval_index: Some(index),
                start_seconds: Some(interval.start_seconds),
                end_seconds: Some(interval.end_seconds),
                detail: format!("구간 {partner}와 겹칩니다."),
            });
        }
    }

    let mut duplicate_first = HashMap::new();
    let mut duplicate_partner: Vec<Option<usize>> = vec![None; intervals.len()];
    for (index, interval) in intervals.iter().enumerate() {
        let key = (
            interval.start_seconds.to_bits(),
            interval.end_seconds.to_bits(),
            interval.text.trim().to_string(),
        );
        if let Some(&first_index) = duplicate_first.get(&key) {
            duplicate_partner[index] = Some(first_index);
            duplicate_partner[first_index].get_or_insert(index);
        } else {
            duplicate_first.insert(key, index);
        }
    }
    for (index, partner) in duplicate_partner.into_iter().enumerate() {
        if let Some(partner) = partner {
            let interval = &intervals[index];
            diagnostics.push(CaptionDiagnostic {
                kind: CaptionDiagnosticKind::Duplicate,
                interval_index: Some(index),
                start_seconds: Some(interval.start_seconds),
                end_seconds: Some(interval.end_seconds),
                detail: format!("구간 {partner}와 같은 시간과 내용의 중복 구간입니다."),
            });
        }
    }

    let mut gap_count = 0usize;
    let mut largest_gap = 0.0f64;
    let mut previous_end = None;
    for (_, interval) in indexed.iter().copied().filter(|(_, interval)| {
        interval.start_seconds.is_finite()
            && interval.end_seconds.is_finite()
            && interval.start_seconds < interval.end_seconds
    }) {
        if let Some(end) = previous_end {
            if interval.start_seconds > end {
                gap_count += 1;
                largest_gap = largest_gap.max(interval.start_seconds - end);
            }
        }
        previous_end = Some(previous_end.map_or(interval.end_seconds, |end: f64| {
            end.max(interval.end_seconds)
        }));
    }
    if gap_count > 0 {
        diagnostics.push(CaptionDiagnostic {
            kind: CaptionDiagnosticKind::GapObserved,
            interval_index: None,
            start_seconds: None,
            end_seconds: None,
            detail: format!(
                "자막 사이 공백 {gap_count}개를 관찰했으며 최대 공백은 {largest_gap}초입니다. 합격 기준은 정하지 않습니다."
            ),
        });
    }
    if verification_state == VerificationState::Unverified {
        diagnostics.push(CaptionDiagnostic {
            kind: CaptionDiagnosticKind::OffsetUnverified,
            interval_index: None,
            start_seconds: None,
            end_seconds: None,
            detail: "원본 영상과의 일정한 시간 오프셋을 아직 검증하지 않았습니다.".into(),
        });
    }
    CaptionValidation {
        intervals,
        diagnostics,
        verification_state,
    }
}

pub fn plan_fallbacks(validation: &CaptionValidation, duration_seconds: f64) -> CaptionPlan {
    let invalid_indices = validation
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.kind,
                CaptionDiagnosticKind::StartAfterEnd
                    | CaptionDiagnosticKind::OutOfRange
                    | CaptionDiagnosticKind::EmptyText
                    | CaptionDiagnosticKind::QualityWarning
            )
        })
        .filter_map(|diagnostic| diagnostic.interval_index)
        .collect::<HashSet<_>>();
    let provenance_invalid = validation
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.kind == CaptionDiagnosticKind::ProvenanceInvalid);
    let trusted = validation
        .intervals
        .iter()
        .enumerate()
        .filter(|(index, _)| !invalid_indices.contains(index))
        .map(|(_, interval)| interval.clone())
        .collect::<Vec<_>>();
    let full_whisper = trusted.is_empty() || provenance_invalid;
    if full_whisper {
        return CaptionPlan {
            trusted: Vec::new(),
            fallback: vec![FallbackInterval {
                start_seconds: 0.0,
                end_seconds: duration_seconds.max(0.0),
                reason: if trusted.is_empty() {
                    "caption_unavailable".into()
                } else {
                    "caption_provenance_invalid".into()
                },
            }],
            diagnostics: validation.diagnostics.clone(),
            verification_state: validation.verification_state,
            full_whisper: true,
        };
    }

    CaptionPlan {
        trusted,
        // A usable YouTube caption track is authoritative for the whole job.
        // Missing, damaged, or unverified portions stay warnings and never
        // trigger automatic Whisper supplementation.
        fallback: Vec::new(),
        diagnostics: validation.diagnostics.clone(),
        verification_state: validation.verification_state,
        full_whisper: false,
    }
}

pub fn summarize_diagnostics(diagnostics: &[CaptionDiagnostic]) -> Vec<CaptionDiagnostic> {
    let mut grouped = std::collections::BTreeMap::<CaptionDiagnosticKind, Vec<&CaptionDiagnostic>>::new();
    for diagnostic in diagnostics {
        grouped.entry(diagnostic.kind).or_default().push(diagnostic);
    }
    grouped
        .into_iter()
        .map(|(_, group)| {
            let representative = group[0];
            let mut summary = representative.clone();
            if group.len() > 1 {
                summary.detail = format!("{}개 · 대표: {}", group.len(), representative.detail);
            }
            summary
        })
        .collect()
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn caption_resume_compatible(
    provenance: &CaptionProvenance,
    source_url: &str,
    sha256: &str,
    revision: &str,
) -> bool {
    provenance.schema_version == CAPTION_SCHEMA_VERSION
        && provenance.source_url == source_url
        && provenance.sha256 == sha256
        && provenance.revision == revision
}

pub fn safe_caption_file_name(track_id: &str) -> String {
    let filtered = track_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if filtered.is_empty() {
        "caption.vtt".into()
    } else {
        format!("{filtered}.vtt")
    }
}

pub fn persist_provenance(
    job_dir: &Path,
    source_url: &str,
    track: &CaptionTrack,
    bytes: &[u8],
    verification_state: VerificationState,
    diagnostics: Vec<CaptionDiagnostic>,
) -> Result<CaptionProvenance, String> {
    let caption_dir = job_dir.join("captions");
    fs::create_dir_all(&caption_dir).map_err(|error| error.to_string())?;
    let file_name = safe_caption_file_name(&track.track_id);
    fs::write(caption_dir.join(&file_name), bytes).map_err(|error| error.to_string())?;
    let provenance = CaptionProvenance {
        schema_version: CAPTION_SCHEMA_VERSION,
        source_url: source_url.into(),
        source: track.source,
        language: track.language.clone(),
        track_id: track.track_id.clone(),
        revision: track.revision.clone(),
        original_file: format!("captions/{file_name}"),
        sha256: sha256_bytes(bytes),
        verification_state,
        diagnostics,
        content_sha256: sha256_bytes(bytes),
    };
    let encoded = serde_json::to_vec_pretty(&provenance).map_err(|error| error.to_string())?;
    fs::write(job_dir.join("caption-provenance.json"), encoded)
        .map_err(|error| error.to_string())?;
    Ok(provenance)
}

pub fn read_provenance(job_dir: &Path) -> Result<Option<(CaptionProvenance, Vec<u8>)>, String> {
    let path = job_dir.join("caption-provenance.json");
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|error| error.to_string())?;
    let mut provenance: CaptionProvenance =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    let original = safe_caption_path(job_dir, &provenance.original_file)?;
    let caption_bytes = fs::read(&original).map_err(|error| error.to_string())?;
    if sha256_bytes(&caption_bytes) != provenance.sha256 {
        return Err("원본 자막 파일의 SHA-256이 저장된 provenance와 다릅니다.".into());
    }
    provenance.content_sha256 = sha256_bytes(&caption_bytes);
    Ok(Some((provenance, caption_bytes)))
}

fn safe_caption_path(job_dir: &Path, original_file: &str) -> Result<PathBuf, String> {
    let relative = Path::new(original_file);
    if relative.is_absolute() {
        return Err("자막 provenance 경로가 절대 경로입니다.".into());
    }
    let components = relative.components().collect::<Vec<_>>();
    if components.len() != 2
        || !matches!(components[0], Component::Normal(value) if value == "captions")
        || !matches!(components[1], Component::Normal(_))
    {
        return Err("자막 provenance 경로가 작업 폴더의 captions 안에 있지 않습니다.".into());
    }
    Ok(job_dir.join(relative))
}

/// Read provenance without treating damaged metadata or mismatched bytes as usable
/// captions. The media policy decides separately whether an approved Whisper run exists.
pub fn read_provenance_with_diagnostics(
    job_dir: &Path,
) -> Result<Option<(CaptionProvenance, Vec<u8>)>, String> {
    let path = job_dir.join("caption-provenance.json");
    if !path.exists() {
        return Ok(None);
    }
    if !path.is_file() {
        return Ok(Some((
            failed_provenance("자막 provenance 파일 형식이 올바르지 않습니다.".into()),
            Vec::new(),
        )));
    }
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Ok(Some((failed_provenance(format!(
                "자막 provenance를 읽지 못했습니다: {error}"
            )), Vec::new())));
        }
    };
    let mut provenance = match serde_json::from_slice::<CaptionProvenance>(&bytes) {
        Ok(value) => value,
        Err(error) => CaptionProvenance {
            schema_version: CAPTION_SCHEMA_VERSION,
            source_url: String::new(),
            source: CaptionSource::Automatic,
            language: "ko".into(),
            track_id: String::new(),
            revision: String::new(),
            original_file: String::new(),
            sha256: String::new(),
            verification_state: VerificationState::Failed,
            diagnostics: vec![CaptionDiagnostic {
                kind: CaptionDiagnosticKind::ProvenanceInvalid,
                interval_index: None,
                start_seconds: None,
                end_seconds: None,
                detail: format!("자막 provenance를 읽지 못했습니다: {error}"),
            }],
            content_sha256: String::new(),
        },
    };
    if provenance.verification_state == VerificationState::Failed
        && provenance.diagnostics.is_empty()
    {
        provenance.diagnostics.push(CaptionDiagnostic {
            kind: CaptionDiagnosticKind::ProvenanceInvalid,
            interval_index: None,
            start_seconds: None,
            end_seconds: None,
            detail: "자막 검증이 실패한 상태입니다.".into(),
        });
    }
    if provenance.schema_version != CAPTION_SCHEMA_VERSION {
        provenance.verification_state = VerificationState::Failed;
        provenance.diagnostics.push(CaptionDiagnostic {
            kind: CaptionDiagnosticKind::ProvenanceInvalid,
            interval_index: None,
            start_seconds: None,
            end_seconds: None,
            detail: "자막 provenance 버전이 현재 형식과 다릅니다.".into(),
        });
    }
    let original = match safe_caption_path(job_dir, &provenance.original_file) {
        Ok(path) => path,
        Err(detail) => {
            provenance.verification_state = VerificationState::Failed;
            provenance.diagnostics.push(CaptionDiagnostic {
                kind: CaptionDiagnosticKind::ProvenanceInvalid,
                interval_index: None,
                start_seconds: None,
                end_seconds: None,
                detail,
            });
            return Ok(Some((provenance, Vec::new())));
        }
    };
    let caption_bytes = match fs::read(&original) {
        Ok(caption_bytes) if sha256_bytes(&caption_bytes) == provenance.sha256 => caption_bytes,
        Ok(caption_bytes) => {
            provenance.verification_state = VerificationState::Failed;
            provenance.diagnostics.push(CaptionDiagnostic {
                kind: CaptionDiagnosticKind::ProvenanceInvalid,
                interval_index: None,
                start_seconds: None,
                end_seconds: None,
                detail: "원본 자막 파일의 SHA-256이 저장된 provenance와 다릅니다.".into(),
            });
            caption_bytes
        }
        Err(error) => {
            provenance.verification_state = VerificationState::Failed;
            provenance.diagnostics.push(CaptionDiagnostic {
                kind: CaptionDiagnosticKind::ProvenanceInvalid,
                interval_index: None,
                start_seconds: None,
                end_seconds: None,
                detail: format!("원본 자막 파일을 읽지 못했습니다: {error}"),
            });
            Vec::new()
        }
    };
    provenance.content_sha256 = sha256_bytes(&caption_bytes);
    Ok(Some((provenance, caption_bytes)))
}

fn failed_provenance(detail: String) -> CaptionProvenance {
    CaptionProvenance {
        schema_version: CAPTION_SCHEMA_VERSION,
        source_url: String::new(),
        source: CaptionSource::Automatic,
        language: String::new(),
        track_id: String::new(),
        revision: String::new(),
        original_file: String::new(),
        sha256: String::new(),
        verification_state: VerificationState::Failed,
        diagnostics: vec![CaptionDiagnostic {
            kind: CaptionDiagnosticKind::ProvenanceInvalid,
            interval_index: None,
            start_seconds: None,
            end_seconds: None,
            detail,
        }],
        content_sha256: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn interval(start: f64, end: f64, text: &str) -> CaptionInterval {
        CaptionInterval {
            start_seconds: start,
            end_seconds: end,
            text: text.into(),
        }
    }

    #[test]
    fn selects_automatic_korean_before_creator_and_excludes_translation_live_chat_and_other_languages(
    ) {
        let info = serde_json::json!({
            "subtitles": {
                "en": [{"url": "en"}],
                "ko": [{"url": "creator", "vss_id": "ko"}],
                "ko-live_chat": [{"url": "chat"}],
                "ko-translation": [{"url": "translated"}]
            },
            "automatic_captions": {"ko": [{"url": "automatic", "vss_id": "ko-auto"}]}
        });
        let track = select_track(&info).unwrap();
        assert_eq!(track.source, CaptionSource::Automatic);
        assert_eq!(track.track_id, "ko-auto");
        assert_eq!(track.url.as_deref(), Some("automatic"));
        assert_eq!(
            select_tracks(&info)
                .into_iter()
                .map(|track| track.source)
                .collect::<Vec<_>>(),
            vec![CaptionSource::Automatic, CaptionSource::Creator]
        );
    }

    #[test]
    fn falls_back_to_creator_korean_only_when_automatic_is_unusable() {
        let info = serde_json::json!({
            "subtitles": {"ko": [{"url": "creator", "vss_id": "ko-creator"}]},
            "automatic_captions": {"ko": [{"url": "https://example.test/automatic?tlang=ko", "vss_id": "ko-auto"}]}
        });
        let track = select_track(&info).unwrap();
        assert_eq!(track.source, CaptionSource::Creator);
        assert_eq!(track.track_id, "ko-creator");
    }

    #[test]
    fn keeps_track_id_empty_when_provider_does_not_expose_one() {
        let info = serde_json::json!({
            "subtitles": {},
            "automatic_captions": {"ko": [{"url": "automatic"}]}
        });
        let track = select_track(&info).unwrap();
        assert_eq!(track.source, CaptionSource::Automatic);
        assert!(track.track_id.is_empty());
        assert!(track.revision.is_empty());
    }

    #[test]
    fn accepts_native_automatic_ko_or_ko_orig_but_rejects_translated_urls() {
        let ko_orig = serde_json::json!({
            "subtitles": {},
            "automatic_captions": {
                "ko": [{"url": "https://example.test/native?tlang=ko"}],
                "ko-orig": [{"url": "https://example.test/native"}],
                "ko-live_chat": [{"url": "https://example.test/chat"}],
                "ko-translation": [{"url": "https://example.test/translated"}],
                "en": [{"url": "https://example.test/en"}]
            }
        });
        let track = select_track(&ko_orig).unwrap();
        assert_eq!(track.language, "ko-orig");
        assert_eq!(track.source, CaptionSource::Automatic);
        assert!(select_track(&serde_json::json!({
            "subtitles": {"ko": [{"url": "https://example.test/caption?tlang=ko"}]},
            "automatic_captions": {}
        }))
        .is_none());
    }

    #[test]
    fn parses_absolute_vtt_times_and_srt_times() {
        let parsed = parse_caption_text("WEBVTT\n\n00:01.500 --> 00:03.000\n안녕 <b>세계</b>\n");
        assert_eq!(parsed, vec![interval(1.5, 3.0, "안녕 세계")]);
        let parsed = parse_caption_text("1\n00:00:04,000 --> 00:00:05,000\n다음\n");
        assert_eq!(parsed, vec![interval(4.0, 5.0, "다음")]);
    }

    #[test]
    fn cleans_youtube_rolling_vtt_without_empty_transition_errors() {
        let parsed = parse_caption_text(
            "WEBVTT\nKind: captions\nLanguage: ko\n\n\
             01:00:13.829 --> 01:00:13.839 align:start position:0%\n한\n \n\n\
             01:00:13.839 --> 01:00:16.309 align:start position:0%\n한\n미역국<01:00:14.359><c> 끓여</c><01:00:14.720><c> 줄까</c>\n\n\
             01:00:16.309 --> 01:00:16.319 align:start position:0%\n미역국 끓여 줄까\n \n\n\
             01:00:16.319 --> 01:00:19.230 align:start position:0%\n미역국 끓여 줄까\n친구가<01:00:17.240><c> 미역국</c><01:00:17.799><c> 끓여줄게.</c>\n\n\
             01:00:19.230 --> 01:00:19.240 align:start position:0%\n \n \n",
        );
        assert_eq!(
            parsed,
            vec![
                interval(3600.0 + 13.839, 3600.0 + 16.309, "한 미역국 끓여 줄까"),
                interval(3600.0 + 16.319, 3600.0 + 19.230, "친구가 미역국 끓여줄게."),
            ]
        );
        let validation = validate_intervals(parsed, 4000.0, VerificationState::Unverified);
        assert_eq!(
            validation
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.kind == CaptionDiagnosticKind::EmptyText)
                .count(),
            0
        );
    }

    #[test]
    fn keeps_every_automatic_track_before_every_creator_track() {
        let info = serde_json::json!({
            "subtitles": {
                "ko": [{"url": "creator-ko", "vss_id": "creator-ko"}],
                "ko-orig": [{"url": "creator-orig", "vss_id": "creator-orig"}]
            },
            "automatic_captions": {
                "ko-orig": [{"url": "automatic-orig", "vss_id": "automatic-orig"}],
                "ko": [{"url": "automatic-ko", "vss_id": "automatic-ko"}]
            }
        });
        let tracks = select_tracks(&info);
        assert_eq!(
            tracks
                .iter()
                .map(|track| (track.source, track.track_id.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (CaptionSource::Automatic, "automatic-ko"),
                (CaptionSource::Automatic, "automatic-orig"),
                (CaptionSource::Creator, "creator-ko"),
                (CaptionSource::Creator, "creator-orig"),
            ]
        );
    }

    #[test]
    fn removes_only_the_repeated_prefix_from_single_row_rolling_cues() {
        let parsed = parse_caption_text(
            "WEBVTT\n\n\
             00:00:01.000 --> 00:00:01.010\n<c>미역국</c>\n\n\
             00:00:01.010 --> 00:00:03.000\n미역국<00:00:02.000><c> 끓여줄까</c>\n\n\
             00:00:03.000 --> 00:00:03.010\n<c>미역국 끓여줄까</c>\n",
        );
        assert_eq!(parsed, vec![interval(1.01, 3.0, "미역국 끓여줄까")]);
    }

    #[test]
    fn short_transition_cues_never_displace_the_long_outer_cue() {
        let parsed = parse_caption_text(
            "WEBVTT\n\n\
             00:00:01.000 --> 00:00:01.010\n<c>안녕</c>\n\n\
             00:00:01.010 --> 00:00:03.000\n안녕<00:00:02.000><c> 반가워</c>\n\n\
             00:00:03.000 --> 00:00:03.010\n<c>안녕 반가워</c>\n",
        );
        assert_eq!(parsed, vec![interval(1.01, 3.0, "안녕 반가워")]);
    }

    #[test]
    fn records_inverted_out_of_range_overlap_duplicate_gaps_and_unverified_offset() {
        let validation = validate_intervals(
            vec![
                interval(4.0, 3.0, "bad"),
                interval(2.0, 7.0, "overlap"),
                interval(2.0, 7.0, "overlap"),
                interval(9.0, 12.0, "out"),
            ],
            10.0,
            VerificationState::Unverified,
        );
        let kinds = validation
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind)
            .collect::<Vec<_>>();
        assert!(kinds.contains(&CaptionDiagnosticKind::StartAfterEnd));
        assert!(kinds.contains(&CaptionDiagnosticKind::OutOfRange));
        assert!(kinds.contains(&CaptionDiagnosticKind::Overlap));
        assert!(kinds.contains(&CaptionDiagnosticKind::Duplicate));
        assert!(kinds.contains(&CaptionDiagnosticKind::GapObserved));
        assert!(kinds.contains(&CaptionDiagnosticKind::OffsetUnverified));
    }

    #[test]
    fn usable_caption_never_creates_whisper_gap_fallbacks() {
        let validation = validate_intervals(
            vec![interval(2.0, 4.0, "trusted"), interval(6.0, 7.0, "trusted")],
            10.0,
            VerificationState::Verified,
        );
        let plan = plan_fallbacks(&validation, 10.0);
        assert!(!plan.full_whisper);
        assert_eq!(plan.trusted.len(), 2);
        assert!(plan.fallback.is_empty());
    }

    #[test]
    fn overlap_warning_does_not_discard_other_usable_cues_or_start_whisper() {
        let validation = validate_intervals(
            vec![
                interval(2.0, 5.0, "one"),
                interval(4.0, 6.0, "overlap"),
                interval(8.0, 9.0, "trusted"),
            ],
            10.0,
            VerificationState::Verified,
        );
        let plan = plan_fallbacks(&validation, 10.0);
        assert!(!plan.full_whisper);
        assert_eq!(plan.trusted.len(), 3);
        assert!(plan.fallback.is_empty());
        assert!(plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == CaptionDiagnosticKind::Overlap));
    }

    #[test]
    fn damaged_cues_are_dropped_individually_without_whisper_supplementation() {
        let validation = validate_intervals(
            vec![
                interval(1.0, 4.0, "overlap"),
                interval(3.0, 5.0, "other overlap"),
                interval(6.0, 7.0, "duplicate"),
                interval(6.0, 7.0, "duplicate"),
                interval(8.0, 7.0, "inverted"),
                interval(-1.0, 9.0, "out"),
                interval(9.0, 10.0, ""),
                interval(10.0, 11.0, "broken �"),
                interval(12.0, 13.0, "trusted"),
            ],
            14.0,
            VerificationState::Verified,
        );
        let plan = plan_fallbacks(&validation, 14.0);
        assert!(!plan.full_whisper);
        assert_eq!(plan.trusted.len(), 5);
        assert!(plan.trusted.iter().any(|interval| interval.text == "trusted"));
        assert!(plan.fallback.is_empty());
    }

    #[test]
    fn large_caption_input_keeps_diagnostics_bounded_and_partition_indices_correctly() {
        let mut intervals = vec![
            interval(0.0, 1.0, "overlap"),
            interval(0.5, 1.5, "other overlap"),
            interval(2.0, 3.0, "duplicate"),
            interval(2.0, 3.0, "duplicate"),
        ];
        intervals.extend((0..10_000).map(|index| {
            let start = 10.0 + index as f64 * 2.0;
            interval(start, start + 1.0, "trusted")
        }));
        let validation = validate_intervals(intervals, 20_010.0, VerificationState::Verified);
        let overlap_count = validation
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == CaptionDiagnosticKind::Overlap)
            .count();
        let duplicate_count = validation
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == CaptionDiagnosticKind::Duplicate)
            .count();
        let gap_count = validation
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == CaptionDiagnosticKind::GapObserved)
            .count();
        assert_eq!(overlap_count, 4);
        assert_eq!(duplicate_count, 2);
        assert_eq!(gap_count, 1);
        assert!(validation.diagnostics.len() <= 7);

        let plan = plan_fallbacks(&validation, 20_010.0);
        assert_eq!(plan.trusted.len(), 10_004);
        assert!(plan.fallback.is_empty());
    }

    #[test]
    fn unverified_usable_caption_is_kept_and_missing_caption_needs_whisper_decision() {
        let validation = validate_intervals(
            vec![interval(1.0, 2.0, "x")],
            10.0,
            VerificationState::Unverified,
        );
        let plan = plan_fallbacks(&validation, 10.0);
        assert!(!plan.full_whisper);
        assert_eq!(plan.trusted, vec![interval(1.0, 2.0, "x")]);
        assert!(plan.fallback.is_empty());

        let missing = plan_fallbacks(
            &validate_intervals(Vec::new(), 10.0, VerificationState::Failed),
            10.0,
        );
        assert!(missing.full_whisper);
        assert_eq!(missing.fallback.len(), 1);
        assert_eq!(missing.fallback[0].end_seconds, 10.0);
    }

    #[test]
    fn repeated_diagnostics_are_summarized_by_kind() {
        let diagnostics = vec![
            CaptionDiagnostic {
                kind: CaptionDiagnosticKind::EmptyText,
                interval_index: Some(1),
                start_seconds: Some(1.0),
                end_seconds: Some(1.01),
                detail: "비어 있음".into(),
            },
            CaptionDiagnostic {
                kind: CaptionDiagnosticKind::EmptyText,
                interval_index: Some(2),
                start_seconds: Some(2.0),
                end_seconds: Some(2.01),
                detail: "비어 있음".into(),
            },
        ];
        let summary = summarize_diagnostics(&diagnostics);
        assert_eq!(summary.len(), 1);
        assert!(summary[0].detail.starts_with("2개 · 대표:"));
    }

    #[test]
    fn provenance_hash_and_resume_require_url_hash_revision_and_schema() {
        let directory = tempdir().unwrap();
        let track = CaptionTrack {
            track_id: "ko/original".into(),
            language: "ko".into(),
            source: CaptionSource::Creator,
            url: Some("caption-url".into()),
            revision: "r1".into(),
        };
        let provenance = persist_provenance(
            directory.path(),
            "video-url",
            &track,
            b"caption",
            VerificationState::Unverified,
            Vec::new(),
        )
        .unwrap();
        assert_eq!(provenance.original_file, "captions/ko_original.vtt");
        assert!(caption_resume_compatible(
            &provenance,
            "video-url",
            &sha256_bytes(b"caption"),
            "r1"
        ));
        assert!(!caption_resume_compatible(
            &provenance,
            "other-url",
            &sha256_bytes(b"caption"),
            "r1"
        ));
        assert!(read_provenance(directory.path()).unwrap().is_some());
    }

    #[test]
    fn mismatched_caption_is_failed_but_does_not_fail_acquisition() {
        let directory = tempdir().unwrap();
        let track = CaptionTrack {
            track_id: "ko".into(),
            language: "ko".into(),
            source: CaptionSource::Creator,
            url: Some("caption-url".into()),
            revision: "r1".into(),
        };
        let provenance = persist_provenance(
            directory.path(),
            "video-url",
            &track,
            b"caption",
            VerificationState::Verified,
            Vec::new(),
        )
        .unwrap();
        fs::write(directory.path().join(&provenance.original_file), b"changed").unwrap();
        let (failed, bytes) = read_provenance_with_diagnostics(directory.path())
            .unwrap()
            .unwrap();
        assert_eq!(bytes, b"changed");
        assert_eq!(failed.verification_state, VerificationState::Failed);
        assert!(failed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == CaptionDiagnosticKind::ProvenanceInvalid));
        let plan = plan_fallbacks(
            &validate_intervals(
                parse_caption_text(&String::from_utf8_lossy(&bytes)),
                10.0,
                failed.verification_state,
            ),
            10.0,
        );
        assert!(plan.full_whisper);
    }

    #[test]
    fn valid_provenance_resume_preserves_verification_and_diagnostics() {
        let directory = tempdir().unwrap();
        let track = CaptionTrack {
            track_id: "ko".into(),
            language: "ko".into(),
            source: CaptionSource::Creator,
            url: Some("caption-url".into()),
            revision: "r1".into(),
        };
        let diagnostics = vec![CaptionDiagnostic {
            kind: CaptionDiagnosticKind::OffsetUnverified,
            interval_index: None,
            start_seconds: None,
            end_seconds: None,
            detail: "실제 영상 대조 전 상태".into(),
        }];
        let saved = persist_provenance(
            directory.path(),
            "video-url",
            &track,
            b"caption",
            VerificationState::Verified,
            diagnostics.clone(),
        )
        .unwrap();
        let (resumed, _) = read_provenance_with_diagnostics(directory.path())
            .unwrap()
            .unwrap();
        assert_eq!(resumed.verification_state, saved.verification_state);
        assert_eq!(resumed.diagnostics, diagnostics);
    }

    #[test]
    fn unsafe_provenance_path_is_rejected_without_reading_outside_captions() {
        let directory = tempdir().unwrap();
        let outside = directory.path().join("outside.vtt");
        fs::write(&outside, b"must not read").unwrap();
        let metadata = serde_json::json!({
            "schemaVersion": CAPTION_SCHEMA_VERSION,
            "sourceUrl": "video-url",
            "source": "creator",
            "language": "ko",
            "trackId": "ko",
            "revision": "r1",
            "originalFile": "captions/../outside.vtt",
            "sha256": sha256_bytes(b"must not read"),
            "verificationState": "VERIFIED",
            "diagnostics": []
        });
        fs::write(
            directory.path().join("caption-provenance.json"),
            serde_json::to_vec(&metadata).unwrap(),
        )
        .unwrap();
        let (provenance, bytes) = read_provenance_with_diagnostics(directory.path())
            .unwrap()
            .unwrap();
        assert!(bytes.is_empty());
        assert_eq!(provenance.verification_state, VerificationState::Failed);
        assert!(provenance
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == CaptionDiagnosticKind::ProvenanceInvalid));
    }
}
