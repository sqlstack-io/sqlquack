//! Each form's state from its linked statements, and the tallies by family.

use std::collections::{BTreeMap, BTreeSet};

use crate::results::models::{
    FormResult, FormTally, RecordedStatement, ResultState, StatementOutcome, StatementTally,
};

/// A form's state: listed, then any statement that differs, then any refused, then any match.
pub(crate) fn form_state(listed: bool, outcomes: &[StatementOutcome]) -> ResultState {
    if listed {
        return ResultState::NotTranslatable;
    }
    if outcomes.is_empty() {
        return ResultState::Unmeasured;
    }
    if outcomes.contains(&StatementOutcome::Differs) {
        return ResultState::Differs;
    }
    if outcomes.iter().any(|outcome| outcome.refused()) {
        return ResultState::Refused;
    }
    match outcomes.iter().any(|outcome| outcome.matches()) {
        true => ResultState::Matching,
        false => ResultState::SyntaxOnly,
    }
}

/// One form's result from the statements linked to it.
pub(crate) fn form_result(
    (id, family): (&str, &str),
    listed: bool,
    linked: &[&RecordedStatement],
) -> FormResult {
    let outcomes: Vec<StatementOutcome> =
        linked.iter().map(|statement| statement.outcome).collect();
    let codes: BTreeSet<String> = linked
        .iter()
        .filter(|statement| statement.outcome.refused())
        .flat_map(|statement| statement.codes.iter().cloned())
        .collect();
    FormResult {
        id: id.to_owned(),
        family: family.to_owned(),
        state: form_state(listed, &outcomes),
        statements: linked.len(),
        matching_statements: outcomes.iter().filter(|outcome| outcome.matches()).count(),
        codes: codes.into_iter().collect(),
    }
}

/// The counts of `forms`.
pub(crate) fn tally<'a>(forms: impl Iterator<Item = &'a FormResult>) -> FormTally {
    let mut tally = FormTally::default();
    for form in forms {
        tally.catalogued += 1;
        let count = match form.state {
            ResultState::NotTranslatable => &mut tally.not_translatable,
            ResultState::Matching => &mut tally.matching,
            ResultState::SyntaxOnly => &mut tally.syntax_only,
            ResultState::Refused => &mut tally.refused,
            ResultState::Differs => &mut tally.differs,
            ResultState::Unmeasured => &mut tally.unmeasured,
        };
        *count += 1;
    }
    tally.translatable = tally.catalogued - tally.not_translatable;
    tally
}

/// The whole catalogue's tally and one per family.
pub(crate) fn family_tallies(forms: &[FormResult]) -> (FormTally, BTreeMap<String, FormTally>) {
    let families: BTreeSet<&str> = forms.iter().map(|form| form.family.as_str()).collect();
    let mut by_family: BTreeMap<String, FormTally> = BTreeMap::new();
    for family in families {
        let members = forms.iter().filter(|form| form.family == family);
        by_family.insert(family.to_owned(), tally(members));
    }
    (tally(forms.iter()), by_family)
}

/// The statement counts; `linked` says which statements link to at least one form.
pub(crate) fn statement_tally(
    statements: &[RecordedStatement],
    linked: &BTreeSet<&str>,
) -> StatementTally {
    let mut tally = StatementTally::default();
    for statement in statements {
        let outcome = statement.outcome;
        *tally.by_outcome.entry(outcome).or_default() += 1;
        if !outcome.recorded() {
            continue;
        }
        tally.recorded += 1;
        tally.matching += usize::from(outcome.matches());
        tally.matching_errors += usize::from(outcome == StatementOutcome::MatchingError);
        tally.unlinked += usize::from(!linked.contains(statement.reference.as_str()));
    }
    tally
}
