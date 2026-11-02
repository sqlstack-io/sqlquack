use std::collections::BTreeMap;
use std::path::Path;

use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_policy::policy::constants::POLICY_FILE_NAME;
use sqlquack_policy::policy::main::load_policy::load_policy;
use sqlquack_rules::conversion::models::{AllowedDivergence, ConversionOptions};
use sqlquack_semantics::types::models::{ColumnSpec, Schema};
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::analysis::main::conversion_options::conversion_options;
use crate::census::_helpers::coverage::{CoverageInput, greedy_order};
use crate::census::_helpers::timed::run_census_timed;
use crate::census::main::render_census_text::render_census_text;
use crate::census::models::CensusProgress;
use crate::census::models::{CensusOptions, CensusReport};
use crate::census::tests::test_types::{
    BlockerSummary, CensusSummary, PlanSummary, ShopFile, UpstreamSummary,
};

pub(crate) const SHOP: &[ShopFile] = &[
    ShopFile {
        path: "marts/customer_totals.sql",
        contents: "SELECT customer_id, SUM(amount) AS total\nFROM __ref(\"orders\")\n\
                   GROUP BY customer_id",
    },
    ShopFile {
        path: "staging/customers.sql",
        contents: "SELECT customer_id, UPPER(name) AS name FROM __source(\"shop\", \"customers\")",
    },
    ShopFile {
        path: "staging/latest_orders.sql",
        contents: "SELECT * FROM __ref(\"orders\")\n\
                   QUALIFY ROW_NUMBER() OVER (PARTITION BY order_id ORDER BY updated_at DESC) = 1",
    },
    ShopFile {
        path: "staging/broken.sql",
        contents: "SELECT FROM WHERE",
    },
    ShopFile {
        path: "staging/notes.txt",
        contents: "not SQL",
    },
];

/// A model holding a FLOAT in a VARIANT, one model reading it back and one not.
pub(crate) const HELD_CHAIN: &[ShopFile] = &[
    ShopFile {
        path: "order_amounts.sql",
        contents: "SELECT ARRAY_CONSTRUCT(CAST(1.5 AS FLOAT), 2) AS amounts, customer_id \
                   FROM __source(\"shop\", \"customers\")",
    },
    ShopFile {
        path: "order_report.sql",
        contents: "SELECT amounts[0]::FLOAT AS first_amount FROM __ref(\"order_amounts\")",
    },
    ShopFile {
        path: "order_customers.sql",
        contents: "SELECT customer_id FROM __ref(\"order_amounts\")",
    },
];

/// A model putting a DATE in a VARIANT, one comparing that column by its text, one reading another.
pub(crate) const TYPED_CHAIN: &[ShopFile] = &[
    ShopFile {
        path: "order_dates.sql",
        contents: "SELECT OBJECT_CONSTRUCT('placed', CAST('2024-01-01' AS DATE)) AS placed, \
                   customer_id FROM __source(\"shop\", \"customers\")",
    },
    ShopFile {
        path: "order_matches.sql",
        contents: "SELECT placed = PARSE_JSON('{\"placed\":\"2024-01-01\"}') AS same \
                   FROM __ref(\"order_dates\")",
    },
    ShopFile {
        path: "order_customers.sql",
        contents: "SELECT customer_id FROM __ref(\"order_dates\")",
    },
];

/// A model putting a NUMBER with a scale in a VARIANT, a reader typing it, one reading text.
pub(crate) const DECIMAL_CHAIN: &[ShopFile] = &[
    ShopFile {
        path: "order_scores.sql",
        contents: "SELECT OBJECT_CONSTRUCT('score', CAST(100 AS NUMBER(5,2))) AS scored, \
                   customer_id FROM __source(\"shop\", \"customers\")",
    },
    ShopFile {
        path: "order_score_types.sql",
        contents: "SELECT TYPEOF(scored:score) AS score_type FROM __ref(\"order_scores\")",
    },
    ShopFile {
        path: "order_score_texts.sql",
        contents: "SELECT scored:score::VARCHAR AS score_text FROM __ref(\"order_scores\")",
    },
];

