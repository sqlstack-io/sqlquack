use crate::helpers::{index_repository_cases, index_shop_corpus};
use crate::test_types::{CaseIndexOutcome, CaseIndexTestCase, CorpusIndexTestCase, IndexOutcome};

#[test]
fn given_a_corpus_and_schema_when_indexing_then_building_blocks_map_to_the_files_using_them() {
    let test_cases = [CorpusIndexTestCase {
        description: "each building block lists exactly the corpus files whose conversion used it, \
                          and a refusal its file; HASH still refuses under the built-in policy, \
                          where PARSE_JSON now converts",
        expected_outcome: IndexOutcome {
            exit_code: Some(0),
            text_to_boolean_files: vec!["corpus/active.sql".to_owned()],
            text_to_number_files: vec!["corpus/credit.sql".to_owned()],
            hash_refusals: vec!["corpus/notes.sql".to_owned()],
            converted: 3,
            refused: 1,
        },
    }];
    for test_case in test_cases {
        assert_eq!(
            index_shop_corpus(),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_the_repository_cases_when_indexing_twice_then_the_output_is_identical_and_attributed() {
    let test_cases = [CaseIndexTestCase {
        description: "the case index is stable and attributes a building block to its cases only",
        expected_outcome: CaseIndexOutcome {
            identical_runs: true,
            boolean_text_cases_attributed: true,
            number_to_boolean_not_attributed: true,
        },
    }];
    for test_case in test_cases {
        assert_eq!(
            index_repository_cases(),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
