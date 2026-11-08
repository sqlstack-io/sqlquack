//! Constants shared by the engine modules, and the placeholders hosts write for template code.

pub(crate) const SQL_WILDCARD: &str = "*";
pub(crate) const OPEN_PAREN: &str = "(";
pub(crate) const CLOSE_PAREN: &str = ")";
pub(crate) const EMPTY_FIXTURE_QUERY_TOKEN_COUNT: usize = 6;
pub(crate) const ENCLOSING_CTE_TOKEN_COUNT: usize = 3;
/// The indent of each line of a configuration snippet printed in remediation text.
pub(crate) const SETTING_SNIPPET_INDENT: &str = "            ";
/// The configuration file remediation text names when the request does not name one.
pub const DEFAULT_CONFIG_FILE: &str = "sqlclean.toml";
/// The identifier prefix a host writes in place of each template interpolation.
pub const INTERPOLATION_PLACEHOLDER_PREFIX: &str = "__sqlclean_interpolation_";
/// The identifier prefix a host writes in place of a SQL macro parameter.
pub const MACRO_PARAMETER_PLACEHOLDER_PREFIX: &str = "__sqlclean_macro_parameter_";
/// The prefix of a relation parameter placeholder, written `<prefix><index>__`.
pub const RELATION_PARAMETER_PLACEHOLDER_PREFIX: &str = "__sqlclean_relation_parameter_";
/// The identifier prefix a host writes in place of a value parameter.
pub const VALUE_PARAMETER_PLACEHOLDER_PREFIX: &str = "__sqlclean_value_parameter_";
/// Placeholders standing for names the host generates, which Rules never judge as authored names.
pub(crate) const GENERATED_PLACEHOLDER_PREFIXES: [&str; 3] = [
    INTERPOLATION_PLACEHOLDER_PREFIX,
    RELATION_PARAMETER_PLACEHOLDER_PREFIX,
    VALUE_PARAMETER_PLACEHOLDER_PREFIX,
];