/// A model passing a source VARIANT on, one comparing it by text downstream, one reading neither.
pub(crate) const SOURCE_VARIANT_CHAIN: &[ShopFile] = &[
    ShopFile {
        path: "order_events.sql",
        contents: "SELECT payload FROM __source(\"shop\", \"events\")",
    },
    ShopFile {
        path: "order_kinds.sql",
        contents: "SELECT payload:kind = PARSE_JSON('\"order\"') AS is_order \
                   FROM __ref(\"order_events\")",
    },
    ShopFile {
        path: "order_customers.sql",
        contents: "SELECT customer_id FROM __source(\"shop\", \"customers\")",
    },
];

/// Roots `a` and `e`; `b`, `c` read `a`; `d` reads `c`, `e`; `f` reads an unknown relation.
pub(crate) const CHAIN: &[ShopFile] = &[
    ShopFile {
        path: "a.sql",
        contents: "SELECT STRTOK(name, 'gift') AS score, customer_id \
                   FROM __source(\"shop\", \"customers\")",
    },
    ShopFile {
        path: "b.sql",
        contents: "SELECT score FROM __ref(\"a\")",
    },
    ShopFile {
        path: "c.sql",
        contents: "SELECT customer_id, score FROM __ref(\"a\")",
    },
    ShopFile {
        path: "d.sql",
        contents: "SELECT c.customer_id, e.label FROM __ref(\"c\") AS c \
                   JOIN __ref(\"e\") AS e ON c.customer_id = e.customer_id",
    },
    ShopFile {
        path: "e.sql",
        contents: "SELECT customer_id, SOUNDEX(name) AS label FROM __source(\"shop\", \"customers\")",
    },
    ShopFile {
        path: "f.sql",
        contents: "SELECT * FROM __source(\"shop\", \"returns\")",
    },
    ShopFile {
        path: "g.sql",
        contents: "SELECT customer_id, name FROM __source(\"shop\", \"customers\")",
    },
];

pub(crate) const PARTIAL_SCAN: &[ShopFile] = &[
    ShopFile {
        path: "name_report.sql",
        contents: "SELECT customer_id, sound FROM __ref(\"sounds\")",
    },
    ShopFile {
        path: "name_summary.sql",
        contents: "SELECT COUNT(*) AS customers FROM __ref(\"name_report\")",
    },
    ShopFile {
        path: "customer_names.sql",
        contents: "SELECT customer_id, name FROM __source(\"shop\", \"customers\")",
    },
];

pub(crate) const ASSUMING: &[ShopFile] = &[
    ShopFile {
        path: "placed_orders.sql",
        contents: "SELECT customer_id, CAST(CAST('2024-03-10 22:30:00' AS TIMESTAMP_NTZ(6)) \
                   AS TIMESTAMP_TZ) AS placed_at FROM __source(\"shop\", \"customers\")",
    },
    ShopFile {
        path: "placed_report.sql",
        contents: "SELECT customer_id FROM __ref(\"placed_orders\")",
    },
    ShopFile {
        path: "customer_ids.sql",
        contents: "SELECT customer_id FROM __source(\"shop\", \"customers\")",
    },
];

pub(crate) const PLACED: &[ShopFile] = &[
    ShopFile {
        path: "a.sql",
        contents: "SELECT customer_id\nFROM __source(\"shop\", \"customers\") WHERE SOUNDEX(name) = 'A'",
    },
    ShopFile {
        path: "b.sql",
        contents: "SELECT customer_id FROM __source(\"shop\", \"customers\")",
    },
];

pub(crate) const MANY_PLACED: &[ShopFile] = &[ShopFile {
    path: "a.sql",
    contents: "SELECT SOUNDEX(name) AS n FROM __source(\"shop\", \"customers\");\nSELECT SOUNDEX(name) AS n FROM __source(\"shop\", \"customers\");\nSELECT SOUNDEX(name) AS n FROM __source(\"shop\", \"customers\");\nSELECT SOUNDEX(name) AS n FROM __source(\"shop\", \"customers\");\nSELECT SOUNDEX(name) AS n FROM __source(\"shop\", \"customers\");\nSELECT SOUNDEX(name) AS n FROM __source(\"shop\", \"customers\");\nSELECT SOUNDEX(name) AS n FROM __source(\"shop\", \"customers\");\n",
}];

