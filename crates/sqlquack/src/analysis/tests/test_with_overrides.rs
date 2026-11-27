use crate::analysis::main::with_overrides::with_overrides;
use crate::analysis::tests::test_types::WithOverridesTestCase;
use sqlquack_convert::snowflake::conversion::models::ConversionOptions;

#[test]
fn given_override_files_that_cannot_load_when_loading_then_the_error_names_the_files() {
    let test_cases = [
        WithOverridesTestCase {
            description: "two files replacing one function",
            files: &[
                (
                    "hash_a.sql",
                    "FUNCTION (\n  arguments (x VARCHAR),\n  returns NUMBER(19,0),\n  replaces HASH,\n  dialect duckdb\n);\nhash(x)\n",
                ),
                (
                    "hash_b.sql",
                    "FUNCTION (\n  arguments (x VARCHAR),\n  returns NUMBER(19,0),\n  replaces HASH,\n  dialect duckdb\n);\nhash(x)\n",
                ),
            ],
            expected_error: "overrides <dir>/hash_a.sql and <dir>/hash_b.sql both replace HASH",
        },
        WithOverridesTestCase {
            description: "a file without its dialect",
            files: &[(
                "hash_a.sql",
                "FUNCTION (\n  arguments (x VARCHAR),\n  replaces HASH\n);\nhash(x)\n",
            )],
            expected_error: "<dir>/hash_a.sql: override hash_a: needs `dialect duckdb`",
        },
    ];
    for test_case in test_cases {
        let folder = tempfile::tempdir().expect("a temporary folder");
        for (name, text) in test_case.files {
            std::fs::write(folder.path().join(name), text).expect("the file is written");
        }

        let error: Option<String> =
            with_overrides(ConversionOptions::default(), (Some(folder.path()), &[])).err();

        assert_eq!(
            error.map(|error| error.replace(&folder.path().display().to_string(), "<dir>")),
            Some(test_case.expected_error.to_owned()),
            "{}",
            test_case.description
        );
    }
}
