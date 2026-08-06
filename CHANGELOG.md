# VOD Scout 변경 이력

이 문서는 사용자에게 영향을 주는 업데이트를 기록한다. 각 릴리스는 기능 추가뿐 아니라 버그 수정, 보안 수정, 알려진 문제를 함께 적는다.

## Unreleased

### v0.4.0 P0 (개발 경로 · 제품 버전 0.3.4 유지 · 릴리스 아님 · 병합 보류)

공개 제품 버전 정본·설치 EXE·태그·배포는 **0.3.4 그대로**다. v0.4.0 공개 릴리스로 읽지 않는다.
**모든 P0가 PASS가 아니므로 코드 PR과 문서 PR을 병합하지 않으며, v0.4.0 버전·설치·배포 계획을 확정하거나 포함하지 않았다.**

#### Fixed (main · PR #11 이미 병합)

- 호환되지 않는 미디어 체크포인트(schema 3·입력 지문·도구/모델 해시·언어·후보 계산 버전 불일치)를 버린 뒤 작업 진행 단위가 이미 앞서 있으면, “작업 스냅샷보다 미디어 체크포인트가 뒤에 있어…”로 재개가 멈추던 문제를 수정했다. 미디어 중간 결과만 다시 계산하고 작업 id·소스·분석 설정은 유지한다 (H5F / PR #11 `d13b864` · main `cca7a9e…`).

#### Added (개발 경로 · 미배포)

- (main) 체크포인트 schema 4: 입력 지문·바이트·런타임 SHA-256·언어·`rules-v0.4.0-p0` 후보 계산 버전.
- (main) 범위 지정 후보를 분석 구간 안으로 제한하고 음량·발화 근거 없는 창 제외.
- (main) 체크포인트·스냅샷 등의 마지막 정상 세대(`.prev`) 보존 후 교체.
- (main) 분석 시작 전 작업 폴더 볼륨 여유 공간 검사(부족 시 한국어 설명).
- (**PR #13 draft/open/unmerged**, 정적 고정 HEAD `e18b73efcb0ea40be812b7da12572e1207854863`) YouTube 다운로드 저장 공간 가드·메타데이터 점검. Oracle 후속: 메타 점검이 고른 정확한 `format_id`를 실제 `--format`에 결박, **exact `filesize`만** 허용(대략 크기 거부), stdout **2 MiB**/stderr **256 KiB** 상한·자식 정리, 최소 구조화 로그, 실패 안내 분리. **main 미병합.**

#### Validation (2026-08-07 · 현재 정본)

증거: `D:\vod-scout-p0-evidence-20260807-023415-pr13-4597010\reports\P0-validation-report-ko.md` · `evidence-index.json` · `ORACLE-final-review-4597010-01a3f6b.md` · `PR13-final-rereview-e18b73e.md`.

- PR #13: **draft · open · unmerged**. 코드 정적 고정 HEAD **`e18b73efcb0ea40be812b7da12572e1207854863`**. 최종 독립 재리뷰 **PASS** · 정규(canonical) 6개 명령 **PASS** · 샘플러 3/3 **PASS** (`PR13-final-rereview-e18b73e.md` · SHA-256 `A4B6752524146DD9BDC9033D80DD6CF4D7E37DC5CDCC31CF0F8A2A4EBC340099` · `evidence-index.json` `postOracleCodeFix`). 제품 P0 merge readiness **아님**.
- 실제 제품 E2E는 기존 HEAD **`4597010c99bf8432f1fe15ba34269fd63d5daa7c`**에서 **한 번만** 수행했다. 새 HEAD `e18b73e…`에서는 실제 YouTube·low-space·장시간 분석을 **재실행하지 않았다** → 제품 P0는 계속 **BLOCKED/HOLD**.
- 유일한 신규 제품 작업 `fd8c1cc5-3bfc-4e02-b97a-036ff1009f5e` (`4597010…`): 메타데이터·저장 공간 점검 **PASS** → 부분 전송 약 **10 MB** 후 YouTube **HTTP 403** → 전체 신선 내려받기 **BLOCKED** · 중단 없는 장시간 분석 **BLOCKED**. 재시도·재개 없음. 403 원인 귀속(제공처·네트워크 vs PR #13 2단계 acquisition 회귀) **HOLD** — 외부 문제로 단정하지 않음.
- 실제 low-space E2E: **HOLD** (`safe low-space E2E environment unavailable`). 자원 메트릭 요약 **HOLD**. 설치 폴더 `D:\VOD Scout` 전후 총 **453,110,594** bytes / **47** files 동일이나 파일별 SHA 매니페스트 없음 → 내용 불변 **HOLD**. WebView 캐시·사용자 jobs before-after **HOLD**.
- 검증용 release `vod-scout.exe`(`4597010…` 빌드) 크기 **15,270,400** · SHA-256 **`2914E618F4F99A075C91E8CC888BD2E277ACFF57CEB34ED2F324519AE49B7D98`** — **설치본·공개 릴리스 자산이 아님**.
- Oracle 최종 리뷰 **exactly once**: 판정 **BLOCKED** · Run `689ab803-0a7b-4695-ad8e-5a495c3b4991` · 보고 SHA-256 `DB59D334824C07A4F0922C454C476D93CDEADED10980C0910839AFFD00A19815` · 요청 모델 `GPT-5.6 Sol` · picker `unavailable`/선택 **검증되지 않음**. 재호출 없음.
- 종합 판정: **BLOCKED**. 코드 PR #13·문서 PR 모두 **merge 금지**. v0.4.0 버전·설치·태그·배포 계획 **미포함**.

#### Historical note (H8–H11 · 원샷 PASS 근거 아님)

- 2026-08-06 main `cca7a9e` 시점 H8 범위 overall PASS, H10 취소·재개 PASS, H11 full 재개 overall PASS 등은 **과거 기록**이다. 이번 PR #13 원샷 실제 미디어 PASS나 P0 마감 근거로 승격하지 않는다.

#### Known issues (P0 잔여)

- full download·무중단 장시간 분석 **BLOCKED**, live low-space·자원 요약·설치 폴더 내용 불변·사용자 데이터 before-after·403 귀속 **HOLD**가 해소될 때까지 **PR 병합·v0.4.0 버전·설치 배포 계획을 진행하지 않는다**.
- probe 취소·hang 실제 E2E, 실행 시 yt-dlp/Deno/FFmpeg/Whisper·DLL·model 매니페스트 결박, 실제 체크포인트 마이그레이션은 단위·정적과 분리해 **미실행 HOLD**.
- Authenticode/SmartScreen 및 P1–P7은 기존과 같이 HOLD.

### v0.4.0 이후 계획 (P1~ · P0 완료 후)

- YouTube 원문 자막 우선 혼합 분석, 검색과 원본 시각 이동, 이야기 후보를 구현한다.
- 필요한 Whisper 음성 인식은 실제 시험을 통과한 GPU를 우선 사용하고 실패한 청크만 CPU로 처리한다.
- 상세 범위와 완료 조건은 `docs/V0.4.0-PLAN.md`를 따른다.

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