pub(crate) const UNRESOLVED_COLUMN_AND_RELATION: &[ShopFile] = &[ShopFile {
    path: "a.sql",
    contents: "SELECT c.nickname, r.amount FROM __source(\"shop\", \"customers\") AS c \
               JOIN __source(\"shop\", \"returns\") AS r ON c.customer_id = r.customer_id; \
               SELECT nickname FROM __source(\"shop\", \"customers\")",
}];

pub(crate) const CURSOR_BOUND: &[ShopFile] = &[ShopFile {
    path: "a.sql",
    contents: "SELECT customer_id FROM __source(\"shop\", \"customers\") \
               WHERE customer_id >= __cursor_start()",
}];

pub(crate) const UNSCOPED_NAMES: &[ShopFile] = &[
    ShopFile {
        path: "a.sql",
        contents: "SELECT UPPER(name) AS label, COUNT(*) AS n \
                   FROM __source(\"shop\", \"customers\") GROUP BY label",
    },
    ShopFile {
        path: "b.sql",
        contents: "SELECT customer_id, (SELECT r FROM (SELECT q * 2 AS r \
                   FROM (SELECT customer_id + 1 AS q))) AS doubled \
                   FROM __source(\"shop\", \"customers\")",
    },
    ShopFile {
        path: "c.sql",
        contents: "SELECT part.seq AS position FROM __source(\"shop\", \"customers\") AS c, \
                   LATERAL SPLIT_TO_TABLE(c.name, ' ') AS part",
    },
];

pub(crate) const QUOTED_LOWER_CASE: &[ShopFile] = &[
    ShopFile {
        path: "a.sql",
        contents: "SELECT SOUNDEX(name) AS label FROM __source(\"shop\", \"customers\")",
    },
    ShopFile {
        path: "b.sql",
        contents: "SELECT label FROM \"__sqlquack_relation__a\"",
    },
];

pub(crate) const CYCLE: &[ShopFile] = &[
    ShopFile {
        path: "a.sql",
        contents: "SELECT customer_id FROM __ref(\"b\")",
    },
    ShopFile {
        path: "b.sql",
        contents: "SELECT customer_id FROM __ref(\"a\")",
    },
    ShopFile {
        path: "c.sql",
        contents: "SELECT customer_id FROM __ref(\"a\")",
    },
];

pub(crate) const TWO_BLOCKERS: &[ShopFile] = &[
    ShopFile {
        path: "a.sql",
        contents: "SELECT HASH(name) AS h, SOUNDEX(name) AS label, customer_id \
                   FROM __source(\"shop\", \"customers\")",
    },
    ShopFile {
        path: "b.sql",
        contents: "SELECT label FROM __ref(\"a\")",
    },
];

/// A refused model, a reader of it, a reader of that reader, and a strict pair beside them.
pub(crate) const UPSTREAM_CHAIN: &[ShopFile] = &[
    ShopFile {
        path: "fuzzy_names.sql",
        contents: "SELECT customer_id, SOUNDEX(name) AS sound FROM __source(\"shop\", \"customers\")",
    },
    ShopFile {
        path: "name_report.sql",
        contents: "SELECT customer_id, UPPER(sound) AS sound FROM __ref(\"fuzzy_names\")",
    },
    ShopFile {
        path: "name_summary.sql",
        contents: "SELECT COUNT(*) AS customers FROM __ref(\"name_report\")",
    },
    ShopFile {
        path: "customer_names.sql",
        contents: "SELECT customer_id, name FROM __source(\"shop\", \"customers\")",
    },
    ShopFile {
        path: "customer_report.sql",
        contents: "SELECT COUNT(*) AS customers FROM __ref(\"customer_names\")",
    },
];

pub(crate) const NOT_SQL: &[ShopFile] = &[ShopFile {
    path: "notes.txt",
    contents: "not SQL",
}];

fn write_files(root: &Path, files: &[ShopFile]) {
    for file in files {
        let path = root.join(file.path);
        std::fs::create_dir_all(path.parent().expect("fixture files have a parent"))
            .expect("fixture directory is created");
        std::fs::write(path, file.contents).expect("fixture file is written");
    }
}

