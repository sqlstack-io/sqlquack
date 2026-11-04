//! Format checks of single records; joins across files belong to the checker.

use std::collections::{BTreeMap, BTreeSet};

use crate::formats::_helpers::hashes::{
    is_git_tree, is_sha256, session_sha256, setup_sha256, sha256_hex,
};
use crate::formats::_helpers::ids::{
    is_case_id, is_form_id, is_rfc3339_utc, is_segment, is_source_id, is_utc,
};
use crate::formats::constants::{
    DEFINITION_SEPARATOR, FAMILIES, GRAMMAR_JOIN, HTTPS, INTERACTION_MEMBERS, PROSE_HASH_PREFIX,
    RECEIPT_FORBIDDEN_TEXT, RECEIPT_FORMAT,
};
use crate::formats::main::obligation_form::obligation_form;
use crate::formats::models::{
    Alias, Case, Definition, Diagnostic, Exclusion, Expect, Form, FormKind, FormStatus, Index,
    Obligation, ObligationKind, Phase, Receipt, Review, Source, SourceStatus, Unobservable,
    UnobservableCase, UnobservableProbe,
};

fn blank(text: &str) -> bool {
    text.trim().is_empty()
}

/// Whether `text` holds an account, host, URL or address.
fn identifying(text: &str) -> bool {
    let lower: String = text.to_ascii_lowercase();
    RECEIPT_FORBIDDEN_TEXT
        .iter()
        .any(|forbidden| lower.contains(forbidden))
}

/// The family of a form id, whatever file it appears in.
fn form_family(id: &str) -> &str {
    id.split('.').nth(1).unwrap_or_default()
}

pub(crate) fn check_source(source: &Source) -> Vec<String> {
    let id: &str = &source.id;
    let mut problems: Vec<String> = Vec::new();
    if !is_source_id(id) {
        problems.push(format!("source {id}: not doc.<page-slug>"));
    }
    if !source.url.starts_with(HTTPS) || blank(&source.title) {
        problems.push(format!("source {id}: needs an https url and a title"));
    }
    if !is_utc(&source.retrieved_utc) {
        problems.push(format!(
            "source {id}: retrieved_utc is not YYYY-MM-DDTHH:MM:SSZ"
        ));
    }
    if !is_sha256(&source.page_sha256) || !is_sha256(&source.syntax_sha256) {
        problems.push(format!("source {id}: a hash is not a lower-case SHA-256"));
    }
    let family_valid: bool = match source.status {
        SourceStatus::OutOfScope => source.family.is_empty(),
        SourceStatus::Forms | SourceStatus::Restated | SourceStatus::NoSyntaxBlock => {
            FAMILIES.contains(&source.family.as_str())
        }
    };
    if !family_valid {
        problems.push(format!(
            "source {id}: an out-of-scope page has no family, every other page a known one"
        ));
    }
    if source.decisions.iter().any(|decision| blank(decision)) {
        problems.push(format!("source {id}: blank decision"));
    }
    problems
}

pub(crate) fn check_index(index: &Index) -> Vec<String> {
    match blank(&index.name) || !is_sha256(&index.sha256) {
        true => vec![format!("index {}: needs a name and a SHA-256", index.name)],
        false => Vec::new(),
    }
}

pub(crate) fn check_alias(alias: &Alias) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    let distinct: BTreeSet<&String> = alias.to.iter().collect();
    if !is_form_id(&alias.from, form_family(&alias.from))
        || alias.to.is_empty()
        || distinct.len() != alias.to.len()
        || alias
            .to
            .iter()
            .any(|to| to == &alias.from || !is_form_id(to, form_family(to)))
    {
        problems.push(format!(
            "alias {}: needs distinct form ids to rename or split into, none of them itself",
            alias.from
        ));
    }
    if blank(&alias.reason) {
        problems.push(format!("alias {}: no reason", alias.from));
    }
    problems
}

