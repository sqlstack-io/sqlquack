use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;

use serde_json::Value;

use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::guard::models::ComplexityGuardOptions;
use crate::guard::models::ParseOptions;
use crate::parser::models::Dialect;
use crate::parser::models::Parser;
use crate::parser::models::ParserConfig;
use crate::parser::tests::test_types::Decision;
use crate::tokens::models::DialectType;
use crate::tokens::models::Span;
use crate::tokens::models::Token;
use crate::tokens::models::TokenType;
use crate::tokens::models::Tokenizer;
use crate::traversal::types::ExpressionWalk;

pub(crate) fn parsed(dialect: Option<DialectType>, sql: &str) -> Result<Value, String> {
    dialect
        .map_or_else(
            || Parser::parse_sql(sql),
            |dialect| Dialect::get(dialect).parse(sql),
        )
        .map(|statements| serde_json::to_value(statements).expect("syntax trees serialize"))
        .map_err(|error| error.to_string())
}

pub(crate) fn value_at(
    dialect: Option<DialectType>,
    sql: &str,
    pointer: &str,
) -> Result<Value, String> {
    parsed(dialect, sql)
        .map(|statements| statements.pointer(pointer).cloned().unwrap_or(Value::Null))
}

pub(crate) fn kind_at(
    dialect: Option<DialectType>,
    sql: &str,
    pointer: &str,
) -> Result<String, String> {
    value_at(dialect, sql, pointer).map(|node| {
        node.as_object()
            .and_then(|fields| fields.keys().next().cloned())
            .unwrap_or_default()
    })
}

pub(crate) fn parse_error(dialect: Option<DialectType>, sql: &str) -> String {
    parsed(dialect, sql)
        .err()
        .unwrap_or_else(|| format!("{sql} parsed"))
}

pub(crate) fn chained_condition(connector: &str, terms: usize) -> String {
    (1..terms).fold(String::from("SELECT 1 WHERE c0 = 0"), |sql, index| {
        format!("{sql} {connector} c{index} = {index}")
    })
}

pub(crate) fn where_depth(sql: &str) -> usize {
    Parser::parse_sql(sql).expect("the query parses")[0]
        .as_select()
        .and_then(|select| select.where_clause.as_ref())
        .map_or(0, |where_clause| where_clause.this.tree_depth())
}

const TERMINATION_BUDGET: Duration = Duration::from_secs(5);
const PARSE_STACK: usize = 16 * 1024 * 1024;
const DEPTH_GUARD_CODE: &str = "E_GUARD_PARSER_DEPTH_EXCEEDED";
const TERMINATION_STATEMENTS: &[&str] = &[
    "CREATE TABLE t (a VARCHAR2(2328 CHAR), b DATETIME2(2))",
    "SELECT CAST(x AS UserDefinedType(1, 2 CHAR)) FROM t",
    "ALTER TABLE t ADD COLUMN c NVARCHAR2(100)",
    "SELECT a.:CustomType(1, 2) FROM t",
    "SELECT JSON_VALUE(a, '$.b' RETURNING VARCHAR2(10)) FROM t",
    "SELECT x::DECIMAL(10, 2) FROM t",
    "SELECT CAST(x AS DECIMAL(10, 2)) FROM t",
    "SELECT CAST(x AS ARRAY<STRUCT<a INT, b TEXT>>) FROM t",
    "SELECT f(a, b, c) FROM t",
    "SELECT * EXCEPT (a, b) FROM t",
    "SELECT * FROM t WITH (NOLOCK, INDEX(i))",
    "SELECT a[1], b:c FROM t",
    "SELECT COUNT(*) OVER (PARTITION BY a ORDER BY b) FROM t",
    "CREATE INDEX i ON t (a ASC, b DESC)",
    "GRANT SELECT, INSERT ON t TO u",
    "INSERT INTO t (a, b) VALUES (1, 2)",
    "WITH c AS (SELECT 1 AS a) SELECT a FROM c",
    "SELECT CASE WHEN a THEN 1 ELSE 2 END FROM t",
];

