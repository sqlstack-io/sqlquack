use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::main::cast::cast;
use crate::blocks::main::decimal_arithmetic::decimal_arithmetic;
use crate::blocks::tests::helpers::number;
use crate::blocks::tests::test_types::{ArithmeticTestCase, CastTestCase};

#[test]
fn given_a_literal_cast_to_a_number_when_widened_then_only_exact_casts_merge() {
    let test_cases = [
        CastTestCase {
            description: "a literal its own type holds, widened, is cast once",
            sql: "CAST(1.5 AS DECIMAL(2,1))",
            target: number(38, 1),
            expected_sql: "CAST(1.5 AS DECIMAL(38,1))",
        },
        CastTestCase {
            description: "a leading zero is not a whole digit",
            sql: "CAST(0.5 AS DECIMAL(1,1))",
            target: number(38, 1),
            expected_sql: "CAST(0.5 AS DECIMAL(38,1))",
        },
        CastTestCase {
            description: "a literal of more than 38 digits, leading zeros included, is a DOUBLE: \
                          the inner cast rounds it back",
            sql: "CAST(00000000000000000000000000000000000000000.1 AS DECIMAL(1,1))",
            target: number(38, 30),
            expected_sql: "CAST(CAST(00000000000000000000000000000000000000000.1 AS DECIMAL(1,1)) AS DECIMAL(38,30))",
        },
        CastTestCase {
            description: "a literal too wide for its own type keeps the cast that fails",
            sql: "CAST(12 AS DECIMAL(2,1))",
            target: number(38, 1),
            expected_sql: "CAST(CAST(12 AS DECIMAL(2,1)) AS DECIMAL(38,1))",
        },
        CastTestCase {
            description: "a fraction longer than its own scale keeps the cast that rounds",
            sql: "CAST(1.25 AS DECIMAL(2,1))",
            target: number(38, 2),
            expected_sql: "CAST(CAST(1.25 AS DECIMAL(2,1)) AS DECIMAL(38,2))",
        },
        CastTestCase {
            description: "a target with fewer places keeps both casts",
            sql: "CAST(1.5 AS DECIMAL(2,1))",
            target: number(38, 0),
            expected_sql: "CAST(CAST(1.5 AS DECIMAL(2,1)) AS DECIMAL(38,0))",
        },
        CastTestCase {
            description: "a target with fewer whole digits keeps both casts",
            sql: "CAST(10 AS DECIMAL(2,0))",
            target: number(2, 1),
            expected_sql: "CAST(CAST(10 AS DECIMAL(2,0)) AS DECIMAL(2,1))",
        },
        CastTestCase {
            description: "a negated literal keeps both casts",
            sql: "CAST(-1 AS DECIMAL(1,0))",
            target: number(38, 0),
            expected_sql: "CAST(CAST(-1 AS DECIMAL(1,0)) AS DECIMAL(38,0))",
        },
        CastTestCase {
            description: "a cast of a column keeps both casts",
            sql: "CAST(\"N\" AS DECIMAL(10,0))",
            target: number(38, 0),
            expected_sql: "CAST(CAST(\"N\" AS DECIMAL(10,0)) AS DECIMAL(38,0))",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            cast(test_case.sql, test_case.target, &ProvenanceLog::default()).as_deref(),
            Ok(test_case.expected_sql),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_literal_operand_when_widened_for_arithmetic_then_only_exact_casts_merge() {
    let test_cases = [
        ArithmeticTestCase {
            description: "a literal its own type holds is widened with one cast",
            operands: [
                ("CAST(1.5 AS DECIMAL(2,1))", 1),
                ("CAST(1 AS DECIMAL(1,0))", 0),
            ],
            expected_sql: "CAST((CAST(1.5 AS DECIMAL(38,1)) + CAST(1 AS DECIMAL(38,0))) AS DECIMAL(3,1))",
        },
        ArithmeticTestCase {
            description: "a literal too wide for its own type keeps the cast that fails",
            operands: [
                ("CAST(12 AS DECIMAL(2,1))", 1),
                ("CAST(1 AS DECIMAL(1,0))", 0),
            ],
            expected_sql: "CAST((CAST(CAST(12 AS DECIMAL(2,1)) AS DECIMAL(38,1)) + \
                       CAST(1 AS DECIMAL(38,0))) AS DECIMAL(3,1))",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            decimal_arithmetic(
                "+",
                test_case.operands,
                number(3, 1),
                &ProvenanceLog::default()
            )
            .as_deref(),
            Ok(test_case.expected_sql),
            "{}",
            test_case.description
        );
    }
}
