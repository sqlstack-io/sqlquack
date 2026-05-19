pub(crate) struct RepoFile {
    pub(crate) path: &'static str,
    pub(crate) contents: &'static str,
}

pub(crate) struct CheckRepoTestCase {
    pub(crate) description: &'static str,
    pub(crate) repo_files: &'static [RepoFile],
    pub(crate) expected_violation_codes: &'static [&'static str],
}
