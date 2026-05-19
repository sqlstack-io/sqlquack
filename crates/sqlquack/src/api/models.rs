use crate::diagnostics::models as diagnostics_models;
use crate::dialect::models as dialect_models;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlUnit {
    pub sql: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeOptions {
    pub dialect: dialect_models::Dialect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisResult {
    pub diagnostics: Vec<diagnostics_models::Diagnostic>,
}
