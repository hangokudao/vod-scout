# VOD Scout 변경 이력

이 문서는 사용자에게 영향을 주는 업데이트를 기록한다. 각 릴리스는 기능 추가뿐 아니라 버그 수정, 보안 수정, 알려진 문제를 함께 적는다.

## 0.5.0 - 2026-08-21

### Changed

- YouTube는 영상 다운로드 전에 한국어 자동 생성 자막을 먼저 확인하고, 그 파일에 유효한 구간이 없을 때만 제작자 한국어 자막을 확인한다. 유효한 자막이 하나라도 있으면 그 자막만 후보 문구에 사용하고 Whisper를 실행하지 않는다.
- 두 한국어 자막에 유효 구간이 모두 없으면 `Whisper 설정 후 진행` 또는 `취소`를 선택할 때까지 영상 다운로드·음성 인식·후보 생성을 시작하지 않는다. 로컬 파일도 사용자가 설정을 확인하고 승인한 작업만 Whisper를 실행한다.

### Fixed

- YouTube 누적 VTT의 0.01초 화면 전환 구간과 반복된 앞 문장을 정리해 빈 구간이 `EmptyText` 오류로 쌓이거나 정상 자막을 Whisper로 바꾸는 결함을 수정했다.
- `UNVERIFIED`와 시간 맞춤 미확인을 경고로만 남기고, 정상 구간을 폐기하거나 빈 부분을 Whisper로 자동 보완하지 않는다. 반복 진단은 종류별 개수와 대표 사례로 줄여 표시한다.
- 자막·미디어 체크포인트 정책 버전을 올리고 Whisper 승인 여부와 설정을 저장해 이전 정책 결과를 새 정책 결과로 재사용하지 않는다.
- 시작·재개·재시도마다 모든 한국어 자동 자막 트랙을 먼저 확인하고, 유효 구간이 없을 때 모든 제작자 한국어 트랙을 확인한다. `FAILED` 또는 현재 영상과 출처가 맞지 않는 자막은 대화 내용으로 사용하지 않는다.
- 수동 시작과 자동 대기열이 같은 정책 검사를 사용하며, 이동된 `NEEDS_INPUT` 작업도 작업 ID로 승인·취소한다. 작업별 Whisper 설정을 다시 불러오고 저장된 GPU/CPU `STARTED`·`COMPLETED`·`FAILED` 시도는 자동으로 중복 실행하지 않는다.

### Validation

- 읽기 전용 결함 자료에서 자막 구간 2,943개와 잘못 기록된 0.01초 `EmptyText` 591개를 확인했으며 사용자 작업 파일은 수정하지 않았다.
- 프런트엔드 `54/54`와 TypeScript·Vite production build는 통과했다. Rust는 정책 회귀를 포함해 `136 passed·1 flaky·1 ignored`다. flaky 1개는 테스트 준비 단계에서 `cmd`의 `ping` 자식 프로세스를 발견하지 못한 Windows 환경 의존 fixture로 분류했으며, 실제 제품의 자식 프로세스 잔류 증거는 없어 Cargo 게이트는 `PASS_WITH_KNOWN_TEST_LIMITATION`이다. 장시간 YouTube E2E는 반복하지 않았다.

## 0.5.0-rc.1 - 2026-08-21

### Changed

- GPU runtime과 `자동(GPU 우선)`·GPU·CPU 모드를 유지한 unsigned GitHub Pre-release로 버전 정본을 `0.5.0-rc.1`에 맞췄다.
- 이 시험판은 updater artifact를 생성하지 않는다. `.sig`, `latest.json`, updater zip 없이 unsigned NSIS installer, SBOM, SHA-256 목록만 배포한다.

### Validation

- 기존 JKY 전체 E2E에서 실제 backend `whisper.cpp-gpu`와 완료된 유효 결과를 확인했다. ZJMp 기존 후보 화면·원본 미리보기 증거도 이번 사용자 흐름 증거로 인정하며 장시간 E2E는 반복하지 않는다.
- 최종 integration 테스트, GTX 1060 CUDA0 backend의 짧은 GPU fixture, unsigned EXE·NSIS, SBOM·checksums와 포커스를 사용하지 않은 CDP 화면 스모크가 통과했다. 최종 NSIS SHA-256은 `ab231a9619cbbd5c1d5c6e132262202e21cecd8db3ccdce255242e1ab51e3558`이다.

### Known issues

- GPU 실패 뒤 CPU 자동 전환은 실제 실패 조건에서 검증하지 않아 `DEFERRED`다. PASS로 기록하지 않는다.
- Authenticode 코드 서명이 없어 Windows SmartScreen 경고가 표시될 수 있다.

## Wave 5 canonical final validation - 2026-08-21

### Changed

