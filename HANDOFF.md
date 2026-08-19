# VOD Scout 인계서

현재 게이트: **v0.5.0 validation-fixes · 자막 선택 순서·E2E 도구·자동 검증 PASS · release-app GPU checkpoint PASS · 자동 GPU→CPU 제품 전환·player-ready/screenshot·설치·updater 서명/공개 Release HOLD · 일반 YouTube 무취소 흐름은 product-path HTTP 403 원인 미확정·수정 미검증**

## Wave 4 자원·로컬 패키지 검증 (2026-08-20)

- 기존 YouTube/플레이어/E2E/fallback은 재실행하지 않고 task TEMP `C:\Users\myhan\AppData\Local\Temp\vod-scout-wave4-20260820`에서 승인된 보존 `probe-motion-30s.mp4`(30.0초·3,427,504 bytes·SHA-256 `c20156488327d68e435120a599970ac03bc716aa55d163b57079f1a2dc5b54fc`)만 사용했다.
- 직접 GPU/CPU는 모두 non-empty SRT를 생성했다. FFmpeg는 elapsed `29.755s`, CPU `0.094s`, peak working set `22,716,416`, peak private `20,803,584`, temp-scope peak `4,393,690`, final job `960,590`, wrapper rc `0`이다. GPU Whisper는 elapsed `2.710s`, CPU `2.438s`, peak working set `273,334,272`, peak private `1,148,428,288`, temp-scope peak `4,395,267`, final job `48`, wrapper rc `0`이며 48-byte SRT SHA-256 `71aca4471d2f8b60f8fee3665378f93851650831ab0334cc05bd516ffac89b64`를 남겼다. CPU Whisper는 elapsed `8.110s`, CPU `14.953s`, peak working set `324,505,600`, peak private `837,447,680`, temp-scope peak `4,396,535`, final job `69`, wrapper rc `0`이며 69-byte SRT SHA-256 `346f8c5f7783d976fce352249764fbbd3a002b1a66dda35746b1e804d1de7bb0`를 남겼다. 세 metric JSON의 `exitCode`는 `null`이고 wrapper 결과 로그가 rc `0`을 증명한다. GPU metric의 `peakGpuBytesObserved=0`은 per-process sample 없음이므로 GPU memory는 `UNAVAILABLE`/`HOLD`; 별도 `nvidia-smi` 1442 MiB는 whole-GPU snapshot이며 app peak이 아니다. 모든 수치는 `MEASURED_NO_THRESHOLD`다.
- Windows Node의 공식 manifest 핀 재준비와 release runtime manifest schema 6의 51/51 파일·해시 검증은 `PASS`다. Linux Node의 `tar.exe` 경로 실패는 `...\logs\prepare-media-tools-linux-failure.log`에 보존했다.
- lockfile v3 기준 `npm.cmd ci`(rc 0) 뒤 단일 Windows `npm.cmd run tauri:build`는 NSIS 생성까지 진행했고 전체 rc `1`로 종료했다. NSIS 본문 생성은 `PASS`; updater private key 부재로 signing만 `HOLD`다. `vod-scout.exe`는 16,274,944 bytes/SHA-256 `8754dc944d8f685195425bb4d3698c8992225888b2b95e9ed484283b7868cced`, PE `0.5.0`; NSIS는 595,736,201 bytes/SHA-256 `cd024e2d4523c34f4795c0e3d5bca1f72edca82b89d294033194fa465624ca36`, PE `0.5.0`이다.
- fresh 격리 데이터 경로 앱은 8초 생존했고 `instance.lock`·`queue.json` 2개를 생성한 뒤 테스트 프로세스를 중단했다(정상 종료 검증 아님). 1~8시간 입력은 승인된 안전한 로컬 증거가 없어 `HOLD`; G7 병렬은 disabled/`HOLD`다. 자동 GPU→CPU fallback은 재실행하지 않았다.

## Wave 3 실제 release-app 검증 (2026-08-20)

