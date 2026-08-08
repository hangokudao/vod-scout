# 디버깅·장애 기록

실제로 재현하거나 로그로 확인한 문제만 기록한다. 원인이 확정되지 않은 항목은 `HOLD`로 표시한다.

## 2026-08-08 · v0.4.0 · 초안 Release 설치 파일 조회 실패

- 증상: installer smoke run `31240213255`가 제품 설치 전 `gh release download v0.4.0`에서 `release not found`로 실패했다.
- 원인: 태그 기반 `gh release download` 경로는 Actions의 제한된 토큰으로 아직 공개하지 않은 초안 Release를 찾지 못했다.
- 수정: 인증된 Release 목록에서 입력 태그와 정확히 일치하는 항목 하나와 `_x64-setup.exe` 자산 하나를 선택하고 asset API로 내려받는다. 일치 항목이 없거나 여러 개면 중단한다.
- 회귀 테스트: 같은 `v0.4.0` 초안 Release를 대상으로 installer smoke를 다시 실행해 설치·버전·권한·빌드 사용자 경로·내장 파일 해시·실행·재실행을 확인한다.
- 상태: 수정 workflow 재실행 전 `HOLD`

## 2026-08-08 · v0.4.0 · 릴리스 workflow의 이전 버전 고정값

- 증상: 제품 버전 정본을 올려도 release workflow의 릴리스 문서 경로, installer smoke의 기본 태그·제품 버전, 로컬 자산 생성 스크립트와 README가 `0.3.4`를 계속 가리켰다.
- 원인: 이전 릴리스에서 버전별로 고정한 값을 v0.4.0 release PR 전까지 의도적으로 유지했다.
- 수정: 세 버전 정본과 잠금 파일, `src/releaseNotes.ts`, README, updater 안내, 두 workflow와 자산 생성 스크립트를 `0.4.0`으로 맞춘다.
- 회귀 테스트: 버전 검색, installer smoke, GitHub Release Actions, 공개 자산 이름·해시·직접 다운로드를 순서대로 확인한다.
- 상태: 소스 정렬·로컬 NSIS 본문 버전 `PASS`; 로컬 updater 서명은 개인키가 없어 태그 Actions로 이관. 공개 CI 자산의 경로 치환·서명·installer smoke는 배포 게이트에서 확인한다.

## 2026-08-08 · v0.4.0 P0 · exact HEAD 전체 검증 완료

- 재현 조건: PR #13 exact HEAD `e18b73efcb0ea40be812b7da12572e1207854863`, 승인된 장시간 YouTube 영상, 격리된 작업 폴더와 FAT32 저용량 USB 시험 환경.
- 확인 결과: 저용량 차단은 첫 미디어 바이트 전에 동작했고, 쿠키 없는 짧은 전송 약 154 MB와 전체 다운로드·약 8시간 53분 분석이 완료됐다. 최종 상태 `REVIEW_READY`, 후보 8개, 처리 약 4,004.51초, 최종 작업 크기 7,068,418,335 bytes, 피크 Working Set 합 약 2,054,066,176 bytes였다. 취소·hang·체크포인트 사본과 종료 후 자식 프로세스 없음도 확인했다.
- 회귀 테스트: 프런트 34, 보안 6, Rust 본체 54(1 ignored), fixture-worker 5, 디스크 샘플러 3, 프런트 빌드 모두 PASS.
- 상태: 기능 P0 `PASS`. PR #13은 main `16c35f2dfa601790689d7295ceaea12af42169b8`로 squash 병합됐다.
- 측정 한계: exact HEAD의 순간 임시 파일 최대값은 다시 측정하지 않았다. 같은 영상의 기존 측정 peak 14,045,353,616 bytes와 최종 7,068,902,876 bytes를 참고값으로 사용하며 현재 HEAD의 정밀 측정값이라고 표시하지 않는다.

## 2026-08-08 · YouTube 후속 재시도의 봇 확인

