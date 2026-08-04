use serde::Serialize;
use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum WorkerEvent<'a> {
    Heartbeat {
        unit: u8,
    },
    Progress {
        unit: u8,
        status: &'a str,
        stage_label: &'a str,
        message: &'a str,
    },
    Candidates {
        candidates: Vec<Candidate<'a>>,
    },
    Failed {
        message: &'a str,
        detail: &'a str,
    },
    Completed,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Candidate<'a> {
    id: &'a str,
    start_seconds: u32,
    end_seconds: u32,
    title: &'a str,
    summary: &'a str,
    transcript_excerpt: &'a str,
    audio_score: u8,
    dialogue_score: u8,
    chat_score: u8,
    total_score: u8,
    decision: &'a str,
    context_start_seconds: f64,
    context_end_seconds: f64,
    context_transcript: Vec<TranscriptEntry<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TranscriptEntry<'a> {
    start_seconds: f64,
    end_seconds: f64,
    text: &'a str,
}

fn emit(event: &WorkerEvent<'_>) {
    println!("{}", serde_json::to_string(event).expect("serialize event"));
    io::stdout().flush().expect("flush stdout");
}

fn value_after(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn stage(unit: u8) -> (&'static str, &'static str, &'static str) {
    match unit {
        1 => (
            "ACQUIRING",
            "입력 준비",
            "입력 소스를 작업 공간에 등록했습니다.",
        ),
        2 => (
            "PROBING",
            "미디어 확인",
            "컨테이너와 재생 시간을 확인했습니다.",
        ),
        3 => (
            "EXTRACTING_AUDIO",
            "오디오 준비",
            "분석용 오디오 청크를 준비했습니다.",
        ),
        4 => (
            "TRANSCRIBING",
            "전사 1/3",
            "첫 번째 전사 청크를 처리했습니다.",
        ),
        5 => (
            "TRANSCRIBING",
            "전사 2/3",
            "두 번째 전사 청크를 처리했습니다.",
        ),
        6 => (
            "TRANSCRIBING",
            "전사 3/3",
            "마지막 전사 청크를 처리했습니다.",
        ),
        7 => (
            "AUDIO_SIGNALS",
            "오디오 신호",
            "말 밀도와 반응 신호를 계산했습니다.",
        ),
        8 => (
            "CHAT_SIGNALS",
            "채팅 신호",
            "채팅 영역의 활동량을 계산했습니다.",
        ),
        9 => (
            "FUSING",
            "신호 결합 1/2",
            "겹치는 반응 구간을 하나로 묶었습니다.",
        ),
        10 => (
            "FUSING",
            "신호 결합 2/2",
            "너무 짧거나 중복된 구간을 정리했습니다.",
        ),
        11 => (
            "RANKING",
            "후보 점수",
            "로컬 규칙으로 후보 점수를 계산했습니다.",
        ),
        _ => (
            "RANKING",
            "검토 목록 준비",
            "검토할 후보 목록을 만들었습니다.",
        ),
    }
}

fn candidates() -> Vec<Candidate<'static>> {
    vec![
        Candidate {
            id: "candidate-1",
            start_seconds: 754,
            end_seconds: 802,
            title: "예상 밖의 보스 역전",
            summary: "목소리 반응과 채팅 활동이 동시에 치솟은 48초 구간",
            transcript_excerpt: "잠깐, 이게 된다고? 아니 진짜 잡았어!",
            audio_score: 92,
            dialogue_score: 81,
            chat_score: 95,
            total_score: 91,
            decision: "PENDING",
            context_start_seconds: 739.0,
            context_end_seconds: 817.0,
            context_transcript: vec![TranscriptEntry {
                start_seconds: 754.0,
                end_seconds: 802.0,
                text: "잠깐, 이게 된다고? 아니 진짜 잡았어!",
            }],
        },
        Candidate {
            id: "candidate-2",
            start_seconds: 1922,
            end_seconds: 1960,
            title: "시청자와 완벽한 티키타카",
            summary: "짧은 발화가 빠르게 오가고 채팅 반응이 이어진 38초 구간",
            transcript_excerpt: "그건 칭찬이 아니잖아. 방금 누가 인정했어?",
            audio_score: 66,
            dialogue_score: 94,
            chat_score: 86,
            total_score: 86,
            decision: "PENDING",
            context_start_seconds: 1907.0,
            context_end_seconds: 1975.0,
            context_transcript: vec![TranscriptEntry {
                start_seconds: 1922.0,
                end_seconds: 1960.0,
                text: "그건 칭찬이 아니잖아. 방금 누가 인정했어?",
            }],
        },
        Candidate {
            id: "candidate-3",
            start_seconds: 3288,
            end_seconds: 3349,
            title: "갑작스러운 웃음 붕괴",
            summary: "반복 웃음과 음량 변화가 길게 유지된 61초 구간",
            transcript_excerpt: "아 그만, 그만해. 나 진짜 숨 못 쉬겠어.",
            audio_score: 96,
            dialogue_score: 72,
            chat_score: 79,
            total_score: 84,
            decision: "PENDING",
            context_start_seconds: 3273.0,
            context_end_seconds: 3364.0,
            context_transcript: vec![TranscriptEntry {
                start_seconds: 3288.0,
                end_seconds: 3349.0,
                text: "아 그만, 그만해. 나 진짜 숨 못 쉬겠어.",
            }],
        },
    ]
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let scenario = value_after(&args, "--scenario").unwrap_or_else(|| "normal".into());
    let start_unit = value_after(&args, "--start-unit")
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(0);

    emit(&WorkerEvent::Heartbeat { unit: start_unit });

    for unit in (start_unit + 1)..=12 {
        thread::sleep(Duration::from_millis(220));
        emit(&WorkerEvent::Heartbeat { unit: unit - 1 });

        if scenario == "malformed" && unit == 3 && start_unit < 2 {
            println!("this-is-not-json");
            io::stdout().flush().expect("flush malformed event");
            thread::sleep(Duration::from_secs(2));
            continue;
        }

        if scenario == "fail" && unit == 5 && start_unit < 4 {
            emit(&WorkerEvent::Failed {
                message: "전사 도구가 응답하지 않았습니다.",
                detail: "fixture failure at unit 5",
            });
            std::process::exit(2);
        }

        if scenario == "hang" && unit == 5 && start_unit < 4 {
            thread::sleep(Duration::from_secs(60));
        }

        let (status, stage_label, message) = stage(unit);
        emit(&WorkerEvent::Progress {
            unit,
            status,
            stage_label,
            message,
        });

        if scenario == "crash" && unit == 6 && start_unit < 6 {
            std::process::exit(17);
        }
    }

    emit(&WorkerEvent::Candidates {
        candidates: candidates(),
    });
    emit(&WorkerEvent::Completed);
}
