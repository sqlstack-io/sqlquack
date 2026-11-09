use crate::ast_node::_helpers::naming::serde_snake_case;
use crate::ast_node::tests::test_types::SnakeCaseTestCase;

#[test]
fn given_variant_name_when_snake_casing_then_matches_serde_serialization() {
    let test_cases = [
        SnakeCaseTestCase {
            description: "a single word is lowercased",
            variant: "Literal",
            expected_snake_case: "literal",
        },
        SnakeCaseTestCase {
            description: "each capital starts a word",
            variant: "ILike",
            expected_snake_case: "i_like",
        },
        SnakeCaseTestCase {
            description: "an acronym splits per letter",
            variant: "JSONBExists",
            expected_snake_case: "j_s_o_n_b_exists",
        },
        SnakeCaseTestCase {
            description: "digits stay with the previous letter",
            variant: "SHA2Digest",
            expected_snake_case: "s_h_a2_digest",
        },
        SnakeCaseTestCase {
            description: "a trailing acronym splits per letter",
            variant: "CurrentTimestampLTZ",
            expected_snake_case: "current_timestamp_l_t_z",
        },
        SnakeCaseTestCase {
            description: "a two-letter suffix splits",
            variant: "PropertyEQ",
            expected_snake_case: "property_e_q",
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            serde_snake_case(test_case.variant),
            test_case.expected_snake_case,
            "{}",
            test_case.description
        );
    }
}
