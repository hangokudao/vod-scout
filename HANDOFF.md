# VOD Scout 인계서 (v0.3.4 공개 + v0.4.0 P0 · PR #13 검증 BLOCKED)

현재 게이트: **v0.3.4 공개 릴리스 PASS · Authenticode/SmartScreen HOLD · PR #10·#11 main 병합 완료 · PR #13 draft/open/unmerged · 제품 버전 정본 0.3.4 유지 · PR #13 실제 P0 검증 종합 BLOCKED · P0 전 게이트 PASS 아님 · 코드 PR·문서 PR 병합 금지 · v0.4.0 버전·설치·배포 계획 미확정·미포함 · P1–P7 HOLD**

## 브랜치·PR 상태

| 항목 | 값 |
|---|---|
| 공개 `main` | `cca7a9e49301b46c857a76e4203eecf008923eed` (PR #11 squash-merge) |
| PR #10 | **MERGED** · squash `f5c161414a546e4d3dee29e8816acb8c0dba76c2` |
| PR #11 | **MERGED** · squash `cca7a9e…` (`feat(v0.4.0-p0): path accuracy, checkpoint recovery, disk guard (dev only)`) |
| **PR #13** | **draft · open · unmerged** · 브랜치 `codex/p0-download-space-guard` · 고정 HEAD **`4597010c99bf8432f1fe15ba34269fd63d5daa7c`** · 제목 *Guard YouTube downloads against low disk space* · URL https://github.com/hangokudao/vod-scout/pull/13 |
| 제품 버전 정본 | `package.json` / `Cargo.toml` / `tauri.conf.json` 모두 **`0.3.4`** (버전 범프 없음) |
| 원본 main worktree | 읽기 전용 · HEAD `5b50ef8…` · dirty **56** · 검증 중 미변경 |

**병합 정책 (현재):** P0가 전부 PASS가 아니므로 **PR #13 코드 PR과 이 문서 PR을 병합하지 않는다.** v0.4.0 버전 정렬·설치 EXE·태그·배포 실행 계획을 확정하거나 문서에 포함하지 않는다.

## v0.3.4 공개 릴리스 상태 (변경 없음)

- 공개 저장소: https://github.com/hangokudao/vod-scout
- 공개 Release: https://github.com/hangokudao/vod-scout/releases/tag/v0.3.4 (ID `365895027`, published `2026-08-06T00:19:47Z`, latest)
- exact merge commit (PR #9): `a341bae3dcf65ff60ecdd3b1ac5c04dd6140952a`
- annotated tag `v0.3.4` object `ea5d807a3535f8fede188d255d2fe7fbf4b03bd0` → peel `a341bae…`
- Actions: run `31057676958` Release Windows app **success**
- Authenticode: 인증서 없음 → **HOLD** (NotSigned; minisign updater와 별개)

상세: `docs/V0.3.4-RELEASE.md`, `BUILD-MANIFEST.md`, `CHANGELOG.md`, `docs/DEBUGGING.md`

## PR #13 실제 P0 검증 (2026-08-07 · 현재 정본)

정본 계획: `docs/V0.4.0-PLAN.md`. 구현·검증 기록: `docs/V0.4.0-RELEASE.md`.

- 검증 고정 HEAD: **`4597010c99bf8432f1fe15ba34269fd63d5daa7c`** (작업 트리 깨끗함)
- 증거 루트: `D:\vod-scout-p0-evidence-20260807-023415-pr13-4597010`
- 보고: `…\reports\P0-validation-report-ko.md` · 인덱스 `…\reports\evidence-index.json`
- 승인 URL: `https://www.youtube.com/watch?v=JN3BO9GLuFU`
- **종합 판정: BLOCKED** (전체 신선 내려받기·중단 없는 장시간 분석 미완료)

### 사전·정적 게이트

| 게이트 | 결과 |
|---|---|
| 고정 HEAD + 클린 트리 | **PASS** |
| 독립 재리뷰 | **PASS** — `D:\vod-scout-p0-evidence-20260807-023415-pr13-4597010\reports\PR13-rereview-4597010.md` · SHA-256 `E5F285F19A6ACEC2FAED90639930A45088F9069D6AB4622D59B8640303885091` · `evidence-index.json` `fixedHeadCodeReview.reviewOutcome` / `gates.independentCodeRereview` |
| 정규(canonical) 6개 명령 | **PASS** — 동일 보고서 Command evidence 1–6 · `gates.canonicalSixCommands` / `fixedHeadCodeReview.canonicalCommands` |
| 샘플러 스모크 (`sample-disk-usage` 3/3) | **PASS** |
| 실제 low-space(저용량) 격리 E2E | **HOLD** — 안전 저용량 환경 없음 (`safe low-space E2E environment unavailable`; 격리 쿼터/볼륨 없음, 관리자 권한 없음, C/D 채우기·자격 증명 요청 금지) |

### 검증용 실행 파일 (설치·릴리스 자산 아님)

| 항목 | 값 |
|---|---|
| 경로(검증 worktree) | `…\vod-followup-h7-post-merge\src-tauri\target\release\vod-scout.exe` |
| 빌드 | `npx tauri build --no-bundle` |
| 표시 버전 | **0.3.4** |
| 크기 | **15,270,400** bytes |
| SHA-256 | **`2914E618F4F99A075C91E8CC888BD2E277ACFF57CEB34ED2F324519AE49B7D98`** |

이 바이너리는 **공개 설치 EXE·GitHub Release 자산·설치 폴더 치환 대상이 아니다.** 검증 전용 release 빌드다. 공개 설치 자산은 계속 v0.3.4 Release다.

### 제품 작업 (유일한 create/start)

| 항목 | 값 |
|---|---|
| 작업 ID | **`fd8c1cc5-3bfc-4e02-b97a-036ff1009f5e`** |
| attempt1 | 하네스 전용 실패 — plain cargo release가 `devUrl` 로드, `create_job` 전 중단. **제품 작업 없음** |
| attempt2 | **유일한 제품 작업** — `create_job`+`start_job` 1회. 재시도·재개 **없음** |
| 메타데이터·저장 공간 점검 | **PASS** — `yt-dlp.metadata.json` 748,069 bytes · 길이 32,000 s · 스트림 합 7,070,731,050 bytes · 피크 추정 15,555,608,310 bytes (~15.56 GB / ~14.49 GiB) · 활동 문구에 저장 공간 점검 포함 |
| 부분 전송 | 약 **10,033,007** bytes (`source.f298.mp4.part` ~10 MB) 후 중단 |
| 전체 신선 내려받기 | **BLOCKED** — YouTube **HTTP 403 Forbidden** (`unable to download video data`) |
| 중단 없는 장시간 분석 · REVIEW_READY | **BLOCKED** (내려받기 실패로 미진입) |
| 단말 status | `FAILED` · `YouTube 영상을 다운로드하지 못했습니다.` · 단계 `YouTube 다운로드 실패` |
| 자원 메트릭 요약 | **HOLD** (샘플러 요약 파일 부재; NDJSON 피크만 재계산) |
| 설치 폴더 `D:\VOD Scout` | **PASS** — 전후 453,110,594 bytes / 47 files **동일** |
| 사용자 앱 데이터 루트 (WebView 캐시 등) | **HOLD** — 전후 집계 변동·EBWebView 터치 관측; 불변 주장 불가 |
| 사용자 jobs 폴더 before-after | **HOLD** — 실행 전 단독 측정 없음. 현재만 5,658,909,957 bytes / 354 files |
| 작업 소유 프로세스 잔존 | **PASS** (없음) |

### 역사적 맥락 (H8–H11 · 현재 원샷 PASS 근거 아님)

PR #11 병합 직후 main `cca7a9e`에서 수행한 H8 범위 분석·H10 취소·재개·H11 full 재개 완료 등은 **과거 세션 기록**이다.
**이번 PR #13 고정 HEAD `4597010…`의 원샷 실제 미디어 검증 PASS 근거로 승격하지 않는다.**
현재 P0 마감·병합 판단은 위 PR #13 증거(종합 **BLOCKED**)와 잔여 HOLD만 사용한다.

| 과거 게이트 (참고) | 당시 결과 | 현재 취급 |
|---|---|---|
| H8 범위 [60,360] | overall PASS · 재다운로드 재현 HOLD | 역사 기록만 |
| H9 단위 디스크 가드 | PASS · live low-disk HOLD | live low-space는 PR #13에서도 **HOLD** |
| H10 취소·재개 | PASS | 역사 기록만 · 이번 런 미실행 |
| H11 full (재개 포함) | overall PASS · 무중단 single-shot HOLD | 역사 기록만 · 이번 런 **BLOCKED**로 미도달 |

## P0 구현 요약 (main + PR #13 미병합 변경)

**main (`cca7a9e`)에 이미 있음**

- `media-checkpoint` schema 4, 범위 후보·근거, `.prev` 세대, 분석 시작 전 free-space, 취소 연결
- H5F: 비호환 체크포인트 폐기 후 미디어 안전 재시작

**PR #13 (`4597010…`, 미병합)**

- YouTube 다운로드 경로 저장 공간 가드·메타데이터 점검 노출 정리 (acquisition)
- 실제 런에서 메타·공간 점검 활동 문구와 메타 로그는 관측됐으나, 미디어 전송은 403으로 실패

## 보존 관찰

| 항목 | 값 |
|---|---|
| 설치 폴더 `D:\VOD Scout` | 전후 동일 **PASS** |
| 사용자 jobs 현재 크기 | **5,658,909,957** bytes (before-after 쌍 **HOLD**) |
| 원본 worktree | `5b50ef8…` · dirty **56** · 미변경 |
| 제품 버전 | **0.3.4** |

## 남은 HOLD / 정확한 다음 작업

P0 **전 게이트 PASS가 아니므로** PR #13·문서 PR 병합과 v0.4.0 버전·설치·배포 실행 계획은 하지 않는다.

1. YouTube **HTTP 403** 원인을 재현 가능한 환경에서 분리한 뒤, **재시도·재개 없이** 새 신선 작업으로 전체 내려받기 완료 게이트를 다시 연다 (정책에 맞는 승인 후).
2. 승인된 **안전 저용량 시험 볼륨**으로 live low-space E2E PASS.
3. 중단 없는 장시간 full 분석 → **REVIEW_READY** 원샷 PASS (이번 403으로 **BLOCKED** 상태).
4. 자원 샘플러 요약 파일을 남기고 메트릭 **HOLD** 해소.
5. 사용자 앱 데이터 루트·jobs **before-after** 쌍 증거를 실행 전에 확보 (WebView 캐시와 작업 데이터 구분).
6. 위가 **모두 PASS**된 뒤에만 코드/문서 병합 검토 → 그 다음 v0.4.0 버전·설치·배포 계획.
7. **P1–P7** · Authenticode · 과거 DisplayVersion 근본 원인 · Whisper 중 취소 — 기존 HOLD.

기존 사용자 데이터와 설치본을 삭제하지 않는다. 공개 문서에 비밀값·불필요한 개인 홈 경로·터미널 핸들을 넣지 않는다. 빌드 산출물(`src-tauri/target`, `dist`)은 스테이징하지 않는다.
