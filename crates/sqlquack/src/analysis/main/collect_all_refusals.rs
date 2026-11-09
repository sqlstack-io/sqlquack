//! `collect_all_refusals`: every refusal in some SQL, converting past each refused node.

use sqlquack_diagnostics::classification::main::classify::classify;
use sqlquack_diagnostics::classification::models::SourceDialect;
use sqlquack_diagnostics::refusals::main::refuse_constructs::refuse_constructs;
use sqlquack_diagnostics::refusals::main::refuse_unparsed::refuse_unparsed;
use sqlquack_diagnostics::refusals::main::snowflake_rejections::snowflake_rejections;
use sqlquack_diagnostics::refusals::models::RefusalCategory;
use sqlquack_diagnostics::refusals::types::ConstructPolicy;
use sqlquack_parse::parsing::main::parse_source::parse_source;
use sqlquack_rules::conversion::main::collect_refusals::collect_refusals;
use sqlquack_rules::conversion::main::quoted_exclusions::quoted_exclusions;
use sqlquack_rules::conversion::models::{CollectedRefusals, ConversionOptions};
use sqlquack_semantics::types::models::Schema;
use sqlquack_syntax::tokens::models::SourceLines;

use sqlquack_convert::analysis::main::placed::placed;

/// The refusals of every statement; the conversions found on the way are discarded.
pub fn collect_all_refusals(
    sql: &str,
    dialect: SourceDialect,
    policy: &dyn ConstructPolicy,
    (schema, options): (&Schema, &ConversionOptions),
) -> CollectedRefusals {
    let lines = SourceLines::new(sql);
    let options = &ConversionOptions {
        quoted_exclusions: quoted_exclusions(sql),
        ..options.clone()
    };
    let statements = match parse_source(sql, dialect.syntax()) {
        Ok(statements) => statements,
        Err(failure) => {
            return CollectedRefusals {
                refusals: vec![refuse_unparsed(&failure, &lines)],
                possibly_incomplete: false,
                columns: None,
            };
        }
    };
    let mut all = CollectedRefusals::default();
    for statement in &statements {
        let occurrences = classify(statement, &lines, dialect);
        all.refusals.extend(
            refuse_constructs(&occurrences, policy)
                .into_iter()
                .filter(|refusal| refusal.category == RefusalCategory::NotExpressible),
        );
        let rejected = snowflake_rejections(statement, (sql, &lines));
        if !rejected.is_empty() {
            all.refusals.extend(rejected);
            continue;
        }
        let collected = collect_refusals(statement, (schema, options, &lines));
        all.refusals.extend(
            collected
                .refusals
                .into_iter()
                .map(|refusal| placed(refusal, statement, &lines)),
        );
        all.possibly_incomplete |= collected.possibly_incomplete;
        all.columns = match statements.len() {
            1 => collected.columns,
            _ => None,
        };
    }
    all
}
