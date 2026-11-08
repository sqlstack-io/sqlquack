use std::collections::BTreeSet;

use sha2::{Digest, Sha256};
use sqlquack_core::diagnostics::models::DiagnosticSpan;
use sqlquack_core::failures::main::duckdb_failure::duckdb_failure;
use sqlquack_core::parsing::models::{SourceDialect, SourceLocation};
use sqlquack_core::provenance::main::merge_provenance::merge_provenance;
use sqlquack_core::refusals::models::{Refusal, RefusalCategory};
use sqlquack_evidence::cases::main::load_cases::load_cases;
use sqlquack_evidence::constants::CASES_DIR;
use sqlquack_evidence::verify::models::Conversion;
use sqlquack_evidence::verify::types::CaseConverter;
use sqlquack_policy::policy::constants::POLICY_FILE_NAME;
use sqlquack_policy::policy::constants::PROJECT_UDF;
use sqlquack_policy::policy::main::load_policy::load_policy;
use sqlquack_policy::policy::models::DivergenceMode;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_rules::catalogue::models::{NotExpressibleDeclaration, RuleCatalogue};
use sqlquack_rules::conversion::models::{ConversionBudget, ConversionOptions, Converted};
use sqlquack_semantics::types::models::{
    ColumnSpec, FunctionLanguage, FunctionReturns, FunctionSpec, FunctionTraits, Schema,
};
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::analysis::main::check_equivalence_cases::check_equivalence_cases;
use crate::analysis::main::check_source::check_source;
use crate::analysis::main::collect_all_refusals::collect_all_refusals;
use crate::analysis::main::confirm_assumption::confirm_assumption;
use crate::analysis::main::refused_construct::refused_construct;
use crate::analysis::models::PipelineConverter;
use crate::analysis::tests::test_types::{
    AnalysisSummary, CollectedSummary, DivergenceModeOutcome, EverydayCorpus, EverydayQuery,
};
use crate::cli::main::convert_report::convert_report;
use crate::cli::models::{
    ColorArguments, ConvertArguments, OutputArguments, OutputFormat, PolicyArguments,
};
use sqlquack_convert::analysis::main::analyze::analyze;
use sqlquack_convert::analysis::main::analyze_function::analyze_function;
use sqlquack_rules::session::main::confirm_utc_session::confirm_utc_session;
use sqlquack_rules::session::main::conversion_options::conversion_options;

