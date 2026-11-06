//! The exact check on every corpus committed with SQLQuack: zero differences, lossless tokens.

use std::collections::BTreeMap;

use sqlquack_polyglot_check::corpora::main::case_corpus::case_corpus;
use sqlquack_polyglot_check::corpora::main::load_corpus::load_corpus;

use crate::helpers::{exact_outcome, repository};
use crate::test_types::{CorpusTestCase, ExactOutcome};

#[test]
fn given_committed_corpora_when_parsed_by_both_parsers_then_results_are_identical() {
    let root = repository();
    let corpora = root.join("corpora/polyglot-check");
    let test_cases = [
        CorpusTestCase {
            description: "the SQL inputs of polyglot's tests",
            entries: load_corpus(&corpora.join("polyglot_tests.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::from([
                    ("malformed_typed_type_text".to_owned(), 1),
                    ("snowflake_bare_values_cte".to_owned(), 1),
                    ("snowflake_table_unset_projection_policy".to_owned(), 1),
                    ("snowflake_put_prose_with_quoted_file_uri".to_owned(), 1),
                    ("snowflake_put_prose_with_placeholder".to_owned(), 1),
                    ("snowflake_put_prose_with_quoted_stage".to_owned(), 1),
                    ("snowflake_put_prose_minimal_with".to_owned(), 1),
                    ("snowflake_put_prose_roundtrip".to_owned(), 1),
                    (
                        "snowflake_put_prose_with_dollar_quoted_stage_target".to_owned(),
                        1,
                    ),
                    ("snowflake_put_prose_with_uuid_name_path".to_owned(), 1),
                    (
                        "snowflake_put_prose_with_uuid_compressupload_path".to_owned(),
                        1,
                    ),
                    (
                        "snowflake_put_prose_with_stage_uuid_auto_compress_false".to_owned(),
                        1,
                    ),
                    (
                        "snowflake_put_prose_with_stage_uuid_auto_compress_true".to_owned(),
                        1,
                    ),
                    (
                        "snowflake_put_prose_with_long_path_stage_uuid".to_owned(),
                        1,
                    ),
                    ("snowflake_cte_without_as_prose".to_owned(), 1),
                    ("snowflake_copy_prose_into_with".to_owned(), 1),
                    ("snowflake_copy_prose_on_error".to_owned(), 1),
                    ("snowflake_copy_prose_reversed".to_owned(), 1),
                    ("snowflake_copy_prose_minimal".to_owned(), 1),
                    ("snowflake_copy_prose_roundtrip".to_owned(), 1),
                    ("snowflake_reserved_type_name".to_owned(), 1),
                    ("snowflake_string_part_extract".to_owned(), 1),
                    ("snowflake_function_body_number".to_owned(), 1),
                    ("snowflake_collate_needs_string".to_owned(), 1),
                    ("snowflake_dot_after_literal".to_owned(), 1),
                    ("snowflake_by_name_on_except".to_owned(), 1),
                    ("snowflake_by_name_on_intersect".to_owned(), 1),
                    ("snowflake_empty_select_list".to_owned(), 1),
                    ("snowflake_empty_select_list_before_from".to_owned(), 1),
                    (
                        "snowflake_empty_select_list_keyword_qualifier".to_owned(),
                        1,
                    ),
                    ("snowflake_empty_select_list_lower_case".to_owned(), 1),
                    ("snowflake_empty_select_list_stage_text".to_owned(), 1),
                    (
                        "snowflake_empty_select_list_trailing_order_by".to_owned(),
                        1,
                    ),
                    ("snowflake_empty_select_list_trailing_space".to_owned(), 1),
                    ("snowflake_reserved_distinct_alias".to_owned(), 1),
                    ("snowflake_reserved_is_alias".to_owned(), 1),
                    ("snowflake_set_operation_join_side".to_owned(), 1),
                    ("snowflake_describe_bare_name".to_owned(), 1),
                    ("snowflake_show_profile".to_owned(), 1),
                    ("snowflake_show_groups".to_owned(), 1),
                    ("snowflake_show_create_table".to_owned(), 1),
                    ("snowflake_stage_reserved_name".to_owned(), 1),
                    ("snowflake_stage_quoted_blank_name".to_owned(), 1),
                    ("snowflake_list_function_call".to_owned(), 1),
                    ("snowflake_list_function_order_by".to_owned(), 1),
                    ("snowflake_list_without_stage".to_owned(), 1),
                    ("snowflake_unpivot_reserved_value_column".to_owned(), 1),
                    ("snowflake_show_grants_for".to_owned(), 1),
                    ("snowflake_role_grant_without_role".to_owned(), 1),
                    (
                        "snowflake_role_grant_without_role_quoted_user".to_owned(),
                        1,
                    ),
                    (
                        "snowflake_role_grant_without_role_admin_option".to_owned(),
                        1,
                    ),
                    ("snowflake_role_grant_without_role_lists".to_owned(), 1),
                    ("snowflake_role_grant_without_role_user_list".to_owned(), 1),
                    ("snowflake_role_grant_quoted_roles".to_owned(), 1),
                    (
                        "snowflake_role_grant_without_role_after_select".to_owned(),
                        1,
                    ),
                    (
                        "snowflake_role_grant_without_role_unterminated".to_owned(),
                        1,
                    ),
                    ("snowflake_role_revoke_without_role".to_owned(), 1),
                    (
                        "snowflake_role_revoke_admin_option_without_role".to_owned(),
                        1,
                    ),
                    ("snowflake_role_revoke_without_role_lists".to_owned(), 1),
                ]),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "sqlglot's fixtures",
            entries: load_corpus(&corpora.join("sqlglot.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::from([
                    ("snowflake_function_body_return".to_owned(), 1),
                    ("snowflake_function_body_return_contains_sql".to_owned(), 1),
                    ("snowflake_function_body_return_modifies_sql".to_owned(), 1),
                    ("snowflake_function_body_return_reads_sql".to_owned(), 1),
                    ("snowflake_cte_without_as".to_owned(), 1),
                    ("snowflake_empty_call_argument".to_owned(), 1),
                    ("snowflake_reserved_cte_name".to_owned(), 1),
                    ("snowflake_reserved_truncate_target".to_owned(), 1),
                    (
                        "snowflake_copy_unload_reserved_table_credentials".to_owned(),
                        1,
                    ),
                    (
                        "snowflake_copy_unload_reserved_table_credentials_flat".to_owned(),
                        1,
                    ),
                    (
                        "snowflake_copy_unload_reserved_table_integration".to_owned(),
                        1,
                    ),
                    (
                        "snowflake_copy_unload_reserved_table_integration_flat".to_owned(),
                        1,
                    ),
                    (
                        "snowflake_empty_select_list_trailing_order_by".to_owned(),
                        1,
                    ),
                    ("snowflake_reserved_window_alias".to_owned(), 1),
                    ("snowflake_string_part_date_part".to_owned(), 1),
                    ("snowflake_frame_bare_preceding".to_owned(), 1),
                    ("snowflake_frame_value_without_direction".to_owned(), 1),
                    ("snowflake_frame_value_without_direction_end".to_owned(), 1),
                    ("snowflake_describe_bare_name".to_owned(), 1),
                    ("snowflake_describe_bare_qualified_name".to_owned(), 1),
                    ("snowflake_describe_extended".to_owned(), 1),
                    ("snowflake_describe_formatted".to_owned(), 1),
                    ("snowflake_describe_query".to_owned(), 1),
                    ("snowflake_describe_bare_quoted_name".to_owned(), 1),
                    ("snowflake_stage_string_name".to_owned(), 1),
                    ("snowflake_bare_interval_type_double_colon".to_owned(), 1),
                    ("snowflake_bare_interval_type_cast".to_owned(), 1),
                    (
                        "snowflake_bare_interval_type_double_colon_window".to_owned(),
                        1,
                    ),
                    ("snowflake_bare_interval_type_cast_window".to_owned(), 1),
                    ("snowflake_bare_interval_type_cast_string".to_owned(), 1),
                    ("snowflake_int_precision_default_uuid".to_owned(), 1),
                    ("snowflake_view_without_query_columns".to_owned(), 1),
                    ("snowflake_view_without_query_column_comments".to_owned(), 1),
                    ("snowflake_merge_insert_star".to_owned(), 1),
                    ("snowflake_view_row_access_policy_without_on".to_owned(), 1),
                    ("snowflake_int_precision_default_null_comment".to_owned(), 1),
                    ("snowflake_int_precision_not_null_default".to_owned(), 1),
                    (
                        "snowflake_int_precision_not_null_default_negative".to_owned(),
                        1,
                    ),
                    (
                        "snowflake_int_precision_collate_auto_increment".to_owned(),
                        1,
                    ),
                ]),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "sqlglot's PostgreSQL tests",
            entries: load_corpus(&corpora.join("sqlglot_postgres.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::new(),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "PostgreSQL's regression statements",
            entries: load_corpus(&corpora.join("postgres_regress.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::new(),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "sqlglot's SQLite tests",
            entries: load_corpus(&corpora.join("sqlglot_sqlite.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::new(),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "sqllogictest's statements",
            entries: load_corpus(&corpora.join("sqllogictest.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::new(),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "every Snowflake type spelling in each cast form",
            entries: load_corpus(&corpora.join("snowflake_types.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::from([
                    ("snowflake_type_bare_interval_cast".to_owned(), 1),
                    ("snowflake_type_bare_interval_try_cast".to_owned(), 1),
                    ("snowflake_type_bare_interval_double_colon".to_owned(), 1),
                ]),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "SQLQuack's cases, source and converted",
            entries: case_corpus(&root.join("cases"), &root.join("sqlquack.toml")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::new(),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            exact_outcome(&test_case.entries),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