- 증상: 앞선 성공 뒤 측정 보완을 위한 후속 요청에서 `Sign in to confirm you're not a bot`가 반환됐다.
- 원인: YouTube 측의 일시적 접근 제한이다. 제품 자동·로컬 경로는 정상이고 exact HEAD의 앞선 전체 성공이 별도로 존재한다.
- 조치: 로그인 자동화, 쿠키 수집, CAPTCHA 우회 없이 중단했다. 이 외부 제한만을 이유로 장시간 영상을 다시 처리하지 않는다.
- 상태: 알려진 외부 제한. 제품 결함 및 출시 차단으로 분류하지 않음.

## 2026-08-06 · v0.4.0 P0 · 비호환 체크포인트 폐기 후 재개 실패

- 증상: schema 3 또는 입력·도구·언어·후보 계산 버전 불일치로 중간 결과를 버린 뒤, 작업 진행 정보가 앞서 있으면 `작업 스냅샷보다 미디어 체크포인트가 뒤에 있어 자동 재개할 수 없습니다.`로 멈췄다.
- 원인: 호환 체크포인트가 실제로 뒤처진 경우와, 호환되지 않는 중간 결과를 의도적으로 버리고 다시 계산하는 경우를 구분하지 않았다.
- 수정: `media_intermediates_rebuilt`일 때 작업 설정은 유지하고 미디어 단계부터 다시 계산한다. 호환 체크포인트가 실제로 뒤처진 경우는 기존 오류를 유지한다.
- 회귀 테스트: 관련 재개 테스트와 장시간 전체 실행의 체크포인트 사본 검증 PASS.
- 상태: `PASS`.

## 2026-08-06 · 장시간 측정 도구의 체크포인트 파일 잠금

- 증상: 외부 측정 도구가 체크포인트 파일을 공유 없이 읽는 동안 제품의 정상 파일 교체가 `os error 32`로 실패했다.
- 원인: 제품 로직이 아니라 검증 도구의 파일 공유 방식이 atomic rename과 충돌했다.
- 조치: 측정 도구가 대상 트리를 열거나 수정하지 않도록 바꾸고, 제품 샘플러는 대상 밖에 출력하도록 고정했다.
- 회귀 테스트: 디스크 샘플러 3개 테스트와 전체 작업 `REVIEW_READY` PASS.
- 상태: 제품 결함 아님. 검증 도구 수정 `PASS`.

## 2026-08-07 · Unreleased · YouTube 내려받기 직전 저장 공간 가드

- 증상: 분석 단계에는 여유 공간 확인이 있으나, yt-dlp 미디어 전송 전에는 선택 스트림 크기 기반 차단이 없어 장시간 내려받기 중 디스크 고갈 위험이 남았다. (P0 분석 가드 이후 남은 항목)
- 원인(1차): `run_yt_dlp`가 메타데이터 용량 조회 없이 바로 전송 자식을 띄웠다. 병합 피크는 분리 스트림+임시 병합 출력으로 최종 크기보다 크게 관측된다.
- 원인(REV1): 1차 가드가 `download_dir`에 2.2×스트림만 적용해 home/temp/job 단계와 분석 workspace를 빠뜨렸고, `aggregate_required_bytes_by_volume`이 생산 경로에서 쓰이지 않았다.
- 수정: 메타데이터 전용 조회로 정확한 `filesize`·`format_id`·길이를 읽고, 순차 단계(max)·동시 필요(sum) 플래너로 볼륨별 필요 여유를 계산한다. 내려받기 피크 `P=2S+⌊2S/10⌋`, 분리 볼륨 `B=S+⌊S/10⌋`, 분석 `W=estimate_analysis_workspace_bytes(S,duration)`. 실제 전송은 probe가 고른 `format_id` 조합(`298+251` 형태)으로 고정한다. `filesize_approx`만 있거나 포맷/크기 불명이면 fail closed. probe stdout 2MiB·stderr 256KiB 상한 초과 시 자식 정리 후 중단. 원시 전체 JSON/stderr는 저장하지 않고 `tool-logs/yt-dlp.metadata.json`에 duration·formatIds·streamFilesizes 등 최소 구조화 필드만 기록. 네트워크·로컬 로그/권한·도구 실행·안전 용량 불가 안내를 분리.
- 회귀 테스트: exact filesize·format pin, cap 읽기, 최소 로그, 메시지 분리, pure plan, overflow, production path, 볼륨 합산과 `scripts/sample-disk-usage.test.mjs`; 실제 저용량 차단·짧은 전송·장시간 전체 작업.
- 상태: 자동 검사와 실제 제품 경로 `PASS`.

