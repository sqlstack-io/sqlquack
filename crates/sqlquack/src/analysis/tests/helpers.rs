use std::collections::BTreeSet;

use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::provenance::main::merge_provenance::merge_provenance;
use sqlquack_core::refusals::models::RefusalCategory;
use sqlquack_evidence::cases::main::load_cases::load_cases;
use sqlquack_evidence::constants::CASES_DIR;
use sqlquack_evidence::verify::models::Conversion;
use sqlquack_evidence::verify::types::CaseConverter;
use sqlquack_policy::policy::constants::POLICY_FILE_NAME;
use sqlquack_policy::policy::main::load_policy::load_policy;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_rules::conversion::models::{
    ColumnSpec, ConversionBudget, ConversionOptions, FunctionLanguage, FunctionReturns,
    FunctionSpec, Schema,
};
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::analysis::main::analyze::analyze;
use crate::analysis::main::analyze_function::analyze_function;
use crate::analysis::main::check_equivalence_cases::check_equivalence_cases;
use crate::analysis::main::conversion_options::conversion_options;
use crate::analysis::models::PipelineConverter;
use crate::analysis::tests::test_types::{AnalysisSummary, DivergenceModeOutcome};

fn shop_schema() -> Schema {
    let mut schema = shop_relations();
    let column = |name: &str, data_type: SnowflakeType| ColumnSpec {
        name: name.to_owned(),
        data_type,
    };
    let amount = SnowflakeType::Number {
        precision: 10,
        scale: 2,
    };
    let function = |language, arguments, returns| FunctionSpec {
        language,
        arguments,
        returns,
    };
    schema.functions.insert(
        "IS_GIFT".to_owned(),
        function(
            FunctionLanguage::Python,
            vec![column("NOTE", SnowflakeType::Varchar)],
            FunctionReturns::Scalar(SnowflakeType::Boolean),
        ),
    );
    schema.functions.insert(
        "__SQLQUACK_UDF__ROUNDED".to_owned(),
        function(
            FunctionLanguage::Sql,
            vec![column("AMOUNT", amount)],
            FunctionReturns::Scalar(amount),
        ),
    );
    schema.functions.insert(
        "PAYLOAD_SIZE".to_owned(),
        function(
            FunctionLanguage::Python,
            vec![column("PAYLOAD", SnowflakeType::SemiStructured)],
            FunctionReturns::Scalar(SnowflakeType::Float),
        ),
    );
    schema.functions.insert(
        "LARGE_ORDERS".to_owned(),
        function(
            FunctionLanguage::Sql,
            vec![column("MINIMUM", amount)],
            FunctionReturns::Table(vec![column("AMOUNT", amount)]),
        ),
    );
    schema
}

fn shop_relations() -> Schema {
    let column = |name: &str, data_type: SnowflakeType| ColumnSpec {
        name: name.to_owned(),
        data_type,
    };
    let mut schema = Schema::default();
    schema.relations.insert(
        "CUSTOMERS".to_owned(),
        vec![
            column(
                "CUSTOMER_ID",
                SnowflakeType::Number {
                    precision: 38,
                    scale: 0,
                },
            ),
            column("NOTE", SnowflakeType::Varchar),
        ],
    );
    schema.relations.insert(
        "EVENTS".to_owned(),
        vec![column("PAYLOAD", SnowflakeType::SemiStructured)],
    );
    schema.relations.insert(
        "ORDERS".to_owned(),
        vec![
            column(
                "CUSTOMER_ID",
                SnowflakeType::Number {
                    precision: 38,
                    scale: 0,
                },
            ),
            column(
                "AMOUNT",
                SnowflakeType::Number {
                    precision: 10,
                    scale: 2,
                },
            ),
        ],
    );
    schema
}

/// The summary of a statement that converts.
pub(crate) fn converted() -> AnalysisSummary {
    AnalysisSummary {
        parsed: true,
        statements: 1,
        categories: vec![],
        converted: true,
    }
}

/// The summary of a statement refused in one category.
pub(crate) fn refused(category: RefusalCategory) -> AnalysisSummary {
    AnalysisSummary {
        parsed: true,
        statements: 1,
        categories: vec![category],
        converted: false,
    }
}

