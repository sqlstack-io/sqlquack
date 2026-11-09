use std::collections::{BTreeMap, BTreeSet};

use sqlquack_diagnostics::provenance::models::{Provenance, Shape, ShapeArgument};

use sqlquack_compare::comparison::models::RowOrder;

use crate::cases::models::{CaseInput, CaseRole, InputColumn, LoadedCase};
use crate::coverage::main::check_coverage::check_coverage;
use crate::coverage::models::{
    ArgumentDomain, FamilyHazards, Form, Hazard, HazardCatalogue, RuleDomain,
};
use crate::coverage::tests::test_types::{RecordedGet, RuleGaps, UnrecordableSplit};
use crate::verify::models::{CaseVerdict, Verdict};

const RULE: &str = "rule.computed_get";

fn case(inputs: &[(&str, &[&str])]) -> LoadedCase {
    LoadedCase {
        reference: "semi-structured/get.toml#get".to_owned(),
        description: String::new(),
        category: String::new(),
        construct: String::new(),
        session_profile: String::new(),
        session_parameters: BTreeMap::new(),
        pinned_parameters: BTreeMap::new(),
        source_sql: String::new(),
        order: RowOrder::Ordered,
        hash: String::new(),
        role: CaseRole::Conversion,
        expected_divergence: None,
        values: true,
        compile_error: false,
        expected_error: false,
        functions: Vec::new(),
        overrides: Vec::new(),
        setup_sql: Vec::new(),
        setup: Vec::new(),
        inputs: inputs
            .iter()
            .map(|(name, cells)| CaseInput {
                column: InputColumn {
                    name: (*name).to_owned(),
                    data_type: String::new(),
                },
                cells: cells.iter().map(|cell| (*cell).to_owned()).collect(),
            })
            .collect(),
        row_sources: Vec::new(),
    }
}

fn argument(family: &str, source: &str) -> ShapeArgument {
    ShapeArgument {
        family: family.to_owned(),
        literal: source.starts_with("literal:"),
        source: Some(source.to_owned()),
    }
}

fn verdict(shape: Shape, verdict: Verdict) -> CaseVerdict {
    CaseVerdict {
        reference: "semi-structured/get.toml#get".to_owned(),
        hash: String::new(),
        verdict,
        provenance: Some(Provenance {
            shapes: BTreeMap::from([(RULE.to_owned(), BTreeSet::from([shape]))]),
            ..Provenance::default()
        }),
        error_check: None,
        snowflake_message: None,
    }
}

fn hazard(id: &str, cells: &[&str]) -> Hazard {
    Hazard {
        id: id.to_owned(),
        description: String::new(),
        cells: cells.iter().map(|cell| (*cell).to_owned()).collect(),
        ..Hazard::default()
    }
}

fn catalogue() -> HazardCatalogue {
    HazardCatalogue {
        version: 1,
        families: BTreeMap::from([(
            "text".to_owned(),
            FamilyHazards {
                hazards: vec![
                    hazard("empty", &["''"]),
                    hazard("dollar_prefix", &["'$.a'"]),
                ],
                ..FamilyHazards::default()
            },
        )]),
    }
}

/// Coverage of a GET-like rule declaring a VARIANT and a key of `key_families`, by one case.
pub(crate) fn get_gaps(key_families: &[&str], recorded: &RecordedGet) -> RuleGaps {
    let domain = RuleDomain {
        id: RULE.to_owned(),
        undeclarable: None,
        probes: Vec::new(),
        forms: vec![Form {
            construct: "fn:GET".to_owned(),
            arguments: Some(vec![
                ArgumentDomain {
                    families: vec!["variant".to_owned()],
                    ..ArgumentDomain::default()
                },
                ArgumentDomain {
                    families: key_families
                        .iter()
                        .map(|family| (*family).to_owned())
                        .collect(),
                    kinds: Some(vec!["value".to_owned()]),
                    ..ArgumentDomain::default()
                },
            ]),
            ..Form::default()
        }],
    };
    let shape = Shape {
        construct: "fn:GET".to_owned(),
        arguments: vec![
            argument("variant", "column:V"),
            argument(recorded.key_family, recorded.key_source),
        ],
        facets: BTreeMap::new(),
    };
    let report = check_coverage(
        &[RULE],
        (&[domain], &catalogue()),
        (
            &[case(&[("v", &["NULL"]), ("k", recorded.cells)])],
            &[verdict(shape, recorded.verdict.clone())],
        ),
    );
    let rule = report.rules.into_iter().next().unwrap_or_default();
    RuleGaps {
        shape_gaps: rule.shape_gaps,
        undeclared_shapes: rule.undeclared_shapes,
        hazard_gaps: rule.hazard_gaps,
    }
}

