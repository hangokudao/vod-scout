# VOD Scout 오픈소스·Windows 배포 계획

상태: **v0.3.2 구현·보안·설치 게이트 완료 · 공개 Release 진행 중**
첫 공개 목표: **v0.3.2**

## 공개 목표

VOD Scout의 최종 배포 형태는 다음 두 가지를 동시에 만족해야 한다.

1. 개발자와 기여자가 검토·빌드할 수 있는 공개 소스 저장소
2. 개발 도구가 없는 Windows 사용자가 바로 설치할 수 있는 설치 EXE

소스만 공개하고 설치 파일을 제공하지 않는 상태는 이 프로젝트의 공개 배포 완료로 보지 않는다.

## 저장소 전환

| 단계 | 저장소 | 공개 범위 | 역할 |
|---|---|---|---|
| 공개 전 보안 수정 | `hangokudao/vod-scout-dev` | private | v0.3.1 HOLD 기준선, v0.3.2 보안·기능 개발, PR 검토 |
| 첫 공개 이후 | `hangokudao/vod-scout` | public | 공식 소스, 이슈, 기여, 릴리스의 정본 |

- 외부 쓰기 전에는 GitHub 계정 `hangokudao`, 저장소 부재·가시성, 대상 브랜치를 다시 읽어 확인한다.
- v0.3.2 공개가 끝나면 public 저장소가 일반 개발의 정본이 된다.
- 공개 이후 비공개 보안 수정이 필요하면 상시 별도 정본을 운영하지 않고 GitHub Security Advisory의 비공개 수정 흐름을 우선한다.
- 저장소 생성, remote 추가, push, 공개 전환은 각각 외부 변경이므로 사용자의 실행 승인을 받은 뒤 수행한다.

## Git과 GitHub Release의 경계

### 공개 Git 저장소에 포함

- 앱·백엔드·fixture worker 소스
- 재현 가능한 빌드·도구 준비 스크립트와 lock 파일
- 테스트, 문서, 아이콘 등 소형 정적 자산
- `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, 승인된 `LICENSE`
- 제3자 고지, 릴리스 기록, 빌드 명세

### 일반 Git 커밋에서 제외

- `VOD-Scout-*-setup.exe`
- FFmpeg·Whisper·yt-dlp·Deno 실행 파일과 모델
- 영상·오디오·전사·로그·체크포인트·화면 캡처
- 인증서·개인키·토큰·쿠키·브라우저 프로필
- `node_modules`, `dist`, Cargo `target` 등 생성물

설치 EXE를 커밋하지 않는 것은 설치 파일을 제공하지 않는다는 뜻이 아니다. 설치 파일은 GitHub Release 자산으로 제공한다. v0.3.2 공개 릴리스의 필수 자산은 다음과 같다.

- `VOD-Scout-0.3.2-windows-x64-setup.exe`
- `SHA256SUMS.txt`
- `SBOM.spdx.json`
- 사용자 변경 사항·버그 수정·보안 수정·알려진 문제를 포함한 릴리스 노트

README의 다운로드 버튼은 공개 전에는 준비 상태를 설명하고, 공개 후에는 `releases/latest`의 설치 자산으로 연결한다. 태그·릴리스·설치 파일의 버전은 모두 같아야 한다.

## 첫 공개 게이트

다음 조건이 모두 확인돼야 `hangokudao/vod-scout`와 v0.3.2 설치 파일을 공개한다.

1. SEC-001~010의 HIGH·MEDIUM 항목이 실제 설치본 기준으로 PASS
2. 현재 작업 트리와 공개용 깨끗한 Git 이력에 대한 secret·개인정보 검사 PASS
3. 개인 영상·전사·로그·절대 사용자 경로·인증 자료가 공개 커밋과 릴리스 자산에 없음
4. 사용자가 라이선스를 결정하고 루트 `LICENSE`를 추가함
5. `THIRD-PARTY-NOTICES.md`와 포함 바이너리·모델 라이선스 검토 PASS
6. 단위·Rust·프론트·실제 진입점 E2E·1~2시간·8시간 빠른 분석 게이트 통과
7. 새로 설치한 Windows 환경에서 설치, 실행, 분석, 제거를 스모크 테스트함
8. 설치 EXE·내장 바이너리·모델의 SHA-256과 `BUILD-MANIFEST.md`가 일치함
9. `docs/V0.3.2-RELEASE.md`, `CHANGELOG.md`, `docs/DEBUGGING.md`, `HANDOFF.md`가 실제 결과와 일치함
10. GitHub Release에서 설치 EXE를 직접 내려받아 SHA-256·실행·표시 버전을 다시 확인함

하나라도 미확인일 때는 `HOLD`이며, 설치 파일 없이 소스만 공개해 목표를 축소하지 않는다.

## GitHub 보안 설정

- `main` 직접 push를 제한하고 PR과 필수 검사를 사용한다.
- GitHub Actions 기본 권한은 `contents: read`로 두고 필요한 job에만 최소 권한을 부여한다.
- fork PR에는 배포 secret을 전달하지 않는다.
- 개인 PC를 public 저장소의 상시 self-hosted runner로 사용하지 않는다.
- `pull_request_target`은 신뢰할 수 없는 PR 코드를 실행하는 용도로 사용하지 않는다.
- 외부 Action은 전체 commit SHA로 고정하고 Dependabot·CodeQL·secret scanning 결과를 검토한다.
- 공개 저장소에서 private vulnerability reporting을 활성화하고 `SECURITY.md`로 신고 경로를 안내한다.

## 설치 파일 신뢰 안내

Authenticode 코드 서명이 가장 좋은 배포 형태다. 인증서가 준비되지 않은 첫 릴리스는 다음 조건을 모두 충족해야 한다.

- 코드 서명 없음과 SmartScreen 경고 가능성을 다운로드 화면과 릴리스 노트에 명시
- SHA-256, SBOM, 빌드 명세, 소스 태그를 함께 제공
- 서명되지 않은 설치 파일을 “서명됨” 또는 “신뢰됨”으로 표현하지 않음

코드 서명 인증서 구매는 v0.3.2 구현의 필수 범위가 아니지만, 서명 부재는 릴리스 문서의 알려진 문제로 남긴다.

## 릴리스·롤백

1. private staging에서 v0.3.2 RC를 검증한다.
2. 공개 허용 파일만 새 공개 이력으로 내보내고 다시 검사한다.
3. `hangokudao/vod-scout`에 검증된 소스를 push한다.
4. `v0.3.2` annotated tag와 GitHub Release를 만든다.
5. 설치 EXE·체크섬·SBOM을 업로드하고 공개 다운로드 스모크를 수행한다.
6. 심각한 회귀나 무결성 불일치가 발견되면 Release 자산을 더 배포하지 않고 릴리스를 명확히 철회한다. 설치본을 조용히 같은 파일명으로 교체하지 않고 수정 버전으로 다시 릴리스한다.

공식 GitHub 문서 기준:

- [About releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
- [About large files on GitHub](https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-large-files-on-github)
- [Secure use of GitHub Actions](https://docs.github.com/en/actions/reference/security/secure-use)
- [Private vulnerability reporting](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/configuring-private-vulnerability-reporting-for-a-repository)
