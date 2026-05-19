#![forbid(unsafe_code)]

mod analysis;
mod api;
mod ast;
mod capabilities;
pub mod diagnostics;
mod dialect;
mod lexer;
mod lineage;
mod parser;
mod span;
mod translation;
mod types;

pub use api::main::analyze::analyze;
pub use api::models::{AnalysisResult, AnalyzeOptions, SqlUnit};
pub use diagnostics::models::{Diagnostic, DiagnosticSeverity};
pub use dialect::models::Dialect;
