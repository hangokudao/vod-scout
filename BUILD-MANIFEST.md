# VOD Scout 빌드 명세

## v0.3.4 pre-PR 패키징 증거 (설치·공개 전)

상태: **부분 PASS · 서명·설치·공개 HOLD** — 소스 게이트 전부 통과했고, 로컬 `npm.cmd run tauri:build`가 NSIS 설치 EXE까지 생성했다. `createUpdaterArtifacts` 서명 단계에서 `TAURI_SIGNING_PRIVATE_KEY` 부재로 종료해 updater `.sig`/`latest.json`/체크섬 생성은 `HOLD`다. 아래 설치 EXE는 **pre-PR 패키징 증거**이며 공개 릴리스 자산이 아니다. 설치·DisplayVersion·updater 인앱 교체·공개 재다운로드는 이 단계에서 하지 않았다. Authenticode는 인증서 없어 `HOLD`다.

- 작업 브랜치: `codex/v034-stability-release` · HEAD `7fd92cb2d28b43e2e95ba2b39a70581ddc0a3d2a` (빌드 시점; 이후 문서/SBOM 갱신 가능)
- 도구: Node.js `v24.18.0`, npm `11.16.0`, rustc/cargo `1.97.1`, Tauri CLI `2.11.4`, Windows 11
- 버전 정본: `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, README 설치 링크·`VOD.Scout_0.3.4_x64-setup.exe` 파일명, release notes, workflows, `scripts/generate-release-assets.mjs` 모두 `0.3.4`
- 소스 게이트: `npm ci` 생략(lockfile 일치 `npm ls` 0 exit) · sidecar/media-tools/check:yt-dlp/test/test:security/build · cargo fmt/test · fixture-worker test · `npm audit` 0 · secret/path/version 스캔 · `git diff --check` 모두 PASS (상세 소요 시간은 작업 리포트)
- yt-dlp: pinned/bundled/latestStable `2026.07.04` · status `PASS`
- npm audit: info/low/moderate/high/critical **0** (dependencies total 209; 도달 가능 취약점 단정 없음)
- SPDX SBOM: `npm.cmd run sbom` 재생성 · SPDX-2.3 · 루트 `vod-scout` `0.3.4` · 656 packages · 로컬 절대 빌드 경로 없음

### pre-PR 로컬 번들 (gitignore · 스테이징 금지)

| 산출물 | 크기(bytes) | SHA-256 | 비고 |
|---|---:|---|---|
| `src-tauri/target/release/bundle/nsis/VOD Scout_0.3.4_x64-setup.exe` | 233,851,958 | `CD5033E86A095F3118D24CAFD2535238B163592D755D0B7BF317028CEC198DA0` | Tauri NSIS 원본 파일명(공백). 공개 계약명은 `VOD.Scout_0.3.4_x64-setup.exe`. ProductVersion/FileVersion `0.3.4`. Authenticode `NotSigned`. **공개 자산 아님** |
| `src-tauri/target/release/vod-scout.exe` | 15,188,992 | `71BF0A68A5677B8B3F7CF2C102C963671B976D4D9059148AB320D4DC807ECBED` | ProductVersion/FileVersion `0.3.4`. Authenticode `NotSigned`. PE ASCII 절대 사용자 경로 문자열 0건 |
| `src-tauri/target/release/fixture-worker.exe` | 180,224 | `C6651D8004EBCCF7785C1C6C54994D6B5332E639CFF4A695FFCA25E0FC57DB10` | Authenticode `NotSigned` |
| `SBOM.spdx.json` (저장소 추적) | 615,471 | `6FF3D7A3130C35A560F06EAB00E48258F45165C6EE2D8379DBC0F30193E6DE9A` | 재생성 PASS |

- 서명 실패 원문(키 값 비공개): `A public key has been found, but no private key. Make sure to set TAURI_SIGNING_PRIVATE_KEY environment variable.`
- updater 산출: `.sig` / `latest.json` / `SHA256SUMS.txt` **미생성** → `HOLD` (키 주입 후 공개 릴리스 파이프라인에서 생성)
- `scripts/generate-release-assets.mjs`: 정품 `.sig` 없어 **미실행** (위조 서명 금지)
- 번들 준비 리소스: `src-tauri/resources/media-tools` schema 5 · runtimeHashes 28 · yt-dlp/deno/ffmpeg/ffprobe/whisper/model 존재. `manifest.json`의 `preparedAt`만 게이트 실행으로 갱신됨
- 설치·ARP DisplayVersion·인앱 updater·공개 재다운로드·GitHub Actions·Authenticode: **HOLD**
- 실제 YouTube 취소·재개(release `vod-scout.exe` 0.3.4 + 내장 yt-dlp `2026.07.04`/FFmpeg, 격리 E2E 데이터 디렉터리, 승인 URL `JN3BO9GLuFU`, 기본 720p): **PASS**
  - 1차 취소: 단말 `CANCELLED` 1,405ms, 소유 yt-dlp 트리 소멸 1,418ms (하드캡 8s, 외부 kill 없음)
  - 재개: 동일 작업에서 yt-dlp 재기동·`ACQUIRING` 진행, 스냅샷 손상 없음
  - 2차 취소(병합 관측 직후): 단말·자식 소멸 3,390ms `CANCELLED`
  - 기존 설치 ARP `DisplayVersion=0.3.2`·사용자 작업 디렉터리 job 집합 해시 전후 불변
- 실제 YouTube 전체 병합 종료 디스크 피크(`scripts/sample-disk-usage.mjs`, 1s, 표본 816, 약 826s, 시작→획득 824.2s): **PASS**
  - peak totalBytes **14,045,353,616** (~13.08 GiB) at 2026-08-05T22:41:45.421Z (병합 직전 끝: 열린 `source.temp.mkv` + 분리 스트림 동시)
  - 피크 구성(이름만): 열린 `source.temp.mkv` 6,974,603,264 · `source.f298.mp4` 6,589,745,009 · `source.f251.webm` 480,986,041
  - 최종 totalBytes **7,068,902,876** (~6.58 GiB); peak−final 임시 오버헤드 **6,976,450,740** (~6.50 GiB)
  - 완성 소스: `source.mkv` 7,060,479,026 bytes · 길이 31,999.981s · `acquisition.json` schema 1 · 분리 스트림/`source.temp` 잔존 없음
  - 제품 상태 `ACQUIRING`→`PROBING`(미디어 확인); FFmpeg 병합 관측; Whisper 전 제품 취소 614ms/`CANCELLED`

### v0.3.4 DisplayVersion 게이트

- 확인된 로컬 불일치(개인 절대 경로 비공개): ARP `DisplayVersion=0.3.2`, 설치 바이너리 제품 버전 `0.3.3`
- Tauri NSIS 템플릿: Install 절에서 `DisplayVersion`을 번들 `${VERSION}`으로 기록
- 조치: 추측 훅 없음. v0.3.4 설치·updater 재현에서 ARP·PE가 `0.3.4`인지 확인 후 닫거나 최소 수정

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
- whisper.cpp `v1.9.1`, CPU x64, multilingual Whisper `base`
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
