# VOD Scout 보안 점검 인수 기록

상태: **v0.3.2 HIGH·MEDIUM 게이트 PASS**
입력: 2026-08-02 전달받은 약 50분 분량의 외부 점검 결과  
주의: 이 문서는 전달받은 결과를 개발 게이트로 구조화한 것이며, 각 발견 사항을 이번 문서화 단계에서 독립적으로 재현한 것은 아니다.

## 사용자에게 중요한 결론

- 현재 소스·문서·설치 산출물·검사한 앱 데이터에서 OpenAI·Anthropic·GitHub·AWS·Google 키나 개인키 헤더가 발견됐다는 증거는 없다.
- 현재 프로세스와 사용자·시스템 영구 환경변수에서도 민감 변수 이름이 보고되지 않았다.
- 새 v0.3.2 패키지는 current-user ACL, 실행 파일의 개인 경로 제거, runtime 28개 해시 검증과 updater 서명 검증을 통과했다.
- 기존 `D:\VOD Scout` 설치본은 새 패키지 검증과 별개이므로 계속 신뢰하지 않으며, 이번 작업에서 수정·삭제하지 않았다.
- 과거 작업 12개는 사용자의 명시적 선택 없이 삭제하지 않는다.

## 발견 사항 등록부

| ID | 심각도 | 전달받은 근거 | v0.3.2 조치 | 완료 검증 | 상태 |
|---|---|---|---|---|---|
| SEC-001 | HIGH | 기존 비표준 설치 폴더의 EXE·DLL·yt-dlp에 `BUILTIN\Users: FullControl`, `Authenticated Users: Modify` 상속 | 사용자 전용 설치 경로와 ACL 강제, 공유 쓰기 가능 경로 실행 차단 | private Windows runner current-user 설치 후 공유 principal 쓰기 권한 0 | PASS |
| SEC-002 | HIGH | Authenticode 없음, 설치된 메인 EXE와 현재 release EXE 해시 불일치 | 설치·빌드 SHA 연결, 도구 실행 전 해시 검증, 불일치 시 차단; updater 서명 적용 | 설치본의 runtime 28개 SHA-256 재해시, minisign 독립 검증, 앱 실행 | PASS |
| SEC-003 | HIGH 개인정보 | 약 3.72GB·13개 작업 중 고아 작업 12개, 영상·URL·전사·로그 잔존 | 모든 작업 목록·용량·최근 시각 표시, 선택·전체 삭제와 재확인, 현재 작업만 복원 구조 개선 | fixture 격리 폴더에서 UUID 고아 선택·전체 삭제와 비 UUID 보존 | PASS |
| SEC-004 | MEDIUM | FFmpeg·yt-dlp·Whisper가 부모 환경과 사용자 권한을 상속, FFmpeg 네트워크 프로토콜 활성 | 자식 환경 `env_clear` 후 필수 값만 전달, 로컬 미디어 프로토콜 allowlist | 가짜 API 키 비상속, `file,crypto,data` allowlist, 정상 파일 회귀 | PASS |
| SEC-005 | MEDIUM | CSV 값이 `= + - @` 등으로 시작하면 Excel 수식 실행 가능 | 위험 시작 문자를 무력화하고 회귀 fixture 추가 | 5개 악성 prefix와 NUL 회귀 | PASS |
| SEC-006 | MEDIUM | IPC CSV 명령이 임의 사용자 쓰기 가능 경로를 덮어쓸 수 있음 | Rust 네이티브 저장 대화상자, `.csv`·symlink 경계 | Rust 경계 테스트·코드 리뷰 | PASS |
| SEC-007 | MEDIUM 개인정보 | release EXE에 빌드 PC 사용자 경로가 다수 포함 | release compile-time 절대 경로 제거, remap-path-prefix·strip, debug 전용 fallback 분리 | 재빌드한 EXE·installer에서 사용자명·빌드 루트 0건 | PASS |
| SEC-008 | LOW/MEDIUM | Asset Protocol이 모든 production·E2E 작업 파일을 포함 | `review-clips/*.mp4`로 축소하고 E2E scope를 테스트 전용 설정으로 분리 | production·E2E scope 구성 검증, 미리보기 PASS | PASS |
| SEC-009 | LOW/MEDIUM | 준비 완료 판정이 manifest 선언과 파일 존재만 확인 | 매 빌드 실제 파일 SHA-256 재계산, DLL·EXE·모델 목록 고정 | schema 5의 28개 runtime 파일 목록·SHA 및 변조 거부 | PASS |
| SEC-010 | LOW | 고정 ZIP을 `tar.exe -xf`로 바로 풀며 엔트리 경로 검증 없음 | 추출 전 절대·상위 경로·드라이브 경로·링크 엔트리 거부 | 정상 1개·공격 5개 Node 테스트 | PASS |
| SEC-011 | BLOCKED | 유효한 Git 이력이 없어 삭제된 키·과거 브랜치 검사 불가, `.gitignore` 비밀 방어 부족 | private 기준선 생성, secret scan, 방어 규칙 | GitHub 계정 `hangokudao`, private 원격 SHA, Git 이력·staged scan, 금지 파일 0개 | PASS |

