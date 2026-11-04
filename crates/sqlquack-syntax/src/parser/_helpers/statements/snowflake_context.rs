//! Snowflake's statement-context rules, which the exact check's polyglot-matching modes skip.

use crate::expressions::tree::models::Expression;
use crate::parser::models::Parser;
use crate::tokens::types::Result;

impl Parser {
    /// Whether Snowflake's statement-context rules apply: in SQLQuack's own faithful modes only.
    pub(crate) fn applies_snowflake_context_rules(&self) -> bool {
        self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake)
            && self.config.faithful_calls
            && !self.config.record_call_spans
    }

    /// Refuses TABLE( ) or IDENTIFIER( ) around nothing or a bare name, except a named argument's TABLE(name).
    pub(crate) fn reject_snowflake_name_wrappers(&self, statements: &[Expression]) -> Result<()> {
        use crate::traversal::types::ExpressionWalk;
        let named_values: Vec<&Expression> = statements
            .iter()
            .flat_map(|statement| statement.dfs())
            .filter_map(|node| match node {
                Expression::NamedArgument(argument) => Some(&argument.value),
                _ => None,
            })
            .collect();
        let wrapper = |node: &Expression| match node {
            Expression::Function(function)
                if function.name.eq_ignore_ascii_case("TABLE")
                    || function.name.eq_ignore_ascii_case("IDENTIFIER") =>
            {
                match function.args.as_slice() {
                    [] => true,
                    [Expression::Column(_)] => {
                        !(function.name.eq_ignore_ascii_case("TABLE")
                            && named_values.iter().any(|value| std::ptr::eq(*value, node)))
                    }
                    _ => false,
                }
            }
            _ => false,
        };
        match statements
            .iter()
            .find_map(|statement| statement.find(wrapper))
        {
            Some(Expression::Function(function)) => Err(self.parse_error(format!(
                "{} needs a string, a variable or a call",
                function.name.to_ascii_uppercase()
            ))),
            _ => Ok(()),
        }
    }
}
