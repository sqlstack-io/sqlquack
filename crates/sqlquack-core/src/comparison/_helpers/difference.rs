//! The first differing path between two normalised trees.

use serde_json::Value;

use crate::comparison::constants::DIFFERENCE_EXCERPT_CHARACTERS;
use crate::comparison::models::TreeDifference;

const ABSENT: &str = "<absent>";

pub(crate) fn first_difference(expected: &Value, actual: &Value) -> Option<TreeDifference> {
    let mut pending: Vec<(String, Option<&Value>, Option<&Value>)> =
        vec![(String::new(), Some(expected), Some(actual))];
    while let Some((path, expected, actual)) = pending.pop() {
        match (expected, actual) {
            (Some(Value::Object(left)), Some(Value::Object(right))) => {
                let mut keys: Vec<&String> = left.keys().chain(right.keys()).collect();
                keys.sort();
                keys.dedup();
                for key in keys.into_iter().rev() {
                    pending.push((format!("{path}.{key}"), left.get(key), right.get(key)));
                }
            }
            (Some(Value::Array(left)), Some(Value::Array(right))) => {
                for index in (0..left.len().max(right.len())).rev() {
                    pending.push((
                        format!("{path}[{index}]"),
                        left.get(index),
                        right.get(index),
                    ));
                }
            }
            (left, right) if left == right => {}
            (left, right) => {
                return Some(TreeDifference {
                    path: if path.is_empty() {
                        ".".to_owned()
                    } else {
                        path
                    },
                    expected: excerpt(left),
                    actual: excerpt(right),
                });
            }
        }
    }
    None
}

fn excerpt(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return ABSENT.to_owned();
    };
    let text = value.to_string();
    if text.chars().count() <= DIFFERENCE_EXCERPT_CHARACTERS {
        return text;
    }
    let mut shortened: String = text.chars().take(DIFFERENCE_EXCERPT_CHARACTERS).collect();
    shortened.push('…');
    shortened
}