pub(crate) fn check_definition(definition: &Definition, sources: &BTreeSet<&str>) -> Vec<String> {
    let id: &str = &definition.id;
    let mut problems: Vec<String> = Vec::new();
    if id
        != format!(
            "{}{DEFINITION_SEPARATOR}{}",
            definition.source, definition.name
        )
        || blank(&definition.name)
        || blank(&definition.grammar)
    {
        problems.push(format!(
            "definition {id}: not <source>::<name> with a grammar"
        ));
    }
    if !sources.contains(definition.source.as_str()) {
        problems.push(format!(
            "definition {id}: unknown source {}",
            definition.source
        ));
    }
    problems
}

/// A form's fields; `sources` are the pinned source ids, `definitions` the file's.
pub(crate) fn check_form(
    form: &Form,
    family: &str,
    sources: &BTreeSet<&str>,
    definitions: &BTreeSet<(&str, &str)>,
) -> Vec<String> {
    let id: &str = &form.id;
    let mut problems: Vec<String> = Vec::new();
    if !is_form_id(id, family) || form.family != family {
        problems.push(format!(
            "form {id}: not sf.{family}.<page-slug>[.<variant>] of family {family}"
        ));
    }
    if !sources.contains(form.source.as_str()) {
        problems.push(format!("form {id}: unknown source {}", form.source));
    }
    if !form.also_sources.is_sorted_by(|left, right| left < right)
        || form.also_sources.contains(&form.source)
    {
        problems.push(format!(
            "form {id}: also_sources must be sorted, distinct and without its own source"
        ));
    }
    for also in &form.also_sources {
        if !sources.contains(also.as_str()) {
            problems.push(format!("form {id}: unknown also_source {also}"));
        }
    }
    if blank(&form.anchor) {
        problems.push(format!("form {id}: no anchor"));
    }
    let prose: bool = form.kind == FormKind::Prose;
    let hash_valid: bool = match prose {
        true => {
            form.grammar.is_empty()
                && form
                    .grammar_sha256
                    .strip_prefix(PROSE_HASH_PREFIX)
                    .is_some_and(|heading| !blank(heading))
        }
        false => {
            !form.grammar.is_empty()
                && !form.grammar.iter().any(|production| blank(production))
                && form.grammar_sha256 == sha256_hex(form.grammar.join(GRAMMAR_JOIN).as_bytes())
        }
    };
    if !hash_valid {
        problems.push(format!(
            "form {id}: grammar_sha256 does not match its grammar, or a prose form has a grammar"
        ));
    }
    for name in &form.uses {
        if !definitions.contains(&(form.source.as_str(), name.as_str())) {
            problems.push(format!(
                "form {id}: uses {name}, which its page does not define"
            ));
        }
    }
    problems
}

pub(crate) fn check_obligation(obligation: &Obligation, family: &str) -> Vec<String> {
    let id: &str = &obligation.id;
    let mut problems: Vec<String> = Vec::new();
    if obligation_form(id, family).is_none() {
        problems.push(format!("obligation {id}: not sf.{family}.<form>.o<NNN>"));
    }
    let depths: bool = !obligation.depths.is_empty();
    let pair: bool = !obligation.pair.is_empty();
    let shaped: bool = match obligation.kind {
        ObligationKind::NestingDepth => depths && !pair,
        ObligationKind::Interaction => {
            obligation.pair.len() == INTERACTION_MEMBERS
                && obligation.pair[0] != obligation.pair[1]
                && !depths
        }
        ObligationKind::InteractionAllOptions | ObligationKind::Extreme => {
            !depths && !pair && obligation.target.is_none()
        }
        _ => !depths && !pair,
    };
    if !shaped {
        problems.push(format!(
            "obligation {id}: nesting_depth needs depths, interaction needs a pair of two, \
             no other kind takes either, and interaction_all_options takes no target, nor does extreme"
        ));
    }
    let targeted: bool = matches!(
        obligation.kind,
        ObligationKind::Alternative
            | ObligationKind::OptionPresent
            | ObligationKind::OptionAbsent
            | ObligationKind::RequiredOperandMissing
            | ObligationKind::OperandTypeClass
            | ObligationKind::BoundaryBelow
            | ObligationKind::BoundaryAt
            | ObligationKind::BoundaryAbove
    );
    if obligation.kind == ObligationKind::Interaction
        && obligation.anchor.as_deref().is_none_or(blank)
    {
        problems.push(format!(
            "obligation {id}: interaction needs the anchor of the usage note constraining it"
        ));
    }
    if obligation.kind == ObligationKind::Extreme
        && (!obligation.required || obligation.note.as_deref().is_none_or(blank))
    {
        problems.push(format!(
            "obligation {id}: extreme is required and needs a note"
        ));
    }
    if targeted && obligation.target.as_deref().is_none_or(blank) {
        problems.push(format!(
            "obligation {id}: {:?} needs a target",
            obligation.kind
        ));
    }
    problems
}

