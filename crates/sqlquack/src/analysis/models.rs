//! Pipeline results.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::Serialize;
use sqlquack_blocks::layout::main::checked_layout::checked_layout;
use sqlquack_convert::snowflake::analysis::main::analyze::analyze;
use sqlquack_convert::snowflake::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_convert::snowflake::conversion::models::{ConversionOptions, Fixture};
use sqlquack_diagnostics::classification::constants::PROJECT_FUNCTION_CONSTRUCT;
use sqlquack_diagnostics::provenance::main::merge_provenance::merge_provenance;
use sqlquack_diagnostics::provenance::models::Provenance;
use sqlquack_diagnostics::snowflake::classification::main::cast_type_name::cast_type_name;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_evidence::constants::{INPUT_RELATION, ORDINAL_COLUMN};
use sqlquack_evidence::replay::models::{PythonFunction, PythonReplay};
use sqlquack_evidence::verify::models::Conversion;
use sqlquack_evidence::verify::types::CaseConverter;
use sqlquack_policy::policy::constants::QUOTED_IDENTIFIERS_IGNORE_CASE;
use sqlquack_policy::policy::models::SessionValue;
use sqlquack_semantics::snowflake::types::models::Schema;
use sqlquack_syntax::{DataType, Dialect};

use crate::analysis::_helpers::case_functions::case_functions;
use crate::analysis::_helpers::override_definition::load_override;
use crate::analysis::constants::{
    CASE_SETUP_UNREPLAYED, PYTHON_RUNTIME_MISSING, STATEMENT_REFUSAL,
};
use crate::analysis::main::override_macros::override_macros;
use crate::analysis::main::refused_construct::refused_construct;
use crate::analysis::main::with_helpers::with_helpers;

/// The production converter: the SQLQuack pipeline with the built-in rule catalogue.
#[derive(Clone, Debug, Default)]
pub struct PipelineConverter {
    /// The known divergences conversion may rely on.
    pub options: ConversionOptions,
    /// A Python interpreter that replays cases calling Python functions; without one they refuse.
    pub python: Option<PathBuf>,
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
    /// The Python functions, which a Python process registers.
    pub python: Vec<PythonFunction>,
}

impl CaseConverter for PipelineConverter {
    fn convert(&self, case: &LoadedCase) -> Conversion {
        if !case.setup.is_empty() {
            return Conversion::Refused {
                reasons: vec![CASE_SETUP_UNREPLAYED.to_owned()],
                constructs: vec![STATEMENT_REFUSAL.to_owned()],
            };
        }
        let mut options = ConversionOptions {
            fixture: Some(Fixture {
                relation: INPUT_RELATION.to_owned(),
                ordering_column: ORDINAL_COLUMN.to_owned(),
            }),
            quoted_identifiers_ignore_case: case
                .session_parameters
                .get(QUOTED_IDENTIFIERS_IGNORE_CASE)
                == Some(&SessionValue::Boolean(true)),
            ..self.options.clone()
        };
        for definition in &case.overrides {
            match load_override(&definition.name, &definition.definition) {
                Ok((replaces, replacement)) => {
                    options.overrides.insert(replaces, replacement);
                }
                Err(reason) => {
                    return Conversion::Refused {
                        reasons: vec![reason],
                        constructs: vec![STATEMENT_REFUSAL.to_owned()],
                    };
                }
            }
        }
        let functions = match case_functions(&case.functions, &options) {
            Ok(functions) => functions,
            Err(reasons) => {
                return Conversion::Refused {
                    reasons,
                    constructs: vec![STATEMENT_REFUSAL.to_owned()],
                };
            }
        };
        let python = match (&self.python, functions.python.is_empty()) {
            (_, true) => None,
            (Some(interpreter), false) => Some(Box::new(PythonReplay {
                interpreter: interpreter.clone(),
                functions: functions.python.clone(),
            })),
            (None, false) => {
                return Conversion::Refused {
                    reasons: vec![PYTHON_RUNTIME_MISSING.to_owned()],
                    constructs: vec![format!("fn:{PROJECT_FUNCTION_CONSTRUCT}")],
                };
            }
        };
        let analysis = analyze(
            &case.source_sql,
            &builtin_catalogue(),
            (&functions.schema, &options),
        );
        let reasons: BTreeSet<String> = analysis
            .refusals
            .iter()
            .map(|refusal| match refusal.location {
                Some(location) => format!(
                    "{} at {}:{}: {}",
                    refusal.category, location.line, location.column, refusal.reason
                ),
                None => format!("{}: {}", refusal.category, refusal.reason),
            })
            .collect();
        let constructs: BTreeSet<String> =
            analysis.refusals.iter().map(refused_construct).collect();
        match (reasons.is_empty(), analysis.conversions.as_slice()) {
            (true, [converted]) => Conversion::Converted {
                sql: converted.sql.clone(),
                raises: converted.raises.clone(),
                setup: with_helpers(
                    functions
                        .setup
                        .into_iter()
                        .chain(override_macros(&converted.provenance.divergences, &options))
                        .collect(),
                    &converted.sql,
                ),
                python,
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

    fn laid_out(&self, sql: &str) -> Result<String, String> {
        checked_layout(sql)
    }
}

/// A Python function as SQLQuack's runtime registers it in DuckDB.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PythonSignature {
    /// The name the converted SQL calls.
    pub name: String,
    /// How values cross: `scalar-v1` or `variant-json-v1`.
    pub abi: String,
    /// The DuckDB parameter types, in order.
    pub arguments: Vec<String>,
    /// The DuckDB type of the result.
    pub returns: String,
    /// `volatile`, never folded or shared, or `immutable`.
    pub volatility: String,
    /// `called` with None for a NULL argument, or `returns_null` without a call.
    pub null_input: String,
}

/// A dialect SQLQuack converts from. Every entry point takes it as a parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDialect {
    /// Snowflake SQL.
    Snowflake,
}

impl SourceDialect {
    /// The parser and printer dialect for this source.
    pub fn syntax(self) -> Dialect {
        match self {
            Self::Snowflake => Dialect::Snowflake,
        }
    }

    /// The lower-case name used on the command line and in reports.
    pub fn name(self) -> &'static str {
        match self {
            Self::Snowflake => "snowflake",
        }
    }

    /// The source dialect with this command-line name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "snowflake" => Some(Self::Snowflake),
            _ => None,
        }
    }

    /// How classification spells a cast's target type in this dialect.
    pub fn type_name(self) -> fn(&DataType) -> String {
        match self {
            Self::Snowflake => cast_type_name,
        }
    }
}