fn files_using(report: &CensusReport, key: &str) -> usize {
    report
        .constructs
        .iter()
        .find(|count| count.construct == key)
        .map_or(0, |count| count.files)
}

fn summarise(report: &CensusReport) -> CensusSummary {
    let text = render_census_text(report, 5);
    CensusSummary {
        files: report.totals.files,
        parsed: report.totals.parsed,
        unparsed: report.totals.unparsed,
        missing_rule: report.refusals.missing_rule,
        unresolved_name: report.refusals.unresolved_name,
        parse_fidelity: report.refusals.parse_fidelity,
        convertible: report.refusals.convertible,
        most_used_files: report.constructs.first().map_or(0, |count| count.files),
        select_files: files_using(report, "node:select"),
        qualify_files: files_using(report, "clause:qualify"),
        all_rules_convertible: report
            .coverage
            .last()
            .map_or(0, |point| point.frequency_order.min(point.greedy_order)),
        text_mentions_sql: text.contains("customers") || text.contains("orders\""),
    }
}

pub(crate) fn census_summary(files: &[ShopFile]) -> Result<CensusSummary, String> {
    let directory = tempfile::tempdir().expect("temporary directory");
    write_files(directory.path(), files);
    run_census_timed(
        &[directory.path().to_path_buf()],
        &CensusOptions {
            dialect: SourceDialect::Snowflake,
            coverage_points: vec![1, 5],
            schema: Schema::default(),
            chain_models: false,
            conversion: ConversionOptions::default(),
            names: false,
            assume_supported: Vec::new(),
            cursor_type: None,
        },
        &CensusProgress::default(),
    )
    .map(|run| summarise(&run.report))
    .map_err(|error| error.to_string())
}

pub(crate) fn greedy(files: &[Option<Vec<usize>>], frequency_order: &[usize]) -> Vec<usize> {
    greedy_order(&CoverageInput {
        files,
        construct_count: frequency_order.len(),
        frequency_order,
    })
}

/// The blockers of `files` run with chained models over a schema holding the shop's customers.
pub(crate) fn blocker_summary(files: &[ShopFile], names: bool) -> BlockerSummary {
    blocker_summary_with(files, names, ConversionOptions::default())
}

/// The blockers of `files` as `blocker_summary`, converting with `conversion`.
pub(crate) fn blocker_summary_with(
    files: &[ShopFile],
    names: bool,
    conversion: ConversionOptions,
) -> BlockerSummary {
    let directory = tempfile::tempdir().expect("temporary directory");
    write_files(directory.path(), files);
    let mut schema = Schema::default();
    schema.relations.insert(
        "__SQLQUACK_RELATION__SHOP__CUSTOMERS".to_owned(),
        vec![
            ColumnSpec::new(
                "CUSTOMER_ID",
                SnowflakeType::Number {
                    precision: 38,
                    scale: 0,
                },
            ),
            ColumnSpec::new("NAME", SnowflakeType::Varchar),
        ],
    );
    let run = run_census_timed(
        &[directory.path().to_path_buf()],
        &CensusOptions {
            dialect: SourceDialect::Snowflake,
            coverage_points: vec![1],
            schema,
            chain_models: true,
            conversion,
            names,
            assume_supported: Vec::new(),
            cursor_type: None,
        },
        &CensusProgress::default(),
    )
    .expect("the census runs");
    let blockers = run.report.blockers;
    BlockerSummary {
        roots: blockers
            .roots
            .iter()
            .map(|root| {
                (
                    root.construct.clone(),
                    [root.direct, root.downstream, root.total, root.sole],
                )
            })
            .collect(),
        single_root_files: blockers.single_root_files,
        possibly_incomplete_files: blockers.possibly_incomplete_files,
        pairs: blockers
            .combinations
            .iter()
            .filter(|combination| combination.roots.len() == 2 && combination.files > 0)
            .map(|combination| {
                (
                    combination
                        .roots
                        .iter()
                        .map(|root| root.construct.clone())
                        .collect(),
                    combination.files,
                )
            })
            .collect(),
        files: blockers
            .files
            .iter()
            .map(|file| {
                (
                    file.model.clone(),
                    file.upstream
                        .iter()
                        .map(|upstream| upstream.model.clone())
                        .collect(),
                    file.unknown_relations.len(),
                )
            })
            .collect(),
    }
}

