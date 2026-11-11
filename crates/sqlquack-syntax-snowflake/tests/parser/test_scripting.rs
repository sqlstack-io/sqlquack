use serde_json::{json, Value};
use sqlquack_syntax::{CallMode, Dialect, Expression, ExpressionWalk, ParseOptions};
use sqlquack_syntax_snowflake::parse_statements_with;

use crate::helpers::{kind, located_text, sorted_strings};
use crate::test_types::{
    LocatedTestCase, ScriptingDialectTestCase, ScriptingStructureTestCase,
    ScriptingVerdictTestCase, ScriptingWalkTestCase,
};

const MODES: [CallMode; 2] = [CallMode::Faithful, CallMode::FaithfulLocated];

#[test]
fn given_scripting_blocks_when_parsing_then_members_keep_written_order_and_operands() {
    let test_cases = [
        ScriptingStructureTestCase {
            description: "declarations keep type, spelling of DEFAULT versus := and their values",
            sql: "DECLARE\n  total NUMBER(38, 2) DEFAULT 17.5;\n  label_text := 'west';\nBEGIN\n  LET extra NUMBER(38, 2) := 31;\n  LET note DEFAULT 'é';\n  total := total + extra;\n  RETURN total;\nEND;",
            expected_fields: vec![
                ("/declarations/0/variable/name/name", json!("total")),
                ("/declarations/0/variable/default/operator", json!("default")),
                ("/declarations/0/variable/default/value/literal/value", json!("17.5")),
                ("/declarations/1/variable/name/name", json!("label_text")),
                ("/declarations/1/variable/data_type", json!(null)),
                ("/declarations/1/variable/default/operator", json!("colon_equals")),
                ("/declarations/1/variable/default/value/literal/value", json!("west")),
                ("/body/0/let/declaration/variable/name/name", json!("extra")),
                ("/body/0/let/declaration/variable/default/operator", json!("colon_equals")),
                ("/body/0/let/declaration/variable/default/value/literal/value", json!("31")),
                ("/body/1/let/declaration/variable/data_type", json!(null)),
                ("/body/1/let/declaration/variable/default/operator", json!("default")),
                ("/body/1/let/declaration/variable/default/value/literal/value", json!("é")),
                ("/body/2/assignment/target/name", json!("total")),
                ("/body/3/return/value/scalar/column/name/name", json!("total")),
                ("/body/4", json!(null)),
                ("/exception_handlers", json!([])),
                ("/label", json!(null)),
            ],
        },
        ScriptingStructureTestCase {
            description: "IF keeps each ELSEIF branch separate from the ELSE body",
            sql: "BEGIN\n  LET amount := 31;\n  IF (amount > 40) THEN\n    RETURN 'high';\n  ELSEIF (amount > 17) THEN\n    RETURN 'middle';\n  ELSE\n    RETURN 'low';\n  END IF;\nEND;",
            expected_fields: vec![
                ("/body/1/if/branches/0/body/0/return/value/scalar/literal/value", json!("high")),
                ("/body/1/if/branches/1/body/0/return/value/scalar/literal/value", json!("middle")),
                ("/body/1/if/branches/2", json!(null)),
                ("/body/1/if/else_branch/body/0/return/value/scalar/literal/value", json!("low")),
            ],
        },
        ScriptingStructureTestCase {
            description: "simple CASE keeps its operand, WHEN values and END CASE spelling",
            sql: "BEGIN\n  LET region := 'west';\n  CASE (region)\n    WHEN 'east' THEN\n      RETURN 17;\n    WHEN 'west' THEN\n      RETURN 31;\n    ELSE\n      RETURN 43;\n  END CASE;\nEND;",
            expected_fields: vec![
                ("/body/1/case/operand/column/name/name", json!("region")),
                ("/body/1/case/branches/0/condition/literal/value", json!("east")),
                ("/body/1/case/branches/1/condition/literal/value", json!("west")),
                ("/body/1/case/branches/1/body/0/return/value/scalar/literal/value", json!("31")),
                ("/body/1/case/else_branch/body/0/return/value/scalar/literal/value", json!("43")),
                ("/body/1/case/end_case", json!(true)),
            ],
        },
        ScriptingStructureTestCase {
            description: "searched CASE closed by bare END has no operand",
            sql: "BEGIN\n  LET amount := 31;\n  CASE\n    WHEN amount < 17 THEN\n      RETURN 'low';\n    ELSE\n      RETURN 'other';\n  END;\nEND;",
            expected_fields: vec![
                ("/body/1/case/operand", json!(null)),
                ("/body/1/case/end_case", json!(false)),
                ("/body/1/case/else_branch/body/0/return/value/scalar/literal/value", json!("other")),
            ],
        },
        ScriptingStructureTestCase {
            description: "counter FOR keeps REVERSE, bounds and DO versus LOOP",
            sql: "DECLARE\n  steps VARCHAR DEFAULT '';\nBEGIN\n  FOR i IN REVERSE 1 TO 3 DO\n    steps := steps || i::VARCHAR;\n  END FOR;\n  FOR j IN 17 TO 31 LOOP\n    steps := steps || '-';\n  END LOOP;\n  RETURN steps;\nEND;",
            expected_fields: vec![
                ("/body/0/for/variable/name", json!("i")),
                ("/body/0/for/iteration/range/reverse", json!(true)),
                ("/body/0/for/iteration/range/start/literal/value", json!("1")),
                ("/body/0/for/iteration/range/end/literal/value", json!("3")),
                ("/body/0/for/keyword", json!("do")),
                ("/body/1/for/variable/name", json!("j")),
                ("/body/1/for/iteration/range/reverse", json!(false)),
                ("/body/1/for/iteration/range/start/literal/value", json!("17")),
                ("/body/1/for/iteration/range/end/literal/value", json!("31")),
                ("/body/1/for/keyword", json!("loop")),
                ("/body/1/for/label", json!(null)),
            ],
        },
        ScriptingStructureTestCase {
            description: "row FOR iterates a declared cursor by name",
            sql: "DECLARE\n  total NUMBER DEFAULT 0;\n  orders_cursor CURSOR FOR SELECT column1 AS amount FROM VALUES (17), (31);\nBEGIN\n  FOR order_row IN orders_cursor DO\n    total := total + order_row.amount;\n  END FOR;\n  RETURN total;\nEND;",
            expected_fields: vec![
                ("/declarations/1/cursor/name/name", json!("orders_cursor")),
                ("/body/0/for/variable/name", json!("order_row")),
                ("/body/0/for/iteration/rows/name", json!("orders_cursor")),
                ("/body/0/for/iteration/range", json!(null)),
            ],
        },
        ScriptingStructureTestCase {
            description: "WHILE keyword pairing and REPEAT condition after its body",
            sql: "DECLARE\n  counter NUMBER DEFAULT 0;\nBEGIN\n  WHILE (counter < 3) DO\n    counter := counter + 1;\n  END WHILE;\n  WHILE (counter < 5) LOOP\n    counter := counter + 17;\n  END LOOP;\n  REPEAT\n    counter := counter + 31;\n  UNTIL (counter > 43)\n  END REPEAT;\n  RETURN counter;\nEND;",
            expected_fields: vec![
                ("/body/0/while/keyword", json!("do")),
                ("/body/1/while/keyword", json!("loop")),
                ("/body/1/while/body/0/assignment/value/add/right/literal/value", json!("17")),
                ("/body/2/repeat/body/0/assignment/value/add/right/literal/value", json!("31")),
                ("/body/2/repeat/condition/gt/right/literal/value", json!("43")),
            ],
        },
        ScriptingStructureTestCase {
            description: "jump synonyms stay distinct and target written labels",
            sql: "BEGIN\n  FOR o IN 1 TO 3 DO\n    FOR n IN 1 TO 3 DO\n      IF (n = 2) THEN\n        CONTINUE outer_loop;\n      END IF;\n      IF (o = 3) THEN\n        BREAK outer_loop;\n      END IF;\n    END FOR inner_loop;\n  END FOR outer_loop;\n  LOOP\n    ITERATE;\n    EXIT;\n  END LOOP;\nEND;",
            expected_fields: vec![
                ("/body/0/for/label/name", json!("outer_loop")),
                ("/body/0/for/body/0/for/label/name", json!("inner_loop")),
                ("/body/0/for/body/0/for/body/0/if/branches/0/body/0/continue/keyword", json!("continue")),
                ("/body/0/for/body/0/for/body/0/if/branches/0/body/0/continue/label/name", json!("outer_loop")),
                ("/body/0/for/body/0/for/body/1/if/branches/0/body/0/break/keyword", json!("break")),
                ("/body/0/for/body/0/for/body/1/if/branches/0/body/0/break/label/name", json!("outer_loop")),
                ("/body/1/loop/body/0/continue/keyword", json!("iterate")),
                ("/body/1/loop/body/0/continue/label", json!(null)),
                ("/body/1/loop/body/1/break/keyword", json!("exit")),
            ],
        },
        ScriptingStructureTestCase {
            description: "cursor statements keep binds, fetch targets and names",
            sql: "DECLARE\n  floor_amount NUMBER DEFAULT 20;\n  first_amount NUMBER;\n  second_amount NUMBER;\n  orders_cursor CURSOR FOR SELECT column1, column2 FROM VALUES (17, 31) WHERE column1 > ?;\nBEGIN\n  OPEN orders_cursor USING (floor_amount);\n  FETCH orders_cursor INTO first_amount, second_amount;\n  CLOSE orders_cursor;\n  RETURN first_amount;\nEND;",
            expected_fields: vec![
                ("/declarations/1/variable/default", json!(null)),
                ("/body/0/open/cursor/name", json!("orders_cursor")),
                ("/body/0/open/using/0/column/name/name", json!("floor_amount")),
                ("/body/1/fetch/targets/0/name", json!("first_amount")),
                ("/body/1/fetch/targets/1/name", json!("second_amount")),
                ("/body/2/close/name/name", json!("orders_cursor")),
            ],
        },
        ScriptingStructureTestCase {
            description: "RESULTSET forms keep ASYNC, the optional value and cursor-over-resultset",
            sql: "DECLARE\n  first_job RESULTSET DEFAULT ASYNC (SELECT 17 AS amount);\n  pending RESULTSET;\nBEGIN\n  LET second_job RESULTSET := (SELECT 31 AS amount);\n  LET later RESULTSET;\n  LET jobs_cursor CURSOR FOR second_job;\n  AWAIT first_job;\n  CANCEL first_job;\n  AWAIT ALL;\n  RETURN TABLE(second_job);\nEND;",
            expected_fields: vec![
                ("/declarations/0/resultset/default/operator", json!("default")),
                ("/declarations/0/resultset/default/asynchronous", json!(true)),
                ("/declarations/1/resultset/default", json!(null)),
                ("/body/0/let/declaration/resultset/default/operator", json!("colon_equals")),
                ("/body/0/let/declaration/resultset/default/asynchronous", json!(false)),
                ("/body/1/let/declaration/resultset/default", json!(null)),
                ("/body/2/let/declaration/cursor/source/resultset/name", json!("second_job")),
                ("/body/3/await/target/resultset/name", json!("first_job")),
                ("/body/4/cancel/name/name", json!("first_job")),
                ("/body/5/await/target", json!("all")),
                ("/body/6/return/value/table/column/name/name", json!("second_job")),
            ],
        },
        ScriptingStructureTestCase {
            description: "exceptions keep number and message, handler names, action and OTHER",
            sql: "DECLARE\n  orders_missing EXCEPTION (-20017, 'orders missing');\n  orders_late EXCEPTION;\nBEGIN\n  RAISE orders_missing;\nEXCEPTION\n  WHEN orders_late OR orders_missing EXIT THEN\n    RETURN 31;\n  WHEN orders_late CONTINUE THEN\n    NULL;\n  WHEN OTHER THEN\n    RAISE;\nEND;",
            expected_fields: vec![
                ("/declarations/0/exception/number/neg/this/literal/value", json!("20017")),
                ("/declarations/0/exception/message/literal/value", json!("orders missing")),
                ("/declarations/1/exception/number", json!(null)),
                ("/declarations/1/exception/message", json!(null)),
                ("/body/0/raise/exception/name", json!("orders_missing")),
                ("/exception_handlers/0/exceptions/named/0/name", json!("orders_late")),
                ("/exception_handlers/0/exceptions/named/1/name", json!("orders_missing")),
                ("/exception_handlers/0/action", json!("exit")),
                ("/exception_handlers/1/action", json!("continue")),
                ("/exception_handlers/2/exceptions", json!("other")),
                ("/exception_handlers/2/action", json!(null)),
                ("/exception_handlers/2/body/0/raise/exception", json!(null)),
            ],
        },
        ScriptingStructureTestCase {
            description: "nested blocks keep their own declarations, handlers and label",
            sql: "DECLARE\n  total NUMBER DEFAULT 17;\nBEGIN\n  DECLARE\n    nested NUMBER DEFAULT 31;\n  BEGIN\n    total := total + nested;\n  EXCEPTION\n    WHEN OTHER THEN\n      NULL;\n  END inner_block;\n  RETURN total;\nEND",
            expected_fields: vec![
                ("/body/0/block/declarations/0/variable/name/name", json!("nested")),
                ("/body/0/block/exception_handlers/0/exceptions", json!("other")),
                ("/body/0/block/label/name", json!("inner_block")),
                ("/body/1/return/value/scalar/column/name/name", json!("total")),
                ("/exception_handlers", json!([])),
            ],
        },
    ];
    for test_case in test_cases {
        for mode in MODES {
            let parsed = parse_statements_with(
                test_case.sql,
                Dialect::Snowflake,
                &ParseOptions::default(),
                mode,
            )
            .unwrap_or_else(|error| panic!("{}: {mode:?}: {error:?}", test_case.description));
            assert_eq!(
                parsed.len(),
                1,
                "{}: one root ({mode:?})",
                test_case.description
            );
            let value = serde_json::to_value(&parsed).expect("serializes");
            let root = &value[0]["scripting_block"];
            let actual_fields = test_case
                .expected_fields
                .iter()
                .map(|(path, _)| (*path, root.pointer(path).cloned().unwrap_or(Value::Null)))
                .collect::<Vec<_>>();
            assert_eq!(
                actual_fields, test_case.expected_fields,
                "{}: {mode:?}",
                test_case.description
            );
            let restored: Vec<Expression> =
                serde_json::from_value(value.clone()).expect("scripting deserializes");
            assert_eq!(
                restored, parsed,
                "{}: roundtrip ({mode:?})",
                test_case.description
            );
        }
        assert_eq!(
            located_text(test_case.sql, "/0").as_deref(),
            Ok(test_case.sql.trim_end_matches(';')),
            "{}: own range",
            test_case.description
        );
    }
}

