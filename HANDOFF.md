# VOD Scout v0.3.3 실제 검증·PR 병합 전 인계서

현재 게이트: **v0.3.3 구현·기본 테스트·승인된 실제 미디어·8시간대 빠른 분석 PASS · 설치본·서명·업데이트 전환 HOLD · PR #5 병합 승인 전**

## v0.3.3 작업 상태

- 통합 브랜치: `hangokudao/codex-v0.3.3-sol-r2`
- 후속 검증 worktree: Orca가 만든 PR #5 기준 clean worktree, 로컬 브랜치 `hangokudao/v033-followup-pr5`
- 기준: `origin/main` `833054017b9977958b923b413a9640d6241b76d6`
- 검증을 시작한 PR #5 head: `3af33b50bb2ddf7782fdd748ba54669af1e33f39`
- 프런트 커밋: `d65b74c0532e71003f64afa6107150edee76276a`
- 백엔드 커밋: `d5cd518af097d3e2c72f65f5ede9e6c0152b1e0f` 및 맥락 기반 커밋 `e9372144fbcd168be124ea745462d7cb40b9f75e`
- 통합 결과: 충돌 없이 통합 후 화면·Rust 단위 검증을 실행했다.
- PR: https://github.com/hangokudao/vod-scout/pull/5 (OPEN, 병합 전)
- Sol 후속 검증: Windows에서 `npm.cmd test` 2개 파일·32개 테스트 PASS, `npm.cmd run build` TypeScript·Vite PASS, cargo 핵심 27개 통과·1개 무시 및 fixture-worker 5개 통과.
- 승인된 실제 입력: YouTube `JN3BO9GLuFU`, 확인 길이 `08:53:20`, 실제 미디어 `08:53:19.981`, 720p. 빠른 분석 `11/11`, `17/17`, 음성 인식 구간 614개, 채팅 움직임 2,132개, 겹치지 않는 후보 8개, `REVIEW_READY` PASS.
- 처리 시간·자원: 첫 시작부터 검토 준비 `36분 01.231초`, 중단 사이 대기를 뺀 누적 활성 `34분 28.605초`; 최대 working set `934,158,336 bytes`, private bytes `1,123,246,080 bytes`, GPU 보드 메모리 기준선/최대 `1,682/2,012 MiB`, 분석 WAV 최대 `19,200,078 bytes`, 후보·맥락 영상 포함 최종 작업 데이터 `7,097,358,568 bytes`.
- 재개 경로: 측정 도구가 두 차례 중단됐지만 제품 체크포인트에서 재개해 완료했다. 중단 없는 단일 실행 성능으로 해석하지 않는다.
- 실제 입력에서 `.mp4.tmp` 임시 이름 때문에 FFmpeg가 MP4 형식을 고르지 못하는 버그를 발견했다. `.tmp.mp4`로 수정하고 단위 테스트, 맥락 75초·후보 49초 생성, 실제 플레이어 준비 상태를 확인했다.
- 남은 HOLD: PR 병합, exact merged commit 설치 EXE, 설치 파일·핵심 바이너리 해시, SBOM, updater 서명, v0.3.2 → v0.3.3 업데이트·재실행·기존 데이터 보존. 내려받기 병합 중 열린 출력 파일 때문에 정확한 순간 최대 임시 용량도 HOLD다.

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
- 현재 PC의 CurrentUser·LocalMachine 인증서 저장소에 Authenticode 코드 서명 인증서가 없고 `signtool.exe`도 확인되지 않았다. 구매하거나 대체 인증서를 만들지 않는다.
- GitHub 저장소에는 `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secret 이름이 존재하지만 현재 로컬 환경에는 키가 없다. 실제 updater 서명 생성·검증 전까지 PASS가 아니다.
- 빠른 분석은 전체 32,000초 중 6,400초를 시간대별로 골라 처리한다. 전체 정밀 분석 성능과 누락 없는 탐지를 주장하지 않는다.
- v0.3.2가 첫 자동 업데이트 지원 버전이므로 v0.3.3 공개 updater 자산이 준비된 뒤에만 실제 전환을 확인할 수 있다.
- GPU 음성 인식, 채팅 글자 인식·자동 영역 탐색, LLM 후보 재정렬·개인화는 지원하지 않는다.

## 다음 작업

1. 문서·릴리스 스크립트 자체검증을 마친 뒤 후속 변경을 로컬 커밋한다.
2. GitHub 계정 `hangokudao`, PR #5의 정확한 head branch `hangokudao/codex-v0.3.3-sol-r2`, 원격 head를 다시 확인한다.
3. 승인 범위 안에서 PR #5 branch에 push하고 PR 본문을 실제 결과·HOLD 항목으로 갱신한다.
4. PR #5를 병합하지 않고 사용자에게 PASS·HOLD와 diff를 보고해 병합 승인을 기다린다.
5. 병합 승인 뒤 merge commit SHA를 확인하고 exact merged commit의 새 clean worktree에서 기본 테스트 4개와 최종 NSIS EXE를 만든다.
6. 버전 정본, 설치·실행, EXE와 핵심 바이너리 크기·SHA-256, SBOM, Tauri updater 서명을 검증한다. Authenticode는 인증서 부재로 `HOLD`를 유지한다.
7. 태그·GitHub Release·자산 업로드는 다시 사용자 승인을 받은 뒤에만 수행한다. 공개 뒤 v0.3.2 → v0.3.3 업데이트·재시작·기존 작업 데이터와 체크포인트 보존을 확인하고 별도 문서 PR에 기록한다.

v0.4.0 기능, 검색 기능, 새 기능, 요청하지 않은 리팩터링은 이 후속 작업에 포함하지 않는다. 기존 사용자 데이터와 설치본은 명시적인 승인 없이 변경하거나 삭제하지 않는다.
