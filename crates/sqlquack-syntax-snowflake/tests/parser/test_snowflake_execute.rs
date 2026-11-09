use serde_json::json;

use sqlquack_syntax::Dialect;

use crate::helpers::{accepted, statement_at};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase};

#[test]
fn given_a_snowflake_execute_of_a_job_notebook_dbt_or_code_bundle_when_parsing_then_receipted_forms_are_accepted(
) {
    let test_cases = [
        AcceptanceTestCase { description: "a job service runs a staged template with USING", sql: "EXECUTE JOB SERVICE IN COMPUTE POOL p1 FROM @s1 SPECIFICATION_TEMPLATE_FILE = m1 USING ( v1 => 1 ) NAME = 'v1' REPLICAS = 'x'", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a job service template needs USING arguments", sql: "EXECUTE JOB SERVICE IN COMPUTE POOL p1 FROM @s1 SPECIFICATION_TEMPLATE_FILE = 'm' USING ( )", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a job service name has no empty part", sql: "EXECUTE JOB SERVICE IN COMPUTE POOL p1 FROM @s1 SPECIFICATION_FILE = 'm' NAME = v1 . . v1", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a job service takes inline specification text", sql: "EXECUTE JOB SERVICE IN COMPUTE POOL p1 FROM SPECIFICATION $$ m $$", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "an inference model may skip its schema", sql: "EXECUTE INFERENCE JOB SERVICE IN COMPUTE POOL p1 WITH SPECIFICATION $$ m $$ FROM ( SELECT 1 ) MODEL = v1 . . v1", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "an inference model is not a string", sql: "EXECUTE INFERENCE JOB SERVICE IN COMPUTE POOL p1 WITH SPECIFICATION $$ m $$ FROM ( SELECT 1 ) MODEL = 'v1'", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "inference replicas are a number", sql: "EXECUTE INFERENCE JOB SERVICE IN COMPUTE POOL p1 WITH SPECIFICATION $$ m $$ FROM @s1/ MODEL = v1 REPLICAS = 'x'", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "inference input is a query", sql: "EXECUTE INFERENCE JOB SERVICE IN COMPUTE POOL p1 WITH SPECIFICATION $$ m $$ FROM ( 1 ) MODEL = v1", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a stage path ends at whitespace", sql: "EXECUTE INFERENCE JOB SERVICE IN COMPUTE POOL p1 WITH SPECIFICATION $$ m $$ FROM @\"s1 Q\" MODEL = v1", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a notebook project takes names as values", sql: "EXECUTE NOTEBOOK PROJECT d1 . s1 . p1 MAIN_FILE = 'n.ipynb' COMPUTE_POOL = m1 QUERY_WAREHOUSE = 'w' RUNTIME = 'r' SECRETS = ( d1 . s1 . k1 )", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a notebook project needs COMPUTE_POOL", sql: "EXECUTE NOTEBOOK PROJECT d1 . s1 . p1 MAIN_FILE = 'n.ipynb' QUERY_WAREHOUSE = 'w' RUNTIME = 'r'", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a notebook takes parameters", sql: "EXECUTE NOTEBOOK n1 ( 'm' , m1 )", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a dbt project runs from a workspace", sql: "EXECUTE DBT PROJECT FROM WORKSPACE w1", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "dbt imports take a system function and a string alias", sql: "EXECUTE DBT PROJECT v1 IMPORTS = ( SYSTEM$DBT_GET_LAST_RUN_TARGET ( 'p1' ) , 'm' AS 'a' ) ENV_VARS = ( m1 = 'm' )", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a dbt import alias is a string", sql: "EXECUTE DBT PROJECT v1 IMPORTS = ( 'm' AS m1 )", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "IF EXISTS is not part of EXECUTE DBT PROJECT", sql: "EXECUTE DBT PROJECT IF EXISTS v1", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a code bundle takes an empty argument list", sql: "EXECUTE CODE BUNDLE v1 ENTRYPOINT = m1 ARGUMENTS = ( ) WITH SPECIFICATION $$ m $$", dialect: Dialect::Snowflake, expected_accepted: true },
    ];
    for test_case in test_cases {
        assert_eq!(
            accepted(test_case.sql, test_case.dialect),
            test_case.expected_accepted,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_snowflake_execute_of_a_job_service_when_parsing_then_the_tree_keeps_its_clauses_in_order(
) {
    const SQL: &str = "EXECUTE JOB SERVICE IN COMPUTE POOL p1 FROM @s1/a SPECIFICATION_TEMPLATE_FILE = 'f' USING ( k => 1 ) ASYNC = TRUE";
    let test_cases = [
        ProjectionTestCase {
            description: "the pool comes first",
            sql: SQL,
            pointer: "/0/execute_object/properties/0/kind",
            expected_value: json!("IN COMPUTE POOL"),
        },
        ProjectionTestCase {
            description: "the stage path is kept whole",
            sql: SQL,
            pointer: "/0/execute_object/properties/1/value/literal/value",
            expected_value: json!("@s1/a"),
        },
        ProjectionTestCase {
            description: "the template file follows the stage",
            sql: SQL,
            pointer: "/0/execute_object/properties/2/kind",
            expected_value: json!("SPECIFICATION_TEMPLATE_FILE"),
        },
        ProjectionTestCase {
            description: "the USING arguments keep their name",
            sql: SQL,
            pointer: "/0/execute_object/properties/3/value/tuple/expressions/0/tuple/expressions/0/identifier/name",
            expected_value: json!("k"),
        },
        ProjectionTestCase {
            description: "the options come last",
            sql: SQL,
            pointer: "/0/execute_object/properties/4/kind",
            expected_value: json!("ASYNC"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            statement_at(test_case.sql, test_case.pointer),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}
