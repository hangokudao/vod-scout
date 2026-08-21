# VOD Scout 빌드 명세

## v0.5.0 unsigned 정식 Release 최종 후보

- 버전 정본: `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, `src/releaseNotes.ts`, EXE와 NSIS가 `0.5.0`으로 일치한다.
- GPU·Auto·CPU 기능과 GPU runtime은 유지한다. Cargo 결과는 `136 passed·1 flaky·1 ignored`이며 판정은 `PASS_WITH_KNOWN_TEST_LIMITATION`; GPU 실패 뒤 CPU 자동 전환은 `DEFERRED`다.
- `createUpdaterArtifacts=false`; 공개 자산은 unsigned NSIS installer, `SBOM.spdx.json`, `SHA256SUMS.txt`, Release notes만 허용하며 `.sig`, `latest.json`, updater zip은 생성하지 않았다.
- 최종 EXE는 `16,313,856` bytes/SHA-256 `ccf1e0022abaf64c158c9b28ad900afb95a96643f3dc53f021a54cb1f3d37cf6`다. 공개용 unsigned NSIS `VOD.Scout_0.5.0_x64-setup.exe`는 `595,413,873` bytes/SHA-256 `abd3c2ea3b49673b438880363995bd750ef36d788a35bd907f6255f10b1f4221`, PE `0.5.0`, Authenticode `NotSigned`다.
- 최종 `SBOM.spdx.json`은 SPDX-2.3, `656` packages, `615,471` bytes/SHA-256 `50cffc7ba05f33c0a55262296a0c346898a06c4193c255d3b99783ace20f2639`다. `SHA256SUMS.txt`는 `177` bytes/SHA-256 `c86eb05fb3257f26b25d2be5d2066805a526ad14cb6704be427855481ad1e62e`이며 installer·SBOM 해시와 일치한다.
- CDP 화면 스모크는 새 임시 데이터·WebView 경로에서 `로컬 파일` 탭을 선택한 뒤 메인 화면, `0.5.0`, Auto/GPU/CPU, 속도 단계와 CPU 사용량을 확인했다. 스크린샷은 `105,842` bytes/SHA-256 `6e48040201047e21b67defd1963819dc266d9dc92f0726f736cf1e8881f80691`, 증거 경로는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-v050-cdp-smoke-retry-20260822-022500-fbf8d7b`이며 기존 설치·사용자 데이터 지문은 전후 일치한다.

## 2026-08-21 이전 엄격 공개 게이트 기록

- validation-fixes `ae25f1342ae25f1ee7a2eb6dc6a694d4aedf14d8`와 integration `53172fab00d61a398333e0cee18652fa0d1b5387`를 기준으로 문서만 갱신한다. 두 worktree는 시작 시 clean이며 docs-only 변경 뒤에도 clean으로 복원한다.
- 공식 최신 `yt-dlp` nightly는 `2026.08.19.233000`이다. asset/digest SHA-256 `02bcc69a2a65a2af5da81a79356763522b611edc028c476e78c282735e28d442`, `SHA2-256SUMS` SHA-256 `6b2471fa596aaa588446fb0dfcf6025f8533d9dc931fa034b21c40c431473ce6`, source commit `594bd50c2c78ac432f81600d309fdc4e0a92d82c`, LICENSE `7e12e5df4bae12cb21581ba157ced20e1986a0508dd10d0e8a4ab9a4cf94e85c`, THIRD_PARTY `472aefe951c7db35e1657c1d13fd337140511ed6f2b329205105ad441c5a02b7`가 manifest 및 단일 `check:yt-dlp` PASS 증거와 일치한다.
- player-ready 평가 수정 후 ZJ integration job `9b6de644-e40f-4130-bffb-301eab4a03a6`는 `REVIEW_READY`, download `100%`, 후보 `20`, `18/18` units, GPU `12/12`, `previewPlayerReady=true`, `bodyVerified=true`를 기록했다. `candidateRevision=0`, `recognitionRuns=[]`라 선택 후보 재인식 완료가 입증되지 않은 ZJ 현재 상태만 `HOLD`다. snapshot은 `C:\Users\myhan\AppData\Local\com.vodscout.app\e2e-requested-data-zjmp-integration\jobs\9b6de644-e40f-4130-bffb-301eab4a03a6\snapshot.json`, screenshot은 `C:\Users\myhan\AppData\Local\Temp\vod-scout-zjmp-integration-screen-m3unDv\review.png`다.
- JKY full E2E job `ee834a3d-fde9-49c9-8cf8-ced9654e1c45`는 `REVIEW_READY`, download `100%`, 후보 `20`, `22/22` units, GPU `16/16`, candidate revision `1`, recognition run `28ba3f98-8727-4517-868b-2a42f9091f51` `COMPLETED`, result revision `1`, `failureReason=null`, 실제 backend `whisper.cpp-gpu`로 current full E2E `PASS`다. snapshot/data root는 `C:\Users\myhan\AppData\Local\com.vodscout.app\e2e-requested-data-jky\jobs\ee834a3d-fde9-49c9-8cf8-ced9654e1c45`, screenshot은 `C:\Users\myhan\AppData\Local\Temp\vod-scout-jky-full-20260820-231751\review.png`다.
- exact automatic GPU→CPU fallback·signing·install·GitHub publication은 `HOLD`이며 overall release judgment도 `HOLD`다.
- 최종 post-provenance 자동 검증: npm 49, Rust `128 passed·1 ignored`, fixture `6`, security `6`, standalone Node `6+3+3`, build 및 diff check `PASS`. 전체 로그·exit code·크기·SHA-256은 `C:\Users\myhan\AppData\Local\Temp\vod-scout-final-post-provenance-vxCIf4`에 있다.
- 새 unsigned package: `vod-scout.exe` `16,279,040` bytes/SHA-256 `4bcac97a30edb54be83b81711524ad4335cce5bdc315a44ff77d53121f52ec74`; `VOD Scout_0.5.0_x64-setup.exe` `595,368,676` bytes/SHA-256 `7c9e013794886fc220d82e1503d97277e5d79afca2b97f6ec7d367eab3041d3e`; both PE `0.5.0`, Authenticode `NotSigned`. Signing/updater/public Release는 `HOLD`; package rebuild는 이 docs-only 변경 뒤 수행하지 않는다.
- SBOM `SBOM.spdx.json`: SPDX-2.3, root `vod-scout@0.5.0`, `656` packages, `615,471` bytes, SHA-256 `418fd00061bc16a517a524691a8db6272e193c46dc47fba5f90bf012d177ed0a`. GitHub publication은 발생하지 않았다.

