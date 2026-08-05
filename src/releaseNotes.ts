export const CURRENT_RELEASE_NOTES = {
  version: "0.3.4",
  added: [
    "설정 진입점에 톱니바퀴 아이콘과 설정 문구 표시",
    "취소 중 종료 대상 안내와 작업 범위 프로세스 종료 감독"
  ],
  changed: [
    "어두운 화면 입력 카드 배경을 테마 변수로 맞춤",
    "내려받기 병합 중 열린 파일도 포함하는 디스크 사용량 표본 수집"
  ],
  fixed: [
    "취소 요청이 디스크 저장보다 늦게 반영되던 순서",
    "응답하지 않는 자식 프로세스 트리 종료가 한없이 기다릴 수 있던 경로"
  ],
  security: [
    "프로세스 종료 범위를 현재 작업의 확인된 자식 트리로 제한",
    "새 외부 AI·API 전송 경로와 API 키 저장 경로를 추가하지 않음"
  ],
  knownIssues: [
    "Windows Authenticode 인증서가 없어 첫 설치에서 SmartScreen 경고가 표시될 수 있음",
    "실제 설치·업데이트 뒤 제거 프로그램 DisplayVersion 일치는 제어된 v0.3.4 재현 전까지 확인 전"
  ]
} as const;
