use crate::helpers::{project_convert, source_failure};
use crate::test_types::{ProjectFile, ProjectTestCase, SourceErrorTestCase};

/// The repository's policy file, as a project's `sqlquack.toml`.
const POLICY_FILE: ProjectFile = ProjectFile {
    path: "sqlquack.toml",
    repository: &["sqlquack.toml"],
    text: "",
};

/// A SQLBuild source file, with keys SQLQuack does not read.
const SOURCES: ProjectFile = ProjectFile {
    path: "sources/raw.yml",
    repository: &[],
    text: "sources:\n  - name: raw__orders\n    description: Orders, as loaded\n    contract: enforced\n    columns:\n      - name: id\n        type: INTEGER\n        nullable: false\n      - name: status\n        type: VARCHAR\n        audits: [not_null]\n  - name: raw__inline\n    expression: SELECT 1 AS id\n    columns:\n      - name: id\n        type: INTEGER\n",
};
/// A SQLBuild SQL function file, its description holding a comma and parentheses.
const SQL_FUNCTION: ProjectFile = ProjectFile {
    path: "functions/sql/is_done.sql",
    repository: &[],
    text: "FUNCTION (\n  description \"Whether a status counts as done (shipped, or delivered)\",\n  arguments (order_status VARCHAR),\n  returns BOOLEAN,\n);\n\norder_status IN ('shipped', 'delivered')\n",
};
/// A SQLBuild Python function file, unchanged.
const PYTHON_FUNCTION: ProjectFile = ProjectFile {
    path: "functions/python/shout.py",
    repository: &[],
    text: "from sqlbuild.functions import udf\n\n\n@udf(\n    arguments={\"text\": \"VARCHAR\"},\n    returns=\"VARCHAR\",\n    runtime_version=\"3.11\",\n    description=\"\"\"Upper case, with a comma, here.\"\"\",\n)\ndef main(text: str | None) -> str | None:\n    return None if text is None else text.upper()\n",
};

#[test]
fn given_a_sqlbuild_project_when_converting_then_its_folders_declare_tables_and_functions() {
    let test_cases = [
        ProjectTestCase {
            description: "a source table types the query",
            files: &[POLICY_FILE, SOURCES],
            folder: "",
            flags: &[],
            sql: "SELECT id + 1 AS n FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "\"converted\": true",
        },
        ProjectTestCase {
            description: "an inline-expression source is not a table, so a query of it refuses",
            files: &[POLICY_FILE, SOURCES],
            folder: "",
            flags: &[],
            sql: "SELECT id + 1 AS n FROM raw__inline",
            expected_exit: Some(1),
            expected_output: "\"converted\": false",
        },
        ProjectTestCase {
            description: "a SQL function file declares a function the query may call",
            files: &[POLICY_FILE, SOURCES, SQL_FUNCTION],
            folder: "",
            flags: &[],
            sql: "SELECT is_done(status) AS d FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "\"converted\": true",
        },
        ProjectTestCase {
            description: "a Python function file is listed once the project confirms its runtime",
            files: &[
                ProjectFile {
                    path: "sqlquack.toml",
                    repository: &["sqlquack.toml"],
                    text: "[project.confirm]\npython_runtime = true\n",
                },
                SOURCES,
                PYTHON_FUNCTION,
            ],
            folder: "",
            flags: &[],
            sql: "SELECT shout(status) AS s FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "\"name\": \"SHOUT\"",
        },
        ProjectTestCase {
            description: "a Python function's arguments keep their written order, not sorted by name",
            files: &[
                ProjectFile {
                    path: "sqlquack.toml",
                    repository: &["sqlquack.toml"],
                    text: "[project.confirm]\npython_runtime = true\n",
                },
                SOURCES,
                ProjectFile {
                    path: "functions/python/label.py",
                    repository: &[],
                    text: "from sqlbuild.functions import udf\n\n@udf(arguments={'name': 'VARCHAR', 'amount': 'NUMBER(10,2)'}, returns='VARCHAR')\ndef main(name, amount):\n    return name\n",
                },
            ],
            folder: "",
            flags: &[],
            sql: "SELECT label(status, id) AS l FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "\"arguments\": [\n        \"VARCHAR\",\n        \"DECIMAL(10,2)\"\n      ]",
        },
        ProjectTestCase {
            description: "a Python function file may import udf from sqlquack instead",
            files: &[
                ProjectFile {
                    path: "sqlquack.toml",
                    repository: &["sqlquack.toml"],
                    text: "[project.confirm]\npython_runtime = true\n",
                },
                SOURCES,
                ProjectFile {
                    path: "functions/python/shout.py",
                    repository: &[],
                    text: "from sqlquack import udf\n\n@udf(arguments={'text': 'VARCHAR'}, returns='VARCHAR')\ndef main(text):\n    return text\n",
                },
            ],
            folder: "",
            flags: &[],
            sql: "SELECT shout(status) AS s FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "\"name\": \"SHOUT\"",
        },
        ProjectTestCase {
            description: "folders named in [project] replace the defaults",
            files: &[
                ProjectFile {
                    path: "sqlquack.toml",
                    repository: &["sqlquack.toml"],
                    text: "[project]\nsources = \"tables\"\n",
                },
                ProjectFile {
                    path: "tables/raw.yml",
                    repository: &[],
                    text: SOURCES.text,
                },
            ],
            folder: "",
            flags: &[],
            sql: "SELECT id + 1 AS n FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "\"converted\": true",
        },
    ];
    for test_case in test_cases {
        let outcome = project_convert(&test_case);
        assert_eq!(
            outcome.exit_code, test_case.expected_exit,
            "{}: {}",
            test_case.description, outcome.stderr
        );
        assert!(
            outcome.stdout.contains(test_case.expected_output),
            "{}: {}",
            test_case.description,
            outcome.stdout
        );
    }
}