## 2026-08-20 Wave 5 release-app validation gates

- stable `yt-dlp 2026.07.04`의 `android_vr`/`ANDR-V` client는 exact `298+251` 선택 후 HTTP `403`을 반환했다. official nightly `2026.08.18.122307`은 source commit `yt-dlp/yt-dlp@5d5b634d8e6b41dc2891847a5ea7a5a3f569a28c`, Windows asset SHA-256 `652e154bce7170070d0f26415c9a3c35c121f5a7903cb8cde6d31c4577517fb9`로 고정됐고 `visionos` 경로 first-byte control에서 403 없이 도달했으며 release-app full transfer는 두 job 모두 100%였다. 기술 수정 커밋은 `6aa2bc83c48835082b5f08ee14fab0f9570eb691`이다.
- ZJ job `6251521b-6749-4415-9bf1-7eac826bae0f` (`ZJMpYThMksM&t=2017s`): download `100%`, `REVIEW_READY`, candidates `20`, `18/18` units, GPU `12/12` completed. Automatic Korean caption provenance: `trackId=Korean`, SHA-256 `d74a0bab2029be6b1d33c27e66031838076b2fe658e13b910c3327e0a3f71562`, `quality=unverified`; unverified intervals used local Whisper.
- JKY job `c8f80e03-33d7-4c3b-af5f-6f498be31f72` (`JKYmw9-xMIo&t=8463s`): download `100%`, `REVIEW_READY`, candidates `20`, `22/22` units, GPU `16/16` completed. Automatic Korean caption provenance: `trackId=Korean`, SHA-256 `81dff33650b150069a03cd73db2aaf8d8e29682cdf4a80c9366e1b4e64cdb6cc`, `quality=unverified`; unverified intervals used local Whisper.
- Both release-app attempts then failed the same player-ready check at `scripts/e2e-local-cdp.mjs:279`; screenshots, preview, candidate re-recognition, and complete UI remain `HOLD`/`BLOCKED`. Automatic GPU→CPU fallback was not safely inducible and remains `HOLD`. Evidence: `C:\Users\myhan\AppData\Local\Temp\vod-scout-v050-public-gate-20260820\evidence-zjmp-retry\e2e-failure-2026-08-19T19-24-56-054Z-17056.json`/`.log` and `...\evidence-jky\e2e-failure-2026-08-19T19-39-18-521Z-24076.json`/`.log`.
- Release build, Rust release, and NSIS body succeeded; the overall `tauri:build` rc `1` is signing-only because `TAURI_SIGNING_PRIVATE_KEY` is absent. Current `vod-scout.exe`: `16,279,040` bytes, SHA-256 `a26e472265e52bd48d02bab6fbee357efa99764dcc56d16f82aa705626fd9fba`, PE `0.5.0`; current NSIS: `595,405,262` bytes, SHA-256 `75d46c747eca1969c46f85160caf4cfb315b6e2485f9d62f54cbde38be1593ff`, PE `0.5.0`. Runtime manifest schema 6 `51/51` runtime + `1/1` license hash, security `6/6`, and `npm audit --omit=dev` 0 vulnerabilities are `PASS`; `cargo-audit` is unavailable/not a project gate. signing/install/public Release are `HOLD`/`BLOCKED`. Windows Sandbox is unavailable; long-run, human content quality, and GPU memory remain `HOLD`; G7 is disabled.
- Measured automatic Korean VTT: ZJ duration `6847.121`, `2943` cues, first `57.640`, last `6829.639`; JKY duration `9590.061`, `4241` cues, first `51.199`, last `9566.720`. Both measured inverted/out-of-range/adjacent overlap/exact-time duplicate `0` and max positive cue gap `0`; constant time drift remains `UNVERIFIED`, with no acceptance threshold invented.
- Candidate display guard masked ZJ `20/20` and JKY `16/20` candidate excerpts with `음성 인식 결과가 불확실해 원문을 표시하지 않습니다.`; visible repeated/broken pattern count was `0`. ZJ: elapsed `662.728s`, final `1,371,590,406` bytes (`1.277 GiB`), media `1,344,021,572`, caption `308,005`, `122` files. JKY: elapsed `784.380s`, final `2,048,005,882` bytes (`1.907 GiB`), media `2,019,916,819`, caption `476,641`, `154` files. These are `MEASURED_NO_THRESHOLD`; CPU/peak memory/GPU memory/temp peak were not sampled in these runs and retain prior/HOLD evidence.
- Tracked `SBOM.spdx.json` parses as SPDX-2.3 with root `vod-scout@0.5.0`, `656` packages, `615,471` bytes, SHA-256 `418fd00061bc16a517a524691a8db6272e193c46dc47fba5f90bf012d177ed0a`. `SHA256SUMS.txt` and release-assets remain ungenerated because the updater signing artifact is absent; no PASS is claimed.

## 2026-08-20 Wave 4 자원·패키지 검증