pub(crate) fn check_case(case: &Case, family: &str) -> Vec<String> {
    let id: &str = &case.case_id;
    let mut problems: Vec<String> = Vec::new();
    if !is_case_id(id) {
        problems.push(format!(
            "case {id}: id is empty, too long or holds whitespace"
        ));
    }
    if case.obligations.is_empty()
        || case
            .obligations
            .iter()
            .any(|obligation| obligation_form(obligation, family).is_none())
    {
        problems.push(format!(
            "case {id}: needs obligation ids of family {family}"
        ));
    }
    if case.stage.is_some() && case.expect != Some(Expect::Accept) {
        problems.push(format!(
            "case {id}: a stage is only for a case expected to be accepted"
        ));
    }
    if case.sql_sha256 != sha256_hex(case.sql.as_bytes())
        || case.setup_sha256 != setup_sha256(&case.setup)
        || case.session_sha256 != session_sha256(&case.session)
    {
        problems.push(format!(
            "case {id}: a hash does not match its SQL, setup or session"
        ));
    }
    problems
}

pub(crate) fn check_receipt(receipt: &Receipt) -> Vec<String> {
    let id: &str = &receipt.case_id;
    let mut problems: Vec<String> = Vec::new();
    if receipt.format != RECEIPT_FORMAT {
        problems.push(format!(
            "receipt {id}: format {} is not {RECEIPT_FORMAT}",
            receipt.format
        ));
    }
    if !is_case_id(id) {
        problems.push(format!(
            "receipt {id}: id is empty, too long or holds whitespace"
        ));
    }
    let hashes: [Option<&String>; 10] = [
        Some(&receipt.sql_sha256),
        Some(&receipt.setup_sha256),
        Some(&receipt.session_sha256),
        Some(&receipt.raw_record_sha256),
        Some(&receipt.run_audit_sha256),
        Some(&receipt.packet_sha256),
        receipt.query_id_sha256.as_ref(),
        receipt.case_hash.as_ref(),
        receipt.input_sha256.as_ref(),
        receipt.supersedes_receipt_sha256.as_ref(),
    ];
    if !hashes.iter().flatten().all(|hash| is_sha256(hash)) {
        problems.push(format!("receipt {id}: a hash is not a lower-case SHA-256"));
    }
    if receipt.session_sha256 != session_sha256(&receipt.session_parameters) {
        problems.push(format!(
            "receipt {id}: session_sha256 does not match its parameters"
        ));
    }
    if !is_rfc3339_utc(&receipt.recorded_at) {
        problems.push(format!("receipt {id}: recorded_at is not RFC 3339 in UTC"));
    }
    if blank(&receipt.run_id)
        || blank(&receipt.warehouse_version)
        || blank(&receipt.outcome)
        || blank(&receipt.phase_basis)
    {
        problems.push(format!(
            "receipt {id}: run, version, outcome and phase basis are required"
        ));
    }
    let categorized: bool = match (&receipt.reason_category, &receipt.reason_category_basis) {
        (None, None) => true,
        (Some(_), Some(basis)) => {
            !blank(basis) && !matches!(receipt.phase, Phase::Accepted | Phase::SyntaxError)
        }
        _ => false,
    };
    if !categorized {
        problems.push(format!(
            "receipt {id}: a reason category needs its basis and a phase after parsing"
        ));
    }
    let text: [Option<&String>; 10] = [
        Some(&receipt.run_id),
        receipt.query_id.as_ref(),
        Some(&receipt.warehouse_version),
        Some(&receipt.outcome),
        Some(&receipt.phase_basis),
        receipt.sqlstate.as_ref(),
        receipt.after_sql.as_ref(),
        receipt.category.as_ref(),
        receipt.reason_category_basis.as_ref(),
        receipt.measure.as_ref(),
    ];
    if text.iter().flatten().any(|field| identifying(field)) {
        problems.push(format!(
            "receipt {id}: holds an account, host, URL or address"
        ));
    }
    problems
}