pub(crate) fn decision(sql: &str) -> Decision {
    let (sender, receiver) = mpsc::channel();
    let input = sql.to_owned();
    std::thread::Builder::new()
        .stack_size(PARSE_STACK)
        .spawn(move || {
            let parse = std::panic::AssertUnwindSafe(|| Parser::parse_sql(&input));
            let outcome = std::panic::catch_unwind(parse)
                .map(|parsed| parsed.map(|_| ()).map_err(|error| error.to_string()));
            sender.send(outcome.map_err(|_| ())).ok();
        })
        .expect("the parse thread starts");
    receiver
        .recv_timeout(TERMINATION_BUDGET)
        .map_or(Decision::OverBudget, |outcome| {
            outcome.map_or(Decision::Panicked, |parsed| {
                parsed.map_or_else(
                    |message| Decision::Rejected {
                        by_depth_guard: message.contains(DEPTH_GUARD_CODE),
                    },
                    |()| Decision::Parsed,
                )
            })
        })
}

pub(crate) fn decisions_outside(
    inputs: &[String],
    allowed: &[Decision],
) -> Vec<(String, Decision)> {
    inputs
        .iter()
        .map(|sql| (sql.clone(), decision(sql)))
        .filter(|(_, decided)| !allowed.contains(decided))
        .collect()
}

pub(crate) fn truncated_prefixes() -> Vec<String> {
    TERMINATION_STATEMENTS
        .iter()
        .flat_map(|sql| {
            (1..=sql.len())
                .filter(|end| sql.is_char_boundary(*end))
                .map(|end| sql[..end].to_owned())
        })
        .collect()
}

pub(crate) fn if_chains(links: usize, tail: &str) -> Vec<String> {
    ["~", "+", "-"]
        .iter()
        .map(|separator| format!("{}{tail}", format!("IF{separator}").repeat(links)))
        .collect()
}

pub(crate) fn depth_exhaustion_inputs() -> Vec<String> {
    let if_chains = ["", "SELECT "].iter().flat_map(|prefix| {
        ["~", "+", "-"].iter().flat_map(move |separator| {
            ["I?{", "1"].iter().map(move |tail| {
                format!("{prefix}{}{tail}", format!("IF{separator}").repeat(4_000))
            })
        })
    });
    let prefixes = ["~ ", "+ ", "- ", "NOT "]
        .iter()
        .map(|prefix| format!("SELECT {}1", prefix.repeat(4_000)));
    let nested_type = std::iter::once(format!(
        "SELECT CAST(x AS {}INT{})",
        "ARRAY<".repeat(4_000),
        ">".repeat(4_000)
    ));
    if_chains.chain(prefixes).chain(nested_type).collect()
}

