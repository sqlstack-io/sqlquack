//! `sample_cases`: a reproducible sample of recorded cases.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::cases::models::LoadedCase;

/// About `percent`% of the recorded cases, chosen by hashing each case hash with `seed`.
pub fn sample_cases<'a>(
    cases: &'a [LoadedCase],
    results: &Path,
    percent: f64,
    seed: u64,
) -> Vec<&'a LoadedCase> {
    let mut scored: Vec<(f64, &LoadedCase)> = cases
        .iter()
        .filter(|case| results.join(format!("{}.json", case.hash)).exists())
        .map(|case| (score(&case.hash, seed), case))
        .collect();
    scored.sort_by(|left, right| left.0.total_cmp(&right.0));
    let wanted = ((scored.len() as f64) * percent.clamp(0.0, 100.0) / 100.0).ceil() as usize;
    scored
        .into_iter()
        .take(wanted)
        .map(|(_, case)| case)
        .collect()
}

fn score(hash: &str, seed: u64) -> f64 {
    let digest = Sha256::digest(format!("{seed}:{hash}").as_bytes());
    let mut prefix = [0u8; 8];
    prefix.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(prefix) as f64 / u64::MAX as f64
}
