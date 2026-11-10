//! `nearest_name`: the known name a misspelling most likely meant.

use crate::policy::constants::MAX_SPELLING_DISTANCE;

/// The name among `known` closest to `name`, when a misspelling could explain the difference.
pub fn nearest_name<'a>(name: &str, known: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    known
        .map(|other| (edit_distance(name, other), other))
        .filter(|(distance, _)| *distance <= MAX_SPELLING_DISTANCE)
        .min()
        .map(|(_, other)| other)
}

/// The Levenshtein distance between `left` and `right`, by character.
fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (row, left_char) in left.chars().enumerate() {
        let mut current = vec![row + 1; right.len() + 1];
        for (column, right_char) in right.iter().enumerate() {
            let substitution = previous[column] + usize::from(left_char != *right_char);
            current[column + 1] = substitution
                .min(previous[column + 1] + 1)
                .min(current[column] + 1);
        }
        previous = current;
    }
    previous[right.len()]
}