/// The census of `files`, the schema file also typing each model in `typed` as an id and a text.
pub(crate) fn upstream_summary(files: &[ShopFile], typed: &[&str]) -> UpstreamSummary {
    let directory = tempfile::tempdir().expect("temporary directory");
    write_files(directory.path(), files);
    let mut schema = Schema::default();
    let id = ColumnSpec::new(
        "CUSTOMER_ID",
        SnowflakeType::Number {
            precision: 38,
            scale: 0,
        },
    );
    schema.relations.insert(
        "__SQLQUACK_RELATION__SHOP__CUSTOMERS".to_owned(),
        vec![id.clone(), ColumnSpec::new("NAME", SnowflakeType::Varchar)],
    );
    for model in typed {
        schema.relations.insert(
            format!("__SQLQUACK_RELATION__{}", model.to_ascii_uppercase()),
            vec![id.clone(), ColumnSpec::new("SOUND", SnowflakeType::Varchar)],
        );
    }
    let run = run_census_timed(
        &[directory.path().to_path_buf()],
        &CensusOptions {
            dialect: SourceDialect::Snowflake,
            coverage_points: vec![1],
            schema,
            chain_models: true,
            conversion: ConversionOptions::default(),
            names: true,
            assume_supported: Vec::new(),
            cursor_type: None,
        },
        &CensusProgress::default(),
    )
    .expect("the census runs");
    let text = render_census_text(&run.report, 10);
    UpstreamSummary {
        convertible: run.report.refusals.convertible,
        strict: run.report.refusals.convertible_strict,
        labelled: run
            .report
            .blockers
            .upstream_refused
            .iter()
            .map(|file| (file.model.clone(), file.labels.clone()))
            .collect(),
        refused: run
            .report
            .blockers
            .files
            .iter()
            .map(|file| file.model.clone())
            .collect(),
        text_names_both: text.contains("convertible: strict") && text.contains("unit-testable"),
    }
}

pub(crate) fn owned(models: &[&str]) -> Vec<String> {
    models.iter().map(|model| (*model).to_owned()).collect()
}

