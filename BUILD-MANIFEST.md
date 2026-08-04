# VOD Scout 빌드 명세

## v0.3.3 PR 단계

상태: **HOLD** — 승인된 실제 미디어·8시간대 빠른 분석과 pre-merge 실행 파일은 검증했다. 설치 EXE와 서명된 updater 자산은 PR 병합 승인 뒤 exact merged commit에서 만든다.

- 예정 설치 파일: `VOD.Scout_0.3.3_x64-setup.exe`
- 크기·SHA-256: 미생성
- updater 서명·`SHA256SUMS.txt`·SBOM 갱신: 미생성. 현재 `SBOM.spdx.json`은 v0.3.2이므로 v0.3.3 배포 근거로 사용하지 않는다. GitHub Actions secret 이름 2개는 존재하지만 현재 로컬 signing key는 없음
- Authenticode: CurrentUser·LocalMachine 코드 서명 인증서 0개, `signtool.exe` 미확인, `HOLD`
- pre-merge no-bundle `vod-scout.exe`: 버전 `0.3.3`, `15,187,456 bytes`, SHA-256 `8F6F052A61773F0963C4460B6AB1922FD4B6AAC0766672A25104BF86DDBDED7E`; 최종 설치본 근거가 아니며 병합 뒤 재생성 필요
- 프런트 테스트: 2개 파일·32개 PASS
- TypeScript + Vite: PASS
- Rust 핵심: 27개 PASS·1개 무시
- fixture-worker: 5개 PASS
- archive 안전성: 정상 1개·공격 경로 5개, 총 6개 PASS

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

### v0.3.3 pre-merge 도구 환경

- Node.js `v24.18.0`, npm `11.16.0`
- rustc `1.97.1`, cargo `1.97.1`, Tauri CLI `2.11.4`
- Windows 11 Pro
- 고정 runtime·모델 SHA-256은 실제 실행이 기록한 `pipeline-provenance.json`과 `src-tauri/resources/media-tools/manifest.json`이 일치했다.

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
