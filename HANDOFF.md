# VOD Scout 인계서 (v0.3.4 공개 + v0.4.0 P0 개발)

현재 게이트: **v0.3.4 공개 릴리스·인앱 updater·DisplayVersion·작업 보존 PASS · Authenticode/SmartScreen HOLD · PR #10(포스트 릴리스 문서) 병합 완료 · PR #11(`codex/v040-p0`)이 현재 개발 PR · 공개 버전 0.3.4 유지 · P1–P7·v0.4.0 릴리스 HOLD**

## 브랜치·PR 상태

- 공개 `main`: `f5c161414a546e4d3dee29e8816acb8c0dba76c2` (PR #10 squash-merge: `docs: v0.3.4 post-release evidence (#10)`)
- PR #10 (`codex/v034-post-release-docs`): **MERGED** (문서 전용; 제품 코드·태그·Release 자산 변경 없음)
- PR #11 (`codex/v040-p0`): **현재 개발 PR** — v0.4.0 P0 구현. 이 인계서는 main 동기화(merge) 후 상태를 반영한다.
- 공개 버전 정본: `package.json` / Cargo.toml / `tauri.conf.json` 모두 `0.3.4` (P0에서 버전 범프 없음)
- 원본 `C:\Users\myhan\repos\vod-scout` 및 다른 Orca worktree: 이 작업에서 수정하지 않음

## v0.3.4 공개 릴리스 상태 (main 정본)

- 공개 저장소: https://github.com/hangokudao/vod-scout
- 공개 Release: https://github.com/hangokudao/vod-scout/releases/tag/v0.3.4 (ID `365895027`, published `2026-08-06T00:19:47Z`, latest)
- exact merge commit (PR #9): `a341bae3dcf65ff60ecdd3b1ac5c04dd6140952a`
- annotated tag `v0.3.4` object `ea5d807a3535f8fede188d255d2fe7fbf4b03bd0` → peel `a341bae…`
- Actions: run `31057676958` Release Windows app **success**
- 버전 정본: `package.json`, lock, Cargo.toml/lock, `tauri.conf.json`, README 설치 링크·파일명 모두 `0.3.4`
- 기능 요약: 설정 진입점·다크 입력 카드·job-scoped 취소·디스크 샘플러 문서화 (스키마 호환 유지)
- Authenticode: 인증서 없음 → **HOLD** (NotSigned; minisign updater와 별개)

### v0.3.4 마지막 PASS (공개 게이트)

| 항목 | 결과 |
|---|---|
| exact merge + 버전 정렬 | PASS · `a341bae…` |
| 태그·CI 초안 릴리스 | PASS · tag peel exact · run `31057676958` success |
| 공개 5자산 계약 | PASS · 설치 EXE 233,849,362 · SHA-256 `6848c438…2fa3` · `.sig` · `latest.json` · `SHA256SUMS` · SBOM |
| 공개 직접 다운로드·API latest | PASS · unauth HTTP 200 · latest=v0.3.4 · digests 일치 |
| updater minisign | PASS · 앱 공개키 대비 설치 EXE 검증 |
| 인앱 v0.3.3→v0.3.4 | PASS · 설정 `지금 업데이트`만 · 메인/`uninstall`/`DisplayVersion` 모두 `0.3.4` · 설정 `최신 상태` |
| 작업·데이터 보존 | PASS · 작업 15 · 데이터 파일 2,087 · 해시/크기/mtime 변경 0 |
| 실제 YouTube 취소·재개 | PASS · 1.4s / 3.4s (하드캡 8s) |
| 전체 병합 디스크 피크 | PASS · peak ~13.08 GiB · final ~6.58 GiB |
| 단위/보안/SBOM (소스 게이트, v0.3.4 기준) | PASS · cargo 32 · npm 34 · security 6 · SBOM 656 pkgs |

상세: `docs/V0.3.4-RELEASE.md`, `BUILD-MANIFEST.md`, `CHANGELOG.md`, `docs/DEBUGGING.md`

### v0.3.4 남은 HOLD

1. **Windows Authenticode / SmartScreen** — 설치 EXE·앱 `NotSigned`. 인증서 구매·생성 미승인. updater minisign은 PASS.
2. **과거 DisplayVersion 근본 원인** — v0.3.2→v0.3.3에서 ARP `0.3.2` 잔류 원인은 미확정. v0.3.3→v0.3.4 인앱 경로 결과는 PASS.
3. **Whisper 음성 인식 중 취소** — 미실행.

**모든** updater 서명·DisplayVersion 게이트를 한꺼번에 HOLD라고 쓰지 않는다.

## v0.4.0 P0 개발 상태 (PR #11)

정본 계획: `docs/V0.4.0-PLAN.md`. 버전 범프·설치 EXE·SBOM·v0.4.0 updater 패키징은 P0에 포함하지 않았다.

| 항목 | 완료 조건 | 로컬 결과 |
|---|---|---|
| 체크포인트 호환성 | 입력 지문·도구/모델 해시·언어·분석 범위·후보 계산 버전 불일치 시 재사용 0 | `PASS` (단위) |
| 범위 밖 후보 | 지정 범위 밖 후보 0 | `PASS` (단위) |
| 근거 없는 후보 | 음량·발화 근거 없는 창 제외 | `PASS` (단위) |
| 취소 감독 | 취소 뒤 관련 자식 0·미리보기 취소 연결 | 단위 취소/트리 `PASS`; 실제 YouTube 장시간 `HOLD` |
| 저장 공간 사전 확인 | 부족 시 분석 시작 전 설명 | `PASS` (단위 메시지·estimate) |
| 마지막 정상 세대 | 체크포인트/스냅샷 `.prev` 보존·손상 live 복구 | `PASS` (단위) |

### P0 마지막 PASS (브랜치 단위/프로젝트 게이트)

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

### P0 코드 요약

- `media-checkpoint` schema 4: fingerprint, input bytes, runtime SHA-256, language `ko`, ranker `rules-v0.4.0-p0` (누락 필드는 빈 값으로 역직렬화 후 호환 실패)
- `build_candidates`: 분석 구간 제한 + audio/dialogue 근거 필수
- `replace_file_preserving_previous` / `.prev` 복구 (media·acquisition·snapshot·provenance); **손상 live JSON은 `.prev`로 복구**, 유효하지만 호환되지 않는 live는 재계산
- 분석 시작 전 `free_disk_space_bytes` + workspace estimate(+10% 여유); 원본이 이미 디스크에 있으면 source 용량 이중 계산 없음
- 미리보기 FFmpeg → `state.cancel_requested`
- 사용자 단계 문구: 진행 메시지에서 `전사` → `음성 인식` (사용자 노출 문자열; AGENTS.md §8)

## v0.3.3 공개 기준선 (변경 금지)

- Release: https://github.com/hangokudao/vod-scout/releases/tag/v0.3.3
- exact commit: `5f756af7390325a99f2820a424f7d4ef05334d14`
- 상세: `docs/V0.3.3-RELEASE.md`, `BUILD-MANIFEST.md`

## 남은 HOLD / 다음 작업

1. PR #11 리뷰·검증 후 병합 여부는 별도 승인. 설치 바이너리·모델·미디어는 커밋하지 않는다.
2. 실제 미디어 E2E(범위 분석, 디스크 부족 차단, 취소·재개, 8시간 자원) 증거 — 미실행 → `HOLD` (P0 단위 결과만 PASS로 기록; 새 미디어 결과를 발명하지 않음)
3. YouTube 선택 스트림 크기 기반 **다운로드 직전** 여유 공간 정밀화 — 현재는 분석 단계 중심; 사전 정밀 측정 PASS를 주장하지 않음
4. **v0.4.0 P1–P7** 구현·merge·release 미승인
5. Authenticode/SmartScreen 및 과거 DisplayVersion 근본 원인만 잔여 HOLD (minisign·in-app 갱신은 PASS). 인증서 없이 PASS로 기록하지 않는다.
6. v0.4.0 버전 정렬·릴리스 문서 전체·설치 EXE는 merge/release 승인 시에만
7. Whisper 음성 인식 중 취소 등은 필요 시 후속 패치에서 측정한다.

기존 사용자 데이터와 설치본을 삭제하지 않는다. 공개 문서에 비밀값·불필요한 개인 홈 경로·터미널 핸들을 넣지 않는다. 빌드 산출물(`src-tauri/target`, `dist`)은 스테이징하지 않는다.