## 2026-08-07 · Unreleased · 디스크 샘플러와 체크포인트 교체 간섭

- 증상: 장시간 피크 측정 중 제품이 체크포인트를 live→`.prev`로 교체할 때 샘플러가 대상 트리를 건드리면 교체가 실패하거나 내용이 바뀔 수 있다는 우려.
- 원인: 샘플러가 대상 안에서 쓰기를 하면 측정 부풀림·교체 경쟁이 생긴다. 기존 구현은 lstat 합산과 출력 경로 외부 강제였으나 체크포인트 교체 스모크가 없었다.
- 수정: 샘플러는 대상 트리에 write/rename/unlink를 하지 않음을 주석·계약으로 고정하고, 출력·stop-file이 target 안이면 exit 2. 스모크에서 샘플 중 `media-checkpoint` 교체 후 live/`.prev` 내용·비관련 `acquisition.json` 해시 불변을 검증한다.
- 회귀 테스트: `node --test scripts/sample-disk-usage.test.mjs` → 3 pass
- 상태: 스모크 `PASS`

## 2026-08-06 · v0.3.4 · 설정 버튼을 알아보기 어려움

- 사용자 제보: 화면 오른쪽 위의 작은 아이콘과 버전 표시가 설정 버튼처럼 보이지 않아, 처음 보는 사용자가 설정 화면의 위치를 알아보기 어렵다. (최초 기록 v0.3.3)
- 원인: 저대비 버전 버튼과 좁은 폭에서 설정 진입점 전체가 숨겨지는 규칙.
- 수정: `.settings-entry`에 톱니바퀴 아이콘, `설정` 문구, 테두리·호버·포커스를 두고, `max-width: 560px`에서는 버전 문자열만 접는다.
- 회귀 테스트: UI 테스트에서 설정 라벨·다이얼로그 열기 PASS. 브라우저에서 1280×900·540×900 설정 진입점 유지 확인. 실제 Tauri 설치 창 수동 회귀는 패키징 후.
- 상태: 단위·설정 진입점 유지 `PASS`, 실제 설치 창 `HOLD`

## 2026-08-06 · v0.3.4 · 다크 모드 입력 카드 대비 부족

- 사용자 제보와 재현 화면: 다크 모드의 새 작업 화면에서 선택하지 않은 입력 카드가 밝은 회색 위에 밝은 글자로 보여 읽기 어려웠다. (최초 기록 v0.3.3)
- 원인: `.source-tabs button` 배경이 `rgba(255, 255, 255, 0.58)`로 고정돼 테마 글자와 충돌.
- 수정: 배경·제목·설명·호버·선택·비활성·포커스를 `var(--panel)` 등 테마 변수로 교체.
- 회귀 테스트: CSS 계약 테스트 PASS. 브라우저 표면 대비(검증 증거) — 선택 탭 helper 5.68:1·title 6.63:1; 비선택 helper 5.89:1·title 13.65:1; 입력 text 13.65:1·label 12.28:1·note 8.97:1. 실제 설치 창 캡처는 패키징 후.
- 상태: 단위·브라우저 대비 측정 `PASS`, 실제 설치 창 `HOLD`

## 2026-08-06 · v0.3.4 · 취소 완료가 1분 이상 지연됨

