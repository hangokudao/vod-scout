# VOD Scout 인계서

현재 게이트: **v0.5.0 로컬 후보 · G1~G7 구현/자동 검증 PASS · G8 NSIS/PE/hash/격리 앱 실행 PASS · 서명/공개 Release HOLD**

## 현재 정본

| 항목 | 값 |
|---|---|
| 기준 커밋 | `9c97600` (G8 패키징 correction 기준) |
| 작업 브랜치 | `codex/v050-g8-integration-package` |
| 공개 정본 | v0.4.0 다운로드와 Release 링크는 README에 유지 |
| 로컬 후보 버전 | `0.5.0` (`package.json`, `package-lock.json`, Cargo manifest/lock, `tauri.conf.json`, release notes) |
| G7 상태 | 순차 처리 고정, 병렬 옵션 사용 불가. 실제 동일 입력·자원 측정 전까지 노출하지 않음 |

## G1~G7 구현 결과

- G1: 제작자 한국어 자막 우선, 한국어 자동 자막 대체, 자동 번역·다른 언어·`live_chat` 제외, 원본 시간 검증과 검증 불가 구간의 로컬 Whisper 대체, 자막 provenance 저장.
- G2: `자동(GPU 우선)`·GPU·CPU 모드와 프로필, GPU 근거 게이트, 실패 청크의 CPU 1회 대체, 재실행 시 상태 보존.
- G3: 반복·깨진 음성 인식 결과를 품질 경고로 보존하고 표시 결과에서 가리며, 선택 후보만 실행 ID·개정과 함께 다시 음성 인식.
- G4: FFmpeg·Whisper·채팅·미리보기·UI 단계를 분리한 자원 기록, 경고와 강제 중단 분리, 체크포인트·실패 이유 보존과 자식 종료.
- G5: 후보 `8/20/30`개와 기본 `20`개, 후보 pool과 화면 목록 동기화, evidence 품질 경고·점수 분리, 개정·기존 판정 보존.
- G6: 영상별 독립 작업과 순차 scheduler, 원자적 `queue.json`/`queue.prev.json`, 실행권 단일화, `INTERRUPTED` 수동 복구, 실패 작업이 다음 작업을 막지 않음, 삭제 순서와 실패 참조 보존.
- G7: 측정되지 않은 병렬 실행을 fail-closed하고 순차 전환 이유를 영속화. 구현·자동 테스트는 PASS이나 실제 병렬 안전성·총시간·자원 측정 전에는 기능을 제공하지 않음.

## 자동 검증 (2026-08-18)

- `npm ci`: PASS.
- `npm test`: PASS — 49 tests, 3 files.
- `npm run build`: PASS — TypeScript + Vite, 1,793 modules.
- `cargo.exe test --manifest-path src-tauri/Cargo.toml`: PASS — 126 passed, 1 ignored.
- `cargo.exe test --manifest-path src-tauri/fixture-worker/Cargo.toml`: PASS — 6 passed.
- `npm run test:security`: PASS — 6 passed.
- `node --test scripts/archive-safety.test.mjs scripts/prepare-media-tools.test.mjs scripts/sample-disk-usage.test.mjs`: PASS — 11 passed.
- `git diff --check`: PASS (최종 문서·코드 확정 후).

## 패키지·진입점 결과

- `npm run tauri:build`를 Windows PowerShell에서 재실행했다. 새 FFmpeg 자산 다운로드·SHA-256·media-tools 준비와 release/NSIS 생성은 성공했고, updater 개인키 부재로 서명 단계에서 종료했다.
- `vod-scout.exe`: 16,270,848 bytes, SHA-256 `d29cbf3f2d55e993ef896ecddcc202b6586e0a335f8cc6692fc51dcca1ac2d2f`, PE ProductVersion/FileVersion `0.5.0`.
- `VOD Scout_0.5.0_x64-setup.exe`: 337,435,060 bytes, SHA-256 `2e8cddd19cb756951b58b8937c3171e4a9029cd7de78136bdcd04d745971d0f8`, PE ProductVersion/FileVersion `0.5.0`.
- fresh `VOD_SCOUT_E2E_DATA_DIR`에서 빌드 앱이 8초 생존 후 종료됐고 `instance.lock`·`queue.json`만 생성됐다. 기존 설치 앱·사용자 데이터는 건드리지 않았다.
- 고정 FFmpeg asset: `https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-08-17-13-05/ffmpeg-n8.1.2-44-g7c533d0f86-win64-lgpl-shared-8.1.zip`, archive size 70,837,934 bytes, SHA-256 `681b9ca6d8f9be1e01d8873ad16f8a632f8a22b9653f1044837de6d5979b0fd6`.
- `.sig`와 공개 Release 자산은 개인키 부재로 생성·검증하지 않았으며 `HOLD`다.

## 문서 상태와 남은 HOLD

- 계획·릴리스·아키텍처·UI·테스트 계약은 현재 G1~G7 구현/자동 검증 결과를 가리키며, 실제 YouTube/reference-video, GPU, Windows UI, resource/long-run, parallel measurements는 HOLD다.
- 실제 치지직/YouTube 기준 영상의 자막 품질·시간 오프셋·사람 판정은 실행하지 않았다.
- 실제 GPU 백엔드 시험, GPU→CPU 대체 장치 로그, Windows 사용자 화면 흐름은 실행하지 않았다.
- 1~8시간 resource/long-run 및 기존 v0.4.0과의 동일 입력 비교는 실행하지 않았다.
- G7 병렬 옵션은 사용할 수 없다.
- updater `.sig`, 공개 v0.5.0 URL/Release는 HOLD다. README에는 공개 v0.4.0 링크만 남긴다.

## 롤백

이 브랜치의 변경을 공개 main이나 기존 설치에 반영하지 않는다. 회귀가 확인되면 공개 v0.4.0 정본을 사용하고, 사용자 작업 폴더·설치 폴더를 삭제하거나 덮어쓰지 않는다. 버전업 기록과 패키지 산출물은 실제 결과만 갱신한다.

## 다음 정확한 작업

1. updater 서명 개인키가 승인된 환경에 있을 때만 `.sig` 생성과 release 자산 검증을 진행한다.
2. 실제 기준 영상·GPU·Windows UI·resource/long-run·parallel 측정이 끝날 때까지 v0.5.0 공개 링크·Release를 만들지 않는다.
