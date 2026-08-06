# VOD Scout 인계서 (v0.3.4 공개 + v0.4.0 P0 병합·검증)

현재 게이트: **v0.3.4 공개 릴리스 PASS · Authenticode/SmartScreen HOLD · PR #10·#11 모두 main squash-merge 완료 · 제품 버전 정본 0.3.4 유지 · P0 실제 미디어 검증 부분 PASS + 잔여 HOLD · P1–P7·v0.4.0 버전 범프·설치 배포 계획 HOLD**

## 브랜치·PR 상태

- 공개 `main`: `cca7a9e49301b46c857a76e4203eecf008923eed` (PR #11 squash-merge 후 최신)
- PR #10 (`codex/v034-post-release-docs`): **MERGED** → squash `f5c161414a546e4d3dee29e8816acb8c0dba76c2` (`docs: v0.3.4 post-release evidence (#10)`)
- PR #11 (`codex/v040-p0`): **MERGED** → squash `cca7a9e49301b46c857a76e4203eecf008923eed` (`feat(v0.4.0-p0): path accuracy, checkpoint recovery, disk guard (dev only) (#11)`). 병합 전 head 포함 `d13b864` (H5F F1 수정)와 독립 재리뷰(H5B) PASS.
- 공개·제품 버전 정본: `package.json` / `Cargo.toml` / `tauri.conf.json` 모두 **`0.3.4`** (P0에서 버전 범프 없음; v0.4.0 릴리스·설치 EXE·태그 없음)
- 원본 `C:\Users\myhan\repos\vod-scout`: 읽기 전용 유지 — HEAD `5b50ef8dc416bba5e44346b4eec0b9b880b909a3`, dirty **56** paths (검증 전후 동일)

## v0.3.4 공개 릴리스 상태 (변경 없음)

- 공개 저장소: https://github.com/hangokudao/vod-scout
- 공개 Release: https://github.com/hangokudao/vod-scout/releases/tag/v0.3.4 (ID `365895027`, published `2026-08-06T00:19:47Z`, latest)
- exact merge commit (PR #9): `a341bae3dcf65ff60ecdd3b1ac5c04dd6140952a`
- annotated tag `v0.3.4` object `ea5d807a3535f8fede188d255d2fe7fbf4b03bd0` → peel `a341bae…`
- Actions: run `31057676958` Release Windows app **success**
- Authenticode: 인증서 없음 → **HOLD** (NotSigned; minisign updater와 별개)

상세: `docs/V0.3.4-RELEASE.md`, `BUILD-MANIFEST.md`, `CHANGELOG.md`, `docs/DEBUGGING.md`

## v0.4.0 P0 병합·검증 상태 (main `cca7a9e`)

정본 계획: `docs/V0.4.0-PLAN.md`. 구현·검증 기록: `docs/V0.4.0-RELEASE.md`.

**중요:** P0가 전부 PASS가 아니므로 **v0.4.0 버전 범프·설치 EXE·배포 실행 계획은 포함 조건을 충족하지 않았다.** 현재 제품 버전은 계속 **0.3.4**이며, v0.4.0 릴리스 준비·배포 가능 상태로 읽히게 쓰지 않는다.

### 병합 앵커

| 항목 | 값 |
|---|---|
| PR #10 squash | `f5c161414a546e4d3dee29e8816acb8c0dba76c2` |
| PR #11 squash (main HEAD) | `cca7a9e49301b46c857a76e4203eecf008923eed` |
| 병합 전 F1 수정 커밋 (PR head) | `d13b864` — 비호환 체크포인트 폐기 후 재개 하드 실패 수정 |
| 제품 버전 | **0.3.4** (세 정본 일치) |

### 정규 병합 후 단위·정적 스위트 (H7B 정본)

H7은 gate 5·6 명령 표기를 잘못 잡아 일부 실패로 기록했으나, **하니스/스펙 오류**이며 제품 실패가 아니다. 정본은 **H7B**다.

| 게이트 | 결과 |
|---|---|
| `npm.cmd test` | **PASS** 34 |
| `npm.cmd run build` | **PASS** |
| `cargo test --manifest-path src-tauri/Cargo.toml` | **PASS** 41 pass / 1 ignored |
| `cargo test --manifest-path src-tauri/fixture-worker/Cargo.toml` | **PASS** 5 |
| `npm.cmd run test:security` | **PASS** 6 |
| `git diff --check` | **PASS** |
| **합계** | **6/6 PASS** |

### 실제 미디어·자원 게이트 (H8–H11)

| 게이트 | 결과 | 핵심 증거 |
|---|---|---|
| H8 범위 분석 | **overall PASS** · 전체 재다운로드 재현 **HOLD** | YouTube `JN3BO9GLuFU`, 길이 **31999.981** s, 구간 **[60, 360]**, 후보 **5**, 모두 범위 안·근거 있음. wall ~884.8 s. 피크 job ~13.51 GiB · 최종 ~7.08 GiB. 두 번째 전체 다운로드/재실행 미실시 → **HOLD** |
| H9 디스크 | **단위/정적 PASS** · live low-disk **HOLD** | 단위 `disk_space_guard_explains_shortage_before_start` PASS. 승인된 안전 저용량 볼륨 없음. 정확한 HOLD 문구: **`HOLD: safe low-space E2E environment unavailable`**. YouTube **acquisition에는 다운로드 직전 free-space 검사 없음**(분석 단계 게이트만) |
| H10 취소·재개 | **PASS** | 로컬 H8 소스, 두 600 s 청크 **[60, 1260]**. chunk1 완료 + chunk2 `ffmpeg` 활성 중 `cancel_job` → **CANCELLED 242 ms**. 같은 작업 재개 **3/8**·chunk1 보존 → **REVIEW_READY** 후보 **8**. 재개 wall ~**110.8** s · 총 ~**191.1** s |
| H11 장시간 full | **overall PASS** · 무중단 single-shot **HOLD** | 같은 소스 full, **54/54** · **REVIEW_READY** · 후보 **8**. 유효 연산 ~**3643.4** s · 달력 start–end ~**3831.5** s. 집계 RAM peak ~**507** MB · job tree peak ~**31** MB · H11 root peak ~**34** MB · 최종 job ~**10** MB. CPU 경로; 제품 GPU 사용 0/N/A. 사용자 작업·H8 소스 delta **0**. 중간 **os error 32**는 **out-of-tree H11 샘플러**가 `media-checkpoint.json`을 잠근 하니스 사고(34/54); share-safe 샘플러 + 같은 작업 재개로 완료. **제품 결함으로 표기하지 않음** |

### P0 구현 요약 (코드는 이미 main)

- `media-checkpoint` schema 4: fingerprint, input bytes, runtime SHA-256, language `ko`, ranker `rules-v0.4.0-p0`
- 비호환 체크포인트 폐기 후 작업 `completed_units`가 앞서 있어도 미디어부터 안전 재시작 (H5F F1)
- `build_candidates`: 분석 구간 제한 + audio/dialogue 근거 필수
- `replace_file_preserving_previous` / `.prev` 복구
- 분석 시작 전 `free_disk_space_bytes` + workspace estimate(+10% 여유); acquisition 사전 검사는 미구현
- 미리보기 FFmpeg → `state.cancel_requested`; 취소 활동에 가짜 wall-clock ms 없음

## 보존 무결성 (검증 런 공통)

| 항목 | 값 |
|---|---|
| 사용자 작업 루트 크기 | **5,658,909,957** bytes (delta **0**) |
| 원본 worktree HEAD | `5b50ef8…` · dirty **56** |
| 제품 버전 | **0.3.4** |

## 남은 HOLD / 정확한 다음 작업

P0 **전 게이트 PASS가 아니므로** v0.4.0 버전·설치·배포 실행 계획은 작성하지 않는다. 다음 순서를 지킨다.

1. **승인된 안전 저용량 시험 볼륨**을 마련하고 live low-disk 게이트 실행
   - 분석 시작 전 부족 거절
   - **acquisition(다운로드 직전) free-space 검사** 포함 (현재 코드에 없음 → 구현 필요 여부 확인 후 검증)
2. **H8 재실행**: 신선한 두 번째 전체 다운로드로 재현 서브게이트 해소
3. **H11 재실행**: share-safe 샘플링으로 **처음부터 무중단 single-shot** full 경로
4. 위 P0 잔여 HOLD가 **모두 PASS**된 뒤에만 v0.4.0 버전 정렬·설치 EXE·배포 계획 초안 작성
5. **P1–P7** 구현·merge·release — 미승인
6. Authenticode/SmartScreen · 과거 DisplayVersion 근본 원인 · Whisper 중 취소 측정 — 기존 HOLD 유지

기존 사용자 데이터와 설치본을 삭제하지 않는다. 공개 문서에 비밀값·불필요한 개인 홈 경로·터미널 핸들을 넣지 않는다. 빌드 산출물(`src-tauri/target`, `dist`)은 스테이징하지 않는다.