fn shop_schema() -> Schema {
    let mut schema = shop_relations();
    let column = |name: &str, data_type: SnowflakeType| ColumnSpec::new(name, data_type);
    let amount = SnowflakeType::Number {
        precision: 10,
        scale: 2,
    };
    let function = |language, arguments, returns| FunctionSpec {
        language,
        arguments,
        returns,
        traits: FunctionTraits::default(),
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
    let column = |name: &str, data_type: SnowflakeType| ColumnSpec::new(name, data_type);
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
    event_results_allowing(sql, payload, &[])
}

/// As `event_results`, with the policy's default for each of `allowed` set to warn, as a user opts in.
pub(crate) fn event_results_allowing(sql: &str, payload: &str, allowed: &[&str]) -> Vec<String> {
    let mut policy =
        load_policy(&repository_root().join(POLICY_FILE_NAME)).expect("repository policy loads");
    for id in allowed {
        policy
            .divergences
            .get_mut(*id)
            .expect("the divergence is in the policy")
            .default = DivergenceMode::Warn;
    }
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
    session_outcome(sql, (strict, false))
}

/// The outcome of `sql` in strict or default mode, with or without a confirmed Python runtime.
pub(crate) fn python_outcome(sql: &str, (strict, runtime): (bool, bool)) -> DivergenceModeOutcome {
    let policy =
        load_policy(&repository_root().join(POLICY_FILE_NAME)).expect("repository policy loads");
    let options = std::iter::repeat_n(&policy, usize::from(runtime))
        .fold(conversion_options(&policy, strict), |options, policy| {
            confirm_assumption(options, policy, PROJECT_UDF)
        });
    outcome(sql, &options)
}

/// The outcome of `sql` in strict or default mode, with or without a confirmed UTC session.
pub(crate) fn session_outcome(
    sql: &str,
    (strict, utc_session): (bool, bool),
) -> DivergenceModeOutcome {
    let policy =
        load_policy(&repository_root().join(POLICY_FILE_NAME)).expect("repository policy loads");
    let options = std::iter::repeat_n(&policy, usize::from(utc_session))
        .fold(conversion_options(&policy, strict), confirm_utc_session);
    outcome(sql, &options)
}

fn outcome(sql: &str, options: &ConversionOptions) -> DivergenceModeOutcome {
    let analysis = analyze(
        sql,
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&shop_schema(), options),
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
        python: None,
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

/// Each refusal of `sql`, with `declared` not expressible and `budget`: category, text and place.
pub(crate) fn refusal_places<'s>(
    sql: &'s str,
    (declared, budget): (Option<&'static str>, Option<ConversionBudget>),
) -> Vec<(RefusalCategory, &'s str, [usize; 2])> {
    let policy =
        load_policy(&repository_root().join(POLICY_FILE_NAME)).expect("repository policy loads");
    let mut options = conversion_options(&policy, false);
    options.budget = budget.unwrap_or(options.budget);
    let catalogue = RuleCatalogue {
        not_expressible: declared
            .map(|construct| NotExpressibleDeclaration {
                construct,
                reason: "declared for the test",
            })
            .into_iter()
            .collect(),
    };
    let analysis = analyze(
        sql,
        SourceDialect::Snowflake,
        &catalogue,
        (&shop_schema(), &options),
    );
    analysis
        .refusals
        .iter()
        .map(|refusal| placed(sql, refusal))
        .collect()
}

/// Each refusal of a NUMBER scalar function's `body`, placed in the body as `refusal_places` does.
pub(crate) fn function_refusal_places(body: &str) -> Vec<(RefusalCategory, &str, [usize; 2])> {
    let amount = SnowflakeType::Number {
        precision: 10,
        scale: 2,
    };
    let spec = FunctionSpec {
        language: FunctionLanguage::Sql,
        arguments: vec![ColumnSpec::new("AMOUNT", amount)],
        returns: FunctionReturns::Scalar(amount),
        traits: FunctionTraits::default(),
    };
    let refusals = analyze_function(
        (body, &spec),
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&shop_schema(), &ConversionOptions::default()),
    )
    .err()
    .unwrap_or_default();
    refusals
        .iter()
        .map(|refusal| placed(body, refusal))
        .collect()
}

/// A refusal's category, the text of `source` at its location, and its line and column.
fn placed<'s>(source: &'s str, refusal: &Refusal) -> (RefusalCategory, &'s str, [usize; 2]) {
    let location = refusal.location.unwrap_or(SourceLocation {
        start: 0,
        end: 0,
        line: 0,
        column: 0,
    });
    let byte = |offset: usize| {
        source
            .char_indices()
            .nth(offset)
            .map_or(source.len(), |(index, _)| index)
    };
    (
        refusal.category,
        &source[byte(location.start)..byte(location.end)],
        [location.line, location.column],
    )
}

/// Tables of the shop schema with a few rows, for running converted SQL in DuckDB.
const SHOP_ROWS: &str = "CREATE TABLE \"CUSTOMERS\" (\"CUSTOMER_ID\" DECIMAL(38, 0), \"NOTE\" VARCHAR); \
     INSERT INTO \"CUSTOMERS\" VALUES (1, '1'), (2, '7'); \
     CREATE TABLE \"ORDERS\" (\"CUSTOMER_ID\" DECIMAL(38, 0), \"AMOUNT\" DECIMAL(10, 2)); \
     INSERT INTO \"ORDERS\" VALUES (1, 5.00), (2, 7.50), (2, 2.50); \
     CREATE TABLE \"EVENTS\" (\"PAYLOAD\" JSON); \
     INSERT INTO \"EVENTS\" VALUES ('{\"k\": \"a\"}'), ('{\"k\": \"b\"}')";