- 자막 선택 계약은 일반 YouTube VOD에서 한국어 자동 자막 우선, 자동 자막이 없거나 사용할 수 없을 때만 제작자 한국어 자막, 두 경로가 없거나 검증할 수 없는 구간은 로컬 Whisper 순서다. 자동 번역·다른 언어·`live_chat`은 제외한다.
- 승인된 보존 `probe-motion-30s.mp4`(30.0초·3,427,504 bytes·SHA-256 `c20156488327d68e435120a599970ac03bc716aa55d163b57079f1a2dc5b54fc`)로 직접 GPU/CPU만 측정했다. FFmpeg: elapsed `29.755s`, CPU `0.094s`, peak WS `22,716,416`, peak private `20,803,584`, temp peak `4,393,690`, final job `960,590`, wrapper rc `0`. GPU Whisper: elapsed `2.710s`, CPU `2.438s`, WS `273,334,272`, private `1,148,428,288`, temp peak `4,395,267`, final `48`, wrapper rc `0`, non-empty SRT SHA-256 `71aca4471d2f8b60f8fee3665378f93851650831ab0334cc05bd516ffac89b64`. CPU Whisper: elapsed `8.110s`, CPU `14.953s`, WS `324,505,600`, private `837,447,680`, temp peak `4,396,535`, final `69`, wrapper rc `0`, non-empty SRT SHA-256 `346f8c5f7783d976fce352249764fbbd3a002b1a66dda35746b1e804d1de7bb0`. 세 JSON의 exitCode는 `null`이며 wrapper 결과 로그가 rc `0`을 증명한다. GPU `peakGpuBytesObserved=0`은 per-process sample 없음이므로 GPU memory는 `UNAVAILABLE`/`HOLD`; `nvidia-smi` 1442 MiB는 whole-GPU snapshot일 뿐 app peak이 아니다. 모든 수치는 `MEASURED_NO_THRESHOLD`다.
- Windows Node에서 공식 manifest 핀 재준비와 release runtime manifest schema 6의 51/51 파일·해시 검증은 `PASS`다. Linux Node의 Windows `tar.exe` 경로 실패는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-wave4-20260820\logs\prepare-media-tools-linux-failure.log`에 보존했다.
- lockfile v3 `npm.cmd ci`(rc 0) 뒤 단일 Windows `npm.cmd run tauri:build`는 NSIS 본문 생성까지 성공했지만 signing key 부재로 전체 rc `1`로 종료했다. NSIS 생성·hash는 `PASS`, signing은 `HOLD`다. `vod-scout.exe`: 16,274,944 bytes/SHA-256 `8754dc944d8f685195425bb4d3698c8992225888b2b95e9ed484283b7868cced`, PE ProductVersion/FileVersion `0.5.0`; `VOD Scout_0.5.0_x64-setup.exe`: 595,736,201 bytes/SHA-256 `cd024e2d4523c34f4795c0e3d5bca1f72edca82b89d294033194fa465624ca36`, PE `0.5.0`이다.
- fresh 격리 data-dir 앱은 8초 생존 후 테스트 프로세스를 의도적으로 중단했다(`PASS`, 정상 종료 아님). 1~8시간 입력은 승인된 안전한 증거가 없어 `HOLD`; G7 병렬은 disabled/`HOLD`다.

## 2026-08-20 Wave 3 production-app 검증

- production-protocol unpacked `vod-scout.exe`는 `cargo.exe build --release --features custom-protocol`로 만들었다. 첫 valid no-cancel YouTube path는 `JKYmw9-xMIo&t=8463s`에서 acquisition까지 도달했으나 HTTP 403으로 실패했다. full command reconstruction은 `C:\Users\myhan\AppData\Local\Temp\vod-scout-e0b0c58-20260820\product-yt-dlp-command.txt`, stderr는 `...\data-release-real\jobs\bfb8c79b-181a-4533-b4fd-ef5a0da29b75\tool-logs\yt-dlp.stderr.log`다.
- Product는 pinned yt-dlp `2026.07.04` SHA-256 `52fe3c26dcf71fbdc85b528589020bb0b8e383155cfa81b64dd447bbe35e24b8`, Deno `2.9.4` SHA-256 `4a2757fe99afc2c62c46500c8221cfa0189ac4bfb7064141875ad9c0f04b60ef`, format `298+251`, Deno JS runtime, Korean auto-caption flags를 사용했다. 동일 control은 HTTP 206과 Korean auto-caption save에 성공했으므로 product 403 원인은 미확정이며 product-path fix/retest는 하지 않았다.
- release-app GPU retest input `probe-motion-30s.mp4`: 30.0 sec, 3,427,504 bytes, SHA-256 `c20156488327d68e435120a599970ac03bc716aa55d163b57079f1a2dc5b54fc`. Checkpoint `...\data-gpu-success-retest\jobs\cc87929e-8980-4c20-a0ee-c3c8016d8dc8\media-checkpoint.json` records GTX 1060 runtime's `device=gpu`, `gpu.status=COMPLETED`, one GPU unit, and non-empty raw SRT; its repeated raw transcript was display-masked as uncertain rather than presented as valid content. Release E2E ended at player-ready and produced no screenshot.
- One task-local package fault injection removed only `whisper-gpu/cublas64_11.dll`; product stopped at the existing exact runtime-file-list integrity guard before Whisper. Snapshot and failure evidence are under `...\data-gpu-fault\jobs\d715e4c6-f9da-46a8-afd5-30ae635e69d0\snapshot.json` and `...\evidence-gpu-fault\`. Automatic GPU→CPU fallback under dependency failure remains `HOLD`.
- One independent intact-resource process-only `CUDA_VISIBLE_DEVICES=-1` attempt was made. The app's child-process environment allowlist removed that variable, so the checkpoint records one completed GPU unit, `cpuFallback.status=PENDING`, and a non-empty raw SRT rather than a fallback; the pre-fix candidate snapshot exposed the raw token `띄웅`, which is not valid content. Evidence: `...\gpu-fallback-env.log`, `...\data-gpu-fallback-env\jobs\996ef279-3c47-4415-8feb-2d3de24b4bce\media-checkpoint.json`, and `...\data-gpu-fallback-env\jobs\996ef279-3c47-4415-8feb-2d3de24b4bce\tool-logs\whisper-gpu-0000-00.stderr.log`; no further attempt was made.
- Display safety now uses known media duration and speech coverage, not an observed token: only a single <=2.5-second, <=2-Korean-character, <=20%-coverage result is masked. A whole ~2-second input with a short one-word utterance remains visible; raw evidence remains preserved.

## 2026-08-20 validation-fixes 후속 검증

- 현재 브랜치 `codex/v050-validation-fixes`, 시작 HEAD `377937e1e5bbf58eb8420416ed9a29803e9fb57b`; 버전은 `0.5.0`으로 유지했다.
- 자막 선택 순서·트랙 식별자 보존 Rust 테스트 15개, `npm run build`, `git diff --check`: `PASS`.
- 고정 yt-dlp `2026.07.04` Windows x64 SHA-256 `52fe3c26dcf71fbdc85b528589020bb0b8e383155cfa81b64dd447bbe35e24b8`, Deno `2.9.4` Windows x64 SHA-256 `4a2757fe99afc2c62c46500c8221cfa0189ac4bfb7064141875ad9c0f04b60ef`를 task TEMP에서 확인했다. 기존 tracked manifest의 URL·버전·라이선스·SHA-256 값은 변경하지 않았다.
- 승인 URL `JKYmw9-xMIo`의 no-cancel 제품 E2E는 acquisition 전에 로컬 CDP/Tauri IPC 평가에서 실패했다. 기존 HTTP 403 원본 증거는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-evidence-full-225324\e2e-failure-2026-08-19T13-53-48-401Z-21572.json` 및 같은 basename의 `.log`다. 고정 yt-dlp/Deno control은 HTTP `206`과 한국어 자동 자막 저장에 성공했지만 기존 앱 403은 재현하지 못했으므로 원인은 미확정이며, 제품 경로 403 수정은 검증되지 않았다. 재검증 상한에 따라 E2E를 재시도하지 않았고 앱 흐름 검증은 로컬 E2E 진입 실패로 `BLOCKED`다.
- Wave 2에서 whisper.cpp `v1.9.1` 공식 GPU archive layout을 확인했다. `ggml-cuda.dll`의 정적 import에 `cublas64_11.dll`이 있었고 archive에는 없었으며, `cublas64_11.dll`은 `cublasLt64_11.dll`을 추가로 import한다. 상세 archive/import 증거는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-wave2-gpu-20260820\dependency-inspection.log`다.
- 공식 NVIDIA CUDA cuBLAS redistributable `11.11.3.6`을 추가로 고정했다: `https://developer.download.nvidia.com/compute/cuda/redist/libcublas/windows-x86_64/libcublas-windows-x86_64-11.11.3.6-archive.zip`, archive SHA-256 `67b0934a6359e4ee26fff823c356021589d392c4fd49ca12624f570edc08e2b9`, license `CUDA Toolkit`, EULA `https://docs.nvidia.com/cuda/archive/11.8.0/eula/index.html`. 설치 파일 SHA-256은 `cublas64_11.dll` `8ca516b96b29c2fba2344909a896bc1cd7951f6cd11fe595a8a3929c02cccbed`, `cublasLt64_11.dll` `3d06ca4e4893adb7a153ecd23a540e92817c967312b44646d8c3f91b089196e6`; notice `src-tauri/resources/media-tools/licenses/NVIDIA-CUDA-Toolkit.txt` SHA-256은 `17a280713a9cf1930d0f3a946935ca968d9726a64f1a41c9a589a959a673784f`다.
- 새 패키지의 직접 CLI 검증은 GTX 1060 3GB, driver `560.94`, VRAM `3072 MiB`에서 같은 2.0초·64078-byte WAV (`7695bcad887367c33cf9f9bce6bf0d98b4fd1547d1b2f9b392c4d426ef7a33c1`)로 GPU `PASS`(`use gpu=1`, CUDA0 backend, non-empty SRT 47 bytes/SHA-256 `74e9f3ff2da6c73ad7d9bb45ee7f1a5be4a7a8cb29c1c2a33920af9be4ed882c`)와 CPU `PASS`(`--no-gpu`, `use gpu=0`, non-empty SRT)을 확인했다. 전체 GPU→CPU 제품 전환은 직접 CLI 범위 밖이라 `HOLD`다. 전체 로그는 `C:\Users\myhan\AppData\Local\Temp\vod-scout-wave2-gpu-20260820\gpu-direct.log`와 `cpu-direct.log`다.
- Wave 2 focused tests: media-tools preparation 2 passed, Whisper settings/profile/retry-gate 5 passed, GPU evidence 1 passed, CPU args 1 passed, Windows `node scripts/prepare-media-tools.mjs` reported `media tools already prepared`, 최종 문서·스크립트 확정 후 `git diff --check`는 `PASS`다.

