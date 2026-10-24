pub(crate) struct CommandTestCase {
    pub(crate) description: &'static str,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) mention: &'static str,
    pub(crate) expected_outcome: CommandOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CommandOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) pending_files: usize,
    pub(crate) stderr_mentions: bool,
}
