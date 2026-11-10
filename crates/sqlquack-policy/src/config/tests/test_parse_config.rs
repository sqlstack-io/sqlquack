use crate::config::main::parse_config::parse_config;
use crate::config::tests::test_types::ConfigTestCase;
use crate::policy::main::evidence_policy::evidence_policy;

#[test]
fn given_config_text_when_parsing_then_only_the_user_schema_and_known_names_pass() {
    let test_cases = [
        ConfigTestCase {
            description: "every section, each with its own keys",
            text: "[project]\nsources = \"src\"\npython_runtime = true\n\n[project.session]\n\
                   TIMEZONE = \"UTC\"\n\n[allow]\nhash = true\nrandom = false\n\n[replace]\n\
                   folder = \"mine\"\nverified = { ARRAY_SORT = \"00\" }\n",
            expected_error: None,
        },
        ConfigTestCase {
            description: "a misspelt known difference names the one it is near",
            text: "[allow]\nhsah = true\n",
            expected_error: Some("[allow] names no known difference `hsah`; did you mean `hash`?"),
        },
        ConfigTestCase {
            description: "a name near none points to the list",
            text: "[allow]\nfn_hash = true\n",
            expected_error: Some(
                "[allow] names no known difference `fn_hash`; `sqlquack explain --differences` \
                 lists them",
            ),
        },
        ConfigTestCase {
            description: "Python functions are allowed by the runtime setting, never by [allow]",
            text: "[allow]\nproject_udf = true\n",
            expected_error: Some("[allow] cannot allow `project_udf`: set python_runtime = true"),
        },
        ConfigTestCase {
            description: "an evidence table is not part of a user's file",
            text: "[divergences.hash]\ndefault = \"warn\"\n",
            expected_error: Some("unknown field `divergences`"),
        },
        ConfigTestCase {
            description: "a file from before 0.4 points to [allow] and the list of names",
            text: "[divergences.hash]\ndefault = \"warn\"\n",
            expected_error: Some(
                "list each known difference as `name = true` or `false` under [allow]; \
                 `sqlquack explain --differences` lists them",
            ),
        },
        ConfigTestCase {
            description: "an allowance is true or false, not a mode",
            text: "[allow]\nhash = \"warn\"\n",
            expected_error: Some("invalid type: string \"warn\", expected a boolean"),
        },
    ];
    for test_case in test_cases {
        let error = parse_config(test_case.text, evidence_policy()).err();
        assert_eq!(
            error.as_deref().map(|error| test_case
                .expected_error
                .filter(|expected| error.contains(expected))
                .unwrap_or(error)),
            test_case.expected_error,
            "{}",
            test_case.description
        );
    }
}
