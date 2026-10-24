use crate::comparison::models::{MismatchKind, RowOrder};
use crate::comparison::tests::helpers::{
    column, date, decimal, float64, int64, replay_mismatch, text, timestamp_ns, timestamp_us,
};
use crate::comparison::tests::test_types::ReplayTestCase;

#[test]
fn given_recorded_and_replayed_results_when_comparing_then_only_exact_agreement_matches() {
    let test_cases = [
        ReplayTestCase {
            description: "an integer recorded as int64 equals the same DECIMAL(38,0) value",
            recorded: vec![column("TOTAL", "NUMBER(38,0)", int64(&[Some(3), None]))],
            replayed: vec![column("total", "", decimal(&[Some(3), None], 38, 0))],
            order: RowOrder::Ordered,
            float64_ulps: 0,
            expected_mismatch: None,
        },
        ReplayTestCase {
            description: "a different decimal value",
            recorded: vec![column("V", "NUMBER(10,2)", decimal(&[Some(150)], 10, 2))],
            replayed: vec![column("v", "", decimal(&[Some(151)], 10, 2))],
            order: RowOrder::Ordered,
            float64_ulps: 0,
            expected_mismatch: Some(MismatchKind::Value),
        },
        ReplayTestCase {
            description: "a scale no declared equality admits",
            recorded: vec![column("V", "NUMBER(10,2)", decimal(&[Some(150)], 10, 2))],
            replayed: vec![column("v", "", decimal(&[Some(1500)], 10, 3))],
            order: RowOrder::Ordered,
            float64_ulps: 0,
            expected_mismatch: Some(MismatchKind::ColumnType),
        },
        ReplayTestCase {
            description: "NULL is not an empty string",
            recorded: vec![column("S", "VARCHAR(16777216)", text(&[None]))],
            replayed: vec![column("s", "", text(&[Some("")]))],
            order: RowOrder::Ordered,
            float64_ulps: 0,
            expected_mismatch: Some(MismatchKind::Value),
        },
        ReplayTestCase {
            description: "a renamed column",
            recorded: vec![column("S", "VARCHAR(16777216)", text(&[Some("a")]))],
            replayed: vec![column("t", "", text(&[Some("a")]))],
            order: RowOrder::Ordered,
            float64_ulps: 0,
            expected_mismatch: Some(MismatchKind::ColumnName),
        },
        ReplayTestCase {
            description: "an extra column",
            recorded: vec![column("S", "VARCHAR(16777216)", text(&[Some("a")]))],
            replayed: vec![
                column("s", "", text(&[Some("a")])),
                column("t", "", text(&[Some("a")])),
            ],
            order: RowOrder::Ordered,
            float64_ulps: 0,
            expected_mismatch: Some(MismatchKind::ColumnCount),
        },
        ReplayTestCase {
            description: "reordered rows fail an ordered comparison",
            recorded: vec![column(
                "S",
                "VARCHAR(16777216)",
                text(&[Some("a"), Some("b")]),
            )],
            replayed: vec![column("s", "", text(&[Some("b"), Some("a")]))],
            order: RowOrder::Ordered,
            float64_ulps: 0,
            expected_mismatch: Some(MismatchKind::Value),
        },
        ReplayTestCase {
            description: "reordered rows pass a bag comparison",
            recorded: vec![column(
                "S",
                "VARCHAR(16777216)",
                text(&[Some("a"), Some("b")]),
            )],
            replayed: vec![column("s", "", text(&[Some("b"), Some("a")]))],
            order: RowOrder::Bag,
            float64_ulps: 0,
            expected_mismatch: None,
        },
        ReplayTestCase {
            description: "a missing row",
            recorded: vec![column(
                "S",
                "VARCHAR(16777216)",
                text(&[Some("a"), Some("b")]),
            )],
            replayed: vec![column("s", "", text(&[Some("a")]))],
            order: RowOrder::Bag,
            float64_ulps: 0,
            expected_mismatch: Some(MismatchKind::RowCount),
        },
        ReplayTestCase {
            description: "floats compare bitwise, so negative zero differs from zero",
            recorded: vec![column("F", "FLOAT", float64(&[Some(0.0)]))],
            replayed: vec![column("f", "", float64(&[Some(-0.0)]))],
            order: RowOrder::Ordered,
            float64_ulps: 0,
            expected_mismatch: Some(MismatchKind::Value),
        },
        ReplayTestCase {
            description: "every NaN is one value",
            recorded: vec![column("F", "FLOAT", float64(&[Some(f64::NAN)]))],
            replayed: vec![column("f", "", float64(&[Some(-f64::NAN)]))],
            order: RowOrder::Ordered,
            float64_ulps: 0,
            expected_mismatch: None,
        },
        ReplayTestCase {
            description: "one ulp apart fails without tolerance",
            recorded: vec![column("F", "FLOAT", float64(&[Some(0.1)]))],
            replayed: vec![column(
                "f",
                "",
                float64(&[Some(f64::from_bits(0.1_f64.to_bits() + 1))]),
            )],
            order: RowOrder::Ordered,
            float64_ulps: 0,
            expected_mismatch: Some(MismatchKind::Value),
        },
        ReplayTestCase {
            description: "one ulp apart passes a declared one-ulp tolerance",
            recorded: vec![column("F", "FLOAT", float64(&[Some(0.1)]))],
            replayed: vec![column(
                "f",
                "",
                float64(&[Some(f64::from_bits(0.1_f64.to_bits() + 1))]),
            )],
            order: RowOrder::Ordered,
            float64_ulps: 1,
            expected_mismatch: None,
        },
        ReplayTestCase {
            description: "the same instant at different precisions and zone labels",
            recorded: vec![column(
                "T",
                "TIMESTAMP_TZ(9)",
                timestamp_ns(&[Some(1_700_000_000_123_456_000)], "+00:00"),
            )],
            replayed: vec![column(
                "t",
                "",
                timestamp_us(&[Some(1_700_000_000_123_456)], "UTC"),
            )],
            order: RowOrder::Ordered,
            float64_ulps: 0,
            expected_mismatch: None,
        },
        ReplayTestCase {
            description: "a sub-microsecond difference is a value difference",
            recorded: vec![column(
                "T",
                "TIMESTAMP_TZ(9)",
                timestamp_ns(&[Some(1_700_000_000_123_456_789)], "UTC"),
            )],
            replayed: vec![column(
                "t",
                "",
                timestamp_us(&[Some(1_700_000_000_123_456)], "UTC"),
            )],
            order: RowOrder::Ordered,
            float64_ulps: 0,
            expected_mismatch: Some(MismatchKind::Value),
        },
        ReplayTestCase {
            description: "a date against text is a type difference",
            recorded: vec![column("D", "DATE", date(&[Some(19_000)]))],
            replayed: vec![column("d", "", text(&[Some("2022-01-08")]))],
            order: RowOrder::Ordered,
            float64_ulps: 0,
            expected_mismatch: Some(MismatchKind::ColumnType),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            replay_mismatch(
                &test_case.recorded,
                &test_case.replayed,
                test_case.order,
                test_case.float64_ulps
            ),
            test_case.expected_mismatch,
            "{}",
            test_case.description
        );
    }
}
