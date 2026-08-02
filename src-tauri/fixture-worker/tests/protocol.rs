use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn worker() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fixture-worker"))
}

fn run(scenario: &str, start_unit: u8) -> std::process::Output {
    worker()
        .args([
            "--scenario",
            scenario,
            "--start-unit",
            &start_unit.to_string(),
        ])
        .output()
        .expect("worker should start")
}

fn json_lines(output: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(output)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[test]
fn normal_run_completes_once_with_three_candidates() {
    let output = run("normal", 0);
    assert!(output.status.success());
    let events = json_lines(&output.stdout);
    assert_eq!(
        events
            .iter()
            .filter(|event| event["kind"] == "completed")
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .find(|event| event["kind"] == "candidates")
            .unwrap()["candidates"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn crash_after_six_resumes_from_seven() {
    let crashed = run("crash", 0);
    assert_eq!(crashed.status.code(), Some(17));
    let crashed_events = json_lines(&crashed.stdout);
    assert_eq!(
        crashed_events
            .iter()
            .filter(|event| event["kind"] == "progress")
            .last()
            .unwrap()["unit"],
        6
    );

    let resumed = run("crash", 6);
    assert!(resumed.status.success());
    let resumed_events = json_lines(&resumed.stdout);
    let first_progress = resumed_events
        .iter()
        .find(|event| event["kind"] == "progress")
        .unwrap();
    assert_eq!(first_progress["unit"], 7);
}

#[test]
fn controlled_failure_is_structured() {
    let output = run("fail", 0);
    assert_eq!(output.status.code(), Some(2));
    let events = json_lines(&output.stdout);
    let failure = events
        .iter()
        .find(|event| event["kind"] == "failed")
        .expect("failure event");
    assert_eq!(failure["message"], "전사 도구가 응답하지 않았습니다.");
}

#[test]
fn hanging_worker_can_be_terminated_well_under_five_seconds() {
    let mut child = worker()
        .args(["--scenario", "hang", "--start-unit", "0"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("worker should start");
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();

    loop {
        line.clear();
        reader.read_line(&mut line).expect("read worker line");
        if line.contains("\"kind\":\"progress\"") && line.contains("\"unit\":4") {
            break;
        }
    }

    let started = Instant::now();
    child.kill().expect("kill hanging worker");
    child.wait().expect("reap hanging worker");
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn malformed_scenario_emits_non_json_for_parent_validation() {
    let mut child = worker()
        .args(["--scenario", "malformed", "--start-unit", "0"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("worker should start");
    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    let mut saw_malformed = false;
    for line in reader.lines() {
        let line = line.expect("read line");
        if serde_json::from_str::<Value>(&line).is_err() {
            saw_malformed = true;
            break;
        }
    }
    child.kill().expect("kill malformed worker");
    child.wait().expect("reap malformed worker");
    assert!(saw_malformed);
}
