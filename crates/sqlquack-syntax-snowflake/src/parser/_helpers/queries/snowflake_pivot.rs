//! Snowflake's PIVOT, UNPIVOT and parenthesized table-function rules.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Refuses a Snowflake PIVOT aggregate that is no call, or that aggregates a string.
    pub(crate) fn refuse_snowflake_pivot_aggregate(&self, aggregate: &Expression) -> Result<()> {
        use sqlquack_syntax::traversal::types::ExpressionWalk;
        if !self.is_snowflake() {
            return Ok(());
        }
        let no_call = matches!(
            aggregate,
            Expression::Paren(_)
                | Expression::Column(_)
                | Expression::Literal(_)
                | Expression::Tuple(_)
                | Expression::Identifier(_)
        ) || matches!(aggregate, Expression::Function(call) if call.args.is_empty());
        let of_string = aggregate.children().iter().any(
            |argument| matches!(argument, Expression::Literal(literal) if literal.is_string()),
        );
        match no_call || of_string {
            true => Err(self.parse_error("Expected an aggregate of a column before FOR")),
            false => Ok(()),
        }
    }

    /// Refuses Snowflake's `IN ( ANY ORDER ... )` without BY.
    pub(crate) fn expect_snowflake_order_by(&self) -> Result<()> {
        match self.is_snowflake() && self.check(TokenType::Order) && !self.check_next(TokenType::By)
        {
            true => Err(self.parse_error("Expected BY after ORDER")),
            false => Ok(()),
        }
    }

    /// Refuses an empty Snowflake PIVOT IN list.
    pub(crate) fn refuse_snowflake_empty_pivot_list(&self, values: &[Expression]) -> Result<()> {
        match self.is_snowflake() && values.is_empty() {
            true => Err(self.parse_error("Expected a value in the PIVOT IN list")),
            false => Ok(()),
        }
    }

    /// A Snowflake PIVOT or UNPIVOT IN alias written without AS, or none.
    pub(crate) fn parse_snowflake_bare_pivot_alias(&mut self) -> Result<Option<Expression>> {
        let bare = self.is_snowflake()
            && (self.check(TokenType::Var)
                || self.check(TokenType::Identifier)
                || self.check(TokenType::QuotedIdentifier));
        match bare {
            true => Ok(self.parse_bitwise()?),
            false => Ok(None),
        }
    }

    /// Refuses a Snowflake `name ( ... )` in FROM that Snowflake takes only inside TABLE( ).
    pub(crate) fn refuse_snowflake_bare_table_call(&self, name: &str) -> Result<()> {
        use crate::parser::constants::{SNOWFLAKE_FROM_CALLS, SNOWFLAKE_FROM_CALLS_OF_A_NAME};
        if !self.applies_snowflake_context_rules() {
            return Ok(());
        }
        let upper: String = name.to_ascii_uppercase();
        if !SNOWFLAKE_FROM_CALLS.contains(&upper.as_str()) {
            return Err(self.parse_error("Expected TABLE( ) around a table function"));
        }
        let no_name = self.check(TokenType::RParen) || self.check(TokenType::String);
        match SNOWFLAKE_FROM_CALLS_OF_A_NAME.contains(&upper.as_str()) && no_name {
            true => Err(self.parse_error(format!("Expected an object name in {upper}( )"))),
            false => Ok(()),
        }
    }

    /// Refuses Snowflake's `FROM ( f ( ... ) )`: a table function needs TABLE( ), not parentheses.
    pub(crate) fn refuse_snowflake_parenthesized_call(&self, source: &Expression) -> Result<()> {
        let call = matches!(source, Expression::Paren(paren) if matches!(paren.this, Expression::Function(_)));
        match self.applies_snowflake_context_rules() && call {
            true => Err(self.parse_error("Expected TABLE( ) around a table function")),
            false => Ok(()),
        }
    }
}