#[test]
fn given_project_settings_when_flags_are_given_then_the_flags_win_for_the_run() {
    let test_cases = [
        ProjectTestCase {
            description: "the project's session applies without flags",
            files: &[
                ProjectFile {
                    path: "sqlquack.toml",
                    repository: &["sqlquack.toml"],
                    text: "[project.session]\nTIMEZONE = \"America/New_York\"\n",
                },
                SOURCES,
            ],
            folder: "",
            flags: &[],
            sql: "SELECT id AS n FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "TIMEZONE = America/New_York is not UTC",
        },
        ProjectTestCase {
            description: "a --session flag replaces the project's value of that parameter",
            files: &[
                ProjectFile {
                    path: "sqlquack.toml",
                    repository: &["sqlquack.toml"],
                    text: "[project.session]\nTIMEZONE = \"America/New_York\"\n",
                },
                SOURCES,
            ],
            folder: "",
            flags: &["--session", "TIMEZONE=UTC"],
            sql: "SELECT id AS n FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "\"notes\": []",
        },
        ProjectTestCase {
            description: "a --python-runtime flag confirms what the project does not",
            files: &[POLICY_FILE, SOURCES, PYTHON_FUNCTION],
            folder: "",
            flags: &["--python-runtime"],
            sql: "SELECT shout(status) AS s FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "\"name\": \"SHOUT\"",
        },
        ProjectTestCase {
            description: "--overrides replaces the project's overrides folder",
            files: &[
                POLICY_FILE,
                SOURCES,
                ProjectFile {
                    path: "overrides/broken.sql",
                    repository: &[],
                    text: "not an override",
                },
                ProjectFile {
                    path: "elsewhere/plain.sql",
                    repository: &[],
                    text: "FUNCTION (\n  arguments (x VARCHAR),\n  returns VARCHAR,\n  replaces UPPER,\n  dialect duckdb\n);\nupper(x)\n",
                },
            ],
            folder: "",
            flags: &["--overrides", "elsewhere"],
            sql: "SELECT UPPER(status) AS s FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "\"converted\": true",
        },
    ];
    for test_case in test_cases {
        let outcome = project_convert(&test_case);
        assert_eq!(
            outcome.exit_code, test_case.expected_exit,
            "{}: {}",
            test_case.description, outcome.stderr
        );
        assert!(
            outcome.stdout.contains(test_case.expected_output),
            "{}: {}",
            test_case.description,
            outcome.stdout
        );
    }
}