## v0.5.0 로컬 후보 통합 빌드

상태: **G1~G8 로컬 통합·자동 검증 PASS · 로컬 NSIS/PE/해시(hash)/빌드 앱 격리 8초 생존 PASS · 실제 설치 파일 설치/Windows 화면/updater 서명/공개 자산 HOLD**

- origin/main 기준: `eee71e04776a6179c289167596e9d82d52e94e13` (PR #18 반영).
- G8 패키지 증거 원본: `6ecbd49` · 로컬 통합 병합: `7c8b336` · 현재 브랜치: `codex/v050-integration`.
- G1~G8은 이 작업 트리(worktree)에 로컬 통합되어 있다. 이 통합에서 push, PR 생성, remote merge(원격 병합), tag, Release, deploy(배포)는 발생하지 않았고 `main`은 수정하지 않았다.
- 버전 정본: `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, `src/releaseNotes.ts`, workflow/helper가 `0.5.0`.
- `npm ci`: PASS. `npm test`: 49 passed. `npm run build`: PASS (1,793 modules).
- `cargo.exe test --manifest-path src-tauri/Cargo.toml`: 128 passed, 1 ignored. Fixture worker: 6 passed.
- `npm run test:security`: 6 passed. Archive/media-tool/sample-disk tests: 11 passed.
- `npm run tauri:build` (PowerShell): **NSIS 생성까지 PASS, 서명은 HOLD** — 새 FFmpeg 고정 자산과 media-tools를 준비하고 NSIS를 생성했으며 updater 개인키가 없다. fresh 격리 앱은 8초 생존 확인 직후 테스트 프로세스를 의도적으로 중단했으므로 정상 종료로 기록하지 않는다.
- `node scripts/generate-release-assets.mjs`: **HOLD** — 공개 Release 자산은 서명 키 부재로 생성하지 않았다.
- `vod-scout.exe`: 16,274,944 bytes · SHA-256 `8754dc944d8f685195425bb4d3698c8992225888b2b95e9ed484283b7868cced` · PE ProductVersion/FileVersion `0.5.0`.
- `VOD Scout_0.5.0_x64-setup.exe`: 595,736,201 bytes · SHA-256 `cd024e2d4523c34f4795c0e3d5bca1f72edca82b89d294033194fa465624ca36` · PE ProductVersion/FileVersion `0.5.0`.
- fresh `VOD_SCOUT_E2E_DATA_DIR`: **PASS** — 빌드 앱이 8초 생존했고, 확인 직후 테스트 프로세스를 의도적으로 중단했다(정상 종료 아님). 격리 폴더에 `instance.lock`·`queue.json` 2개가 생성됐으며 실제 설치 파일 설치·설치 후 실행·Windows 화면은 확인하지 않았다. 기존 설치 앱·사용자 데이터는 변경하지 않았다.
- 실제 설치 파일 설치·설치 후 실행, Windows 화면, updater 서명, 공개 v0.5.0 Release 자산, YouTube/reference-video, 자동 GPU→CPU 제품 전환, 자원·장시간·병렬 측정: **HOLD**. 직접 GPU·CPU CLI 음성 인식은 아래 Wave 2 증거로 `PASS`다.
- G7 병렬 옵션: 같은 입력의 자원 측정을 통과하기 전까지 **사용할 수 없음**.

정확한 FFmpeg asset: `https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-08-17-13-05/ffmpeg-n8.1.2-44-g7c533d0f86-win64-lgpl-shared-8.1.zip` · size 70,837,934 bytes · SHA-256 `681b9ca6d8f9be1e01d8873ad16f8a632f8a22b9653f1044837de6d5979b0fd6`.

## v0.4.0 공개 빌드

상태: **PASS — exact tag·updater 서명·공개 자산·installer smoke·토큰 없는 직접 다운로드 확인 완료**. Authenticode `NotSigned`는 알려진 한계다.

- release PR #14 squash: `a39d62eda4666ac848fe2e7aadaa1e74c7b9a53e`
- annotated tag `v0.4.0`: object `b744801716374c9e10bdbf02e14397467dd43a17` → peel `a39d62eda4666ac848fe2e7aadaa1e74c7b9a53e`
- Release: https://github.com/hangokudao/vod-scout/releases/tag/v0.4.0 · `draft=false` · `prerelease=false` · Latest
- release workflow: run `31239478473` · exact tag SHA · **PASS**
- installer smoke: run `31240405719` · 버전 0.4.0 · runtime 28개 · 재실행 · **PASS**
- draft 조회 workflow 수정 PR #15: squash `2f132d017f384c33d8010a7e4c7edc6665ca38aa`
- 기능 코드 exact HEAD: `e18b73efcb0ea40be812b7da12572e1207854863` (PR #13 squash `16c35f2dfa601790689d7295ceaea12af42169b8`)
- 버전 정본: `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, README와 설치 파일명 모두 `0.4.0`
- 도구: Node.js `v24.18.0`, npm `11.16.0`, rustc/cargo `1.97.1`, Tauri CLI `2.11.4`, Windows 11
- yt-dlp: pinned/bundled/latestStable `2026.07.04` · **PASS**
- SPDX SBOM: SPDX-2.3 · 루트 `vod-scout@0.4.0` · 656 packages · 개인 절대 경로 없음
- GitHub Actions updater secret: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 이름 존재 확인. 값은 읽지 않음
- `npm audit --omit=dev`: 취약점 0개
- 전체 `npm audit`: high 1 · Vite→PostCSS의 개발용 `nanoid@3.3.16`; 제품 실행 코드는 취약 조건인 사용자 정의 0길이 생성기를 호출하지 않음

### 소스 검사

| 검사 | 결과 |
|---|---|
| `npm.cmd test` | **PASS** · 34 tests |
| `npm.cmd run test:security` | **PASS** · 6 tests |
| `npm.cmd run build` | **PASS** |
| `cargo test --manifest-path src-tauri/Cargo.toml` | **PASS** · 54 passed, 1 ignored |
| `cargo test --manifest-path src-tauri/fixture-worker/Cargo.toml` | **PASS** · 5 passed |
| 내장 runtime 재해시 | **PASS** · 28 files |

### 로컬 패키징 참고 (공개 자산 아님)

| 파일 | 크기(bytes) | SHA-256 | 버전 |
|---|---:|---|---|
| `vod-scout.exe` | 15,292,416 | `DB341C2B22D6803964A58D957EE1A658FBB66FCECE205DFFCABA1D1D3AB82703` | 0.4.0 |
| `VOD Scout_0.4.0_x64-setup.exe` | 233,881,664 | `E3CE705383F346FE06AD4410E06E88EBD7758424A06BD37CFA14BC09C128E29B` | 0.4.0 |

- 로컬 환경에는 updater 개인키가 없어 NSIS 본문 생성 뒤 `.sig` 단계에서 예상대로 중단됐다. 공개 태그 빌드는 GitHub Actions secret으로 서명해 PASS했다.
- 로컬 앱 바이너리는 로컬 체크아웃 경로 문자열을 포함해 공개하지 않았다. release workflow의 `RUSTFLAGS --remap-path-prefix`가 적용된 CI 바이너리는 installer smoke에서 빌드 사용자 경로 부재를 확인했다.
- 로컬 EXE와 설치 파일은 Authenticode `NotSigned`다. Authenticode 인증서 구매·생성은 이번 범위가 아니다.

### 실제 P0 자원 근거

- 승인된 약 8시간 53분 입력 전체 작업: `REVIEW_READY`, 후보 8개, 처리 약 4,004.51초
- 최종 작업 크기 7,068,418,335 bytes, 피크 Working Set 합 약 2,054,066,176 bytes
- exact HEAD 순간 임시 파일 최대값은 미재측정. 같은 영상의 기존 측정 peak 14,045,353,616, final 7,068,902,876, peak-final 6,976,450,740 bytes를 구분해 참고

### 공개 자산

| 파일 | 크기(bytes) | SHA-256 |
|---|---:|---|
| `VOD.Scout_0.4.0_x64-setup.exe` | 233,879,794 | `5ba960e35ae9d55512ecc79b8c49da611e7bb3fec12210f87e5496056a3f578a` |
| `VOD.Scout_0.4.0_x64-setup.exe.sig` | 420 | `3d640c525b3101dd63b3299811676598a298c972bde8204e3d965ba3bb1ed887` |
| `latest.json` | 10,882 | `66fc58ff771a4f5f3809b00bf4cfc51e625d333f82445e39d5b99f7ba603214a` |
| `SBOM.spdx.json` | 615,471 | `2485ac5bf74e594da2b0f250709c61f78c63b52effd9d56e1e5e4e92c18600bc` |
| `SHA256SUMS.txt` | 359 | `469908605c31a79de5ad179747796676e5795f0801561228a004e5ea068e08c9` |

- `SHA256SUMS.txt`의 설치 파일·서명·`latest.json`·SBOM 해시와 새로 내려받은 파일 해시 일치.
- GitHub asset digest와 내려받은 파일 해시 일치.
- 토큰 없는 공개 URL 5개 모두 HTTP 200.
- 공개 설치 파일 ProductVersion/FileVersion 0.4.0, updater 서명과 `latest.json` 서명 일치.
- 공개 설치 파일 Authenticode는 `NotSigned`이며 첫 설치 SmartScreen 경고 가능성을 릴리스 노트에 알렸다.

## v0.3.4 공개 빌드

상태: **PASS** — exact merge에서 태그·CI·공개 자산·minisign·인앱 v0.3.3→v0.3.4·DisplayVersion·작업 보존을 확인했다. Authenticode는 승인된 예외로 `HOLD`다.

- exact commit: `a341bae3dcf65ff60ecdd3b1ac5c04dd6140952a` (PR #9)
- 태그·Release: `v0.3.4`, https://github.com/hangokudao/vod-scout/releases/tag/v0.3.4 (ID `365895027`, published `2026-08-06T00:19:47Z`, draft=false, prerelease=false, GitHub latest)
- annotated tag object: `ea5d807a3535f8fede188d255d2fe7fbf4b03bd0` → peel `a341bae…`
- GitHub Actions: run `31057676958` Release Windows app, **success**, head_sha `a341bae…`
- 버전 정본: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, README 설치 링크와 설치 파일명 모두 `0.3.4`
- 도구(소스 게이트): Node.js `v24.18.0`, npm `11.16.0`, rustc/cargo `1.97.1`, Tauri CLI `2.11.4`, Windows 11
- yt-dlp: pinned/bundled/latestStable `2026.07.04` · status `PASS`
- npm audit: info/low/moderate/high/critical **0** (dependencies total 209)
- SPDX SBOM: SPDX-2.3 · 루트 `vod-scout@0.3.4` · 656 packages · 개인 절대 경로 없음

### 공개 자산

| 자산 | 크기(bytes) | SHA-256 |
|---|---:|---|
| `VOD.Scout_0.3.4_x64-setup.exe` | 233,849,362 | `6848c438f8401e964608cb14e8aae34fce1df6551b6142303ddae45cf8942fa3` |
| `VOD.Scout_0.3.4_x64-setup.exe.sig` | 420 | `a8f6bba8a379e030865b3a9f250cdf1d73463e38a2a9534b173a1da8c2b548a5` |
| `latest.json` | 14,563 | `3ee2dad252892b7106d8a5fb466998cedd728d52d29ae241c7066a12f1280ff0` |
| `SHA256SUMS.txt` | 355 | `a4640d71e3495fdbb767b86e01525aa80607c13a5ecf2f4f299e322c541cce30` |
| `SBOM.spdx.json` | 615,471 | `6ff3d7a3130c35a560f06eab00e48258f45165c6ee2d8379dbc0f30193e6de9a` |

- GitHub asset digest와 재다운로드 SHA-256 일치: 5/5 PASS
- 공개 직접 다운로드: 5/5 HTTP 200 (Content-Length 일치)
- `SHA256SUMS.txt`: 포함된 4개 자산 재계산 PASS (정렬·두 칸 공백)
- SBOM: SPDX 2.3, 루트 `vod-scout 0.3.4`, 총 656 packages
- updater 서명: `latest.json`과 `.sig` 일치, 앱 공개키로 독립 minisign 검증 PASS
- 설치 EXE PE ProductVersion/FileVersion `0.3.4(.0)`
- Authenticode: 설치 EXE·업데이트 후 앱 `NotSigned`. 인증서 없어 `HOLD`

### 인앱 업데이트 후 설치 바이너리

| 파일 | 크기 | SHA-256 | 제품 버전 |
|---|---:|---|---|
| 설치 폴더 `vod-scout.exe` | 15,182,336 | `A875FA69CBD2D89FC431A700B388A5E3E6F1C4DD98D012345C81372A96FFCEF7` | 0.3.4 |
| 설치 폴더 `uninstall.exe` | 157,371 | `698CF81370A483D95CF562C416F25409891FE75F62F6EB832077AC44FC50F7D4` | 0.3.4 |

### 실제 업데이트 결과 (v0.3.3 → v0.3.4)

- 시작: 공개 v0.3.3 설치본, 메인 PE `0.3.3`, ARP `DisplayVersion=0.3.2`
- 경로: 설정 인앱 updater만 (`지금 업데이트`). 운영자 직접 설치 EXE 실행·제거 재설치 없음
- 완료: 메인·`uninstall` PE `0.3.4`, 단일 HKCU 제거 항목 `DisplayVersion=0.3.4`, 설정 `최신 상태` / `현재 최신 안정 버전`
- 데이터 보존: 작업 15개 ID 동일, 데이터 파일 2,087개, 누락·추가·해시/크기/mtime 변경 0
- 공개 `latest.json` version 유지 `0.3.4`
- HOLD: Authenticode/SmartScreen (`NotSigned`)

### 실제 YouTube·디스크 측정 (소스 게이트)

- 취소·재개(승인 URL `JN3BO9GLuFU`, release exe 0.3.4 + 내장 yt-dlp/FFmpeg, 격리 E2E): **PASS** — 1차 취소 1,405ms/자식 1,418ms; 재개 후 yt-dlp 재기동; 2차 취소 3,390ms
- 전체 병합 종료 디스크 피크(`sample-disk-usage.mjs` 1s, 표본 816): **PASS** — peak 14,045,353,616 (~13.08 GiB); final 7,068,902,876; peak−final 임시 6,976,450,740; 완성 `source.mkv` 7,060,479,026 · 31,999.981s

### pre-PR 로컬 패키징 참고 (공개 자산 아님)

로컬 `tauri:build` NSIS 본문 해시는 CI 공개 자산과 다르다. 공개 정본은 위 5개 자산 표다.

### v0.3.4 DisplayVersion 게이트

- 과거: v0.3.2→v0.3.3 후 ARP `DisplayVersion=0.3.2`, 바이너리 `0.3.3` — 근본 원인 미확정 `HOLD`
- NSIS 템플릿: Install 절에서 `DisplayVersion`을 번들 `${VERSION}`으로 기록
- 결과: v0.3.3→v0.3.4 인앱 경로에서 ARP·PE 모두 `0.3.4` **PASS**. 추측 훅 없음

## v0.3.3 공개 빌드

상태: **PASS** — exact merge commit에서 테스트·패키징하고 공개 자산의 해시·updater 서명·실제 v0.3.2 업데이트·기존 작업 보존을 확인했다. Authenticode는 승인된 예외로 `HOLD`다.

- exact commit: `5f756af7390325a99f2820a424f7d4ef05334d14`
- 태그·Release: `v0.3.3`, https://github.com/hangokudao/vod-scout/releases/tag/v0.3.3
- GitHub Actions: run `30963107742`, PASS
- 버전 정본: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, README 설치 링크와 설치 파일명 모두 `0.3.3`
- 프런트 테스트: 2개 파일·32개 PASS
- TypeScript + Vite: 1,793 modules PASS
- Rust 핵심: 27개 PASS·1개 무시
- fixture-worker: 5개 PASS
- archive 안전성: 정상 1개·공격 경로 5개, 총 6개 PASS
- npm audit: 취약점 0개

### 공개 자산

| 자산 | 크기 | SHA-256 |
|---|---:|---|
| `VOD.Scout_0.3.3_x64-setup.exe` | 233,845,158 | `53070183C2DE64F61480355A550924A0A89F28C6E83323F262ADC7926251ACF6` |
| `VOD.Scout_0.3.3_x64-setup.exe.sig` | 420 | `485708031741E6B65DD79C58DA858B0CCF757E08217F71E917F7D502DA8B7C46` |
| `latest.json` | 9,513 | `36D7AB3457E9D639569BD2FF79F4285394D425387CCAB4071A441FE41ED4FD70` |
| `SHA256SUMS.txt` | 355 | `CEAF00234AE46A2A19483AEC59D6799AA47AB5E79EF1A1094360A7ECDBECE5C4` |
| `SBOM.spdx.json` | 615,471 | `EC96DF1B4EE28DA16225AC31754D4CBC993DB61D8A0BAB856D05F828CC14C044` |

- GitHub asset digest와 재다운로드 SHA-256 일치: 5/5 PASS
- 공개 직접 다운로드: 5/5 HTTP 200
- `SHA256SUMS.txt`: 포함된 4개 자산 재계산 PASS
- SBOM: SPDX 2.3, 루트 `vod-scout 0.3.3`, 총 656 packages
- updater 서명: `latest.json`과 `.sig` 일치, 공개키로 독립 `minisign-verify 0.2.5` 검증 PASS
- Authenticode: 설치 EXE와 설치된 앱 `NotSigned`. 인증서가 없어 `HOLD`

### 설치된 핵심 바이너리

| 파일 | 크기 | SHA-256 |
|---|---:|---|
| `D:\VOD Scout\vod-scout.exe` | 15,180,800 | `7ED1484CEF507CBE4851E6E5F326FD754BAC71133E9CF713F8449B1D714079C5` |
| `D:\VOD Scout\fixture-worker.exe` | 180,224 | `682E828EA9F57085B6506EECBD0FE01C6A3C5976C6F524426E709772B2F47DFC` |
| `resources/media-tools/yt-dlp/yt-dlp.exe` | 18,226,085 | `52FE3C26DCF71FBDC85B528589020BB0B8E383155CFA81B64DD447BBE35E24B8` |
| `resources/media-tools/deno/deno.exe` | 97,175,328 | `4A2757FE99AFC2C62C46500C8221CFA0189AC4BFB7064141875AD9C0F04B60EF` |
| `resources/media-tools/models/ggml-base.bin` | 147,951,465 | `60ED5BC3DD14EEA856493D334349B405782DDCAF0028D4B5DF4088345FBA2EFE` |
| `resources/media-tools/manifest.json` | 5,185 | `9F86BB181F7942D385534002218A09BC1C144AC310E4B83FDD4D32136D777931` |

runtime manifest schema 5가 열거한 28개 파일을 설치 폴더에서 다시 계산해 28/28 일치를 확인했다.

### v0.3.3 실제 미디어·자원 측정

| 항목 | 값 |
|---|---|
| 입력 | YouTube `JN3BO9GLuFU`, 확인 길이 `32,000초`, 실제 미디어 `31,999.981초`, 1280×720 H.264/Opus |
| 모드·완료 | 빠른 분석, 음성 인식 예산 `6,400초`, 11/11 구간, 17/17 단계, 후보 8개, `REVIEW_READY` |
| 시간 | 첫 시작→검토 준비 `2,161.231초`; 두 번의 중단 사이 대기 제외 누적 활성 `2,068.605초` |
| CPU·RAM | Intel Core i5-8400 6C/6T, RAM `25,709,678,592 bytes` |
| GPU | NVIDIA GeForce GTX 1060 3GB, driver 560.94; 보드 메모리 기준선 `1,682 MiB`, 최대 `2,012 MiB`, 증가 `330 MiB` |
| 프로세스 메모리 | 합산 최대 working set `934,158,336 bytes`; 최대 private bytes `1,123,246,080 bytes` |
| 분석 임시 파일 | WAV 최대 `19,200,078 bytes` |
| 내려받기 임시 파일 | 닫힌 영상·음성 입력 최소 확인값 `7,070,731,050 bytes`; 병합 중 열린 출력 포함 정확한 순간 최대는 `HOLD` |
| 최종 작업 데이터 | 원본·상태·로그·후보 49초·맥락 75초 MP4 포함 `7,097,358,568 bytes`, 66 files |
| 종료 정리 | 관련 자식 프로세스 0개 |

시험 화면 SHA-256: `5CD7F5E36FD05ACFBC0F89EFF169027174F01DEA6D0217A0C93BA3AA9869D148`.

시험용 맥락 MP4: `21,614,567 bytes`, SHA-256 `68216547975653F36E768AA5FA6043D161A25A7E1EA57FCF05628395F97D3576`. 시험용 후보 MP4: `14,123,834 bytes`, SHA-256 `F69F7E28BE257B95CAD45110F238142EE206AE3BCBA2C61D7905F00D01E0E84F`. 두 파일은 배포 자산이 아니다.

### v0.3.3 도구 환경

- Node.js `v24.18.0`, npm `11.16.0`
- rustc `1.97.1`, cargo `1.97.1`, Tauri CLI `2.11.4`
- Windows 11 Pro
- 고정 runtime·모델 SHA-256은 실제 실행이 기록한 `pipeline-provenance.json`, 빌드 manifest, 설치 폴더 재계산 결과가 일치했다.

### 실제 업데이트 결과

- 시작 상태: 공개 v0.3.2, `D:\VOD Scout\vod-scout.exe` 제품 버전 `0.3.2`
- 완료 상태: 앱·실행 파일 `v0.3.3`, 설정 화면 `최신 상태`, 재실행 PASS
- 데이터 보존: 기존 작업 14개와 현재 작업 `#92bbf85a`, 후보 8개 복원
- 체크포인트 보존: `current-job.json`, `media-checkpoint.json`, `pipeline-provenance.json`, `transcript.json`, `chat-motion.json`의 업데이트 전후 SHA-256 일치
- 검토 시 새로 생성된 파일: `review-clips` 로그 2개와 맥락 MP4 1개. 기존 상태·체크포인트를 덮어쓰지 않았다.
- HOLD: HKCU 제거 프로그램 레지스트리의 `DisplayVersion`은 `0.3.2`로 남았지만 실행 파일과 `uninstall.exe`는 `0.3.3`이다.

## 이전 v0.3.2 공개 빌드

빌드 일자: 2026-08-02 (Asia/Seoul)  
대상: Windows x64  
패키지: NSIS current-user install + Tauri updater artifact

## 배포 파일

| 항목 | 값 |
|---|---|
| 설치·updater 파일 | `VOD.Scout_0.3.2_x64-setup.exe` |
| 크기 | `233,848,505 bytes` |
| SHA-256 | `FF9C6F7421793618D8053D6790AF8964326E4B8F6B7C99875616C4501C8A5D01` |
| updater 서명 | `VOD.Scout_0.3.2_x64-setup.exe.sig`, 공개 재다운로드 후 독립 minisign 검증 PASS |
| Authenticode | 없음. 첫 설치에서 SmartScreen 경고 가능 |
| SBOM | SPDX 2.3, npm·Cargo 656 packages |

## 핵심 바이너리

| 파일 | 크기 | SHA-256 |
|---|---:|---|
| release `vod-scout.exe` | 15,104,512 | `B595921F865AD78BC0793BB46E56EB9470F6AC5CB8F0DAD7CCCA1674DDEE4AC3` |
| `yt-dlp.exe` | 18,226,085 | `52FE3C26DCF71FBDC85B528589020BB0B8E383155CFA81B64DD447BBE35E24B8` |
| `deno.exe` | 97,175,328 | `4A2757FE99AFC2C62C46500C8221CFA0189AC4BFB7064141875AD9C0F04B60EF` |
| `ggml-base.bin` | 147,951,465 | `60ED5BC3DD14EEA856493D334349B405782DDCAF0028D4B5DF4088345FBA2EFE` |

runtime manifest schema 5는 FFmpeg·Whisper의 EXE/DLL, yt-dlp, Deno, 모델을 포함한 28개 파일을 열거하고 실행 전에 SHA-256을 검증한다.

## 고정된 외부 리소스

- yt-dlp `2026.07.04`, 빌드 고정본·현재 latest stable 일치 및 YouTube metadata probe PASS
- Deno `2.9.4`, Windows x64
- FFmpeg `n8.1.2-34-g9b6c8969e0-20260801`, Windows x64 LGPL shared
- whisper.cpp `v1.9.1`, CPU/GPU x64, multilingual Whisper `base`; NVIDIA CUDA cuBLAS redistributable `11.11.3.6`의 `cublas64_11.dll`·`cublasLt64_11.dll`을 private `whisper-gpu`에 포함
- 원본 URL·다운로드 SHA-256·runtime SHA-256: `src-tauri/resources/media-tools/manifest.json`
- Apache-2.0 프로젝트 라이선스와 외부 구성요소 라이선스 사본 포함

## 최종 검증 결과

| 검증 | 결과 |
|---|---|
| TypeScript + Vite | PASS |
| 프런트 테스트 | 6 PASS |
| Rust Core | 22 PASS, actual-media 1 ignored |
| fixture-worker | 5 PASS |
| ZIP 안전성 | 정상 1·공격 5, 총 6 PASS |
| npm production audit | 취약점 0 |
| 1시간 5분 29초 실제 한국어 VOD 빠른 분석 | PASS, 약 382초·3청크·전사 241·채팅 261·후보 8 |
| 최신 yt-dlp YouTube metadata probe | PASS, 길이 3929초·extractor Youtube |
| ETA·플레이어·CSV·작업 용량·격리 삭제 | PASS |
| updater minisign | 독립 streaming 검증 PASS |
| 개인 빌드 경로 | release EXE·installer 문자열 스캔 0건 |
| secret·금지 파일 검사 | Git 이력·staged scan PASS, 커밋된 금지 파일 0개 |
| 8시간 실제 영상 | 사용자 마감 승인에 따라 생략, 96분·10청크 예산 단위 테스트만 PASS, 실시간 결과 HOLD |
| 새 Windows 설치 ACL·runtime 28개 재해시·실행 | PASS, public run `30754986062` |
| public Release 직접 다운로드 | PASS, 설치 EXE·서명·manifest·SBOM 4개 SHA-256 및 updater 서명 검증 |
| 설치 후 종료·재실행 | PASS, public Windows runner run `30754986062`, `restart=true` |

실제 테스트 데이터는 `VOD_SCOUT_E2E_DATA_DIR`로 사용자 작업과 분리했다. 기존 사용자 데이터와 `D:\VOD Scout` 설치본은 수정하거나 삭제하지 않았다. 상세 근거는 `validation/v0.3.2.json`과 `docs/V0.3.2-RELEASE.md`에 기록한다.
