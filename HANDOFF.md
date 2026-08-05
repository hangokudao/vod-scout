# VOD Scout v0.3.3 공개 배포·v0.3.4/v0.4.0 계획 인계서

현재 게이트: **v0.3.3 공개 배포·실제 업데이트·기존 작업 보존 PASS · v0.3.4 후속 패치 문서 준비·구현 전 HOLD · v0.4.0 설계·릴리스 문서 준비·구현 전 HOLD · Authenticode HOLD**

## v0.3.3 최종 상태

- 공개 저장소: https://github.com/hangokudao/vod-scout
- 구현 PR #5와 최종 릴리스 workflow PR #6을 squash merge했다. v0.3.3 태그와 설치본의 exact commit은 `5f756af7390325a99f2820a424f7d4ef05334d14`다.
- 공개 Release: https://github.com/hangokudao/vod-scout/releases/tag/v0.3.3
- 공개 Actions run `30963107742` PASS. 설치 EXE·updater 서명·`latest.json`·체크섬·SPDX 2.3 SBOM 5개 자산의 GitHub digest와 재다운로드 SHA-256이 일치했다.
- 최종 기본 검증: 프런트 2개 파일·32개, Vite 1,793 modules, Rust 27개 통과·1개 무시, fixture-worker 5개, archive 안전성 6개 PASS. npm 취약점 0개.
- 승인된 실제 입력: YouTube `JN3BO9GLuFU`, 실제 미디어 `08:53:19.981`, 720p. 빠른 분석 `11/11`, `17/17`, 후보 8개, `REVIEW_READY` PASS.
- 처리 시간·자원: 첫 시작부터 검토 준비 `36분 01.231초`, 누적 활성 `34분 28.605초`; 최대 working set `934,158,336 bytes`, private bytes `1,123,246,080 bytes`, GPU 보드 메모리 기준선/최대 `1,682/2,012 MiB`, 분석 WAV 최대 `19,200,078 bytes`, 최종 작업 데이터 `7,097,358,568 bytes`.
- 실제 v0.3.2 설치본에서 공개 v0.3.3을 찾아 설치하고 재실행했다. 앱·실행 파일은 `v0.3.3`, 설정 화면은 `최신 상태`였다.
- 기존 작업 14개와 현재 작업 `#92bbf85a`, 후보 8개를 다시 열었다. 현재 작업의 핵심 상태·체크포인트 5개 SHA-256은 업데이트 전과 일치했다.
- 설치된 runtime manifest 28개 파일의 SHA-256이 전부 일치했다.

## 남은 HOLD

- Windows Authenticode 인증서가 없다. 구매하거나 대체 인증서를 만들지 않았으며 설치 EXE와 앱 실행 파일은 `NotSigned`다.
- 업데이트 뒤 실행 파일과 `uninstall.exe`는 `0.3.3`이지만 HKCU 제거 프로그램 레지스트리 `DisplayVersion`은 `0.3.2`로 남았다. 원인은 확정하지 않았다.
- YouTube 영상·음성 병합 중 열린 출력 파일을 측정 도구가 읽지 못해 내려받기 단계의 정확한 순간 최대 임시 용량은 `HOLD`다. 닫힌 입력 파일 최소 확인값은 `7,070,731,050 bytes`다.

상세 결과는 `docs/V0.3.3-RELEASE.md`, `BUILD-MANIFEST.md`, `docs/DEBUGGING.md`를 따른다. v0.3.3 태그와 공개 자산은 바꾸지 않으며 후속 문제는 v0.3.4로 분리한다.

## v0.3.4 후속 패치 계획

- 설정 버튼 가시성, 다크 모드 입력 카드 대비, 취소 완료 지연, updater 뒤 HKCU `DisplayVersion`, 내려받기 병합 순간 최대 임시 용량 측정을 v0.3.4 범위로 고정했다.
- 기능 범위와 완료 조건은 `docs/V0.3.4-PLAN.md`, 실제 결과 기록 형식은 `docs/V0.3.4-RELEASE.md`를 따른다.
- v0.4.0의 자막·검색·이야기 후보·GPU 음성 인식·API·OCR은 v0.3.4에 넣지 않는다.
- 구현·테스트·설치본·태그·Release는 아직 시작하지 않았으므로 모두 `HOLD`다.

## 이전 v0.3.2 공개 배포 상태

현재 게이트: **공개 배포·보안·설치 PASS · 8시간 실제 영상 검사는 HOLD**

## 공개 상태

- 공개 저장소: https://github.com/hangokudao/vod-scout
- v0.3.2 배포: https://github.com/hangokudao/vod-scout/releases/tag/v0.3.2
- 설치 파일: `VOD.Scout_0.3.2_x64-setup.exe`
- 크기: `233,848,505 bytes`
- SHA-256: `FF9C6F7421793618D8053D6790AF8964326E4B8F6B7C99875616C4501C8A5D01`
- 라이선스: Apache-2.0

