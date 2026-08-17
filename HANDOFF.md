# VOD Scout 인계서

현재 게이트: **v0.4.0 공개 완료 · v0.5.0 G1·G2·G3 코드·자동 테스트 PASS · 실제 미디어·실제 Windows UI 검증 HOLD**

## 현재 정본

| 항목 | 값 |
|---|---|
| 저장소 | `hangokudao/vod-scout` |
| 문서 기준 | `origin/main` `eee71e04776a6179c289167596e9d82d52e94e13` |
| 최신 공개 Release | `v0.4.0` · https://github.com/hangokudao/vod-scout/releases/tag/v0.4.0 |
| 현재 제품 버전 | `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` 모두 `0.4.0` |
| 문서 변경 추적 | PR #18 merged · roadmap correction committed |
| 현재 구현 | `codex/v050-g2-whisper-device` 로컬·미 push 구현 |
| v0.5.0 상태 | G1·G2 코드·자동 테스트·Rust 빌드 PASS, 실제 GPU·자원·Windows UI 검증 `HOLD` |

v0.4.0의 기능·장시간 입력·설치·공개 자산 검증 근거는 [v0.4.0 릴리스 기록](docs/V0.4.0-RELEASE.md)과 [빌드 명세](BUILD-MANIFEST.md)를 따른다. 이 문서에서 해시와 실행 결과를 중복 관리하지 않는다.

## v0.5.0 대상과 확정 원칙

- 치지직 버츄얼 스트리머의 저스트채팅·게임 VOD
- 주요 시청자 규모 약 5~30명
- 방송 화면 안의 익명 채팅 오버레이
- 작성자 ID와 서로 다른 참여자 수를 알 수 있다고 가정하지 않음
- 채팅은 현재 VOD의 고정 채팅 영역 움직임이 내부 평소 분포보다 늘었는지를 보조 신호로만 사용
- 채팅만으로 후보를 만들지 않고 오디오·말하기·화면 근거와 함께 사용
- 움직임 분석만으로 일반 문장과 이모티콘을 구분했다고 표시하지 않음
- 외부 AI·유료 API 없이 핵심 작업 완료

## v0.5.0 포함 범위

1. YouTube 제작자 한국어 자막 우선, 한국어 자동 자막 대체, 시간·품질 검증과 사용할 수 없거나 품질이 낮거나 검증할 수 없는 자막·구간의 로컬 Whisper 대체
2. 실제 시험을 통과한 GPU 우선 실행, CPU 대체 처리와 `자동`·GPU·CPU·속도·정확도·CPU 사용량 제어
3. 기존 음성 인식 품질 안전장치 재사용 계약과 선택 후보 재음성 인식
4. 분석 중 렉 원인 측정과 자원 제한
5. 후보 8·20·30개 설정, 후보 내용 품질과 선택 이유 표시
6. 여러 영상의 기본 순차 대기열과 재실행 복원
7. 순차 대기열이 안정된 뒤 측정하는 제한적 병렬 처리

상세 범위와 완료 조건은 [v0.5.0 계획](docs/V0.5.0-PLAN.md), 릴리스 게이트는 [v0.5.0 릴리스 작업 정본](docs/V0.5.0-RELEASE.md)을 따른다.

## 비범위와 후속 목록

- YouTube 자막 검색 UI와 검색 결과의 원본 시각 이동
- 수분 단위 이야기 후보와 멀리 떨어진 관련 장면 연결
- 채팅 글자 인식·작성자 식별·채팅 영역 자동 탐색
- 사용자 API·외부 AI 후보 재정렬
- 완성 쇼츠 렌더링·자동 게시
- Authenticode 인증서 구매·적용

별도 승인과 별도 버전 없이 위 항목을 구현하지 않는다.

## 종료가 보장되는 규칙

- 실패·취소 작업은 자동 재시작하지 않고 사용자가 요청할 때만 재시작한다.
- 앱 재실행 시 실행 중이던 작업은 `INTERRUPTED`로 멈추며 사용자가 재개·취소하기 전에는 자동 진행하지 않는다.
- 품질 경고만으로 후보를 자동 재분석하지 않는다.
- 같은 청크의 GPU→CPU 자동 전환은 한 번만 허용하고 전환 상태를 저장한다. CPU도 실패하면 작업을 `FAILED`로 끝낸다.
- 한 작업의 실패가 다음 대기 작업을 막지 않는다.
- 병렬 처리의 자원 조건이 맞지 않으면 순차 처리로 한 번 전환해 상태를 저장하고 앱 재실행 뒤에도 자동으로 켜지 않는다.

## Oracle 읽기 전용 검수 결과

