use serde_json::json;

use crate::parser::tests::helpers::chained_condition;
use crate::parser::tests::helpers::kind_at;
use crate::parser::tests::helpers::parse_error;
use crate::parser::tests::helpers::value_at;
use crate::parser::tests::helpers::where_depth;
use crate::parser::tests::test_types::ChainDepthTestCase;
use crate::parser::tests::test_types::ParseErrorTestCase;
use crate::parser::tests::test_types::ParseKindTestCase;
use crate::parser::tests::test_types::ParseValueTestCase;
use crate::tokens::models::DialectType;

#[test]
fn given_queries_when_parsing_then_each_clause_has_its_node() {
    let test_cases = [
        ParseKindTestCase {
            description: "a bare SELECT is a select",
            dialect: None,
            sql: "SELECT 1",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "FROM lists its table",
            dialect: None,
            sql: "SELECT a, b FROM t",
            pointer: "/0/select/from/expressions/0",
            expected_kind: Ok("table"),
        },
        ParseKindTestCase {
            description: "WHERE holds its condition",
            dialect: None,
            sql: "SELECT * FROM t WHERE x = 1",
            pointer: "/0/select/where_clause/this",
            expected_kind: Ok("eq"),
        },
        ParseKindTestCase {
            description: "a join is parsed into the joins",
            dialect: None,
            sql: "SELECT * FROM a JOIN b ON a.id = b.id",
            pointer: "/0/select/joins/0/this",
            expected_kind: Ok("table"),
        },
        ParseKindTestCase {
            description: "multiplication binds tighter than addition",
            dialect: None,
            sql: "SELECT 1 + 2 * 3",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("add"),
        },
        ParseKindTestCase {
            description: "a CASE expression",
            dialect: None,
            sql: "SELECT CASE WHEN x = 1 THEN 'a' ELSE 'b' END",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("case"),
        },
        ParseKindTestCase {
            description: "INSERT",
            dialect: None,
            sql: "INSERT INTO t (a, b) VALUES (1, 2)",
            pointer: "/0",
            expected_kind: Ok("insert"),
        },
        ParseKindTestCase {
            description: "UPDATE",
            dialect: None,
            sql: "UPDATE t SET a = 1 WHERE b = 2",
            pointer: "/0",
            expected_kind: Ok("update"),
        },
        ParseKindTestCase {
            description: "DELETE",
            dialect: None,
            sql: "DELETE FROM t WHERE a = 1",
            pointer: "/0",
            expected_kind: Ok("delete"),
        },
        ParseKindTestCase {
            description: "a template variable is a parameter",
            dialect: None,
            sql: "SELECT ${x} FROM ${y} WHERE ${z} > 1",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("parameter"),
        },
        ParseKindTestCase {
            description: "an array subscript",
            dialect: None,
            sql: "SELECT arr[0]",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("subscript"),
        },
        ParseKindTestCase {
            description: "a subscript on a call result",
            dialect: None,
            sql: "SELECT SPLIT(name, ',')[0]",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("subscript"),
        },
        ParseKindTestCase {
            description: "a DuckDB string literal without AS stays a projection",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT 'item count'",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("literal"),
        },
        ParseKindTestCase {
            description: "a registered aggregate takes ORDER BY and LIMIT",
            dialect: Some(DialectType::Generic),
            sql: "SELECT ARBITRARY(x ORDER BY y LIMIT 1) FROM t",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            kind_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_function_calls_when_parsing_then_known_functions_become_typed_nodes() {
    let test_cases = [
        ParseKindTestCase {
            description: "COUNT(*)",
            dialect: None,
            sql: "SELECT COUNT(*)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("count"),
        },
        ParseKindTestCase {
            description: "an unknown function is generic",
            dialect: None,
            sql: "SELECT MY_CUSTOM_FUNC(name)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("function"),
        },
        ParseKindTestCase {
            description: "SUM",
            dialect: None,
            sql: "SELECT SUM(amount)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("sum"),
        },
        ParseKindTestCase {
            description: "a window function",
            dialect: None,
            sql: "SELECT ROW_NUMBER() OVER (PARTITION BY category ORDER BY id)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("window_function"),
        },
        ParseKindTestCase {
            description: "a window function with a frame",
            dialect: None,
            sql: "SELECT SUM(amount) OVER (ORDER BY date ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("window_function"),
        },
        ParseKindTestCase {
            description: "AVG",
            dialect: None,
            sql: "SELECT AVG(price)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("avg"),
        },
        ParseKindTestCase {
            description: "MIN",
            dialect: None,
            sql: "SELECT MIN(a), MAX(b)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("min"),
        },
        ParseKindTestCase {
            description: "MAX",
            dialect: None,
            sql: "SELECT MIN(a), MAX(b)",
            pointer: "/0/select/expressions/1",
            expected_kind: Ok("max"),
        },
        ParseKindTestCase {
            description: "STDDEV",
            dialect: None,
            sql: "SELECT STDDEV(x), VARIANCE(y)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("stddev"),
        },
        ParseKindTestCase {
            description: "VARIANCE",
            dialect: None,
            sql: "SELECT STDDEV(x), VARIANCE(y)",
            pointer: "/0/select/expressions/1",
            expected_kind: Ok("variance"),
        },
        ParseKindTestCase {
            description: "BOOL_AND",
            dialect: None,
            sql: "SELECT BOOL_AND(x), BOOL_OR(y), EVERY(z)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("logical_and"),
        },
        ParseKindTestCase {
            description: "BOOL_OR",
            dialect: None,
            sql: "SELECT BOOL_AND(x), BOOL_OR(y), EVERY(z)",
            pointer: "/0/select/expressions/1",
            expected_kind: Ok("logical_or"),
        },
        ParseKindTestCase {
            description: "EVERY is a logical AND",
            dialect: None,
            sql: "SELECT BOOL_AND(x), BOOL_OR(y), EVERY(z)",
            pointer: "/0/select/expressions/2",
            expected_kind: Ok("logical_and"),
        },
        ParseKindTestCase {
            description: "ROW_NUMBER inside OVER",
            dialect: None,
            sql: "SELECT ROW_NUMBER() OVER (ORDER BY id)",
            pointer: "/0/select/expressions/0/window_function/this",
            expected_kind: Ok("row_number"),
        },
        ParseKindTestCase {
            description: "RANK inside OVER",
            dialect: None,
            sql: "SELECT RANK() OVER (), DENSE_RANK() OVER ()",
            pointer: "/0/select/expressions/0/window_function/this",
            expected_kind: Ok("rank"),
        },
        ParseKindTestCase {
            description: "DENSE_RANK inside OVER",
            dialect: None,
            sql: "SELECT RANK() OVER (), DENSE_RANK() OVER ()",
            pointer: "/0/select/expressions/1/window_function/this",
            expected_kind: Ok("dense_rank"),
        },
        ParseKindTestCase {
            description: "LEAD inside OVER",
            dialect: None,
            sql: "SELECT LEAD(val, 1, 0) OVER (ORDER BY id)",
            pointer: "/0/select/expressions/0/window_function/this",
            expected_kind: Ok("lead"),
        },
        ParseKindTestCase {
            description: "NTILE inside OVER",
            dialect: None,
            sql: "SELECT NTILE(4) OVER (ORDER BY score)",
            pointer: "/0/select/expressions/0/window_function/this",
            expected_kind: Ok("n_tile"),
        },
        ParseKindTestCase {
            description: "CONTAINS",
            dialect: None,
            sql: "SELECT CONTAINS(name, 'test')",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("contains"),
        },
        ParseKindTestCase {
            description: "STARTS_WITH",
            dialect: None,
            sql: "SELECT STARTS_WITH(name, 'A')",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("starts_with"),
        },
        ParseKindTestCase {
            description: "ENDS_WITH",
            dialect: None,
            sql: "SELECT ENDS_WITH(name, 'z')",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("ends_with"),
        },
        ParseKindTestCase {
            description: "MOD",
            dialect: None,
            sql: "SELECT MOD(10, 3)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("mod_func"),
        },
        ParseKindTestCase {
            description: "RANDOM",
            dialect: None,
            sql: "SELECT RANDOM()",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("random"),
        },
        ParseKindTestCase {
            description: "RAND with a seed",
            dialect: None,
            sql: "SELECT RAND(42)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("rand"),
        },
        ParseKindTestCase {
            description: "SIN",
            dialect: None,
            sql: "SELECT SIN(x), COS(x), TAN(x)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("sin"),
        },
        ParseKindTestCase {
            description: "COS",
            dialect: None,
            sql: "SELECT SIN(x), COS(x), TAN(x)",
            pointer: "/0/select/expressions/1",
            expected_kind: Ok("cos"),
        },
        ParseKindTestCase {
            description: "TAN",
            dialect: None,
            sql: "SELECT SIN(x), COS(x), TAN(x)",
            pointer: "/0/select/expressions/2",
            expected_kind: Ok("tan"),
        },
        ParseKindTestCase {
            description: "YEAR",
            dialect: None,
            sql: "SELECT YEAR(date_col), MONTH(date_col), DAY(date_col)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("year"),
        },
        ParseKindTestCase {
            description: "MONTH",
            dialect: None,
            sql: "SELECT YEAR(date_col), MONTH(date_col), DAY(date_col)",
            pointer: "/0/select/expressions/1",
            expected_kind: Ok("month"),
        },
        ParseKindTestCase {
            description: "DAY",
            dialect: None,
            sql: "SELECT YEAR(date_col), MONTH(date_col), DAY(date_col)",
            pointer: "/0/select/expressions/2",
            expected_kind: Ok("day"),
        },
        ParseKindTestCase {
            description: "EPOCH",
            dialect: None,
            sql: "SELECT EPOCH(ts), EPOCH_MS(ts)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("epoch"),
        },
        ParseKindTestCase {
            description: "EPOCH_MS",
            dialect: None,
            sql: "SELECT EPOCH(ts), EPOCH_MS(ts)",
            pointer: "/0/select/expressions/1",
            expected_kind: Ok("epoch_ms"),
        },
        ParseKindTestCase {
            description: "ARRAY_LENGTH",
            dialect: None,
            sql: "SELECT ARRAY_LENGTH(arr)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("array_length"),
        },
        ParseKindTestCase {
            description: "ARRAY_CONTAINS",
            dialect: None,
            sql: "SELECT ARRAY_CONTAINS(arr, 1)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("array_contains"),
        },
        ParseKindTestCase {
            description: "EXPLODE",
            dialect: None,
            sql: "SELECT EXPLODE(arr)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("explode"),
        },
        ParseKindTestCase {
            description: "JSON_EXTRACT",
            dialect: None,
            sql: "SELECT JSON_EXTRACT(data, '$.name')",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("json_extract"),
        },
        ParseKindTestCase {
            description: "JSON_ARRAY_LENGTH",
            dialect: None,
            sql: "SELECT JSON_ARRAY_LENGTH(arr)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("json_array_length"),
        },
        ParseKindTestCase {
            description: "TO_JSON",
            dialect: None,
            sql: "SELECT TO_JSON(obj), PARSE_JSON(str)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("to_json"),
        },
        ParseKindTestCase {
            description: "PARSE_JSON",
            dialect: None,
            sql: "SELECT TO_JSON(obj), PARSE_JSON(str)",
            pointer: "/0/select/expressions/1",
            expected_kind: Ok("parse_json"),
        },
        ParseKindTestCase {
            description: "a JSON literal parses as JSON",
            dialect: None,
            sql: "SELECT JSON '\"foo\"'",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("parse_json"),
        },
        ParseKindTestCase {
            description: "MAP_KEYS",
            dialect: None,
            sql: "SELECT MAP_KEYS(m), MAP_VALUES(m)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("map_keys"),
        },
        ParseKindTestCase {
            description: "MAP_VALUES",
            dialect: None,
            sql: "SELECT MAP_KEYS(m), MAP_VALUES(m)",
            pointer: "/0/select/expressions/1",
            expected_kind: Ok("map_values"),
        },
        ParseKindTestCase {
            description: "ELEMENT_AT",
            dialect: None,
            sql: "SELECT ELEMENT_AT(m, 'key')",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("element_at"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            kind_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_ddl_and_pivots_when_parsing_then_each_statement_has_its_node() {
    let test_cases = [
        ParseKindTestCase {
            description: "CREATE TABLE",
            dialect: None,
            sql: "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100) NOT NULL)",
            pointer: "/0",
            expected_kind: Ok("create_table"),
        },
        ParseKindTestCase {
            description: "DROP TABLE",
            dialect: None,
            sql: "DROP TABLE IF EXISTS users CASCADE",
            pointer: "/0",
            expected_kind: Ok("drop_table"),
        },
        ParseKindTestCase {
            description: "ALTER TABLE",
            dialect: None,
            sql: "ALTER TABLE users ADD COLUMN email VARCHAR(255)",
            pointer: "/0",
            expected_kind: Ok("alter_table"),
        },
        ParseKindTestCase {
            description: "ADD COLUMN is an alter action",
            dialect: None,
            sql: "ALTER TABLE users ADD COLUMN email VARCHAR(255)",
            pointer: "/0/alter_table/actions/0",
            expected_kind: Ok("AddColumn"),
        },
        ParseKindTestCase {
            description: "DROP COLUMN is an alter action",
            dialect: None,
            sql: "ALTER TABLE users DROP COLUMN email",
            pointer: "/0/alter_table/actions/0",
            expected_kind: Ok("DropColumn"),
        },
        ParseKindTestCase {
            description: "CREATE INDEX",
            dialect: None,
            sql: "CREATE UNIQUE INDEX idx_email ON users (email)",
            pointer: "/0",
            expected_kind: Ok("create_index"),
        },
        ParseKindTestCase {
            description: "DROP INDEX",
            dialect: None,
            sql: "DROP INDEX IF EXISTS idx_email ON users",
            pointer: "/0",
            expected_kind: Ok("drop_index"),
        },
        ParseKindTestCase {
            description: "CREATE VIEW",
            dialect: None,
            sql: "CREATE VIEW active_users AS SELECT * FROM users WHERE active = 1",
            pointer: "/0",
            expected_kind: Ok("create_view"),
        },
        ParseKindTestCase {
            description: "DROP VIEW",
            dialect: None,
            sql: "DROP VIEW IF EXISTS active_users",
            pointer: "/0",
            expected_kind: Ok("drop_view"),
        },
        ParseKindTestCase {
            description: "TRUNCATE",
            dialect: None,
            sql: "TRUNCATE TABLE users CASCADE",
            pointer: "/0",
            expected_kind: Ok("truncate"),
        },
        ParseKindTestCase {
            description: "a simple DuckDB PIVOT statement",
            dialect: None,
            sql: "PIVOT Cities ON Year USING SUM(Population)",
            pointer: "/0",
            expected_kind: Ok("pivot"),
        },
        ParseKindTestCase {
            description: "a DuckDB PIVOT with GROUP BY",
            dialect: None,
            sql: "PIVOT Cities ON Year USING SUM(Population) GROUP BY Country",
            pointer: "/0",
            expected_kind: Ok("pivot"),
        },
        ParseKindTestCase {
            description: "a DuckDB UNPIVOT statement",
            dialect: None,
            sql: "UNPIVOT monthly_sales ON jan, feb, mar INTO NAME month VALUE sales",
            pointer: "/0",
            expected_kind: Ok("pivot"),
        },
        ParseKindTestCase {
            description: "a PIVOT in FROM",
            dialect: None,
            sql: "SELECT * FROM cities PIVOT(SUM(population) FOR year IN (2000, 2010, 2020))",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "WITH CUBE adds an empty cube",
            dialect: None,
            sql: "SELECT key, value FROM T1 GROUP BY key, value WITH CUBE",
            pointer: "/0/select/group_by/expressions/2",
            expected_kind: Ok("cube"),
        },
        ParseKindTestCase {
            description: "WITH ROLLUP adds an empty rollup",
            dialect: None,
            sql: "SELECT key, value FROM T1 GROUP BY key, value WITH ROLLUP",
            pointer: "/0/select/group_by/expressions/2",
            expected_kind: Ok("rollup"),
        },
        ParseKindTestCase {
            description: "a Snowflake ROLLUP list is a rollup, not a function call",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT a, b, SUM(c) FROM t GROUP BY ROLLUP (a, b)",
            pointer: "/0/select/group_by/expressions/0",
            expected_kind: Ok("rollup"),
        },
        ParseKindTestCase {
            description: "a Snowflake CUBE list is a cube, not a function call",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT a, b, SUM(c) FROM t GROUP BY CUBE (a, b)",
            pointer: "/0/select/group_by/expressions/0",
            expected_kind: Ok("cube"),
        },
        ParseKindTestCase {
            description: "Snowflake GROUPING SETS are grouping sets, not a function call",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT a, b, SUM(c) FROM t GROUP BY GROUPING SETS ((a), (b), ())",
            pointer: "/0/select/group_by/expressions/0",
            expected_kind: Ok("grouping_sets"),
        },
        ParseKindTestCase {
            description: "a ROLLUP nested in GROUPING SETS is a rollup",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT a, b FROM t GROUP BY GROUPING SETS (ROLLUP (a, b), (b))",
            pointer: "/0/select/group_by/expressions/0/grouping_sets/expressions/0",
            expected_kind: Ok("rollup"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            kind_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_statements_when_parsing_then_their_parts_are_kept() {
    let test_cases = [
        ParseValueTestCase {
            description: "a select keeps both projections",
            dialect: None,
            sql: "SELECT a, b FROM t",
            pointer: "/0/select/expressions/2",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "a join defaults to INNER",
            dialect: None,
            sql: "SELECT * FROM a JOIN b ON a.id = b.id",
            pointer: "/0/select/joins/0/kind",
            expected_value: Ok(json!("Inner")),
        },
        ParseValueTestCase {
            description: "a template variable keeps its name",
            dialect: None,
            sql: "SELECT ${x} FROM ${y} WHERE ${z} > 1",
            pointer: "/0/select/expressions/0/parameter/name",
            expected_value: Ok(json!("x")),
        },
        ParseValueTestCase {
            description: "a template variable keeps its style",
            dialect: None,
            sql: "SELECT ${x} FROM ${y} WHERE ${z} > 1",
            pointer: "/0/select/expressions/0/parameter/style",
            expected_value: Ok(json!("DollarBrace")),
        },
        ParseValueTestCase {
            description: "a DuckDB string projection keeps its text",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT 'item count'",
            pointer: "/0/select/expressions/0/literal/value",
            expected_value: Ok(json!("item count")),
        },
        ParseValueTestCase {
            description: "CREATE TABLE keeps its name",
            dialect: None,
            sql: "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100) NOT NULL)",
            pointer: "/0/create_table/name/name/name",
            expected_value: Ok(json!("users")),
        },
        ParseValueTestCase {
            description: "a column can be the primary key",
            dialect: None,
            sql: "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100) NOT NULL)",
            pointer: "/0/create_table/columns/0/primary_key",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "NOT NULL makes a column non-nullable",
            dialect: None,
            sql: "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100) NOT NULL)",
            pointer: "/0/create_table/columns/1/nullable",
            expected_value: Ok(json!(false)),
        },
        ParseValueTestCase {
            description: "IF NOT EXISTS is kept",
            dialect: None,
            sql: "CREATE TABLE IF NOT EXISTS t (id INT)",
            pointer: "/0/create_table/if_not_exists",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "TEMPORARY is kept",
            dialect: None,
            sql: "CREATE TEMPORARY TABLE t (id INT)",
            pointer: "/0/create_table/temporary",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "DROP TABLE keeps IF EXISTS",
            dialect: None,
            sql: "DROP TABLE IF EXISTS users CASCADE",
            pointer: "/0/drop_table/if_exists",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "DROP TABLE keeps CASCADE",
            dialect: None,
            sql: "DROP TABLE IF EXISTS users CASCADE",
            pointer: "/0/drop_table/cascade",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "DROP TABLE names one table",
            dialect: None,
            sql: "DROP TABLE IF EXISTS users CASCADE",
            pointer: "/0/drop_table/names/1",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "ALTER TABLE has one action",
            dialect: None,
            sql: "ALTER TABLE users ADD COLUMN email VARCHAR(255)",
            pointer: "/0/alter_table/actions/1",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "CREATE INDEX keeps UNIQUE",
            dialect: None,
            sql: "CREATE UNIQUE INDEX idx_email ON users (email)",
            pointer: "/0/create_index/unique",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "CREATE INDEX keeps its name",
            dialect: None,
            sql: "CREATE UNIQUE INDEX idx_email ON users (email)",
            pointer: "/0/create_index/name/name",
            expected_value: Ok(json!("idx_email")),
        },
        ParseValueTestCase {
            description: "CREATE INDEX keeps its table",
            dialect: None,
            sql: "CREATE UNIQUE INDEX idx_email ON users (email)",
            pointer: "/0/create_index/table/name/name",
            expected_value: Ok(json!("users")),
        },
        ParseValueTestCase {
            description: "CREATE INDEX has one column",
            dialect: None,
            sql: "CREATE UNIQUE INDEX idx_email ON users (email)",
            pointer: "/0/create_index/columns/1",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "DROP INDEX keeps IF EXISTS",
            dialect: None,
            sql: "DROP INDEX IF EXISTS idx_email ON users",
            pointer: "/0/drop_index/if_exists",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "DROP INDEX keeps an unquoted name",
            dialect: None,
            sql: "DROP INDEX IF EXISTS idx_email ON users",
            pointer: "/0/drop_index/name/name",
            expected_value: Ok(
                json!({"name": "idx_email", "quoted": false, "trailing_comments": [], "span": {"start": 21, "end": 30, "line": 1, "column": 31}}),
            ),
        },
        ParseValueTestCase {
            description: "DROP INDEX keeps its table",
            dialect: None,
            sql: "DROP INDEX IF EXISTS idx_email ON users",
            pointer: "/0/drop_index/table/name/name",
            expected_value: Ok(json!("users")),
        },
        ParseValueTestCase {
            description: "DROP INDEX keeps a quoted schema",
            dialect: None,
            sql: "DROP INDEX IF EXISTS \"public\".\"IdxEmail\"",
            pointer: "/0/drop_index/name/schema/quoted",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "DROP INDEX keeps a quoted schema name",
            dialect: None,
            sql: "DROP INDEX IF EXISTS \"public\".\"IdxEmail\"",
            pointer: "/0/drop_index/name/schema/name",
            expected_value: Ok(json!("public")),
        },
        ParseValueTestCase {
            description: "DROP INDEX keeps a quoted name",
            dialect: None,
            sql: "DROP INDEX IF EXISTS \"public\".\"IdxEmail\"",
            pointer: "/0/drop_index/name/name/quoted",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "DROP INDEX keeps a quoted name's case",
            dialect: None,
            sql: "DROP INDEX IF EXISTS \"public\".\"IdxEmail\"",
            pointer: "/0/drop_index/name/name/name",
            expected_value: Ok(json!("IdxEmail")),
        },
        ParseValueTestCase {
            description: "MATERIALIZED is kept",
            dialect: None,
            sql: "CREATE MATERIALIZED VIEW stats AS SELECT COUNT(*) FROM users",
            pointer: "/0/create_view/materialized",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "TRUNCATE keeps CASCADE",
            dialect: None,
            sql: "TRUNCATE TABLE users CASCADE",
            pointer: "/0/truncate/cascade",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "COUNT keeps DISTINCT",
            dialect: None,
            sql: "SELECT COUNT(DISTINCT user_id)",
            pointer: "/0/select/expressions/0/count/distinct",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "COUNT of a column is not a star count",
            dialect: None,
            sql: "SELECT COUNT(DISTINCT user_id)",
            pointer: "/0/select/expressions/0/count/star",
            expected_value: Ok(json!(false)),
        },
        ParseValueTestCase {
            description: "LEAD keeps its offset",
            dialect: None,
            sql: "SELECT LEAD(val, 1, 0) OVER (ORDER BY id)",
            pointer: "/0/select/expressions/0/window_function/this/lead/offset/literal/value",
            expected_value: Ok(json!("1")),
        },
        ParseValueTestCase {
            description: "LEAD keeps its default",
            dialect: None,
            sql: "SELECT LEAD(val, 1, 0) OVER (ORDER BY id)",
            pointer: "/0/select/expressions/0/window_function/this/lead/default/literal/value",
            expected_value: Ok(json!("0")),
        },
        ParseValueTestCase {
            description: "EXCLUDE keeps its first column",
            dialect: None,
            sql: "SELECT * EXCLUDE (col1, col2) FROM t",
            pointer: "/0/select/expressions/0/star/except/0/name",
            expected_value: Ok(json!("col1")),
        },
        ParseValueTestCase {
            description: "EXCLUDE keeps its second column",
            dialect: None,
            sql: "SELECT * EXCLUDE (col1, col2) FROM t",
            pointer: "/0/select/expressions/0/star/except/1/name",
            expected_value: Ok(json!("col2")),
        },
        ParseValueTestCase {
            description: "EXCLUDE has two columns",
            dialect: None,
            sql: "SELECT * EXCLUDE (col1, col2) FROM t",
            pointer: "/0/select/expressions/0/star/except/2",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "EXCEPT is EXCLUDE",
            dialect: None,
            sql: "SELECT * EXCEPT (id, created_at) FROM t",
            pointer: "/0/select/expressions/0/star/except/1/name",
            expected_value: Ok(json!("created_at")),
        },
        ParseValueTestCase {
            description: "a qualified star keeps its table",
            dialect: None,
            sql: "SELECT t.* EXCLUDE (col1) FROM t",
            pointer: "/0/select/expressions/0/star/table/name",
            expected_value: Ok(json!("t")),
        },
        ParseValueTestCase {
            description: "a qualified star keeps EXCLUDE",
            dialect: None,
            sql: "SELECT t.* EXCLUDE (col1) FROM t",
            pointer: "/0/select/expressions/0/star/except/0/name",
            expected_value: Ok(json!("col1")),
        },
        ParseValueTestCase {
            description: "REPLACE keeps its alias",
            dialect: None,
            sql: "SELECT * REPLACE (UPPER(name) AS name) FROM t",
            pointer: "/0/select/expressions/0/star/replace/0/alias/name",
            expected_value: Ok(json!("name")),
        },
        ParseValueTestCase {
            description: "REPLACE has one entry",
            dialect: None,
            sql: "SELECT * REPLACE (UPPER(name) AS name) FROM t",
            pointer: "/0/select/expressions/0/star/replace/1",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "REPLACE keeps every entry",
            dialect: None,
            sql: "SELECT * REPLACE (a + 1 AS a, b * 2 AS b) FROM t",
            pointer: "/0/select/expressions/0/star/replace/1/alias/name",
            expected_value: Ok(json!("b")),
        },
        ParseValueTestCase {
            description: "RESAMPLE keeps its USING column",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT * FROM t1 RESAMPLE (USING ts INCREMENT BY INTERVAL '1 hour')",
            pointer: "/0/select/from/expressions/0/resample/using/name",
            expected_value: Ok(json!("ts")),
        },
        ParseValueTestCase {
            description: "RESAMPLE keeps the table it resamples",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT * FROM t1 RESAMPLE (USING ts INCREMENT BY INTERVAL '1 hour')",
            pointer: "/0/select/from/expressions/0/resample/this/table/name/name",
            expected_value: Ok(json!("t1")),
        },
        ParseValueTestCase {
            description: "RESAMPLE follows the table's alias",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT * FROM t1 AS a RESAMPLE (USING ts INCREMENT BY INTERVAL '1 hour' PARTITION BY k1, k2)",
            pointer: "/0/select/from/expressions/0/resample/this/table/alias/name",
            expected_value: Ok(json!("a")),
        },
        ParseValueTestCase {
            description: "RESAMPLE keeps every PARTITION BY expression",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT * FROM t1 AS a RESAMPLE (USING ts INCREMENT BY INTERVAL '1 hour' PARTITION BY k1, k2)",
            pointer: "/0/select/from/expressions/0/resample/partition_by/1/column/name/name",
            expected_value: Ok(json!("k2")),
        },
        ParseValueTestCase {
            description: "RESAMPLE keeps a metadata column's alias",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT * FROM t1 RESAMPLE (USING ts INCREMENT BY INTERVAL '1 hour' METADATA_COLUMNS IS_GENERATED() AS g, BUCKET_START() b)",
            pointer: "/0/select/from/expressions/0/resample/metadata_columns/1/1/name",
            expected_value: Ok(json!("b")),
        },
        ParseValueTestCase {
            description: "ILIKE keeps its pattern on the star",
            dialect: None,
            sql: "SELECT * ILIKE '%_ID' FROM orders",
            pointer: "/0/select/expressions/0/star/ilike",
            expected_value: Ok(json!("%_ID")),
        },
        ParseValueTestCase {
            description: "ILIKE keeps the pattern's value, not its quoting",
            dialect: None,
            sql: "SELECT * ILIKE 'it''s%' FROM t",
            pointer: "/0/select/expressions/0/star/ilike",
            expected_value: Ok(json!("it's%")),
        },
        ParseValueTestCase {
            description: "ILIKE on a qualified star keeps the qualifier",
            dialect: None,
            sql: "SELECT t.* ILIKE 'a%' REPLACE (1 AS a1) RENAME (b1 AS c1) FROM t",
            pointer: "/0/select/expressions/0/star/ilike",
            expected_value: Ok(json!("a%")),
        },
        ParseValueTestCase {
            description: "ILIKE is followed by REPLACE and RENAME",
            dialect: None,
            sql: "SELECT t.* ILIKE 'a%' REPLACE (1 AS a1) RENAME (b1 AS c1) FROM t",
            pointer: "/0/select/expressions/0/star/rename/0/1/name",
            expected_value: Ok(json!("c1")),
        },
        ParseValueTestCase {
            description: "a star without ILIKE has no pattern",
            dialect: None,
            sql: "SELECT * EXCLUDE (id) FROM t",
            pointer: "/0/select/expressions/0/star/ilike",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "RENAME keeps the old name",
            dialect: None,
            sql: "SELECT * RENAME (old_col AS new_col, x AS y) FROM t",
            pointer: "/0/select/expressions/0/star/rename/0/0/name",
            expected_value: Ok(json!("old_col")),
        },
        ParseValueTestCase {
            description: "RENAME keeps the new name",
            dialect: None,
            sql: "SELECT * RENAME (old_col AS new_col, x AS y) FROM t",
            pointer: "/0/select/expressions/0/star/rename/0/1/name",
            expected_value: Ok(json!("new_col")),
        },
        ParseValueTestCase {
            description: "RENAME keeps every pair",
            dialect: None,
            sql: "SELECT * RENAME (old_col AS new_col, x AS y) FROM t",
            pointer: "/0/select/expressions/0/star/rename/1/1/name",
            expected_value: Ok(json!("y")),
        },
        ParseValueTestCase {
            description: "EXCLUDE combines with REPLACE: exclude",
            dialect: None,
            sql: "SELECT * EXCLUDE (id) REPLACE (name || '!' AS name) FROM t",
            pointer: "/0/select/expressions/0/star/except/0/name",
            expected_value: Ok(json!("id")),
        },
        ParseValueTestCase {
            description: "EXCLUDE combines with REPLACE: replace",
            dialect: None,
            sql: "SELECT * EXCLUDE (id) REPLACE (name || '!' AS name) FROM t",
            pointer: "/0/select/expressions/0/star/replace/0/alias/name",
            expected_value: Ok(json!("name")),
        },
        ParseValueTestCase {
            description: "a geometry keeps its subtype and SRID",
            dialect: None,
            sql: "CREATE TABLE t (geom GEOMETRY(Point, 4326))",
            pointer: "/0/create_table/columns/0/data_type",
            expected_value: Ok(json!({"data_type": "geometry", "subtype": "POINT", "srid": 4326})),
        },
        ParseValueTestCase {
            description: "a bare geography has no subtype or SRID",
            dialect: None,
            sql: "CREATE TABLE t (loc GEOGRAPHY)",
            pointer: "/0/create_table/columns/0/data_type",
            expected_value: Ok(json!({"data_type": "geography", "subtype": null, "srid": null})),
        },
        ParseValueTestCase {
            description: "a geometry subtype without SRID",
            dialect: None,
            sql: "CREATE TABLE t (geom GEOMETRY(LineString))",
            pointer: "/0/create_table/columns/0/data_type",
            expected_value: Ok(
                json!({"data_type": "geometry", "subtype": "LINESTRING", "srid": null}),
            ),
        },
        ParseValueTestCase {
            description: "POINT is a point geometry",
            dialect: None,
            sql: "CREATE TABLE t (pt POINT)",
            pointer: "/0/create_table/columns/0/data_type",
            expected_value: Ok(json!({"data_type": "geometry", "subtype": "POINT", "srid": null})),
        },
        ParseValueTestCase {
            description: "a simple PIVOT is not an unpivot",
            dialect: None,
            sql: "PIVOT Cities ON Year USING SUM(Population)",
            pointer: "/0/pivot/unpivot",
            expected_value: Ok(json!(false)),
        },
        ParseValueTestCase {
            description: "a simple PIVOT keeps its ON expression",
            dialect: None,
            sql: "PIVOT Cities ON Year USING SUM(Population)",
            pointer: "/0/pivot/expressions/0/column/name/name",
            expected_value: Ok(json!("Year")),
        },
        ParseValueTestCase {
            description: "a simple PIVOT keeps its USING expression",
            dialect: None,
            sql: "PIVOT Cities ON Year USING SUM(Population)",
            pointer: "/0/pivot/using/0/sum/name",
            expected_value: Ok(json!("SUM")),
        },
        ParseValueTestCase {
            description: "an empty cube has no expressions",
            dialect: None,
            sql: "SELECT key, value FROM T1 GROUP BY key, value WITH CUBE",
            pointer: "/0/select/group_by/expressions/2/cube/expressions",
            expected_value: Ok(json!([])),
        },
        ParseValueTestCase {
            description: "an empty rollup has no expressions",
            dialect: None,
            sql: "SELECT key, value FROM T1 GROUP BY key, value WITH ROLLUP",
            pointer: "/0/select/group_by/expressions/2/rollup/expressions",
            expected_value: Ok(json!([])),
        },
        ParseValueTestCase {
            description: "a Snowflake ROLLUP keeps its keys in order",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT a, b, SUM(c) FROM t GROUP BY ROLLUP (a, b)",
            pointer: "/0/select/group_by/expressions/0/rollup/expressions/1/column/name/name",
            expected_value: Ok(json!("b")),
        },
        ParseValueTestCase {
            description: "Snowflake GROUPING SETS keep the empty set",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT a, b, SUM(c) FROM t GROUP BY GROUPING SETS ((a), (b), ())",
            pointer: "/0/select/group_by/expressions/0/grouping_sets/expressions/2/tuple/expressions",
            expected_value: Ok(json!([])),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            value_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_malformed_aliases_when_parsing_duckdb_then_parsing_fails() {
    let test_cases = [
        ParseErrorTestCase {
            description: "DuckDB needs AS before a string alias",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT 1 'item count'",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "DuckDB needs AS before a string alias after a call",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT COUNT(*) 'item count' FROM events",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "AS needs an alias",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT 1 AS",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "an alias cannot be a number",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT 1 AS 123",
            expected_error: "Parse error",
        },
    ];

    for test_case in test_cases {
        let error = parse_error(test_case.dialect, test_case.sql);
        assert!(
            error.contains(test_case.expected_error),
            "{}: {error}",
            test_case.description
        );
    }
}

#[test]
fn given_long_boolean_chains_when_parsing_then_the_tree_stays_balanced() {
    let test_cases = [
        ChainDepthTestCase {
            description: "a long AND chain is balanced",
            connector: "AND",
            terms: 4096,
            expected_depth_below: 128,
        },
        ChainDepthTestCase {
            description: "a long OR chain is balanced",
            connector: "OR",
            terms: 4096,
            expected_depth_below: 128,
        },
    ];

    for test_case in test_cases {
        let depth = where_depth(&chained_condition(test_case.connector, test_case.terms));
        assert!(
            depth < test_case.expected_depth_below,
            "{}: depth {depth}",
            test_case.description
        );
    }
}

#[test]
fn given_a_snowflake_cte_whose_body_is_a_bare_values_list_when_parsing_then_parsing_fails() {
    let test_cases = [ParseErrorTestCase {
        description: "Snowflake takes VALUES only in FROM, not as a CTE body",
        dialect: Some(DialectType::Snowflake),
        sql: "WITH v AS (VALUES (1), (2)) SELECT v.column1 FROM v",
        expected_error: "Snowflake does not accept a bare VALUES list as the body of CTE 'v'",
    }];
    for test_case in test_cases {
        let error = parse_error(test_case.dialect, test_case.sql);
        assert!(
            error.contains(test_case.expected_error),
            "{}: {error}",
            test_case.description
        );
    }
}
