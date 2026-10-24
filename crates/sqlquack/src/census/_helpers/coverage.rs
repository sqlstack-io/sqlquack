//! Convertible files as constructs get rules, in frequency and greedy order.

use std::cmp::Reverse;

pub(crate) struct CoverageInput<'a> {
    pub(crate) files: &'a [Option<Vec<usize>>],
    pub(crate) construct_count: usize,
    pub(crate) frequency_order: &'a [usize],
}

pub(crate) fn greedy_order(input: &CoverageInput<'_>) -> Vec<usize> {
    let mut frequency_rank = vec![0usize; input.construct_count];
    for (rank, construct) in input.frequency_order.iter().enumerate() {
        frequency_rank[*construct] = rank;
    }
    let mut chosen = vec![false; input.construct_count];
    let mut missing: Vec<usize> = input
        .files
        .iter()
        .map(|file| file.as_ref().map_or(0, Vec::len))
        .collect();
    let mut order: Vec<usize> = Vec::with_capacity(input.construct_count);
    let mut unlocks = vec![0usize; input.construct_count];
    while order.len() < input.construct_count {
        unlocks.iter_mut().for_each(|count| *count = 0);
        for (file, remaining) in input.files.iter().zip(&missing) {
            if let (Some(constructs), 1) = (file, remaining)
                && let Some(last) = constructs.iter().find(|construct| !chosen[**construct])
            {
                unlocks[*last] += 1;
            }
        }
        let Some(next) = (0..input.construct_count)
            .filter(|construct| !chosen[*construct])
            .min_by_key(|construct| (Reverse(unlocks[*construct]), frequency_rank[*construct]))
        else {
            break;
        };
        chosen[next] = true;
        order.push(next);
        for (file, remaining) in input.files.iter().zip(missing.iter_mut()) {
            if file
                .as_ref()
                .is_some_and(|constructs| constructs.contains(&next))
            {
                *remaining -= 1;
            }
        }
    }
    order
}

pub(crate) fn convertible_at(
    files: &[Option<Vec<usize>>],
    order: &[usize],
    construct_count: usize,
    points: &[usize],
) -> Vec<usize> {
    let mut rank = vec![usize::MAX; construct_count];
    for (position, construct) in order.iter().enumerate() {
        rank[*construct] = position;
    }
    let mut needed: Vec<usize> = Vec::with_capacity(files.len());
    for constructs in files.iter().flatten() {
        needed.push(rules_needed(constructs, &rank));
    }
    points
        .iter()
        .map(|point| convertible_within(&needed, *point))
        .collect()
}

fn convertible_within(needed: &[usize], rules: usize) -> usize {
    needed.iter().filter(|need| **need <= rules).count()
}

fn rules_needed(constructs: &[usize], rank: &[usize]) -> usize {
    constructs
        .iter()
        .map(|construct| rank[*construct].saturating_add(1))
        .max()
        .unwrap_or(0)
}
