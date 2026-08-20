# 테스트와 완료 기준

## v0.5.0-rc.1 Pre-release 최종 게이트

한 번만 실행할 범위는 integration의 짧은 GPU fixture, 기본 npm·Cargo 테스트와 build, 보안 검사, `git diff --check`, unsigned EXE·NSIS 생성, 새 임시 데이터 경로의 최종 EXE 화면 스모크다.

PASS 조건은 실제 GPU backend/device와 유효한 음성 인식 결과, 메인 화면, `0.5.0-rc.1`, Auto/GPU/CPU 설정 표시, 기존 설치·사용자 데이터 무변경이다. 장시간 E2E는 반복하지 않는다. 자동 GPU→CPU 전환은 `DEFERRED`이며 이번 Pre-release를 막지 않는다.

updater artifact는 만들지 않는다. `.sig`, `latest.json`, updater zip이 release 디렉터리나 GitHub Pre-release 자산에 있으면 실패다.

최종 결과는 npm 49, Rust `128 passed·1 ignored`, fixture-worker 6, 보안 6, production dependency audit 0 vulnerabilities, build·diff 검사, 실제 CUDA0 GPU fixture, unsigned EXE·NSIS와 CDP 화면 스모크가 모두 `PASS`다. CDP 증거는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-rc1-cdp-smoke-20260821-024212`, GPU 증거는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-rc1-gpu-20260821-020758`에 보존했다.

## 2026-08-21 이전 엄격 공개 게이트 기록

### 필수 게이트 판정

| 순서 | 게이트 | 결과 |
|---|---|---|
| 1 | ZJ `9b6de644-e40f-4130-bffb-301eab4a03a6` acquisition/analysis → `REVIEW_READY` | download `100%`, 후보 `20`, `18/18` units, GPU `12/12`, `previewPlayerReady=true`, `bodyVerified=true` `PASS` |
| 2 | ZJ 선택 후보 재인식 | `candidateRevision=0`, `recognitionRuns=[]`; 완료 증거 부족으로 현재 ZJ만 `HOLD` |
| 3 | JKY `ee834a3d-fde9-49c9-8cf8-ced9654e1c45` full E2E | download `100%`, `REVIEW_READY`, 후보 `20`, `22/22` units, GPU `16/16`, recognition `28ba3f98-8727-4517-868b-2a42f9091f51` `COMPLETED`, revision `1`, `PASS` |

player-ready 평가는 수정 후 PASS다. ZJ snapshot은 `C:\Users\myhan\AppData\Local\com.vodscout.app\e2e-requested-data-zjmp-integration\jobs\9b6de644-e40f-4130-bffb-301eab4a03a6\snapshot.json`, screenshot은 `C:\Users\myhan\AppData\Local\Temp\vod-scout-zjmp-integration-screen-m3unDv\review.png`다. JKY snapshot/data root는 `C:\Users\myhan\AppData\Local\com.vodscout.app\e2e-requested-data-jky\jobs\ee834a3d-fde9-49c9-8cf8-ced9654e1c45`, screenshot은 `C:\Users\myhan\AppData\Local\Temp\vod-scout-jky-full-20260820-231751\review.png`다. 과거 player-ready 실패 JSON/log는 2026-08-20 historical section에서 보존하며, 현재 판정에 재사용하지 않는다.

### Provenance·자동 검증

- Official latest yt-dlp nightly `2026.08.19.233000`: asset/digest `02bcc69a2a65a2af5da81a79356763522b611edc028c476e78c282735e28d442`, checksum file `6b2471fa596aaa588446fb0dfcf6025f8533d9dc931fa034b21c40c431473ce6`, source commit `594bd50c2c78ac432f81600d309fdc4e0a92d82c`, LICENSE `7e12e5df4bae12cb21581ba157ced20e1986a0508dd10d0e8a4ab9a4cf94e85c`, THIRD_PARTY `472aefe951c7db35e1657c1d13fd337140511ed6f2b329205105ad441c5a02b7`; single `check:yt-dlp` was `PASS` and is not rerun in this docs-only task.
- Final post-provenance checks: npm `49`, Rust main `128 passed·1 ignored`, fixture `6`, security `6`, standalone Node archive/media/sample `6/3/3`, build `PASS`, `git diff --check` `PASS`. Logs and exit codes: `C:\Users\myhan\AppData\Local\Temp\vod-scout-final-post-provenance-vxCIf4`.
- Package evidence remains EXE `16,279,040` bytes/SHA `4bcac97a30edb54be83b81711524ad4335cce5bdc315a44ff77d53121f52ec74`, NSIS `595,368,676` bytes/SHA `7c9e013794886fc220d82e1503d97277e5d79afca2b97f6ec7d367eab3041d3e`, PE `0.5.0`, Authenticode `NotSigned`; exact automatic GPU→CPU fallback, signing, install, and GitHub publication are `HOLD`, overall judgment is `HOLD`. No package rebuild follows this Markdown-only change.
- No GitHub publication, push, PR, tag, Release, remote merge, install, or additional E2E run is part of this final docs task.