/// Hazard gaps, unrecordable hazards and hazard count of a text key beside unrecordable ones.
pub(crate) fn unrecordable_split() -> UnrecordableSplit {
    let hazards = HazardCatalogue {
        version: 1,
        families: BTreeMap::from([(
            "text".to_owned(),
            FamilyHazards {
                hazards: vec![
                    hazard("empty", &["''"]),
                    hazard("dollar_prefix", &["'$.a'"]),
                    Hazard {
                        unrecordable: Some("No connector converts it.".to_owned()),
                        ..hazard("unconvertible", &["'x'"])
                    },
                ],
                ..FamilyHazards::default()
            },
        )]),
    };
    let domain = RuleDomain {
        id: RULE.to_owned(),
        undeclarable: None,
        probes: Vec::new(),
        forms: vec![Form {
            construct: "fn:GET".to_owned(),
            arguments: Some(vec![
                ArgumentDomain {
                    families: vec!["variant".to_owned()],
                    ..ArgumentDomain::default()
                },
                ArgumentDomain {
                    families: vec!["text".to_owned()],
                    kinds: Some(vec!["value".to_owned()]),
                    ..ArgumentDomain::default()
                },
            ]),
            ..Form::default()
        }],
    };
    let shape = Shape {
        construct: "fn:GET".to_owned(),
        arguments: vec![
            argument("variant", "column:V"),
            argument("text", "column:K"),
        ],
        facets: BTreeMap::new(),
    };
    let report = check_coverage(
        &[RULE],
        (&[domain], &hazards),
        (
            &[case(&[("v", &["NULL", "NULL"]), ("k", &["''", "'$.a'"])])],
            &[verdict(shape, Verdict::Passed)],
        ),
    );
    let rule = report.rules.into_iter().next().unwrap_or_default();
    UnrecordableSplit {
        hazard_gaps: rule.hazard_gaps,
        unrecordable_hazards: rule.unrecordable_hazards,
        hazards: rule.hazards,
    }
}

/// The row-set hazard gaps of an aggregate over `values` grouped by `groups`.
pub(crate) fn group_gaps(groups: &[&str], values: &[&str]) -> Vec<String> {
    let domain = RuleDomain {
        id: RULE.to_owned(),
        undeclarable: None,
        probes: Vec::new(),
        forms: vec![Form {
            construct: "fn:ANY_VALUE".to_owned(),
            arguments: Some(vec![ArgumentDomain {
                families: vec!["boolean".to_owned()],
                ..ArgumentDomain::default()
            }]),
            rows: Some("group".to_owned()),
            ..Form::default()
        }],
    };
    let group_hazard = |id: &str, matcher: &str| Hazard {
        id: id.to_owned(),
        group: Some(matcher.to_owned()),
        ..Hazard::default()
    };
    let catalogue = HazardCatalogue {
        version: 1,
        families: BTreeMap::from([(
            "group".to_owned(),
            FamilyHazards {
                hazards: vec![
                    group_hazard("all_null", "all_null"),
                    group_hazard("mixed", "mixed"),
                ],
                ..FamilyHazards::default()
            },
        )]),
    };
    let shape = Shape {
        construct: "fn:ANY_VALUE".to_owned(),
        arguments: vec![argument("boolean", "column:X")],
        facets: BTreeMap::new(),
    };
    let report = check_coverage(
        &[RULE],
        (&[domain], &catalogue),
        (
            &[case(&[("g", groups), ("x", values)])],
            &[verdict(shape, Verdict::Passed)],
        ),
    );
    report
        .rules
        .into_iter()
        .next()
        .unwrap_or_default()
        .hazard_gaps
}

/// The shapes outside the domain when a GET of an integer key is only a probe of the rule.
pub(crate) fn probed_shapes() -> Vec<String> {
    let get = |family: &str| Form {
        construct: "fn:GET".to_owned(),
        arguments: Some(vec![
            ArgumentDomain {
                families: vec!["variant".to_owned()],
                ..ArgumentDomain::default()
            },
            ArgumentDomain {
                families: vec![family.to_owned()],
                ..ArgumentDomain::default()
            },
        ]),
        ..Form::default()
    };
    let domain = RuleDomain {
        id: RULE.to_owned(),
        undeclarable: None,
        probes: vec![get("integer")],
        forms: vec![get("text")],
    };
    let shape = Shape {
        construct: "fn:GET".to_owned(),
        arguments: vec![
            argument("variant", "column:V"),
            argument("integer", "column:K"),
        ],
        facets: BTreeMap::new(),
    };
    let report = check_coverage(
        &[RULE],
        (&[domain], &catalogue()),
        (
            &[case(&[("v", &["NULL"]), ("k", &["1"])])],
            &[verdict(shape, Verdict::Passed)],
        ),
    );
    report
        .rules
        .into_iter()
        .next()
        .unwrap_or_default()
        .undeclared_shapes
}
