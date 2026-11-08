use std::path::Path;

use serde_json::Value;

use crate::columns::main::compare_columns::compare_columns;
use crate::columns::tests::test_types;

/// Lets DuckDB 2.0 bind `->` lambdas, which it otherwise refuses at bind.
const SINGLE_ARROW_LAMBDAS: &str = "SET lambda_syntax = 'ENABLE_SINGLE_ARROW'";
/// The format corpus, beside the formatter's own corpus test.
const FORMAT_CORPUS: &str = "../sqlquack-format/src/format/tests/fixtures/format_corpus.json";

#[test]
fn given_duckdb_queries_when_laid_out_then_duckdb_binds_the_same_output_columns() {
    let test_cases = [
        test_types::CompareColumnsTestCase {
            description: "DuckDB corpus subset",
            corpus_dialect: "duckdb",
            queries: &[],
            expected_bound: 188,
            expected_unbound: 190,
            expected_refused: 0,
        },
        test_types::CompareColumnsTestCase {
            description: "operator, spacing and keyword-case inputs over inline relations",
            corpus_dialect: "",
            queries: &[
                "select a+b, a-b, a*b, a=b, a<>b, a!=b, a<=b, a>=b, a<b, a>b from (select 1 as a, 2 as b)",
                "select s||t, upper( s ), lower(s), - 1, -1, - -1, 1e-3, 1E+5, .5e-2 from (select 'x' as s, 'y' as t)",
                "select count(*), Count( distinct a ), max( a ), sum(a)+1 from (select 1 as a)",
                "select case when a=1 then 'x' else 'y' end, a between 1 and 2, a in (1,2), not a=1, a is null from (select 1 as a)",
                "select cast(a as varchar), a::int, cast( a AS bigint ) from (select 1 as a)",
                "select [1,2][1], {'k':1}, [ 1 ], [1,2,3][1:2] from (select 1)",
                "select a+b\n  + 3 from (select 1 as a, 2 as b)",
                "select \"a+b\" from (select a+b from (select 1 as a, 2 as b))",
                "select \"(a + b)\" from (select a+b from (select 1 as a, 2 as b))",
                "select distinct A from (select 1 as a) where a=1 order by a",
                "select 1 x, 2 AS y, coalesce( a , 0 ) from (select null as a)",
                "select list_transform([1,2], x -> x+1), list_filter([1,2], lambda x: x>1)",
                "select interval 1 day, date '2024-01-01' + interval 1 day, current_date",
                "select t.a, t . a, t.* from (select 1 as a) as t",
                "select a+b as c, a+b c from (select 1 as a, 2 as b)",
                "with q as (select a*2 from (select 1 as a)) select * from q",
                "select * from (select a+b from (select 1 as a, 2 as b)) union all select 3",
                "select (a), ((a+b)), (select 1) from (select 1 as a, 2 as b)",
                "select a % b, a // b, a / b, 2 ** 3, a & b, a | b, a << 1 from (select 7 as a, 2 as b)",
                "select upper('a') || lower('B'), 'a' like 'a%', 'a' ilike 'A', 'a' similar to 'a' from (select 1)",
                "select sum(a) over (order by a), row_number() over () from (select 1 as a)",
                "select a::varchar || 'x', a::int+1 from (select 1 as a)",
                "select * from (select a+b, a*b from (select 1 as a, 2 as b))",
                "select - @a from (select -2 as a)",
                "select + @a from (select -2 as a)",
                "select @ +1",
                "select @ -1",
                "select @ - a from (select 2 as a)",
                "select a - @b, @a, @(-1) from (select 1 as a, -2 as b)",
            ],
            expected_bound: 42,
            expected_unbound: 4,
            expected_refused: 12,
        },
    ];
    let corpus_text =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(FORMAT_CORPUS))
            .unwrap_or_default();
    let corpus: Vec<Value> = serde_json::from_str(&corpus_text).unwrap_or_default();
    let connection = duckdb::Connection::open_in_memory().and_then(|connection| {
        connection
            .execute_batch(SINGLE_ARROW_LAMBDAS)
            .map(|()| connection)
    });
    for test_case in test_cases {
        let queries: Vec<&str> = corpus
            .iter()
            .filter(|entry| entry["dialect"] == test_case.corpus_dialect)
            .filter_map(|entry| entry["sql"].as_str())
            .chain(test_case.queries.iter().copied())
            .collect();
        let outcome = connection
            .as_ref()
            .map(|connection| compare_columns(connection, &queries))
            .map_err(ToString::to_string);
        assert_eq!(
            outcome.as_ref().map(|outcome| (
                outcome.bound,
                outcome.unbound,
                outcome.refused,
                outcome.changes.len()
            )),
            Ok((
                test_case.expected_bound,
                test_case.expected_unbound,
                test_case.expected_refused,
                0
            )),
            "{}: {}",
            test_case.description,
            outcome
                .as_ref()
                .map(|outcome| outcome.changes.join("\n\n"))
                .unwrap_or_default()
        );
    }
}
