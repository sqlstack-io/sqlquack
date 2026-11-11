//! `current_verdicts`: SQLQuack's verdict on every statement of the corpora, and broken invariants.

use std::collections::BTreeMap;
use std::path::Path;

use crate::verdicts::_helpers::corpus::{corpus_files, file_verdicts, read_corpus};
use crate::verdicts::constants::ABOUT;
use crate::verdicts::models::{CorpusParse, Verdicts};

/// The verdicts of every `.json` corpus file in `corpora`, and the statements that break an invariant.
pub fn current_verdicts(corpora: &Path) -> Result<CorpusParse, String> {
    let mut parse: CorpusParse = CorpusParse {
        verdicts: Verdicts {
            about: ABOUT.to_owned(),
            corpora: BTreeMap::new(),
        },
        broken: Vec::new(),
    };
    for path in corpus_files(corpora)? {
        let name: String = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (verdicts, broken) = file_verdicts(&read_corpus(&path)?, &name)?;
        parse.verdicts.corpora.insert(name, verdicts);
        parse.broken.extend(broken);
    }
    Ok(parse)
}