- 사용자 제보: YouTube 작업에서 취소를 눌렀지만 1분 이상 `취소 중…`과 `worker 종료 요청` 상태가 계속됐다. (최초 기록 v0.3.3)
- 원인: (1) `cancel_requested`를 디스크 저장 뒤에 세워 도구 루프 반영이 늦음 (2) 자식 종료가 무한 `wait`/`join`에 막힐 수 있음 (3) Job Object에 능동 `TerminateJobObject`가 없어 소프트 킬 무시 시 트리가 남을 수 있음.
- 수정: 작업 ID 검증 후 메모리 취소 신호를 먼저 세우고, `terminate_child_tree`에 유예·강제 종료·상한을 두며 yt-dlp 로그 리더를 취소 시 분리한다. 잘못된 작업 ID는 전역 취소를 켜지 않는다.
- 회귀 테스트: main cargo cancel/terminate 관련 단위 테스트 포함 32 pass / 0 fail / 1 ignored, fixture-worker 5 pass. 실제 YouTube(승인 공개 URL, release `vod-scout.exe`, 내장 yt-dlp 2026.07.04, 격리 E2E 데이터 디렉터리): yt-dlp 생존 중 1차 취소 → `CANCELLED` 1,405ms·자식 트리 소멸 1,418ms(하드캡 8s 이내, 외부 강제 kill 없음); 같은 작업 재개 → yt-dlp 재기동·ACQUIRING 진행; 병합 관측 직후 2차 취소 → `CANCELLED` 3,390ms·자식 소멸 3,390ms. Whisper 중 취소는 이 런에서 미실행.
- 상태: 단위 `PASS`, 실제 YouTube 취소·재개 `PASS`, Whisper 중 취소 `HOLD`

## 2026-08-06 · v0.3.4 · 내려받기 병합 중 임시 용량 측정

- 증상: 병합 중 열린 출력 파일을 측정 도구가 읽지 못해 순간 최대 임시 용량이 `HOLD`였다. (v0.3.3)
- 수정: `scripts/sample-disk-usage.mjs`가 메타데이터만으로 재귀 합산해 열린 파일 길이 증가를 포함한다. 출력은 NDJSON 표본과 `.summary.json`이다. `docs/DEVELOPMENT.md`에 Windows 실행 명령을 연결했다.
- 회귀 테스트: 열린 핸들로 65536→131072 성장 시 최종 표본·summary `totalBytes=131072` PASS. 출력이 target 안이면 exit 2. 실제 YouTube(승인 `JN3BO9GLuFU`, ~32,000s, 720p 분리 스트림, release exe + 내장 yt-dlp/FFmpeg, 격리 E2E): 1s 표본 **816**회, 전체 병합 종료 peak **14,045,353,616 bytes** (~13.08 GiB) — 피크 시 열린 `source.temp.mkv` 6,974,603,264 + `source.f298.mp4` 6,589,745,009 + `source.f251.webm` 480,986,041. 최종 totalBytes **7,068,902,876**, peak−final 임시 오버헤드 **6,976,450,740**. 완성 소스 `source.mkv` 7,060,479,026 bytes·길이 31,999.981s, `acquisition.json` 기록, 분리 스트림·`source.temp` 잔존 없음. 시작→획득 완료 824.2s 후 `PROBING`에서 제품 취소(Whisper 전) 614ms.
- 상태: 제한 검증 `PASS`, 전체 병합 종료 피크 `PASS`

## 2026-08-06 · v0.3.4 · 업데이트 뒤 제거 프로그램 DisplayVersion