- 세션: `vod-541e-v050-doc-independen`
- 13개 Markdown 문서만 전송했으며 파일 수정·패치·코드 작성을 요청하지 않았다.
- 요청 모델은 `gpt-5.6-sol`이었지만 브라우저의 현재 선택 모델을 사용했고 도구가 실제 선택 모델을 별도로 검증하지 못했다.
- 최초 판정은 `HOLD`였으며 필수 수정 7개는 앱 재실행과 단일 실행권, 자동 대체 처리 상태 보존, 기존 후보 판정, 실행 중 삭제, 자원 상한, 품질 조건 효과, 채팅 기준선이었다.
- 위 7개는 현재 문서에 최소 계약으로 반영했다. 같은 요청을 반복하지 않기 위해 두 번째 Oracle 검수는 실행하지 않았으므로 `Oracle PASS`라고 기록하지 않는다.

## 이전 문서 보정 실행 기록

- 동시 worker 상한: 1개
- 브랜치: `codex/v050-roadmap-correction`
- 미커밋 정본: 지정된 여섯 개 문서
- 요청 모델: `gpt-5.6-luna`
- 실제 적용 모델: `gpt-5.6-luna`
- Oracle: 없음
- 수행 범위: 지정된 여섯 Markdown 문서의 문서 보정
- 수행하지 않음: 제품 구현, 통합, push, PR, merge
- 정본 반영: `origin/main` `eee71e04776a6179c289167596e9d82d52e94e13`, PR #18 merged, roadmap correction committed

## 일시 중지된 기존 작업

worktree `codex/v050-transcript-quality`는 이미 존재하며 커밋되지 않은 변경이 있다. 이 작업은 일시 중지·보존하고 지금 편집하거나 통합하지 않는다. 그 구현은 나중에 음성 인식 품질 안전장치로만 재사용할 수 있다.

## 구현 순서

1. 사용 권한과 보관 범위를 확인한 YouTube 기준 영상에서 자막 선택·시간 품질과 로컬 대체를 재현한다.
2. 실제 시험 음성 인식이 성공한 경우의 GPU 경로, CPU 대체와 사용자 제어를 구현·검증한다.
3. YouTube 자막 구현과 실제 기준 영상 검증이 PASS가 된 뒤에만 기존 작업의 음성 인식 품질 안전장치 재사용을 별도로 검토하고 선택 후보 재음성 인식을 구현·검증한다.
4. 자원 측정과 앱 반응성 제한을 구현·검증한다.
5. 후보 수·내용 품질을 구현·검증한다.
6. 순차 대기열과 복원을 구현·검증한다.
7. 앞 단계가 모두 PASS일 때만 제한적 병렬 처리를 측정한다.

기능별 검증이 끝나기 전에 다음 기능을 미리 구현하지 않는다. 구현·PR·병합·배포 승인은 서로 별개다.

## G2 구현 및 자동 검증

- `JobSnapshot`과 미디어 체크포인트에 Whisper 장치(`자동(GPU 우선)`·GPU·CPU), 프로필(`빠르게`·`균형`·`정확하게`), CPU 스레드 자동·1~32개를 저장·복원한다.
- CPU 명령에는 `-ng`를 명시하고, GPU는 백엔드 로그와 비어 있지 않은 음성 인식 결과를 함께 확인한 실제 실행만 성공으로 기록한다. GPU 실패는 같은 구간에서 CPU 한 번으로만 대체하며, 시도 전·후 상태와 실패 이유를 체크포인트에 저장한다.
- 기존 v0.4 체크포인트는 schema 4의 호환 필드를 확인하고 CPU 기본값으로 완료 청크를 보존해 재개한다. CUDA 11.8 Windows x64 런타임은 현재 다운로드하지 않았으며, 준비 스크립트가 고정 URL·SHA-256을 확인하고 `whisper-gpu` 실행 파일·DLL을 manifest schema 6에 생성한다.
- 자동 검증 PASS: `cmd.exe /c npm.cmd test` 36개, `cmd.exe /c npm.cmd run build`, `cargo.exe test --manifest-path src-tauri/Cargo.toml` 87 passed·1 ignored, `cargo.exe test --manifest-path src-tauri/fixture-worker/Cargo.toml` 5 passed, `node --test scripts/archive-safety.test.mjs scripts/prepare-media-tools.test.mjs` 8 passed, `git diff --check`.
- 검증 HOLD: 실제 GPU 장치 실행·실제 Windows UI·실제 미디어 장시간 검증은 실행하지 않았다. 설치 파일 생성·배포도 이 작업 범위가 아니다.

## G3 구현 및 자동 검증