## 2026-08-20 Wave 5 release-app 실제 검증

- stable `yt-dlp 2026.07.04`의 `android_vr`/`ANDR-V` client는 exact `298+251` 선택 후 HTTP `403`을 반환했다. official nightly `2026.08.18.122307`(source commit `yt-dlp/yt-dlp@5d5b634d8e6b41dc2891847a5ea7a5a3f569a28c`, asset SHA-256 `652e154bce7170070d0f26415c9a3c35c121f5a7903cb8cde6d31c4577517fb9`)은 `visionos` 경로 first-byte control에서 403 없이 도달했으며 release-app full transfer는 두 job 모두 100%였고 기술 수정 커밋은 `6aa2bc83c48835082b5f08ee14fab0f9570eb691`이다.
- ZJ release-app job `6251521b-6749-4415-9bf1-7eac826bae0f` (`ZJMpYThMksM&t=2017s`)는 download `100%`, `REVIEW_READY`, 후보 `20`, `18/18` units, GPU `12/12` completed였다. 자동 한국어 caption provenance는 `trackId=Korean`, SHA-256 `d74a0bab2029be6b1d33c27e66031838076b2fe658e13b910c3327e0a3f71562`, `quality=unverified`이고 검증 불가 구간은 local Whisper로 처리됐다.
- JKY release-app job `c8f80e03-33d7-4c3b-af5f-6f498be31f72` (`JKYmw9-xMIo&t=8463s`)도 download `100%`, `REVIEW_READY`, 후보 `20`, `22/22` units, GPU `16/16` completed였다. 자동 한국어 caption provenance는 `trackId=Korean`, SHA-256 `81dff33650b150069a03cd73db2aaf8d8e29682cdf4a80c9366e1b4e64cdb6cc`, `quality=unverified`이고 검증 불가 구간은 local Whisper로 처리됐다.
- 두 E2E는 이후 `scripts/e2e-local-cdp.mjs:279`의 동일 player-ready 검사에서 실패했다. screenshot·preview·후보 재인식·전체 UI는 `HOLD`/`BLOCKED`; 자동 GPU→CPU 제품 fallback은 안전하게 유발하지 못해 `HOLD`다. 증거는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-v050-public-gate-20260820\evidence-zjmp-retry\e2e-failure-2026-08-19T19-24-56-054Z-17056.json` 및 `.log`, `...\evidence-jky\e2e-failure-2026-08-19T19-39-18-521Z-24076.json` 및 `.log`다.
- Release build/Rust release/NSIS 본문은 성공했지만 전체 `tauri:build` rc `1`은 `TAURI_SIGNING_PRIVATE_KEY` 부재로 signing 단계에서 발생했다. EXE `16,279,040` bytes/SHA-256 `a26e472265e52bd48d02bab6fbee357efa99764dcc56d16f82aa705626fd9fba`, NSIS `595,405,262` bytes/SHA-256 `75d46c747eca1969c46f85160caf4cfb315b6e2485f9d62f54cbde38be1593ff`, PE `0.5.0`; runtime manifest schema 6 `51/51` + license `1/1`, security `6/6`, `npm audit --omit=dev` 0 vulnerabilities `PASS`, `cargo-audit` unavailable/not project gate. signing/install/public Release `HOLD`/`BLOCKED`; 1~8시간·사람 품질·GPU memory `HOLD`, G7 disabled.
- Measured automatic Korean VTT: ZJ duration `6847.121`, `2943` cues, first `57.640`, last `6829.639`; JKY duration `9590.061`, `4241` cues, first `51.199`, last `9566.720`. Both measured inverted/out-of-range/adjacent overlap/exact-time duplicate `0` and max positive cue gap `0`; constant time drift remains `UNVERIFIED`, with no acceptance threshold invented.
- Candidate display guard masked ZJ `20/20` and JKY `16/20` excerpts with `음성 인식 결과가 불확실해 원문을 표시하지 않습니다.`; visible repeated/broken pattern count was `0`. ZJ elapsed `662.728s`, final `1,371,590,406` bytes (`1.277 GiB`), media `1,344,021,572`, caption `308,005`, `122` files; JKY elapsed `784.380s`, final `2,048,005,882` bytes (`1.907 GiB`), media `2,019,916,819`, caption `476,641`, `154` files. These are `MEASURED_NO_THRESHOLD`; CPU/peak memory/GPU memory/temp peak were not sampled in these runs and retain prior/HOLD evidence.
- `SBOM.spdx.json` parses as SPDX-2.3, root `vod-scout@0.5.0`, `656` packages, `615,471` bytes, SHA-256 `418fd00061bc16a517a524691a8db6272e193c46dc47fba5f90bf012d177ed0a`. `SHA256SUMS.txt` and release-assets remain ungenerated because updater signing artifact is absent; no PASS is claimed.

## 2026-08-20 Wave 4 자원·패키지 측정

- 승인된 보존 30초 프로브(`probe-motion-30s.mp4`, SHA-256 `c20156488327d68e435120a599970ac03bc716aa55d163b57079f1a2dc5b54fc`)에서 직접 GPU/CPU를 측정했다. FFmpeg는 elapsed `29.755s`, CPU `0.094s`, WS `22,716,416`, private `20,803,584`, temp peak `4,393,690`, final `960,590`; GPU Whisper는 `2.710s`, `2.438s`, `273,334,272`, `1,148,428,288`, `4,395,267`, `48`; CPU Whisper는 `8.110s`, `14.953s`, `324,505,600`, `837,447,680`, `4,396,535`, `69`(순서: elapsed/CPU/WS/private/temp/final)이며 세 wrapper rc는 `0`이다. 세 JSON exitCode는 `null`; GPU `peakGpuBytesObserved=0`은 per-process sample 없음이므로 GPU memory는 `UNAVAILABLE`/`HOLD`이고, 1442 MiB는 whole-GPU snapshot이다. 모든 수치는 `MEASURED_NO_THRESHOLD`이며 JSON/SRT/log는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-wave4-20260820`에 있다.
- 공식 manifest 핀 재준비와 release runtime manifest 51/51 파일·해시 검증은 `PASS`다. lockfile v3 `npm.cmd ci` 후 Windows `npm.cmd run tauri:build`는 NSIS 생성까지 성공했지만 signing key 부재로 전체 rc `1`로 종료했으며 NSIS 생성·hash는 `PASS`, signing은 `HOLD`다. fresh 격리 data-dir 앱은 8초 생존 후 의도적으로 종료했다(`PASS`, 정상 종료 아님).
- 1~8시간 실입력은 승인된 안전한 로컬 증거를 사용할 수 없어 `HOLD`다. G7 병렬은 disabled/`HOLD`다. 기존 YouTube 403·player-ready·screenshot·자동 fallback은 재검증하지 않았다.