- 증상: 공개 v0.3.2→v0.3.3 업데이트 뒤 실행 파일·`uninstall.exe` 제품 버전은 `0.3.3`인데 HKCU 제거 프로그램 `DisplayVersion`은 `0.3.2`로 남았다. (절대 설치 경로는 공개 기록에 적지 않음)
- 템플릿 증거: Tauri NSIS 설치 템플릿은 Install 절에서 `WriteRegStr SHCTX "${UNINSTKEY}" "DisplayVersion" "${VERSION}"`을 수행한다. `currentUser` 설치·passive updater 설정과 일치한다.
- 조치: 추측성 앱 레지스트리 수정 훅을 넣지 않았다. 제품 버전만 `0.3.4`로 정렬했다.
- 제어된 재현(공개 v0.3.3 설치본 → 공개 v0.3.4 인앱 updater만 사용): 업데이트 전 메인/`uninstall` PE `0.3.3`·ARP `DisplayVersion=0.3.2` → 업데이트 후 메인/`uninstall` PE `0.3.4`·단일 HKCU 제거 항목 `DisplayVersion=0.3.4`·설정 화면 `최신 상태`. 작업 15개·데이터 파일 2,087개 해시/크기/mtime 불변.
- 원인: 과거 `0.3.2` 잔류의 근본 원인은 확정하지 않았다. v0.3.4 인앱 경로에서는 NSIS Install 경로가 `DisplayVersion`을 `0.3.4`로 기록한 결과만 확인했다.
- 상태: v0.3.3→v0.3.4 결과 `PASS` · 과거 잔류 근본 원인 `HOLD`

## 2026-08-06 · v0.3.4 · 공개 릴리스와 인앱 업데이트

- 증상과 재현 조건: exact merge `a341bae3dcf65ff60ecdd3b1ac5c04dd6140952a`에 annotated tag `v0.3.4`를 달고 Actions run `31057676958`로 초안 자산 생성 후 공개 게시. 설치된 v0.3.3에서 설정 → 업데이트 확인 → `지금 업데이트`만 사용.
- 확인한 결과: 공개 latest·5개 자산 직접 다운로드·API digest·`SHA256SUMS`·SBOM·minisign PASS. 인앱 완료 후 앱 버전·설정 표시 `v0.3.4`/`최신 상태`, DisplayVersion `0.3.4`, 작업·체크포인트 보존 PASS.
- 패키지 검증: 설치 EXE SHA-256 `6848c438f8401e964608cb14e8aae34fce1df6551b6142303ddae45cf8942fa3` (233,849,362 bytes). Authenticode `NotSigned` → 별도 `HOLD`.
- 수정: 제품 코드 추가 변경 없음(포스트 릴리스 문서만 갱신).
- 상태: 공개 배포·인앱 경로 `PASS`, Authenticode `HOLD`

## 2026-08-04 · v0.3.3 · 후보 ID와 맥락 캐시

- 증상: 정렬 후 선택을 후보 배열 위치로 기억하면 목록 순서가 바뀔 때 다른 후보가 선택될 수 있고, 같은 시작 초 구간은 식별자가 충돌할 수 있었다.
- 원인: 화면 선택 키가 안정적인 후보 ID가 아니었고 ID에 시작 초만 포함되어 있었다.
- 수정: 선택을 후보 ID로 저장하고, 후보 ID를 시작·끝 원본 초로 생성했다. 맥락 캐시 키에는 작업·후보·원본 fingerprint·맥락 범위·프록시 종류를 포함했다.
- 회귀 테스트: 동일 입력에서 ID 재생성 일치, 같은 시작·다른 끝 구간 ID 구분, 맥락 캐시 키 각 필드 구분, 이전 snapshot 맥락 필드 기본값 읽기를 통과했다.
- 상태: `PASS`

## 2026-08-05 · v0.3.3 · 후보·맥락 MP4 임시 이름

