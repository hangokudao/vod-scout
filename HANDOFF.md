# VOD Scout v0.4.0 P0 인계서

현재 게이트: **v0.4.0 P0 로컬 구현·독립 리뷰 수정 반영 · 단위/프로젝트 게이트 재검증 대상 · 커밋/PR 전 · 공개 버전 0.3.4 유지 · P1–P7·v0.4.0 릴리스 HOLD**

## 작업 트리

- Orca 전용 worktree: `C:\Users\myhan\orca\workspaces\vod-scout\v040-p0`
- 기준 main SHA: `a341bae3dcf65ff60ecdd3b1ac5c04dd6140952a`
- 로컬 브랜치: `codex/v040-p0` (미커밋 변경 유지; 이 세션에서 commit/push/PR 없음)
- 원본 `C:\Users\myhan\repos\vod-scout` 및 다른 Orca worktree/PR #10: 읽기 전용·미수정

## P0 정의 (정본: `docs/V0.4.0-PLAN.md`)

| 항목 | 완료 조건 | 로컬 결과 |
|---|---|---|
| 체크포인트 호환성 | 입력 지문·도구/모델 해시·언어·분석 범위·후보 계산 버전 불일치 시 재사용 0 | `PASS` (단위) |
| 범위 밖 후보 | 지정 범위 밖 후보 0 | `PASS` (단위) |
| 근거 없는 후보 | 음량·발화 근거 없는 창 제외 | `PASS` (단위) |
| 취소 감독 | 취소 뒤 관련 자식 0·미리보기 취소 연결 | 단위 취소/트리 `PASS`; 실제 YouTube 장시간 `HOLD` |
| 저장 공간 사전 확인 | 부족 시 분석 시작 전 설명 | `PASS` (단위 메시지·estimate) |
| 마지막 정상 세대 | 체크포인트/스냅샷 `.prev` 보존·손상 live 복구 | `PASS` (단위) |

버전 범프·설치 EXE·SBOM·v0.4.0 updater 패키징은 P0에 포함하지 않았다. `package.json` / Cargo / tauri 정본은 `0.3.4`.

## 마지막 PASS

| 항목 | 결과 |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml --lib` | **PASS** 39 pass / 0 fail / 1 ignored |
| `cargo test --manifest-path src-tauri/fixture-worker/Cargo.toml` | **PASS** 5 pass |
| `npm test` / `npm.cmd test` | **PASS** 34 pass |
| `npm.cmd run build` | **PASS** (Windows) |
| `npm.cmd run test:security` | **PASS** 6 pass |
| `git diff --check` | **PASS** (trailing whitespace 수정 후) |

상세 구현 보고: `/tmp/T6A-V040-P0-IMPLEMENTATION.md`
독립 리뷰 보고: `/tmp/T6R-V040-P0-INDEPENDENT-REVIEW.md`

## 코드 요약

- `media-checkpoint` schema 4: fingerprint, input bytes, runtime SHA-256, language `ko`, ranker `rules-v0.4.0-p0` (누락 필드는 빈 값으로 역직렬화 후 호환 실패)
- `build_candidates`: 분석 구간 제한 + audio/dialogue 근거 필수
- `replace_file_preserving_previous` / `.prev` 복구 (media·acquisition·snapshot·provenance); **손상 live JSON은 `.prev`로 복구**, 유효하지만 호환되지 않는 live는 재계산
- 분석 시작 전 `free_disk_space_bytes` + workspace estimate(+10% 여유); 원본이 이미 디스크에 있으면 source 용량 이중 계산 없음
- 미리보기 FFmpeg → `state.cancel_requested`
- 사용자 단계 문구: 진행 메시지에서 `전사` → `음성 인식` (사용자 노출 문자열; AGENTS.md §8)

## v0.3.4 공개 릴리스 상태 (정본 증거, PR #10 문서 미병합과 별개)

post-release 검증 증거(`/tmp/T4C-…`, `/tmp/T4D-…`, `/tmp/T4E-…`, T5 보고) 기준:

| 게이트 | 결과 |
|---|---|
| updater minisign (공개 자산) | **PASS** |
| in-app 0.3.3→0.3.4 (PE / uninstaller / ARP DisplayVersion / 설정 표시) | **PASS** |
| 과거 DisplayVersion 불일치의 근본 원인 재현 | **HOLD** (미증명; 현재 설치 경로는 PASS) |
| Authenticode / SmartScreen | **HOLD** (알려진 한계; 인증서 없음) |

**모든** updater 서명·DisplayVersion 게이트를 한꺼번에 HOLD라고 쓰지 않는다. PR #10(문서 정리)은 이 worktree에서 건드리지 않는다.

## 남은 HOLD / 다음 작업

1. 독립 리뷰 보고 확정 후 개발 PR (커밋·push는 승인 후). 설치 바이너리·모델·미디어는 커밋하지 않는다.
2. 실제 미디어 E2E(범위 분석, 디스크 부족 차단, 취소·재개, 8시간 자원) 증거 — 미실행 → `HOLD`
3. YouTube 선택 스트림 크기 기반 **다운로드 직전** 여유 공간 정밀화 — 현재는 분석 단계 중심; 사전 정밀 측정 PASS를 주장하지 않음
4. P1(자막)·P2~P7 구현은 별도 승인
5. Authenticode/SmartScreen 및 과거 DisplayVersion 근본 원인만 잔여 HOLD (minisign·in-app 갱신은 PASS)
6. v0.4.0 버전 정렬·릴리스 문서 전체·설치 EXE는 merge/release 승인 시에만

기존 사용자 데이터와 설치본을 삭제하지 않는다. 공개 문서에 로컬 절대 경로·터미널 핸들·비밀값을 넣지 않는다.
