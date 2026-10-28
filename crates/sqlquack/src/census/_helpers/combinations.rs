//! Small sets of root refusals ranked by the refused files fixing all of them would unblock.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use crate::census::constants::{
    COMBINATION_SEED_PAIRS, COMBINATION_SEED_TRIPLES, COMBINATIONS_PER_SIZE, LARGEST_COMBINATION,
    PAIR,
};
use crate::census::models::{BlockedFile, RootCombination, RootKey};

/// Pairs and triples of roots, each size ranked by the complete files they unblock.
pub(crate) fn combinations(files: &[BlockedFile]) -> Vec<RootCombination> {
    let mut exact: BTreeMap<BTreeSet<&RootKey>, [usize; 3]> = BTreeMap::new();
    for file in files {
        let roots: BTreeSet<&RootKey> = file.roots.iter().collect();
        if roots.is_empty() || roots.len() > LARGEST_COMBINATION {
            continue;
        }
        let counts = exact.entry(roots).or_default();
        match (file.possibly_incomplete, file.upstream.is_empty()) {
            (true, _) => counts[2] += 1,
            (false, true) => counts[0] += 1,
            (false, false) => counts[1] += 1,
        }
    }
    let mut singles: Vec<(&RootKey, usize)> = Vec::new();
    for (roots, counts) in &exact {
        if let Some(root) = sole_root(roots) {
            singles.push((root, counts[0] + counts[1]));
        }
    }
    singles.sort_by_key(|(root, files)| (Reverse(*files), (*root).clone()));
    let seeds: Vec<&RootKey> = singles.iter().map(|(root, _)| *root).collect();
    let mut candidates: BTreeSet<BTreeSet<&RootKey>> = BTreeSet::new();
    for roots in exact.keys().filter(|roots| roots.len() > 1) {
        candidates.insert(roots.clone());
        let listed: Vec<&RootKey> = roots.iter().copied().collect();
        for (position, first) in listed.iter().enumerate() {
            for second in &listed[position + 1..] {
                candidates.insert([*first, *second].into_iter().collect());
            }
        }
        if roots.len() == PAIR {
            for seed in seeds.iter().take(COMBINATION_SEED_TRIPLES) {
                let mut extended = roots.clone();
                if extended.insert(*seed) {
                    candidates.insert(extended);
                }
            }
        }
    }
    let pairs = seeds.iter().take(COMBINATION_SEED_PAIRS);
    for (position, first) in pairs.clone().enumerate() {
        for second in pairs.clone().skip(position + 1) {
            candidates.insert([*first, *second].into_iter().collect());
        }
    }
    let mut ranked: Vec<RootCombination> = candidates
        .into_iter()
        .map(|candidate| {
            let mut counts = [0usize; 3];
            for (roots, found) in &exact {
                if roots.is_subset(&candidate) {
                    for (total, count) in counts.iter_mut().zip(found) {
                        *total += count;
                    }
                }
            }
            RootCombination {
                roots: candidate.into_iter().cloned().collect(),
                direct: counts[0],
                downstream: counts[1],
                files: counts[0] + counts[1],
                uncertain: counts[2],
            }
        })
        .collect();
    ranked.sort_by_key(|combination| {
        (
            combination.roots.len(),
            Reverse(combination.files),
            Reverse(combination.direct),
            combination.roots.clone(),
        )
    });
    let mut kept: Vec<RootCombination> = Vec::new();
    for size in PAIR..=LARGEST_COMBINATION {
        kept.extend(
            ranked
                .iter()
                .filter(|combination| combination.roots.len() == size)
                .take(COMBINATIONS_PER_SIZE)
                .cloned(),
        );
    }
    kept
}

/// The one root of a single-root set.
fn sole_root<'a>(roots: &BTreeSet<&'a RootKey>) -> Option<&'a RootKey> {
    match roots.len() {
        1 => roots.first().copied(),
        _ => None,
    }
}
