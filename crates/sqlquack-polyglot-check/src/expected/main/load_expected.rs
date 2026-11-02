//! `load_expected`: the reviewed expected-differences file.

use std::path::Path;

use crate::expected::models::{ExpectedDifferences, ExpectedFile};

/// The expected differences in `path`; each spelling may appear once.
pub fn load_expected(path: &Path) -> Result<ExpectedDifferences, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let file: ExpectedFile = toml::from_str(&text)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    let mut expected = ExpectedDifferences {
        fixes: file.fixes,
        rejections: file.rejections,
        kept_quotings: file.kept_quotings,
        ..ExpectedDifferences::default()
    };
    for spelling in file.spellings {
        let nodes: std::collections::BTreeSet<String> = spelling.polyglot.into_iter().collect();
        if expected
            .allowed
            .insert(spelling.name.clone(), nodes)
            .is_some()
        {
            return Err(format!(
                "{}: {} is listed twice",
                path.display(),
                spelling.name
            ));
        }
    }
    Ok(expected)
}
