# VOD Scout 인계서

현재 게이트: **v0.5.0 validation-fixes · 자막 선택 순서·E2E 도구·자동 검증 PASS · GPU 패키지 BLOCKED · 일반 YouTube 무취소 흐름은 HTTP 403 재현·외부 원인 분리 후 BLOCKED · 제품 provenance/Whisper 대체/전체 앱 흐름/화면·설치·updater 서명/공개 Release HOLD**

## 현재 정본

| 항목 | 값 |
|---|---|
| origin/main 기준 | `eee71e04776a6179c289167596e9d82d52e94e13` (PR #18 반영) |
| G8 패키지 증거 원본 | `6ecbd49` |
| 로컬 통합 병합 | `7c8b336` |
| 작업 브랜치 | `codex/v050-validation-fixes` |
| 공개 정본 | v0.4.0 다운로드와 Release 링크는 README에 유지 |
| 로컬 후보 버전 | `0.5.0` (`package.json`, `package-lock.json`, Cargo manifest/lock, `tauri.conf.json`, release notes) |
| G7 상태 | 순차 처리 고정, 병렬 옵션 사용 불가. 실제 동일 입력·자원 측정 전까지 노출하지 않음 |

## validation-fixes 결과 (2026-08-19)

- E2E 검증 도구는 `chatScore: null`을 기본 허용하고, 저장된 작업 스냅샷을 최대 10초 확인해 `CANCELLED`를 판정한다. 오류 시 마지막 스냅샷·예외·구조화 로그를 보존하며 PowerShell 실행기는 앱 경로·스크린샷·자식 프로세스 정리를 지원한다.
- `npm test` 49개, `npm run build`, Rust 본체 127 passed·1 ignored, fixture-worker 6개, 관련 Node 17개, 보안 6개, `git diff --check`는 `PASS`다. 실제 G8 앱은 승인 URL의 시작 단계와 취소 완료를 확인했다.
- 세 승인 URL은 일반 VOD의 자동 한국어 자막 우선 대표 입력이며 `language=ko`, 자동 생성 한국어 SRT, 파일 무결성·기본 시간 범위를 확인했다. 제작자 자막 부재는 `HOLD`나 릴리스 차단 사유가 아니며, 제품 snapshot provenance와 Whisper 대체는 별도 `HOLD`다. 겹침·공백 수치는 품질 `PASS`가 아니며 일정한 시간 오프셋과 내용 품질도 `HOLD`다.
- 자막 증거는 다음과 같다: `JKYmw9-xMIo` — 133570 bytes, SHA-256 `af6aa5d008bbdd36e60f8c07d556da52686cb52be99b660e8e555783b4f510ef`, 2121 cues, start>=end 0, out-of-range 0, reverse 0, adjacent overlaps 1346, exact duplicate groups 0, max positive gap 47.080 sec; `LVZ6hFhlF2k` — 399263 bytes, SHA-256 `24857fa9aee1fd459e040d5939159ca3c0ea45bb69fa0f7ed7925bf5dfcf1efa`, 5832 cues, start>=end 0, out-of-range 0, reverse 0, adjacent overlaps 5364, exact duplicate groups 0, max positive gap 207.361 sec; `ZJMpYThMksM` — 88141 bytes, SHA-256 `64d876c1ff3813bfc2309d1302af60a12618a55ace924843d7a536a4136c6c55`, 1472 cues, start>=end 0, out-of-range 0, reverse 0, adjacent overlaps 880, exact duplicate groups 0, max positive gap 147.040 sec.
- 실제 앱은 취소 요청 뒤 4779 ms에 `CANCELLED`를 확인했지만 HTTP 403이 검토 화면 전에 발생했다. 따라서 전체 앱 흐름은 외부 YouTube 접근 제한으로 `BLOCKED`이며, 화면 캡처와 전체 후보·검토·삭제 흐름은 후속 검증 `HOLD`다. 최종 증거는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-evidence-full-225324\e2e-failure-2026-08-19T13-53-48-401Z-21572.json`과 같은 basename의 `.log`다.
- 이번 재검증에서는 `JKYmw9-xMIo` 무취소 앱 E2E를 한 번 실행했으나 acquisition 전 CDP/Tauri IPC 평가에서 실패했다. 기존 HTTP 403 원본 JSON/로그는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-evidence-full-225324\e2e-failure-2026-08-19T13-53-48-401Z-21572.json` 및 같은 basename의 `.log`로 보존했고, coordinator의 재검증 상한에 따라 E2E를 다시 실행하지 않았다. 고정 yt-dlp/Deno의 signed URL 범위 요청은 HTTP `206`, 자동 한국어 자막 저장은 성공했으므로 앱 전체 403은 외부 YouTube 접근 제한 `BLOCKED`로 남긴다.
- G8 GPU 패키지는 `whisper-cli.exe` 489984 bytes/SHA-256 `4bf174113843613cbec146e73e6820a767e54b0e1c736f2c6d7ab16aac4c245d`, `ggml-cuda.dll` 562600960 bytes/SHA-256 `24af2cd89090175beffdf77cd25c176d76f09c4018644915f302d2de64d67631`, `cudart32_110.dll` 467456 bytes/SHA-256 `b8bfc244dd0916ddf7b45e39c101f165a0d9f4846616eaf34336a2c374409408`, `cudart64_110.dll` 526848 bytes/SHA-256 `ba5c2fb526c4ee4bb218ceb3fa5e8bfde89ce474f38711fdcce802549bf9fc6f`이며 `cublas64_11.dll`이 없다. CPU backend 로그는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-evidence-full-225324\gpu-probe\whisper-gpu.log`, valid `probe.srt`는 47 bytes/SHA-256 `5845cd37d6a0bbae0ce13a136d7652d6b5688938ceaf32a6974a20a57a24a97d`다. 외부 GPU 바이너리 추가 없이는 패키지 보완과 실제 GPU 검증을 끝낼 수 없어 `BLOCKED`다.
- 이번 변경은 G8 패키지·설치 파일·모델을 수정하지 않았다.

G1~G8은 이 작업 트리(worktree)에 로컬 통합되어 있다. 이 통합에서 push, PR 생성, remote merge(원격 병합), tag, Release, deploy(배포)는 발생하지 않았고 `main`은 수정하지 않았다.

## G1~G8 구현 결과

- G1: 한국어 자동 자막 우선, 없거나 사용할 수 없을 때 제작자 한국어 자막 대체, 자동 번역·다른 언어·`live_chat` 제외, 원본 시간 검증과 검증 불가 구간의 로컬 Whisper 대체, 자막 provenance 저장. 선택 순서·제외 트랙 테스트와 세 대표 자동 자막 입력은 `PASS`이며 제품 snapshot provenance·Whisper 대체는 `HOLD`.
- G2: `자동(GPU 우선)`·GPU·CPU 모드와 프로필, GPU 근거 게이트, 실패 청크의 CPU 1회 대체, 재실행 시 상태 보존. G8 GPU 패키지는 `cublas64_11.dll` 누락으로 `BLOCKED`.
- G3: 반복·깨진 음성 인식 결과를 품질 경고로 보존하고 표시 결과에서 가리며, 선택 후보만 실행 ID·개정과 함께 다시 음성 인식.
- G4: FFmpeg·Whisper·채팅·미리보기·UI 단계를 분리한 자원 기록, 경고와 강제 중단 분리, 체크포인트·실패 이유 보존과 자식 종료.
- G5: 후보 `8/20/30`개와 기본 `20`개, 후보 pool과 화면 목록 동기화, evidence 품질 경고·점수 분리, 개정·기존 판정 보존.
- G6: 영상별 독립 작업과 순차 scheduler, 원자적 `queue.json`/`queue.prev.json`, 실행권 단일화, `INTERRUPTED` 수동 복구, 실패 작업이 다음 작업을 막지 않음, 삭제 순서와 실패 참조 보존.
- G7: 측정되지 않은 병렬 실행을 fail-closed하고 순차 전환 이유를 영속화. 구현·자동 테스트는 PASS이나 실제 병렬 안전성·총시간·자원 측정 전에는 기능을 제공하지 않음.
- G8: `6ecbd49`의 FFmpeg 패키지 증거와 NSIS/PE/해시(hash)/빌드 앱 진입점 결과를 `7c8b336`으로 로컬 통합했다. 로컬 NSIS 생성, PE 버전·크기·SHA-256, fresh 격리 경로의 빌드 앱 8초 생존 확인은 PASS이고 실제 설치 파일 설치·Windows 화면·updater 서명·공개 Release는 HOLD다.

## 자동 검증 (2026-08-18 기준선)

- `npm ci`: PASS.
- `npm test`: PASS — 49 tests, 3 files.
- `npm run build`: PASS — TypeScript + Vite, 1,793 modules.
- `cargo.exe test --manifest-path src-tauri/Cargo.toml`: PASS — 127 passed, 1 ignored.
- `cargo.exe test --manifest-path src-tauri/fixture-worker/Cargo.toml`: PASS — 6 passed.
- `npm run test:security`: PASS — 6 passed.
- `node --test scripts/archive-safety.test.mjs scripts/prepare-media-tools.test.mjs scripts/sample-disk-usage.test.mjs`: PASS — 11 passed.
- `git diff --check`: PASS (최종 문서·코드 확정 후).

## 패키지·진입점 결과

- `npm run tauri:build`의 Windows PowerShell 패키지 증거에서 새 FFmpeg 자산 다운로드·SHA-256·media-tools 준비와 release/NSIS 생성은 성공했다. updater 개인키가 없어 서명은 HOLD이며, fresh 격리 앱은 8초 생존 확인 직후 테스트 프로세스를 의도적으로 중단했으므로 정상 종료로 기록하지 않는다.
- `vod-scout.exe`: 16,270,848 bytes, SHA-256 `d29cbf3f2d55e993ef896ecddcc202b6586e0a335f8cc6692fc51dcca1ac2d2f`, PE ProductVersion/FileVersion `0.5.0`.
- `VOD Scout_0.5.0_x64-setup.exe`: 337,435,060 bytes, SHA-256 `2e8cddd19cb756951b58b8937c3171e4a9029cd7de78136bdcd04d745971d0f8`, PE ProductVersion/FileVersion `0.5.0`.
- fresh `VOD_SCOUT_E2E_DATA_DIR`에서 빌드 앱이 8초 생존했고 `instance.lock`·`queue.json`만 생성됐다. 생존 확인 직후 테스트 프로세스를 의도적으로 중단했으며 정상 종료가 아니다. 실제 설치 파일 설치·설치 후 실행·Windows 화면은 확인하지 않았고, 기존 설치 앱·사용자 데이터는 건드리지 않았다.
- 고정 FFmpeg asset: `https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-08-17-13-05/ffmpeg-n8.1.2-44-g7c533d0f86-win64-lgpl-shared-8.1.zip`, archive size 70,837,934 bytes, SHA-256 `681b9ca6d8f9be1e01d8873ad16f8a632f8a22b9653f1044837de6d5979b0fd6`.
- 실제 설치 파일 설치·설치 후 실행·Windows 화면, updater `.sig`, 공개 Release 자산은 생성·검증하지 않았으며 `HOLD`다.

## 문서 상태와 남은 HOLD

- 계획·릴리스·아키텍처·UI·테스트 계약은 현재 G1~G8 로컬 통합과 자동 검증 결과를 가리킨다. 제작자 자막 부재는 릴리스 차단 사유가 아니며, 제품 snapshot provenance, Whisper 대체, GPU, 설치·Windows 화면, 자원·장시간·병렬 측정은 `HOLD` 또는 `BLOCKED`다.
- 세 SRT의 start/end 범위·역순·중복 그룹·겹침·공백을 기록했지만 겹침·공백을 품질 `PASS`로 판정하지 않았고, 일정한 시간 오프셋·내용 품질·사람 판정은 `HOLD`다.
- 실제 GPU 백엔드 시험은 패키지의 `cublas64_11.dll` 누락으로 `BLOCKED`이며, CPU fallback probe 로그만 보존했다. HTTP 403이 검토 화면 전에 발생해 Windows 사용자 화면 흐름과 screen capture도 `HOLD`다.
- 1~8시간 resource/long-run 및 기존 v0.4.0과의 동일 입력 비교는 실행하지 않았다.
- G7 병렬 옵션은 사용할 수 없다.
- 실제 설치 파일 설치·Windows 화면, updater `.sig`, 공개 v0.5.0 URL/Release는 HOLD다. README에는 공개 v0.4.0 다운로드·Release 링크만 남긴다.

## 롤백

이 브랜치의 변경을 공개 main이나 기존 설치에 반영하지 않는다. 회귀가 확인되면 공개 v0.4.0 정본을 사용하고, 사용자 작업 폴더·설치 폴더를 삭제하거나 덮어쓰지 않는다. 버전업 기록과 패키지 산출물은 실제 결과만 갱신한다.

## 다음 정확한 작업

1. 외부 GPU 바이너리 추가가 승인되고 출처·SHA-256·라이선스가 고정될 때만 G8 패키지를 보완한 뒤 실제 GPU 시험을 진행한다.
2. YouTube HTTP 403이 해소된 승인 입력에서만 전체 후보·검토·삭제 흐름을 재시도한다. 같은 입력에 대한 추가 재시도는 계약 제한으로 하지 않는다.
3. updater 서명 개인키가 승인된 환경에 있을 때만 `.sig` 생성과 Release 자산 검증을 진행한다.
4. 실제 기준 영상·Windows UI·resource/long-run·parallel 측정과 위 HOLD/BLOCKED가 끝날 때까지 v0.5.0 공개 링크·Release를 만들지 않는다.
