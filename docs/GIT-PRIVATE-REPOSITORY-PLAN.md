# VOD Scout 비공개 준비·공개 전환 계획

상태: **완료 · private staging과 clean public export 적용**
대상 프로젝트: 이 문서가 포함된 VOD Scout 프로젝트 루트

## 현재 확인된 상태

- 프로젝트와 상위 경로에 유효한 Git 저장소가 없다.
- 작업 시작 당시 상위 경로의 `.git`은 항목이 없는 빈 디렉터리였으며 저장소가 아니었다.
- 연결된 GitHub 계정은 `hangokudao`다.
- 연결된 GitHub 범위에서 `vod-scout` 또는 `VOD Scout` 저장소는 검색되지 않았다.
- Windows에 Git `2.53.0`은 있지만 GitHub CLI `gh`는 없다.
- 첨부 보안 점검은 현재 파일에서 알려진 API 키·개인키 형식과 고엔트로피 비밀값을 발견하지 못했다. Git 과거 이력은 존재하지 않아 검사할 수 없다.

이 문서는 계획만 승인한다. `git init`, 커밋, GitHub 저장소 생성, push는 사용자의 별도 실행 승인 전에는 하지 않는다.

## 제안 대상

| 항목 | 제안 값 | 게이트 |
|---|---|---|
| GitHub 소유자 | `hangokudao` | 생성 직전 canonical 계정 재확인 |
| 저장소 이름 | `vod-scout-dev` | 동일 이름 충돌 재검색 |
| 공개 범위 | `private` | 생성 후 API로 private 재확인 |
| 기본 브랜치 | `main` | 최초 push 뒤 read-back |
| 첫 작업 브랜치 | `security/v0.3.2` | 기준선 push 뒤 생성 |

비공개 저장소도 비밀 저장소가 아니다. 키·인증서·개인 영상·전사·로그는 커밋하지 않는다.

## 커밋 대상과 제외 대상

### 커밋

- `src/`, `src-tauri/src/`, `src-tauri/fixture-worker/src/`
- `scripts/`, 프로젝트 설정, lock 파일
- 고정 다운로드 URL·SHA-256을 가진 준비 스크립트
- README, 라이선스, 계획·테스트·릴리스·디버깅 문서
- 아이콘 등 소형 정적 자산

### 제외

- `node_modules/`, `dist/`, 모든 Cargo `target/`
- `VOD-Scout-*-setup.exe`
- 생성된 `fixture-worker.exe`와 다운로드된 FFmpeg·Whisper·yt-dlp·Deno·모델
- MP4/MKV/WebM/MOV/AVI/FLV/WAV/SRT와 분석 작업 데이터
- 로그, 캡처, E2E 임시 데이터, `%LOCALAPPDATA%/com.vodscout.app`
- `.env*`, `*.pem`, `*.key`, `*.pfx`, `*.p12`, `*.jks`, `*.keystore`
- API 토큰·쿠키·OAuth 파일·서비스 계정 JSON·브라우저 프로필

다운로드 리소스와 설치 파일은 Git이 아니라 준비 스크립트·고정 해시·`BUILD-MANIFEST.md`로 재현한다. 설치 EXE는 일반 Git 커밋에서만 제외하며, 보안 게이트 통과 후 public GitHub Release의 필수 자산으로 제공한다.

## 실행 순서와 게이트

### G0. 쓰기 전 기준선

1. 취약점 점검과 다른 빌드가 파일을 수정 중인지 확인한다.
2. 프로젝트 파일 목록·크기·최근 수정 시각을 기록한다.
3. 현재 보안 HOLD와 미수정 상태를 문서에 남긴다.

완료 조건: 동일 파일을 다른 작업이 쓰고 있지 않고, 기준선 목록이 저장됨.

### G1. `.gitignore` 보강

비밀·대용량·생성물을 차단하는 규칙을 추가한 뒤, 금지 파일을 일부러 생성하는 테스트 fixture로 ignore 동작을 확인한다. 기존 설치 파일과 미디어 도구가 stage 후보에 나타나면 실패다.

완료 조건: `git check-ignore`와 stage 목록에서 모든 금지 유형이 차단됨.

### G2. 최초 secret·개인정보 검사

1. 소스·문서·설정·lock 파일에서 알려진 OpenAI, Anthropic, GitHub, AWS, Google 키 패턴과 개인키 헤더를 검색한다.
2. 고엔트로피 문자열을 별도 검사한다.
3. 절대 사용자 경로, 이메일, 영상 URL, 전사문, 쿠키를 검색한다.
4. `gitleaks` 또는 동등 도구를 현재 작업 트리 대상으로 실행한다. 도구가 없으면 설치를 조용히 가정하지 않고 대체 검사와 한계를 기록한다.
5. 발견된 실제 키는 커밋하지 않고 먼저 폐기·회전한다. 단순 예시 문자열은 근거와 함께 예외 처리한다.

