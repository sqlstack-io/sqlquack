/// Recorded and current verdicts of one corpus file, as (dialect, statements, refused positions).
pub(crate) struct ChangedTestCase {
    pub(crate) description: &'static str,
    pub(crate) recorded: Vec<(&'static str, usize, Vec<usize>)>,
    pub(crate) current: Vec<(&'static str, usize, Vec<usize>)>,
    pub(crate) expected_lines: Vec<&'static str>,
}

/// A corpus file's entries, as (dialect, SQL), and the verdicts expected for it.
pub(crate) struct CurrentTestCase {
    pub(crate) description: &'static str,
    pub(crate) entries: Vec<(&'static str, &'static str)>,
    pub(crate) expected_verdicts: Vec<(&'static str, usize, Vec<usize>)>,
}