- 공식 `yt-dlp/yt-dlp-nightly-builds` 최신 nightly `2026.08.19.233000`을 고정했다. Windows asset SHA-256과 GitHub asset digest는 `02bcc69a2a65a2af5da81a79356763522b611edc028c476e78c282735e28d442`, `SHA2-256SUMS` 파일 SHA-256은 `6b2471fa596aaa588446fb0dfcf6025f8533d9dc931fa034b21c40c431473ce6`, source commit은 `yt-dlp/yt-dlp@594bd50c2c78ac432f81600d309fdc4e0a92d82c`다. Unlicense SHA-256 `7e12e5df4bae12cb21581ba157ced20e1986a0508dd10d0e8a4ab9a4cf94e85c`와 `THIRD_PARTY_LICENSES.txt` SHA-256 `472aefe951c7db35e1657c1d13fd337140511ed6f2b329205105ad441c5a02b7`도 manifest와 일치한다.

### Validation

- player-ready 평가 수정 후 ZJ integration job `9b6de644-e40f-4130-bffb-301eab4a03a6` (`ZJMpYThMksM&t=2017s`)는 download `100%`, `REVIEW_READY`, 후보 `20`, `18/18` units, GPU `12/12`, `previewPlayerReady=true`, `bodyVerified=true`를 확인했다. 다만 `candidateRevision=0`, `recognitionRuns=[]`라 선택 후보 재인식 완료를 증명하지 못해 ZJ 현재 게이트만 `HOLD`다. snapshot은 `C:\Users\myhan\AppData\Local\com.vodscout.app\e2e-requested-data-zjmp-integration\jobs\9b6de644-e40f-4130-bffb-301eab4a03a6\snapshot.json`, screenshot은 `C:\Users\myhan\AppData\Local\Temp\vod-scout-zjmp-integration-screen-m3unDv\review.png`다.
- JKY full E2E job `ee834a3d-fde9-49c9-8cf8-ced9654e1c45` (`JKYmw9-xMIo&t=8463s`)는 download `100%`, `REVIEW_READY`, 후보 `20`, `22/22` units, GPU `16/16`, candidate revision `1`, 선택 후보 재인식 `28ba3f98-8727-4517-868b-2a42f9091f51` `COMPLETED`, result revision `1`, `failureReason=null`, 실제 backend `whisper.cpp-gpu`를 기록해 current full E2E `PASS`다. snapshot/data root는 `C:\Users\myhan\AppData\Local\com.vodscout.app\e2e-requested-data-jky\jobs\ee834a3d-fde9-49c9-8cf8-ced9654e1c45`, screenshot은 `C:\Users\myhan\AppData\Local\Temp\vod-scout-jky-full-20260820-231751\review.png`다.
- exact automatic GPU→CPU fallback은 직접 증명되지 않아 `HOLD`다. signing·install·GitHub publication도 `HOLD`이며, overall release judgment는 `HOLD` 하나로 유지한다.
- provenance 이후 최종 자동 검증은 npm 49, Rust 본체 `128 passed·1 ignored`, fixture-worker `6`, `npm.cmd run test:security` `6`, standalone Node `archive-safety 6·prepare-media-tools 3·sample-disk-usage 3` passed이며 build와 `git diff --check`도 `PASS`다. 전체 로그는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-final-post-provenance-vxCIf4`에 보존했다.

### Package gate

- 새 runtime을 포함한 unsigned package build는 EXE `16,279,040` bytes/SHA-256 `4bcac97a30edb54be83b81711524ad4335cce5bdc315a44ff77d53121f52ec74`, NSIS `595,368,676` bytes/SHA-256 `7c9e013794886fc220d82e1503d97277e5d79afca2b97f6ec7d367eab3041d3e`, PE `0.5.0`을 생성했다. Authenticode는 두 파일 모두 `NotSigned`이고 updater private key 부재로 signing은 `HOLD`다. Runtime manifest schema 6은 `51/51` runtime 및 `1/1` license hash, SBOM은 SPDX-2.3/root `vod-scout@0.5.0`/`656` packages다.

### Known issues

- ZJ는 선택 후보 재인식 증거 부족으로 `HOLD`이고, exact automatic GPU→CPU fallback·설치·updater 서명·공개 Release도 `HOLD`다. 이 canonical docs 작업에서는 GitHub publication, push, PR, tag, Release, remote merge를 하지 않았다.

## Wave 5 release-app product validation - 2026-08-20

### Added

- 모든 `REVIEW_READY` 화면 E2E가 검토 화면의 첫 후보에서 실제 `다시 음성 인식` 버튼을 누르도록 했다. `run-e2e-smoke.ps1 -ReviewExisting`는 기존 작업을 다시 분석하지 않고 불러오며, fresh full YouTube 흐름도 같은 새 개정·완료 상태·backend evidence·화면 완료 문구·스크린샷 검사를 수행한다.

### Fixed

- CDP E2E 실행기가 앱 포트가 열린 직후 page target이 아직 등록되지 않은 startup race를 최대 10초·250ms 간격으로 재시도하고, timeout 주 오류에 마지막 target의 title/url과 마지막 조회 오류를 함께 남기도록 했다. PowerShell readiness 계약과 제품 저장 범위는 변경하지 않았다.
- 무창 E2E 실행기의 `DataDirectory`를 `%LOCALAPPDATA%\com.vodscout.app\e2e-*` 아래로 정규화해 기존 Asset Protocol 허용 범위와 맞췄다. 제품 저장 경로나 Asset Protocol 범위는 넓히지 않았고, 플레이어 준비 실패 증거에 `video`, `readyState`, `networkState`, `errorCode`를 남기도록 진단을 보강했다.

### Validation

- stable `yt-dlp 2026.07.04`의 `android_vr`/`ANDR-V` client는 exact `298+251`을 선택한 뒤 media transfer에서 HTTP `403`을 반환했다. official nightly `2026.08.18.122307`(source commit `yt-dlp/yt-dlp@5d5b634d8e6b41dc2891847a5ea7a5a3f569a28c`, Windows asset SHA-256 `652e154bce7170070d0f26415c9a3c35c121f5a7903cb8cde6d31c4577517fb9`)은 `android_vr`를 제거하고 `visionos` 경로에서 first-byte control에 403 없이 도달했으며, release-app full transfer는 두 job 모두 100%에 도달했다. 이 first-transfer 원인·수정은 기술 수정 커밋 `6aa2bc83c48835082b5f08ee14fab0f9570eb691`에 고정했다.
- release-app 실제 ZJ job `6251521b-6749-4415-9bf1-7eac826bae0f`(`ZJMpYThMksM&t=2017s`)는 download `100%`, `REVIEW_READY`, 후보 `20`개, `18/18` units, GPU `12/12` completed를 기록했다. snapshot의 자막 provenance는 `source=automatic`, `language=ko`, `trackId=Korean`, SHA-256 `d74a0bab2029be6b1d33c27e66031838076b2fe658e13b910c3327e0a3f71562`, `quality=unverified`였고 검증할 수 없는 구간에는 local Whisper가 사용됐다.
- release-app 실제 JKY job `c8f80e03-33d7-4c3b-af5f-6f498be31f72`(`JKYmw9-xMIo&t=8463s`)도 download `100%`, `REVIEW_READY`, 후보 `20`개, `22/22` units, GPU `16/16` completed를 기록했다. snapshot의 자막 provenance는 `source=automatic`, `language=ko`, `trackId=Korean`, SHA-256 `81dff33650b150069a03cd73db2aaf8d8e29682cdf4a80c9366e1b4e64cdb6cc`, `quality=unverified`였고 검증할 수 없는 구간에는 local Whisper가 사용됐다.
- 두 기존 E2E 시도는 이후 모두 `scripts/e2e-local-cdp.mjs:279`의 같은 player-ready 검사에서 실패했다. 따라서 screenshot·preview·후보 재인식·전체 UI 흐름은 `HOLD`/`BLOCKED`이며, 자동 GPU→CPU 제품 fallback은 안전하게 유발하지 못해 `HOLD`다. 원본 증거는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-v050-public-gate-20260820\evidence-zjmp-retry\e2e-failure-2026-08-19T19-24-56-054Z-17056.json` 및 `.log`, `...\evidence-jky\e2e-failure-2026-08-19T19-39-18-521Z-24076.json` 및 `.log`다.

