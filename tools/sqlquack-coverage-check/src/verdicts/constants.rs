//! Where the corpora and their recorded verdicts are, and how many changes a failure shows.

/// The vendored corpora, relative to the repository.
pub const CORPORA_DIR: &str = "corpora/polyglot-check";
/// The extension of a corpus file.
pub const CORPUS_EXTENSION: &str = "json";
/// The recorded verdicts, relative to the repository.
pub const VERDICTS_FILE: &str = "coverage/syntax/corpus-verdicts.json";
/// What the recorded file says about itself.
pub const ABOUT: &str = "SQLQuack's parse verdict on every statement of the corpora under corpora/polyglot-check: per file and dialect, how many statements it holds and the positions of those SQLQuack refuses. Any change, and any statement whose lossless tokens do not rebuild it or whose located parse differs from its faithful parse, fails `make corpus-verdicts`; record a reviewed change with `make corpus-verdicts-baseline`.";
/// The field located parses add to a node, ignored when comparing with the faithful parse.
pub const SPAN_FIELD: &str = "span";
/// Changed verdicts printed before the rest are counted.
pub const SHOWN_CHANGES: usize = 20;