- 증상과 재현 조건: 승인된 YouTube `JN3BO9GLuFU`를 빠른 분석해 `REVIEW_READY`에 도달한 뒤 첫 후보의 맥락 영상을 만들면 FFmpeg가 `Unable to choose an output format for '...context-<hash>.mp4.tmp'; use a standard extension for the filename or specify the format manually.`과 `Invalid argument`를 기록하고 플레이어가 준비되지 않았다.
- 확인한 로그·파일: 수정 전 stderr SHA-256 `DA5DC45BD811B37F9E676D1BFD81E3BA2925FEAD710B2F327981D6E1E45FD982`; 공통 `prepare_preview`가 후보와 맥락 모두에 최종 `.mp4` 이름 뒤 `.tmp`를 붙였다.
- 원인: FFmpeg는 출력 파일의 마지막 확장자로 컨테이너를 추론하는데 임시 파일이 `.mp4.tmp`라서 MP4 muxer를 선택하지 못했다.
- 수정: 최종 출력 경로의 확장자를 `tmp.mp4`로 바꿔 임시 파일도 MP4 확장자를 유지하고, 성공한 뒤 기존 최종 `.mp4` 경로로 rename하는 흐름은 유지했다.
- 회귀 테스트: `preview_temporary_path_keeps_an_mp4_extension_for_ffmpeg` PASS. 실제 입력에서 H.264/AAC 맥락 75초 `21,614,567 bytes`와 후보 49초 `14,123,834 bytes`를 새로 만들고 플레이어 준비 상태를 확인했다.
- 상태: `PASS`

## 2026-08-05 · v0.3.3 · 측정용 경로의 Asset Protocol 403

- 증상과 재현 조건: 위 MP4 생성 수정 뒤 worktree 아래 측정 폴더에서는 파일이 정상이어도 WebView `<video>`가 `readyState=0`, `networkState=3`, 오류 코드 4였고 `http://asset.localhost/...` 요청이 HTTP 403이었다.
- 확인한 로그·파일: 맥락 MP4는 H.264 Constrained Baseline·AAC LC·1280×720·75초로 `ffprobe`를 통과했다. `tauri.conf.json`은 `$APPLOCALDATA/jobs/*/review-clips/*.mp4`와 `$APPLOCALDATA/e2e-*/jobs/*/review-clips/*.mp4`만 허용한다.
- 원인: 제품 미리보기 문제가 아니라 검증 도구가 `VOD_SCOUT_E2E_DATA_DIR`를 허용 범위 밖의 `src-tauri/target/v033-evidence/...`로 지정한 경로 불일치였다.
- 수정: Asset Protocol 범위를 넓히지 않았다. 실제 사용자 작업과 분리된 `$APPLOCALDATA/e2e-v033-JN3BO9GLuFU`에 1.14 MB의 상태 파일만 복제하고 원본 미디어는 기존 측정 폴더에서 읽어, 허용 경로에 맥락·후보 영상을 새로 만들었다.
- 회귀 테스트: 같은 앱·작업에서 HTTP 403 없이 후보 8개 검토 화면과 영상 플레이어 준비 상태 PASS. 확인 뒤 관련 자식 프로세스 0개.
- 상태: `PASS`

## 2026-08-05 · v0.3.3 · 설치·업데이트 전환

- 증상과 재현 조건: 설치된 공개 v0.3.2에서 GitHub Release의 v0.3.3을 발견해 `지금 업데이트`를 실행했다.
- 확인한 결과: 앱이 다시 실행됐고 화면·설정·`D:\VOD Scout\vod-scout.exe`의 제품·파일 버전이 모두 `0.3.3`이었다. 설정 화면은 `최신 상태`를 표시했다. 기존 작업 14개, 현재 작업 `#92bbf85a`, 후보 8개와 실행 기록을 다시 열었다.
- 패키지 검증: 설치 EXE의 공개 재다운로드 SHA-256은 `53070183C2DE64F61480355A550924A0A89F28C6E83323F262ADC7926251ACF6`이고, updater 공개키와 `.sig`를 사용한 독립 minisign 검증을 통과했다. 설치된 runtime manifest 28개 파일도 전부 해시가 일치했다.
- 데이터 보존: `current-job.json`, 현재 작업의 `media-checkpoint.json`, `pipeline-provenance.json`, `transcript.json`, `chat-motion.json` SHA-256이 업데이트 전과 모두 같았다. 검토 화면을 열 때 기존 파일을 덮어쓰지 않고 `review-clips` 캐시 3개만 새로 생성됐다.
- 수정: 구현 변경 없음. 공개 패키지와 기존 updater 경로를 그대로 검증했다.
- 상태: `PASS`

