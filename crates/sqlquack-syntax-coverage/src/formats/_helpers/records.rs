//! Format checks of single records; joins across files belong to the checker.

use std::collections::BTreeSet;

use crate::formats::_helpers::hashes::{
    is_git_tree, is_sha256, session_sha256, setup_sha256, sha256_hex,
};
use crate::formats::_helpers::ids::{
    is_case_id, is_form_id, is_rfc3339_utc, is_segment, is_source_id, is_utc, obligation_form,
};
use crate::formats::constants::{
    DEFINITION_SEPARATOR, FAMILIES, GRAMMAR_JOIN, HTTPS, INTERACTION_MEMBERS, PROSE_HASH_PREFIX,
    RECEIPT_FORBIDDEN_TEXT, RECEIPT_FORMAT,
};
use crate::formats::models::{
    Alias, Case, Definition, Form, FormKind, Index, Obligation, ObligationKind, Receipt, Review,
    Source, SourceStatus,
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
        SourceStatus::Forms | SourceStatus::NoSyntaxBlock => {
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
    if !is_form_id(&alias.from, form_family(&alias.from))
        || !is_form_id(&alias.to, form_family(&alias.to))
        || alias.from == alias.to
    {
        problems.push(format!(
            "alias {} -> {}: not two distinct form ids",
            alias.from, alias.to
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
        _ => !depths && !pair,
    };
    if !shaped {
        problems.push(format!(
            "obligation {id}: nesting_depth needs depths, interaction needs a pair of two, \
             no other kind takes either"
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
    let hashes: [Option<&String>; 9] = [
        Some(&receipt.sql_sha256),
        Some(&receipt.setup_sha256),
        Some(&receipt.session_sha256),
        Some(&receipt.raw_record_sha256),
        Some(&receipt.run_audit_sha256),
        Some(&receipt.packet_sha256),
        receipt.query_id_sha256.as_ref(),
        receipt.case_hash.as_ref(),
        receipt.input_sha256.as_ref(),
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
    let text: [Option<&String>; 8] = [
        Some(&receipt.run_id),
        receipt.query_id.as_ref(),
        Some(&receipt.warehouse_version),
        Some(&receipt.outcome),
        Some(&receipt.phase_basis),
        receipt.sqlstate.as_ref(),
        receipt.after_sql.as_ref(),
        receipt.category.as_ref(),
    ];
    if text.iter().flatten().any(|field| identifying(field)) {
        problems.push(format!(
            "receipt {id}: holds an account, host, URL or address"
        ));
    }
    problems
}

pub(crate) fn check_review(review: &Review) -> Vec<String> {
    let id: &str = &review.id;
    let mut problems: Vec<String> = Vec::new();
    if !is_segment(id) {
        problems.push(format!("review {id}: id is not a lower-case name"));
    }
    if !FAMILIES.contains(&review.family.as_str()) {
        problems.push(format!("review {id}: unknown family {}", review.family));
    }
    if !is_sha256(&review.catalogue_sha256) || !is_sha256(&review.obligations_sha256) {
        problems.push(format!(
            "review {id}: a file hash is not a lower-case SHA-256"
        ));
    }
    if !is_git_tree(&review.parser_tree) {
        problems.push(format!("review {id}: parser_tree is not a Git tree id"));
    }
    if !is_utc(&review.reviewed_utc) {
        problems.push(format!(
            "review {id}: reviewed_utc is not YYYY-MM-DDTHH:MM:SSZ"
        ));
    }
    if review
        .exclusions
        .iter()
        .any(|exclusion| !is_form_id(&exclusion.form, &review.family) || blank(&exclusion.reason))
    {
        problems.push(format!(
            "review {id}: an exclusion is not a form id of its family with a reason"
        ));
    }
    problems
}
