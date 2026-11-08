//! The gap worklist: differs first, then refusals by forms blocked, then forms needing receipts.

use std::collections::{BTreeMap, BTreeSet};

use crate::results::constants::WORK_EXAMPLES;
use crate::results::models::{
    FormResult, RecordedStatement, ResultState, StatementOutcome, WorkItem,
};

/// Statements and the forms they link to, keyed by a gap.
#[derive(Default)]
struct Gap {
    forms: BTreeSet<String>,
    statements: Vec<String>,
}

/// The refusal key: its codes, then its refused constructs.
fn refusal_key(statement: &RecordedStatement) -> String {
    let mut key: String = statement.codes.join(",");
    if !statement.refused.is_empty() {
        key.push(' ');
        key.push_str(&statement.refused.join(","));
    }
    key
}

fn items(kind: &str, gaps: BTreeMap<String, Gap>) -> Vec<WorkItem> {
    let mut items: Vec<WorkItem> = gaps
        .into_iter()
        .map(|(subject, gap)| WorkItem {
            kind: kind.to_owned(),
            subject,
            forms: gap.forms.len(),
            statements: gap.statements.len(),
            examples: gap.statements.iter().take(WORK_EXAMPLES).cloned().collect(),
        })
        .collect();
    items.sort_by(|left, right| {
        (right.forms, right.statements, &left.subject).cmp(&(
            left.forms,
            left.statements,
            &right.subject,
        ))
    });
    items
}

/// The worklist; `links` holds each statement's forms, and only forms in `states` count.
pub(crate) fn worklist(
    statements: &[RecordedStatement],
    links: &BTreeMap<&str, BTreeSet<String>>,
    forms: &[FormResult],
) -> Vec<WorkItem> {
    let states: BTreeMap<&str, ResultState> = forms
        .iter()
        .map(|form| (form.id.as_str(), form.state))
        .collect();
    let mut differs: BTreeMap<String, Gap> = BTreeMap::new();
    let mut refused: BTreeMap<String, Gap> = BTreeMap::new();
    let mut receipts: BTreeMap<String, Gap> = BTreeMap::new();
    for statement in statements {
        let (gaps, key, state) = match statement.outcome {
            StatementOutcome::Differs => (
                &mut differs,
                statement.reference.clone(),
                ResultState::Differs,
            ),
            outcome if outcome.refused() => {
                (&mut refused, refusal_key(statement), ResultState::Refused)
            }
            StatementOutcome::ShapeOnly | StatementOutcome::UnrecordedTranslates => (
                &mut receipts,
                statement.reference.clone(),
                ResultState::SyntaxOnly,
            ),
            _ => continue,
        };
        let gap = gaps.entry(key).or_default();
        gap.statements.push(statement.reference.clone());
        for form in links
            .get(statement.reference.as_str())
            .into_iter()
            .flatten()
        {
            if states.get(form.as_str()) == Some(&state) {
                gap.forms.insert(form.clone());
            }
        }
    }
    let mut list: Vec<WorkItem> = items("differs", differs);
    list.extend(items("refused", refused));
    list.extend(
        items("needs_receipt", receipts)
            .into_iter()
            .filter(|item| item.forms > 0),
    );
    list
}
