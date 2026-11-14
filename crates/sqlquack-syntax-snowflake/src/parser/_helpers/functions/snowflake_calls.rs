//! Snowflake call arguments the shared argument list does not read.

use crate::parser::_helpers::statements::analyze::snowflake_reserved_word;
use crate::parser::constants::VARIADIC_KEYWORD;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Column;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::MethodCall;
use sqlquack_syntax::expressions::queries::models::Subquery;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::SourceSpan;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Refuses a string after a Snowflake path colon, as in `v1 :'v1'`.
    pub(crate) fn refuse_snowflake_string_path(&self) -> Result<()> {
        match self.applies_snowflake_context_rules() && self.check(TokenType::String) {
            true => Err(self.parse_error("Expected a path element, not a string, after :")),
            false => Ok(()),
        }
    }

    /// Refuses a name as a Snowflake interval's value, as in `INTERVAL m1 YEAR`; the value is a string.
    pub(crate) fn refuse_snowflake_interval_name(&self, value: Option<&Expression>) -> Result<()> {
        let name = matches!(
            value,
            Some(Expression::Column(_) | Expression::Identifier(_))
        );
        match self.applies_snowflake_context_rules() && name {
            true => Err(self.parse_error("Expected a string as the interval's value")),
            false => Ok(()),
        }
    }

    /// An array spread into a call's arguments: PostgreSQL's `VARIADIC <array>` or Snowflake's `** <array>`.
    pub(crate) fn parse_variadic_argument(&mut self) -> Result<Option<Expression>> {
        let postgres_variadic = self.config.dialect
            == Some(sqlquack_syntax::tokens::models::DialectType::PostgreSQL)
            && self.check(TokenType::Var)
            && self.peek_text().eq_ignore_ascii_case(VARIADIC_KEYWORD);
        let argument = matches!(
            self.previous_token_type(),
            Some(TokenType::LParen | TokenType::Comma)
        );
        let snowflake_spread = self.is_snowflake() && argument && self.check(TokenType::DStar);
        if !postgres_variadic && !snowflake_spread {
            return Ok(None);
        }
        self.skip();
        let this = self.parse_bitwise_or()?;
        Ok(Some(Expression::Variadic(Box::new(
            sqlquack_syntax::expressions::statements::models::Variadic {
                this: Box::new(this),
            },
        ))))
    }

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
                sqlquack_syntax::expressions::types::models::Tuple {
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
        let reserved = self.check(TokenType::Var)
            && self
                .peek_text()
                .strip_prefix('@')
                .and_then(|rest| rest.split(['.', '/']).next())
                .is_some_and(snowflake_reserved_word);
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

    /// Snowflake's `<database>..<function>( ... )`: a call on the database with the schema left out.
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
        let database = Expression::Column(Box::new(Column {
            name: database,
            table: None,
            join_mark: false,
            trailing_comments: Vec::new(),
            span: None,
            inferred_type: None,
        }));
        let mut call = MethodCall::new(database, method, args);
        call.schema_omitted = true;
        Ok(Some(Expression::MethodCall(Box::new(call))))
    }

    /// Reads the OVER clause of a schema-qualified Snowflake call, `<schema>.<function>( ... ) OVER ( ... )`.
    pub(crate) fn parse_qualified_window(&mut self, call: Expression) -> Result<Expression> {
        if self.applies_snowflake_context_rules() && self.check(TokenType::Over) {
            return self.maybe_parse_over(call);
        }
        Ok(call)
    }
}
