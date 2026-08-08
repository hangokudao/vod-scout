# VOD Scout 인계서 (v0.4.0 출시 완료)

현재 게이트: **v0.4.0 출시 완료 · Latest 공개 Release·updater 서명·installer smoke·공개 다운로드 PASS**

## 저장소 상태

| 항목 | 값 |
|---|---|
| 저장소 | `hangokudao/vod-scout` |
| PR #13 | **MERGED** · squash `16c35f2dfa601790689d7295ceaea12af42169b8` |
| PR #12 | **MERGED** · squash `ea7aa08c1f217a0f537e0252ae5ed4b25143b374` |
| release PR #14 | **MERGED** · squash와 태그 대상 `a39d62eda4666ac848fe2e7aadaa1e74c7b9a53e` |
| installer smoke 수정 PR #15 | **MERGED** · squash `2f132d017f384c33d8010a7e4c7edc6665ca38aa` |
| 공개 Release | **PASS** · https://github.com/hangokudao/vod-scout/releases/tag/v0.4.0 · Latest |
| 현재 제품 버전 | `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` 모두 `0.4.0` |

## v0.4.0 범위

이번 출시 범위는 완성된 P0다.

- 장시간 입력을 청크 단위로 처리
- 체크포인트 schema 4 호환성, 마지막 정상 세대 보존, 손상 복구와 재개
- 분석 범위 밖 후보와 근거 없는 후보 제외
- 취소 시 현재 작업의 자식 프로세스 정리
- YouTube 전송 전에 정확한 선택 스트림 크기로 저장 공간 확인
- 외부 AI 없이 로컬 CPU 경로로 전체 작업 완료와 후보 생성

P1~P7의 자막 검색, 이야기 후보 확장, GPU와 외부 AI는 후속 아이디어이며 v0.4.0 출시 조건이 아니다.

## 실제 P0 검증

exact 코드 HEAD: `e18b73efcb0ea40be812b7da12572e1207854863`

| 항목 | 결과 |
|---|---|
| 자동·보안 테스트 | **PASS** |
| FAT32 저용량 차단 | **PASS** · 첫 미디어 바이트 전 중단 |
| 쿠키 없는 짧은 실제 전송 | **PASS** · 약 154 MB |
| 장시간 전체 다운로드·분석 | **PASS** · 약 8시간 53분 입력 |
| 최종 상태 | `REVIEW_READY` · 후보 8개 |
| 처리 시간 | 약 4,004.51초 |
| 최종 작업 크기 | 7,068,418,335 bytes |
| 피크 Working Set 합 | 약 2,054,066,176 bytes |
| 취소·hang·체크포인트 사본 | **PASS** |
| 종료 후 관련 자식 프로세스 | 없음 |

이번 문서 갱신 세션에서 다시 확인한 검사:

- `npm.cmd test` — 34 passed
- `npm.cmd run test:security` — 6 passed
- `npm.cmd run build` — PASS
- `cargo test --manifest-path src-tauri/Cargo.toml` — 54 passed, 1 ignored
- `cargo test --manifest-path src-tauri/fixture-worker/Cargo.toml` — 5 passed
- `node --test scripts/sample-disk-usage.test.mjs` — 3 passed

## 공개 배포 검증

- release workflow run `31239478473` — **PASS**
- installer smoke run `31240405719` — 버전 0.4.0, runtime 28개, 실행·재실행 **PASS**
- 공개 자산 5개 토큰 없는 직접 다운로드 — 모두 HTTP 200
- 설치 파일 — 233,879,794 bytes · SHA-256 `5ba960e35ae9d55512ecc79b8c49da611e7bb3fec12210f87e5496056a3f578a`
- `latest.json` 버전·URL·서명과 `.sig` 일치 — **PASS**
- SPDX SBOM과 `SHA256SUMS.txt` 공개 — **PASS**

## 알려진 한계

- exact HEAD의 순간 임시 파일 최대값은 다시 측정하지 않았다. 같은 영상의 기존 측정 peak 14,045,353,616 bytes와 최종 7,068,902,876 bytes를 참고값으로 기록한다.
- 후속 YouTube 요청에서 `Sign in to confirm you're not a bot`가 발생할 수 있다. 로그인 우회 없이 외부 제한으로 처리한다.
- Windows Authenticode 인증서가 없어 SmartScreen 경고가 표시될 수 있다. updater 서명은 포함돼 있다.
- `npm audit` high 1건은 개발용 Vite→PostCSS→`nanoid@3.3.16` 경로다. 제품 실행 경로는 해당 사용자 정의 생성기를 호출하지 않는다.
- 로컬 NSIS 본문은 버전 0.4.0과 runtime 28개 해시를 확인했지만 updater 개인키가 없어 서명하지 않았다. 로컬 빌드 경로 문자열도 포함하므로 공개하지 않고 GitHub Actions 산출물만 배포한다.

## 다음 작업

v0.4.0 출시 작업은 남아 있지 않다. P1~P7 아이디어는 필요성이 확인될 때 별도 버전과 별도 승인으로 진행한다.

원본 작업 폴더, 기존 설치 폴더, 기존 작업 데이터와 개인 파일은 수정하거나 삭제하지 않는다. 공개 문서에는 비밀값과 개인 절대 경로를 넣지 않는다.
