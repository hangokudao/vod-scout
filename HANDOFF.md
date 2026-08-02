# VOD Scout 0.3.1 구현·v0.3.2 준비 인계서

현재 게이트: **기능 기준선 PASS · 보안/설치본 HOLD**

2026-08-02 전달받은 보안 점검에서 실제 API 키·개인키 노출 증거는 발견되지 않았다. 그러나 기존 비표준 설치 경로 ACL, 설치된 메인 EXE와 현재 release EXE 해시 차이, 삭제할 수 없는 과거 작업 개인정보 때문에 기존 설치본은 안전 PASS가 아니다. 설치 폴더와 과거 작업은 아직 변경·삭제하지 않았다.

다음 정본 문서를 먼저 따른다.

- `docs/SECURITY-AUDIT-2026-08-02.md`
- `docs/GIT-PRIVATE-REPOSITORY-PLAN.md`
- `docs/V0.3.2-PLAN.md`
- `docs/V0.3.2-RELEASE.md`
- `docs/OPEN-SOURCE-RELEASE-PLAN.md`

## 결과

로컬 파일과 공개 YouTube 단일 영상이 같은 로컬 분석 경로로 연결되고, 후보를 앱 안의 영상 플레이어로 검토할 수 있다.

`로컬 파일 또는 YouTube URL → 로컬 영상 확보 → ffprobe → 10분 오디오 청크 → 한국어 whisper.cpp base → 오디오·발화·채팅 움직임 → 중복 제거 → 영상 검토 UI`

LLM/API 비용은 없으며, YouTube 입력은 다운로드에만 네트워크를 사용한다.

## v0.3.1 주요 구현

- `src-tauri/src/media.rs`: 한국어 고정 전사, 무음·반복 환각 필터, 채팅 움직임 샘플링, 겹치지 않는 후보 순위, H.264/AAC 검토 프록시
- `src-tauri/src/storage.rs`: 재귀 용량 계산, UUID 경로 검증, 실행 중 삭제 차단, 현재 작업만 삭제
- `src-tauri/src/lib.rs`: 플레이어·용량·삭제·CSV용 좁은 Tauri 명령과 상태 저장
- `src/App.tsx`: 후보 클릭 자동 재생, 경과·ETA, 용량·삭제, 타임코드 복사, CSV, 채팅 Signal Rail
- `scripts/e2e-local-cdp.mjs`: 플레이어 ready, 환각·중복, 채팅 신호, 용량·CSV·삭제와 선택적 최종 캡처 검증
- `src-tauri/tauri.conf.json`: 앱 작업 폴더만 허용하는 asset protocol과 미디어 CSP

## 실제 동작 경계

- 지원: 로컬 MP4/MKV/WebM/MOV/AVI/FLV, 공개 YouTube 단일 영상과 종료된 공개 라이브
- 미지원: 비공개·멤버십·로그인 필요 영상, 진행 중인 라이브·예약 영상, 채널·재생목록
- 플레이어: 원본 후보 주변을 최대 720p H.264/AAC 프록시로 한 번 만들고 재사용
- 채팅 신호: 화면 오른쪽 38%를 5초 간격 키프레임으로 비교한 변화량이며 채팅 글자를 읽지 않음
- 후보 점수: 채팅 신호가 있으면 오디오 45% + 발화 35% + 채팅 20%, 없으면 오디오 55% + 발화 45%
- 삭제: 현재 작업이 멈춘 상태에서 해당 UUID 작업 폴더만 삭제

## 자동 검증 결과

| 검증 | 결과 |
|---|---|
| TypeScript + Vite build | PASS |
| 프런트 테스트 | 5/5 PASS |
| Rust Core | 14 PASS, 1 ignored |
| fixture-worker | 5/5 PASS |
| 2분 한국어 로컬 E2E | PASS, 7/7, 전사 33, 채팅 23, 후보 3 |
| 취소 후 같은 작업 재개 | PASS |
| 1시간 5분 29초 한국어 E2E | PASS, 13/13, 전사 702, 채팅 785, 후보 8 |
| 후보 시간 겹침·알려진 영어 반복 환각 | PASS, 각각 0개 |
| release 실행 파일 플레이어·CSV·용량·삭제 | PASS |
| v0.3.1 실제 YouTube 재회귀 | HOLD, 로컬 명령 정책이 실행 전 차단 |
| Oracle Ubuntu 시각 검수 | HOLD, 연결·파일 업로드 뒤 원격 입력/창 전환이 불안정 |

## 배포물

- `VOD-Scout-0.3.1-windows-x64-setup.exe`
- 크기: `232,809,048 bytes`
- SHA-256: `7F1FE923757032E0A08E8A93D8D2DF33025F0F01A39739372D7AA91BCAC0D230`
- 코드 서명: 없음

설치 파일에는 앱·fixture worker·yt-dlp·Deno·FFmpeg/ffprobe·whisper.cpp·다국어 `ggml-base.bin`·라이선스가 들어 있다.

## 남은 한계

- 채팅 OCR과 사용자가 지정하는 채팅 ROI는 아직 없다.
- GPU 전사 가속, LLM 재순위, 개인화 학습은 없다.
- 검토 프록시는 작업 용량을 추가로 사용하지만 작업 삭제로 함께 제거된다.
- 1시간 5분 영상은 통과했지만 2시간·8시간과 여러 방송 레이아웃의 품질·피크 메모리는 HOLD다.
- yt-dlp 자동 업데이트가 아니라 고정 버전이므로 YouTube 변경 시 새 빌드가 필요하다.
- 코드 서명이 없어 SmartScreen 경고가 표시될 수 있다.

## 다음 작업

1. 별도 승인 후 `hangokudao/vod-scout-dev` private Git 기준선·secret scan·최초 push
2. v0.3.2 RC1 보안 수정과 실제 설치 ACL·무결성 검증
3. provenance, `빠른 분석`·`구간 지정`·`전체 정밀 분석`
4. 실제 1~2시간 YouTube와 8시간 빠른 분석 회귀
5. 라이선스 결정, 공개용 깨끗한 이력 검사, `hangokudao/vod-scout` 공개 소스와 GitHub Release 설치 EXE 배포
6. 이후 GPU, 채팅 자동 ROI·선택 OCR, 개인화·LLM
