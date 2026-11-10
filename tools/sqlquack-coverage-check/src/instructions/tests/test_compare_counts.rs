use std::collections::BTreeMap;

use crate::instructions::main::compare_counts::compare_counts;
use crate::instructions::models::InstructionBaseline;
use crate::instructions::tests::test_types::CompareTestCase;

#[test]
fn given_measured_counts_when_comparing_with_the_baseline_then_moves_past_the_tolerance_fail() {
    let baseline = InstructionBaseline {
        arch: "x86_64".to_owned(),
        rustc: "rustc 1.99.1 (58cf8f9c3 2026-10-13)".to_owned(),
        tolerance_percent: 2,
        counts: BTreeMap::from([("parse".to_owned(), 1_000), ("analysis".to_owned(), 2_000)]),
    };
    let test_cases = [
        CompareTestCase {
            description: "both at the edge of the tolerance, one up and one down",
            arch: "x86_64",
            rustc: "rustc 1.99.1 (58cf8f9c3 2026-10-13)",
            measured: vec![("parse", 1_020), ("analysis", 1_960)],
            expected_problems: vec![],
        },
        CompareTestCase {
            description: "one instruction past the tolerance over",
            arch: "x86_64",
            rustc: "rustc 1.99.1 (58cf8f9c3 2026-10-13)",
            measured: vec![("parse", 1_021), ("analysis", 2_000)],
            expected_problems: vec!["parse: 1021 instructions, 2.10% over"],
        },
        CompareTestCase {
            description: "a gain past the tolerance asks for a new baseline",
            arch: "x86_64",
            rustc: "rustc 1.99.1 (58cf8f9c3 2026-10-13)",
            measured: vec![("parse", 1_000), ("analysis", 1_959)],
            expected_problems: vec!["analysis: 1959 instructions, 2.05% under"],
        },
        CompareTestCase {
            description: "a workload the baseline lacks, and one not measured",
            arch: "x86_64",
            rustc: "rustc 1.99.1 (58cf8f9c3 2026-10-13)",
            measured: vec![("parse", 1_000), ("format", 5)],
            expected_problems: vec!["analysis: in the baseline", "format: measured"],
        },
        CompareTestCase {
            description: "another architecture",
            arch: "aarch64",
            rustc: "rustc 1.99.1 (58cf8f9c3 2026-10-13)",
            measured: vec![("parse", 1_000), ("analysis", 2_000)],
            expected_problems: vec!["recorded on x86_64, and this is aarch64"],
        },
        CompareTestCase {
            description: "another compiler, even with counts inside the tolerance",
            arch: "x86_64",
            rustc: "rustc 1.100.0 (0a1b2c3d4 2026-11-24)",
            measured: vec![("parse", 1_000), ("analysis", 2_000)],
            expected_problems: vec![
                "recorded with rustc 1.99.1 (58cf8f9c3 2026-10-13), and this build uses rustc 1.100.0 (0a1b2c3d4 2026-11-24); a new compiler emits different instructions, so record its counts with `make instruction-baseline`",
            ],
        },
    ];
    for test_case in test_cases {
        let measured: BTreeMap<String, u64> = test_case
            .measured
            .iter()
            .map(|(name, count)| ((*name).to_owned(), *count))
            .collect();
        let problems: Vec<String> =
            compare_counts(&baseline, &measured, test_case.arch, test_case.rustc);
        assert_eq!(
            problems.len(),
            test_case.expected_problems.len(),
            "{}: {problems:?}",
            test_case.description
        );
        for (problem, expected) in problems.iter().zip(&test_case.expected_problems) {
            assert!(
                problem.contains(expected),
                "{}: {problem:?} lacks {expected:?}",
                test_case.description
            );
        }
    }
}
