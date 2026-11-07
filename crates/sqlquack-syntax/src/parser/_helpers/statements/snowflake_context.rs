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

    /// Refuses a COPY INTO <location> order Snowflake rejects as a syntax error, in its faithful modes.
    pub(crate) fn reject_snowflake_copy_order(
        &mut self,
        missing_from: bool,
        after_location_parameter: bool,
        into_location: bool,
    ) -> Result<()> {
        use crate::tokens::models::TokenType;
        if !self.applies_snowflake_context_rules() {
            return Ok(());
        }
        if missing_from {
            return Err(self
                .parse_error("Snowflake takes FROM right after the location COPY INTO writes to"));
        }
        if !into_location && self.check(TokenType::Partition) && self.check_next(TokenType::By) {
            return Err(self.parse_error(
                "COPY INTO a table has no PARTITION BY; only COPY INTO a location partitions its files",
            ));
        }
        if after_location_parameter
            && self.check(TokenType::Partition)
            && self.check_next(TokenType::By)
        {
            self.skip();
            return Err(self.parse_error(
                "Snowflake takes PARTITION BY before a location's STORAGE_INTEGRATION, CREDENTIALS or ENCRYPTION",
            ));
        }
        Ok(())
    }

    /// Refuses a backtick-quoted identifier, which Snowflake does not tokenize, in its faithful modes.
    pub(crate) fn reject_snowflake_backticks(&mut self) -> Result<()> {
        if !self.applies_snowflake_context_rules() {
            return Ok(());
        }
        let backtick = self.tokens.iter().position(|token| {
            token.token_type == crate::tokens::models::TokenType::QuotedIdentifier
                && self.source_text_range(token.span.start, token.span.start + 1)
                    == crate::parser::constants::BACKTICK
        });
        match backtick {
            Some(index) => {
                self.current = index;
                Err(self.parse_error("Snowflake does not quote identifiers with backticks"))
            }
            None => Ok(()),
        }
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