- 시작 HEAD는 `1d23c46b844e9d8fb71dd604052a24f2e62242d1`이고 시작·종료 상태를 확인했다. `src-tauri/Cargo.toml`에 `custom-protocol = ["tauri/custom-protocol"]`을 추가해 production-protocol unpacked binary를 만들 수 있게 했고, `src-tauri/src/media.rs`는 실제 `load_backend: loaded CUDA backend` 로그를 GPU 성공 증거로 인정하도록 최소 수정했다.
- 첫 유효한 production-protocol 무취소 YouTube E2E는 `JKYmw9-xMIo&t=8463s`에서 CDP/Tauri IPC·acquisition까지 도달했지만 HTTP 403으로 실패했다. 원본 snapshot/job/evidence는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-e0b0c58-20260820\data-release-real`, `...\evidence-release-real\e2e-failure-2026-08-19T16-12-41-115Z-23048.json`, 같은 basename `.log`, product command reconstruction은 `...\product-yt-dlp-command.txt`다.
- 제품 transfer는 고정 yt-dlp `2026.07.04`, Deno `2.9.4`, `298+251`, `skip=translated_subs`, 한국어 자동 자막 옵션을 사용했고 metadata probe는 성공했으나 transfer가 0.5%에서 403으로 실패했다. 같은 핀과 signed-range control은 HTTP 206 및 자동 한국어 자막 저장에 성공했으므로 결정적 제품 인자·경로 결함을 확정할 수 없다. 403 원인은 미확정, 제품 경로 수정·재검증은 `HOLD`다.
- release-app local GPU E2E 재검증은 `probe-motion-30s.mp4`(30.0초, SHA-256 `c20156488327d68e435120a599970ac03bc716aa55d163b57079f1a2dc5b54fc`)로 checkpoint `device=gpu`, `gpu.status=COMPLETED`, `completedGpuUnits=1`, `whisper.cpp-gpu`와 비어 있지 않은 raw SRT를 남겼다. raw transcript의 반복 결과 `오오오오오오오오오오`는 quality warning과 함께 candidate 표시에서 `음성 인식 결과가 불확실해 원문을 표시하지 않습니다.`로 가려졌다. 증거는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-e0b0c58-20260820\data-gpu-success-retest\jobs\cc87929e-8980-4c20-a0ee-c3c8016d8dc8`다. 마지막 player-ready 검사 실패로 screenshot은 생성되지 않았고 전체 UI 흐름은 `HOLD`다.
- task-local `cublas64_11.dll` 장애 주입 package는 무결성 검사에서 파일 목록 불일치로 `FAILED`되어 GPU 시도나 CPU fallback에 도달하지 않았다. 증거는 `...\gpu-fault.log`, `...\evidence-gpu-fault\e2e-failure-2026-08-19T16-24-56-910Z-12060.json`, `...\data-gpu-fault\jobs\d715e4c6-f9da-46a8-afd5-30ae635e69d0\snapshot.json`이다. 따라서 자동 GPU→CPU fallback의 의도된 dependency-failure 조건은 `HOLD`이며 fallback 성공을 주장하지 않는다.
- intact verified resources에서 process-only `CUDA_VISIBLE_DEVICES=-1`을 한 번 시도했지만 child command environment allowlist가 이를 제거해 GPU가 실제로 실행됐다. checkpoint `...\data-gpu-fallback-env\jobs\996ef279-3c47-4415-8feb-2d3de24b4bce\media-checkpoint.json`은 `gpu.status=COMPLETED`, `cpuFallback.status=PENDING`과 비어 있지 않은 raw SRT를 남겼고 fallback은 발생하지 않았다. raw token `띄웅`은 이 수정 전 증거에서 candidate snapshot에 노출된 비정보성 결과이며 유효한 콘텐츠로 판정하지 않는다. 증거 `...\gpu-fallback-env.log`와 `...\data-gpu-fallback-env\jobs\996ef279-3c47-4415-8feb-2d3de24b4bce\tool-logs\whisper-gpu-0000-00.stderr.log`를 보존했으며 추가 시도는 하지 않는다.
- 품질 안전 수정은 관찰된 토큰을 하드코딩하지 않는다. 알려진 전체 미디어 길이 대비 음성 구간 비율을 사용해 단일 세그먼트가 2.5초 이하이고 전체 입력의 20% 이하를 차지하면서 짧은 한국어 정보량이 2자 이하인 경우에만 표시를 불확실 placeholder로 바꾼다. 따라서 30초 nonspeech probe의 `띄웅` 유형은 가려지고, 전체 입력이 약 2초인 `네`·`안녕` 같은 짧은 정상 음성은 표시된다. focused lib test는 `1 passed`다.

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
- `npm test` 49개, `npm run build`, Rust 본체 128 passed·1 ignored, fixture-worker 6개, 관련 Node 17개, 보안 6개, `git diff --check`는 `PASS`다. 실제 G8 앱은 승인 URL의 시작 단계와 취소 완료를 확인했다.
- 세 승인 URL은 일반 VOD의 자동 한국어 자막 우선 대표 입력이며 `language=ko`, 자동 생성 한국어 SRT, 파일 무결성·기본 시간 범위를 확인했다. 제작자 자막 부재는 `HOLD`나 릴리스 차단 사유가 아니며, 제품 snapshot provenance와 Whisper 대체는 별도 `HOLD`다. 겹침·공백 수치는 품질 `PASS`가 아니며 일정한 시간 오프셋과 내용 품질도 `HOLD`다.
- 자막 증거는 다음과 같다: `JKYmw9-xMIo` — 133570 bytes, SHA-256 `af6aa5d008bbdd36e60f8c07d556da52686cb52be99b660e8e555783b4f510ef`, 2121 cues, start>=end 0, out-of-range 0, reverse 0, adjacent overlaps 1346, exact duplicate groups 0, max positive gap 47.080 sec; `LVZ6hFhlF2k` — 399263 bytes, SHA-256 `24857fa9aee1fd459e040d5939159ca3c0ea45bb69fa0f7ed7925bf5dfcf1efa`, 5832 cues, start>=end 0, out-of-range 0, reverse 0, adjacent overlaps 5364, exact duplicate groups 0, max positive gap 207.361 sec; `ZJMpYThMksM` — 88141 bytes, SHA-256 `64d876c1ff3813bfc2309d1302af60a12618a55ace924843d7a536a4136c6c55`, 1472 cues, start>=end 0, out-of-range 0, reverse 0, adjacent overlaps 880, exact duplicate groups 0, max positive gap 147.040 sec.
- 실제 앱은 취소 요청 뒤 4779 ms에 `CANCELLED`를 확인했지만 HTTP 403이 검토 화면 전에 발생했다. 기존 앱 403의 원인은 미확정이며 제품 경로 수정은 검증되지 않았고, 화면 캡처와 전체 후보·검토·삭제 흐름은 후속 검증 `HOLD`다. 최종 증거는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-evidence-full-225324\e2e-failure-2026-08-19T13-53-48-401Z-21572.json`과 같은 basename의 `.log`다.
- 이번 재검증에서는 `JKYmw9-xMIo` 무취소 제품 E2E를 한 번 실행했으나 acquisition 전 로컬 CDP/Tauri IPC 평가에서 실패했다. 기존 HTTP 403 원본 JSON/로그는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-evidence-full-225324\e2e-failure-2026-08-19T13-53-48-401Z-21572.json` 및 같은 basename의 `.log`로 보존했고, coordinator의 재검증 상한에 따라 E2E를 다시 실행하지 않았다. 고정 yt-dlp/Deno control은 HTTP `206`과 자동 한국어 자막 저장에 성공했지만 기존 앱 403은 재현하지 못했으므로 원인은 미확정이며 제품 경로 403 수정은 검증되지 않았다. 앱 흐름 검증은 로컬 E2E 진입 실패로 `BLOCKED`다.
- G8 GPU 패키지는 `whisper-cli.exe` 489984 bytes/SHA-256 `4bf174113843613cbec146e73e6820a767e54b0e1c736f2c6d7ab16aac4c245d`, `ggml-cuda.dll` 562600960 bytes/SHA-256 `24af2cd89090175beffdf77cd25c176d76f09c4018644915f302d2de64d67631`, `cudart32_110.dll` 467456 bytes/SHA-256 `b8bfc244dd0916ddf7b45e39c101f165a0d9f4846616eaf34336a2c374409408`, `cudart64_110.dll` 526848 bytes/SHA-256 `ba5c2fb526c4ee4bb218ceb3fa5e8bfde89ce474f38711fdcce802549bf9fc6f`이며 `cublas64_11.dll`이 없다. CPU backend 로그는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-evidence-full-225324\gpu-probe\whisper-gpu.log`, valid `probe.srt`는 47 bytes/SHA-256 `5845cd37d6a0bbae0ce13a136d7652d6b5688938ceaf32a6974a20a57a24a97d`다. 외부 GPU 바이너리 추가 없이는 패키지 보완과 실제 GPU 검증을 끝낼 수 없어 `BLOCKED`다.
- 이번 변경은 G8 패키지·설치 파일·모델을 수정하지 않았다.

## Wave 2 GPU 패키지·실제 음성 인식 결과 (2026-08-20)

- 원인: whisper.cpp v1.9.1 공식 GPU archive에는 `ggml-cuda.dll`이 `cublas64_11.dll`을 import하지만 해당 DLL이 없었다. `cublas64_11.dll`은 `cublasLt64_11.dll`도 import한다. archive layout/import 전체 증거는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-wave2-gpu-20260820\dependency-inspection.log`다.
- 수정: 공식 NVIDIA CUDA cuBLAS redistributable `11.11.3.6` URL `https://developer.download.nvidia.com/compute/cuda/redist/libcublas/windows-x86_64/libcublas-windows-x86_64-11.11.3.6-archive.zip`, archive SHA-256 `67b0934a6359e4ee26fff823c356021589d392c4fd49ca12624f570edc08e2b9`, license `CUDA Toolkit`, EULA `https://docs.nvidia.com/cuda/archive/11.8.0/eula/index.html`을 고정하고 `cublas64_11.dll`·`cublasLt64_11.dll`만 private `whisper-gpu`에 복사했다. 설치 SHA-256은 각각 `8ca516b96b29c2fba2344909a896bc1cd7951f6cd11fe595a8a3929c02cccbed`, `3d06ca4e4893adb7a153ecd23a540e92817c967312b44646d8c3f91b089196e6`이고 `NVIDIA-CUDA-Toolkit.txt` SHA-256은 `17a280713a9cf1930d0f3a946935ca968d9726a64f1a41c9a589a959a673784f`다.
- 하드웨어: fresh `nvidia-smi`는 NVIDIA GeForce GTX 1060 3GB, driver `560.94`, `3072 MiB`를 보고했다. 입력은 보존된 2.0초 `probe.wav`, 64078 bytes, SHA-256 `7695bcad887367c33cf9f9bce6bf0d98b4fd1547d1b2f9b392c4d426ef7a33c1`이며 task TEMP `C:\Users\myhan\AppData\Local\Temp\vod-scout-wave2-gpu-20260820\probe.wav`에 복사했다.
- 결과: GPU direct CLI는 CUDA device/backend marker(`found 1 CUDA devices`, GTX 1060, `using CUDA0 backend`, `use gpu = 1`)와 non-empty valid 2.0초 SRT를 기록해 `PASS`했다. CPU direct CLI는 `--no-gpu`, `use gpu = 0`, `no GPU found`와 non-empty valid SRT를 기록해 `PASS`했다. SRT는 47 bytes, SHA-256 `74e9f3ff2da6c73ad7d9bb45ee7f1a5be4a7a8cb29c1c2a33920af9be4ed882c`이며 GPU/CPU 전체 로그는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-wave2-gpu-20260820\gpu-direct.log`, `cpu-direct.log`다.
- 설정·회귀: GPU는 threads `4`, CPU는 `--no-gpu`와 threads `2`; Whisper profiles/retry gate 5 tests, GPU evidence 1 test, CPU args 1 test, media-tools preparation 2 tests가 `PASS`다. 기존 제품 pipeline의 GPU 실패를 안전하게 주입해 자동 CPU 전환하는 진입점은 직접 CLI 검증 범위 밖이므로 `HOLD`이며 자동 전환을 주장하지 않는다.

G1~G8은 이 작업 트리(worktree)에 로컬 통합되어 있다. 이 통합에서 push, PR 생성, remote merge(원격 병합), tag, Release, deploy(배포)는 발생하지 않았고 `main`은 수정하지 않았다.

## G1~G8 구현 결과

- G1: 한국어 자동 자막 우선, 없거나 사용할 수 없을 때 제작자 한국어 자막 대체, 자동 번역·다른 언어·`live_chat` 제외, 원본 시간 검증과 검증 불가 구간의 로컬 Whisper 대체, 자막 provenance 저장. 선택 순서·제외 트랙 테스트와 세 대표 자동 자막 입력은 `PASS`이며 제품 snapshot provenance·Whisper 대체는 `HOLD`.
- G2: `자동(GPU 우선)`·GPU·CPU 모드와 프로필, GPU 근거 게이트, 실패 청크의 CPU 1회 대체, 재실행 시 상태 보존. 공식 CUDA cuBLAS 의존성 보강과 직접 GPU·CPU 결과는 `PASS`; 제품 자동 전환·화면은 `HOLD`.
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
- `cargo.exe test --manifest-path src-tauri/Cargo.toml`: PASS — 128 passed, 1 ignored.
- `cargo.exe test --manifest-path src-tauri/fixture-worker/Cargo.toml`: PASS — 6 passed.
- `npm run test:security`: PASS — 6 passed.
- `node --test scripts/archive-safety.test.mjs scripts/prepare-media-tools.test.mjs scripts/sample-disk-usage.test.mjs`: PASS — 11 passed.
- `git diff --check`: PASS (최종 문서·코드 확정 후).

## 패키지·진입점 결과

- `npm run tauri:build`의 Windows PowerShell 패키지 증거에서 새 FFmpeg 자산 다운로드·SHA-256·media-tools 준비와 release/NSIS 생성은 성공했다. updater 개인키가 없어 서명은 HOLD이며, fresh 격리 앱은 8초 생존 확인 직후 테스트 프로세스를 의도적으로 중단했으므로 정상 종료로 기록하지 않는다.
- `vod-scout.exe`: 16,274,944 bytes, SHA-256 `8754dc944d8f685195425bb4d3698c8992225888b2b95e9ed484283b7868cced`, PE ProductVersion/FileVersion `0.5.0`.
- `VOD Scout_0.5.0_x64-setup.exe`: 595,736,201 bytes, SHA-256 `cd024e2d4523c34f4795c0e3d5bca1f72edca82b89d294033194fa465624ca36`, PE ProductVersion/FileVersion `0.5.0`.
- fresh `VOD_SCOUT_E2E_DATA_DIR`에서 빌드 앱이 8초 생존했고 `instance.lock`·`queue.json`만 생성됐다. 생존 확인 직후 테스트 프로세스를 의도적으로 중단했으며 정상 종료가 아니다. 실제 설치 파일 설치·설치 후 실행·Windows 화면은 확인하지 않았고, 기존 설치 앱·사용자 데이터는 건드리지 않았다.
- 고정 FFmpeg asset: `https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-08-17-13-05/ffmpeg-n8.1.2-44-g7c533d0f86-win64-lgpl-shared-8.1.zip`, archive size 70,837,934 bytes, SHA-256 `681b9ca6d8f9be1e01d8873ad16f8a632f8a22b9653f1044837de6d5979b0fd6`.
- 실제 설치 파일 설치·설치 후 실행·Windows 화면, updater `.sig`, 공개 Release 자산은 생성·검증하지 않았으며 `HOLD`다.

## 문서 상태와 남은 HOLD

- 계획·릴리스·아키텍처·UI·테스트 계약은 현재 G1~G8 로컬 통합과 자동 검증 결과를 가리킨다. 제작자 자막 부재는 릴리스 차단 사유가 아니며, 제품 snapshot provenance, Whisper 대체, 자동 GPU→CPU 제품 전환, 설치·Windows 화면, 1~8시간·동일 입력 자원 비교와 G7 병렬 측정은 `HOLD` 또는 `BLOCKED`다. GTX 1060 직접 GPU·CPU CLI는 `PASS`이고 30초 구성요소 자원 측정은 `MEASURED_NO_THRESHOLD`로 완료했다.
- 세 SRT의 start/end 범위·역순·중복 그룹·겹침·공백을 기록했지만 겹침·공백을 품질 `PASS`로 판정하지 않았고, 일정한 시간 오프셋·내용 품질·사람 판정은 `HOLD`다.
- 직접 GPU 백엔드 시험과 CPU 시험은 `PASS`이며, HTTP 403 원인과 제품 경로 수정이 미확정이고 새 E2E가 acquisition 전에 실패해 자동 GPU→CPU 제품 전환·Windows 사용자 화면 흐름과 screen capture도 `HOLD`다.
- 1~8시간 resource/long-run 및 기존 v0.4.0과의 동일 입력 비교는 실행하지 않았다.
- G7 병렬 옵션은 사용할 수 없다.
- 실제 설치 파일 설치·Windows 화면, updater `.sig`, 공개 v0.5.0 URL/Release는 HOLD다. README에는 공개 v0.4.0 다운로드·Release 링크만 남긴다.

## 롤백

이 브랜치의 변경을 공개 main이나 기존 설치에 반영하지 않는다. 회귀가 확인되면 공개 v0.4.0 정본을 사용하고, 사용자 작업 폴더·설치 폴더를 삭제하거나 덮어쓰지 않는다. 버전업 기록과 패키지 산출물은 실제 결과만 갱신한다.

## 다음 정확한 작업

1. 제품 pipeline에서 GPU 실패 1회 뒤 CPU fallback 1회와 checkpoint 상태를 직접 확인하고, Windows 화면 검증을 마친다.
2. HTTP 403 원인이 확인되고 E2E 진입이 정상화된 승인 입력에서만 전체 후보·검토·삭제 흐름을 재시도한다. 같은 입력에 대한 추가 재시도는 계약 제한으로 하지 않는다.
3. updater 서명 개인키가 승인된 환경에 있을 때만 `.sig` 생성과 Release 자산 검증을 진행한다.
4. 실제 기준 영상·Windows UI·resource/long-run·parallel 측정과 위 HOLD/BLOCKED가 끝날 때까지 v0.5.0 공개 링크·Release를 만들지 않는다.