/// `sql` converted over the shop schema under the repository policy, or its refusals' reasons.
pub(crate) fn shop_conversion(sql: &str) -> Result<Converted, Vec<String>> {
    let policy =
        load_policy(&repository_root().join(POLICY_FILE_NAME)).expect("repository policy loads");
    let analysis = analyze(
        sql,
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&shop_schema(), &conversion_options(&policy, false)),
    );
    let reasons: Vec<String> = analysis
        .refusals
        .iter()
        .map(|refusal| refusal.reason.clone())
        .collect();
    let single = reasons.is_empty() && analysis.conversions.len() == 1;
    analysis
        .conversions
        .into_iter()
        .next()
        .filter(|_| single)
        .ok_or(reasons)
}

/// How many rows `sql` returns when converted and run on the shop rows, or why it does not.
pub(crate) fn shop_row_count(sql: &str) -> Result<usize, Vec<String>> {
    let converted = shop_conversion(sql)?;
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection
        .execute_batch(SHOP_ROWS)
        .expect("the shop tables are created");
    connection
        .query_row(
            &format!("SELECT COUNT(*) FROM ({}) AS t", converted.sql),
            [],
            |row| row.get::<_, usize>(0),
        )
        .map_err(|error| vec![error.to_string()])
}

/// The lines and columns of each raise ID's locations in the conversion of `sql`, sorted.
pub(crate) fn raise_places(sql: &str) -> Result<Vec<Vec<[usize; 2]>>, Vec<String>> {
    let mut raises: Vec<Vec<[usize; 2]>> = shop_conversion(sql)?
        .raises
        .values()
        .map(|locations| places(locations))
        .collect();
    raises.sort();
    Ok(raises)
}

/// Where the error raised running the conversion of `sql` on the shop rows comes from.
pub(crate) fn raised_places(sql: &str) -> Result<Vec<[usize; 2]>, Vec<String>> {
    let converted = shop_conversion(sql)?;
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection
        .execute_batch(SHOP_ROWS)
        .expect("the shop tables are created");
    let error = connection
        .execute_batch(&converted.sql)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    Ok(places(converted.raise_locations(&error)))
}

fn places(locations: &[SourceLocation]) -> Vec<[usize; 2]> {
    locations
        .iter()
        .map(|location| [location.line, location.column])
        .collect()
}

/// Whether every SQL text converts to the same DuckDB SQL, byte for byte.
pub(crate) fn converts_identically(texts: &[&str]) -> Result<bool, Vec<String>> {
    let converted: Vec<String> = texts
        .iter()
        .map(|sql| shop_conversion(sql).map(|converted| converted.sql))
        .collect::<Result<_, _>>()?;
    Ok(converted.windows(2).all(|pair| pair[0] == pair[1]))
}

/// Every refused construct of `sql` over the shop, collected past each refusal.
pub(crate) fn collected_summary(sql: &str) -> CollectedSummary {
    let collected = collect_all_refusals(
        sql,
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&shop_schema(), &ConversionOptions::default()),
    );
    CollectedSummary {
        constructs: collected
            .refusals
            .iter()
            .map(refused_construct)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        possibly_incomplete: collected.possibly_incomplete,
    }
}

/// Each statement's converted SQL, or `None` where it is refused.
pub(crate) fn converted_sql(statements: &[&str]) -> Vec<Option<String>> {
    statements
        .iter()
        .map(|sql| {
            let analysis = analyze(
                sql,
                SourceDialect::Snowflake,
                &builtin_catalogue(),
                (&shop_schema(), &ConversionOptions::default()),
            );
            let sql: Vec<String> = analysis
                .conversions
                .iter()
                .map(|converted| converted.sql.clone())
                .collect();
            analysis.refusals.is_empty().then(|| sql.join(";\n"))
        })
        .collect()
}

