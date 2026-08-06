# VOD Scout 빌드 명세

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