- 음성 인식 결과의 비정상 반복·깨진 문자 품질 정보를 원문과 분리해 저장하고, 불확실한 원문은 후보 제목·요약·화면·CSV에서 가리면서 오디오·화면 근거 후보는 유지한다. 웃음·감탄·노래·의도적 반복은 원문을 삭제하지 않는다.
- `REVIEW_READY`에서 선택한 후보만 내장 G2 Whisper 런타임·모델로 다시 음성 인식하며, 실행 ID·개정·STARTED와 정확히 하나의 COMPLETED/FAILED·원문·표시 결과·실제 백엔드/대체 근거를 작업 데이터에 저장한다. 기존 후보 판정과 순위는 유지하고 자동 재분석·재정렬은 하지 않는다.
- 자동 검증 PASS: `cmd.exe /c npm.cmd test` 41 passed, `cmd.exe /c npm.cmd run build`, `cargo.exe test --manifest-path src-tauri/Cargo.toml` 96 tests total·95 passed·1 ignored, `cargo.exe test --manifest-path src-tauri/fixture-worker/Cargo.toml` 5 passed, `node --test scripts/archive-safety.test.mjs` 6 passed, `git diff --check`.
- 실제 선택 후보 미디어를 이용한 Whisper 재실행·GPU/CPU 대체 증거와 실제 Windows UI 흐름은 입력·환경이 없어 실행하지 않았다.

## G1 이후 HOLD

- YouTube 기준 영상의 사용 권한·보관 범위와 실제 입력 미확정
- 실제 자막 시간 오프셋·트랙 품질 검증 미실행. 기준 영상 전에는 시간 허용 오차나 시간 오차 합격 임계값을 정하지 않는다.
- `codex/v050-transcript-quality`는 커밋되지 않은 변경이 있는 상태로 일시 중지·보존 중이며, 현재 편집·통합하지 않는다.
- 실제 G1 YouTube 자막·시간 오프셋·장시간 검증과 v0.5.0 GPU·자원·사용자 화면 흐름 검증 미실행

## 다음 정확한 작업

1. 기준 영상의 사용 권한·보관 범위와 실제 비교 입력을 먼저 확정한다.
2. G1 실제 YouTube 자막 선택·시간 오프셋·Whisper 혼합 처리 검증을 수행한다.
3. 그 뒤에만 `codex/v050-transcript-quality`의 음성 인식 품질 안전장치 재사용을 별도로 검토하며, 그 전에는 편집하거나 통합하지 않는다.

원본 작업 폴더, 기존 설치 폴더, 기존 작업 데이터와 개인 파일은 수정하거나 삭제하지 않는다. 공개 문서에는 비밀값·개인 영상·개인 절대 경로를 넣지 않는다.

# G4 자원 제한 구현 인계

## 현재 완료 상태

- G4 구현은 현재 worktree의 `codex/v050-g4-resource-limits` 브랜치에만 반영했다.
- 작업 데이터 스키마 5와 v0.4 체크포인트를 깨지 않도록 자원 정책·단계별 자원 기록·자원 제한 실패 기록을 선택 필드로 추가했다.
- FFmpeg 오디오, Whisper, 채팅 영역 디코딩, 후보 미리보기, UI 반응성 단계를 구분해 표시한다. 실제로 수집할 수 없는 CPU·메모리·임시 파일 값은 `null`과 `unavailableReasons`로 표시하며, 기본 경고·강제 중단 기준은 미설정 `HOLD`다.
- 분석·수동 후보 음성 인식·미리보기는 하나의 heavy-tool gate를 공유한다. 작업 종료·취소·실패 뒤 `ownedChildProcesses: 0`을 저장하고, 주입된 하드 제한은 현재 작업만 `FAILED`로 끝내며 마지막 완료 단위와 정확한 이유를 보존한다. 낮은 설정 자동 재시작은 없다.

## G4 자동 검증

- `cmd.exe /c npm.cmd test`: **PASS, 42 passed**
- `cmd.exe /c npm.cmd run build`: **PASS**
- `cargo.exe test --manifest-path src-tauri/Cargo.toml`: **PASS, 104 passed, 1 ignored**
- `cargo.exe test --manifest-path src-tauri/fixture-worker/Cargo.toml`: **PASS, 5 passed**
- `node --test scripts/archive-safety.test.mjs scripts/prepare-media-tools.test.mjs`: **PASS, 8 passed**
- `git diff --check`: **PASS**
- Rust 집중 테스트: 자원 정책 경고·강제 중단 판정, 미측정값 직렬화, 기존 스냅샷 기본값 복원을 확인했다.
- UI 집중 테스트: 미측정값을 0과 구분하는 `측정 불가` 표시를 확인했다.

## 실제 측정 HOLD

- 실제 기준 영상에서 단계별 CPU·메모리·임시 파일 피크와 UI 반응성 수치를 수집하지 않았다. 따라서 수치 기준의 PASS나 성능 개선 주장을 하지 않는다.
- 실제 Windows 화면·실제 미디어에서 취소·자식 프로세스 0개와 하드 제한 종료를 실행하지 않았다.
- Oracle 없음. push·merge·PR·Release·deploy는 수행하지 않았다.

## 다음 정확한 작업

1. 승인된 기준 영상과 Windows 실행 환경에서 G4 실제 자원 측정을 수행한다.
2. 측정값이 확보된 뒤에만 경고·강제 중단 수치를 별도로 고정한다.
