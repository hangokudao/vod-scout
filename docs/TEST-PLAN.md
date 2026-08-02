# 테스트와 완료 기준

## 자동 테스트

### 프런트엔드

- 시간 표시와 소스 축약
- 상태 라벨
- 경과 시간·ETA 계산과 표시
- TypeScript 프로덕션 빌드

### Rust Core

- 허용·거부 상태 전이
- 진행 단위 단조 증가
- 손상된 최신 스냅샷에서 이전 사본 복구
- SRT 시간 파싱
- 오디오·발화·채팅 후보 순위
- 후보 시간 겹침과 유사 전사 중복 제거
- 영어 반복·무음 환각 필터
- 잘못된 UTF-8 바이트가 포함된 SRT 손실 허용 파싱
- 작업 용량과 요청한 UUID 작업만 삭제
- YouTube URL 허용 목록과 위장 호스트 거부
- yt-dlp 진행률 파싱과 영상·오디오 진행률 결합
- `.part` 파일을 완료 영상으로 오인하지 않음

### fixture-worker

- 정상 완료
- 제어된 실패
- 중간 충돌과 재개
- heartbeat 정지 감지
- 잘못된 JSON 이벤트 거부

### 실제 미디어 무창 통합

1. 11초 MP4 → ffprobe JSON
2. FFmpeg → 16 kHz mono WAV
3. whisper.cpp base → 실제 SRT 문장
4. Rust → RMS·후보 생성
5. 체크포인트 저장

### 제품 경로 무창 E2E

- Tauri `create_job/start_job` 실제 호출
- React DOM이 검토 화면으로 전환
- 실제 Whisper 문장과 채팅 움직임 점수가 후보에 포함
- 후보 겹침과 알려진 영어 반복 환각 0개
- 후보 영상 프록시 생성과 `<video>.readyState >= 1`
- 작업 용량, UTF-8 BOM CSV, 작업 삭제
- 영상 처리 중 `cancel_job`
- 10초 안에 `CANCELLED`
- 같은 작업 재개 후 `REVIEW_READY`
- 취소 활동, 전사 5개 세그먼트, 체크포인트 보존
- 부모 앱 강제 종료 시 관찰 중인 ffprobe/FFmpeg/Whisper 자식 PID 소멸

### 실제 한국어 장시간 E2E

- 1시간 5분 29초 원본 → 10분 청크 7개
- 중간 체크포인트 3/7에서 재개 → `REVIEW_READY`, 13/13
- 전사 702세그먼트, 채팅 움직임 785포인트, 후보 8개
- 후보 시간 겹침과 알려진 영어 반복 환각 0개
- ETA 표시, 플레이어 준비, CSV·용량·삭제 확인

### 실제 YouTube 무창 E2E

- 공개 단일 영상 URL → yt-dlp + Deno → 최대 720p 로컬 파일
- 다운로드 진행률 → `acquisition.json`과 완료 영상 저장
- 다운로드 직후 취소 → `CANCELLED` → 같은 작업 재개
- ffprobe·FFmpeg·Whisper → `REVIEW_READY`, 실제 전사와 후보 생성
- 삭제·사용 불가 영상 → 사용자 오류와 yt-dlp 진단 분리

무창 E2E에서만 `VOD_SCOUT_HEADLESS_E2E`와 로컬 CDP 포트를 사용한다. 배포 앱은 디버그 포트를 열지 않는다.

## 패키지 게이트

- NSIS 설치 파일 생성
- 설치 파일 SHA-256 기록
- 모델·FFmpeg DLL·whisper.cpp·yt-dlp·Deno·라이선스 포함
- 같은 release 빌드 실행 파일을 무창으로 실행해 실제 미디어 E2E 재검증
- 설치 후 시스템 PATH의 FFmpeg/Python/Node에 의존하지 않음
- 코드 서명 없음 표시

## 아직 HOLD인 검증

- 사람 기준 한국어 하이라이트 정확도
- 2시간·8시간 처리 시간·피크 메모리
- Windows 배율별 0.3.1 UI 수동 회귀
- SmartScreen 신뢰도와 코드 서명
- 30분 이상 YouTube 다운로드 취소·재개와 봇 확인 발생률
- v0.3.1 실제 YouTube URL 재회귀
- 채팅 OCR·GPU 전사
- Oracle Ubuntu 최종 이미지 열기 단계