#[test]
fn given_recorded_block_verdicts_when_parsing_then_acceptance_matches_snowflake() {
    let test_cases = [
        ScriptingVerdictTestCase { description: "recorded accepted: LET RESULTSET without a value", sql: "BEGIN\n  LET orders RESULTSET;\n  RETURN 17;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded accepted: RESULTSET declared without a value, then assigned", sql: "DECLARE\n  pending RESULTSET;\nBEGIN\n  LET later RESULTSET;\n  later := (SELECT column1 AS amount FROM VALUES (17), (31));\n  RETURN TABLE(later);\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded rejected: RESULTSET operator without a query", sql: "BEGIN\n  LET orders RESULTSET :=;\n  RETURN 17;\nEND;", expected_error: Some("Expected LParen") },
        ScriptingVerdictTestCase { description: "recorded accepted: final END without semicolon", sql: "DECLARE\n  total NUMBER DEFAULT 17;\nBEGIN\n  RETURN total;\nEND", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded rejected: IF closed by bare END", sql: "BEGIN\n  IF (TRUE) THEN\n    RETURN 17;\n  END;\nEND;", expected_error: Some("Expected END IF") },
        ScriptingVerdictTestCase { description: "recorded rejected: WHILE condition without parentheses", sql: "DECLARE\n  counter NUMBER DEFAULT 0;\nBEGIN\n  WHILE counter < 3 DO\n    counter := counter + 1;\n  END WHILE;\n  RETURN counter;\nEND;", expected_error: Some("Expected LParen") },
        ScriptingVerdictTestCase { description: "recorded rejected: FOR DO closed by END LOOP", sql: "BEGIN\n  FOR i IN 1 TO 3 DO\n    NULL;\n  END LOOP;\nEND;", expected_error: Some("Expected END FOR") },
        ScriptingVerdictTestCase { description: "recorded rejected: OTHER handler before another handler", sql: "BEGIN\n  RETURN 17;\nEXCEPTION\n  WHEN OTHER THEN\n    RETURN 31;\n  WHEN STATEMENT_ERROR THEN\n    RETURN 43;\nEND;", expected_error: Some("WHEN OTHER must be the last") },
        ScriptingVerdictTestCase { description: "recorded rejected: empty block body", sql: "BEGIN\nEND;", expected_error: Some("Expected at least one statement") },
        ScriptingVerdictTestCase { description: "recorded rejected: ELSE IF is not ELSEIF", sql: "BEGIN\n  IF (TRUE) THEN\n    RETURN 17;\n  ELSE IF (FALSE) THEN\n    RETURN 31;\n  END IF;\nEND;", expected_error: Some("Expected END IF") },
        ScriptingVerdictTestCase { description: "recorded rejected: LET variable without type or value", sql: "BEGIN\n  LET amount;\n  RETURN amount;\nEND;", expected_error: Some("Expected a type or a value") },
        ScriptingVerdictTestCase { description: "recorded rejected: string exception number", sql: "DECLARE\n  orders_missing EXCEPTION ('17', 'orders missing');\nBEGIN\n  RETURN 17;\nEND;", expected_error: Some("Expected an integer exception number") },
        ScriptingVerdictTestCase { description: "recorded rejected: UNTIL condition without parentheses", sql: "DECLARE\n  counter NUMBER DEFAULT 0;\nBEGIN\n  REPEAT\n    counter := counter + 1;\n  UNTIL counter > 3\n  END REPEAT;\n  RETURN counter;\nEND;", expected_error: Some("Expected LParen") },
        ScriptingVerdictTestCase { description: "recorded rejected: DECLARE without members", sql: "DECLARE\nBEGIN\n  RETURN 17;\nEND;", expected_error: Some("Expected a declaration after DECLARE") },
        ScriptingVerdictTestCase { description: "recorded rejected: FETCH without INTO", sql: "DECLARE\n  first_amount NUMBER;\n  orders_cursor CURSOR FOR SELECT 17;\nBEGIN\n  OPEN orders_cursor;\n  FETCH orders_cursor first_amount;\n  RETURN first_amount;\nEND;", expected_error: Some("Expected Into") },
        ScriptingVerdictTestCase { description: "recorded rejected: LOOP closed by END WHILE", sql: "BEGIN\n  LOOP\n    BREAK;\n  END WHILE;\nEND;", expected_error: Some("Expected END LOOP") },
        ScriptingVerdictTestCase { description: "recorded rejected: statement without semicolon before END", sql: "BEGIN\n  RETURN 17\nEND;", expected_error: Some("Expected Semicolon") },
        ScriptingVerdictTestCase { description: "recorded accepted: a nested procedure declaration", sql: "DECLARE\n  adjust PROCEDURE (amount NUMBER) RETURNS NUMBER AS BEGIN RETURN amount + 17; END;\nBEGIN\n  RETURN 31;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded accepted: block closed by END IF", sql: "BEGIN\n  RETURN 17;\nEND IF;", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded accepted: block closed by END CASE", sql: "BEGIN\n  RETURN 17;\nEND CASE;", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded accepted: keyword label after END LOOP", sql: "BEGIN\n  LOOP\n    BREAK;\n  END LOOP WHILE;\n  RETURN 17;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded rejected: block closed by END FOR", sql: "BEGIN\n  RETURN 17;\nEND FOR;", expected_error: Some("not a reserved word") },
        ScriptingVerdictTestCase { description: "recorded rejected: a reserved word as a block end label", sql: "BEGIN\n  RETURN 17;\nEND SELECT;", expected_error: Some("not a reserved word") },
        ScriptingVerdictTestCase { description: "recorded rejected: a reserved word as a loop variable", sql: "BEGIN FOR SELECT IN 1 TO 1 DO NULL ; END FOR ; END ;", expected_error: Some("not a reserved word") },
        ScriptingVerdictTestCase { description: "recorded rejected: a reserved word as a handled exception", sql: "BEGIN NULL ; EXCEPTION WHEN SELECT THEN NULL ; NULL ; END ;", expected_error: Some("not a reserved word") },
        ScriptingVerdictTestCase { description: "recorded rejected: a bare expression as a statement", sql: "BEGIN IF ( TRUE ) THEN 1 ; END IF ; END ;", expected_error: Some("Expected a statement") },
        ScriptingVerdictTestCase { description: "recorded rejected: an empty statement", sql: "BEGIN NULL ; EXCEPTION WHEN STATEMENT_ERROR THEN ; NULL ; END ;", expected_error: Some("Expected a statement") },
        ScriptingVerdictTestCase { description: "recorded accepted: a declared cursor over a result set name", sql: "DECLARE\n  rs RESULTSET DEFAULT (SELECT 17 AS a);\n  c CURSOR FOR rs;\nBEGIN\n  RETURN 17;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded accepted: cursor FOR with LOOP and END LOOP", sql: "DECLARE\n  orders_cursor CURSOR FOR SELECT 17 AS amount;\nBEGIN\n  FOR order_row IN orders_cursor LOOP\n    NULL;\n  END LOOP;\n  RETURN 17;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "cursor FOR with LOOP still needs END LOOP", sql: "DECLARE\n  orders_cursor CURSOR FOR SELECT 17 AS amount;\nBEGIN\n  FOR order_row IN orders_cursor LOOP\n    NULL;\n  END FOR;\nEND;", expected_error: Some("Expected END LOOP") },
        ScriptingVerdictTestCase { description: "recorded accepted: LET with a type and no value", sql: "BEGIN\n  LET amount NUMBER;\n  RETURN 17;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded semantic error: DECLARE without a type or value parses", sql: "DECLARE\n  amount;\nBEGIN\n  RETURN 17;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded accepted: RETURN without a value", sql: "BEGIN\n  RETURN;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded accepted: simple CASE operand without parentheses", sql: "BEGIN\n  LET region := 'west';\n  CASE region\n    WHEN 'west' THEN\n      RETURN 31;\n  END CASE;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded rejected: IF condition without parentheses", sql: "BEGIN\n  IF TRUE THEN\n    RETURN 17;\n  END IF;\nEND;", expected_error: Some("Expected LParen") },
        ScriptingVerdictTestCase { description: "recorded semantic error: exception number -20999 parses", sql: "DECLARE\n  e EXCEPTION (-20999, 'west');\nBEGIN\n  RETURN 17;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded semantic error: exception number -20000 parses", sql: "DECLARE\n  e EXCEPTION (-20000, 'west');\nBEGIN\n  RETURN 17;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "a positive exception number parses; its range is checked later", sql: "DECLARE\n  e EXCEPTION (20001, 'west');\nBEGIN\n  RETURN 17;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "an exception number is an integer literal, optionally negated", sql: "DECLARE\n  e EXCEPTION (+20001, 'west');\nBEGIN\n  RETURN 17;\nEND;", expected_error: Some("Expected an integer exception number") },
        ScriptingVerdictTestCase { description: "recorded semantic error: exception number -21000 parses", sql: "DECLARE\n  e EXCEPTION (-21000, 'west');\nBEGIN\n  RETURN 17;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "exception number -1 parses; its range is checked later", sql: "DECLARE\n  e EXCEPTION (-1, 'west');\nBEGIN\n  RETURN 17;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "recorded accepted -20001 and unrecorded -20998 are accepted", sql: "DECLARE\n  low EXCEPTION (-20998, 'west');\n  high EXCEPTION (-20001, 'east');\nBEGIN\n  RETURN 17;\nEND;", expected_error: None },
        ScriptingVerdictTestCase { description: "quoted END cannot close a block", sql: "BEGIN\n  RETURN 17;\n\"END\";", expected_error: Some("Expected a statement") },
        ScriptingVerdictTestCase { description: "label must not swallow an adjacent statement", sql: "BEGIN\n  RETURN 17;\nEND inner_block SELECT 31;", expected_error: Some("Unexpected token") },
    ];
    for test_case in test_cases {
        for mode in MODES {
            let result = parse_statements_with(
                test_case.sql,
                Dialect::Snowflake,
                &ParseOptions::default(),
                mode,
            );
            let error: Option<String> = result.err().map(|error| format!("{error:?}"));
            assert_eq!(
                error.is_some(),
                test_case.expected_error.is_some(),
                "{}: {mode:?}: {error:?}",
                test_case.description
            );
            assert!(
                error
                    .unwrap_or_default()
                    .contains(test_case.expected_error.unwrap_or_default()),
                "{}: {mode:?}",
                test_case.description
            );
        }
    }
}

#[test]
fn given_scripting_members_when_locating_then_ranges_belong_to_the_written_member() {
    let sql = "DECLARE\n  \"total é\" NUMBER DEFAULT 17;\nBEGIN\n  LET note := '🦆';\n  IF (\"total é\" > 31) THEN\n    RETURN 'high';\n  END IF;\nEXCEPTION\n  WHEN OTHER CONTINUE THEN\n    NULL;\nEND outer_block; SELECT 97";
    let test_cases = [
        LocatedTestCase { description: "block range excludes the adjacent statement", sql, pointer: "/0", expected_text: Ok("DECLARE\n  \"total é\" NUMBER DEFAULT 17;\nBEGIN\n  LET note := '🦆';\n  IF (\"total é\" > 31) THEN\n    RETURN 'high';\n  END IF;\nEXCEPTION\n  WHEN OTHER CONTINUE THEN\n    NULL;\nEND outer_block") },
        LocatedTestCase { description: "declaration excludes its semicolon", sql, pointer: "/0/scripting_block/declarations/0", expected_text: Ok("\"total é\" NUMBER DEFAULT 17") },
        LocatedTestCase { description: "quoted Unicode variable name keeps its quotes", sql, pointer: "/0/scripting_block/declarations/0/variable/name", expected_text: Ok("\"total é\"") },
        LocatedTestCase { description: "LET statement owns its keyword and value", sql, pointer: "/0/scripting_block/body/0", expected_text: Ok("LET note := '🦆'") },
        LocatedTestCase { description: "IF statement ends at END IF", sql, pointer: "/0/scripting_block/body/1", expected_text: Ok("IF (\"total é\" > 31) THEN\n    RETURN 'high';\n  END IF") },
        LocatedTestCase { description: "branch owns its condition and body", sql, pointer: "/0/scripting_block/body/1/if/branches/0", expected_text: Ok("IF (\"total é\" > 31) THEN\n    RETURN 'high';") },
        LocatedTestCase { description: "handler owns WHEN through its body", sql, pointer: "/0/scripting_block/exception_handlers/0", expected_text: Ok("WHEN OTHER CONTINUE THEN\n    NULL;") },
        LocatedTestCase { description: "NULL statement is a ranged member, not a value", sql, pointer: "/0/scripting_block/exception_handlers/0/body/0", expected_text: Ok("NULL") },
        LocatedTestCase { description: "block label has its own range", sql, pointer: "/0/scripting_block/label", expected_text: Ok("outer_block") },
        LocatedTestCase { description: "the adjacent statement stays separate", sql, pointer: "/1", expected_text: Ok("SELECT 97") },
    ];
    for test_case in test_cases {
        assert_eq!(
            located_text(test_case.sql, test_case.pointer)
                .as_deref()
                .map_err(String::as_str),
            test_case.expected_text,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_scripting_blocks_when_walking_then_nested_operands_are_reachable_after_roundtrip() {
    let test_cases = [ScriptingWalkTestCase {
        description: "a block with a declaration, a cursor, a FOR loop, an IF and a handler",
        sql: "DECLARE\n  floor_amount NUMBER DEFAULT 17;\n  orders_cursor CURSOR FOR SELECT amount FROM orders WHERE region = 'west';\nBEGIN\n  FOR order_row IN 1 TO 31 DO\n    IF (floor_amount > 43) THEN\n      RETURN 'high';\n    END IF;\n  END FOR;\nEXCEPTION\n  WHEN OTHER THEN\n    RETURN 'east';\nEND;",
        expected_literals: vec!["1", "17", "31", "43", "east", "high", "west"],
        expected_columns: vec!["amount", "floor_amount", "region"],
    }];
    for test_case in test_cases {
        for mode in MODES {
            let parsed = parse_statements_with(
                test_case.sql,
                Dialect::Snowflake,
                &ParseOptions::default(),
                mode,
            )
            .expect(test_case.description);
            let restored: Vec<Expression> =
                serde_json::from_value(serde_json::to_value(&parsed).expect("serializes"))
                    .expect("deserializes");
            for root in [&parsed[0], &restored[0]] {
                for nodes in [
                    root.dfs().collect::<Vec<_>>(),
                    root.bfs().collect::<Vec<_>>(),
                ] {
                    let values: Vec<Value> = nodes
                        .into_iter()
                        .map(|node| serde_json::to_value(node).expect("walk node"))
                        .collect();
                    assert_eq!(
                        (
                            sorted_strings(&values, "/literal/value"),
                            sorted_strings(&values, "/column/name/name")
                        ),
                        (
                            test_case.expected_literals.clone(),
                            test_case.expected_columns.clone()
                        ),
                        "{}: {mode:?}",
                        test_case.description
                    );
                }
            }
        }
    }
}

#[test]
fn given_begin_and_declare_when_parsing_then_only_snowflake_blocks_change() {
    let test_cases = [
        ScriptingDialectTestCase {
            description: "Snowflake BEGIN TRANSACTION stays a transaction",
            sql: "BEGIN TRANSACTION",
            dialect: Dialect::Snowflake,
            expected_root: Ok("transaction"),
        },
        ScriptingDialectTestCase {
            description: "Snowflake bare BEGIN stays a transaction",
            sql: "BEGIN;",
            dialect: Dialect::Snowflake,
            expected_root: Ok("transaction"),
        },
        ScriptingDialectTestCase {
            description: "Snowflake BEGIN WORK stays a transaction",
            sql: "BEGIN WORK",
            dialect: Dialect::Snowflake,
            expected_root: Ok("transaction"),
        },
        ScriptingDialectTestCase {
            description: "Snowflake BEGIN NAME stays a named transaction, not a block",
            sql: "BEGIN NAME orders_tx",
            dialect: Dialect::Snowflake,
            expected_root: Ok("transaction"),
        },
        ScriptingDialectTestCase {
            description: "Snowflake variable named name is a block member",
            sql: "BEGIN name := 17; END;",
            dialect: Dialect::Snowflake,
            expected_root: Ok("scripting_block"),
        },
        ScriptingDialectTestCase {
            description: "Snowflake block followed by a statement keeps both",
            sql: "BEGIN RETURN 17; END; SELECT 31",
            dialect: Dialect::Snowflake,
            expected_root: Ok("scripting_block"),
        },
        ScriptingDialectTestCase {
            description: "DuckDB BEGIN TRANSACTION is unchanged",
            sql: "BEGIN TRANSACTION",
            dialect: Dialect::DuckDB,
            expected_root: Ok("transaction"),
        },
        ScriptingDialectTestCase {
            description: "DuckDB keeps its existing opaque BEGIN path",
            sql: "BEGIN RETURN 17; END;",
            dialect: Dialect::DuckDB,
            expected_root: Ok("command"),
        },
    ];
    for test_case in test_cases {
        let result = parse_statements_with(
            test_case.sql,
            test_case.dialect,
            &ParseOptions::default(),
            CallMode::Faithful,
        );
        let root = result
            .map(|statements| kind(&serde_json::to_value(&statements[0]).expect("serializes")))
            .map_err(|_| ());
        assert_eq!(
            root.as_deref().map_err(|_| ()),
            test_case.expected_root,
            "{}",
            test_case.description
        );
    }
}
