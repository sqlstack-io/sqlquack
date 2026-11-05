use std::process::Command;
use std::time::{Duration, Instant};

use crate::check::models::{LocalFailure, LocalVerdict};
use crate::cli::_helpers::worker::Worker;
use crate::cli::tests::helpers::failed;
use crate::cli::tests::test_types::WorkerTestCase;

const REJECTED: &str = "while read l; do echo '{\"verdict\":\"rejected\"}'; done";

#[test]
fn given_a_worker_that_fails_when_parsing_then_only_that_case_fails_with_its_reason() {
    let test_cases = [
        WorkerTestCase {
            description: "a worker that answers keeps answering",
            script: REJECTED,
            bound_ms: 5_000,
            expected_verdicts: [LocalVerdict::Rejected, LocalVerdict::Rejected],
        },
        WorkerTestCase {
            description: "a worker killed by a signal, restarted for the next case",
            script: "read l; kill -SEGV $$",
            bound_ms: 5_000,
            expected_verdicts: [failed(LocalFailure::Crash), failed(LocalFailure::Crash)],
        },
        WorkerTestCase {
            description: "a worker that exits as a panicking Rust process does",
            script: "read l; exit 101",
            bound_ms: 5_000,
            expected_verdicts: [failed(LocalFailure::Panic), failed(LocalFailure::Panic)],
        },
        WorkerTestCase {
            description: "a worker that hangs past the bound",
            script: "read l; sleep 30",
            bound_ms: 200,
            expected_verdicts: [failed(LocalFailure::Timeout), failed(LocalFailure::Timeout)],
        },
        WorkerTestCase {
            description: "a worker that prints something other than a verdict",
            script: "while read l; do echo nonsense; done",
            bound_ms: 5_000,
            expected_verdicts: [failed(LocalFailure::Crash), failed(LocalFailure::Crash)],
        },
    ];
    for test_case in test_cases {
        let script: &'static str = test_case.script;
        let mut worker = Worker::new(
            Box::new(move || {
                let mut command = Command::new("sh");
                command.arg("-c").arg(script);
                command
            }),
            Duration::from_millis(test_case.bound_ms),
        );
        let started: Instant = Instant::now();
        let verdicts: [LocalVerdict; 2] =
            [worker.verdict("SELECT 17"), worker.verdict("SELECT 31")];
        assert_eq!(
            verdicts, test_case.expected_verdicts,
            "{}",
            test_case.description
        );
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{}: took {:?}",
            test_case.description,
            started.elapsed()
        );
    }
}
