# VOD Scout 기여 가이드

VOD Scout는 Windows에서 긴 VOD의 쇼츠 후보를 좁히는 로컬 우선 앱이다. 첫 공개 전에는 이 문서가 준비 기준이며, 실제 공개 저장소 정책은 v0.3.2 릴리스 때 다시 검증한다.

## 개발 환경

- Windows 10/11 x64
- Node.js와 npm
- Rust toolchain
- 프로젝트가 고정한 FFmpeg·Whisper·yt-dlp·Deno·모델 자산

```powershell
npm.cmd install
npm.cmd run media-tools
npm.cmd run tauri:dev
```

## 변경 원칙

- 한 PR에는 한 기능 또는 한 문제 해결만 담는다.
- 실제 영상·전사·로그·쿠키·토큰·인증서·설치 EXE를 커밋하지 않는다.
- 테스트에는 합성 fixture 또는 배포 허가가 명확한 최소 자료만 사용한다.
- API·클라우드 업로드·새 네트워크 전송을 추가하려면 비용·개인정보·오프라인 동작의 영향을 먼저 문서화한다.
- 체크포인트 schema나 분석 결과를 바꾸면 이전 작업 호환성과 무효화 규칙을 함께 검증한다.
- 보안 문제는 공개 Issue가 아니라 `SECURITY.md`의 비공개 신고 경로를 사용한다.

## 테스트

```powershell
npm.cmd test
npm.cmd run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/fixture-worker/Cargo.toml
```

미디어·YouTube·장시간 경로를 변경했다면 관련 실제 진입점 E2E와 자식 프로세스 종료·재개·메모리·디스크 결과도 기록한다. 실행하지 않은 검증은 PR에서 `HOLD`로 표시한다.

## PR 기록

PR 설명에는 다음을 포함한다.

- 사용자에게 보이는 변화
- 변경 파일과 의도
- 버그 재현 또는 완료 조건
- 실행한 테스트와 정확한 결과
- 보안·개인정보·비용·체크포인트 영향
- 알려진 한계와 롤백 방법

버전업 PR은 `AGENTS.md`와 `docs/RELEASE-PROCESS.md`의 문서·패키지 게이트를 모두 따른다.

## 라이선스

공개 전 라이선스는 아직 결정되지 않았다. `docs/LICENSE-DECISION.md`의 결정을 완료하고 루트 `LICENSE`가 추가되기 전에는 외부 기여를 병합하지 않는다. 공개 후 제출한 기여는 최종 선택된 프로젝트 라이선스로 배포할 수 있어야 한다.