pub(crate) fn depth_scope_trace(max_parser_depth: usize) -> (usize, bool, usize, bool, bool, bool) {
    let mut parser = Parser::with_config(
        Tokenizer::default()
            .tokenize("SELECT 1")
            .expect("the query tokenizes"),
        ParserConfig {
            complexity_guard: ComplexityGuardOptions {
                max_parser_depth: Some(max_parser_depth),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let outer = parser.enter_parser_depth(2).expect("two levels fit");
    let child = parser
        .tidb_fragment_parser("AUTO_RANDOM")
        .expect("the child parser starts");
    let scope = child.enter_parser_depth(1).expect("the third level fits");
    let inside = parser.recursion.depth.load(Ordering::Relaxed);
    drop(scope);
    let child_refused = child
        .enter_parser_depth(2)
        .err()
        .is_some_and(|error| error.to_string().contains(DEPTH_GUARD_CODE));
    drop(child);
    drop(outer);
    let after = parser.recursion.depth.load(Ordering::Relaxed);
    let statement_refused = parser
        .parse_statement()
        .err()
        .is_some_and(|error| error.to_string().contains(DEPTH_GUARD_CODE));
    parser.current = 0;
    let disjunction_refused = parser
        .parse_disjunction()
        .err()
        .is_some_and(|error| error.to_string().contains(DEPTH_GUARD_CODE));
    (
        inside,
        child_refused,
        after,
        statement_refused,
        disjunction_refused,
        Parser::parse_sql("SELECT 1").is_ok(),
    )
}

const EOF_PART: &str = "<EOF>";

pub(crate) type FragmentParser =
    fn(&mut Parser) -> crate::tokens::types::Result<Option<Expression>>;

pub(crate) const FRAGMENT_PARSERS: &[(&str, FragmentParser)] = &[
    ("disjunction", Parser::parse_disjunction),
    ("conjunction", Parser::parse_conjunction),
    ("select_or_expression", Parser::parse_select_or_expression),
    ("value", Parser::parse_value),
    ("set_item_assignment", Parser::parse_set_item_assignment),
];

pub(crate) fn eof() -> Token {
    Token::new(TokenType::Eof, "", Span::default())
}

pub(crate) fn stream(parts: &[&str]) -> Vec<Token> {
    parts
        .iter()
        .flat_map(|part| {
            part.strip_prefix(EOF_PART).map_or_else(
                || {
                    Tokenizer::default()
                        .tokenize(part)
                        .expect("the part tokenizes")
                },
                |_| vec![eof()],
            )
        })
        .collect()
}

fn new_parser(tokens: Vec<Token>, _: &str) -> Parser {
    Parser::new(tokens)
}

fn configured_parser(tokens: Vec<Token>, _: &str) -> Parser {
    Parser::with_config(tokens, ParserConfig::default())
}

fn sourced_parser(tokens: Vec<Token>, source: &str) -> Parser {
    Parser::with_source(tokens, ParserConfig::default(), source.to_owned())
}

type ParserConstructor = fn(Vec<Token>, &str) -> Parser;

const CONSTRUCTORS: &[(&str, ParserConstructor)] = &[
    ("new", new_parser),
    ("with_config", configured_parser),
    ("with_source", sourced_parser),
];

pub(crate) fn nonempty_fragments(parts: &[&str]) -> Vec<String> {
    let tokens = stream(parts);
    FRAGMENT_PARSERS
        .iter()
        .flat_map(|(name, fragment)| {
            CONSTRUCTORS
                .iter()
                .filter(|(_, construct)| {
                    let mut parser = construct(tokens.clone(), "SELECT 1");
                    !(0..2).all(|_| matches!(fragment(&mut parser), Ok(None)))
                })
                .map(move |(constructor, _)| format!("{name} via {constructor}"))
        })
        .collect()
}

pub(crate) fn fragment_changes_with_eof(fragment: FragmentParser, sql: &str) -> Vec<&'static str> {
    CONSTRUCTORS
        .iter()
        .filter(|(_, construct)| {
            let expected = fragment(&mut construct(stream(&[sql]), sql)).ok().flatten();
            let terminated = fragment(&mut construct(stream(&[sql, EOF_PART]), sql))
                .ok()
                .flatten();
            expected.is_none() || terminated != expected
        })
        .map(|(constructor, _)| *constructor)
        .collect()
}

pub(crate) fn statement(sql: &str, terminated: bool) -> crate::tokens::types::Result<Expression> {
    let tokens = stream(&[sql])
        .into_iter()
        .chain(terminated.then(eof))
        .collect();
    Parser::new(tokens).parse_statement()
}

pub(crate) fn eof_keeps_the_parse(sql: &str) -> (bool, bool) {
    let unterminated = statement(sql, false);
    let terminated = statement(sql, true);
    (
        unterminated.is_ok(),
        format!("{terminated:?}") == format!("{unterminated:?}"),
    )
}

pub(crate) fn terminated_value_at(sql: &str, terminated: bool, pointer: &str) -> Value {
    serde_json::to_value(statement(sql, terminated).expect("the statement parses"))
        .expect("syntax trees serialize")
        .pointer(pointer)
        .cloned()
        .unwrap_or(Value::Null)
}

pub(crate) fn parse_outcome(tokens: Vec<Token>) -> String {
    Parser::new(tokens).parse().map_or_else(
        |error| error.to_string(),
        |statements| format!("statements: {}", statements.len()),
    )
}

pub(crate) fn configured_parse_outcome(tokens: Vec<Token>) -> String {
    Parser::with_config(tokens, ParserConfig::default())
        .parse()
        .map_or_else(
            |error| error.to_string(),
            |statements| format!("statements: {}", statements.len()),
        )
}

pub(crate) fn sourced_parse_outcome(tokens: Vec<Token>) -> String {
    Parser::with_source(tokens, ParserConfig::default(), "SELECT 1".to_owned())
        .parse()
        .map_or_else(
            |error| error.to_string(),
            |statements| format!("statements: {}", statements.len()),
        )
}

pub(crate) fn statement_outcome(tokens: Vec<Token>) -> String {
    Parser::new(tokens)
        .parse_statement()
        .map_or_else(|error| error.to_string(), |_| "statement".to_owned())
}

pub(crate) fn data_type_outcome(tokens: Vec<Token>) -> String {
    Parser::new(tokens)
        .parse_standalone_data_type()
        .map_or_else(|error| error.to_string(), |_| "data type".to_owned())
}

pub(crate) fn parse_sql_outcome(sql: &str) -> String {
    Parser::parse_sql(sql).map_or_else(
        |error| error.to_string(),
        |statements| format!("statements: {}", statements.len()),
    )
}

pub(crate) fn present_at(
    dialect: Option<DialectType>,
    sql: &str,
    pointer: &str,
) -> Result<bool, String> {
    value_at(dialect, sql, pointer).map(|node| !node.is_null())
}

pub(crate) fn first_token_text(dialect: DialectType, sql: &str, token_type: TokenType) -> String {
    Dialect::get(dialect)
        .tokenize(sql)
        .expect("the SQL tokenizes")
        .into_iter()
        .find(|token| token.token_type == token_type)
        .map(|token| token.text)
        .unwrap_or_default()
}

pub(crate) fn data_type(
    dialect: DialectType,
    sql: &str,
    options: &ParseOptions,
) -> Result<DataType, String> {
    Dialect::get(dialect)
        .parse_data_type_with_options(sql, options)
        .map_err(|error| error.to_string())
}

pub(crate) fn stream_data_type(parts: &[&str]) -> Result<DataType, String> {
    Parser::new(stream(parts))
        .parse_standalone_data_type()
        .map_err(|error| error.to_string())
}

pub(crate) fn guarded(guard: ComplexityGuardOptions) -> ParseOptions {
    ParseOptions {
        complexity_guard: Some(guard),
    }
}

fn outcome_of(parsed: crate::tokens::types::Result<Vec<Expression>>) -> String {
    parsed.map_or_else(|error| error.to_string(), |_| "parsed".to_owned())
}

pub(crate) fn configured_outcome(sql: &str, max_parser_depth: Option<usize>) -> String {
    let tokens = Tokenizer::default()
        .tokenize(sql)
        .expect("the query tokenizes");
    outcome_of(
        Parser::with_config(
            tokens,
            ParserConfig {
                complexity_guard: ComplexityGuardOptions {
                    max_parser_depth,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .parse(),
    )
}

pub(crate) fn duckdb_outcome(sql: &str, max_parser_depth: Option<usize>) -> String {
    outcome_of(Dialect::get(DialectType::DuckDB).parse_with_options(
        sql,
        &guarded(ComplexityGuardOptions {
            max_parser_depth,
            ..Default::default()
        }),
    ))
}

pub(crate) fn nested_calls(depth: usize, name: &str) -> String {
    let call = (0..depth).fold(String::from("id"), |inner, _| format!("{name}({inner})"));
    format!("SELECT {call} FROM t")
}

pub(crate) fn wrapped_lambda(wrappers: usize) -> String {
    format!(
        "SELECT list_transform([1,2], {}x -> x + 1{})",
        "(".repeat(wrappers),
        ")".repeat(wrappers)
    )
}

pub(crate) fn listed(template: &str, count: usize, separator: &str) -> String {
    (1..=count)
        .map(|index| template.replace("{}", &index.to_string()))
        .collect::<Vec<_>>()
        .join(separator)
}
