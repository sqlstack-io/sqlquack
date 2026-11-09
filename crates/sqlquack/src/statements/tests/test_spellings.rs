use std::collections::BTreeSet;

use sqlquack_syntax::{Dialect, ParseOptions};
use sqlquack_syntax_snowflake::parse_statements;

use crate::statements::_helpers::spellings::tree_spellings;
use crate::statements::tests::test_types::SpellingsTestCase;

#[test]
fn given_calls_when_reading_spellings_then_each_call_keys_its_full_name_and_shape() {
    let test_cases = [
        SpellingsTestCase {
            description: "a schema-qualified call keys its qualified name",
            sql: "SELECT SNOWFLAKE.CORE.NULL_COUNT(SELECT c FROM t)",
            expected_keys: &["call:SNOWFLAKE.CORE.NULL_COUNT"],
        },
        SpellingsTestCase {
            description: "a call qualified by schema alone keys both parts",
            sql: "SELECT analytics.normalize(x, 2) FROM t",
            expected_keys: &["call:ANALYTICS.NORMALIZE"],
        },
        SpellingsTestCase {
            description: "an unqualified call keys its bare name",
            sql: "SELECT NULL_COUNT(c) FROM t",
            expected_keys: &["call:NULL_COUNT"],
        },
    ];
    for test_case in test_cases {
        let statements =
            parse_statements(test_case.sql, Dialect::Snowflake, &ParseOptions::default())
                .expect("the SQL parses");
        let keys: BTreeSet<String> = statements.iter().flat_map(tree_spellings).collect();
        let calls: Vec<&str> = keys
            .iter()
            .map(String::as_str)
            .filter(|key| key.starts_with("call"))
            .collect();

        assert_eq!(calls, test_case.expected_keys, "{}", test_case.description);
    }
}
