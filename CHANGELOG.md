# VOD Scout 변경 이력

이 문서는 사용자에게 영향을 주는 업데이트를 기록한다. 각 릴리스는 기능 추가뿐 아니라 버그 수정, 보안 수정, 알려진 문제를 함께 적는다.

## 0.5.0 local candidate - 2026-08-18

상태: **G1~G7 구현·자동 검증 PASS · 실제 입력/장치/UI/자원·장시간/패키지 HOLD**

### Added

- G1~G4의 자막 provenance·GPU/CPU 대체·음성 인식 품질·자원 제한 상태를 작업 데이터에 기록한다.
- G5 후보 `8/20/30`개 설정, 후보 pool/evidence 분리, 품질 경고와 후보 개정·판정 보존을 통합했다.
- G6 여러 영상의 독립 작업·순차 대기열·실행권·`INTERRUPTED` 복구·작업별 삭제 경계를 통합했다.

### Changed

- 로컬 소스·package lock·Cargo lock·Tauri 설정·release notes·installer workflow/helper 기대 버전을 `0.5.0`으로 맞췄다.
- G7은 측정되지 않은 병렬 처리를 fail-closed하고 순차 처리로 고정한다. 실제 측정 전 선택 항목은 제공하지 않는다.

### Fixed

- 후보 pool과 화면 목록의 동기화가 후보 수 변경·정렬·수동 재음성 인식 뒤에도 기존 판정을 잃지 않도록 했다.
- 대기열 저장 실패·복구·실행권·실패 작업의 다음 작업 진행·실행 중 삭제 순서를 닫아 두었다.

### Security

- 새 외부 AI·유료 API·API 키 저장·원본 미디어 전송 경로를 추가하지 않았다.
- archive/media-tool/경로·자식 프로세스·체크포인트 경계 자동 테스트를 통과했다.

### Known issues

- 실제 YouTube/reference-video, GPU, Windows UI, resource/long-run, parallel measurement는 실행하지 않아 `HOLD`다.
- `npm run tauri:build`는 고정 FFmpeg URL HTTP 404로 NSIS 전에 중단됐다. installer/PE hash/`.sig`/공개 v0.5.0 자산은 검증되지 않았다.
- 자동 검증: npm 49, Rust 126 passed·1 ignored, fixture 6, security 6, archive/media-tool 11 passed. `npm audit`의 개발 의존성 high 1건은 제품 경로 취약점으로 단정하지 않는다.

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