#[test]
fn given_a_project_when_a_declaration_clashes_or_is_unconfirmed_then_the_error_says_what_to_change()
{
    let test_cases = [
        ProjectTestCase {
            description: "a table in two source files is an error naming both",
            files: &[
                POLICY_FILE,
                SOURCES,
                ProjectFile {
                    path: "sources/staging/raw.yml",
                    repository: &[],
                    text: "sources:\n  - name: RAW__ORDERS\n    columns:\n      - name: id\n        type: INTEGER\n",
                },
            ],
            folder: "",
            flags: &[],
            sql: "SELECT id AS n FROM raw__orders",
            expected_exit: Some(2),
            expected_output: "the table RAW__ORDERS is declared twice, in ./sources/raw.yml and in ./sources/staging/raw.yml",
        },
        ProjectTestCase {
            description: "a SQL and a Python function of one name is an error",
            files: &[
                POLICY_FILE,
                SQL_FUNCTION,
                ProjectFile {
                    path: "functions/python/is_done.py",
                    repository: &[],
                    text: PYTHON_FUNCTION.text,
                },
            ],
            folder: "",
            flags: &[],
            sql: "SELECT 1 AS n",
            expected_exit: Some(2),
            expected_output: "the function IS_DONE is declared twice",
        },
        ProjectTestCase {
            description: "a Python function without the confirmation names the line to add",
            files: &[POLICY_FILE, SOURCES, PYTHON_FUNCTION],
            folder: "",
            flags: &[],
            sql: "SELECT shout(status) AS s FROM raw__orders",
            expected_exit: Some(1),
            expected_output: "add `python_runtime = true` under [project.confirm] in sqlquack.toml, or pass --python-runtime for one run",
        },
        ProjectTestCase {
            description: "a source listing no columns is left out with a warning",
            files: &[
                POLICY_FILE,
                ProjectFile {
                    path: "sources/raw.yml",
                    repository: &[],
                    text: "sources:\n  - name: raw__empty\n",
                },
            ],
            folder: "",
            flags: &[],
            sql: "SELECT 1 AS n",
            expected_exit: Some(0),
            expected_output: "sqlquack convert: warning: ./sources/raw.yml: the source raw__empty is left out, because it lists no columns",
        },
        ProjectTestCase {
            description: "a YAML merge key brings in another mapping's columns, as PyYAML does",
            files: &[
                POLICY_FILE,
                ProjectFile {
                    path: "sources/raw.yml",
                    repository: &[],
                    text: "base: &base\n  columns:\n    - name: amount\n      type: NUMBER(10,2)\nsources:\n  - name: raw__payments\n    <<: *base\n",
                },
            ],
            folder: "",
            flags: &[],
            sql: "SELECT amount * 2 AS doubled FROM raw__payments",
            expected_exit: Some(0),
            expected_output: "\"converted\": true",
        },
        ProjectTestCase {
            description: "a source with an untyped column is left out with a warning naming it",
            files: &[
                POLICY_FILE,
                ProjectFile {
                    path: "sources/raw.yml",
                    repository: &[],
                    text: "sources:\n  - name: raw__customers\n    columns:\n      - name: id\n        type: INTEGER\n      - name: email\n",
                },
            ],
            folder: "",
            flags: &[],
            sql: "SELECT id AS n FROM raw__customers",
            expected_exit: Some(1),
            expected_output: "sqlquack convert: warning: ./sources/raw.yml: the source raw__customers is left out, because its column email has no type",
        },
        ProjectTestCase {
            description: "a Python function file not marked as SQLBuild marks one is an error",
            files: &[
                POLICY_FILE,
                ProjectFile {
                    path: "functions/python/plain.py",
                    repository: &[],
                    text: "def main(text):\n    return text\n",
                },
            ],
            folder: "",
            flags: &[],
            sql: "SELECT 1 AS n",
            expected_exit: Some(2),
            expected_output: "plain.py: has no @udf(...) decorator",
        },
        ProjectTestCase {
            description: "a Python function naming one argument twice is an error, as Python keeps only the last",
            files: &[
                POLICY_FILE,
                SOURCES,
                ProjectFile {
                    path: "functions/python/label.py",
                    repository: &[],
                    text: "from sqlbuild.functions import udf\n\n@udf(arguments={'a': 'INTEGER', 'a': 'VARCHAR'}, returns='VARCHAR')\ndef main(a):\n    return a\n",
                },
            ],
            folder: "",
            flags: &[],
            sql: "SELECT 1 AS n",
            expected_exit: Some(2),
            expected_output: "line 3: the argument a is given twice; Python keeps only the last",
        },
    ];
    for test_case in test_cases {
        let outcome = project_convert(&test_case);
        assert_eq!(
            outcome.exit_code, test_case.expected_exit,
            "{}: {}",
            test_case.description, outcome.stderr
        );
        assert!(
            outcome.stdout.contains(test_case.expected_output)
                || outcome.stderr.contains(test_case.expected_output),
            "{}: {}{}",
            test_case.description,
            outcome.stdout,
            outcome.stderr
        );
    }
}

