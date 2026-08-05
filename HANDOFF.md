# VOD Scout v0.3.4 통합 인계서

현재 게이트: **v0.3.4 소스 게이트·pre-PR NSIS 패키징 증거·실제 YouTube 취소/재개 PASS · 전체 병합 종료 디스크 피크 PASS · 서명된 updater 자산·실제 설치/updater/DisplayVersion/공개 재다운로드·Authenticode HOLD · v0.4.0 구현 전 HOLD**

## v0.3.4 현재 상태

- 작업 브랜치: `codex/v034-stability-release` (Orca 작업 트리). 원본 main 작업 트리는 읽기 전용으로 유지했으며 빌드 원본이 아니다.
- pre-PR 게이트 시점 HEAD: `7fd92cb2d28b43e2e95ba2b39a70581ddc0a3d2a` (이후 문서·SBOM·manifest `preparedAt` 갱신 가능)
- 버전 정본: `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, README 설치 링크·파일명, `src/releaseNotes.ts`, release/installer-smoke workflow, `scripts/generate-release-assets.mjs` 모두 `0.3.4`.
- 기능 변경(다른 워커 소유, 통합 시 보존):
  - 취소: job-scoped 종료, 취소 신호 순서, 단계 안내 (`src-tauri/src/{media,acquisition,lib}.rs`)
  - UI: 설정 진입점·다크 입력 카드 (`src/App.tsx`, `src/App.css`, `src/App.test.tsx`)
  - 측정: `scripts/sample-disk-usage.mjs` + `docs/DEVELOPMENT.md` 연결
- DisplayVersion: 추측 훅 없음. ARP `0.3.2` / 바이너리 `0.3.3` 불일치와 NSIS `DisplayVersion=${VERSION}` 템플릿 증거를 기록. 제어된 v0.3.4 설치·updater 재현 게이트 `HOLD`.
- Authenticode: 인증서 없음 → `HOLD`.
- v0.4.0 P0~P7: 구현하지 않음.

## 마지막 PASS (pre-PR 패키징 게이트)

| 항목 | 결과 |
|---|---|
| cancel/job-scoped 종료 cargo 테스트 | 32 pass / 0 fail / 1 ignored |
| fixture-worker | 5 pass |
| `npm.cmd test` | 34 pass |
| `npm.cmd run test:security` | 6 pass |
| `npm.cmd run build` | PASS |
| sidecar release · media-tools · check:yt-dlp | PASS (`2026.07.04`) |
| `cargo fmt --check` · `git diff --check` · secret/path/version 스캔 | PASS |
| `npm audit` | 0 / 0 / 0 / 0 / 0 (info…critical) |
| SPDX SBOM 재생성 | PASS · 루트 `0.3.4` · 656 packages · SHA-256 `6FF3D7A3130C35A560F06EAB00E48258F45165C6EE2D8379DBC0F30193E6DE9A` |
| `npm.cmd run tauri:build` NSIS 본문 | PASS 생성 · 설치 EXE SHA-256 `CD5033E86A095F3118D24CAFD2535238B163592D755D0B7BF317028CEC198DA0` (233,851,958 bytes) · 앱 PE `0.3.4` · **pre-PR 증거, 공개 자산 아님, gitignore** |
| updater 서명 단계 | FAIL → `HOLD` (`TAURI_SIGNING_PRIVATE_KEY` 부재; 키 미생성·미노출) |
| 디스크 샘플러 열린 핸들 65536→131072 | PASS (이전 제한 검증) |
| 브라우저 대비·1280×900/540×900 설정 진입점 | PASS (이전 통합 측정) |
| 실제 YouTube 취소·재개 (release exe + 내장 yt-dlp/FFmpeg, 승인 URL `JN3BO9GLuFU`) | **PASS** — 1차 취소 1,405ms / 자식 1,418ms; 재개 yt-dlp 재기동; 2차 취소 3,390ms (하드캡 8s 이내) |
| 실제 YouTube 전체 병합 종료 디스크 피크 (`sample-disk-usage.mjs` 1s) | **PASS** — peak 14,045,353,616 bytes (~13.08 GiB); final 7,068,902,876; peak−final 임시 6,976,450,740; 완성 `source.mkv` 7,060,479,026 bytes · 31,999.981s; `ACQUIRING`→`PROBING`; 표본 816; 시작→획득 824.2s |

상세: `BUILD-MANIFEST.md`, `docs/V0.3.4-RELEASE.md`, `/tmp/T3A-V034-PREPR-PACKAGING.md`, `/tmp/T3B-V034-YOUTUBE-CANCEL-DISK.md`, `/tmp/T3B2-V034-FULL-MERGE-PEAK.md`

## 남은 HOLD

1. 로컬/CI 서명 키로 updater `.sig`·`latest.json`·`SHA256SUMS` 생성과 `generate-release-assets` (위조 서명 금지)
2. 실제 설치·in-app updater·HKCU `DisplayVersion` 재현 (기대: 앱·PE·ARP 모두 `0.3.4`)
3. GitHub Actions release/installer-smoke 실행 · 공개 재다운로드
4. Windows Authenticode
5. v0.4.0 P1–P7 구현·merge·release (미승인). P0 브랜치·개발 PR은 v0.3.4 릴리스 게이트 완료 후에만 시작(해당 범위는 이미 승인됨)

## v0.3.3 공개 기준선 (변경 금지)

- 공개 저장소: https://github.com/hangokudao/vod-scout
- Release: https://github.com/hangokudao/vod-scout/releases/tag/v0.3.3
- exact commit: `5f756af7390325a99f2820a424f7d4ef05334d14`
- 상세: `docs/V0.3.3-RELEASE.md`, `BUILD-MANIFEST.md`

## 다음 작업

1. 이 브랜치 커밋·push·PR은 승인된 범위에서 진행 가능. **이 pre-PR 세션에서는 커밋·push·PR·태그·설치·사용자 데이터 변경을 하지 않았다.**
2. 서명 키를 주입한 환경에서 규정 `tauri:build` + `generate-release-assets`로 공개 계약 자산 생성.
3. 공개 v0.3.3 → v0.3.4 updater 재현으로 DisplayVersion 게이트 닫기. 실패 시에만 최소 수정.
4. (완료) 실제 YouTube 취소·재개·전체 병합 종료 디스크 피크를 릴리스 문서에 기록.
5. v0.3.4 릴리스 게이트 완료 후 v0.4.0 P0 브랜치·개발 PR 시작(승인됨). P1–P7 구현과 v0.4.0 merge/release는 미승인.

기존 사용자 데이터와 설치본을 삭제하지 않는다. 공개 문서에 로컬 절대 경로·터미널 핸들·비밀값을 넣지 않는다. 빌드 산출물(`src-tauri/target`, `dist`)은 스테이징하지 않는다.