### Package gate

- release build·Rust release·NSIS 본문 생성은 성공했지만 전체 `tauri:build` rc `1`은 `TAURI_SIGNING_PRIVATE_KEY` 부재로 signing 단계에서 발생했다. `vod-scout.exe`는 `16,279,040` bytes, SHA-256 `a26e472265e52bd48d02bab6fbee357efa99764dcc56d16f82aa705626fd9fba`, PE `0.5.0`; NSIS는 `595,405,262` bytes, SHA-256 `75d46c747eca1969c46f85160caf4cfb315b6e2485f9d62f54cbde38be1593ff`, PE `0.5.0`이다. Runtime manifest schema 6의 `51/51` runtime 및 `1/1` license hash, security `6/6`, `npm audit --omit=dev` 0 vulnerabilities는 `PASS`이며 `cargo-audit`는 unavailable/not project gate다.

### Known issues

- local signing environment와 Windows Sandbox가 없어 설치·updater signature·공개 Release는 `HOLD`/`BLOCKED`다. 1~8시간·사람 기준 내용 품질·GPU memory는 `HOLD`이고 G7 병렬은 disabled다.

## Wave 5 official yt-dlp nightly provenance validation - 2026-08-20

### Changed

- `yt-dlp nightly 2026.08.18.122307`을 공식 `yt-dlp/yt-dlp-nightly-builds` release asset으로 고정했다. Windows asset SHA-256은 `652e154bce7170070d0f26415c9a3c35c121f5a7903cb8cde6d31c4577517fb9`이며, source commit `yt-dlp/yt-dlp@5d5b634d8e6b41dc2891847a5ea7a5a3f569a28c`, SHA2-256SUMS, Unlicense와 exact `THIRD_PARTY_LICENSES.txt` URL·SHA-256 provenance를 manifest와 준비 스크립트에 기록했다. Official README 기준 Windows PyInstaller 실행 파일은 `GPL-3.0-or-later` combined work이고 yt-dlp source는 Unlicense이며, 전체 third-party 고지를 동봉한다.