#[test]
fn given_a_project_folder_when_converting_then_it_needs_no_policy_and_may_live_elsewhere() {
    let test_cases = [
        ProjectTestCase {
            description: "sources are read without any sqlquack.toml",
            files: &[SOURCES],
            folder: "",
            flags: &[],
            sql: "SELECT id + 1 AS n FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "\"converted\": true",
        },
        ProjectTestCase {
            description: "a sqlquack.toml holding only [project] chooses the folders",
            files: &[
                ProjectFile {
                    path: "sqlquack.toml",
                    repository: &[],
                    text: "[project]\nsources = \"tables\"\n",
                },
                ProjectFile {
                    path: "tables/raw.yml",
                    repository: &[],
                    text: SOURCES.text,
                },
            ],
            folder: "",
            flags: &[],
            sql: "SELECT id + 1 AS n FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "\"converted\": true",
        },
        ProjectTestCase {
            description: "a sqlquack.toml holding only [project] keeps the built-in policy, which refuses RANDOM",
            files: &[
                ProjectFile {
                    path: "sqlquack.toml",
                    repository: &[],
                    text: "[project]\nsources = \"sources\"\n",
                },
                SOURCES,
            ],
            folder: "",
            flags: &[],
            sql: "SELECT RANDOM() AS r FROM raw__orders",
            expected_exit: Some(1),
            expected_output: "\"converted\": false",
        },
        ProjectTestCase {
            description: "--project reads a project in another folder",
            files: &[POLICY_FILE, SOURCES],
            folder: "warehouse",
            flags: &["--project", "warehouse"],
            sql: "SELECT id + 1 AS n FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "\"converted\": true",
        },
        ProjectTestCase {
            description: "without --project a project in another folder is not read",
            files: &[POLICY_FILE, SOURCES],
            folder: "warehouse",
            flags: &[],
            sql: "SELECT id + 1 AS n FROM raw__orders",
            expected_exit: Some(1),
            expected_output: "\"converted\": false",
        },
        ProjectTestCase {
            description: "--builtin-policy replaces the policy but keeps the project's sources",
            files: &[POLICY_FILE, SOURCES],
            folder: "",
            flags: &["--builtin-policy"],
            sql: "SELECT id + 1 AS n FROM raw__orders",
            expected_exit: Some(0),
            expected_output: "\"converted\": true",
        },
        ProjectTestCase {
            description: "a --project folder that does not exist is an error",
            files: &[],
            folder: "",
            flags: &["--project", "missing"],
            sql: "SELECT 1 AS n",
            expected_exit: Some(2),
            expected_output: "the project folder missing does not exist",
        },
    ];
    for test_case in test_cases {
        let outcome = project_convert(&test_case);
        assert_eq!(
            outcome.exit_code, test_case.expected_exit,
            "{}: {}",
            test_case.description, outcome.stderr
        );
        assert!(
            outcome.stdout.contains(test_case.expected_output)
                || outcome.stderr.contains(test_case.expected_output),
            "{}: {}{}",
            test_case.description,
            outcome.stdout,
            outcome.stderr
        );
    }
}

#[test]
fn given_a_malformed_source_file_when_converting_then_the_error_names_what_is_wrong_in_source_terms()
 {
    let test_cases = [
        SourceErrorTestCase {
            description: "columns not listed",
            sources: "sources:\n  - name: orders\n    columns:\n      a: NUMBER\n",
            expected_error: "orders.yml: is not a source file: invalid type: map, expected a sequence",
        },
        SourceErrorTestCase {
            description: "a source without a name",
            sources: "sources:\n  - columns:\n      - name: a\n        type: NUMBER\n",
            expected_error: "orders.yml: is not a source file: missing field `name`",
        },
        SourceErrorTestCase {
            description: "text that is not YAML",
            sources: "sources:\n  - name: [orders\n",
            expected_error: "orders.yml: line 3 column 1: while parsing a flow sequence",
        },
    ];
    for test_case in test_cases {
        let (exit_code, stderr) = source_failure(test_case.sources);

        assert_eq!(exit_code, Some(2), "{}: {stderr}", test_case.description);
        assert!(
            stderr.contains(test_case.expected_error) && !stderr.contains("untagged"),
            "{}: {stderr}",
            test_case.description
        );
    }
}