/// The first value `sql` returns as text when converted and run on the shop rows, or why not.
pub(crate) fn shop_first_value(sql: &str) -> Result<String, Vec<String>> {
    let converted = shop_conversion(sql)?;
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection
        .execute_batch(SHOP_ROWS)
        .expect("the shop tables are created");
    connection
        .query_row(
            &format!(
                "SELECT CAST(COLUMNS(*) AS VARCHAR) FROM ({}) AS t",
                converted.sql
            ),
            [],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| vec![error.to_string()])
}

/// The first value, or the expected fragment when a reason contains it, else every reason.
pub(crate) fn shop_first_outcome(
    sql: &str,
    expected: Result<&str, &str>,
) -> Result<String, String> {
    let fragment = expected.err();
    shop_first_value(sql).map_err(|reasons| {
        reasons
            .iter()
            .find(|reason| fragment.is_some_and(|fragment| reason.contains(fragment)))
            .and(fragment)
            .map_or_else(|| reasons.join("; "), str::to_owned)
    })
}

/// How many times the converted `wrapper` writes `ARGUMENT`, to the nearest copy, as it grows.
pub(crate) fn argument_copies(wrapper: &str, short: &str, long: &str) -> usize {
    let length = |argument: &str| {
        shop_conversion(&wrapper.replace("ARGUMENT", argument))
            .map(|converted| converted.sql.len())
            .expect("the wrapper converts")
    };
    let bare = |argument: &str| {
        shop_conversion(&format!("SELECT {argument} AS v"))
            .map(|converted| converted.sql.len())
            .expect("the argument converts")
    };
    let grown = bare(long) - bare(short);
    (length(long) - length(short) + grown / 2) / grown
}

/// The outcome of a statement that converts relying only on `session_timezone_utc` at `constructs`.
pub(crate) fn relying_on_utc_session(constructs: &[&str]) -> DivergenceModeOutcome {
    DivergenceModeOutcome {
        converted: true,
        divergences: vec![(
            "session_timezone_utc".to_owned(),
            constructs
                .iter()
                .map(|construct| (*construct).to_owned())
                .collect(),
        )],
        categories: vec![],
    }
}

/// A corpus's regressions against its recorded outcomes; improvements and rates are printed.
pub(crate) fn everyday_changes(directory: &str, description: &str) -> Vec<String> {
    let (root, corpus) = everyday_corpus(directory);
    let strict = everyday_arguments(&root, true);
    let repository = everyday_arguments(&root, false);
    let fixture = std::fs::read_to_string(root.join(EVERYDAY_FIXTURE)).ok();
    let mut regressions = Vec::new();
    let mut improvements = Vec::new();
    let recorded: Vec<EverydayQuery> = corpus
        .query
        .iter()
        .map(|query| {
            let policies = [
                (STRICT, &strict, &query.expect, &query.result),
                (
                    REPOSITORY,
                    &repository,
                    &query.expect_repository,
                    &query.result_repository,
                ),
            ];
            let [
                (strict_outcome, strict_result),
                (repository_outcome, repository_result),
            ] = policies.map(|(policy, arguments, expected, expected_result)| {
                let (outcome, translated) = everyday_outcome(&query.sql, arguments);
                let result = fixture
                    .as_deref()
                    .zip(translated.as_deref())
                    .map(|(rows, sql)| fixture_result(rows, sql));
                let place = format!("{} under {policy}", query.sql);
                let changed = (outcome != *expected)
                    .then(|| format!("{place}: expected {expected}, got {outcome}"));
                // Only a refusal that now translates improves; a parse or check diagnostic that
                // disappears means SQL Snowflake may reject now converts, so it fails.
                let improved = expected == UNRECORDED
                    || (outcome == TRANSLATED && expected.starts_with(REFUSED));
                improvements.extend(changed.clone().filter(|_| improved));
                regressions.extend(changed.filter(|_| !improved));
                regressions.extend((outcome == *expected && result != *expected_result).then(
                    || {
                        format!(
                            "{place}: the result on the fixture rows was {}, is now {}",
                            expected_result.as_deref().unwrap_or("none"),
                            result.as_deref().unwrap_or("none")
                        )
                    },
                ));
                (outcome, result)
            });
            EverydayQuery {
                expect: strict_outcome,
                result: strict_result,
                expect_repository: repository_outcome,
                result_repository: repository_result,
                ..query.clone()
            }
        })
        .collect();
    let rerecording = std::env::var_os(RERECORD_VARIABLE).is_some();
    rerecording.then(|| rerecord(&root, &recorded));
    eprintln!(
        "{description}: {} refused statements now translate; run `{RERECORD_COMMAND}` to record them:\n{}",
        improvements.len(),
        improvements.join("\n")
    );
    eprintln!("{description}: {}", everyday_translation_rates(directory));
    regressions.into_iter().filter(|_| !rerecording).collect()
}

