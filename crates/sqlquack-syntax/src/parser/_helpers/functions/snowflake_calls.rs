//! Snowflake call arguments the shared argument list does not read.

use crate::expressions::names::models::Column;
use crate::expressions::names::models::Identifier;
use crate::expressions::queries::models::Subquery;
use crate::expressions::tree::models::Expression;
use crate::parser::models::Parser;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// The operand of `ANY`/`ALL`, which Snowflake also reads as a list of values, as in `1 = ALL ( 1 )`.
    pub(crate) fn parse_quantified_operand(&mut self) -> Result<Expression> {
        let query = self.check(TokenType::Select)
            || self.check(TokenType::With)
            || self.check(TokenType::LParen);
        if !self.is_snowflake() || query {
            return self.parse_statement();
        }
        let mut values = self.parse_expression_list()?;
        match values.len() {
            1 => Ok(values.remove(0)),
            _ => Ok(Expression::Tuple(Box::new(
                crate::expressions::types::models::Tuple {
                    expressions: values,
                },
            ))),
        }
    }

    /// Consumes a pattern's `ESCAPE`, which Snowflake reads as an alias when nothing follows it.
    pub(crate) fn match_like_escape(&mut self) -> bool {
        let alias = self.is_snowflake()
            && matches!(
                self.peek_nth(1).map(|token| token.token_type),
                None | Some(
                    TokenType::Eof
                        | TokenType::Semicolon
                        | TokenType::Comma
                        | TokenType::RParen
                        | TokenType::From
                        | TokenType::Where
                        | TokenType::Group
                        | TokenType::GroupBy
                        | TokenType::Order
                        | TokenType::OrderBy
                        | TokenType::Having
                        | TokenType::Qualify
                        | TokenType::Limit
                        | TokenType::Union
                )
            );
        self.check(TokenType::Escape) && !alias && self.match_token(TokenType::Escape)
    }

    /// A qualified Snowflake call's lone query argument, as in `SNOWFLAKE.CORE.NULL_COUNT(SELECT c FROM t)`.
    pub(crate) fn parse_snowflake_query_argument(&mut self) -> Result<Option<Expression>> {
        let query_start = self.check(TokenType::Select) || self.check(TokenType::With);
        let column = matches!(
            self.peek_nth(1).map(|token| token.token_type),
            Some(TokenType::Comma | TokenType::RParen | TokenType::Dot)
        );
        if !self.is_snowflake() || !query_start || column {
            return Ok(None);
        }
        let query = self.parse_statement()?;
        Ok(Some(Expression::Subquery(Box::new(Subquery {
            this: query,
            alias: None,
            column_aliases: Vec::new(),
            alias_explicit_as: false,
            alias_keyword: None,
            order_by: None,
            limit: None,
            offset: None,
            distribute_by: None,
            sort_by: None,
            cluster_by: None,
            lateral: false,
            modifiers_inside: false,
            trailing_comments: Vec::new(),
            inferred_type: None,
            span: SourceSpan::default(),
        }))))
    }

    /// Refuses a Snowflake `<receiver>!<method>( )` without a name or `MODEL( <names> )` receiver, or a method.
    pub(crate) fn refuse_snowflake_bang_call(&self, receiver: &Expression) -> Result<()> {
        if !self.is_snowflake() {
            return Ok(());
        }
        let model_args = match receiver {
            Expression::Function(call) if call.name.eq_ignore_ascii_case("MODEL") => {
                Some(&call.args)
            }
            _ => None,
        };
        let bad_model = model_args.is_some_and(|args| {
            args.is_empty() || args.iter().any(|arg| !matches!(arg, Expression::Column(_)))
        });
        let bad_receiver = matches!(
            receiver,
            Expression::Literal(_) | Expression::Paren(_) | Expression::Tuple(_)
        );
        let no_method = self.peek_nth(1).map(|token| token.token_type) == Some(TokenType::LParen);
        match bad_model || bad_receiver || no_method {
            true => {
                Err(self.parse_error("Expected a name or MODEL( ) before ! and a method after it"))
            }
            false => Ok(()),
        }
    }

    /// Refuses a Snowflake stage reference `@'x'`, `@"x y"` with blanks in its quotes, or `@<reserved word>`.
    pub(crate) fn refuse_snowflake_stage_reference(&self) -> Result<()> {
        if !self.is_snowflake() {
            return Ok(());
        }
        let next = self.peek_nth(1);
        let split = self.check(TokenType::DAt)
            && next.is_some_and(|token| match token.token_type {
                TokenType::String => true,
                TokenType::QuotedIdentifier => token.text.contains(char::is_whitespace),
                _ => false,
            });
        let text = self.peek_text();
        let word = text
            .strip_prefix('@')
            .and_then(|rest| rest.split(['.', '/']).next())
            .unwrap_or_default()
            .to_ascii_uppercase();
        let reserved = self.check(TokenType::Var)
            && crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS.contains(&word.as_str());
        match split || reserved {
            true => Err(self.parse_error("Expected a stage name after @")),
            false => Ok(()),
        }
    }

    /// Refuses an unquoted Snowflake reserved word as a later part of a dotted name, as in `a . SELECT`.
    pub(crate) fn refuse_snowflake_reserved_part(&self) -> Result<()> {
        let word = self.peek_text().to_ascii_uppercase();
        let reserved = !self.check(TokenType::QuotedIdentifier)
            && crate::parser::constants::SNOWFLAKE_REFUSED_NAME_PARTS.contains(&word.as_str());
        match self.is_snowflake() && reserved {
            true => Err(self.parse_error(format!(
                "Reserved word {word} cannot be an unquoted name part"
            ))),
            false => Ok(()),
        }
    }

    /// Snowflake's `<database>..<function>( ... )`, the schema left out; it is kept as an empty part.
    pub(crate) fn parse_snowflake_default_schema_call(
        &mut self,
        database: Identifier,
    ) -> Result<Option<Expression>> {
        let call = self.check(TokenType::Dot)
            && self.peek_nth(2).map(|token| token.token_type) == Some(TokenType::LParen);
        if !self.is_snowflake() || !call {
            return Ok(None);
        }
        self.skip();
        let method = self.expect_identifier_or_keyword_with_quoted()?;
        self.expect_token(TokenType::LParen)?;
        let args = match self.check(TokenType::RParen) {
            true => Vec::new(),
            false => self.parse_method_arguments_for(&method.name)?,
        };
        self.expect_token(TokenType::RParen)?;
        let schema = Expression::Column(Box::new(Column {
            name: Identifier::new(String::new()),
            table: Some(database),
            join_mark: false,
            trailing_comments: Vec::new(),
            span: None,
            inferred_type: None,
        }));
        Ok(Some(
            self.parse_method_call_expression(schema, method, args),
        ))
    }
}
