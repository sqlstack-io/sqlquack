//! Pipeline results.

use std::collections::BTreeSet;

use serde::Serialize;
use sqlquack_core::classification::models::Occurrence;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::provenance::main::merge_provenance::merge_provenance;
use sqlquack_core::provenance::models::Provenance;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_evidence::constants::{INPUT_RELATION, ORDINAL_COLUMN};
use sqlquack_evidence::verify::models::Conversion;
use sqlquack_evidence::verify::types::CaseConverter;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_rules::conversion::models::{ConversionOptions, Converted, Fixture, Schema};

use crate::analysis::_helpers::case_functions::case_functions;
use crate::analysis::constants::STATEMENT_REFUSAL;
use crate::analysis::main::analyze::analyze;
use crate::analysis::main::refused_construct::refused_construct;

/// What the pipeline found in one SQL input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Analysis {
    /// Whether the input parsed.
    pub parsed: bool,
    /// Statements parsed.
    pub statements: usize,
    /// Every construct used, in source order.
    pub occurrences: Vec<Occurrence>,
    /// Every refusal; the input converts only when this is empty.
    pub refusals: Vec<Refusal>,
    /// The conversion of each statement that converted.
    pub conversions: Vec<Converted>,
}

/// The production converter: the SQLQuack pipeline with the built-in rule catalogue.
#[derive(Clone, Debug, Default)]
pub struct PipelineConverter {
    /// The known divergences conversion may rely on.
    pub options: ConversionOptions,
}

/// The project functions a case defines, ready for its conversion and replay.
#[derive(Clone, Debug, Default)]
pub struct CaseFunctions {
    /// Their signatures, which the case's calls are typed by.
    pub schema: Schema,
    /// One `CREATE MACRO` per function, in file order.
    pub setup: Vec<String>,
    /// What each body's conversion used.
    pub provenance: Vec<Provenance>,
}

impl CaseConverter for PipelineConverter {
    fn convert(&self, case: &LoadedCase) -> Conversion {
        let options = ConversionOptions {
            fixture: Some(Fixture {
                relation: INPUT_RELATION.to_owned(),
                ordering_column: ORDINAL_COLUMN.to_owned(),
            }),
            ..self.options.clone()
        };
        let functions = match case_functions(&case.functions, &options) {
            Ok(functions) => functions,
            Err(reasons) => {
                return Conversion::Refused {
                    reasons,
                    constructs: vec![STATEMENT_REFUSAL.to_owned()],
                };
            }
        };
        let analysis = analyze(
            &case.source_sql,
            SourceDialect::Snowflake,
            &builtin_catalogue(),
            (&functions.schema, &options),
        );
        let reasons: BTreeSet<String> = analysis
            .refusals
            .iter()
            .map(|refusal| format!("{}: {}", refusal.category, refusal.reason))
            .collect();
        let constructs: BTreeSet<String> =
            analysis.refusals.iter().map(refused_construct).collect();
        match (reasons.is_empty(), analysis.conversions.as_slice()) {
            (true, [converted]) => Conversion::Converted {
                sql: converted.sql.clone(),
                setup: functions.setup,
                provenance: merge_provenance(
                    std::iter::once(&converted.provenance).chain(&functions.provenance),
                ),
            },
            (true, _) => Conversion::Refused {
                reasons: vec!["a case must be exactly one convertible statement".to_owned()],
                constructs: vec![STATEMENT_REFUSAL.to_owned()],
            },
            (false, _) => Conversion::Refused {
                reasons: reasons.into_iter().collect(),
                constructs: constructs.into_iter().collect(),
            },
        }
    }
}
