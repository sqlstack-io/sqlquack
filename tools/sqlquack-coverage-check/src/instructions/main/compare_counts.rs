//! `compare_counts`: measured instruction counts against the committed baseline.

use std::collections::BTreeMap;

use crate::instructions::models::InstructionBaseline;

/// `count`'s change from `expected` in hundredths of a percent, rounded toward zero.
fn change_hundredths(count: u64, expected: u64) -> i128 {
    (i128::from(count) - i128::from(expected)) * 10_000 / i128::from(expected.max(1))
}

/// Hundredths of a percent as a percentage with two decimals, sign dropped.
fn percent(hundredths: i128) -> String {
    let magnitude: u128 = hundredths.unsigned_abs();
    format!("{}.{:02}%", magnitude / 100, magnitude % 100)
}

/// Every count further than the tolerance from its baseline, either way; a gain must be recorded too.
pub fn compare_counts(
    baseline: &InstructionBaseline,
    measured: &BTreeMap<String, u64>,
    arch: &str,
    rustc: &str,
) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    if baseline.arch != arch {
        problems.push(format!(
            "the baseline was recorded on {}, and this is {arch}; counts are not comparable",
            baseline.arch
        ));
        return problems;
    }
    if baseline.rustc != rustc {
        problems.push(format!(
            "the baseline was recorded with {}, and this build uses {rustc}; a new compiler emits different instructions, so record its counts with `make instruction-baseline` in the toolchain change",
            baseline.rustc
        ));
        return problems;
    }
    for name in baseline.counts.keys() {
        if !measured.contains_key(name) {
            problems.push(format!("{name}: in the baseline, but not measured"));
        }
    }
    let tolerance: i128 = i128::from(baseline.tolerance_percent) * 100;
    for (name, &count) in measured {
        let Some(&expected) = baseline.counts.get(name) else {
            problems.push(format!("{name}: measured, but the baseline has no count"));
            continue;
        };
        let change: i128 = change_hundredths(count, expected);
        let limit: u64 = baseline.tolerance_percent;
        if change > tolerance {
            problems.push(format!(
                "{name}: {count} instructions, {} over the baseline {expected} (tolerance {limit}%); a change made this path slower",
                percent(change)
            ));
        } else if change < -tolerance {
            problems.push(format!(
                "{name}: {count} instructions, {} under the baseline {expected} (tolerance {limit}%); record the gain with `make instruction-baseline`",
                percent(change)
            ));
        }
    }
    problems
}
