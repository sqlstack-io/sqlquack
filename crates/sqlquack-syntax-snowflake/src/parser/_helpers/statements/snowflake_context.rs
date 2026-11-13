//! Snowflake's statement-context rules.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Whether Snowflake's statement-context rules apply: in SQLQuack's own faithful modes only.
    pub(crate) fn applies_snowflake_context_rules(&self) -> bool {
        self.config.dialect == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
    }

    /// Refuses a SET target that Snowflake does not read as a variable name: a string, a reserved keyword or punctuation.
    pub(crate) fn reject_snowflake_set_name(&self) -> Result<()> {
        use sqlquack_syntax::tokens::models::TokenType;
        let refused = self.applies_snowflake_context_rules()
            && !self.check(TokenType::LParen)
            && (self.check(TokenType::String)
                || self.is_snowflake_reserved_keyword()
                || !(self.is_identifier_token() || self.check_keyword()));
        match refused {
            true => Err(self.parse_error("Expected a variable name after SET")),
            false => Ok(()),
        }
    }

    /// Refuses a backtick-quoted identifier, which Snowflake does not tokenize, in its faithful modes.
    pub(crate) fn reject_snowflake_backticks(&mut self) -> Result<()> {
        if !self.applies_snowflake_context_rules() {
            return Ok(());
        }
        let backtick = self.tokens.iter().position(|token| {
            token.token_type == sqlquack_syntax::tokens::models::TokenType::QuotedIdentifier
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
        use sqlquack_syntax::traversal::types::ExpressionWalk;
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