pub(crate) fn summarise(sql: &str) -> AnalysisSummary {
    let analysis = analyze(
        sql,
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&shop_schema(), &ConversionOptions::default()),
    );
    eprintln!(
        "DEBUG {sql}: {:?}",
        analysis
            .refusals
            .iter()
            .map(|r| r.reason.clone())
            .collect::<Vec<_>>()
    );
    AnalysisSummary {
        parsed: analysis.parsed,
        statements: analysis.statements,
        categories: analysis
            .refusals
            .iter()
            .map(|refusal| refusal.category)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        converted: analysis.refusals.is_empty() && analysis.conversions.len() == 1,
    }
}

fn repository_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Converts a SQL function body declared by the shop schema, and runs its macro over `rows`.
pub(crate) fn function_body_results(name: &str, body: &str, call: &str) -> Vec<String> {
    let schema = shop_schema();
    let spec = &schema.functions[name];
    let converted = analyze_function(
        (body, spec),
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&schema, &ConversionOptions::default()),
    )
    .map_err(|refusals| format!("{refusals:?}"))
    .expect("the body converts");
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection
        .execute_batch(&format!(
            "CREATE TABLE \"ORDERS\" (\"CUSTOMER_ID\" DECIMAL(38,0), \"AMOUNT\" DECIMAL(10,2)); \
             INSERT INTO \"ORDERS\" VALUES (1, 5.25), (2, 12.50), (3, NULL); \
             CREATE MACRO \"{name}\"({}) AS {}",
            converted.parameters.join(", "),
            converted.body
        ))
        .expect("the macro is created");
    let mut statement = connection.prepare(call).expect("the call prepares");
    statement
        .query_map([], |row| row.get::<_, Option<String>>(0))
        .expect("the call runs")
        .map(|value| value.expect("a value").unwrap_or_else(|| "NULL".to_owned()))
        .collect()
}

/// Converts `sql` over the shop schema under the repository policy, and runs it on one event row.
pub(crate) fn event_results(sql: &str, payload: &str) -> Vec<String> {
    let policy =
        load_policy(&repository_root().join(POLICY_FILE_NAME)).expect("repository policy loads");
    let analysis = analyze(
        sql,
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&shop_schema(), &conversion_options(&policy, false)),
    );
    let [converted] = analysis.conversions.as_slice() else {
        panic!("the statement converts: {:?}", analysis.refusals);
    };
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection
        .execute_batch(&format!(
            "CREATE TABLE \"EVENTS\" (\"PAYLOAD\" JSON); \
             INSERT INTO \"EVENTS\" VALUES ('{}')",
            payload.replace('\'', "''")
        ))
        .expect("the events table is created");
    let mut statement = connection
        .prepare(&format!(
            "SELECT CAST(c AS VARCHAR) FROM ({}) AS t(c)",
            converted.sql
        ))
        .expect("the conversion prepares");
    statement
        .query_map([], |row| row.get::<_, Option<String>>(0))
        .expect("the conversion runs")
        .map(|value| value.expect("a value").unwrap_or_else(|| "NULL".to_owned()))
        .collect()
}

/// The equivalence-case problems in the repository's own cases.
pub(crate) fn equivalence_case_checks() -> Vec<String> {
    let root = repository_root();
    let policy = load_policy(&root.join(POLICY_FILE_NAME)).expect("the repository policy loads");
    let cases = load_cases(&root.join(CASES_DIR), &policy).expect("the repository cases load");
    check_equivalence_cases(&cases)
}

/// Converts `sql` under the repository policy's known divergences in one mode.
pub(crate) fn divergence_outcome(sql: &str, strict: bool) -> DivergenceModeOutcome {
    let policy =
        load_policy(&repository_root().join(POLICY_FILE_NAME)).expect("repository policy loads");
    let options = conversion_options(&policy, strict);
    let analysis = analyze(
        sql,
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&shop_schema(), &options),
    );
    let provenance = merge_provenance(
        analysis
            .conversions
            .iter()
            .map(|converted| &converted.provenance),
    );
    DivergenceModeOutcome {
        converted: analysis.refusals.is_empty() && analysis.conversions.len() == 1,
        divergences: provenance
            .divergences
            .into_iter()
            .map(|(id, constructs)| (id, constructs.into_iter().collect()))
            .collect(),
        categories: analysis
            .refusals
            .iter()
            .map(|refusal| refusal.category)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    }
}