/// Each refused file's placed refusals as text, and the report text's refusal position lines.
pub(crate) fn placed_refusals(
    files: &[ShopFile],
    names: bool,
) -> (Vec<(String, Vec<String>)>, Vec<String>) {
    let directory = tempfile::tempdir().expect("temporary directory");
    write_files(directory.path(), files);
    let mut schema = Schema::default();
    schema.relations.insert(
        "__SQLQUACK_RELATION__SHOP__CUSTOMERS".to_owned(),
        vec![
            ColumnSpec::new(
                "CUSTOMER_ID",
                SnowflakeType::Number {
                    precision: 38,
                    scale: 0,
                },
            ),
            ColumnSpec::new("NAME", SnowflakeType::Varchar),
        ],
    );
    let run = run_census_timed(
        &[directory.path().to_path_buf()],
        &CensusOptions {
            dialect: SourceDialect::Snowflake,
            coverage_points: vec![1],
            schema,
            chain_models: false,
            conversion: ConversionOptions::default(),
            names,
            assume_supported: Vec::new(),
            cursor_type: None,
        },
        &CensusProgress::default(),
    )
    .expect("the census runs");
    let placed = run
        .report
        .blockers
        .files
        .iter()
        .map(|file| {
            let refusals = file
                .placed
                .iter()
                .map(|refusal| {
                    format!(
                        "{}:{} {}: {}",
                        refusal.line.unwrap_or_default(),
                        refusal.column.unwrap_or_default(),
                        refusal.category,
                        refusal.construct
                    )
                })
                .collect();
            (file.model.clone(), refusals)
        })
        .collect();
    let text = render_census_text(&run.report, 5);
    let lines = text
        .lines()
        .skip_while(|line| !line.starts_with("Refusal positions"))
        .skip(1)
        .take_while(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    (placed, lines)
}

/// Conversion options allowing the `hash` divergence for HASH.
pub(crate) fn hash_allowed() -> ConversionOptions {
    let mut options = ConversionOptions::default();
    options.divergences.insert(
        "hash".to_owned(),
        AllowedDivergence {
            constructs: vec!["fn:HASH".to_owned()],
            reason: "test".to_owned(),
            assumption: false,
        },
    );
    options
}

/// The repository policy's conversion options.
pub(crate) fn policy_allowed() -> ConversionOptions {
    let policy = load_policy(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(POLICY_FILE_NAME),
    )
    .expect("the repository policy loads");
    conversion_options(&policy, false)
}

/// The divergence tally of `files` run with chained models and the repository policy.
pub(crate) fn chained_divergences(files: &[ShopFile]) -> BTreeMap<String, usize> {
    chained_report(files).provenance.divergences
}

/// Unit-testable and strict counts of `files`, with and without assumptions, and files per one.
pub(crate) fn assumption_counts(files: &[ShopFile]) -> ([usize; 4], BTreeMap<String, usize>) {
    let refusals = chained_report(files).refusals;
    (
        [
            refusals.convertible,
            refusals.convertible_strict,
            refusals.convertible_no_assumptions,
            refusals.convertible_strict_no_assumptions,
        ],
        refusals.assumptions,
    )
}

/// The census of `files` run with chained models and the repository policy.
fn chained_report(files: &[ShopFile]) -> CensusReport {
    let policy = load_policy(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(POLICY_FILE_NAME),
    )
    .expect("the repository policy loads");
    let directory = tempfile::tempdir().expect("temporary directory");
    write_files(directory.path(), files);
    let mut schema = Schema::default();
    schema.relations.insert(
        "__SQLQUACK_RELATION__SHOP__CUSTOMERS".to_owned(),
        vec![ColumnSpec::new(
            "CUSTOMER_ID",
            SnowflakeType::Number {
                precision: 38,
                scale: 0,
            },
        )],
    );
    schema.relations.insert(
        "__SQLQUACK_RELATION__SHOP__EVENTS".to_owned(),
        vec![ColumnSpec::new("PAYLOAD", SnowflakeType::SemiStructured)],
    );
    run_census_timed(
        &[directory.path().to_path_buf()],
        &CensusOptions {
            dialect: SourceDialect::Snowflake,
            coverage_points: vec![1],
            schema,
            chain_models: true,
            conversion: conversion_options(&policy, false),
            names: true,
            assume_supported: Vec::new(),
            cursor_type: None,
        },
        &CensusProgress::default(),
    )
    .expect("the census runs")
    .report
}

/// The plan of `files` with `assumed` supported, beside the census's own counts.
pub(crate) fn plan_summary(files: &[ShopFile], assumed: &[&str]) -> PlanSummary {
    let directory = tempfile::tempdir().expect("temporary directory");
    write_files(directory.path(), files);
    let mut schema = Schema::default();
    schema.relations.insert(
        "__SQLQUACK_RELATION__SHOP__CUSTOMERS".to_owned(),
        vec![
            ColumnSpec::new(
                "CUSTOMER_ID",
                SnowflakeType::Number {
                    precision: 38,
                    scale: 0,
                },
            ),
            ColumnSpec::new("NAME", SnowflakeType::Varchar),
        ],
    );
    let run = run_census_timed(
        &[directory.path().to_path_buf()],
        &CensusOptions {
            dialect: SourceDialect::Snowflake,
            coverage_points: vec![1],
            schema,
            chain_models: true,
            conversion: ConversionOptions::default(),
            names: true,
            assume_supported: assumed
                .iter()
                .map(|construct| (*construct).to_owned())
                .collect(),
            cursor_type: None,
        },
        &CensusProgress::default(),
    )
    .expect("the census runs");
    let plan = &run.report.plan;
    PlanSummary {
        convertible: [
            run.report.refusals.convertible,
            run.report.refusals.convertible_strict,
        ],
        upper_bounds: [plan.unit_testable_upper_bound, plan.strict_upper_bound],
        next: plan
            .next
            .iter()
            .map(|next| (next.root.construct.clone(), next.files, next.sole))
            .collect(),
        steps: plan
            .steps
            .iter()
            .map(|step| {
                (
                    step.roots
                        .iter()
                        .map(|root| root.construct.clone())
                        .collect(),
                    [step.unit_testable_upper_bound, step.strict_upper_bound],
                )
            })
            .collect(),
    }
}
