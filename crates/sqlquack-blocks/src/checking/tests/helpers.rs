use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::checking::errors::CheckStage;
use crate::checking::main::check_printed::check_printed;

/// The stage at which `sql` fails against the relation `ORDERS`, or `None` when it passes.
pub(crate) fn failed_stage(sql: &str) -> Option<CheckStage> {
    failed_stage_over(sql, &["ORDERS"])
}

/// The stage at which `sql` fails against `relations`, each with the columns of `ORDERS`.
pub(crate) fn failed_stage_over(sql: &str, relations: &[&str]) -> Option<CheckStage> {
    let orders = |name: &str| -> Option<Vec<(String, SnowflakeType)>> {
        relations.contains(&name).then(|| {
            vec![
                (
                    "ID".to_owned(),
                    SnowflakeType::Number {
                        precision: 38,
                        scale: 0,
                    },
                ),
                ("DETAILS".to_owned(), SnowflakeType::SemiStructured),
                (
                    "PLACED".to_owned(),
                    SnowflakeType::TimestampNtz { precision: 9 },
                ),
                ("NOTE".to_owned(), SnowflakeType::Null),
            ]
        })
    };
    check_printed(sql, (&orders, &|_| None))
        .err()
        .map(|failure| failure.stage)
}

/// `depth` selects joined by `UNION ALL`, each pair in its own parentheses.
pub(crate) fn nested_union(depth: usize) -> String {
    (2..=depth).fold("(SELECT 1)".to_owned(), |inner, value| {
        format!("({inner} UNION ALL (SELECT {value}))")
    })
}
