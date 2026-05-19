use crate::api::models;

pub fn analyze(unit: models::SqlUnit, _options: models::AnalyzeOptions) -> models::AnalysisResult {
    let _ = unit;

    models::AnalysisResult {
        diagnostics: Vec::new(),
    }
}
