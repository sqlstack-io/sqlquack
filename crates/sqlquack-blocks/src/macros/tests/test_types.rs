pub(crate) struct HelperListTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_heads: Vec<&'static str>,
}

pub(crate) struct CreationTestCase {
    pub(crate) description: &'static str,
    pub(crate) runs: usize,
    pub(crate) expected_failures: Vec<String>,
}

pub(crate) struct ExplainTestCase {
    pub(crate) description: &'static str,
    pub(crate) key: &'static str,
    pub(crate) expected_first_line: Option<&'static str>,
}

pub(crate) struct UnsitedRaiseTestCase {
    pub(crate) description: &'static str,
    pub(crate) marker: &'static str,
    pub(crate) expected_unsited: Vec<String>,
}

pub(crate) struct PinnedDefinitionTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_md5: Option<&'static str>,
}

/// A VARIANT's JSON text and what both versions of each writing helper give for it.
pub(crate) struct VariantTextTestCase {
    pub(crate) description: &'static str,
    pub(crate) value: Option<&'static str>,
    /// variant_text's result, or a fragment of the error it raises.
    pub(crate) expected_text: Result<Option<&'static str>, &'static str>,
    /// variant_json_text's result, or a fragment of the error it raises.
    pub(crate) expected_json_text: Result<Option<&'static str>, &'static str>,
}

/// A writing helper and how many times its definition calls json_escaped_v1.
pub(crate) struct EscapeCallsTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_calls: usize,
}

/// Text a TRY_PARSE_JSON read checks, and what every version of the read check gives for it.
pub(crate) struct JsonReadTestCase {
    pub(crate) description: &'static str,
    pub(crate) text: &'static str,
    /// The JSON read, NULL, or the tagged message raised.
    pub(crate) expected_outcome: Result<Option<&'static str>, &'static str>,
}
