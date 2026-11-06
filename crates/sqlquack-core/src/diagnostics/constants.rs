//! The diagnostic code registry, the one source of every `SQ` code; append only, see codes.lock.

use crate::diagnostics::models::CodeEntry;

/// The prefix of a parse refusal's reason, before the parser's own message.
pub const UNPARSED_PREFIX: &str = "the SQL does not parse: ";
/// The words of an unresolved name's reason when it matches a declared name only ignoring case.
pub const CASE_ONLY_MATCH: &str = "only without regard to case";
/// The help for such a name, replacing SQ0004's own.
pub const CASE_ONLY_MATCH_HELP: &str = "Snowflake folds unquoted names to upper case, so declare \
    the relation or column in the schema as the SQL resolves it (`ORDERS` for `FROM orders`), or \
    quote the name in the SQL to keep its case";
/// The words of an unresolved name's reason when two result columns share a name.
pub const REPEATED_RESULT_NAME: &str = "repeats, possibly differing only in case";
/// The help for such a name, replacing SQ0004's own.
pub const REPEATED_RESULT_NAME_HELP: &str =
    "give each result column its own alias, such as `SELECT 1 AS first, 1 AS second`";
/// The words of a refusal reading a TIMESTAMP_TZ's local time, which needs the offset DuckDB drops.
pub const ZONED_LOCAL_TIME: &str =
    "which reads the local time of an offset DuckDB's TIMESTAMPTZ drops";
/// Why such a reading is refused, a note on its diagnostic.
pub const ZONED_LOCAL_TIME_NOTE: &str = "a Snowflake TIMESTAMP_TZ stores each value's own UTC \
    offset, and local dates, times, day boundaries and formats read that offset; DuckDB's \
    TIMESTAMPTZ keeps only the instant, so SQLQuack cannot reproduce them and adds no columns or \
    hidden state to keep the offset";
/// The help for such a reading, replacing SQ0003's own.
pub const ZONED_LOCAL_TIME_HELP: &str = "convert the value to a wall time first: \
    `CONVERT_TIMEZONE('UTC', col)::TIMESTAMP_NTZ` for UTC, or \
    `CONVERT_TIMEZONE(<zone column>, col)::TIMESTAMP_NTZ` for local day boundaries in a known zone";
/// The prefix of a complexity guard's message, before the rest of its `E_GUARD_*` name.
pub const GUARD_PREFIX: &str = "E_GUARD_";
/// The longest source line shown in full; longer lines show a window around the span.
pub const MAX_SHOWN_LINE: usize = 120;
/// How many columns a tab takes when a source line is shown.
pub const TAB_WIDTH: usize = 4;

/// SQL that does not parse.
pub const PARSE_FIDELITY: &str = "SQ0001";
/// No verified rule converts the construct.
pub const MISSING_RULE: &str = "SQ0002";
/// The construct's operand types fall outside every rule's proven conditions.
pub const TYPE_CONDITION: &str = "SQ0003";
/// A relation or column name does not resolve.
pub const UNRESOLVED_NAME: &str = "SQ0004";
/// DuckDB cannot express the construct's meaning.
pub const NOT_EXPRESSIBLE: &str = "SQ0005";
/// Conversion took longer, or emitted more SQL, than its budget.
pub const CONVERSION_BUDGET: &str = "SQ0006";
/// The session parameters match no recorded session profile.
pub const SESSION_PARAMETERS: &str = "SQ0007";
/// The conversion needs a known divergence the policy refuses in this mode.
pub const DIVERGENCE_REFUSED: &str = "SQ0008";
/// An unaliased select item's Snowflake-generated column name is not verified.
pub const UNALIASED_OUTPUT_NAME: &str = "SQ0009";
/// The construct name of that refusal, which its code is registered under.
pub const UNALIASED_OUTPUT_NAME_CONSTRUCT: &str = "unaliased_output_name";