/// The divergences of a conversion relying on `numeric_range` at one construct.
pub(crate) fn numeric_range(construct: &str) -> Vec<(String, Vec<String>)> {
    vec![("numeric_range".to_owned(), vec![construct.to_owned()])]
}

/// The rules a repository case's conversion relies on, which exclude its input fixture's.
pub(crate) fn case_rules(reference: &str) -> Vec<String> {
    let root = repository_root();
    let policy = load_policy(&root.join(POLICY_FILE_NAME)).expect("the repository policy loads");
    let cases = load_cases(&root.join(CASES_DIR), &policy).expect("the repository cases load");
    let case = cases
        .iter()
        .find(|case| case.reference == reference)
        .expect("the case exists");
    let converter = PipelineConverter {
        options: conversion_options(&policy, false),
    };
    let Conversion::Converted { provenance, .. } = converter.convert(case) else {
        panic!("{reference} converts");
    };
    provenance.rules.into_iter().collect()
}

/// Whether `sql`'s conversion evaluates its guarded conjuncts only behind the others.
pub(crate) fn guards_gated(sql: &str) -> bool {
    let policy =
        load_policy(&repository_root().join(POLICY_FILE_NAME)).expect("repository policy loads");
    let analysis = analyze(
        sql,
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&shop_schema(), &conversion_options(&policy, false)),
    );
    let [converted] = analysis.conversions.as_slice() else {
        panic!("the statement converts: {:?}", analysis.refusals);
    };
    converted.sql.contains(" ELSE FALSE END")
}

/// The DuckDB SQL `sql` converts to against the shop schema, and how long conversion took.
pub(crate) fn timed_conversion(sql: &str) -> (Option<String>, std::time::Duration) {
    let policy =
        load_policy(&repository_root().join(POLICY_FILE_NAME)).expect("repository policy loads");
    let started = std::time::Instant::now();
    let analysis = analyze(
        sql,
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&shop_schema(), &conversion_options(&policy, false)),
    );
    let elapsed = started.elapsed();
    let converted = analysis
        .conversions
        .first()
        .filter(|_| analysis.conversions.len() == 1)
        .map(|converted| converted.sql.clone());
    (converted, elapsed)
}

/// Flattened VARIANT elements unpacked into `columns` texts, each nested `depth` times in PARSE_JSON.
pub(crate) fn unpacking_query(columns: usize, depth: usize) -> String {
    let written = |key: usize| {
        (0..depth).fold(format!("f.value:k{key}"), |inner, _| {
            format!("PARSE_JSON(TO_VARCHAR({inner}))")
        })
    };
    let projections: Vec<String> = (0..columns)
        .map(|key| {
            format!(
                "CAST(COALESCE(TO_VARCHAR({}), 'none') AS VARCHAR(16777216)) AS c{key}",
                written(key)
            )
        })
        .collect();
    format!(
        "SELECT {}, TRY_TO_DATE(f.value:d::STRING) AS d FROM events AS e, \
         LATERAL FLATTEN(input => e.payload, outer => TRUE) AS f \
         WHERE f.value IS NULL OR f.value:k0::STRING = 'a'",
        projections.join(", ")
    )
}

/// The category and reason refusing `sql` under `budget`, or `None` when it converts.
pub(crate) fn budget_refusal(
    sql: &str,
    budget: ConversionBudget,
) -> Option<(RefusalCategory, String)> {
    let policy =
        load_policy(&repository_root().join(POLICY_FILE_NAME)).expect("repository policy loads");
    let mut options = conversion_options(&policy, false);
    options.budget = budget;
    let analysis = analyze(
        sql,
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&shop_schema(), &options),
    );
    analysis
        .refusals
        .first()
        .map(|refusal| (refusal.category, refusal.reason.clone()))
}
