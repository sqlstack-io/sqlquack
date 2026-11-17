//! A value a gap used to drop stays a gap when keeping it would pass the tree depth limit.

use crate::parse::tests::helpers::verdict;
use crate::parse::tests::test_types::{DeepValueTestCase, ExpectedVerdict};

/// Terms of the sum: past the depth limit, which a dropped value never reached.
const TERMS: usize = 9000;

#[test]
fn given_a_treed_form_holding_a_value_past_the_depth_limit_when_parsing_then_it_stays_an_accepted_gap()
 {
    let test_cases = [
        DeepValueTestCase {
            description: "a row filter's columns",
            shape: "ALTER TABLE t1 SET ROW FILTER f1 ON ( {E} )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        DeepValueTestCase {
            description: "a column mask's arguments in CREATE TABLE",
            shape: "CREATE TABLE t1 ( c1 INT MASK m1 USING COLUMNS ( {E} ) )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        DeepValueTestCase {
            description: "a column mask's arguments on an added column",
            shape: "ALTER TABLE t1 ADD COLUMN c1 INT MASK m1 USING COLUMNS ( {E} )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        DeepValueTestCase {
            description: "CREATE TABLE's row filter",
            shape: "CREATE TABLE t1 ( c1 INT ) WITH ROW FILTER f1 ON ( {E} )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        DeepValueTestCase {
            description: "the partition a rename starts from",
            shape: "ALTER TABLE t1 PARTITION ( c1 = {E} ) RENAME TO PARTITION ( c1 = 1 )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        DeepValueTestCase {
            description: "SHOW PARTITIONS' partition",
            shape: "SHOW PARTITIONS t1 PARTITION ( c1 = {E} )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        DeepValueTestCase {
            description: "RESTORE's timestamp",
            shape: "RESTORE t1 TIMESTAMP AS OF {E}",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        DeepValueTestCase {
            description: "CACHE SELECT's condition, read through its documented form",
            shape: "CACHE SELECT c1 FROM t1 WHERE {E} = 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        DeepValueTestCase {
            description: "a partition DESCRIBE kept before its column is treed is still refused",
            shape: "DESCRIBE TABLE t1 PARTITION ( c1 = {E} ) c2",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        DeepValueTestCase {
            description: "that DESCRIBE as a procedure's body, which its gap leaves opaque",
            shape: "CREATE PROCEDURE p1 ( ) LANGUAGE SQL AS DESCRIBE TABLE t1 PARTITION ( c1 = {E} ) c2",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        DeepValueTestCase {
            description: "a value kept outside any treed form is still refused",
            shape: "ALTER TABLE t1 ADD CONSTRAINT k1 CHECK ( {E} > 0 )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
    ];
    let sum: String = vec!["1"; TERMS].join(" + ");
    for test_case in test_cases {
        let sql: String = test_case.shape.replace("{E}", &sum);
        let found: ExpectedVerdict = std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn(move || verdict(&sql))
            .expect("the parse thread starts")
            .join()
            .expect("the parse thread finishes");
        assert_eq!(
            found, test_case.expected_verdict,
            "{}",
            test_case.description
        );
    }
}