/// Every diagnostic code, in the order issued.
pub const CODES: &[CodeEntry] = &[
    CodeEntry {
        code: PARSE_FIDELITY,
        name: "parse_fidelity",
        alias: None,
        title: "the SQL does not parse",
        help: None,
        explanation: "SQLQuack's Snowflake parser could not read the SQL, so nothing in it was \
            converted. The span points where the parser stopped and the label carries the \
            parser's message. Check the SQL at that point against Snowflake's grammar; if \
            Snowflake accepts the statement, SQLQuack's parser is missing the syntax and the \
            statement should be reported with the label.",
        example: r#"echo "SELECT a, FROM orders WHERE (b = 1" | sqlquack check"#,
    },
    CodeEntry {
        code: MISSING_RULE,
        name: "missing_rule",
        alias: None,
        title: "no verified rule converts this construct",
        help: None,
        explanation: "Every construct SQLQuack converts comes from a rule verified against \
            recorded Snowflake results. No such rule covers the construct at the span yet, so \
            the statement is refused rather than converted to SQL that might mean something \
            else in DuckDB. Rewrite the statement without the construct, or wait for a rule; \
            `sqlquack census` shows which constructs a body of SQL needs.",
        example: r#"echo "SELECT WEEKISO(CURRENT_DATE()) AS w" | sqlquack convert --builtin-policy"#,
    },
    CodeEntry {
        code: TYPE_CONDITION,
        name: "type_condition",
        alias: None,
        title: "the operand types fall outside every rule's proven conditions",
        help: Some(
            "declare the operands' Snowflake types in the schema, or cast them to a type the rule covers",
        ),
        explanation: "A rule covers the construct, but only for operand types its recorded \
            cases prove. The operands at the span have other types, or types SQLQuack could not \
            determine. Declaring the relation's columns and types in the schema often lets the \
            types resolve; an explicit cast to a covered type also works when it keeps the \
            statement's meaning.",
        example: r#"echo "SELECT FLOOR('abc') AS f" | sqlquack convert --builtin-policy"#,
    },
    CodeEntry {
        code: UNRESOLVED_NAME,
        name: "unresolved_name",
        alias: None,
        title: "a name does not resolve",
        help: Some(
            "declare the relation and its columns in the schema (`--schema` on the command line)",
        ),
        explanation: "Converting a statement needs to know what each relation and column \
            name refers to and what type it has. The name at the span is unknown, or resolves to \
            more than one column. Add the relation and its columns to the schema file, or \
            qualify an ambiguous column with its relation.",
        example: r#"echo "SELECT a FROM orders" | sqlquack convert --builtin-policy"#,
    },
    CodeEntry {
        code: NOT_EXPRESSIBLE,
        name: "not_expressible",
        alias: None,
        title: "DuckDB cannot express this construct's Snowflake meaning",
        help: None,
        explanation: "The construct at the span has a Snowflake meaning DuckDB has no way to \
            reproduce, for the reason in the label. No future rule will convert it as written. \
            Rewrite the statement so it does not depend on that behaviour.",
        example: "no construct is declared inexpressible yet; one would be refused with the reason as its label",
    },
    CodeEntry {
        code: CONVERSION_BUDGET,
        name: "conversion_budget",
        alias: None,
        title: "conversion exceeded its budget",
        help: Some("split the statement into smaller statements"),
        explanation: "Converting the statement took longer, or produced more DuckDB SQL, than \
            the conversion budget allows. The budget keeps a pathological input from stalling a \
            caller. Splitting the statement, for example into common table expressions or \
            separate statements, usually brings it under the budget.",
        example: "a statement whose conversion runs past the time budget or emits more DuckDB SQL than it allows",
    },
    CodeEntry {
        code: SESSION_PARAMETERS,
        name: "session_parameters",
        alias: None,
        title: "the session parameters match no recorded session profile",
        help: Some("pass only session parameters a recorded profile covers, or leave them unset"),
        explanation: "Conversions are proven under recorded Snowflake session profiles. The \
            session parameters given match none of them, so no conversion is proven for that \
            session. The label names the parameters; unset parameters take Snowflake's \
            documented defaults.",
        example: r#"echo "SELECT 1 AS one" | sqlquack convert --builtin-policy --session WEEK_START=1"#,
    },
    CodeEntry {
        code: DIVERGENCE_REFUSED,
        name: "divergence_refused",
        alias: None,
        title: "the conversion needs a known divergence the policy refuses",
        help: Some(
            "the help names the divergence and the fixes that work in the current mode: allow \
             it in the policy file, drop --strict or --loud-divergences-only, or confirm its \
             assumption; --builtin-policy refuses every one",
        ),
        explanation: "The construct at the span converts only by relying on the known \
            divergence the label names, where DuckDB may raise or differ from Snowflake as the \
            policy file describes. The policy refuses that divergence in the current mode, so \
            the statement is refused. Allow it in the policy file, use a mode that allows it, \
            or confirm the assumption it rests on; otherwise rewrite the statement without the \
            construct.",
        example: r#"echo "SELECT HASH(1) AS h" | sqlquack convert --builtin-policy"#,
    },
    CodeEntry {
        code: UNALIASED_OUTPUT_NAME,
        name: UNALIASED_OUTPUT_NAME_CONSTRUCT,
        alias: None,
        title: "this column has no alias, and SQLQuack has not verified the name Snowflake gives it",
        help: Some("add an alias, e.g. `AS one`"),
        explanation: "Snowflake names an unaliased select item after the expression's text, and \
            a converted query must return the same column names. SQLQuack emits Snowflake's name \
            only for the shapes recorded results verify, named by their text upper-cased as \
            written: literals, columns and COUNT, SUM, MAX, AVG, LOWER, UPPER and COALESCE of \
            columns, a column plus, minus, times, divided by or concatenated with a literal, and \
            a column cast to INTEGER, written with single spaces and at most one line break. \
            Other items, such as paths, CASE, nested calls or other functions, are refused \
            rather than converted with a different column name. An alias fixes the name in both \
            engines.",
        example: r#"echo "SELECT 'café' || 'b'" | sqlquack convert --builtin-policy"#,
    },
    CodeEntry {
        code: "SQ0101",
        name: "input_too_large",
        alias: Some("E_GUARD_INPUT_TOO_LARGE"),
        title: "the input is larger than the parser accepts",
        help: Some("split the input into smaller files"),
        explanation: "The parser refuses inputs beyond a fixed size so that one input cannot \
            exhaust memory. Split the SQL into several inputs and convert each.",
        example: r#"python3 -c "print('SELECT 1;' * 2000000)" | sqlquack check"#,
    },
    CodeEntry {
        code: "SQ0102",
        name: "token_budget_exceeded",
        alias: Some("E_GUARD_TOKEN_BUDGET_EXCEEDED"),
        title: "the input has more tokens than the parser accepts",
        help: Some("split the input into smaller statements or files"),
        explanation: "The parser refuses inputs with more tokens than its budget so that \
            parsing time stays bounded. Split the SQL into several statements or inputs.",
        example: r#"python3 -c "print('SELECT 1;' * 400000)" | sqlquack check"#,
    },
    CodeEntry {
        code: "SQ0103",
        name: "nesting_depth_exceeded",
        alias: Some("E_GUARD_NESTING_DEPTH_EXCEEDED"),
        title: "brackets nest deeper than the parser accepts",
        help: Some(
            "remove redundant parentheses, or move nested parts into common table expressions",
        ),
        explanation: "Parentheses, brackets and braces nest deeper at the span than the \
            parser's limit. The limit keeps parsing from exhausting the stack. Redundant \
            parentheses can be removed; deeply nested subqueries can usually become common \
            table expressions.",
        example: r#"python3 -c "print('SELECT ' + '(' * 600 + '1' + ')' * 600)" | sqlquack check"#,
    },
    CodeEntry {
        code: "SQ0104",
        name: "parser_depth_exceeded",
        alias: Some("E_GUARD_PARSER_DEPTH_EXCEEDED"),
        title: "the statement nests deeper than the parser accepts",
        help: Some("move nested parts into common table expressions"),
        explanation: "The parser's recursion reached its depth limit at the span. The limit \
            keeps parsing from exhausting the stack. Restructure the deepest part of the \
            statement, for example with common table expressions.",
        example: r#"python3 -c "print('SELECT ' + 'NOT ' * 2000 + 'TRUE')" | sqlquack check"#,
    },
    CodeEntry {
        code: "SQ0105",
        name: "ast_depth_exceeded",
        alias: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        title: "the parsed statement nests deeper than SQLQuack accepts",
        help: Some("move nested parts into common table expressions"),
        explanation: "The statement parsed, but its tree nests deeper than the limit SQLQuack \
            places on later analysis, which keeps conversion from exhausting the stack. \
            Restructure the deepest part of the statement.",
        example: r#"python3 -c "print('SELECT ' + 'CASE WHEN TRUE THEN ' * 600 + '1' + ' END' * 600)" | sqlquack check"#,
    },
    CodeEntry {
        code: "SQ0106",
        name: "ast_budget_exceeded",
        alias: Some("E_GUARD_AST_BUDGET_EXCEEDED"),
        title: "the parsed statement is larger than SQLQuack accepts",
        help: Some("split the statement into smaller statements"),
        explanation: "The statement parsed into more tree nodes than SQLQuack's budget, which \
            keeps conversion time and memory bounded. Split it into smaller statements.",
        example: "a statement that parses into more than a million tree nodes",
    },
    CodeEntry {
        code: "SQ0107",
        name: "function_nesting_depth_exceeded",
        alias: Some("E_GUARD_FUNCTION_NESTING_DEPTH_EXCEEDED"),
        title: "function calls nest deeper than SQLQuack accepts",
        help: Some("compute inner calls in a common table expression first"),
        explanation: "Function calls nest inside each other deeper than SQLQuack's limit at \
            the span. Compute the innermost calls in a common table expression or a subquery \
            and refer to their results.",
        example: r#"python3 -c "print('SELECT ' + 'ABS(' * 300 + '1' + ')' * 300)" | sqlquack check"#,
    },
    CodeEntry {
        code: "SQ0201",
        name: "set_operation_all",
        alias: None,
        title: "Snowflake does not support INTERSECT, EXCEPT or MINUS with ALL",
        help: Some("drop ALL to compare distinct rows; only UNION accepts ALL in Snowflake"),
        explanation: "Snowflake accepts ALL only after UNION. INTERSECT ALL, EXCEPT ALL and \
            MINUS ALL parse, but Snowflake rejects them as an unsupported feature, so there is \
            no Snowflake meaning to convert. Without ALL the \
            operators compare distinct rows; keeping duplicates needs a rewrite, for example \
            numbering each row with ROW_NUMBER() over all columns and comparing those pairs.",
        example: r#"echo "SELECT id FROM orders INTERSECT ALL SELECT id FROM returns" | sqlquack check"#,
    },
    CodeEntry {
        code: "SQ0202",
        name: "lock_wait",
        alias: None,
        title: "Snowflake does not support WAIT in FOR UPDATE",
        help: Some("remove the WAIT clause"),
        explanation: "FOR UPDATE WAIT n parses, but Snowflake rejects it as an unsupported \
            feature, so the statement never runs there. Remove the WAIT clause; Snowflake's \
            lock timeout comes from the LOCK_TIMEOUT session parameter instead.",
        example: r#"echo "SELECT order_id FROM orders FOR UPDATE WAIT 7" | sqlquack check"#,
    },
    CodeEntry {
        code: "SQ0203",
        name: "qualify_without_window",
        alias: None,
        title: "QUALIFY needs a window function",
        help: Some(
            "move the condition to WHERE or HAVING, or filter on a window function's result",
        ),
        explanation: "Snowflake evaluates QUALIFY after window functions, and rejects a \
            QUALIFY clause when neither the SELECT list nor QUALIFY itself calls a window \
            function (Snowflake error 992). A window function the QUALIFY condition reaches \
            through a SELECT-list alias counts. A condition on plain columns belongs in WHERE, \
            and one on aggregates in HAVING.",
        example: r#"echo "SELECT 1 QUALIFY TRUE" | sqlquack check"#,
    },
    CodeEntry {
        code: "SQ0204",
        name: "star_filter_drops_named_column",
        alias: None,
        title: "REPLACE or RENAME names a column the star's ILIKE pattern removed",
        help: Some("name a column the ILIKE pattern keeps, or widen the pattern"),
        explanation: "In `* ILIKE 'pattern' REPLACE (...)` or `RENAME (...)`, Snowflake applies \
            the pattern first, ignoring case, and rejects a REPLACE or RENAME of a column the \
            pattern did not keep (Snowflake error 1431). A name that fails the pattern ignoring \
            case fails it whatever the table holds, so no schema is needed to see it.",
        example: r#"echo "SELECT * ILIKE '%_id' REPLACE (price * 2 AS price) FROM orders" | sqlquack check"#,
    },
];