## 2026-08-05 · v0.3.3 · 업데이트 뒤 제거 프로그램 레지스트리 버전

- 증상과 재현 조건: 위 업데이트와 재실행 뒤 설치 폴더의 메인 실행 파일과 `uninstall.exe`는 `0.3.3`이지만 HKCU 제거 프로그램의 VOD Scout `DisplayVersion`은 `0.3.2`였다. (공개 문서에는 개인 절대 경로를 적지 않는다.)
- 영향 확인: 앱 화면과 updater의 현재 버전은 `v0.3.3`이며 다시 확인했을 때 `최신 상태`였다. 제품 실행과 다음 업데이트 확인은 정상이나 Windows 앱 목록의 버전 표시가 오래된 값일 수 있다.
- 원인: 확정하지 않았다. 레지스트리를 임의 수정하지 않았다.
- 수정: 없음. v0.3.4에서 NSIS 템플릿 증거와 제어 재현 게이트를 문서화했다. 상세는 위 `2026-08-06 · v0.3.4 · 업데이트 뒤 제거 프로그램 DisplayVersion`을 따른다.
- 상태: 과거 잔류 근본 원인 `HOLD` · v0.3.3→v0.3.4 재현 결과는 위 항목 `PASS`

## 기록 형식

- 날짜·버전
- 증상과 재현 조건
- 확인한 로그·파일
- 원인
- 수정
- 회귀 테스트
- 상태: `PASS`, `HOLD`, `BLOCKED`

## 2026-08-02 · v0.3.1 · Whisper SRT UTF-8 오류

- 증상: 1시간 5분 한국어 영상의 전사 재개 중 SRT에 잘못된 UTF-8 바이트가 포함되어 파싱이 중단됐다.
- 원인: Whisper 출력 파일을 유효한 UTF-8 문자열이라고 가정했다.
- 수정: SRT를 바이트로 읽고 손실 허용 UTF-8 변환 후 시간과 문장을 파싱하도록 변경했다.
- 회귀 테스트: 잘못된 바이트가 포함된 SRT 단위 테스트와 같은 장시간 체크포인트 재개를 통과했다.
- 상태: `PASS`

## 2026-08-02 · v0.3.1 · 한국어 전사 환각과 후보 중복

- 증상: 무음 구간에 반복 영어 문구가 생성되고 겹치거나 유사한 후보가 여러 개 표시됐다.
- 원인: Whisper 반복 출력을 그대로 후보에 사용했고 후보 제거가 점수 순위에만 의존했다.
- 수정: 알려진 무음·반복 환각 필터와 전사 정규화, 시간 중첩·문장 유사도 기반 제거를 추가했다.
- 회귀 테스트: 1시간 5분 영상에서 알려진 영어 반복 문구 0개, 후보 시간 겹침 0개를 확인했다.
- 상태: `PASS`

## 2026-08-02 · 설치 폴더 권한과 실행 파일 해시

- 증상: 진행 중인 취약점 점검에서 기존 비표준 설치 폴더가 상위 권한을 상속하며 일반 사용자 수정 권한이 있고, 설치된 `vod-scout.exe` 해시가 현재 release EXE와 다르다고 보고됐다.
- 원인: 기존 비표준 설치본은 v0.3.2 패키지와 다른 산출물이었고 상위 폴더의 공유 쓰기 권한을 상속했다.
- 수정: v0.3.2를 current-user NSIS로 패키징하고 runtime 28개 SHA-256 검증, updater minisign, release EXE 경로 치환을 적용했다.
- 회귀 테스트: private Windows runner에서 새 설치 ACL, runtime 28개 재해시, v0.3.2 실행을 확인했다. 기존 `D:\VOD Scout`는 수정하지 않았다.
- 상태: `PASS`

