# VOD Scout v0.3.4 통합 인계서

현재 게이트: **v0.3.4 공개 릴리스·인앱 updater·DisplayVersion·작업 보존 PASS · Authenticode/SmartScreen HOLD · 포스트 릴리스 문서 PR 대기 · v0.4.0 구현 전 HOLD**

## v0.3.4 현재 상태

- 공개 저장소: https://github.com/hangokudao/vod-scout
- 공개 Release: https://github.com/hangokudao/vod-scout/releases/tag/v0.3.4 (ID `365895027`, published `2026-08-06T00:19:47Z`, latest)
- exact merge commit: `a341bae3dcf65ff60ecdd3b1ac5c04dd6140952a` (PR #9)
- annotated tag `v0.3.4` object `ea5d807a3535f8fede188d255d2fe7fbf4b03bd0` → peel `a341bae…`
- Actions: run `31057676958` Release Windows app **success**
- 버전 정본: `package.json`, lock, Cargo.toml/lock, `tauri.conf.json`, README 설치 링크·파일명 모두 `0.3.4`
- 기능 요약: 설정 진입점·다크 입력 카드·job-scoped 취소·디스크 샘플러 문서화 (스키마 호환 유지)
- Authenticode: 인증서 없음 → **HOLD** (NotSigned; minisign updater와 별개)
- v0.4.0 P0~P7: 구현하지 않음

## 마지막 PASS

| 항목 | 결과 |
|---|---|
| exact merge + 버전 정렬 | PASS · `a341bae…` |
| 태그·CI 초안 릴리스 | PASS · tag peel exact · run `31057676958` success |
| 공개 5자산 계약 | PASS · 설치 EXE 233,849,362 · SHA-256 `6848c438…2fa3` · `.sig` · `latest.json` · `SHA256SUMS` · SBOM |
| 공개 직접 다운로드·API latest | PASS · unauth HTTP 200 · latest=v0.3.4 · digests 일치 |
| updater minisign | PASS · 앱 공개키 대비 설치 EXE 검증 |
| 인앱 v0.3.3→v0.3.4 | PASS · 설정 `지금 업데이트`만 · 메인/`uninstall`/`DisplayVersion` 모두 `0.3.4` · 설정 `최신 상태` |
| 작업·데이터 보존 | PASS · 작업 15 · 데이터 파일 2,087 · 해시/크기/mtime 변경 0 |
| 실제 YouTube 취소·재개 | PASS · 1.4s / 3.4s (하드캡 8s) |
| 전체 병합 디스크 피크 | PASS · peak ~13.08 GiB · final ~6.58 GiB |
| 단위/보안/SBOM (소스 게이트) | PASS · cargo 32 · npm 34 · security 6 · SBOM 656 pkgs |

상세: `docs/V0.3.4-RELEASE.md`, `BUILD-MANIFEST.md`, `CHANGELOG.md`, `docs/DEBUGGING.md`

## 남은 HOLD

1. **Windows Authenticode / SmartScreen** — 설치 EXE·앱 `NotSigned`. 인증서 구매·생성 미승인. updater minisign은 PASS.
2. **과거 DisplayVersion 근본 원인** — v0.3.2→v0.3.3에서 ARP `0.3.2` 잔류 원인은 미확정. v0.3.3→v0.3.4 인앱 경로 결과는 PASS.
3. **Whisper 음성 인식 중 취소** — 미실행.
4. **v0.4.0 P0–P7** — 구현·merge·release 미승인(범위는 별도 계획).

## v0.3.3 공개 기준선 (변경 금지)

- Release: https://github.com/hangokudao/vod-scout/releases/tag/v0.3.3
- exact commit: `5f756af7390325a99f2820a424f7d4ef05334d14`
- 상세: `docs/V0.3.3-RELEASE.md`, `BUILD-MANIFEST.md`

## 다음 작업

1. 이 포스트 릴리스 문서 브랜치 `codex/v034-post-release-docs` PR을 main에 병합(문서만; 제품 코드·태그·자산 변경 없음).
2. Authenticode 정책은 사용자 승인 후에만 진행. 인증서 없이 PASS로 기록하지 않는다.
3. v0.4.0은 별도 승인·브랜치에서만 시작. 이 문서 작업은 v0.4 구현을 포함하지 않는다.
4. Whisper 중 취소 등은 필요 시 후속 패치에서 측정한다.

기존 사용자 데이터와 설치본을 삭제하지 않는다. 공개 문서에 비밀값·불필요한 개인 홈 경로를 넣지 않는다. 빌드 산출물(`src-tauri/target`, `dist`)은 스테이징하지 않는다.