## 2026-08-20 Wave 3 측정 결과

- Production-protocol release no-cancel YouTube E2E: `JKYmw9-xMIo&t=8463s`는 metadata probe와 acquisition 진입 뒤 HTTP 403으로 `BLOCKED`; full command/stderr는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-e0b0c58-20260820\product-yt-dlp-command.txt` 및 `...\data-release-real\jobs\bfb8c79b-181a-4533-b4fd-ef5a0da29b75\tool-logs\yt-dlp.stderr.log`다. Pinned yt-dlp+Deno control의 HTTP 206/자동 한국어 자막 저장과 비교해 원인은 미확정이며 같은 실패를 반복하지 않는다.
- Release-app product GPU E2E는 `probe-motion-30s.mp4`(30.0 sec, SHA-256 `c20156488327d68e435120a599970ac03bc716aa55d163b57079f1a2dc5b54fc`)에서 GPU checkpoint completion과 non-empty Korean SRT를 확인했다. player-ready 단계 실패로 screenshot/전체 UI 검증은 `HOLD`다.
- Task-local `cublas64_11.dll` 제거 주입은 strict runtime manifest guard에서 중단되어 CPU fallback에 도달하지 않았다. Automatic GPU→CPU fallback gate는 `HOLD`; evidence는 `...\data-gpu-fault\jobs\d715e4c6-f9da-46a8-afd5-30ae635e69d0\snapshot.json` 및 `...\evidence-gpu-fault\`다.
- One independent intact-resource attempt with process-only `CUDA_VISIBLE_DEVICES=-1` was run; the app child environment allowlist stripped it, so checkpoint records GPU completion, `cpuFallback.status=PENDING`, and non-empty raw SRT only. The pre-fix candidate snapshot exposed raw token `띄웅`; it is not valid content, and the new display-safety rule masks this class of result. Evidence is `...\data-gpu-fallback-env\jobs\996ef279-3c47-4415-8feb-2d3de24b4bce\media-checkpoint.json` and `...\gpu-fallback-env.log`; automatic fallback remains `HOLD` and was not retried.
- Display-safety regression: with known media duration, a single <=2.5-second segment occupying <=20% of the input and containing <=2 Korean characters is masked; the same short one-word speech in an approximately 2-second whole input remains visible. `cargo.exe test --manifest-path src-tauri/Cargo.toml --lib masks_sparse_very_short_low_information_results_but_keeps_short_speech` reports `1 passed`.

v0.5.0 G1~G8은 `codex/v050-integration` 작업 트리(worktree)에 로컬 통합되어 있고 구현·자동 검증은 **PASS**다. origin/main 기준은 `eee71e04776a6179c289167596e9d82d52e94e13`(PR #18 반영), G8 패키지 증거 원본은 `6ecbd49`, 로컬 통합 병합은 `7c8b336`이다. GTX 1060 직접 GPU·CPU CLI 음성 인식은 **PASS**이며 30초 구성요소 자원 측정은 `MEASURED_NO_THRESHOLD`로 완료했다. 실제 설치 파일 설치·Windows 화면·updater 서명·공개 Release, 자동 GPU→CPU 제품 전환, 실제 YouTube/reference 전체 흐름, 1~8시간·동일 입력 자원 비교와 G7 병렬 측정은 **HOLD 또는 BLOCKED**다. G7 병렬 옵션은 disabled다.

G8 로컬 패키지 증거는 NSIS 생성, PE 버전·크기·SHA-256, fresh 격리 경로의 빌드 앱 8초 생존 확인까지 **PASS**다. 8초 생존 확인 직후 테스트 프로세스를 의도적으로 중단했으며 정상 종료가 아니다. 실제 설치 파일 설치·설치 후 실행·Windows 화면과 updater 서명·공개 Release는 확인하지 않았고, 이 통합에서 push·PR·remote merge(원격 병합)·tag·Release·deploy(배포)는 발생하지 않았으며 `main`은 수정하지 않았다. README의 공개 v0.4.0 다운로드·Release 상태는 유지한다.

## 자동 테스트

### 프런트엔드

- 시간 표시와 소스 축약
- 상태 라벨
- 경과 시간·ETA 계산과 표시
- TypeScript 프로덕션 빌드

### Rust Core

- 허용·거부 상태 전이
- 진행 단위 단조 증가
- 손상된 최신 스냅샷에서 이전 사본 복구
- SRT 시간 파싱
- 오디오·발화·채팅 후보 순위
- 후보 시간 겹침과 유사한 음성 인식 문장 중복 제거
- 영어 반복·무음 환각 필터
- 잘못된 UTF-8 바이트가 포함된 SRT 손실 허용 파싱
- 작업 용량과 요청한 UUID 작업만 삭제
- YouTube URL 허용 목록과 위장 호스트 거부
- yt-dlp 진행률 파싱과 영상·오디오 진행률 결합
- `.part` 파일을 완료 영상으로 오인하지 않음

### fixture-worker

- 정상 완료
- 제어된 실패
- 중간 충돌과 재개
- heartbeat 정지 감지
- 잘못된 JSON 이벤트 거부

### 실제 미디어 무창 통합

1. 11초 MP4 → ffprobe JSON
2. FFmpeg → 16 kHz mono WAV
3. whisper.cpp base → 실제 SRT 문장
4. Rust → RMS·후보 생성
5. 체크포인트 저장

### 제품 경로 무창 E2E

- Tauri `create_job/start_job` 실제 호출
- React DOM이 검토 화면으로 전환
- 실제 Whisper 문장과 채팅 움직임 점수가 후보에 포함
- 후보 겹침과 알려진 영어 반복 환각 0개
- 후보 영상 프록시 생성과 `<video>.readyState >= 1`
- 작업 용량, UTF-8 BOM CSV, 작업 삭제
- 영상 처리 중 `cancel_job`
- 10초 안에 `CANCELLED`
- 같은 작업 재개 후 `REVIEW_READY`
- 취소 활동, 음성 인식 결과 5개 구간, 체크포인트 보존
- 부모 앱 강제 종료 시 관찰 중인 ffprobe/FFmpeg/Whisper 자식 PID 소멸

### 실제 한국어 장시간 E2E

- 1시간 5분 29초 원본 → 10분 청크 7개
- 중간 체크포인트 3/7에서 재개 → `REVIEW_READY`, 13/13
- 음성 인식 결과 702개 구간, 채팅 움직임 785포인트, 후보 8개
- 후보 시간 겹침과 알려진 영어 반복 환각 0개
- ETA 표시, 플레이어 준비, CSV·용량·삭제 확인

### 실제 YouTube 무창 E2E

- 공개 단일 영상 URL → yt-dlp + Deno → 최대 720p 로컬 파일
- 다운로드 진행률 → `acquisition.json`과 완료 영상 저장
- 다운로드 직후 취소 → `CANCELLED` → 같은 작업 재개
- ffprobe·FFmpeg·Whisper → `REVIEW_READY`, 실제 음성 인식 결과와 후보 생성
- 삭제·사용 불가 영상 → 사용자 오류와 yt-dlp 진단 분리

무창 E2E에서만 `VOD_SCOUT_HEADLESS_E2E`와 로컬 CDP 포트를 사용한다. 배포 앱은 디버그 포트를 열지 않는다.

## 패키지 게이트

- NSIS 설치 파일 생성
- 설치 파일 SHA-256 기록
- 모델·FFmpeg DLL·whisper.cpp·yt-dlp·Deno·라이선스 포함
- 같은 release 빌드 실행 파일을 무창으로 실행해 실제 미디어 E2E 재검증
- 설치 후 시스템 PATH의 FFmpeg/Python/Node에 의존하지 않음
- 코드 서명 없음 표시

## 현재 HOLD인 검증

- 사람 기준 한국어 하이라이트 정확도
- v0.5.0 변경 뒤 1~2시간 처리 시간·피크 메모리·임시 파일 비교
- Windows 배율별 v0.3.3 UI 수동 회귀
- SmartScreen 신뢰도와 코드 서명
- 30분 이상 YouTube 다운로드 취소·재개와 봇 확인 발생률
- 실제 Whisper 음성 인식 도중 취소·재개
- GPU 음성 인식 장치별 시험
- 채팅 글자 인식·이야기 구간 탐색

v0.4.0의 승인된 약 8시간 53분 입력 전체 다운로드·분석은 `PASS`다. v0.5.0 변경 뒤 같은 기준의 재검증은 구현 후 별도 `HOLD`로 관리한다.

## v0.3.3 UI·검토 계획

화면 구현 담당은 버전 시작 승인 시 확정한다.

### 자동 테스트

- 점수·원본 시간·오디오·발화·채팅·판정 상태 정렬
- 같은 점수에서 원본 시간순 고정
- 정렬 변경 뒤 선택 후보와 판정 상태 유지
- `시스템 설정 사용`, `밝게`, `어둡게` 저장과 복원
- 업데이트 최신·새 버전·연결 실패 상태 분리
- 서명된 v0.3.2 설치본에서 v0.3.3 발견·설치·재실행
- 후보·맥락 시작·맥락 끝 타임코드 범위 검증
- 맥락 프록시 캐시 키와 중복 생성 방지

### 실제 화면 확인

- 밝은 화면·어두운 화면의 텍스트 대비와 포커스 표시
- 1280px 이상, 900~1279px, 899px 이하에서 후보 검토 동작
- 키보드와 화면 버튼이 같은 후보에 같은 행동 수행
- 실제 영상에서 후보 앞뒤 음성 인식 문장과 플레이어 시간이 일치
- 업데이트 서버 연결 실패 중에도 기존 작업 열기와 로컬 분석 가능

## v0.3.4 안정성·접근성 패치

### 실제 화면과 키보드

- 화면 오른쪽 위 설정 진입점에 톱니바퀴 아이콘과 `설정` 문구가 있으며 마우스·키보드로 열 수 있다.
- 1280px 이상, 900~1279px, 899px 이하에서 설정 버튼이 버전·Windows 로컬 상태와 겹치지 않는다.
- 밝은 화면·어두운 화면에서 입력 카드의 기본·마우스 올림·선택·비활성화·키보드 포커스 상태를 구분한다.
- 일반 크기 카드 제목·설명 글자는 배경과 4.5:1 이상의 대비를 목표로 측정하고 실제 화면 캡처도 확인한다.

### 실제 취소·재개

- YouTube 내려받기, FFmpeg 추출, Whisper 음성 인식 단계에서 각각 취소한다.
- 취소 요청부터 worker·자식 프로세스 종료·체크포인트 저장·`CANCELLED` 반영까지 단계별 시간을 기록한다.
- 실제 작업이 60초 이상 `취소 중…`에 머물면 `HOLD`다.
- 취소 완료 뒤 관련 자식 프로세스가 0개이고 같은 작업이 마지막 정상 체크포인트부터 재개되는지 확인한다.
- 강제 종료는 현재 작업의 확인된 프로세스 트리에만 적용되고 다른 앱이나 다른 작업 프로세스를 종료하지 않는다.

### updater와 측정

- 공개 v0.3.3 설치본에서 v0.3.4를 찾아 설치하고 재실행한다.
- 앱·실행 파일·`uninstall.exe`·HKCU 제거 프로그램 `DisplayVersion`이 모두 `0.3.4`인지 확인한다.
- 기존 작업·후보·체크포인트의 수와 핵심 파일 SHA-256을 업데이트 전후로 비교한다.
- YouTube 병합 중 열린 출력 파일을 포함해 내려받기·분석 WAV·미리보기·최종 작업 용량의 최대값을 기록한다.

## v0.4.0 최적화 측정

### O0 기준선

같은 입력을 v0.3.3 CPU 경로로 실행해 다음 값을 기록한다.

- 원본 길이·해상도·코덱·파일 크기
- CPU·메모리·GPU·저장 장치
- 분석 방식과 Whisper 모델
- 전체 처리 시간과 각 단계 처리 시간
- 피크 메모리와 GPU 메모리
- 임시 파일 최대 크기와 최종 작업 용량
- 음성 인식 구간·이야기 후보·반응 후보 수
- 취소·재개와 남은 자식 프로세스

기준선을 측정하기 전에는 성능 개선률이나 합격 수치를 확정하지 않는다.

### YouTube 입력(기존 경로)

v0.4.0의 기존 YouTube 다운로드·재개·취소 검증은 유지한다. 자막 선택·검증과 로컬 대체, 원본 자막 메타데이터는 아래 v0.5.0 순서 1에서 검증하며, 자막 검색 UI와 검색 결과의 원본 시각 이동은 v0.5.0 비범위다.

### 하드웨어 경로

| 경로 | 확인 내용 |
|---|---|
| CPU 전용 | 현재 기준 결과와 체크포인트 호환성 |
| 지원 GPU | 실제 GPU 백엔드 로드와 시험 음성 인식, 장치·메모리 기록, 같은 입력 완료 |
| GPU 메모리 부족 | 실패 청크만 CPU로 재처리 |
| 지원하지 않는 GPU | 오류 대신 CPU 경로 선택과 설명 표시 |
| 드라이버 오류 | 자식 프로세스 정리, 체크포인트 보존, CPU 재개 |

실제로 시험하지 않은 장치 조합은 지원 목록에 넣지 않는다.

- 기본 장치 `자동(GPU 우선)`에서 GPU 백엔드가 실제로 로드됐다는 로그와 실행 기록이 있을 때만 GPU PASS로 판정한다.
- CPU와 GPU가 같은 청크를 동시에 처리하지 않는지 PID·로그·체크포인트로 확인한다.
- CPU와 GPU 결과가 같은 시간 정보·체크포인트 형식을 사용하고 GPU 실패 뒤 기존 완료 청크를 다시 계산하지 않는지 확인한다.

### 1~2시간 회귀

- 말이 많은 방송, 조용한 방송, 게임 화면이 자주 바뀌는 방송을 분리해 시험
- 빠르게·균형·정확하게 설정별 자원과 결과 비교
- CPU와 지원 GPU의 처리 시간·결과 차이 기록
- 중간 취소와 앱 강제 종료 후 마지막 완료 단계부터 재개
- 디스크 공간 부족을 시작 전과 실행 중에 각각 처리
- 사람이 표시한 이야기 구간과 후보의 겹침·시작 오차·끝 오차 기록

### 8시간 빠른 분석

1. 실제 8시간 영상 또는 사용자가 승인한 동등한 입력을 사용한다.
2. 전체 저비용 색인이 처음부터 끝까지 만들어지는지 확인한다.
3. 음성 인식 예산이 영상의 한 시간대에만 몰리지 않는지 확인한다.
4. 처리 시간, 피크 메모리, GPU 메모리, 임시 파일, 최종 용량을 기록한다.
5. 취소·재개와 앱 종료·재실행을 각각 확인한다.
6. 완료 뒤 남은 FFmpeg·Whisper·GPU 자식 프로세스가 없어야 한다.
7. 전체 PCM·원본 프레임이 작업 폴더에 누적되지 않아야 한다.
8. 후보 타임코드와 시작·절정·마무리 근거가 원본과 일치해야 한다.

### 이야기 구간 품질

- 기준 영상마다 사람이 하나 이상의 시작·전개·절정·마무리 구간을 표시한다.
- 시스템이 제안한 이야기 후보와 사람 구간의 겹침, 시작 오차, 끝 오차를 측정한다.
- 이야기 후보 안에서 15~90초 반응 후보가 실제 핵심 순간을 포함하는지 별도로 판정한다.
- 반응이 크지만 맥락이 없는 후보와 조용하지만 중요한 후보를 구분해 오류 유형을 기록한다.
- 합격선은 기준 자료를 충분히 확보한 뒤 고정하며 단일 영상 결과로 일반화하지 않는다.

### 선택형 사용자 API

- API 기능을 끈 상태에서 외부 요청이 없고 기존 규칙 결과가 동일하게 완료되는지 확인한다.
- API 키가 설정 파일·작업 폴더·체크포인트·로그·내보내기·오류 메시지에 남지 않는지 확인한다.
- 원본 영상·음성·전체 음성 인식 결과가 아니라 선택한 상위 후보의 음성 인식 결과와 신호 요약만 전송되는지 확인한다.
- 전송 전에 제공처·모델·후보 수·예상 토큰과 비용·작업당 비용 한도를 표시하고 동의를 받는지 확인한다.
- 잘못된 키, 연결 실패, 시간 초과, 잘못된 응답, 비용 한도 초과에서 규칙 기반 결과와 사용자 판정이 보존되는지 확인한다.
- 응답에 없는 후보 ID나 새 타임코드가 포함되면 거부하고 기존 후보만 재정렬하는지 확인한다.
- 실제 유료 호출 검사는 전용 시험 키와 낮은 비용 한도를 사용하며 키와 인증 헤더를 검증 산출물에 남기지 않는다.

위 이야기·사용자 API 항목은 v0.4.0 이후 보류된 확장 기능의 검증 계약이다. 자막 검색 UI도 v0.5.0 완료 조건에는 포함하지 않는다.

## v0.5.0 품질·작업 대기열 검증

현재 상태: G1~G7 구현·자동 검증과 GTX 1060 직접 GPU·CPU CLI 음성 인식은 **PASS**. 30초 구성요소 자원 측정은 `MEASURED_NO_THRESHOLD`로 완료했다. 실제 YouTube/reference 전체 흐름, 자동 GPU→CPU 제품 전환, 1~8시간·동일 입력 자원 비교, Windows 사용자 화면 흐름과 G7 병렬 측정은 **HOLD 또는 BLOCKED**다.

### 1. YouTube 자막 확보·검증

- 일반 VOD에서는 한국어 자동 자막을 우선하고, 자동 자막이 없거나 사용할 수 없을 때만 제작자 제공 한국어 자막을 선택한다. 자동 번역·다른 언어·`live_chat`은 제외한다.
- 원본 영상 시간 기준을 유지하면서 각 시작 시각이 끝 시각보다 앞서고 두 시각이 원본 영상 길이 범위 안에 있는지, 겹침·중복·긴 공백·일정한 시간 오프셋을 검사한다.
- 실제 기준 영상 전에는 시간 허용 오차나 시간 오차 합격 임계값을 정하지 않는다. 자막이나 구간을 사용할 수 없거나 품질이 낮거나 검증할 수 없으면 로컬 Whisper로 대체한다.
- 원본 자막 파일, 제작자·자동 출처, 언어, 트랙 식별자와 파일 SHA-256이 작업 데이터에 저장되고 재개 뒤 재사용되는지 확인한다.
- 세 승인 URL은 자동 한국어 자막 우선 대표 입력으로 `PASS`이며, 제작자 자막이 없는 것은 `HOLD`나 릴리스 차단 사유가 아니다. 제품 snapshot provenance·Whisper 대체와 전체 사용자 흐름은 `HOLD`다.

### 2. GPU·CPU Whisper 실행과 사용자 제어

- `자동(GPU 우선)`·GPU·CPU 모드, `빠르게`·`균형`·`정확하게` 단계, CPU 사용량·스레드 수 제어가 작업 설정과 UI에 저장·복원되는지 확인한다.
- GPU 백엔드 로드와 시험 음성 인식 결과가 실제로 성공한 뒤에만 GPU PASS로 판정한다. 시험·실행 실패는 실패 청크만 CPU로 한 번 대체한다.
- CPU 전용·지원 GPU·지원하지 않는 GPU·GPU 메모리 부족·드라이버 오류를 검사하고 장치·모델·단계·처리 시간·전환 상태를 기록한다. GTX 1060 3GB direct GPU/CPU와 설정·profile/retry focused tests는 `PASS`였고, 자동 GPU→CPU 제품 전환과 화면 검증은 `HOLD`다. 입력·로그·해시는 `docs/DEBUGGING.md` Wave 2 기록을 따른다.

### 3. 기존 음성 인식 품질 안전장치와 선택 후보 재음성 인식

- 한 글자·짧은 단어·짧은 문장의 비정상 반복과 `�`를 불확실로 판정하고, 실제 반복 발언·웃음·감탄·노래는 일괄 제거하지 않는다.
- 불확실한 문장은 후보 제목에 사용하지 않지만 원본·오디오·화면 근거는 보존한다. G3 품질 안전장치 구현·자동 테스트는 `PASS`이며 실제 표본·사용자 흐름은 `HOLD`다.
- 사용자가 선택한 후보에만 재음성 인식을 제공하고 실행 ID·결과 개정·실패 이유를 저장한다. 완료·실패가 자동 재분석을 호출하지 않는지 확인한다. 구현·자동 테스트는 `PASS`, 사용자 흐름은 `HOLD`다.

### 4. 렉과 자원 제한

- FFmpeg, Whisper, 채팅 영역 디코딩, 미리보기, UI 반응을 분리해 같은 입력의 변경 전후 처리 시간·CPU·메모리·디스크·자식 프로세스를 기록한다.
- 측정 전 임의 합격선을 만들지 않고 경고와 강제 중단을 구분한다. 강제 기준 초과 시 현재 작업 자식만 종료하고 체크포인트·실패 이유 보존·`FAILED`를 확인한다.
- 취소·화면 이동·상태 확인과 취소 뒤 관련 자식 0개를 확인한다. 실제 측정은 `HOLD`다.

### 5. 후보 수와 내용 품질

- 후보 `8개`·`20개`·`30개` 설정과 기본 `20개`를 저장·복원하고 부족한 개수를 채우지 않는지 확인한다.
- 후보 점수와 내용 품질을 분리해 선택 이유·불확실한 이유, 깨진 문자·비정상 반복·근거 부족·중복의 경고·순위 하락·제외 효과와 게임·노래·웃음·조용한 대화 예외를 검사한다.
- 작성자 ID·고유 참여자 수·메시지 수 없이 고정 채팅 영역 움직임을 내부 평소 분포와 비교하고, 표본 부족 시 `확인 가능한 채팅 영역 움직임 없음`으로 처리한다. 구현·자동 테스트는 `PASS`, 사람 판정은 `HOLD`다.

### 6. 여러 영상 작업 대기열

- 영상 3개를 등록해 `대기 → 분석 중 → 검토 준비`와 실패 후 다음 작업 진행을 확인한다.
- 순서 저장·복원, 기본 실행 작업 1개, 단일 인스턴스 실행권, `INTERRUPTED` 수동 복구, 자동 재시작 금지를 확인한다.
- 실행 중 삭제는 취소→자식 0개→종료 상태 저장→작업 폴더 삭제→대기열 참조 제거 순서를 지키며, 실패 시 참조와 `삭제 실패`를 보존한다. 구현·자동 테스트는 `PASS`, 실제 흐름은 `HOLD`다.

### 7. 제한적 병렬 처리

- 기본값은 순차 처리이며 1~6 단계가 PASS인 뒤에만 같은 입력 묶음의 총시간·피크 자원을 비교한다.
- 경쟁이 적은 조합부터 시험하고 자원 부족 시 순차 전환 상태·이유를 저장하며 앱 재실행 뒤 자동 병렬화를 막는다. 구현·자동 테스트는 `PASS`이고 병렬 옵션은 실제 측정 전까지 사용 불가다. 실제 측정은 `HOLD`다.

### 실제 화면 확인

- 1280px 이상, 900~1279px, 899px 이하에서 자막 출처·GPU 제어·대기열·후보 수·품질 경고가 겹치지 않는지 확인한다.
- `음성 인식 결과 불확실`, `채팅 영역 움직임`, GPU→CPU 전환과 실패 이유를 색상만이 아니라 문구로 표시하고, 키보드·버튼으로 재음성 인식·대기 작업 이동·취소·재시작을 수행한다. 실제 화면은 `HOLD`다.

## 최적화 회귀 금지 조건

다음 중 하나라도 발생하면 더 빠르더라도 최적화 PASS가 아니다.

- 기존 작업이나 완료 체크포인트 손상
- 취소 뒤 자식 프로세스 잔존
- GPU 실패 뒤 작업 전체 초기화
- 사용자 동의 없는 외부 전송 또는 비용 발생
- 같은 입력·설정에서 후보 타임코드가 실행마다 불안정하게 변경
- 임시 파일 또는 메모리가 입력 길이에 비례해 제한 없이 누적
- CPU 전용 PC에서 핵심 분석 불가
- 실패·취소 작업, 후보 재분석 또는 GPU·병렬 대체 처리가 자동으로 반복됨