/// The rates of valid queries, not DDL, recorded as translated: strict, and repository policy.
pub(crate) fn everyday_translation_rates(directory: &str) -> String {
    let (_, corpus) = everyday_corpus(directory);
    let valid: Vec<&EverydayQuery> = corpus
        .query
        .iter()
        .filter(|query| !EVERYDAY_NOT_TRANSLATABLE.contains(&query.category.as_str()))
        .filter(|query| {
            query.snowflake.as_deref().map_or_else(
                || !query.category.starts_with(EVERYDAY_INVALID_PREFIX),
                |verdict| verdict == SNOWFLAKE_COMPILES,
            )
        })
        .collect();
    let strict = valid
        .iter()
        .filter(|query| query.expect == TRANSLATED)
        .count();
    let repository = valid
        .iter()
        .filter(|query| query.expect_repository == TRANSLATED)
        .count();
    format!(
        "{strict} of {} strict, {repository} of {} repository",
        valid.len(),
        valid.len()
    )
}

/// Queries Snowflake rejects that pass SQLQuack's parse and checks, once every receipt matches.
pub(crate) fn everyday_snowflake_rejections_not_caught(directory: &str) -> Vec<String> {
    let (root, corpus) = everyday_corpus(directory);
    let Ok(text) = std::fs::read_to_string(root.join("snowflake-receipts.json")) else {
        return Vec::new();
    };
    let receipts: serde_json::Value = serde_json::from_str(&text).expect("receipts parse");
    // Results are a list, or an object keyed by query id; a receipt without a verdict is skipped.
    let results: Vec<&serde_json::Value> = receipts["results"].as_array().map_or_else(
        || {
            receipts["results"]
                .as_object()
                .into_iter()
                .flat_map(|by_id| by_id.values())
                .collect()
        },
        |list| list.iter().collect(),
    );
    for receipt in results
        .into_iter()
        .filter(|receipt| receipt["compiles"].is_boolean())
    {
        let sql = receipt["sql"]
            .as_str()
            .expect("a receipt names its SQL")
            .trim();
        let verdict = receipt["compiles"]
            .as_bool()
            .filter(|compiles| *compiles)
            .map_or_else(
                || format!("rejects {}", receipt["errno"]),
                |_| SNOWFLAKE_COMPILES.to_owned(),
            );
        let query = corpus.query.iter().find(|query| query.sql.trim() == sql);
        assert_eq!(
            query.and_then(|query| query.snowflake.clone()),
            Some(verdict),
            "{sql}"
        );
    }
    let strict = everyday_arguments(&root, true);
    corpus
        .query
        .iter()
        .filter(|query| {
            query
                .snowflake
                .as_deref()
                .is_some_and(|verdict| verdict != SNOWFLAKE_COMPILES)
        })
        .filter(|query| {
            let (outcome, _) = everyday_outcome(&query.sql, &strict);
            !outcome.starts_with("parse ") && !outcome.starts_with("check ")
        })
        .map(|query| query.sql.clone())
        .collect()
}

