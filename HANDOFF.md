# VOD Scout v0.3.2 공개 배포 인계서

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

우선순위와 이번 공개 반영의 제외 범위는 `docs/PUBLIC-REPOSITORY-NEXT-STEPS.md`를 정본으로 사용한다.

1. 다음 버전에서 실제 자동 업데이트와 재실행을 검증한다.
2. 1~2시간 영상 재확인 뒤 8시간 빠른 분석을 측정한다.
3. 채팅 영역 직접 지정, 선별 글자 인식, GPU 전사를 순서대로 검토한다.

유료 API는 사용자가 별도로 승인하기 전까지 제품 실행 경로에 추가하지 않는다. 기존 사용자 데이터와 설치본은 명시적인 승인 없이 변경하거나 삭제하지 않는다.