설치 파일, 업데이트 서명, `latest.json`, 체크섬, SBOM을 공개 배포 자산으로 제공한다. 설치 파일과 내장 실행 파일은 일반 Git 커밋에 포함하지 않는다.

## 완료된 검증

- 프런트 테스트 6개 PASS
- Rust 핵심 테스트 22개 PASS, 실제 미디어 테스트 1개는 별도 실행 항목
- 보조 작업 프로그램 테스트 5개 PASS
- 압축 파일 안전성 테스트 6개 PASS
- 1시간 5분 실제 한국어 영상 빠른 분석 PASS
- 공개 YouTube 영상 정보 확인과 확보한 원본 분석 PASS
- 공개 설치 파일·서명·업데이트 정보·SBOM 재다운로드 및 SHA-256 확인 PASS
- 새 Windows 사용자 경로 설치, 공유 사용자 쓰기 권한 차단, 내장 파일 28개 무결성 확인 PASS
- 설치 후 실행·종료·재실행 PASS
- 알려진 HIGH·MEDIUM 보안 항목과 Git 이력 비밀값 검사 PASS

상세 근거는 `docs/V0.3.2-RELEASE.md`, `docs/SECURITY-AUDIT-2026-08-02.md`, `BUILD-MANIFEST.md`, `validation/v0.3.2.json`을 따른다.

## 알려진 한계

- Windows 코드 서명 인증서가 없어 SmartScreen 경고가 표시될 수 있다.
- Authenticode와 Tauri updater 서명은 별개다. 기존 GitHub Actions signing secret으로 생성한 updater 서명은 검증 PASS지만 Authenticode는 `HOLD`다.
- 빠른 분석은 전체 32,000초 중 6,400초를 시간대별로 골라 처리한다. 전체 정밀 분석 성능과 누락 없는 탐지를 주장하지 않는다.
- GPU 음성 인식, 채팅 글자 인식·자동 영역 탐색, LLM 후보 재정렬·개인화는 지원하지 않는다.

## v0.4.0 Oracle 설계 검토

- 2026-08-05에 저장소·성능 감사, YouTube 자막 우선 설계 반대 검토, 이야기 후보 품질 설계의 독립 Oracle 자문 3건을 완료했다.
- 종합 판정은 v0.3.3 반응 후보 구조 `PASS`, 일반적인 2~8시간 최적화 `HOLD`, 이야기 구간 탐지와 자막 우선 혼합 구현 `BLOCKED`다.
- 최종 방향은 `자막 탐색 → 품질 검사 → 전체 시간대 저비용 음성·장면·채팅 움직임 색인 → 이야기 후보 → 필요한 구간만 Whisper 확인`이다.
- Ollama와 대형 로컬 언어 모델은 포함하지 않는다. DeepSeek·OpenAI 호환 API는 오프라인 규칙 기반 결과가 완성된 뒤 사용자가 선택하는 보조 기능으로만 검토한다.
- 구현 전 P0는 체크포인트 출처 검증, 범위 밖 후보 차단, 취소 감독, 저장 공간 사전 확인, 마지막 정상 파일 세대 보존이다.
- 후속 사용자 결정으로 GPU 음성 인식은 P7이 아니라 선택형 Whisper를 구현하는 P4에 포함한다. P0~P3에서 불필요한 음성 인식을 먼저 줄이고, P4에서는 실제 시험을 통과한 GPU를 우선 사용하되 실패한 청크는 CPU로 자동 처리한다.
- 검수 증거와 결정은 `docs/V0.4.0-ORACLE-REVIEW.md`, 작업 순서와 완료 게이트는 `docs/V0.4.0-PLAN.md`를 따른다.
- 이번 문서 작업에서는 v0.4.0 코드·의존성·설치본·사용자 작업 데이터를 변경하지 않는다.

## 다음 작업

1. v0.3.4 시작 승인이 있으면 `docs/V0.3.4-PLAN.md`의 취소 지연 재현부터 시작하고 UI·updater·측정 보완을 정해진 순서로 진행한다.
2. v0.3.4 PR 병합과 릴리스 결과를 확인한 뒤 v0.4.0의 별도 시작 승인을 받는다.
3. v0.4.0은 `docs/V0.4.0-PLAN.md`의 P0 정확성·복구부터 시작한다. 자막·검색과 이야기 후보를 건너뛰고 GPU부터 붙이지 않으며 GPU 우선 실행과 CPU 자동 전환은 P4에서 함께 구현한다.
4. 구현 결과에 맞춰 각 버전 릴리스 기록, `CHANGELOG.md`, `docs/DEBUGGING.md`, `HANDOFF.md`, `BUILD-MANIFEST.md`를 갱신한다.
5. 그 외 새 기능, 요청하지 않은 리팩터링·의존성 추가·폴더 개편은 별도 승인과 계획 없이 시작하지 않는다.

기존 사용자 데이터와 설치본을 삭제하지 않는다.