/// The everyday corpus directory and its queries.
fn everyday_corpus(directory: &str) -> (std::path::PathBuf, EverydayCorpus) {
    let root = repository_root().join(directory);
    let text = std::fs::read_to_string(root.join(EVERYDAY_QUERIES)).expect("the corpus reads");
    let corpus: EverydayCorpus = toml::from_str(&text).expect("the corpus parses");
    assert!(corpus.query.len() > 300, "{} queries", corpus.query.len());
    (root, corpus)
}

/// Rewrites the corpus file with `queries`' outcomes, keeping its comments.
fn rerecord(root: &std::path::Path, queries: &[EverydayQuery]) {
    let path = root.join(EVERYDAY_QUERIES);
    let text = std::fs::read_to_string(&path).expect("the corpus reads");
    let mut chunks = text.split("[[query]]");
    let header = chunks.next().unwrap_or_default();
    // Comment lines between queries end the previous chunk; keep them before the next query.
    let separators: Vec<String> = chunks
        .map(|chunk| {
            let lines: Vec<&str> = chunk.lines().collect();
            let kept = lines
                .iter()
                .rev()
                .take_while(|line| line.is_empty() || line.starts_with('#'))
                .count();
            lines[lines.len() - kept..]
                .iter()
                .map(|line| format!("{line}\n"))
                .collect()
        })
        .collect();
    let blocks: String = queries
        .iter()
        .enumerate()
        .map(|(index, query)| {
            let optional: String = [
                ("snowflake", &query.snowflake),
                ("result", &query.result),
                ("result_repository", &query.result_repository),
            ]
            .into_iter()
            .filter_map(|(key, value)| value.as_ref().map(|value| format!("{key} = {value:?}\n")))
            .collect();
            let uncertain = query.uncertain.then_some("uncertain = true\n");
            let separator = separators
                .get(index)
                .filter(|separator| !separator.is_empty())
                .cloned()
                .unwrap_or_else(|| "\n".repeat(usize::from(index + 1 < queries.len())));
            format!(
                "[[query]]\ncategory = {:?}\nsql = '''{}'''\nexpect = {:?}\nexpect_repository = {:?}\n{optional}{}{separator}",
                query.category,
                query.sql,
                query.expect,
                query.expect_repository,
                uncertain.unwrap_or_default()
            )
        })
        .collect();
    std::fs::write(path, format!("{header}{blocks}")).expect("the corpus writes");
}

/// `convert`'s arguments: the built-in policy, or the repository's under a UTC session.
fn everyday_arguments(root: &std::path::Path, builtin_policy: bool) -> ConvertArguments {
    ConvertArguments {
        path: None,
        schema: None,
        overrides: None,
        verified_overrides: Vec::new(),
        session: Vec::new(),
        compact: true,
        output: OutputArguments {
            format: OutputFormat::Json,
            json: false,
        },
        colors: ColorArguments {
            color: clap::ColorChoice::Never,
        },
        conversion: PolicyArguments {
            project: Some(root.to_path_buf()),
            policy: Some(repository_root().join(POLICY_FILE_NAME)),
            builtin_policy,
            strict: false,
            utc_session: !builtin_policy,
            confirm_declared_offsets: false,
            python_runtime: false,
            loud_divergences_only: false,
        },
    }
}

/// What SQLQuack does with `sql`: its first diagnostic or refusal, with span, or `translated`.
fn everyday_outcome(sql: &str, arguments: &ConvertArguments) -> (String, Option<String>) {
    let diagnostics = check_source(sql).expect("the query holds a statement");
    diagnostics.first().map_or_else(
        || converted_outcome(sql, arguments),
        |found| {
            let stage = (found.code == PARSE_FAILURE_CODE).then_some("parse");
            let outcome = format!("{} {}", stage.unwrap_or("check"), found.code);
            (with_span(outcome, found.span.as_ref()), None)
        },
    )
}

