# 개발·실행·패키징

## 필수 도구

- Windows 10/11 x64 + WebView2
- Microsoft C++ Build Tools `Desktop development with C++`
- Rust stable MSVC
- Node.js와 npm
- 최초 리소스 준비 시 약 400 MB 이상의 여유 공간과 네트워크

설치 앱 사용자는 Python, Node, Rust, 시스템 FFmpeg가 필요 없습니다.

## 일반 명령

```powershell
npm.cmd install
npm.cmd run media-tools
npm.cmd test
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/fixture-worker/Cargo.toml
npm.cmd run tauri:dev
npm.cmd run tauri:build
```

PowerShell 정책에서 `npm.ps1`이 막힐 수 있어 `npm.cmd`를 사용합니다.

## 미디어 리소스 준비

`npm.cmd run media-tools`는 다음 작업을 합니다.

1. FFmpeg·whisper.cpp·모델·yt-dlp·Deno·라이선스를 임시 캐시에 다운로드
2. 모든 파일의 SHA-256 확인
3. Windows `tar.exe`로 ZIP 해제
4. `src-tauri/resources/media-tools`에 실행 파일, DLL, 모델, 라이선스 복사
5. `manifest.json` 저장

고정 해시와 원격 파일이 달라지면 조용히 새 버전을 쓰지 않고 실패합니다. 버전을 올릴 때 URL·해시·라이선스·스모크 테스트를 함께 갱신해야 합니다.

## 내려받기·병합 임시 용량 표본 수집

제품 경로 밖에서 YouTube 내려받기·병합 중 작업 폴더의 순간 용량을 읽기 전용으로 표본 수집할 때는 `scripts/sample-disk-usage.mjs`를 사용합니다. 파일 내용을 열지 않고 메타데이터만 합산하므로, 병합 중 열린 출력 파일의 길이 증가도 포함됩니다. 측정 대상 트리를 수정하지 않습니다.

Windows PowerShell 예 (작업 폴더와 출력 경로는 실제 값으로 바꿉니다. 출력 파일은 반드시 `--target` 밖이어야 합니다):

```powershell
node scripts\sample-disk-usage.mjs `
  --target "$env:LOCALAPPDATA\com.vodscout.app\jobs\<job-id>" `
  --output "$env:TEMP\vod-scout-disk-samples.ndjson" `
  --interval-ms 1000 `
  --duration-ms 600000
```

- `<output>`: 폴링마다 한 줄 NDJSON (`timestamp`, `totalBytes`, `fileCount`, `largestFiles` 등)
- `<output>.summary.json`: 종료 시 `peak` / `final` / `sampleCount` / `stopReason` 요약
- `--duration-ms` 또는 `--stop-file` 중 하나가 필요합니다. `Ctrl+C`로도 요약을 남기고 종료합니다.

제품 분석 결과·체크포인트·내려받기 파일은 이 스크립트가 바꾸지 않습니다. 장시간 측정 기록은 릴리스 문서와 `BUILD-MANIFEST.md`에 남깁니다.

## 무창 실제 미디어 테스트

일반 단위 테스트는 큰 모델을 실행하지 않습니다. 준비된 11초 MP4로 실제 도구 체인을 확인하려면:

```powershell
$env:VOD_SCOUT_SMOKE_VIDEO='C:\absolute\sample-local.mp4'
cargo test --manifest-path src-tauri/Cargo.toml bundled_pipeline_reaches_review_ready_without_a_window -- --ignored --nocapture
```

제품의 Tauri 상태·저장·React DOM까지 무창으로 확인하는 스크립트는 `scripts/e2e-local-cdp.mjs`입니다. `--youtube`를 붙이면 실제 YouTube 다운로드부터 검증하고, `--cancel-resume`은 취소 후 같은 작업 재개를 확인합니다. `--screenshot <절대 경로>`를 붙이면 검토 화면을 PNG로 남깁니다. 테스트 실행에서만 `VOD_SCOUT_HEADLESS_E2E=1`, 격리된 `VOD_SCOUT_E2E_DATA_DIR`, WebView2 디버그 포트를 사용하며 배포 앱은 포트를 열지 않습니다.

## 패키지

```powershell
npm.cmd run tauri:build
```

결과는 `src-tauri/target/release/bundle/nsis` 아래 생성됩니다. 배포 전 아래를 확인합니다.

- installer와 설치된 앱의 버전
- `yt-dlp.exe`, `deno.exe`, `ffmpeg.exe`, `ffprobe.exe`, DLL, `whisper-cli.exe`, 모델 포함
- 제3자 라이선스 포함
- 개발 서버 없이 무창 E2E 통과
- installer SHA-256