### Validation

- 같은 restricted env·cookies 없음·`youtube:skip=translated_subs`로 nightly 제품 metadata probe를 1회 실행해 exact `298+251`을 확인했다. 이어 같은 transfer argv에 `--test`만 추가한 first-byte control을 1회 실행해 `source.f298.mp4`와 `source.f251.webm` 각 10,241 bytes, Korean automatic caption VTT 10,241 bytes 저장까지 도달했다. `--test`의 잘린 입력을 FFmpeg가 처리하지 못해 전체 rc `1`이 되었지만 first-byte 도달은 `PASS`로 분리했다.
- `npm.cmd run check:yt-dlp`는 pinned/latest `2026.08.18.122307`, `latestNightlyVerified=true`, binary/checksum/LICENSE/THIRD_PARTY_LICENSES 해시와 source commit을 모두 `PASS`로 보고했다.

### Known issues

- stable `2026.07.04`의 `298+251`은 `ANDR-V` client로 선택되어 첫 media transfer에서 HTTP 403을 냈고, nightly `2026.08.18.122307`은 `android_vr` 제거 후 `visionos` client를 선택해 동일 restricted first-byte control에서 HTTP 403 없이 두 media stream과 Korean automatic-caption bytes에 도달했다. 이 stable ANDR-V → nightly visionos 차이는 first transfer의 확인된 원인·수정이며, 전체 product transfer 완료나 full ordinary YouTube E2E 성공을 뜻하지 않는다. full ordinary YouTube E2E는 `HOLD`다.

## Wave 4 resource/package validation - 2026-08-20

### Changed

- 승인된 30초 보존 프로브에서 FFmpeg `29.755s/0.094s/22,716,416/20,803,584/4,393,690/960,590`, GPU Whisper `2.710s/2.438s/273,334,272/1,148,428,288/4,395,267/48`, CPU Whisper `8.110s/14.953s/324,505,600/837,447,680/4,396,535/69`(elapsed/CPU/peak WS/peak private/temp peak/final job), wrapper rc `0`을 수집했다. 임계값은 만들지 않고 `MEASURED_NO_THRESHOLD`로 기록했으며 두 경로 모두 non-empty SRT와 backend 로그를 남겼다. GPU per-process memory는 sample 없음으로 `UNAVAILABLE`/`HOLD`다.
- 공식 manifest 핀 도구를 Windows Node에서 재준비하고 release runtime manifest의 51개 파일·해시를 재검증했다. Linux Node의 `tar.exe` 경로 실패와 Windows 재준비 성공 로그를 task TEMP에 보존했다.
- 불완전한 `node_modules`의 `tauri.cmd`를 lockfile v3 기준 `npm.cmd ci`로 task worktree 안에서만 복원했다. 이후 단일 Windows `npm.cmd run tauri:build`가 release·NSIS 생성까지 도달했다.

### Known issues

- NSIS 본문 생성은 `PASS`지만 updater private key가 없어 signing은 `HOLD`다. 1~8시간 실입력은 승인된 안전한 로컬 증거가 없어 `HOLD`이며 G7 병렬은 disabled/`HOLD`다.

## Wave 3 validation correction - 2026-08-20

### Fixed

- Production-protocol release build에 `custom-protocol` feature를 명시해 실제 앱 프로토콜로 검증할 수 있게 했다.
- 실제 whisper.cpp GPU 로그의 `loaded CUDA backend`를 positive CUDA backend evidence로 인정하고 회귀 테스트를 추가했다.

### Known issues