## 2026-08-03 · v0.3.2 · 깨끗한 CI 릴리스 준비 실패

- 증상: public 태그의 첫 Actions run `30753813573`이 FFmpeg archive SHA 불일치와 fixture sidecar 부재로 패키징 전에 중단됐다.
- 원인: FFmpeg 다운로드가 이동하는 `latest` 자산을 사용했고, 로컬 빌드가 남겨 둔 sidecar를 깨끗한 runner에서도 존재한다고 가정했다.
- 수정: FFmpeg를 `autobuild-2026-08-01-13-21`의 불변 URL과 GitHub asset SHA-256에 고정하고, CI 검증 전에 `npm run sidecar`를 실행한다.
- 추가 수정: Tauri Action의 `VOD.Scout_<version>_x64-setup.exe` 이름을 설치 스모크 workflow가 찾도록 release asset 패턴을 `*_x64-setup.exe`로 맞췄다.
- 회귀 테스트: 새 archive SHA-256, 재생성한 runtime manifest, 반복 `npm run media-tools`, yt-dlp 검사를 통과했다. public release run `30754174632`와 설치·재실행 run `30754986062`가 PASS했다.
- 상태: `PASS`

## 2026-08-02 · v0.3.2 · runtime DLL 무결성 누락

- 증상: 초기 보강안은 `ffmpeg.exe`, `ffprobe.exe`, `whisper-cli.exe`와 모델만 해시로 확인해 같은 폴더의 DLL 바꿔치기를 탐지하지 못했다.
- 원인: 실행 진입 파일만 manifest에 열거하고 동적 라이브러리를 신뢰했다.
- 수정: FFmpeg·Whisper의 모든 EXE·DLL과 모델·yt-dlp·Deno의 상대 경로 목록·SHA-256을 manifest schema 5에 고정했다. 앱은 최초 도구 사용 전에 파일 목록과 전체 해시를 비교한다.
- 회귀 테스트: 파일 목록 불일치·변조 파일 거부 Rust 테스트와 빌드 시 실제 자산 재해시를 수행한다.
- 상태: `PASS`

## 2026-08-02 · v0.3.2 · 고아 작업 전체 삭제 누락

- 증상: snapshot JSON이 손상된 UUID 작업 폴더는 목록에서 제외되어 전체 삭제로도 지울 수 없었다.
- 원인: 전체 삭제가 화면에 복원 가능한 snapshot 목록만 순회했다.
- 수정: 전체 삭제는 `jobs` 바로 아래 UUID 디렉터리를 직접 열거하고, 선택 삭제는 UUID·심볼릭 링크 경계를 유지한다. snapshot 내부 ID와 폴더 ID가 다르면 목록에 표시하지 않는다.
- 회귀 테스트: 손상 snapshot과 미디어를 가진 격리 UUID 폴더가 전체 삭제되고 비 UUID 외부 파일은 보존되는 테스트를 추가했다.
- 상태: `PASS`

## 2026-08-02 · v0.3.2 · CSV 저장 경계

- 증상: 프런트엔드가 전달한 절대 `.csv` 경로라면 백엔드가 사용자 쓰기 가능 위치의 기존 파일을 덮어쓸 수 있었다.
- 원인: 저장 대화상자를 프런트엔드에서 열고 최종 경로를 IPC 인자로 신뢰했다.
- 수정: Rust 백엔드가 네이티브 저장 대화상자를 직접 열어 선택한 로컬 `.csv` 경로만 사용한다. 심볼릭 링크와 비 CSV 경로를 거부하고 위험한 셀 접두사를 무력화한다.
- 회귀 테스트: 다섯 가지 수식 접두사와 NUL 제거 Rust 단위 테스트를 통과했다.
- 상태: `PASS`