완료 조건: 실제 비밀값 0개. 미확인 결과가 있으면 `HOLD`.

### G3. 대용량·민감 파일 stage 검사

1. 10MB·50MB·100MB 초과 파일을 각각 목록화한다.
2. `git add` 뒤 `git diff --cached --stat`, `git diff --cached`, `git ls-files`를 검토한다.
3. 설치 EXE, 모델, DLL, 영상, 전사, 작업 로그가 하나라도 stage되면 초기화 작업을 중단한다.

완료 조건: 재현 가능한 소스와 문서만 stage됨.

### G4. 로컬 기준선

프로젝트 루트에서 `main`으로 초기화하고 `chore: establish v0.3.1 security-hold baseline` 커밋을 만든다. 이 커밋은 안전한 릴리스 선언이 아니라, 알려진 취약점 수정 전 상태를 보존하는 기준선이다.

권장 로컬 태그: `baseline-v0.3.1-hold`. 공식 `v0.3.1` 릴리스 태그는 설치본 무결성이 HOLD이므로 만들지 않는다.

완료 조건: 깨끗한 working tree, 커밋 파일 목록·작성자·이메일 확인, 금지 파일 0개.

### G5. GitHub private 생성과 push

1. GitHub 계정 `hangokudao`와 저장소 부재를 다시 확인한다.
2. `hangokudao/vod-scout-dev`를 private로 생성한다.
3. private 상태를 읽어 확인한 뒤에만 `origin`을 추가한다.
4. `main`을 push하고 원격 커밋 SHA가 로컬과 같은지 확인한다.
5. 저장소 공개 범위가 private가 아니거나 계정이 다르면 즉시 중단한다.

완료 조건: 정확한 계정·private·SHA 일치. 외부 쓰기이므로 사용자 실행 승인이 필수다.

### G6. private PR 운영과 v0.3.2 검증

- `main` 직접 개발 금지
- 기능·보안별 브랜치 사용: `security/v0.3.2`, `feature/analysis-profiles`
- PR마다 테스트, 보안 영향, 체크포인트 호환성, 문서 변경을 기록
- 리뷰 발견 사항이 `HOLD`면 병합하지 않음
- v0.3.2의 코드·보안·설치본 게이트가 PASS가 될 때까지 public 저장소에 push하지 않음

### G7. 공개용 깨끗한 이력 준비

1. private staging의 검증된 태그 후보에서 공개 허용 파일만 새 디렉터리와 새 Git 이력으로 내보낸다.
2. 공개 export에 secret·고엔트로피 문자열·개인 경로·이메일·영상 URL·전사·로그·대용량 바이너리가 없는지 다시 검사한다.
3. 승인된 `LICENSE`, `CONTRIBUTING.md`, `SECURITY.md`, 제3자 고지와 빌드 재현 문서를 확인한다.
4. 공개 source archive를 만든 뒤 그 archive 자체도 목록과 해시로 검사한다.

완료 조건: 실제 비밀값과 개인정보 0개, 금지 바이너리 0개, 라이선스 게이트 PASS.

### G8. public 저장소와 첫 GitHub Release

1. GitHub 계정 `hangokudao`와 `vod-scout` 저장소 부재를 다시 확인한다.
2. `hangokudao/vod-scout`를 public으로 생성하고 공개 상태를 read-back한다.
3. 깨끗한 `main`을 push하고 원격 SHA를 검증한다.
4. `v0.3.2` annotated tag와 GitHub Release를 만든다.
5. 설치 EXE·`SHA256SUMS.txt`·`SBOM.spdx.json`을 Release 자산으로 올린다.
6. 공개 URL에서 다시 내려받아 해시·버전·설치·실행을 확인한다.

완료 조건: public·SHA 일치, 필수 Release 자산 존재, 직접 다운로드 스모크 PASS. 이 시점부터 `hangokudao/vod-scout`가 일반 개발의 정본이다.

## 사고 대응

비공개 저장소에 키를 잘못 push해도 삭제 커밋만으로 해결되지 않는다.

1. 노출된 키를 먼저 폐기·회전한다.
2. push와 자동화 실행을 중단한다.
3. 전체 Git 이력에서 비밀값을 제거한다.
4. 강제 push 영향과 collaborator clone을 확인한다.
5. 사고·회전·검증 결과를 보안 문서에 남긴다.

## 이 계획에서 하지 않는 것

- 첨부·영상·전사·설치 파일을 일반 Git 커밋에 백업하지 않는다. 검증된 설치 EXE만 공식 GitHub Release 자산으로 배포한다.
- 사용자 승인 없이 GitHub 저장소를 만들거나 remote를 추가하지 않는다.
- GitHub private라는 이유로 코드 서명·ACL·바이너리 해시 문제를 안전하다고 보지 않는다.
