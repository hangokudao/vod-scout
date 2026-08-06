# VOD Scout

긴 유튜버 VOD에서 쇼츠 후보를 먼저 좁혀 주는 Windows용 오프라인 편집 보조 앱입니다.

## Windows 설치 파일

[**VOD Scout v0.3.4 다운로드 (.exe)**](https://github.com/hangokudao/vod-scout/releases/download/v0.3.4/VOD.Scout_0.3.4_x64-setup.exe)

[변경 내용·SHA-256·서명 파일 보기](https://github.com/hangokudao/vod-scout/releases/tag/v0.3.4)

Windows 코드 서명 인증서가 없어 처음 설치할 때 SmartScreen 경고가 표시될 수 있습니다. 설치 파일 크기·SHA-256·updater 서명은 [v0.3.4 릴리스 기록](docs/V0.3.4-RELEASE.md)과 [빌드 명세](BUILD-MANIFEST.md)를 따릅니다.

## 0.3.4에서 되는 것

`로컬 영상 또는 YouTube URL → 로컬 영상 확보 → ffprobe → 10분 오디오 청크 → 한국어 Whisper 음성 인식 → 오디오·발화·채팅 움직임 점수 → 겹치지 않는 후보 → 앱 내 영상 검토`

v0.3.3 기능에 더해 설정 진입점 가시성, 어두운 화면 입력 카드 대비, 취소 완료 감독, 내려받기 임시 용량 측정 도구를 보완합니다.

- MP4, MKV, WebM, MOV, AVI, FLV 로컬 파일 선택
- 공개 YouTube 단일 영상을 `yt-dlp`로 최대 720p 다운로드
- Deno 런타임까지 내장해 별도 프로그램 설치 없이 YouTube 추출
- 다운로드 진행률, 취소, `.part` 이어받기와 완료 파일 체크포인트
- FFmpeg 8.1과 다국어 Whisper `base` 모델 내장
- API 키·구독료·클라우드 업로드 없이 PC 안에서 분석
- `빠른 분석`(최소 30분·최대 120분 분산 음성 인식), `구간 지정`, `전체 정밀 분석`
- 청크마다 음성 인식 결과·오디오 신호·작업 상태 저장
- 실행 중 FFmpeg/Whisper 종료, 취소 후 완료 청크 다음부터 재개
- 한국어 고정 음성 인식과 무음·영어 반복 환각 억제
- 우측 화면을 5초 간격 키프레임으로 비교한 채팅 움직임 신호
- 시간·음성 인식 문장 유사도를 함께 사용한 겹치는 후보 제거
- 후보 클릭 시 FFmpeg가 만든 H.264/AAC 검토 프록시를 앱 안에서 해당 시점부터 재생
- 경과 시간과 최근 청크 처리 속도 기반 예상 남은 시간
- 후보별 타임코드, 음성 인식 문장, 오디오 반응·발화 밀도·채팅 움직임 근거
- 후보 채택·제외·보류 및 앱 재실행 후 복원
- 작업별 저장 용량 표시와 해당 작업 폴더만 삭제
- 저장된 전체 작업 목록과 선택·전체 삭제
- 선택 후보 타임코드 복사와 UTF-8 BOM CSV 내보내기
- GitHub Releases의 서명된 안정 버전 확인·설치와 다운그레이드 차단
- 후보를 종합 점수·원본 시간·오디오 반응·발화 밀도·채팅 움직임·판정 상태로 정렬하고 선택 후보 유지
- Windows 화면 설정을 따르는 밝은 화면·어두운 화면 전환과 설정 저장
- 후보 앞뒤 영상 구간과 음성 인식 문장을 원본 타임코드로 확인하고 바로 이동
- 실제 미디어와 별개인 결정론적 실패/충돌/무응답 데모

채팅 글자를 읽는 OCR, GPU 음성 인식, LLM 재순위, 완성 쇼츠 렌더링은 아직 구현하지 않았습니다. 채팅 움직임은 화면 오른쪽 영역의 변화량이지 채팅 내용 분석이 아닙니다. 비공개·멤버십·로그인 필요 영상과 진행 중인 라이브는 지원하지 않습니다.

## 설치 안내

[최신 버전과 변경 내용 확인](https://github.com/hangokudao/vod-scout/releases/latest)

- Windows 10/11 x64
- 설치 파일 코드 서명 없음: SmartScreen 경고가 표시될 수 있음
- 모델과 도구를 포함해 설치 파일이 큼
- CPU 음성 인식이므로 영상 길이와 PC 성능에 따라 상당한 시간이 걸릴 수 있음

정확한 파일 크기·SHA-256과 검증 결과는 [빌드 명세](BUILD-MANIFEST.md)에 기록합니다.

## 개발 실행

```powershell
npm.cmd install
npm.cmd run tauri:dev
```

첫 실행 전 `media-tools` 단계가 고정된 URL에서 바이너리와 모델을 내려받고 SHA-256을 검증합니다. 준비된 리소스가 있으면 다시 받지 않습니다.

```powershell
npm.cmd run media-tools
npm.cmd run check:yt-dlp
npm.cmd test
npm.cmd run test:security
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/fixture-worker/Cargo.toml
```

## 비용과 데이터

- LLM/API 호출: 없음, 실행당 0원
- 네트워크: 로컬 파일 분석에는 불필요; YouTube 입력은 다운로드 단계에서만 필요
- 영상 처리: 다운로드가 끝난 뒤 음성 인식·점수 계산은 PC 안에서 실행
- 작업 데이터: `%LOCALAPPDATA%/com.vodscout.app/jobs/{job-id}`
- 임시 WAV: 청크 처리 후 삭제
- 보존 파일: 작업 스냅샷, JSONL 활동 로그, 체크포인트, 음성 인식 JSON, 도구 진단 로그
- YouTube 입력: 작업 폴더 안에 다운로드 영상과 재개용 `.part` 파일 보존

## 문서

- [프로젝트 작업 규칙](AGENTS.md)
- [변경 이력](CHANGELOG.md)
- [공개 저장소 다음 작업](docs/PUBLIC-REPOSITORY-NEXT-STEPS.md)
- [버전업·릴리스 절차](docs/RELEASE-PROCESS.md)
- [디버깅·장애 기록](docs/DEBUGGING.md)
- [비공개 Git 저장소 전환 계획](docs/GIT-PRIVATE-REPOSITORY-PLAN.md)
- [오픈소스·Windows 설치 EXE 배포 계획](docs/OPEN-SOURCE-RELEASE-PLAN.md)
- [2026-08-02 보안 점검 인수 기록](docs/SECURITY-AUDIT-2026-08-02.md)
- [v0.3.2 구현·검증 계획](docs/V0.3.2-PLAN.md)
- [v0.3.2 릴리스 작업 정본](docs/V0.3.2-RELEASE.md)
- [v0.3.3 릴리스 작업 정본](docs/V0.3.3-RELEASE.md)
- [v0.3.3 UI·검토 개선 계획](docs/V0.3.3-PLAN.md)
- [v0.3.4 안정성·접근성 후속 패치 계획](docs/V0.3.4-PLAN.md)
- [v0.3.4 릴리스 기록](docs/V0.3.4-RELEASE.md)
- [v0.4.0 장시간 최적화·이야기 구간 계획](docs/V0.4.0-PLAN.md)
- [v0.4.0 Oracle 설계 검토 기록](docs/V0.4.0-ORACLE-REVIEW.md)
- [v0.4.0 릴리스 기록 초안](docs/V0.4.0-RELEASE.md)
- [오픈소스 라이선스 결정 기록](docs/LICENSE-DECISION.md)
- [제품·개발 계획](docs/PLAN.md)
- [아키텍처와 상태 모델](docs/ARCHITECTURE.md)
- [테스트와 완료 기준](docs/TEST-PLAN.md)
- [개발·실행·패키징](docs/DEVELOPMENT.md)
- [v0.3.1 구현·검증 기록](docs/V0.3.1-IMPLEMENTATION.md)
- [구현 인계서](HANDOFF.md)
- [제3자 라이선스 고지](THIRD-PARTY-NOTICES.md)
- [기여 가이드](CONTRIBUTING.md)
- [보안 정책과 취약점 신고](SECURITY.md)

## 다음 순서

1. v0.3.4 공개·인앱 업데이트·DisplayVersion·데이터 보존 검증 완료 (Authenticode는 계속 `HOLD`)
2. v0.4.0 P0 체크포인트·범위·취소·저장 공간 정확성
3. YouTube 자막 확보·품질 검사·검색과 전체 저비용 시간축
4. 이야기 구간 탐색과 필요한 Whisper의 GPU 우선·CPU 자동 전환
5. 1~2시간 품질 비교 뒤 8시간 처리 시간·메모리·저장 공간 검증
6. 후순위 채팅 영역 지정·선별 글자 인식·자동 영역 탐색
