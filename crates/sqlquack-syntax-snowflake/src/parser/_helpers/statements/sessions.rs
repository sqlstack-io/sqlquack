//! Parser statements: sessions.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::columns::models::AutoIncrementColumnConstraint;
use sqlquack_syntax::expressions::columns::models::CheckColumnConstraint;
use sqlquack_syntax::expressions::columns::models::ColumnConstraint;
use sqlquack_syntax::expressions::columns::models::ColumnDef;
use sqlquack_syntax::expressions::columns::models::CommentColumnConstraint;
use sqlquack_syntax::expressions::columns::models::Comprehension;
use sqlquack_syntax::expressions::columns::models::Constraint;
use sqlquack_syntax::expressions::columns::models::DefaultColumnConstraint;
use sqlquack_syntax::expressions::columns::models::GeneratedAsIdentityColumnConstraint;
use sqlquack_syntax::expressions::columns::models::NotNullColumnConstraint;
use sqlquack_syntax::expressions::columns::models::PathColumnConstraint;
use sqlquack_syntax::expressions::columns::models::PrimaryKeyColumnConstraint;
use sqlquack_syntax::expressions::columns::models::UniqueColumnConstraint;
use sqlquack_syntax::expressions::functions::models::ForIn;
use sqlquack_syntax::expressions::functions::models::Function;
use sqlquack_syntax::expressions::names::main::identifier_from_token::identifier_from_token;
use sqlquack_syntax::expressions::names::models::BooleanLiteral;
use sqlquack_syntax::expressions::names::models::Column;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::operators::models::Cast;
use sqlquack_syntax::expressions::operators::models::Changes;
use sqlquack_syntax::expressions::operators::models::DotAccess;
use sqlquack_syntax::expressions::operators::models::ForeignKey;
use sqlquack_syntax::expressions::operators::models::NamedArgSeparator;
use sqlquack_syntax::expressions::operators::models::NamedArgument;
use sqlquack_syntax::expressions::operators::models::TableArgument;
use sqlquack_syntax::expressions::statements::models::Attach;
use sqlquack_syntax::expressions::statements::models::AttachOption;
use sqlquack_syntax::expressions::statements::models::Command;
use sqlquack_syntax::expressions::statements::models::Declare;
use sqlquack_syntax::expressions::statements::models::DeclareItem;
use sqlquack_syntax::expressions::statements::models::Detach;
use sqlquack_syntax::expressions::statements::models::Install;
use sqlquack_syntax::expressions::statements::models::Summarize;
use sqlquack_syntax::expressions::tables::models::CollateProperty;
use sqlquack_syntax::expressions::tables::models::ConstraintType;
use sqlquack_syntax::expressions::tables::models::DistKeyProperty;
use sqlquack_syntax::expressions::tables::models::DistributedByProperty;
use sqlquack_syntax::expressions::tables::models::SemanticView;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::typed_functions::models::ModelAttribute;
use sqlquack_syntax::expressions::types::models::Array;
use sqlquack_syntax::expressions::types::models::DataType;
use sqlquack_syntax::expressions::types::models::Struct;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::guard::main::is_guard_error::is_guard_error;
use sqlquack_syntax::tokens::models::SourceSpan;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Parses DuckDB ATTACH/DETACH.
    pub(crate) fn parse_attach_detach(&mut self, is_attach: bool) -> Result<Expression> {
        let _ = self.match_identifier("DATABASE") || self.match_token(TokenType::Database);

        let exists = if is_attach {
            self.match_text_seq(&["IF", "NOT", "EXISTS"])
        } else {
            self.match_text_seq(&["IF", "EXISTS"])
        };

        let this_expr = self.parse_expression()?;

        let this = if self.match_token(TokenType::As) {
            let alias = self.expect_identifier_or_keyword_with_quoted()?;
            Expression::Alias(Box::new(Alias {
                this: this_expr,
                alias,
                column_aliases: Vec::new(),
                alias_explicit_as: false,
                alias_keyword: None,
                pre_alias_comments: Vec::new(),
                trailing_comments: Vec::new(),
                inferred_type: None,
                span: SourceSpan::default(),
            }))
        } else {
            this_expr
        };

        if is_attach {
            let expressions = if self.match_token(TokenType::LParen) {
                let mut opts: Vec<Expression> = Vec::new();
                loop {
                    let key_name = self.advance_text()?.to_ascii_uppercase();
                    let key = Expression::Identifier(Identifier::new(key_name));
                    let value = if !self.check(TokenType::Comma) && !self.check(TokenType::RParen) {
                        let val_token = self.advance()?;
                        let val_expr = if val_token.token_type == TokenType::String {
                            Expression::Literal(Box::new(Literal::String(
                                val_token.text.to_string(),
                            )))
                        } else if val_token.token_type == TokenType::True {
                            Expression::Boolean(BooleanLiteral { value: true })
                        } else if val_token.token_type == TokenType::False {
                            Expression::Boolean(BooleanLiteral { value: false })
                        } else {
                            Expression::Identifier(
                                Identifier::new(val_token.text.clone()).with_span(val_token.span),
                            )
                        };
                        Some(Box::new(val_expr))
                    } else {
                        None
                    };
                    opts.push(Expression::AttachOption(Box::new(AttachOption {
                        this: Box::new(key),
                        expression: value,
                    })));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
                opts
            } else {
                Vec::new()
            };

            Ok(Expression::Attach(Box::new(Attach {
                this: Box::new(this),
                exists,
                expressions,
            })))
        } else {
            Ok(Expression::Detach(Box::new(Detach {
                this: Box::new(this),
                exists,
            })))
        }
    }

    /// Parses DuckDB INSTALL.
    pub(crate) fn parse_install(&mut self, force: bool) -> Result<Expression> {
        let name = self.expect_identifier_or_keyword()?;
        let this = Expression::Identifier(Identifier::new(name));

        let from_ = if self.match_token(TokenType::From) {
            Some(Box::new(self.parse_primary()?))
        } else {
            None
        };

        Ok(Expression::Install(Box::new(Install {
            this: Box::new(this),
            from_,
            force: if force {
                Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: true,
                })))
            } else {
                None
            },
        })))
    }

    /// Parses DuckDB FORCE INSTALL/CHECKPOINT.
    pub(crate) fn parse_force_statement(&mut self) -> Result<Expression> {
        if self.match_identifier("INSTALL") {
            return self.parse_install(true);
        }
        self.parse_as_command()?
            .ok_or_else(|| self.parse_error("Failed to parse FORCE statement"))
    }

    /// Parses DuckDB SUMMARIZE.
    pub(crate) fn parse_summarize_statement(&mut self) -> Result<Expression> {
        let is_table = self.match_token(TokenType::Table);

        let this = if self.check(TokenType::Select) || self.check(TokenType::With) {
            self.parse_select()?
        } else if self.check(TokenType::String) {
            self.parse_primary()?
        } else {
            self.parse_table_parts()?
                .unwrap_or(Expression::Identifier(Identifier::new(String::new())))
        };

        Ok(Expression::Summarize(Box::new(Summarize {
            this: Box::new(this),
            table: if is_table {
                Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: true,
                })))
            } else {
                None
            },
        })))
    }

    /// Parses Presto/Trino DEALLOCATE PREPARE name.
    pub(crate) fn parse_deallocate_prepare(&mut self) -> Result<Expression> {
        self.skip();

        if self.match_identifier("PREPARE") {
            let name = if !self.is_at_end() && !self.check(TokenType::Semicolon) {
                self.advance_text()?
            } else {
                String::new()
            };

            let command_text = if name.is_empty() {
                "DEALLOCATE PREPARE".to_string()
            } else {
                format!("DEALLOCATE PREPARE {}", name)
            };

            Ok(Expression::Command(Box::new(Command {
                this: command_text,
            })))
        } else {
            let mut parts = vec!["DEALLOCATE".to_string()];
            while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                let token = self.advance()?;
                parts.push(token.text.to_string());
            }
            Ok(Expression::Command(Box::new(Command {
                this: parts.join(" "),
            })))
        }
    }

    /// Parses the remaining tokens as a raw command, the fallback when parsing fails.
    pub(crate) fn parse_as_command(&mut self) -> Result<Option<Expression>> {
        let start_text = if self.current > 0 {
            self.tokens
                .get(self.current - 1)
                .map(|t| t.text.to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let mut tokens_info: Vec<(String, TokenType)> = Vec::new();
        while !self.is_at_end() {
            let token = self.advance()?;
            tokens_info.push((token.text.to_string(), token.token_type));
        }

        let mut expression = String::new();
        for (i, (text, token_type)) in tokens_info.iter().enumerate() {
            if i > 0 {
                let prev_type = &tokens_info[i - 1].1;
                let needs_space = !Self::is_punctuation_token(prev_type)
                    && !Self::is_punctuation_token(token_type);
                if needs_space {
                    expression.push(' ');
                }
            }
            expression.push_str(text);
        }

        Ok(Some(Expression::Command(Box::new(Command {
            this: if expression.is_empty() {
                start_text
            } else {
                format!("{} {}", start_text, expression)
            },
        }))))
    }

    /// Helper to determine if a token type is punctuation that shouldn't have spaces around it
    pub(crate) fn is_punctuation_token(token_type: &TokenType) -> bool {
        matches!(
            token_type,
            TokenType::Dot | TokenType::Colon | TokenType::DColon
        )
    }

    /// Falls back to a Command from a saved position, keeping source text to the semicolon.
    pub(crate) fn fallback_to_command(&mut self, start_pos: usize) -> Result<Expression> {
        let start_span = self.tokens[start_pos].span.start;
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            self.skip();
        }
        let command_text = if self.source.is_some() {
            let end_span = if self.current > 0 {
                self.tokens[self.current - 1].span.end
            } else {
                start_span
            };
            self.source_text_range(start_span, end_span)
                .trim()
                .to_string()
        } else {
            let mut parts: Vec<String> = Vec::new();
            for i in start_pos..self.current {
                if self.tokens[i].token_type == TokenType::String {
                    parts.push(format!("'{}'", self.tokens[i].text.replace('\'', "''")));
                } else {
                    parts.push(self.tokens[i].text.to_string());
                }
            }
            parts.join(" ")
        };
        Ok(Expression::Command(Box::new(Command {
            this: command_text,
        })))
    }

    /// Parses an assignment `variable := value`.
    pub(crate) fn parse_assignment(&mut self) -> Result<Option<Expression>> {
        self.with_parser_depth(|parser| parser.parse_assignment_inner())
    }

    #[inline(never)]
    pub(crate) fn parse_assignment_inner(&mut self) -> Result<Option<Expression>> {
        let mut this = self.parse_disjunction()?;

        while self.match_token(TokenType::ColonEq) {
            if let Some(left) = this {
                let right = self.parse_assignment()?;
                if let Some(right_expr) = right {
                    this = Some(Expression::PropertyEQ(Box::new(BinaryOp {
                        left,
                        right: right_expr,
                        left_comments: Vec::new(),
                        operator_comments: Vec::new(),
                        trailing_comments: Vec::new(),
                        inferred_type: None,
                        span: SourceSpan::default(),
                    })));
                } else {
                    this = Some(left);
                    break;
                }
            } else {
                break;
            }
        }

        Ok(this)
    }

    /// Parses bitwise OR/XOR/AND, delegating to the precedence chain.
    pub(crate) fn parse_bitwise(&mut self) -> Result<Option<Expression>> {
        let start = self.current;
        match self.parse_bitwise_or() {
            Ok(expr) => Ok(Some(expr)),
            Err(_err) if self.current == start => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Parses TRUE or FALSE.
    pub(crate) fn parse_boolean(&mut self) -> Result<Option<Expression>> {
        if self.match_token(TokenType::True) {
            return Ok(Some(Expression::Boolean(BooleanLiteral { value: true })));
        }
        if self.match_token(TokenType::False) {
            return Ok(Some(Expression::Boolean(BooleanLiteral { value: false })));
        }
        Ok(None)
    }

    /// Parses brackets: subscripts, array literals and struct literals.
    pub(crate) fn parse_bracket(&mut self) -> Result<Option<Expression>> {
        self.parse_bracket_with_expr(None)
    }

    /// parse_bracket_with_expr - Parses bracket with optional left-side expression
    pub(crate) fn parse_bracket_with_expr(
        &mut self,
        this: Option<Expression>,
    ) -> Result<Option<Expression>> {
        let is_bracket = self.match_token(TokenType::LBracket);
        let is_brace = if !is_bracket {
            self.match_token(TokenType::LBrace)
        } else {
            false
        };

        if !is_bracket && !is_brace {
            return Ok(this);
        }

        let mut expressions: Vec<Expression> = Vec::new();

        if is_bracket && !self.check(TokenType::RBracket) {
            let first_expr = if self.check(TokenType::Colon) {
                if let Some(slice) = self.parse_slice()? {
                    slice
                } else {
                    self.parse_expression()?
                }
            } else if let Ok(Some(expr)) = self.parse_bracket_key_value() {
                expr
            } else {
                let expr = self.parse_expression()?;
                if self.check(TokenType::Colon) {
                    if let Some(slice) = self.parse_slice_with_this(Some(expr))? {
                        slice
                    } else {
                        return Err(self.parse_error("Failed to parse slice"));
                    }
                } else {
                    expr
                }
            };

            if self.match_token(TokenType::For) {
                let loop_var = self.parse_primary()?;

                let position = if self.match_token(TokenType::Comma) {
                    Some(self.parse_primary()?)
                } else {
                    None
                };

                if !self.match_token(TokenType::In) {
                    return Err(self.parse_error("Expected IN in comprehension"));
                }

                let iterator = self.parse_expression()?;

                let condition = if self.match_token(TokenType::If) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };

                self.expect_token(TokenType::RBracket)?;

                return Ok(Some(Expression::Comprehension(Box::new(Comprehension {
                    this: Box::new(first_expr),
                    expression: Box::new(loop_var),
                    position: position.map(Box::new),
                    iterator: Some(Box::new(iterator)),
                    condition: condition.map(Box::new),
                }))));
            }

            expressions.push(first_expr);

            while self.match_token(TokenType::Comma) {
                if let Ok(Some(expr)) = self.parse_bracket_key_value() {
                    expressions.push(expr);
                } else {
                    match self.parse_expression() {
                        Ok(expr) => expressions.push(expr),
                        Err(_) => break,
                    }
                }
            }
        } else if is_brace && !self.check(TokenType::RBrace) {
            loop {
                if let Ok(Some(expr)) = self.parse_bracket_key_value() {
                    expressions.push(expr);
                } else {
                    match self.parse_expression() {
                        Ok(expr) => expressions.push(expr),
                        Err(_) => break,
                    }
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        if is_bracket {
            self.expect_token(TokenType::RBracket)?;
        } else if is_brace {
            self.expect_token(TokenType::RBrace)?;
        }

        if is_brace {
            let fields: Vec<(Option<String>, Expression)> =
                expressions.into_iter().map(|e| (None, e)).collect();
            Ok(Some(Expression::Struct(Box::new(Struct { fields }))))
        } else if let Some(base_expr) = this {
            if expressions.len() == 1 {
                Ok(Some(self.make_subscript(base_expr, expressions.remove(0))))
            } else {
                let mut result = base_expr;
                for expr in expressions {
                    result = self.make_subscript(result, expr);
                }
                Ok(Some(result))
            }
        } else {
            Ok(Some(Expression::Array(Box::new(Array {
                expressions,
                inferred_type: None,
            }))))
        }
    }

    /// Parses `key: value` or `key => value` pairs in brackets.
    pub(crate) fn parse_bracket_key_value(&mut self) -> Result<Option<Expression>> {
        let saved_pos = self.current;

        if let Ok(key) = self.parse_primary() {
            if self.match_token(TokenType::Colon) || self.match_text_seq(&["=>"]) {
                match self.parse_expression() {
                    Ok(value) => {
                        let name = match &key {
                            Expression::Identifier(id) => id.clone(),
                            Expression::Literal(lit)
                                if matches!(
                                    lit.as_ref(),
                                    sqlquack_syntax::expressions::names::models::Literal::String(
                                        _s
                                    )
                                ) =>
                            {
                                let sqlquack_syntax::expressions::names::models::Literal::String(s) =
                                    lit.as_ref()
                                else {
                                    return Err(sqlquack_syntax::tokens::errors::Error::internal(
                                        "the parser reached a state its checks rule out",
                                    ));
                                };
                                Identifier::new(s.clone())
                            }
                            _ => Identifier::new("".to_string()),
                        };
                        return Ok(Some(Expression::NamedArgument(Box::new(NamedArgument {
                            name,
                            value,
                            separator: NamedArgSeparator::DArrow,
                        }))));
                    }
                    Err(_) => {
                        self.current = saved_pos;
                        return Ok(None);
                    }
                }
            }
            self.current = saved_pos;
        }

        Ok(None)
    }

    /// Parses CHANGES(INFORMATION => var) AT|BEFORE(...) END(...).
    pub(crate) fn parse_changes(&mut self) -> Result<Option<Expression>> {
        if self.is_snowflake() && self.match_text_seq(&["CHANGES", "(", ")"]) {
            return Ok(Some(Expression::Changes(Box::new(Changes {
                information: None,
                at_before: None,
                end: None,
            }))));
        }
        if !self.match_text_seq(&["CHANGES", "(", "INFORMATION", "=>"]) {
            return Ok(None);
        }

        let information = if !self.is_at_end() && !self.check(TokenType::RParen) {
            let tok = self.advance()?;
            Some(Box::new(Expression::Var(Box::new(
                sqlquack_syntax::expressions::statements::models::Var {
                    this: tok.text.to_string(),
                },
            ))))
        } else {
            None
        };

        let closed = self.match_token(TokenType::RParen);
        if self.is_snowflake() && (information.is_none() || !closed) {
            return Err(
                self.parse_error("Expected a value and ) in CHANGES ( INFORMATION => ... )")
            );
        }

        let at_before = self.parse_historical_data()?.map(Box::new);

        let end = self.parse_historical_data()?.map(Box::new);

        Ok(Some(Expression::Changes(Box::new(Changes {
            information,
            at_before,
            end,
        }))))
    }

    /// Parses a column reference and its operators.
    pub(crate) fn parse_column(&mut self) -> Result<Option<Expression>> {
        let column_ref = self.parse_column_reference()?;
        if column_ref.is_some() {
            return self.parse_column_ops_with_expr(column_ref);
        }
        self.parse_bracket()
    }

    /// Parses a column constraint: NOT NULL, PRIMARY KEY, UNIQUE, DEFAULT, CHECK, ...
    pub(crate) fn parse_column_constraint(&mut self) -> Result<Option<Expression>> {
        let constraint_name = if self.match_token(TokenType::Constraint) {
            self.parse_id_var()?.and_then(|e| {
                if let Expression::Identifier(id) = e {
                    Some(id)
                } else {
                    None
                }
            })
        } else {
            None
        };

        if self.match_text_seq(&["NOT", "NULL"]) {
            return Ok(Some(Expression::NotNullColumnConstraint(Box::new(
                NotNullColumnConstraint { allow_null: None },
            ))));
        }

        if self.match_text_seq(&["NOT", "FOR", "REPLICATION"]) {
            return Ok(Some(Expression::Property(Box::new(
                sqlquack_syntax::expressions::tables::models::Property {
                    this: Box::new(Expression::Identifier(Identifier::new(
                        "NOT FOR REPLICATION".to_string(),
                    ))),
                    value: None,
                },
            ))));
        }

        if self.match_text_seq(&["NULL"]) {
            return Ok(Some(Expression::NotNullColumnConstraint(Box::new(
                NotNullColumnConstraint {
                    allow_null: Some(Box::new(Expression::Boolean(BooleanLiteral {
                        value: true,
                    }))),
                },
            ))));
        }

        if self.match_text_seq(&["PRIMARY", "KEY"]) {
            return Ok(Some(Expression::PrimaryKeyColumnConstraint(Box::new(
                PrimaryKeyColumnConstraint {
                    desc: None,
                    options: Vec::new(),
                },
            ))));
        }

        if self.match_text_seq(&["UNIQUE"]) {
            let _ = self.match_texts(&["KEY", "INDEX"]);
            let nulls = if self.match_text_seq(&["NULLS", "NOT", "DISTINCT"]) {
                Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: true,
                })))
            } else {
                None
            };
            return Ok(Some(Expression::UniqueColumnConstraint(Box::new(
                UniqueColumnConstraint {
                    this: None,
                    index_type: None,
                    on_conflict: None,
                    nulls,
                    options: Vec::new(),
                },
            ))));
        }

        if self.match_text_seq(&["DEFAULT"]) {
            let default_value = self.parse_select_or_expression()?;
            if let Some(val) = default_value {
                return Ok(Some(Expression::DefaultColumnConstraint(Box::new(
                    DefaultColumnConstraint {
                        this: Box::new(val),
                        for_column: None,
                    },
                ))));
            }
            return Ok(None);
        }

        if self.match_text_seq(&["CHECK"]) {
            if self.match_token(TokenType::LParen) {
                let expr = self.parse_select_or_expression()?;
                self.match_token(TokenType::RParen);
                if let Some(check_expr) = expr {
                    return Ok(Some(Expression::CheckColumnConstraint(Box::new(
                        CheckColumnConstraint {
                            this: Box::new(check_expr),
                            enforced: None,
                        },
                    ))));
                }
            }
            return Ok(None);
        }

        if self.match_text_seq(&["REFERENCES"]) {
            return self.parse_references_column_constraint();
        }

        if self.match_texts(&["AUTO_INCREMENT", "AUTOINCREMENT", "IDENTITY"]) {
            return self.parse_identity_column_constraint();
        }

        if self.match_text_seq(&["COMMENT"]) {
            if let Some(_comment) = self.parse_string()? {
                return Ok(Some(Expression::CommentColumnConstraint(
                    CommentColumnConstraint,
                )));
            }
            return Ok(None);
        }

        if self.match_text_seq(&["COLLATE"]) {
            if let Some(collation) = self.parse_id_var()? {
                return Ok(Some(Expression::CollateProperty(Box::new(
                    CollateProperty {
                        this: Box::new(collation),
                        default: None,
                    },
                ))));
            }
            return Ok(None);
        }

        if self.match_text_seq(&["GENERATED"]) {
            return self.parse_generated_constraint_kind();
        }

        if self.match_text_seq(&["PATH"]) {
            if let Some(path_expr) = self.parse_string()? {
                return Ok(Some(Expression::PathColumnConstraint(Box::new(
                    PathColumnConstraint {
                        this: Box::new(path_expr),
                    },
                ))));
            }
            return Ok(None);
        }

        if let Some(name) = constraint_name {
            return Ok(Some(Expression::Identifier(name)));
        }

        Ok(None)
    }

    fn parse_references_column_constraint(&mut self) -> Result<Option<Expression>> {
        let table = self.parse_table_parts()?;
        let columns = if self.match_token(TokenType::LParen) {
            let mut cols: Vec<Expression> = Vec::new();
            loop {
                if let Some(col) = self.parse_id_var()? {
                    cols.push(col);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.match_token(TokenType::RParen);
            cols
        } else {
            Vec::new()
        };

        Ok(Some(Expression::ForeignKey(Box::new(ForeignKey {
            expressions: columns,
            reference: table.map(Box::new),
            delete: None,
            update: None,
            options: Vec::new(),
        }))))
    }

    fn parse_identity_column_constraint(&mut self) -> Result<Option<Expression>> {
        let mut start = None;
        let mut increment = None;

        if self.match_token(TokenType::LParen) {
            start = self.parse_bitwise()?;
            if self.match_token(TokenType::Comma) {
                increment = self.parse_bitwise()?;
            }
            self.expect_token(TokenType::RParen)?;
        } else if self.match_text_seq(&["START"]) {
            start = self.parse_bitwise()?;
            if self.match_text_seq(&["INCREMENT"]) {
                increment = self.parse_bitwise()?;
            }
        }

        if start.is_some() || increment.is_some() {
            return Ok(Some(Expression::GeneratedAsIdentityColumnConstraint(
                Box::new(GeneratedAsIdentityColumnConstraint {
                    this: Some(Box::new(Expression::Boolean(BooleanLiteral {
                        value: false,
                    }))),
                    expression: None,
                    on_null: None,
                    start: start.map(Box::new),
                    increment: increment.map(Box::new),
                    minvalue: None,
                    maxvalue: None,
                    cycle: None,
                    order: None,
                }),
            )));
        }
        Ok(Some(Expression::AutoIncrementColumnConstraint(
            AutoIncrementColumnConstraint,
        )))
    }

    fn parse_generated_constraint_kind(&mut self) -> Result<Option<Expression>> {
        let always = self.match_text_seq(&["ALWAYS"]);
        if !always {
            self.match_text_seq(&["BY", "DEFAULT"]);
        }
        let on_null = self.match_text_seq(&["ON", "NULL"]);
        if self.match_text_seq(&["AS", "IDENTITY"]) {
            return Ok(Some(Expression::GeneratedAsIdentityColumnConstraint(
                Box::new(GeneratedAsIdentityColumnConstraint {
                    this: None,
                    expression: None,
                    on_null: if on_null {
                        Some(Box::new(Expression::Boolean(BooleanLiteral {
                            value: true,
                        })))
                    } else {
                        None
                    },
                    start: None,
                    increment: None,
                    minvalue: None,
                    maxvalue: None,
                    cycle: None,
                    order: None,
                }),
            )));
        }
        Ok(None)
    }

    /// Parses column operators on `this`: dot access, brackets and casts.
    pub(crate) fn parse_column_ops_with_expr(
        &mut self,
        this: Option<Expression>,
    ) -> Result<Option<Expression>> {
        let mut result = if let Some(expr) = this {
            if self.match_token(TokenType::LBracket) {
                let index = self.parse_disjunction()?;
                self.match_token(TokenType::RBracket);
                if let Some(idx) = index {
                    Some(self.make_subscript(expr, idx))
                } else {
                    Some(expr)
                }
            } else {
                Some(expr)
            }
        } else {
            None
        };

        while self.match_token(TokenType::Dot) {
            if result.is_none() {
                break;
            }
            if self.match_token(TokenType::Star) {
                let mut table_name = match &result {
                    Some(Expression::Column(col)) if col.table.is_none() => Some(col.name.clone()),
                    Some(Expression::Dot(dot)) => {
                        fn dot_to_name(expr: &Expression) -> String {
                            match expr {
                                Expression::Column(col) => {
                                    if let Some(ref table) = col.table {
                                        format!("{}.{}", table.name, col.name.name)
                                    } else {
                                        col.name.name.clone()
                                    }
                                }
                                Expression::Dot(d) => {
                                    format!("{}.{}", dot_to_name(&d.this), d.field.name)
                                }
                                _ => String::new(),
                            }
                        }
                        Some(Identifier::new(dot_to_name(&Expression::Dot(dot.clone()))))
                    }
                    _ => None,
                };
                if let Some(table) = &mut table_name {
                    table.span = result.as_ref().and_then(Self::reference_span);
                }
                let star = self.parse_star_modifiers(table_name)?;
                result = Some(Expression::Star(Box::new(star)));
                break;
            }
            if self.is_identifier_or_keyword_token() || self.check(TokenType::QuotedIdentifier) {
                let token = self.advance()?;
                let field_ident = Identifier {
                    name: token.text,
                    quoted: token.token_type == TokenType::QuotedIdentifier,
                    trailing_comments: Vec::new(),
                    span: Some(token.span),
                    omitted: false,
                };
                result = Some(Expression::Dot(Box::new(DotAccess {
                    this: result.take().ok_or_else(|| {
                        sqlquack_syntax::tokens::errors::Error::internal(
                            "a value the parser checked is missing",
                        )
                    })?,
                    field: field_ident,
                    inferred_type: None,
                })));
            } else {
                break;
            }
        }

        if self.match_token(TokenType::Exclamation) {
            if let Some(expr) = result.take() {
                let attr = self.parse_unary()?;
                result = Some(Expression::ModelAttribute(Box::new(ModelAttribute {
                    this: Box::new(expr),
                    expression: Box::new(attr),
                })));
            }
        }

        if self.match_token(TokenType::DColon) {
            if let Some(type_expr) = self.parse_types()? {
                if let Some(expr) = result {
                    let data_type = match type_expr {
                        Expression::DataType(dt) => dt,
                        _ => {
                            result = Some(expr);
                            return Ok(result);
                        }
                    };
                    result = Some(Expression::Cast(Box::new(Cast {
                        this: expr,
                        to: data_type,
                        trailing_comments: Vec::new(),
                        double_colon_syntax: true,
                        typed_literal: false,
                        format: None,
                        default: None,
                        inferred_type: None,
                        span: SourceSpan::default(),
                    })));
                }
            }
        }

        Ok(result)
    }

    /// Parses a column reference from a field.
    pub(crate) fn parse_column_reference(&mut self) -> Result<Option<Expression>> {
        if let Some(field) = self.parse_field()? {
            match &field {
                Expression::Identifier(id) => {
                    return Ok(Some(Self::parsed_column(Column {
                        name: id.clone(),
                        table: None,
                        join_mark: false,
                        trailing_comments: Vec::new(),
                        span: None,
                        inferred_type: None,
                    })));
                }
                _ => return Ok(Some(field)),
            }
        }
        Ok(None)
    }

    /// Parses a generic command with no specific parser.
    pub(crate) fn parse_command(&mut self) -> Result<Option<Expression>> {
        let command_text = self.previous_text().to_ascii_uppercase();

        let mut tokens: Vec<(String, TokenType)> = vec![(command_text, TokenType::Var)];
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let token = self.advance()?;
            let text = if token.token_type == TokenType::QuotedIdentifier {
                let quote_char = { '"' };
                format!("{}{}{}", quote_char, token.text, quote_char)
            } else if token.token_type == TokenType::String {
                format!("'{}'", token.text)
            } else {
                token.text.to_string()
            };
            tokens.push((text, token.token_type));
        }

        Ok(Some(Expression::Command(Box::new(Command {
            this: self.join_command_tokens(tokens),
        }))))
    }

    /// Parses AND expressions, delegating to the precedence chain.
    pub(crate) fn parse_conjunction(&mut self) -> Result<Option<Expression>> {
        match self.parse_and() {
            Ok(expr) => Ok(Some(expr)),
            Err(_) => Ok(None),
        }
    }

    /// Parses a named or unnamed constraint.
    pub(crate) fn parse_constraint(&mut self) -> Result<Option<Expression>> {
        self.with_parser_depth(|parser| parser.parse_constraint_inner())
    }

    #[inline(never)]
    pub(crate) fn parse_constraint_inner(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::Constraint) {
            return self.parse_unnamed_constraint();
        }

        let name = self.parse_id_var()?;
        if name.is_none() {
            return Ok(None);
        }

        let expressions = self.parse_unnamed_constraints()?;

        Ok(Some(Expression::Constraint(Box::new(Constraint {
            this: Box::new(name.ok_or_else(|| {
                sqlquack_syntax::tokens::errors::Error::internal(
                    "a value the parser checked is missing",
                )
            })?),
            expressions,
        }))))
    }

    /// Parses several unnamed constraints.
    pub(crate) fn parse_unnamed_constraints(&mut self) -> Result<Vec<Expression>> {
        let mut constraints: Vec<Expression> = Vec::new();

        while let Some(constraint) = self.parse_unnamed_constraint()? {
            constraints.push(constraint);
        }

        Ok(constraints)
    }

    /// Parses one unnamed constraint.
    pub(crate) fn parse_unnamed_constraint(&mut self) -> Result<Option<Expression>> {
        if self.match_text_seq(&["PRIMARY", "KEY"]) {
            return self.parse_primary_key();
        }

        if self.match_texts(&["UNIQUE"]) {
            return self.parse_unique();
        }

        if self.match_text_seq(&["FOREIGN", "KEY"]) {
            return self.parse_foreign_key();
        }

        if self.match_texts(&["CHECK"]) {
            let expr = self.parse_wrapped()?;
            if let Some(check_expr) = expr {
                return Ok(Some(Expression::CheckColumnConstraint(Box::new(
                    CheckColumnConstraint {
                        this: Box::new(check_expr),
                        enforced: None,
                    },
                ))));
            }
        }

        if self.match_text_seq(&["NOT", "NULL"]) {
            return Ok(Some(Expression::NotNullColumnConstraint(Box::new(
                NotNullColumnConstraint { allow_null: None },
            ))));
        }

        if self.match_texts(&["NULL"]) {
            return Ok(Some(Expression::NotNullColumnConstraint(Box::new(
                NotNullColumnConstraint {
                    allow_null: Some(Box::new(Expression::Boolean(BooleanLiteral {
                        value: true,
                    }))),
                },
            ))));
        }

        if self.match_token(TokenType::Default) {
            let default_value = self.parse_bitwise()?;
            if let Some(val) = default_value {
                let for_column = if self.match_token(TokenType::For) {
                    Some(self.expect_identifier_with_quoted()?)
                } else {
                    None
                };
                return Ok(Some(Expression::DefaultColumnConstraint(Box::new(
                    DefaultColumnConstraint {
                        this: Box::new(val),
                        for_column,
                    },
                ))));
            }
        }

        if self.match_texts(&["REFERENCES"]) {
            return self.parse_references();
        }

        Ok(None)
    }

    /// Parses BigQuery FOR variable IN (query) DO statements END FOR.
    pub(crate) fn parse_for_in(&mut self) -> Result<Expression> {
        let this = self
            .parse_range()?
            .ok_or_else(|| self.parse_error("Expected expression after FOR"))?;

        self.match_text_seq(&["DO"]);

        let expression = self.parse_statement()?;

        Ok(Expression::ForIn(Box::new(ForIn {
            this: Box::new(this),
            expression: Box::new(expression),
        })))
    }

    /// Parses DECLARE var type [DEFAULT expr], ...
    pub(crate) fn parse_declare(&mut self) -> Result<Option<Expression>> {
        let replace = self.match_text_seq(&["OR", "REPLACE"]);

        let mut expressions: Vec<Expression> = Vec::new();

        let saved = self.current;
        let mut multi_names: Vec<Expression> = Vec::new();
        if let Some(first_var) = self.parse_id_var()? {
            if self.check(TokenType::Comma) && !self.check_identifier("CURSOR") {
                multi_names.push(first_var);
                while self.match_token(TokenType::Comma) {
                    if let Some(next_var) = self.parse_id_var()? {
                        multi_names.push(next_var);
                    } else {
                        break;
                    }
                }
                if multi_names.len() > 1 && !self.is_at_end() && !self.check(TokenType::Semicolon) {
                    let data_type = self.parse_data_type()?;
                    let kind_str = self.data_type_to_sql(&data_type);
                    let default = if self.match_token(TokenType::Default)
                        || self.match_token(TokenType::Eq)
                    {
                        Some(Box::new(self.parse_expression()?))
                    } else {
                        None
                    };
                    let first_name = multi_names.remove(0);
                    expressions.push(Expression::DeclareItem(Box::new(DeclareItem {
                        this: Box::new(first_name),
                        kind: Some(kind_str),
                        default,
                        has_as: false,
                        additional_names: multi_names,
                        span: SourceSpan::default(),
                    })));
                    return Ok(Some(Expression::Declare(Box::new(Declare {
                        expressions,
                        replace,
                    }))));
                }
            }
        }
        self.current = saved;

        loop {
            if let Some(item) = self.parse_declareitem()? {
                expressions.push(item);
            } else {
                break;
            }
            if self.match_token(TokenType::Comma) {
                continue;
            }
            break;
        }

        if !expressions.is_empty() {
            return Ok(Some(Expression::Declare(Box::new(Declare {
                expressions,
                replace,
            }))));
        }

        Ok(None)
    }

    /// Parses one DECLARE item, TSQL `@var [AS] type [= expr]` and cursors included.
    pub(crate) fn parse_declareitem(&mut self) -> Result<Option<Expression>> {
        if self.check_identifier("VAR") || self.check_identifier("VARIABLE") {
            self.skip();
        }

        let var = if let Some(v) = self.parse_id_var()? {
            v
        } else {
            return Ok(None);
        };

        if self.check_identifier("CURSOR") {
            self.skip();
            if self.match_token(TokenType::For) {
                let start = self.current;
                while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                    self.skip();
                }
                let query_str = self.tokens_to_sql_uppercased(start, self.current);
                let kind_str = format!("CURSOR FOR {}", query_str);
                return Ok(Some(Expression::DeclareItem(Box::new(DeclareItem {
                    this: Box::new(var),
                    kind: Some(kind_str),
                    default: None,
                    has_as: false,
                    additional_names: Vec::new(),
                    span: SourceSpan::default(),
                }))));
            } else {
                return Ok(Some(Expression::DeclareItem(Box::new(DeclareItem {
                    this: Box::new(var),
                    kind: Some("CURSOR".to_string()),
                    default: None,
                    has_as: false,
                    additional_names: Vec::new(),
                    span: SourceSpan::default(),
                }))));
            }
        }

        let has_as = self.match_token(TokenType::As);

        if self.check(TokenType::Table) {
            self.skip();
            if self.match_token(TokenType::LParen) {
                let start = self.current;
                let mut depth = 1;
                while depth > 0 && !self.is_at_end() {
                    if self.check(TokenType::LParen) {
                        depth += 1;
                    }
                    if self.check(TokenType::RParen) {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    self.skip();
                }
                let col_defs_str = self.tokens_to_sql_uppercased(start, self.current);
                self.expect_token(TokenType::RParen)?;
                let kind_str = format!("TABLE ({})", col_defs_str);
                return Ok(Some(Expression::DeclareItem(Box::new(DeclareItem {
                    this: Box::new(var),
                    kind: Some(kind_str),
                    default: None,
                    has_as,
                    additional_names: Vec::new(),
                    span: SourceSpan::default(),
                }))));
            } else {
                return Ok(Some(Expression::DeclareItem(Box::new(DeclareItem {
                    this: Box::new(var),
                    kind: Some("TABLE".to_string()),
                    default: None,
                    has_as,
                    additional_names: Vec::new(),
                    span: SourceSpan::default(),
                }))));
            }
        }

        let kind_str = if self.check(TokenType::Eq)
            || self.check(TokenType::Default)
            || self.is_at_end()
            || self.check(TokenType::Semicolon)
            || self.check(TokenType::Comma)
        {
            None
        } else {
            let data_type = self.parse_data_type()?;
            Some(self.data_type_to_sql(&data_type))
        };

        let default = if self.match_token(TokenType::Default) || self.match_token(TokenType::Eq) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        Ok(Some(Expression::DeclareItem(Box::new(DeclareItem {
            this: Box::new(var),
            kind: kind_str,
            default,
            has_as,
            additional_names: Vec::new(),
            span: SourceSpan::default(),
        }))))
    }

    /// Convert a DataType to its SQL string representation
    pub(crate) fn data_type_to_sql(&self, dt: &DataType) -> String {
        match dt {
            DataType::Int128 => "INT128".to_string(),
            DataType::UInt8 => "UTINYINT".to_string(),
            DataType::UInt16 => "USMALLINT".to_string(),
            DataType::UInt32 => "UINTEGER".to_string(),
            DataType::UInt64 => "UBIGINT".to_string(),
            DataType::UInt128 => "UINT128".to_string(),
            DataType::Boolean => "BOOLEAN".to_string(),
            DataType::TinyInt { length } => {
                if let Some(n) = length {
                    format!("TINYINT({})", n)
                } else {
                    "TINYINT".to_string()
                }
            }
            DataType::SmallInt { length } => {
                if let Some(n) = length {
                    format!("SMALLINT({})", n)
                } else {
                    "SMALLINT".to_string()
                }
            }
            DataType::Int {
                length,
                integer_spelling,
            } => {
                if let Some(n) = length {
                    if *integer_spelling {
                        format!("INTEGER({})", n)
                    } else {
                        format!("INT({})", n)
                    }
                } else if *integer_spelling {
                    "INTEGER".to_string()
                } else {
                    "INT".to_string()
                }
            }
            DataType::BigInt { length } => {
                if let Some(n) = length {
                    format!("BIGINT({})", n)
                } else {
                    "BIGINT".to_string()
                }
            }
            DataType::Float {
                precision, scale, ..
            } => match (precision, scale) {
                (Some(p), Some(s)) => format!("FLOAT({}, {})", p, s),
                (Some(p), None) => format!("FLOAT({})", p),
                _ => "FLOAT".to_string(),
            },
            DataType::Double { precision, scale } => match (precision, scale) {
                (Some(p), Some(s)) => format!("DOUBLE({}, {})", p, s),
                (Some(p), None) => format!("DOUBLE({})", p),
                _ => "DOUBLE".to_string(),
            },
            DataType::Decimal { precision, scale } => match (precision, scale) {
                (Some(p), Some(s)) => format!("DECIMAL({}, {})", p, s),
                (Some(p), None) => format!("DECIMAL({})", p),
                _ => "DECIMAL".to_string(),
            },
            DataType::Oracle { oracle_type } => self.oracle_data_type_to_string(oracle_type),
            DataType::Char { length } => {
                if let Some(n) = length {
                    format!("CHAR({})", n)
                } else {
                    "CHAR".to_string()
                }
            }
            DataType::VarChar { length, .. } => {
                if let Some(n) = length {
                    format!("VARCHAR({})", n)
                } else {
                    "VARCHAR".to_string()
                }
            }
            DataType::Text => "TEXT".to_string(),
            DataType::Date => "DATE".to_string(),
            DataType::Time { precision, .. } => {
                if let Some(p) = precision {
                    format!("TIME({})", p)
                } else {
                    "TIME".to_string()
                }
            }
            DataType::Timestamp { precision, .. } => {
                if let Some(p) = precision {
                    format!("TIMESTAMP({})", p)
                } else {
                    "TIMESTAMP".to_string()
                }
            }
            DataType::Binary { length } => {
                if let Some(n) = length {
                    format!("BINARY({})", n)
                } else {
                    "BINARY".to_string()
                }
            }
            DataType::VarBinary { length } => {
                if let Some(n) = length {
                    format!("VARBINARY({})", n)
                } else {
                    "VARBINARY".to_string()
                }
            }
            DataType::Blob => "BLOB".to_string(),
            DataType::String { length: Some(n) } => format!("STRING({})", n),
            DataType::String { length: None } => "STRING".to_string(),
            DataType::Json => "JSON".to_string(),
            DataType::Uuid => "UUID".to_string(),
            DataType::Custom { name } => name.clone(),
            _ => format!("{:?}", dt),
        }
    }

    /// Parses OR expressions, delegating to the precedence chain.
    pub(crate) fn parse_disjunction(&mut self) -> Result<Option<Expression>> {
        match self.parse_or() {
            Ok(expr) => Ok(Some(expr)),
            Err(error) if is_guard_error(&error) => Err(error),
            Err(_) => Ok(None),
        }
    }

    /// Parses the Redshift DISTKEY(column) property.
    pub(crate) fn parse_distkey(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::LParen) {
            return Ok(None);
        }

        let column = self.parse_id_var()?;
        if column.is_none() {
            return Ok(None);
        }

        self.match_token(TokenType::RParen);

        Ok(Some(Expression::DistKeyProperty(Box::new(
            DistKeyProperty {
                this: Box::new(column.ok_or_else(|| {
                    sqlquack_syntax::tokens::errors::Error::internal(
                        "a value the parser checked is missing",
                    )
                })?),
            },
        ))))
    }

    /// Parses the DISTRIBUTED BY property.
    pub(crate) fn parse_distributed_property(&mut self) -> Result<Option<Expression>> {
        let mut kind = "HASH".to_string();
        let mut expressions: Vec<Expression> = Vec::new();

        if self.match_text_seq(&["BY", "HASH"]) {
            if let Some(Expression::Tuple(t)) = self.parse_wrapped_id_vars()? {
                expressions = t.expressions;
            }
        } else if self.match_text_seq(&["BY", "RANDOM"]) {
            kind = "RANDOM".to_string();
        } else {
            return Ok(None);
        }

        let buckets = if self.match_text_seq(&["BUCKETS"]) {
            if !self.match_text_seq(&["AUTO"]) {
                self.parse_number()?
            } else {
                None
            }
        } else {
            None
        };

        let order = self.parse_order()?;

        Ok(Some(Expression::DistributedByProperty(Box::new(
            DistributedByProperty {
                expressions,
                kind,
                buckets: buckets.map(Box::new),
                order: order.map(Box::new),
            },
        ))))
    }

    /// Parses a LIKE pattern's ESCAPE clause, if present.
    pub(crate) fn parse_escape(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::Escape) {
            return Ok(None);
        }

        if let Some(escape_char) = self.parse_string()? {
            return Ok(Some(escape_char));
        }

        if let Some(null_expr) = self.parse_null()? {
            return Ok(Some(null_expr));
        }

        Ok(None)
    }

    /// Parses a field: a primary, a function call or an identifier.
    pub(crate) fn parse_field(&mut self) -> Result<Option<Expression>> {
        if let Some(expr) = self.parse_string()? {
            return Ok(Some(expr));
        }
        if let Some(expr) = self.parse_number()? {
            return Ok(Some(expr));
        }
        if let Some(expr) = self.parse_boolean()? {
            return Ok(Some(expr));
        }
        if let Some(expr) = self.parse_null()? {
            return Ok(Some(expr));
        }
        if let Some(expr) = self.parse_star()? {
            return Ok(Some(expr));
        }
        if let Some(expr) = self.parse_identifier()? {
            return Ok(Some(expr));
        }
        if let Some(expr) = self.parse_var()? {
            return Ok(Some(expr));
        }
        if self.check_keyword() {
            let token = self.advance()?;
            return Ok(Some(Expression::Identifier(identifier_from_token(
                token, false,
            ))));
        }
        Ok(None)
    }

    /// Parses a field definition: name, type and optional constraints.
    pub(crate) fn parse_field_def(&mut self) -> Result<Option<Expression>> {
        let field = self.parse_field()?;

        if field.is_none() {
            return Ok(None);
        }

        self.parse_column_def_with_field(field)
    }

    /// Helper to parse a column definition with a pre-parsed field name
    pub(crate) fn parse_column_def_with_field(
        &mut self,
        field: Option<Expression>,
    ) -> Result<Option<Expression>> {
        if field.is_none() {
            return Ok(None);
        }

        let this = field.ok_or_else(|| {
            sqlquack_syntax::tokens::errors::Error::internal(
                "a value the parser checked is missing",
            )
        })?;

        let name_ident = match &this {
            Expression::Column(col) => col.name.clone(),
            Expression::Identifier(id) => id.clone(),
            Expression::Var(v) => Identifier::new(v.this.clone()),
            _ => return Ok(None),
        };

        let data_type = match self.parse_data_type_optional()? {
            Some(dt) => dt,
            None => DataType::Unknown,
        };

        let mut col_def = ColumnDef::new(name_ident.name.clone(), data_type);
        col_def.name = name_ident;

        if self.match_text_seq(&["FOR", "ORDINALITY"]) {
            return Ok(Some(Expression::ColumnDef(Box::new(col_def))));
        }

        while let Some(constraint) = self.parse_column_constraint()? {
            match &constraint {
                Expression::NotNullColumnConstraint(_) => {
                    col_def.nullable = Some(false);
                    col_def.constraints.push(ColumnConstraint::NotNull);
                    col_def.constraint_order.push(ConstraintType::NotNull);
                }
                Expression::PrimaryKeyColumnConstraint(_) => {
                    col_def.primary_key = true;
                    col_def.constraints.push(ColumnConstraint::PrimaryKey);
                }
                Expression::UniqueColumnConstraint(_) => {
                    col_def.unique = true;
                    col_def.constraints.push(ColumnConstraint::Unique);
                }
                Expression::DefaultColumnConstraint(dc) => {
                    col_def.default = Some((*dc.this).clone());
                    col_def
                        .constraints
                        .push(ColumnConstraint::Default((*dc.this).clone()));
                }
                Expression::AutoIncrementColumnConstraint(_) => {
                    col_def.auto_increment = true;
                }
                Expression::CommentColumnConstraint(_) => {}
                Expression::CheckColumnConstraint(cc) => {
                    col_def
                        .constraints
                        .push(ColumnConstraint::Check((*cc.this).clone()));
                }
                Expression::PathColumnConstraint(pc) => {
                    col_def
                        .constraints
                        .push(ColumnConstraint::Path((*pc.this).clone()));
                    col_def.constraint_order.push(ConstraintType::Path);
                }
                _ => {}
            }
        }

        Ok(Some(Expression::ColumnDef(Box::new(col_def))))
    }

    /// Parses a FOREIGN KEY constraint.
    pub(crate) fn parse_foreign_key(&mut self) -> Result<Option<Expression>> {
        if self.match_text_seq(&["NO", "ACTION"]) {
            return Ok(Some(Expression::ForeignKey(Box::new(ForeignKey {
                expressions: Vec::new(),
                reference: None,
                delete: None,
                update: None,
                options: Vec::new(),
            }))));
        }
        Ok(None)
    }

    /// Parses a function call, ODBC `{fn name(args)}` included.
    pub(crate) fn parse_function(&mut self) -> Result<Option<Expression>> {
        let fn_syntax = if self.check(TokenType::LBrace) {
            if let Some(next) = self.tokens.get(self.current + 1) {
                if next.text.eq_ignore_ascii_case("FN") {
                    self.skip();
                    self.skip();
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        let func = self.parse_function_call()?;

        if fn_syntax {
            self.match_token(TokenType::RBrace);
        }

        Ok(func)
    }

    /// Parses a call's arguments, with aliases and key-value pairs.
    pub(crate) fn parse_function_args_list(&mut self) -> Result<Vec<Expression>> {
        let mut args: Vec<Expression> = Vec::new();

        if self.check(TokenType::RParen) {
            return Ok(args);
        }

        loop {
            let is_table_or_model_arg = !self.is_at_end()
                && (self.check(TokenType::Table) || self.peek_text().eq_ignore_ascii_case("MODEL"));

            let expr = if let Some(star_like) = self.parse_clickhouse_bare_star_like()? {
                Some(star_like)
            } else if is_table_or_model_arg {
                let prefix = self.peek_text().to_ascii_uppercase();
                let saved_pos = self.current;
                self.skip();

                if !self.is_at_end()
                    && !self.check(TokenType::FArrow)
                    && !self.check(TokenType::ColonEq)
                {
                    if let Some(table_expr) = self.parse_table_parts()? {
                        Some(Expression::TableArgument(Box::new(TableArgument {
                            prefix,
                            this: table_expr,
                        })))
                    } else {
                        self.current = saved_pos;
                        self.parse_assignment()?
                    }
                } else {
                    self.current = saved_pos;
                    self.parse_assignment()?
                }
            } else {
                self.parse_assignment()?
            };

            let Some(expr) = expr else {
                return Err(self.parse_error("Expected a function argument"));
            };
            if self.match_token(TokenType::As) {
                let alias_token = self.advance()?;
                let alias_name = if alias_token.token_type == TokenType::QuotedIdentifier {
                    let raw = alias_token.text.to_string();
                    let mut ident = Identifier::new(raw);
                    ident.quoted = true;
                    ident
                } else {
                    Identifier::new(alias_token.text.clone()).with_span(alias_token.span)
                };
                args.push(Expression::Alias(Box::new(
                    sqlquack_syntax::expressions::operators::models::Alias {
                        span: SourceSpan::default(),
                        this: expr,
                        alias: alias_name,
                        column_aliases: Vec::new(),
                        alias_explicit_as: false,
                        alias_keyword: None,
                        pre_alias_comments: Vec::new(),
                        trailing_comments: Vec::new(),
                        inferred_type: None,
                    },
                )));
            } else {
                args.push(expr);
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(args)
    }

    /// Parses a function call `name(arg, ...)`.
    pub(crate) fn parse_function_call(&mut self) -> Result<Option<Expression>> {
        if self.is_at_end() {
            return Ok(None);
        }

        let token = self.peek().clone();
        let token_type = token.token_type;
        let name = token.text.to_string();
        let _upper_name = name.to_ascii_uppercase();

        if self.is_no_paren_function() && !self.check_next(TokenType::LParen) {
            self.skip();
            return Ok(Some(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name,
                args: Vec::new(),
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: true,
                quoted: false,
                span: SourceSpan::default(),
                inferred_type: None,
                export_state: false,
            }))));
        }

        if !self.check_next(TokenType::LParen) {
            return Ok(None);
        }

        let is_valid_func_token = matches!(
            token_type,
            TokenType::Identifier
                | TokenType::Var
                | TokenType::If
                | TokenType::Left
                | TokenType::Right
                | TokenType::Insert
                | TokenType::Replace
                | TokenType::Row
                | TokenType::Index
        );
        if !is_valid_func_token {
            return Ok(None);
        }

        self.skip();
        self.skip();

        let distinct = self.match_token(TokenType::Distinct);

        let args = self.parse_function_args_list()?;

        self.match_token(TokenType::RParen);

        let func_expr = Expression::Function(Box::new(Function {
            on_error: None,
            qualified_name: Vec::new(),
            source_dialect: None,

            name,
            args,
            distinct,
            trailing_comments: Vec::new(),
            use_bracket_syntax: false,
            no_parens: false,
            quoted: false,
            span: SourceSpan::default(),
            inferred_type: None,
            export_state: false,
        }));

        if self.match_token(TokenType::Over) && self.match_token(TokenType::LParen) {
            let spec = self.parse_window_spec_inner()?;
            self.expect_token(TokenType::RParen)?;

            if let Some(spec_expr) = spec {
                return Ok(Some(spec_expr));
            }
        }

        Ok(Some(func_expr))
    }

    /// Parses Snowflake SEMANTIC_VIEW(name METRICS ... DIMENSIONS ... WHERE ...).
    pub(crate) fn parse_semantic_view(&mut self) -> Result<Expression> {
        if self.is_snowflake() && self.check(TokenType::String) {
            return Err(self.parse_error("Expected a semantic view name"));
        }
        let this = match self.is_snowflake() {
            true => Expression::Table(Box::new(self.parse_object_name()?)),
            false => self.parse_primary()?,
        };

        let mut metrics = None;
        let mut dimensions = None;
        let mut facts = None;
        let mut where_clause = None;
        let mut dimensions_first = false;

        while !self.check(TokenType::RParen) && !self.is_at_end() {
            if self.match_identifier("METRICS") {
                dimensions_first |= dimensions.is_some();
                let exprs = self.parse_semantic_view_list()?;
                metrics = Some(Box::new(Expression::Tuple(Box::new(Tuple {
                    expressions: exprs,
                }))));
            } else if self.match_identifier("DIMENSIONS") {
                let exprs = self.parse_semantic_view_list()?;
                dimensions = Some(Box::new(Expression::Tuple(Box::new(Tuple {
                    expressions: exprs,
                }))));
            } else if self.match_identifier("FACTS") {
                dimensions_first |= dimensions.is_some();
                let exprs = self.parse_semantic_view_list()?;
                facts = Some(Box::new(Expression::Tuple(Box::new(Tuple {
                    expressions: exprs,
                }))));
            } else if self.match_token(TokenType::Where) {
                where_clause = Some(Box::new(self.parse_expression()?));
                break;
            } else {
                break;
            }
        }

        if self.is_snowflake() && metrics.is_none() && dimensions.is_none() && facts.is_none() {
            return Err(self.parse_error("SEMANTIC_VIEW needs METRICS, DIMENSIONS or FACTS"));
        }
        Ok(Expression::SemanticView(Box::new(SemanticView {
            this: Box::new(this),
            metrics,
            dimensions,
            facts,
            where_: where_clause,
            dimensions_first,
        })))
    }

    /// Parses a SEMANTIC_VIEW clause list up to the next clause keyword, aliases allowed.
    pub(crate) fn parse_semantic_view_list(&mut self) -> Result<Vec<Expression>> {
        let first = self.parse_semantic_view_element()?;
        let mut exprs = vec![first];
        while self.match_token(TokenType::Comma) {
            if self.check_identifier("METRICS")
                || self.check_identifier("DIMENSIONS")
                || self.check_identifier("FACTS")
                || self.check(TokenType::Where)
                || self.check(TokenType::RParen)
            {
                break;
            }
            exprs.push(self.parse_semantic_view_element()?);
        }
        Ok(exprs)
    }

    /// Parse a single SEMANTIC_VIEW element: expression [AS alias]
    pub(crate) fn parse_semantic_view_element(&mut self) -> Result<Expression> {
        let expr = self
            .parse_disjunction()?
            .ok_or_else(|| self.parse_error("Expected expression in SEMANTIC_VIEW clause"))?;
        let bare_alias = self.is_snowflake()
            && (self.check(TokenType::Identifier)
                || self.check(TokenType::QuotedIdentifier)
                || self.check(TokenType::Var))
            && !self.is_snowflake_reserved_keyword()
            && (self.check(TokenType::QuotedIdentifier)
                || !crate::parser::constants::SNOWFLAKE_SEMANTIC_VIEW_CLAUSES
                    .iter()
                    .any(|word| self.peek_text().eq_ignore_ascii_case(word)));
        let explicit_as = !bare_alias && self.match_token(TokenType::As);
        if bare_alias || explicit_as {
            let alias_keyword = explicit_as.then(|| {
                let token = self.previous();
                match self.source.is_some() {
                    true => self.source_text_range(token.span.start, token.span.end),
                    false => token.text.to_string(),
                }
            });
            let alias = self.expect_identifier_or_keyword_with_quoted()?;
            Ok(Expression::Alias(Box::new(
                sqlquack_syntax::expressions::operators::models::Alias {
                    span: SourceSpan::default(),
                    this: expr,
                    alias,
                    column_aliases: Vec::new(),
                    alias_explicit_as: explicit_as,
                    alias_keyword,
                    pre_alias_comments: Vec::new(),
                    trailing_comments: Vec::new(),
                    inferred_type: None,
                },
            )))
        } else {
            Ok(expr)
        }
    }
}