/// The conversion's first refusal with its span, or `translated` with the converted SQL.
fn converted_outcome(sql: &str, arguments: &ConvertArguments) -> (String, Option<String>) {
    let report = convert_report(arguments, sql).expect("the query converts or refuses");
    report.refusals.first().map_or_else(
        || (TRANSLATED.to_owned(), Some(report.sql.clone())),
        |refusal| {
            let found = refusal.diagnostic.as_ref();
            let code = found.map_or("", |found| found.code);
            let outcome = format!("refused {code} {}", refusal.construct);
            (
                with_span(outcome, found.and_then(|found| found.span.as_ref())),
                None,
            )
        },
    )
}

/// `outcome` with the characters its diagnostic underlines, when it has a span.
fn with_span(outcome: String, span: Option<&DiagnosticSpan>) -> String {
    span.map_or_else(
        || outcome.clone(),
        |span| format!("{outcome} at {}..{}", span.start, span.end),
    )
}

/// The fixture rows `sql` returns, sorted and hashed, or the Snowflake error it raises.
fn fixture_result(fixture: &str, sql: &str) -> String {
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection
        .execute_batch(fixture)
        .expect("the fixture loads");
    let query = format!("SELECT COLUMNS(*)::VARCHAR FROM ({sql}) AS result");
    let rows = connection.prepare(&query).and_then(|mut statement| {
        statement
            .query_map([], |row| {
                let width = row.as_ref().column_count();
                (0..width)
                    .map(|index| {
                        row.get::<_, Option<String>>(index)
                            .map(|value| value.unwrap_or_else(|| "NULL".to_owned()))
                    })
                    .collect::<Result<Vec<String>, _>>()
                    .map(|values| values.join("\t"))
            })?
            .collect::<Result<Vec<String>, _>>()
    });
    rows.map_or_else(
        |error| {
            duckdb_failure(&error.to_string())
                .and_then(|failure| failure.snowflake)
                .map_or_else(
                    || "raises a DuckDB error".to_owned(),
                    |code| format!("raises {}", code.errno),
                )
        },
        |mut rows| {
            rows.sort();
            let digest = Sha256::digest(rows.join("\n").as_bytes());
            let hex: String = digest[..8]
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            format!("{} rows {hex}", rows.len())
        },
    )
}

/// The command that rewrites every corpus's recorded outcomes.
pub(crate) const RERECORD_COMMAND: &str = "make corpus-rerecord";

/// Set to rewrite the recorded outcomes instead of comparing with them.
const RERECORD_VARIABLE: &str = "SQLQUACK_RERECORD";

/// Each corpus directory's queries and their recorded outcomes.
const EVERYDAY_QUERIES: &str = "queries.toml";

/// DuckDB SQL creating and filling a corpus's relations, where it has one.
const EVERYDAY_FIXTURE: &str = "fixture.sql";

/// How a recorded refusal's outcome begins.
const REFUSED: &str = "refused ";

/// The outcome the basics generator writes, for `make corpus-rerecord` to fill in.
const UNRECORDED: &str = "unrecorded";

/// The policy each query is converted under: the built-in one.
const STRICT: &str = "strict";

/// The policy each query is converted under: the repository's, in a UTC session.
const REPOSITORY: &str = "repository";

/// The everyday corpus directory, under the repository root.
pub(crate) const EVERYDAY_CORPUS: &str = "corpora/everyday";

/// The generated corpus of basic statements, laid out as the everyday corpus.
pub(crate) const BASICS_CORPUS: &str = "corpora/basics";

/// The dbt-style model corpus directory, laid out as the everyday corpus.
pub(crate) const EVERYDAY_MODELS_CORPUS: &str = "corpora/everyday-models";

/// Corpus categories of statements no rule translates: DDL and DML.
const EVERYDAY_NOT_TRANSLATABLE: [&str; 1] = ["ddl_dml"];

/// The prefix of corpus categories of SQL Snowflake rejects.
const EVERYDAY_INVALID_PREFIX: &str = "invalid";

/// Snowflake's verdict on a query it compiles.
const SNOWFLAKE_COMPILES: &str = "compiles";

/// The outcome of a query that converts.
const TRANSLATED: &str = "translated";

/// The diagnostic code of SQL that does not parse.
const PARSE_FAILURE_CODE: &str = "SQ0001";
