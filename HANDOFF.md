# VOD Scout v0.3.3 공개 배포 인계서

현재 게이트: **v0.3.3 공개 배포·실제 업데이트·기존 작업 보존 PASS · Authenticode와 제거 프로그램 레지스트리 버전 표시는 HOLD**

## v0.3.3 최종 상태

- 공개 저장소: https://github.com/hangokudao/vod-scout
- 구현 PR #5와 최종 릴리스 workflow PR #6을 squash merge했다. v0.3.3 태그와 설치본의 exact commit은 `5f756af7390325a99f2820a424f7d4ef05334d14`다.
- 공개 Release: https://github.com/hangokudao/vod-scout/releases/tag/v0.3.3
- 공개 Actions run `30963107742` PASS. 설치 EXE·updater 서명·`latest.json`·체크섬·SPDX 2.3 SBOM 5개 자산의 GitHub digest와 재다운로드 SHA-256이 일치했다.
- 최종 기본 검증: 프런트 2개 파일·32개, Vite 1,793 modules, Rust 27개 통과·1개 무시, fixture-worker 5개, archive 안전성 6개 PASS. npm 취약점 0개.
- 승인된 실제 입력: YouTube `JN3BO9GLuFU`, 실제 미디어 `08:53:19.981`, 720p. 빠른 분석 `11/11`, `17/17`, 후보 8개, `REVIEW_READY` PASS.
- 처리 시간·자원: 첫 시작부터 검토 준비 `36분 01.231초`, 누적 활성 `34분 28.605초`; 최대 working set `934,158,336 bytes`, private bytes `1,123,246,080 bytes`, GPU 보드 메모리 기준선/최대 `1,682/2,012 MiB`, 분석 WAV 최대 `19,200,078 bytes`, 최종 작업 데이터 `7,097,358,568 bytes`.
- 실제 v0.3.2 설치본에서 공개 v0.3.3을 찾아 설치하고 재실행했다. 앱·실행 파일은 `v0.3.3`, 설정 화면은 `최신 상태`였다.
- 기존 작업 14개와 현재 작업 `#92bbf85a`, 후보 8개를 다시 열었다. 현재 작업의 핵심 상태·체크포인트 5개 SHA-256은 업데이트 전과 일치했다.
- 설치된 runtime manifest 28개 파일의 SHA-256이 전부 일치했다.

## 남은 HOLD

- Windows Authenticode 인증서가 없다. 구매하거나 대체 인증서를 만들지 않았으며 설치 EXE와 앱 실행 파일은 `NotSigned`다.
- 업데이트 뒤 실행 파일과 `uninstall.exe`는 `0.3.3`이지만 HKCU 제거 프로그램 레지스트리 `DisplayVersion`은 `0.3.2`로 남았다. 원인은 확정하지 않았다.
- YouTube 영상·음성 병합 중 열린 출력 파일을 측정 도구가 읽지 못해 내려받기 단계의 정확한 순간 최대 임시 용량은 `HOLD`다. 닫힌 입력 파일 최소 확인값은 `7,070,731,050 bytes`다.

상세 결과는 `docs/V0.3.3-RELEASE.md`, `BUILD-MANIFEST.md`, `docs/DEBUGGING.md`를 따른다. v0.3.3 배포에 추가 작업은 필요하지 않으며, 다음 패치에서 제거 프로그램 레지스트리 버전 갱신만 별도로 재현한다.

## 이전 v0.3.2 공개 배포 상태

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
- Authenticode와 Tauri updater 서명은 별개다. 기존 GitHub Actions signing secret으로 생성한 updater 서명은 검증 PASS지만 Authenticode는 `HOLD`다.
- 빠른 분석은 전체 32,000초 중 6,400초를 시간대별로 골라 처리한다. 전체 정밀 분석 성능과 누락 없는 탐지를 주장하지 않는다.
- GPU 음성 인식, 채팅 글자 인식·자동 영역 탐색, LLM 후보 재정렬·개인화는 지원하지 않는다.

## 다음 작업

1. 화면 오른쪽 위의 버전 표시가 설정 버튼임을 바로 알 수 있도록 톱니바퀴 아이콘, `설정` 문구, 버튼 모양과 대비를 보강한다.
2. YouTube 작업 취소가 1분 이상 `worker 종료 요청`에 머무는 조건을 재현하고, 취소 요청부터 worker와 자식 프로세스 종료·최종 상태 반영까지 걸린 시간을 측정해 지연 지점을 수정한다.
3. 다음 패치 계획에서 NSIS updater 뒤 HKCU `DisplayVersion`이 갱신되지 않는 조건을 격리 재현한다.
4. 장시간 측정 도구가 병합 중 열린 출력 파일 길이를 안전하게 표본 수집하도록 보완한 뒤 내려받기 순간 최대 임시 용량을 다시 측정한다.
5. 그 외 v0.4.0 기능, 검색 기능, 새 기능, 리팩터링은 별도 승인과 계획 없이 시작하지 않는다.

기존 사용자 데이터와 설치본을 삭제하지 않는다.