/// The exclusion problem, if any: exactly one target, a reason, and a target the review may name.
fn exclusion_problem(exclusion: &Exclusion, family: &str) -> Option<&'static str> {
    let target_valid: bool = match (&exclusion.form, &exclusion.source) {
        (Some(form), None) => !family.is_empty() && is_form_id(form, family),
        (None, Some(source)) => family.is_empty() && is_source_id(source),
        _ => false,
    };
    match (target_valid, blank(&exclusion.reason)) {
        (true, false) => None,
        (false, _) => Some(
            "an exclusion needs exactly one target: a form id of its family, or a source id in a \
                 page review",
        ),
        (true, true) => Some("an exclusion needs a reason"),
    }
}

pub(crate) fn check_review(review: &Review) -> Vec<String> {
    let id: &str = &review.id;
    let mut problems: Vec<String> = Vec::new();
    if !is_segment(id) {
        problems.push(format!("review {id}: id is not a lower-case name"));
    }
    let hash = |value: &Option<String>| value.as_deref().is_some_and(is_sha256);
    let bound: bool = match review.family.as_str() {
        "" => {
            hash(&review.sources_sha256)
                && review.catalogue_sha256.is_none()
                && review.obligations_sha256.is_none()
        }
        family => {
            FAMILIES.contains(&family)
                && hash(&review.catalogue_sha256)
                && hash(&review.obligations_sha256)
                && review.sources_sha256.is_none()
        }
    };
    if !bound {
        problems.push(format!(
            "review {id}: a family review binds its catalogue and obligation hashes, a page review \
             (empty family) binds only sources_sha256"
        ));
    }
    let tree_valid: bool = match (&review.parser_tree, review.family.as_str()) {
        (Some(tree), _) => is_git_tree(tree),
        (None, family) => family.is_empty(),
    };
    if !tree_valid {
        problems.push(format!(
            "review {id}: parser_tree is not a Git tree id; only a page review may omit it"
        ));
    }
    if let Some(forms) = &review.forms {
        if forms.is_empty() || review.family.is_empty() {
            problems.push(format!(
                "review {id}: forms, when present, lists at least one form of a family review"
            ));
        }
        problems.extend(
            forms
                .iter()
                .filter(|form| !is_form_id(form, &review.family))
                .map(|form| {
                    format!(
                        "review {id}: form {form} is not of family {}",
                        review.family
                    )
                }),
        );
    }
    let listed: Vec<&String> = review.forms.iter().flatten().collect();
    problems.extend(
        review
            .fingerprints
            .iter()
            .filter(|(form, fingerprint)| !listed.contains(form) || !is_sha256(fingerprint))
            .map(|(form, _)| {
                format!("review {id}: fingerprint of {form} is not a SHA-256 of a form it lists")
            }),
    );
    if !review.fingerprints.is_empty()
        && listed
            .iter()
            .any(|form| !review.fingerprints.contains_key(*form))
    {
        problems.push(format!(
            "review {id}: fingerprints, when present, give one for every form it lists"
        ));
    }
    if !is_utc(&review.reviewed_utc) {
        problems.push(format!(
            "review {id}: reviewed_utc is not YYYY-MM-DDTHH:MM:SSZ"
        ));
    }
    problems.extend(
        review
            .exclusions
            .iter()
            .filter_map(|exclusion| exclusion_problem(exclusion, &review.family))
            .map(|problem| format!("review {id}: {problem}")),
    );
    problems
}

