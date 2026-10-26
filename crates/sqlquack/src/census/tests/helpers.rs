use std::path::Path;

use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_rules::conversion::models::{ConversionOptions, Schema};

use crate::census::_helpers::coverage::{CoverageInput, greedy_order};
use crate::census::main::render_census_text::render_census_text;
use crate::census::main::run_census::run_census;
use crate::census::models::{CensusOptions, CensusReport};
use crate::census::tests::test_types::{CensusSummary, ShopFile};

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
    run_census(
        &[directory.path().to_path_buf()],
        &CensusOptions {
            dialect: SourceDialect::Snowflake,
            coverage_points: vec![1, 5],
            schema: Schema::default(),
            chain_models: false,
            conversion: ConversionOptions::default(),
        },
    )
    .map(|report| summarise(&report))
    .map_err(|error| error.to_string())
}

pub(crate) fn greedy(files: &[Option<Vec<usize>>], frequency_order: &[usize]) -> Vec<usize> {
    greedy_order(&CoverageInput {
        files,
        construct_count: frequency_order.len(),
        frequency_order,
    })
}
