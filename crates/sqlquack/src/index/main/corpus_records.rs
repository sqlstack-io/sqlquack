//! `corpus_records`: one conversion record per corpus file.

use crate::census::models::CorpusFile;
use crate::index::models::{ConversionRecord, RecordSource};

/// The corpus files as conversion records, named by path with `/` separators.
pub fn corpus_records(files: Vec<CorpusFile>) -> Vec<ConversionRecord> {
    let mut records: Vec<ConversionRecord> = Vec::with_capacity(files.len());
    for file in files {
        let parts: Vec<String> = file
            .path
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        records.push(ConversionRecord {
            source: RecordSource::File(parts.join("/")),
            provenance: file.provenance,
            refused_constructs: file.refused_constructs,
        });
    }
    records
}