## 전달받은 안전 확인

- 소스·문서·dist·설치 파일·release EXE에서 알려진 키 형식과 개인키 헤더 미발견
- 추가 고엔트로피 검사에서 비밀값 미발견
- 검사한 앱 데이터 텍스트 351개에서 알려진 키 형식 미발견
- OpenAI·Claude API 호출과 엔드포인트 미발견
- 점검 당시 VOD Scout TCP 연결 0개
- npm audit: 207개, 취약점 0건
- fixture-worker Rust 의존성 OSV 0건
- Windows 제품 경로에서 확인된 Rust 항목은 유지보수 중단 경고로 보고됐으며, 확인된 Windows 취약점으로 판정되지 않음
- 프론트엔드 임의 HTML 삽입·`eval`·`dangerouslySetInnerHTML` 미발견
- yt-dlp가 `--ignore-config`, `--no-playlist`, 고정 출력 파일명과 인자 배열을 사용

이 목록은 2026-08-02 점검 시점의 보고다. 버전이 바뀌면 다시 검사한다.

## 즉시 운영 경계

1. 기존 설치본은 원인 확인 전 신뢰하지 않는다.
2. 설치 폴더·과거 작업 데이터는 사용자의 명시적 승인 없이 변경·삭제하지 않는다.
3. v0.3.2 개발은 소스와 격리된 E2E 데이터에서 수행한다.
4. HIGH 항목 중 하나라도 미확인이면 새 설치본은 배포·교체하지 않는다.
5. 비밀값이 새로 발견되면 Git 삭제보다 키 폐기·회전을 먼저 한다.

## v0.3.2 보안 완료 게이트

- SEC-001~003 재현 테스트와 수정 검증 `PASS`
- SEC-004~010 관련 단위·통합 테스트 `PASS` 또는 명시적 비범위 승인
- 설치 후 ACL·EXE·도구·모델 해시 read-back `PASS`
- 설치본 내부와 release EXE에서 사용자명·빌드 경로 검색 0건
- 고아 작업 관리 화면에서 선택 삭제·전체 삭제·취소가 실제 격리 데이터로 검증됨
- 키 패턴·고엔트로피·개인키·환경변수 비상속 검사 `PASS`
- 결과가 `docs/V0.3.2-RELEASE.md`, `CHANGELOG.md`, `docs/DEBUGGING.md`, `BUILD-MANIFEST.md`에 반영됨

## 외부 참고 기준

- [OWASP CSV Injection](https://owasp.org/www-community/attacks/CSV_Injection)
- [Tauri Asset Protocol](https://v2.tauri.app/security/asset-protocol/)
- [FFmpeg Security](https://ffmpeg.org/security.html)
- [yt-dlp Releases](https://github.com/yt-dlp/yt-dlp/releases)
