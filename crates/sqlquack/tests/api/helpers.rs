use crate::test_types;

pub(crate) fn analyze_sql(test_case: &test_types::AnalyzeTestCase) -> sqlquack::AnalysisResult {
    sqlquack::analyze(
        sqlquack::SqlUnit {
            sql: test_case.sql.to_owned(),
        },
        sqlquack::AnalyzeOptions {
            dialect: sqlquack::Dialect::Generic,
        },
    )
}