pub(crate) fn check_unobservable(entry: &Unobservable, family: &str) -> Vec<String> {
    check_record(
        &UnobservableRecord {
            obligation: &entry.obligation,
            form: &entry.form,
            construct: &entry.construct,
            reason: &entry.reason,
            cases: &entry.cases,
            runs: &entry.runs,
            probes: &entry.probes,
        },
        family,
        "unobservable",
    )
}

pub(crate) fn check_diagnostic(entry: &Diagnostic, family: &str) -> Vec<String> {
    check_record(
        &UnobservableRecord {
            obligation: &entry.obligation,
            form: &entry.form,
            construct: &entry.construct,
            reason: &entry.reason,
            cases: &entry.captured,
            runs: &entry.runs,
            probes: &entry.probes,
        },
        family,
        "diagnostic",
    )
}

/// The fields an unobservable entry and a diagnostic share.
struct UnobservableRecord<'a> {
    obligation: &'a str,
    form: &'a str,
    construct: &'a str,
    reason: &'a str,
    cases: &'a [UnobservableCase],
    runs: &'a [String],
    probes: &'a [UnobservableProbe],
}

fn check_record(entry: &UnobservableRecord, family: &str, table: &str) -> Vec<String> {
    let id: &str = entry.obligation;
    let mut problems: Vec<String> = Vec::new();
    if obligation_form(id, family) != Some(entry.form) {
        problems.push(format!(
            "{table} {id}: not an obligation of form {} in family {family}",
            entry.form
        ));
    }
    if blank(entry.construct) || blank(entry.reason) || entry.runs.iter().any(|run| blank(run)) {
        problems.push(format!(
            "{table} {id}: needs a construct, a reason and run ids"
        ));
    }
    if entry.cases.is_empty() || entry.runs.is_empty() {
        problems.push(format!("{table} {id}: needs at least one case and one run"));
    }
    let hashes = entry
        .cases
        .iter()
        .map(|case| &case.sql_sha256)
        .chain(entry.probes.iter().map(|probe| &probe.sql_sha256));
    if hashes.clone().any(|hash| !is_sha256(hash)) {
        problems.push(format!("{table} {id}: a sql_sha256 is not a SHA-256"));
    }
    problems
}

/// Restated pages against the forms: each is listed in an `also_sources`, none has a live form.
pub(crate) fn check_restatements<'a>(
    sources: &[Source],
    forms: impl Iterator<Item = &'a Form>,
) -> Vec<String> {
    let status: BTreeMap<&str, SourceStatus> = sources
        .iter()
        .map(|source| (source.id.as_str(), source.status))
        .collect();
    let mut problems: Vec<String> = Vec::new();
    let mut cited: BTreeSet<&str> = BTreeSet::new();
    for form in forms {
        if form.status == FormStatus::Catalogued
            && status.get(form.source.as_str()) == Some(&SourceStatus::Restated)
        {
            problems.push(format!(
                "form {}: its source {} is restated, so it has no form of its own",
                form.id, form.source
            ));
        }
        for also in &form.also_sources {
            cited.insert(also.as_str());
            if matches!(
                status.get(also.as_str()),
                Some(SourceStatus::NoSyntaxBlock | SourceStatus::OutOfScope)
            ) {
                problems.push(format!(
                    "form {}: also_source {also} has no in-scope syntax to restate",
                    form.id
                ));
            }
        }
    }
    problems.extend(
        sources
            .iter()
            .filter(|source| {
                source.status == SourceStatus::Restated && !cited.contains(source.id.as_str())
            })
            .map(|source| {
                format!(
                    "source {}: restated, but no form lists it in also_sources",
                    source.id
                )
            }),
    );
    problems
}
