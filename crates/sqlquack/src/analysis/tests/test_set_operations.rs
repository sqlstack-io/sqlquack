use crate::analysis::tests::helpers::shop_result;
use crate::analysis::tests::test_types::SetResultTestCase;

/// `count` UNION ALL branches, the `index`th selecting the items `item` writes for it.
fn branches(count: usize, item: fn(usize) -> String) -> String {
    (1..=count)
        .map(|index| format!("SELECT {}", item(index)))
        .collect::<Vec<String>>()
        .join(" UNION ALL ")
}

/// Rows of text cells, sorted as the helper sorts them.
fn rows(cells: &[&[&str]]) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = cells
        .iter()
        .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
        .collect();
    rows.sort();
    rows
}

#[test]
fn given_set_operations_when_converting_then_names_types_and_rows_are_snowflakes() {
    let numbered = |count: usize| {
        let mut numbered: Vec<Vec<String>> = (1..=count)
            .map(|index| vec![index.to_string(), format!("r{index}")])
            .collect();
        numbered.sort();
        numbered
    };
    let test_cases = [
        SetResultTestCase {
            description: "integer literals of one and two digits across twelve branches",
            sql: branches(12, |index| format!("{index} AS k, 'r{index}' AS s")),
            expected_columns: vec!["K DECIMAL(2,0)", "S VARCHAR"],
            expected_rows: numbered(12),
        },
        SetResultTestCase {
            description: "an 18-digit literal beside small ones, and a 19-digit one in another column",
            sql: branches(12, |index| match index {
                1 => "1234567890123456789 AS k, 999999999999999999 AS m".to_owned(),
                _ => format!("{index} AS k, {index} AS m"),
            }),
            expected_columns: vec!["K DECIMAL(19,0)", "M DECIMAL(18,0)"],
            expected_rows: rows(&[
                &["1234567890123456789", "999999999999999999"],
                &["2", "2"],
                &["3", "3"],
                &["4", "4"],
                &["5", "5"],
                &["6", "6"],
                &["7", "7"],
                &["8", "8"],
                &["9", "9"],
                &["10", "10"],
                &["11", "11"],
                &["12", "12"],
            ]),
        },
        SetResultTestCase {
            description: "the result takes the first branch's names, not the second's",
            sql: "SELECT 1 AS a, 'x' AS b UNION ALL SELECT 2 AS z, 'y' AS w".to_owned(),
            expected_columns: vec!["A DECIMAL(1,0)", "B VARCHAR"],
            expected_rows: rows(&[&["1", "x"], &["2", "y"]]),
        },
        SetResultTestCase {
            description: "UNION ALL BY NAME matches the second branch's columns by name",
            sql: "SELECT 1 AS a, 'x' AS b UNION ALL BY NAME SELECT 'y' AS b, 2 AS a".to_owned(),
            expected_columns: vec!["A DECIMAL(1,0)", "B VARCHAR"],
            expected_rows: rows(&[&["1", "x"], &["2", "y"]]),
        },
        SetResultTestCase {
            description: "a parenthesised UNION ALL on the right of EXCEPT stays one branch",
            sql: "SELECT 1 AS x EXCEPT (SELECT 2 AS x UNION ALL SELECT 1 AS x)".to_owned(),
            expected_columns: vec!["X DECIMAL(1,0)"],
            expected_rows: Vec::new(),
        },
        SetResultTestCase {
            description: "a parenthesised UNION ALL on the left of INTERSECT stays one branch",
            sql: "(SELECT 1 AS x UNION ALL SELECT 2 AS x) INTERSECT SELECT 2 AS x".to_owned(),
            expected_columns: vec!["X DECIMAL(1,0)"],
            expected_rows: rows(&[&["2"]]),
        },
        SetResultTestCase {
            description: "a long UNION ALL as the second branch of a UNION",
            sql: format!(
                "SELECT 100 AS k, 'r100' AS s UNION ({})",
                branches(12, |index| format!("{index} AS k, 'r{index}' AS s"))
            ),
            expected_columns: vec!["K DECIMAL(3,0)", "S VARCHAR"],
            expected_rows: {
                let mut expected = numbered(12);
                expected.push(vec!["100".to_owned(), "r100".to_owned()]);
                expected.sort();
                expected
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            shop_result(&test_case.sql),
            Ok((
                test_case
                    .expected_columns
                    .iter()
                    .map(|column| (*column).to_owned())
                    .collect(),
                test_case.expected_rows
            )),
            "{}",
            test_case.description
        );
    }
}