- 첫 valid production-app no-cancel YouTube flow는 metadata probe 성공 뒤 transfer HTTP 403으로 중단됐다. pinned yt-dlp/Deno control은 HTTP 206과 Korean automatic-caption save에 성공했으므로 product 403 원인은 미확정 상태로 유지한다. full command/stderr evidence는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-e0b0c58-20260820\product-yt-dlp-command.txt`와 `...\data-release-real\jobs\bfb8c79b-181a-4533-b4fd-ef5a0da29b75\tool-logs\yt-dlp.stderr.log`다.
- Release-app GPU checkpoint success는 확인했지만 player-ready 검사 실패로 screenshot과 전체 화면 흐름은 `HOLD`다. task-local missing-cuBLAS injection은 integrity guard에서 중단되어 자동 GPU→CPU fallback 성공은 확인하지 못했다.
- Intact-resource process-only `CUDA_VISIBLE_DEVICES=-1` fallback 시도는 child command environment allowlist로 변수가 제거되어 GPU 성공으로 끝났다(`cpuFallback=PENDING`). 추가 fallback 시도는 하지 않았고 자동 전환은 `HOLD`다.
- Raw Whisper evidence remains stored, while candidate/UI display now masks only a sparse very-short low-information result using known media duration and speech coverage; a whole ~2-second short utterance remains visible. The regression test covers both cases without token-specific logic.

## 0.5.0 local candidate - 2026-08-18

상태: **G1~G7 구현·자동 검증 PASS · G8 NSIS/PE/hash/격리 앱 실행 PASS · 실제 입력/장치/UI/자원·장시간·서명/공개 패키지 HOLD**

### Added

- G1~G4의 자막 provenance·GPU/CPU 대체·음성 인식 품질·자원 제한 상태를 작업 데이터에 기록한다.
- G5 후보 `8/20/30`개 설정, 후보 pool/evidence 분리, 품질 경고와 후보 개정·판정 보존을 통합했다.
- G6 여러 영상의 독립 작업·순차 대기열·실행권·`INTERRUPTED` 복구·작업별 삭제 경계를 통합했다.

### Changed

- 로컬 소스·package lock·Cargo lock·Tauri 설정·release notes·installer workflow/helper 기대 버전을 `0.5.0`으로 맞췄다.
- G7은 측정되지 않은 병렬 처리를 fail-closed하고 순차 처리로 고정한다. 실제 측정 전 선택 항목은 제공하지 않는다.

### Fixed

- YouTube 일반 VOD의 한국어 자막 선택 순서를 한국어 자동 자막 우선으로 바꾸고, 자동 자막이 없거나 사용할 수 없을 때만 제작자 한국어 자막을 사용하도록 맞췄다. 자동 번역·다른 언어·`live_chat`은 계속 제외하며, 제공되지 않은 트랙 식별자를 새로 만들지 않는다.
- whisper.cpp v1.9.1 GPU archive에 빠진 `cublas64_11.dll`·`cublasLt64_11.dll`을 공식 NVIDIA CUDA cuBLAS redistributable `11.11.3.6`에서 SHA-256 검증 후 private GPU runtime에 포함하고 CUDA Toolkit license notice를 함께 패키징했다. 직접 GPU·CPU 음성 인식 결과는 PASS이며 자동 GPU→CPU 제품 전환과 Windows 화면은 아직 HOLD다.
- 후보 pool과 화면 목록의 동기화가 후보 수 변경·정렬·수동 재음성 인식 뒤에도 기존 판정을 잃지 않도록 했다.
- 대기열 저장 실패·복구·실행권·실패 작업의 다음 작업 진행·실행 중 삭제 순서를 닫아 두었다.
- 404가 된 FFmpeg autobuild 핀을 공식 `autobuild-2026-08-17-13-05` 자산 URL·archive·SHA-256으로 교정했다.
- 검증 스크립트가 음성 중심 후보의 `chatScore: null`을 실패시키고 화면 문구와 반복 `bootstrap` 조회를 취소 완료로 오인하던 문제를 고쳤다. 저장된 작업 스냅샷 기반 취소 폴링, 선택적 `--require-chat-score`, 오류 증거 보존, 앱 경로·스크린샷·자식 프로세스 정리를 추가했다.

### Security

- 새 외부 AI·유료 API·API 키 저장·원본 미디어 전송 경로를 추가하지 않았다.
- archive/media-tool/경로·자식 프로세스·체크포인트 경계 자동 테스트를 통과했다.

### Known issues

- 실제 YouTube/reference-video, Windows UI, resource/long-run, parallel measurement는 실행하지 않아 `HOLD`다. 직접 GPU·CPU CLI는 `PASS`이며 자동 GPU→CPU 제품 전환은 `HOLD`다.
- 승인 URL의 기존 재개 흐름에서 YouTube HTTP 403이 발생했고, 이번 재검증 상한에 따라 같은 E2E는 재시도하지 않았다. 고정 yt-dlp/Deno의 signed URL 범위 요청과 자동 자막 저장 control은 HTTP `206`으로 성공했지만 기존 앱 403은 재현하지 못했으므로 원인은 미확정이며 제품 경로 403 수정은 검증되지 않았다. 새 ordinary no-cancel 제품 E2E는 acquisition 전 로컬 CDP/Tauri IPC 평가에서 실패해 앱 흐름 검증이 `BLOCKED`다. 제작자 자막 부재는 `HOLD`나 릴리스 차단 사유가 아니다. 직접 GPU·CPU CLI는 PASS지만 자동 GPU→CPU 제품 전환·Windows 화면·resource/long-run은 `HOLD`다.
- 로컬 NSIS installer와 핵심 EXE의 크기·SHA-256·PE 버전, fresh 격리 데이터 경로 앱 실행은 `PASS`다. updater 개인키 부재로 `.sig`와 공개 v0.5.0 자산은 `HOLD`다.
- 자동 검증: npm 49, Rust 128 passed·1 ignored, fixture 6, security 6, archive/media-tool 11 passed. `npm audit`의 개발 의존성 high 1건은 제품 경로 취약점으로 단정하지 않는다.

## 0.4.0 - 2026-08-08

### Added

- 체크포인트 schema 4에 입력 지문·크기·런타임 해시·언어·후보 계산 버전을 기록하고, 호환되지 않는 중간 결과만 다시 계산한다.
- 분석 범위 밖이거나 음량·대화 근거가 없는 후보를 제외하고 마지막 정상 체크포인트 세대를 보존한다.
- YouTube 미디어 전송 전에 선택 스트림 메타데이터(용량·길이)만으로 다단계 저장 공간 계획을 세운다. 내려받기 피크와 이어지는 분석 workspace를 볼륨별로 반영하고, 동시 필요는 합산·순차 단계는 최댓값으로 계산한다. 용량·길이·여유 공간·계산 오버플로를 알 수 없으면 전송을 시작하지 않고 한국어로 조치를 안내한다.

### Fixed

- 읽기 전용 Actions 토큰에 초안 Release가 보이지 않아 설치 검사가 시작 전에 멈추던 문제를 고쳤다. 수동 설치검사에만 초안 조회 권한을 주고, 정확한 태그의 Release와 설치 파일 하나만 선택해 인증 다운로드한다.
- 호환되지 않는 미디어 체크포인트를 버린 뒤 작업 진행 정보가 앞서 있으면 재개가 멈추던 문제를 고쳤다. 작업 설정은 유지하고 미디어 중간 결과만 다시 계산한다.
- 내려받기 직전 가드가 download 폴더 피크만 보던 한계를 고쳤다. home/temp/job 볼륨과 분석 workspace(`estimate_analysis_workspace_bytes`)를 한 플래너로 묶고, 동일 볼륨 합산은 `aggregate_required_bytes_by_volume` 생산 경로로 검증한다.
- 메타데이터 조회에서 고른 정확한 `format_id` 조합을 실제 미디어 전송에 고정하고, 정확한 `filesize`만으로 공간 계획을 세운다. `filesize_approx`만 있거나 크기·포맷이 불명이면 전송 전에 중단한다.
- 메타데이터 probe stdout/stderr에 상한을 두고 초과 시 자식을 정리하며, 원시 JSON·stderr 대신 duration·format_id·filesize 등 최소 구조화 로그만 남긴다. 네트워크·로컬 환경·도구 실행·안전 용량 계산 불가 안내를 분리한다.
- 장시간 디스크 샘플러(`scripts/sample-disk-usage.mjs`)가 측정 대상 트리를 수정하지 않음을 체크포인트 교체 스모크로 고정했다. 표본 출력은 대상 밖이어야 하며, 실행 중 `media-checkpoint` live→`.prev` 교체가 방해받지 않는다.

### Validation

- PR #13 exact HEAD `e18b73efcb0ea40be812b7da12572e1207854863`에서 자동·보안 테스트, 실제 저용량 차단, 짧은 전송, 장시간 전체 다운로드·분석, 취소·재개·체크포인트와 자식 프로세스 정리를 확인했다.
- 장시간 전체 작업은 `REVIEW_READY`, 후보 8개, 약 4,004.51초로 완료됐고 PR #13은 `16c35f2dfa601790689d7295ceaea12af42169b8`로 main에 squash 병합됐다.
- 초안 Release installer smoke run `31240405719`에서 0.4.0 설치·권한·빌드 사용자 경로 부재·내장 파일 28개 해시·실행·재실행을 확인했다.
- `v0.4.0` Release를 Latest로 공개하고 설치 파일·서명·`latest.json`·SBOM·체크섬의 토큰 없는 직접 다운로드와 해시 일치를 확인했다.

### Security

- 메타데이터 원문과 진단 출력 전체를 저장하지 않고, 외부 AI·유료 API·API 키 저장 경로를 추가하지 않았다.
- `npm audit` high 1건은 Vite→PostCSS의 개발용 `nanoid@3.3.16` 경로이며 제품 실행 코드에서 취약 조건인 사용자 정의 0길이 생성기를 호출하지 않는다.

### Known issues

- exact HEAD의 순간 임시 파일 최대값은 다시 측정하지 않았다. 같은 승인 영상의 기존 측정값과 이번 최종 작업 크기를 릴리스 기록에 함께 남긴다.
- YouTube가 후속 재시도에서 봇 확인을 요구할 수 있다. 앞선 exact HEAD 성공과 로컬 분석 결과에는 영향을 주지 않는다.
- Windows Authenticode 인증서가 없어 첫 설치에서 SmartScreen 경고가 표시될 수 있다. updater 서명은 별도 필수 게이트다.

## 0.3.4 - 2026-08-06

### Added

- 설정 진입점에 톱니바퀴 아이콘과 `설정` 문구, 버튼 경계·포커스 상태를 표시한다.
- 취소 중 종료 대상 안내와 현재 작업 범위 자식 프로세스 종료 감독을 둔다.
- 내려받기·병합 중 작업 폴더 용량을 읽기 전용으로 표본 수집하는 `scripts/sample-disk-usage.mjs`를 문서화했다.

### Changed

- 어두운 화면 입력 카드 배경을 고정 밝은 반투명 색 대신 테마 변수로 맞춘다.
- 제품·릴리스 버전 정본과 설치·updater 자산 이름·workflow 기대 버전을 `0.3.4`로 정렬한다.

### Fixed

- 취소 요청이 디스크 저장보다 늦게 반영되던 순서를 바로잡았다.
- 응답하지 않는 자식 프로세스 트리가 한없이 기다려 취소가 끝나지 않을 수 있던 경로를 제한 시간 종료로 줄였다.

### Security

- 프로세스 종료 범위를 현재 작업의 확인된 자식 트리로 제한한다.
- 새 외부 AI·API 전송 경로와 API 키 저장 경로를 추가하지 않았다.
- Windows Authenticode 인증서는 없어 설치 EXE·앱 실행 파일 코드 서명은 계속 `HOLD`다.

### Known issues

- 공개 Release `v0.3.4`(exact `a341bae…`, Actions `31057676958`, 5개 자산·minisign·인앱 v0.3.3→v0.3.4·DisplayVersion `0.3.4`·작업 15개·데이터 파일 2,087개 보존)는 검증 완료다. 상세·해시는 `docs/V0.3.4-RELEASE.md`, `BUILD-MANIFEST.md`.
- 실제 YouTube 취소·재개와 전체 병합 종료 디스크 피크는 측정 완료다(취소 약 1.4s/3.4s, peak 약 13.08 GiB · 최종 약 6.58 GiB · peak−final 임시 약 6.50 GiB). Whisper 음성 인식 중 취소는 아직 `HOLD`다.
- Windows Authenticode 인증서가 없어 설치 EXE·앱이 `NotSigned`이며 SmartScreen 경고가 표시될 수 있다. 인증서 구매·생성은 하지 않았고 `HOLD`다. updater minisign 경로는 PASS다.
- 과거 공개 v0.3.2→v0.3.3 업데이트에서 HKCU `DisplayVersion`이 `0.3.2`로 남았던 근본 원인은 확정하지 않았다(`HOLD`). 공개 v0.3.3→v0.3.4 인앱 경로에서는 `DisplayVersion`이 `0.3.4`로 맞춰졌다.

## 0.3.3 - 2026-08-05

### Added

- 후보 정렬 기준 6가지와 정렬 후 선택 후보 유지
- 시스템 설정·밝게·어둡게 화면 설정과 저장
- 후보 앞뒤 맥락의 원본 타임코드·음성 인식 문장·바로가기
- 업데이트 확인 상태를 최신·새 버전·설치 대기·연결 실패로 구분

### Changed

- 후보 ID를 시작·끝 원본 구간 기반으로 고정해 같은 입력을 다시 열어도 선택 상태를 유지한다.
- 맥락 프록시 캐시에 작업·후보·원본 fingerprint·구간·프록시 종류를 포함한다.

### Fixed

- 같은 시작 초를 가진 후보가 선택 상태를 공유할 수 있던 문제를 구간 기반 ID로 수정했다.
- 후보·맥락 미리보기의 임시 이름이 `.mp4.tmp`가 되어 FFmpeg가 MP4 출력 형식을 고르지 못하던 문제를 `.tmp.mp4` 이름으로 수정했다.

### Security

- 외부 AI·API 전송 경로와 새 API 키 저장 경로는 추가하지 않았다.

### Known issues

- 승인된 8시간 53분 실제 입력의 빠른 분석·체크포인트 재개·후보와 맥락 재생을 확인했다. 공개 v0.3.2에서 v0.3.3으로 실제 업데이트·재실행하고 기존 작업과 체크포인트 보존도 확인했다.
- Windows Authenticode 인증서가 없어 첫 설치에서 SmartScreen 경고가 표시될 수 있다.
- 실제 업데이트 뒤 앱과 실행 파일은 `v0.3.3`이지만 Windows 제거 프로그램 레지스트리의 `DisplayVersion`은 `0.3.2`로 남는다. 원인은 `HOLD`다.
- YouTube 내려받기 병합 중 열린 출력 파일을 측정 도구가 읽지 못해 그 순간의 정확한 최대 임시 용량은 `HOLD`다.
- 다크 모드의 새 작업 화면에서 선택하지 않은 입력 카드가 밝은 배경과 밝은 글자로 표시돼 내용을 읽기 어렵고 비활성화된 항목처럼 보인다. 수정과 밝은 화면·어두운 화면 회귀 검증은 `HOLD`다.

## 0.3.2 - 2026-08-02

### Added

- 장시간 영상을 위한 `빠른 분석`, `구간 지정`, `전체 정밀 분석`
- 분석 모드·원본 fingerprint·실제 runtime SHA-256·전사 backend·채팅 ROI·ranker를 기록하는 `pipeline-provenance.json`
- 저장된 전체 작업의 최근 시각·용량·선택 삭제·전체 삭제
- GitHub Releases 기반 안정 버전 자동 확인, 수동 확인, 서명 검증 설치·재시작·실패 재시도
- Apache-2.0 라이선스, 기여 가이드, 보안 신고 정책, Windows 공개 릴리스 워크플로

### Changed

- ETA와 체크포인트를 실제 분석 모드·범위·전사 예산 기준으로 계산하고, 설정이 달라지면 이전 미완료 체크포인트를 안전하게 무효화한다.
- 빠른 분석은 전체 길이의 20%, 최소 30분·최대 120분을 10분 청크로 시간대별 분산 전사한다.
- 채팅 움직임 raw frame은 전체 메모리에 누적하지 않고 프레임 단위로 읽고 버린다.
- `yt-dlp 2026.07.04`를 control·최신 안정판·실제 번들 버전으로 교차 확인하고 새 버전이 발견되면 릴리스를 중단한다.

### Fixed

- `=`, `+`, `-`, `@`로 시작하는 CSV 셀이 스프레드시트 수식으로 실행될 수 있던 문제를 수정했다.
- CSV 저장 경로를 프런트엔드 IPC 입력으로 받지 않고 Rust 백엔드의 네이티브 저장 대화상자에서만 선택한다.
- 분석 모드·범위가 달라진 뒤 이전 전사와 후보가 섞일 수 있던 체크포인트 재사용을 수정했다.
- 손상된 snapshot을 가진 UUID 고아 작업이 전체 삭제에서 빠지던 문제와 snapshot ID 불일치 경계를 수정했다.
- 날짜에 따라 내용이 바뀌는 FFmpeg `latest` URL을 불변 autobuild URL과 asset SHA-256으로 교체했다.
- 깨끗한 GitHub Actions runner에서 Rust 검증 전에 fixture sidecar를 만들지 않아 릴리스가 중단되던 문제를 수정했다.
- Tauri Action이 만든 설치 파일 이름과 설치 스모크 workflow의 다운로드 패턴이 달랐던 문제를 수정했다.

### Security

- FFmpeg·Whisper·yt-dlp·Deno 자식 프로세스 환경을 초기화해 API 키 등 부모 비밀값을 전달하지 않는다.
- FFmpeg 입력 프로토콜을 로컬 `file,crypto,data`로 제한한다.
- FFmpeg·Whisper의 모든 EXE·DLL과 모델·yt-dlp·Deno 파일 목록·SHA-256을 빌드 manifest와 실행 시 검증한다.
- 압축 해제 전에 절대·상위·드라이브·UNC 경로를 거부하고 추출 뒤 심볼릭 링크를 거부한다.
- Asset Protocol을 UUID 작업의 `review-clips/*.mp4`로 축소했다.
- Tauri updater 개인키는 저장소 밖과 GitHub Actions Secrets에만 두고, 공개키만 앱에 포함한다.

### Known issues

- Windows Authenticode 인증서가 없어 첫 설치에서 SmartScreen 경고가 표시될 수 있다.
- GPU 전사, 채팅 OCR·자동 ROI, LLM 재순위·개인화는 아직 지원하지 않는다.
- 8시간 실제 영상 회귀와 8시간 전체 정밀 분석은 `HOLD`이며 지원을 주장하지 않는다.
- v0.3.2가 최초 updater 탑재 버전이므로 이전 서명 버전에서의 실제 인앱 교체는 다음 patch 릴리스에서 검증한다.

## 0.3.1 - 2026-08-02

### Added

- 후보 클릭 시 원본 후보 구간을 재생하는 앱 내 플레이어
- 경과 시간과 예상 남은 시간
- 작업별 용량 표시·삭제, 타임코드 복사, CSV 내보내기
- 화면 오른쪽 영역의 채팅 움직임 신호

### Changed

- Whisper 전사를 한국어로 고정하고 오디오·발화·채팅 신호를 결합하도록 후보 순위를 개선했다.
- 시간 중첩과 전사 유사도를 함께 사용해 중복 후보를 제거한다.

### Fixed

- Whisper SRT에 잘못된 UTF-8 바이트가 포함되면 장시간 작업이 중단되던 문제를 손실 허용 파싱으로 수정했다.
- 무음 구간의 반복 영어 문구가 한국어 후보에 남는 환각을 필터링했다.
- 서로 겹치는 후보와 동일 전사 후보가 함께 노출되는 문제를 수정했다.

### Security

- 작업 삭제 대상을 현재 UUID 작업 폴더로 제한하고 실행 중 삭제를 차단했다.
- asset protocol 접근 범위를 앱 작업 폴더로 제한했다.
- yt-dlp, Deno, FFmpeg, whisper.cpp 모델을 고정 URL과 SHA-256으로 준비한다.

### Known issues

- 설치 파일은 코드 서명되지 않았다.
- GPU 전사, 채팅 OCR, 자동 ROI, LLM 재순위는 구현되지 않았다.
- 실제 YouTube v0.3.1 재회귀와 2시간·8시간 검증은 `HOLD`다.
