# VOD Scout v0.3.3 PR 준비 인계서

현재 게이트: **v0.3.3 구현·단위 검증 PASS · 설치본·실제 미디어·업데이트 전환 HOLD · PR 병합 전**

## v0.3.3 작업 상태

- 통합 브랜치: `hangokudao/codex-v0.3.3-sol-r2`
- 기준: `origin/main` `833054017b9977958b923b413a9640d6241b76d6`
- 프런트 커밋: `d65b74c0532e71003f64afa6107150edee76276a`
- 백엔드 커밋: `d5cd518af097d3e2c72f65f5ede9e6c0152b1e0f` 및 맥락 기반 커밋 `e9372144fbcd168be124ea745462d7cb40b9f75e`
- 통합 결과: 충돌 없이 통합 후 화면·Rust 단위 검증을 실행했다.
- PR: https://github.com/hangokudao/vod-scout/pull/5 (OPEN, 병합 전)
- Sol 통합 검증: Windows `cmd.exe` 경유 `npm.cmd test` 2개 파일·32개 테스트 PASS, `npm.cmd run build` TypeScript·Vite PASS, cargo 핵심 26개 통과·1개 무시 및 fixture-worker 5개 통과.
- 남은 HOLD: 실제 미디어·설치본·서명·업데이트 전환·8시간 자원 측정.

상세 결과는 `docs/V0.3.3-RELEASE.md`, `BUILD-MANIFEST.md`, `docs/DEBUGGING.md`를 따른다.

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
- 8시간 실제 영상의 처리 시간·메모리·저장 공간은 아직 측정하지 않았다.
- v0.3.2가 첫 자동 업데이트 지원 버전이므로 이전 버전에서 v0.3.2로 교체되는 실제 흐름은 검증할 수 없다. 다음 배포에서 확인한다.
- GPU 전사, 채팅 글자 인식·자동 영역 탐색, LLM 후보 재정렬·개인화는 지원하지 않는다.

## 다음 작업

전체 우선순위는 `docs/PLAN.md`, 다음 버전의 상세 완료 조건은 아래 문서를 정본으로 사용한다.

v0.3.3의 프론트엔드 디자인과 전반적인 UI/UX 설계·구현은 Orca-Claude에 맡긴다.

1. `docs/V0.3.3-PLAN.md`: 정렬·시스템 화면 설정·전후 맥락·UI 정리
2. `docs/V0.4.0-PLAN.md`: 기준선·GPU 전사·Whisper 성능 단계·이야기 구간·8시간 최적화
3. `docs/TEST-PLAN.md`: 1~2시간 회귀 뒤 8시간 빠른 분석, 취소·재개·자원 측정

구현은 v0.3.3 UI 개선과 v0.4.0 분석 엔진을 한 PR에 섞지 않는다. 각 버전은 작업 브랜치, 관련 테스트, PR 검토를 거쳐 `main`에 병합한다. 새 설치 파일은 버전 태그를 만든 경우에만 배포한다.

로컬 AI 실행기와 대형 모델은 포함하지 않고, 향후 사용자가 직접 등록한 API로 상위 후보만 선택 재정렬하는 방향을 따른다. 현재 제품에는 아직 구현하지 않았으며 별도 버전 범위에서 보안 저장·전송 동의·비용 한도 검사를 통과해야 한다. 기존 사용자 데이터와 설치본은 명시적인 승인 없이 변경하거나 삭제하지 않는다.
