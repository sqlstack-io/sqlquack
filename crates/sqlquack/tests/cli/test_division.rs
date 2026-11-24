use crate::helpers::converted_value_over;
use crate::test_types::ValueTestCase;

#[test]
fn given_a_dividend_of_30_places_when_running_then_the_quotient_is_exact_at_its_scale() {
    let sources = r#"sources:
  - name: t
    columns:
      - name: a
        type: NUMBER(30,30)
      - name: b
        type: NUMBER(12,2)
      - name: c
        type: NUMBER(5,0)
"#;
    let setup = "CREATE TABLE \"T\" (\"A\" DECIMAL(30,30), \"B\" DECIMAL(12,2), \"C\" DECIMAL(5,0)); \
                 INSERT INTO \"T\" VALUES (0.035560308504274254737431201230, -9.55, 2), \
                 (-0.000000000000000000000000000005, 1, 2)";
    let test_cases = [
        ValueTestCase {
            description: "a fraction past 19 places divides without overflowing DECIMAL(38)",
            sql: "SELECT (a / b)::VARCHAR AS q FROM t WHERE c = 2 AND b < 0",
            expected_value: "-0.003723592513536571176694366621",
        },
        ValueTestCase {
            description: "a half in the last place rounds away from zero",
            sql: "SELECT (a / c)::NUMBER(38,30)::VARCHAR AS q FROM t WHERE b = 1",
            expected_value: "-0.000000000000000000000000000003",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            converted_value_over(test_case.sql, (sources, setup)),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}
