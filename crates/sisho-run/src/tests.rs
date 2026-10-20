use std::time::{Duration, Instant};

use crate::{Job, Outcome, Report, Status, Step, run_parallel};

fn sh(script: &str) -> Step {
    Step::new("sh", "sh").arg("-c").arg(script)
}

#[test]
fn exit_zero_passes_and_output_is_captured() {
    let outcome = sh("echo out; echo err >&2").run();
    assert_eq!(outcome.status, Status::Passed);
    assert_eq!(outcome.stdout, "out\n");
    assert_eq!(outcome.stderr, "err\n");
}

#[test]
fn non_zero_exit_fails_with_its_code() {
    let outcome = sh("exit 3").run();
    assert_eq!(outcome.status, Status::Failed { code: 3 });
    assert!(!outcome.is_pass());
}

#[cfg(unix)]
#[test]
fn a_signal_is_not_an_exit_code() {
    let outcome = sh("kill -KILL $$").run();
    assert_eq!(outcome.status, Status::Signalled { signal: 9 });
}

#[test]
fn a_missing_program_is_not_found_not_a_pass() {
    let outcome = Step::new("ghost", "sisho-no-such-tool-0f3a").run();
    assert_eq!(outcome.status, Status::NotFound);
    assert!(!outcome.is_pass());
}

#[test]
fn a_program_that_cannot_start_is_reported_with_its_reason() {
    // A directory is found on the path but cannot be executed.
    let outcome = Step::new("dir", "/").run();
    assert!(
        matches!(outcome.status, Status::CouldNotStart { .. }),
        "got {:?}",
        outcome.status
    );
}

#[test]
fn standard_input_is_closed() {
    // `cat` with a closed stdin ends at once; with an inherited one it would
    // hang the test run.
    let outcome = Step::new("cat", "cat").run();
    assert_eq!(outcome.status, Status::Passed);
    assert_eq!(outcome.stdout, "");
}

#[test]
fn current_dir_is_honoured() {
    let outcome = sh("pwd").current_dir("/").run();
    assert_eq!(outcome.stdout, "/\n");
}

#[test]
fn command_line_joins_program_and_arguments() {
    let step = Step::new("check", "tomet")
        .arg("check")
        .args(["--json", "."]);
    assert_eq!(step.command_line(), "tomet check --json .");
    assert_eq!(step.name(), "check");
}

#[test]
fn parallel_results_keep_plan_order_not_finish_order() {
    let slow = sh("sleep 0.3; echo slow");
    let fast = sh("echo fast");
    let outcomes = run_parallel(vec![Job::step(&slow), Job::step(&fast)]);
    assert_eq!(outcomes[0].stdout, "slow\n");
    assert_eq!(outcomes[1].stdout, "fast\n");
}

#[test]
fn parallel_streaming_delivers_outcomes_as_they_finish() {
    use std::sync::Mutex;
    let slow = sh("sleep 0.2; echo slow");
    let fast = sh("echo fast");
    let received = Mutex::new(Vec::new());

    let outcomes = crate::run_parallel_streaming(
        vec![Job::step(&slow), Job::step(&fast)],
        |index, outcome| {
            received.lock().unwrap().push((index, outcome.stdout));
        },
    );

    assert_eq!(outcomes[0].stdout, "slow\n");
    assert_eq!(outcomes[1].stdout, "fast\n");

    let streamed = received.into_inner().unwrap();
    assert_eq!(streamed.len(), 2);
    // fast finished first even though it was index 1
    assert_eq!(streamed[0].0, 1);
    assert_eq!(streamed[0].1, "fast\n");
    assert_eq!(streamed[1].0, 0);
    assert_eq!(streamed[1].1, "slow\n");
}


#[test]
fn parallel_jobs_overlap_in_time() {
    let a = sh("sleep 0.5");
    let b = sh("sleep 0.5");
    let started = Instant::now();
    let outcomes = run_parallel(vec![Job::step(&a), Job::step(&b)]);
    let elapsed = started.elapsed();
    assert!(outcomes.iter().all(Outcome::is_pass));
    // Run one after the other these would take a full second.
    assert!(elapsed < Duration::from_millis(900), "took {elapsed:?}");
}

#[test]
fn in_process_work_mixes_with_steps() {
    let step = sh("echo from-a-tool");
    let outcomes = run_parallel(vec![
        Job::step(&step),
        Job::new(|| Outcome::in_process("in-process", Status::Passed, "", "", Duration::ZERO)),
    ]);
    assert_eq!(outcomes[0].name, "sh");
    assert_eq!(outcomes[1].name, "in-process");
}

#[test]
fn a_report_passes_only_if_every_step_passed() {
    let ok = sh("exit 0").run();
    let bad = sh("exit 1").run();
    let missing = Step::new("ghost", "sisho-no-such-tool-0f3a").run();

    assert_eq!(Report::new(vec![]).exit_code(), 0);
    assert_eq!(Report::new(vec![ok.clone()]).exit_code(), 0);
    assert_eq!(Report::new(vec![ok.clone(), bad.clone()]).exit_code(), 1);

    let report = Report::new(vec![ok, bad, missing]);
    assert_eq!(report.exit_code(), 1);
    let names: Vec<_> = report.failures().map(|o| o.name.as_str()).collect();
    assert_eq!(names, ["sh", "ghost"]);
}

#[test]
fn an_inherited_run_reports_the_same_statuses() {
    assert_eq!(sh("exit 0").run_inherited(), Status::Passed);
    assert_eq!(sh("exit 4").run_inherited(), Status::Failed { code: 4 });
    assert_eq!(
        Step::new("ghost", "sisho-no-such-tool-0f3a").run_inherited(),
        Status::NotFound
    );
    assert!(matches!(
        Step::new("dir", "/").run_inherited(),
        Status::CouldNotStart { .. }
    ));
}

#[cfg(unix)]
#[test]
fn an_inherited_run_also_tells_a_signal_from_an_exit_code() {
    assert_eq!(
        sh("kill -KILL $$").run_inherited(),
        Status::Signalled { signal: 9 }
    );
}
