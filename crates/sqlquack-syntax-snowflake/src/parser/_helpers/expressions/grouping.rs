//! Parser expressions: grouping.

use crate::parser::constants::BINARY_ARITY;
use crate::parser::constants::QUALIFIED_NAME_PARTS;
use crate::parser::constants::RIGHT_PAREN;
use crate::parser::constants::TERNARY_ARITY;
use crate::parser::models::Parser;
use crate::parser::models::ParserDepthScope;
use sqlquack_syntax::expressions::columns::models::PrimaryKeyColumnConstraint;
use sqlquack_syntax::expressions::functions::models::Function;
use sqlquack_syntax::expressions::functions::models::JSONColumnDef;
use sqlquack_syntax::expressions::functions::models::JSONSchema;
use sqlquack_syntax::expressions::functions::models::OpenJSON;
use sqlquack_syntax::expressions::functions::models::OpenJSONColumnDef;
use sqlquack_syntax::expressions::names::main::identifier_from_token::identifier_from_token;
use sqlquack_syntax::expressions::names::models::BooleanLiteral;
use sqlquack_syntax::expressions::names::models::Column;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::names::models::Null;
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::operators::models::DotAccess;
use sqlquack_syntax::expressions::operators::models::Group;
use sqlquack_syntax::expressions::operators::models::IndexParameters;
use sqlquack_syntax::expressions::operators::models::LambdaExpr;
use sqlquack_syntax::expressions::operators::models::Parameter;
use sqlquack_syntax::expressions::operators::models::ParameterStyle;
use sqlquack_syntax::expressions::operators::models::Placeholder;
use sqlquack_syntax::expressions::operators::models::PrimaryKey;
use sqlquack_syntax::expressions::operators::models::Returning;
use sqlquack_syntax::expressions::operators::models::WithFill;
use sqlquack_syntax::expressions::queries::models::OrderBy;
use sqlquack_syntax::expressions::queries::models::Ordered;
use sqlquack_syntax::expressions::queries::models::Subquery;
use sqlquack_syntax::expressions::statements::models::HistoricalData;
use sqlquack_syntax::expressions::statements::models::Merge;
use sqlquack_syntax::expressions::statements::models::NextValueFor;
use sqlquack_syntax::expressions::statements::models::Paren;
use sqlquack_syntax::expressions::statements::models::Var;
use sqlquack_syntax::expressions::statements::models::When;
use sqlquack_syntax::expressions::statements::models::Whens;
use sqlquack_syntax::expressions::tables::models::OnCluster;
use sqlquack_syntax::expressions::tables::models::Partition;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::typed_functions::models::DateTimeField;
use sqlquack_syntax::expressions::typed_functions::models::IfFunc;
use sqlquack_syntax::expressions::typed_functions::models::StrPosition;
use sqlquack_syntax::expressions::types::main::parse_interval_unit_from_string::parse_interval_unit_from_string;
use sqlquack_syntax::expressions::types::models::DataType;
use sqlquack_syntax::expressions::types::models::IntervalUnit;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::SourceSpan;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Parses a GROUP BY clause.
    pub(crate) fn parse_group(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::Group) {
            return Ok(None);
        }
        self.match_token(TokenType::By);

        let all = if self.match_token(TokenType::All) {
            Some(true)
        } else if self.match_token(TokenType::Distinct) {
            Some(false)
        } else {
            None
        };

        let mut expressions: Vec<Expression> = Vec::new();
        loop {
            match self.parse_expression() {
                Ok(expr) => expressions.push(expr),
                Err(_) => break,
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        let totals = if self.match_text_seq(&["WITH", "TOTALS"]) || self.match_text_seq(&["TOTALS"])
        {
            Some(Box::new(Expression::Boolean(BooleanLiteral {
                value: true,
            })))
        } else {
            None
        };

        Ok(Some(Expression::Group(Box::new(Group {
            expressions,
            grouping_sets: None,
            cube: None,
            rollup: None,
            totals,
            all,
        }))))
    }

    /// Parses Snowflake AT/BEFORE time travel, such as AT(TIMESTAMP => expr).
    pub(crate) fn parse_historical_data(&mut self) -> Result<Option<Expression>> {
        let start_index = self.current;

        let this = if self.match_texts(&["AT", "BEFORE", "END"]) {
            self.previous_text().to_ascii_uppercase()
        } else {
            return Ok(None);
        };

        if !self.match_token(TokenType::LParen) {
            self.current = start_index;
            return Ok(None);
        }

        let kind = if self.match_texts(&["OFFSET", "STATEMENT", "STREAM", "TIMESTAMP", "VERSION"]) {
            self.previous_text().to_ascii_uppercase()
        } else {
            self.current = start_index;
            return Ok(None);
        };

        if !self.match_token(TokenType::FArrow) {
            self.current = start_index;
            return Ok(None);
        }

        let expression = self.parse_bitwise()?;
        if expression.is_none() {
            self.current = start_index;
            return Ok(None);
        }

        if !self.match_token(TokenType::RParen) && self.is_snowflake() {
            return Err(self.parse_error(format!("Expected ) to close {this}")));
        }

        Ok(Some(Expression::HistoricalData(Box::new(HistoricalData {
            this: Box::new(Expression::Identifier(Identifier::new(this))),
            kind,
            expression: Box::new(expression.ok_or_else(|| {
                sqlquack_syntax::tokens::errors::Error::internal(
                    "a value the parser checked is missing",
                )
            })?),
        }))))
    }

    /// Parses an identifier or variable, more permissively than parse_identifier.
    pub(crate) fn parse_id_var(&mut self) -> Result<Option<Expression>> {
        if let Some(ident) = self.parse_identifier()? {
            return Ok(Some(ident));
        }

        if self.check(TokenType::Var) {
            return Ok(Some(Expression::Identifier(identifier_from_token(
                self.advance()?,
                false,
            ))));
        }

        if self.check(TokenType::String) {
            return Ok(Some(Expression::Identifier(identifier_from_token(
                self.advance()?,
                true,
            ))));
        }

        if self.check(TokenType::Select)
            || self.check(TokenType::From)
            || self.check(TokenType::Where)
            || self.check(TokenType::And)
            || self.check(TokenType::Or)
            || self.check(TokenType::Not)
            || self.check(TokenType::True)
            || self.check(TokenType::False)
            || self.check(TokenType::Null)
        {
            return Ok(None);
        }

        Ok(None)
    }

    /// Parses a quoted identifier.
    pub(crate) fn parse_identifier(&mut self) -> Result<Option<Expression>> {
        if self.check(TokenType::QuotedIdentifier) || self.check(TokenType::Identifier) {
            let quoted = self.peek().token_type == TokenType::QuotedIdentifier;
            return Ok(Some(Expression::Identifier(identifier_from_token(
                self.advance()?,
                quoted,
            ))));
        }
        Ok(None)
    }

    /// Parses IF(cond, a, b) or IF cond THEN a ELSE b END.
    pub(crate) fn parse_if(&mut self) -> Result<Option<Expression>> {
        if !self.check(TokenType::LParen) {
            return self.parse_if_statement();
        }
        if let Some(expr) = self.parse_if_special()? {
            return Ok(Some(expr));
        }
        self.parse_if_statement()
    }

    #[inline(never)]
    pub(crate) fn parse_if_special(&mut self) -> Result<Option<Expression>> {
        let original_name = self
            .tokens
            .get(self.current.saturating_sub(1))
            .map(|token| {
                if self.source.is_some() {
                    self.source_text_range(token.span.start, token.span.end)
                } else {
                    token.text.to_string()
                }
            });

        if self.match_token(TokenType::LParen) {
            if self.check(TokenType::RParen) {
                self.skip();
                return Ok(Some(Expression::Function(Box::new(Function {
                    on_error: None,
                    qualified_name: Vec::new(),
                    source_dialect: None,

                    name: original_name.clone().unwrap_or_else(|| "IF".to_string()),
                    args: vec![],
                    distinct: false,
                    trailing_comments: Vec::new(),
                    use_bracket_syntax: false,
                    no_parens: false,
                    quoted: false,
                    span: SourceSpan::default(),
                    inferred_type: None,
                    export_state: false,
                }))));
            }
            let args = self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;

            if args.len() == TERNARY_ARITY {
                return Ok(Some(Expression::IfFunc(Box::new(IfFunc {
                    original_name: original_name.clone(),
                    condition: args[0].clone(),
                    true_value: args[1].clone(),
                    false_value: Some(args[2].clone()),
                    inferred_type: None,
                }))));
            } else if args.len() == BINARY_ARITY {
                return Ok(Some(Expression::IfFunc(Box::new(IfFunc {
                    original_name: original_name.clone(),
                    condition: args[0].clone(),
                    true_value: args[1].clone(),
                    false_value: None,
                    inferred_type: None,
                }))));
            } else if args.len() == 1 {
                return Ok(Some(Expression::Function(Box::new(Function {
                    on_error: None,
                    qualified_name: Vec::new(),
                    source_dialect: None,

                    name: original_name.clone().unwrap_or_else(|| "IF".to_string()),
                    args,
                    distinct: false,
                    trailing_comments: Vec::new(),
                    use_bracket_syntax: false,
                    no_parens: false,
                    quoted: false,
                    span: SourceSpan::default(),
                    inferred_type: None,
                    export_state: false,
                }))));
            } else {
                return Err(self.parse_error("IF function requires 2 or 3 arguments"));
            }
        }

        Ok(None)
    }

    #[inline(never)]
    pub(crate) fn parse_if_statement(&mut self) -> Result<Option<Expression>> {
        let condition = match self.parse_disjunction()? {
            Some(c) => c,
            None => return Ok(None),
        };

        self.parse_if_statement_tail(condition)
    }

    #[inline(never)]
    pub(crate) fn parse_if_statement_tail(
        &mut self,
        condition: Expression,
    ) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::Then) {
            return Ok(Some(condition));
        }

        let true_value = match self.parse_disjunction()? {
            Some(v) => v,
            None => return Err(self.parse_error("Expected expression after THEN")),
        };

        let false_value = if self.match_token(TokenType::Else) {
            match self.parse_disjunction()? {
                Some(v) => Some(v),
                None => return Err(self.parse_error("Expected expression after ELSE")),
            }
        } else {
            None
        };

        self.match_token(TokenType::End);

        Ok(Some(Expression::IfFunc(Box::new(IfFunc {
            original_name: None,
            condition,
            true_value,
            false_value,
            inferred_type: None,
        }))))
    }

    /// Parses index parameters.
    pub(crate) fn parse_index_params(&mut self) -> Result<Option<Expression>> {
        if self.match_text_seq(&["INCLUDE"]) {
            return Ok(Some(Expression::IndexParameters(Box::new(
                IndexParameters {
                    using: None,
                    include: None,
                    columns: Vec::new(),
                    with_storage: None,
                    partition_by: None,
                    tablespace: None,
                    where_: None,
                    on: None,
                },
            ))));
        }
        if self.match_text_seq(&["USING", "INDEX", "TABLESPACE"]) {
            return Ok(None);
        }
        Ok(None)
    }

    /// Parses INTO [TEMPORARY] [UNLOGGED] [TABLE] name.
    pub(crate) fn parse_into(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::Into) {
            return Ok(None);
        }

        let _temp = self.match_token(TokenType::Temporary);

        let _unlogged = self.match_text_seq(&["UNLOGGED"]);

        let _ = self.match_token(TokenType::Table);

        self.parse_table_parts()
    }

    /// Parses JSON_TABLE COLUMNS (def, ...), with PATH, FOR ORDINALITY and NESTED columns.
    pub(crate) fn parse_json_table_columns(&mut self) -> Result<Option<Expression>> {
        self.with_parser_depth(|parser| parser.parse_json_table_columns_inner())
    }

    #[inline(never)]
    pub(crate) fn parse_json_table_columns_inner(&mut self) -> Result<Option<Expression>> {
        if !self.match_text_seq(&["COLUMNS"]) {
            return Ok(None);
        }

        let has_parens = self.match_token(TokenType::LParen);

        let mut columns: Vec<Expression> = Vec::new();

        if has_parens {
            if !self.check(TokenType::RParen) {
                loop {
                    if let Some(col_def) = self.parse_json_table_column_def()? {
                        columns.push(col_def);
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            self.expect_token(TokenType::RParen)?;
        } else {
            if let Some(col_def) = self.parse_json_table_column_def()? {
                columns.push(col_def);
            }
        }

        Ok(Some(Expression::JSONSchema(Box::new(JSONSchema {
            expressions: columns,
        }))))
    }

    /// Parses one JSON_TABLE column definition, NESTED PATH included.
    pub(crate) fn parse_json_table_column_def(&mut self) -> Result<Option<Expression>> {
        if self.match_text_seq(&["NESTED"]) {
            self.match_text_seq(&["PATH"]);
            let path = self.parse_string()?;
            let nested_schema = self.parse_json_table_columns()?;

            return Ok(Some(Expression::JSONColumnDef(Box::new(JSONColumnDef {
                source_dialect: None,

                data_type: None,
                encoding: None,
                options: None,

                this: None,
                kind: None,
                format_json: false,
                path: path.map(Box::new),
                nested_schema: nested_schema.map(Box::new),
                ordinality: None,
            }))));
        }

        let name = self.parse_id_var()?;
        if name.is_none() {
            return Ok(None);
        }

        let ordinality = if self.match_text_seq(&["FOR", "ORDINALITY"]) {
            Some(Box::new(Expression::Boolean(BooleanLiteral {
                value: true,
            })))
        } else {
            None
        };

        let kind = if ordinality.is_none() {
            let data_type = self.parse_data_type_optional()?;
            data_type.map(|dt| self.data_type_to_string(&dt))
        } else {
            None
        };

        let format_json = self.match_text_seq(&["FORMAT", "JSON"]);

        let path = if self.match_text_seq(&["PATH"]) {
            self.parse_string()?
        } else {
            None
        };

        Ok(Some(Expression::JSONColumnDef(Box::new(JSONColumnDef {
            source_dialect: None,

            data_type: None,
            encoding: None,
            options: None,

            this: name.map(Box::new),
            kind,
            format_json,
            path: path.map(Box::new),
            nested_schema: None,
            ordinality,
        }))))
    }

    pub(crate) fn parse_function_lambda(&mut self) -> Result<Option<Expression>> {
        let start = self.current;
        let mut parentheses: Vec<Vec<String>> = Vec::new();
        let mut depths: Vec<ParserDepthScope> = Vec::new();
        loop {
            if let Some(mut expression) = self.parse_lambda()? {
                for mut comments in parentheses.into_iter().rev() {
                    if !self.match_token(TokenType::RParen) {
                        self.current = start;
                        return Ok(None);
                    }
                    comments.extend_from_slice(self.previous_trailing_comments());
                    expression = Expression::Paren(Box::new(Paren {
                        this: expression,
                        trailing_comments: comments,
                        span: SourceSpan::default(),
                    }));
                }
                return Ok(Some(expression));
            }
            if !self.match_token(TokenType::LParen) {
                self.current = start;
                return Ok(None);
            }
            depths.push(self.enter_parser_depth(1)?);
            parentheses.push(self.previous_trailing_comments().to_vec());
        }
    }

    /// Parses a lambda: `x -> x + 1`, `(x, y) -> ...` or DuckDB `LAMBDA row, x : x + 1`, whose parameter may be a keyword.
    pub(crate) fn parse_lambda(&mut self) -> Result<Option<Expression>> {
        let start_index = self.current;

        if self.match_token(TokenType::Lambda) {
            let mut params: Vec<Identifier> = Vec::new();
            loop {
                let word = self
                    .peek()
                    .text
                    .starts_with(|first: char| first.is_ascii_alphabetic() || first == '_');
                let keyword_name = word
                    && (self.check_next(TokenType::Comma) || self.check_next(TokenType::Colon));
                if self.is_identifier_token() || keyword_name {
                    let token = self.advance()?;
                    let quoted = token.token_type == TokenType::QuotedIdentifier;
                    params.push(identifier_from_token(token, quoted));
                } else {
                    break;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }

            if params.is_empty() {
                return Err(self.parse_error("LAMBDA requires at least one parameter"));
            }

            if !self.match_token(TokenType::Colon) {
                return Err(self.parse_error("Expected ':' after LAMBDA parameters"));
            }

            let body = self.parse_expression()?;
            return Ok(Some(Expression::Lambda(Box::new(LambdaExpr {
                parameters: params,
                body,
                colon: true,
                parameter_types: Vec::new(),
            }))));
        }

        let parenthesized = self.match_token(TokenType::LParen);
        let mut parameters: Vec<Identifier> = Vec::new();
        let mut parameter_types: Vec<Option<DataType>> = Vec::new();
        while !(parenthesized && self.check(TokenType::RParen)) {
            if !self.is_identifier_token() {
                self.current = start_index;
                return Ok(None);
            }
            let token = self.advance()?;
            let quoted = token.token_type == TokenType::QuotedIdentifier;
            parameters.push(identifier_from_token(token, quoted));
            let typed = parenthesized && self.is_type_keyword();
            parameter_types.push(match typed {
                true => match self.parse_data_type() {
                    Ok(data_type) => Some(data_type),
                    Err(_) => {
                        self.current = start_index;
                        return Ok(None);
                    }
                },
                false => None,
            });
            if !parenthesized || !self.match_token(TokenType::Comma) {
                break;
            }
        }
        if parenthesized && !self.match_token(TokenType::RParen) {
            self.current = start_index;
            return Ok(None);
        }

        if self.match_token(TokenType::Arrow) || self.match_token(TokenType::FArrow) {
            let body = self.parse_expression()?;
            let parameter_types = match parameter_types.iter().any(Option::is_some) {
                true => parameter_types,
                false => Vec::new(),
            };
            Ok(Some(Expression::Lambda(Box::new(LambdaExpr {
                parameters,
                body,
                colon: false,
                parameter_types,
            }))))
        } else {
            self.current = start_index;
            Ok(None)
        }
    }

    /// A MERGE target or source table with its optional alias, which `stop` ends when there is none.
    fn parse_merge_table_alias(
        &mut self,
        table: Expression,
        stop: TokenType,
    ) -> Result<Expression> {
        let explicit_as = self.match_token(TokenType::As);
        if !explicit_as && self.check(stop) {
            return Ok(table);
        }
        let Some(Expression::Identifier(alias)) = self.parse_id_var()? else {
            return Ok(table);
        };
        Ok(Expression::Alias(Box::new(Alias {
            this: table,
            alias,
            column_aliases: Vec::new(),
            alias_explicit_as: explicit_as,
            alias_keyword: None,
            pre_alias_comments: Vec::new(),
            trailing_comments: Vec::new(),
            inferred_type: None,
            span: SourceSpan::default(),
        })))
    }

    /// Parses a MERGE statement.
    pub(crate) fn parse_merge(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::Into) && self.applies_snowflake_context_rules() {
            return Err(self.parse_error("Expected INTO after MERGE"));
        }

        let mut target = Expression::Table(Box::new(self.parse_table_ref()?));

        if self.check(TokenType::With) && self.check_next(TokenType::LParen) {
            if let Expression::Table(ref mut table) = target {
                if let Some(hint_expr) = self.parse_table_hints()? {
                    match hint_expr {
                        Expression::Tuple(tuple) => {
                            table.hints = tuple.expressions;
                        }
                        other => {
                            table.hints = vec![other];
                        }
                    }
                }
            }
        }

        let target = self.parse_merge_table_alias(target, TokenType::Using)?;

        if !self.match_token(TokenType::Using) {
            return Err(self.parse_error("Expected USING in MERGE statement"));
        }

        let using = if self.match_token(TokenType::LParen) {
            let query = self.parse_statement()?;
            self.expect_token(TokenType::RParen)?;
            let trailing = self.previous_trailing_comments().to_vec();
            let mut subq = Subquery {
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
                trailing_comments: trailing,
                inferred_type: None,
                span: SourceSpan::default(),
            };
            let explicit_as = self.match_token(TokenType::As);
            if explicit_as || self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                let alias_name =
                    self.parse_unquoted_identifier(Self::expect_identifier_or_keyword)?;
                subq.alias = Some(alias_name);
                subq.alias_explicit_as = explicit_as;
                if self.match_token(TokenType::LParen) {
                    let mut cols: Vec<Identifier> = Vec::new();
                    loop {
                        let col_name =
                            self.parse_unquoted_identifier(Self::expect_identifier_or_keyword)?;
                        cols.push(col_name);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    self.expect_token(TokenType::RParen)?;
                    subq.column_aliases = cols;
                }
            }
            Expression::Subquery(Box::new(subq))
        } else {
            let table = Expression::Table(Box::new(self.parse_table_ref()?));
            self.parse_merge_table_alias(table, TokenType::On)?
        };

        let on = if self.match_token(TokenType::On) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        let using_cond = if self.match_token(TokenType::Using) {
            if self.match_token(TokenType::LParen) {
                let mut idents: Vec<Expression> = Vec::new();
                loop {
                    if let Some(ident) = self.parse_id_var()? {
                        idents.push(ident);
                    } else {
                        break;
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.match_token(TokenType::RParen);
                if !idents.is_empty() {
                    Some(Box::new(Expression::Tuple(Box::new(Tuple {
                        expressions: idents,
                    }))))
                } else {
                    None
                }
            } else {
                let mut idents: Vec<Expression> = Vec::new();
                loop {
                    if let Some(ident) = self.parse_id_var()? {
                        idents.push(ident);
                    } else {
                        break;
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                if !idents.is_empty() {
                    Some(Box::new(Expression::Tuple(Box::new(Tuple {
                        expressions: idents,
                    }))))
                } else {
                    None
                }
            }
        } else {
            None
        };

        let whens = self.parse_when_matched_clauses()?;

        let returning = if let Some(ret) = self.parse_returning()? {
            Some(ret)
        } else if self.match_token(TokenType::Output) {
            let output = self.parse_output_clause()?;
            Some(Expression::Returning(Box::new(Returning {
                expressions: output.columns,
                into: output.into_table.map(Box::new),
            })))
        } else {
            None
        };

        Ok(Some(Expression::Merge(Box::new(Merge {
            this: Box::new(target),
            using: Box::new(using),
            on,
            using_cond,
            whens: whens.map(Box::new),
            with_: None,
            returning: returning.map(Box::new),
            returning_aliases: Vec::new(),
        }))))
    }

    /// Parse multiple WHEN [NOT] MATCHED clauses for MERGE
    pub(crate) fn parse_when_matched_clauses(&mut self) -> Result<Option<Expression>> {
        let mut whens: Vec<Expression> = Vec::new();

        while self.match_token(TokenType::When) {
            let matched = !self.match_token(TokenType::Not);
            if !self.match_text_seq(&["MATCHED"]) && self.is_snowflake() {
                return Err(self.parse_error("Expected MATCHED in a MERGE WHEN clause"));
            }

            let source = if self.match_text_seq(&["BY", "TARGET"]) {
                Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: false,
                })))
            } else if self.match_text_seq(&["BY", "SOURCE"]) {
                Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: true,
                })))
            } else {
                None
            };

            let condition = if self.match_token(TokenType::And) {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };

            if !self.match_token(TokenType::Then) {
                return Err(self.parse_error("Expected THEN in WHEN clause"));
            }

            let then: Expression = if self.match_token(TokenType::Insert) {
                self.parse_merge_when_insert()?
            } else if self.match_token(TokenType::Update) {
                self.parse_merge_when_update()?
            } else if self.match_token(TokenType::Delete) {
                let mut elements = vec![Expression::Var(Box::new(Var {
                    this: "DELETE".to_string(),
                }))];

                if self.match_token(TokenType::Where) {
                    elements.push(Expression::Where(Box::new(
                        sqlquack_syntax::expressions::queries::models::Where {
                            this: self.parse_expression()?,
                        },
                    )));
                }

                if elements.len() == 1 {
                    elements[0].clone()
                } else {
                    Expression::Tuple(Box::new(Tuple {
                        expressions: elements,
                    }))
                }
            } else if self.match_identifier("DO") {
                if self.match_identifier("NOTHING") {
                    Expression::Var(Box::new(Var {
                        this: "DO NOTHING".to_string(),
                    }))
                } else {
                    return Err(self.parse_error("Expected NOTHING after DO"));
                }
            } else {
                if let Some(var) = self.parse_var()? {
                    var
                } else {
                    return Err(
                        self.parse_error("Expected INSERT, UPDATE, DELETE, or action keyword")
                    );
                }
            };

            whens.push(Expression::When(Box::new(When {
                matched: Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: matched,
                }))),
                source,
                condition,
                then: Box::new(then),
            })));
        }

        if whens.is_empty() && self.is_snowflake() {
            Err(self.parse_error("Expected a WHEN clause in MERGE"))
        } else if whens.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Expression::Whens(Box::new(Whens {
                expressions: whens,
            }))))
        }
    }

    /// In a Snowflake parse, whether a MERGE action stops here, before its required clause.
    fn merge_action_ends(&self) -> bool {
        self.config.dialect == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
            && (self.is_at_end() || self.check(TokenType::Semicolon) || self.check(TokenType::When))
    }

    fn parse_merge_when_insert(&mut self) -> Result<Expression> {
        Ok({
            let mut elements = vec![Expression::Var(Box::new(Var {
                this: "INSERT".to_string(),
            }))];

            let by_name = self.is_snowflake() && self.match_text_seq(&["ALL", "BY", "NAME"]);
            if by_name {
                elements.push(Expression::Var(Box::new(Var {
                    this: "ALL BY NAME".to_string(),
                })));
            } else if self.match_token(TokenType::Star) {
                elements.push(Expression::Star(Box::new(
                    sqlquack_syntax::expressions::names::models::Star {
                        table: None,
                        ilike: None,
                        except: None,
                        replace: None,
                        rename: None,
                        trailing_comments: Vec::new(),
                        span: SourceSpan::default(),
                        qualified: None,
                    },
                )));
            } else if self.match_token(TokenType::LParen) {
                let mut columns: Vec<Expression> = Vec::new();
                loop {
                    if self.check(TokenType::String)
                        && self.config.dialect
                            == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
                    {
                        return Err(self.parse_error("Expected a column name, not a string"));
                    }
                    if let Some(col) = self.parse_id_var()? {
                        let col = if self.match_token(TokenType::Dot) {
                            if let Expression::Identifier(table_ident) = col {
                                if let Some(col_expr) = self.parse_id_var()? {
                                    if let Expression::Identifier(col_ident) = col_expr {
                                        Self::parsed_column(Column {
                                            name: col_ident,
                                            table: Some(table_ident),
                                            join_mark: false,
                                            trailing_comments: Vec::new(),
                                            span: None,
                                            inferred_type: None,
                                        })
                                    } else {
                                        col_expr
                                    }
                                } else {
                                    return Err(self.parse_error(
                                        "Expected column name after dot in MERGE INSERT",
                                    ));
                                }
                            } else {
                                col
                            }
                        } else {
                            col
                        };
                        columns.push(col);
                    } else {
                        break;
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                let closed = self.match_token(TokenType::RParen);
                if (columns.is_empty() || !closed)
                    && self.config.dialect
                        == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
                {
                    return Err(self.parse_error("Expected a closed column list in MERGE INSERT"));
                }
                if !columns.is_empty() {
                    elements.push(Expression::Tuple(Box::new(Tuple {
                        expressions: columns,
                    })));
                }
            }

            if self.match_text_seq(&["VALUES"]) {
                if let Some(values) = self.parse_value()? {
                    let empty = matches!(&values, Expression::Tuple(t) if t.expressions.is_empty());
                    if (empty || self.previous_text() != RIGHT_PAREN)
                        && self.config.dialect
                            == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
                    {
                        return Err(self.parse_error("Expected a closed, non-empty VALUES list"));
                    }
                    elements.push(values);
                } else if self.config.dialect
                    == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
                {
                    return Err(self.parse_error("Expected a value list after VALUES"));
                }
            } else if self.merge_action_ends() && !by_name {
                return Err(self.parse_error("Expected VALUES in MERGE INSERT"));
            } else if self.match_text_seq(&["ROW"]) {
                elements.push(Expression::Var(Box::new(Var {
                    this: "ROW".to_string(),
                })));
            }

            if self.match_token(TokenType::Where) {
                elements.push(Expression::Where(Box::new(
                    sqlquack_syntax::expressions::queries::models::Where {
                        this: self.parse_expression()?,
                    },
                )));
            }

            if elements.len() == 1 {
                elements[0].clone()
            } else {
                Expression::Tuple(Box::new(Tuple {
                    expressions: elements,
                }))
            }
        })
    }

    fn parse_merge_when_update(&mut self) -> Result<Expression> {
        Ok({
            let mut elements = vec![Expression::Var(Box::new(Var {
                this: "UPDATE".to_string(),
            }))];

            if self.is_snowflake() && self.match_text_seq(&["ALL", "BY", "NAME"]) {
                elements.push(Expression::Var(Box::new(Var {
                    this: "ALL BY NAME".to_string(),
                })));
            } else if self.match_token(TokenType::Star) {
                elements.push(Expression::Star(Box::new(
                    sqlquack_syntax::expressions::names::models::Star {
                        table: None,
                        ilike: None,
                        except: None,
                        replace: None,
                        rename: None,
                        trailing_comments: Vec::new(),
                        span: SourceSpan::default(),
                        qualified: None,
                    },
                )));
            } else if self.merge_action_ends() {
                return Err(self.parse_error("Expected SET in MERGE UPDATE"));
            } else if self.match_token(TokenType::Set) {
                if self.match_token(TokenType::Star) {
                    elements.push(Expression::Star(Box::new(
                        sqlquack_syntax::expressions::names::models::Star {
                            table: None,
                            ilike: None,
                            except: None,
                            replace: None,
                            rename: None,
                            trailing_comments: Vec::new(),
                            span: SourceSpan::default(),
                            qualified: None,
                        },
                    )));
                } else {
                    let mut assignments: Vec<Expression> = Vec::new();
                    loop {
                        if self.check(TokenType::String)
                            && self.config.dialect
                                == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
                        {
                            return Err(self.parse_error("Expected a column name, not a string"));
                        }
                        if let Some(col) = self.parse_id_var()? {
                            let col = if self.match_token(TokenType::Dot) {
                                if let Expression::Identifier(table_ident) = col {
                                    if let Some(col_expr) = self.parse_id_var()? {
                                        if let Expression::Identifier(col_ident) = col_expr {
                                            Self::parsed_column(Column {
                                                name: col_ident,
                                                table: Some(table_ident),
                                                join_mark: false,
                                                trailing_comments: Vec::new(),
                                                span: None,
                                                inferred_type: None,
                                            })
                                        } else {
                                            col_expr
                                        }
                                    } else {
                                        return Err(
                                            self.parse_error("Expected column name after dot")
                                        );
                                    }
                                } else {
                                    col
                                }
                            } else {
                                col
                            };
                            if self.match_token(TokenType::Eq) {
                                let value = self.parse_expression()?;
                                let assignment = Expression::Eq(Box::new(BinaryOp {
                                    left: col,
                                    right: value,
                                    left_comments: Vec::new(),
                                    operator_comments: Vec::new(),
                                    trailing_comments: Vec::new(),
                                    inferred_type: None,
                                    span: SourceSpan::default(),
                                }));
                                assignments.push(assignment);
                            }
                        }
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    if assignments.is_empty()
                        && self.config.dialect
                            == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
                    {
                        return Err(self.parse_error("Expected an assignment after SET"));
                    }
                    if !assignments.is_empty() {
                        elements.push(Expression::Tuple(Box::new(Tuple {
                            expressions: assignments,
                        })));
                    }
                }
            }

            if self.match_token(TokenType::Where) {
                elements.push(Expression::Where(Box::new(
                    sqlquack_syntax::expressions::queries::models::Where {
                        this: self.parse_expression()?,
                    },
                )));
            }

            if elements.len() == 1 {
                elements[0].clone()
            } else {
                Expression::Tuple(Box::new(Tuple {
                    expressions: elements,
                }))
            }
        })
    }

    /// Parses NEXT VALUE FOR sequence.
    pub(crate) fn parse_next_value_for(&mut self) -> Result<Option<Expression>> {
        if !self.match_text_seq(&["VALUE", "FOR"]) {
            if self.current > 0 {
                self.current -= 1;
            }
            return Ok(None);
        }

        let first = self
            .parse_id_var()?
            .ok_or_else(|| self.parse_error("Expected sequence name after NEXT VALUE FOR"))?;
        let first_id = match first {
            Expression::Identifier(id) => id,
            Expression::Var(v) => Identifier {
                name: v.this,
                quoted: false,
                trailing_comments: Vec::new(),
                span: None,
            },
            _ => Identifier {
                name: String::new(),
                quoted: false,
                trailing_comments: Vec::new(),
                span: None,
            },
        };

        let mut parts = vec![first_id];
        while self.match_token(TokenType::Dot) {
            if self.is_identifier_or_keyword_token() {
                let token = self.advance()?;
                parts.push(Identifier {
                    name: token.text,
                    quoted: token.token_type == TokenType::QuotedIdentifier,
                    trailing_comments: Vec::new(),
                    span: Some(token.span),
                });
            } else {
                break;
            }
        }

        let this = if parts.len() == 1 {
            Self::parsed_column(Column {
                name: parts.remove(0),
                table: None,
                join_mark: false,
                trailing_comments: Vec::new(),
                span: None,
                inferred_type: None,
            })
        } else if parts.len() == QUALIFIED_NAME_PARTS {
            Self::parsed_column(Column {
                name: parts.remove(1),
                table: Some(parts.remove(0)),
                join_mark: false,
                trailing_comments: Vec::new(),
                span: None,
                inferred_type: None,
            })
        } else {
            let mut expr = Expression::Identifier(parts.remove(0));
            for part in parts.drain(..) {
                expr = Expression::Dot(Box::new(DotAccess {
                    this: expr,
                    field: part,
                    inferred_type: None,
                }));
            }
            expr
        };

        let order = if self.match_token(TokenType::Over) {
            if self.match_token(TokenType::LParen) {
                let ord = self.parse_order()?;
                self.expect_token(TokenType::RParen)?;
                ord.map(Box::new)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Some(Expression::NextValueFor(Box::new(NextValueFor {
            this: Box::new(this),
            order,
        }))))
    }

    /// Parses the NULL literal.
    pub(crate) fn parse_null(&mut self) -> Result<Option<Expression>> {
        if self.match_token(TokenType::Null) {
            return Ok(Some(Expression::Null(Null)));
        }
        if self.match_token(TokenType::Unknown) {
            return Ok(Some(Expression::Null(Null)));
        }
        Ok(None)
    }

    /// Parses a numeric literal, Hive/Spark suffixes encoded as `number::TYPE` included.
    pub(crate) fn parse_number(&mut self) -> Result<Option<Expression>> {
        if self.match_token(TokenType::Number) {
            let text = self.previous_text().to_string();
            if let Some(sep_pos) = text.find("::") {
                let num_part = &text[..sep_pos];
                let type_name = &text[sep_pos + 2..];
                let num_expr = Expression::Literal(Box::new(Literal::Number(num_part.to_string())));
                let data_type = match type_name {
                    "BIGINT" => sqlquack_syntax::expressions::types::models::DataType::BigInt {
                        length: None,
                    },
                    "SMALLINT" => sqlquack_syntax::expressions::types::models::DataType::SmallInt {
                        length: None,
                    },
                    "TINYINT" => sqlquack_syntax::expressions::types::models::DataType::TinyInt {
                        length: None,
                    },
                    "DOUBLE" => sqlquack_syntax::expressions::types::models::DataType::Double {
                        precision: None,
                        scale: None,
                    },
                    "FLOAT" => sqlquack_syntax::expressions::types::models::DataType::Float {
                        precision: None,
                        scale: None,
                        real_spelling: false,
                    },
                    "DECIMAL" => sqlquack_syntax::expressions::types::models::DataType::Decimal {
                        precision: None,
                        scale: None,
                    },
                    _ => sqlquack_syntax::expressions::types::models::DataType::Custom {
                        name: type_name.to_string(),
                    },
                };
                return Ok(Some(Expression::TryCast(Box::new(
                    sqlquack_syntax::expressions::operators::models::Cast {
                        span: SourceSpan::default(),
                        this: num_expr,
                        to: data_type,
                        trailing_comments: Vec::new(),
                        double_colon_syntax: false,
                        typed_literal: false,
                        format: None,
                        default: None,
                        inferred_type: None,
                    },
                ))));
            }
            return Ok(Some(Expression::Literal(Box::new(Literal::Number(text)))));
        }
        Ok(None)
    }

    /// Parses SQL Server OPENJSON(json, path) WITH (...).
    pub(crate) fn parse_open_json(&mut self) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;

        let path = if self.match_token(TokenType::Comma) {
            self.parse_string()?.map(Box::new)
        } else {
            None
        };

        let expressions = if self.match_token(TokenType::RParen)
            && self.match_token(TokenType::With)
        {
            self.expect_token(TokenType::LParen)?;
            let mut cols: Vec<Expression> = Vec::new();
            loop {
                let col_name = self.parse_field()?;
                if col_name.is_none() {
                    break;
                }
                let col_type = self.parse_data_type()?;
                let col_path = self.parse_string()?.map(Box::new);
                let as_json = if self.match_token(TokenType::As) && self.match_identifier("JSON") {
                    Some(Box::new(Expression::Boolean(BooleanLiteral {
                        value: true,
                    })))
                } else {
                    None
                };
                cols.push(Expression::OpenJSONColumnDef(Box::new(OpenJSONColumnDef {
                    this: Box::new(col_name.ok_or_else(|| {
                        self.parse_error("Expected column name in OPENJSON WITH clause")
                    })?),
                    kind: String::new(),
                    path: col_path,
                    as_json,
                    data_type: Some(col_type),
                })));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            cols
        } else {
            Vec::new()
        };

        Ok(Some(Expression::OpenJSON(Box::new(OpenJSON {
            this: Box::new(this),
            path,
            expressions,
        }))))
    }

    /// Parses an ORDER BY clause.
    pub(crate) fn parse_order(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::Order) {
            return Ok(None);
        }
        self.match_token(TokenType::By);

        let mut expressions: Vec<Ordered> = Vec::new();
        loop {
            if let Some(ordered) = self.parse_ordered_item()? {
                expressions.push(ordered);
            } else {
                break;
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        let (expressions, all) = self.take_order_by_all(expressions);
        Ok(Some(Expression::OrderBy(Box::new(OrderBy {
            expressions,
            siblings: false,
            comments: Vec::new(),
            all,
        }))))
    }

    /// parse_ordered_item - Parse a single ORDER BY item (expr [ASC|DESC] [NULLS FIRST|LAST])
    pub(crate) fn parse_ordered_item(&mut self) -> Result<Option<Ordered>> {
        let expr = match self.parse_expression() {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };

        let mut desc = false;
        let mut explicit_asc = false;
        if self.match_token(TokenType::Asc) {
            explicit_asc = true;
        } else if self.match_token(TokenType::Desc) {
            desc = true;
        }

        let expr = self.maybe_parse_clickhouse_order_collate(expr)?;

        let nulls_first = if self.match_text_seq(&["NULLS", "FIRST"]) {
            Some(true)
        } else if self.match_text_seq(&["NULLS", "LAST"]) {
            Some(false)
        } else {
            None
        };

        let with_fill = if self.match_text_seq(&["WITH", "FILL"]) {
            let from_ = if self.match_token(TokenType::From) {
                Some(Box::new(self.parse_or()?))
            } else {
                None
            };
            let to = if self.match_text_seq(&["TO"]) {
                Some(Box::new(self.parse_or()?))
            } else {
                None
            };
            let step = if self.match_text_seq(&["STEP"]) {
                Some(Box::new(self.parse_or()?))
            } else {
                None
            };
            let staleness = if self.match_text_seq(&["STALENESS"]) {
                Some(Box::new(self.parse_or()?))
            } else {
                None
            };
            let interpolate = if self.match_text_seq(&["INTERPOLATE"]) {
                if self.match_token(TokenType::LParen) {
                    let exprs = self.parse_expression_list()?;
                    self.expect_token(TokenType::RParen)?;
                    if exprs.len() == 1 {
                        Some(Box::new(exprs.into_iter().next().ok_or_else(|| {
                            sqlquack_syntax::tokens::errors::Error::internal(
                                "a value the parser checked is missing",
                            )
                        })?))
                    } else {
                        Some(Box::new(Expression::Tuple(Box::new(
                            sqlquack_syntax::expressions::types::models::Tuple {
                                expressions: exprs,
                            },
                        ))))
                    }
                } else {
                    None
                }
            } else {
                None
            };
            Some(Box::new(WithFill {
                from_,
                to,
                step,
                staleness,
                interpolate,
            }))
        } else {
            None
        };

        Ok(Some(Ordered {
            nulls_auto: false,
            this: expr,
            desc,
            nulls_first,
            explicit_asc,
            with_fill,
        }))
    }

    /// Parses a PARTITION/SUBPARTITION clause.
    pub(crate) fn parse_partition(&mut self) -> Result<Option<Expression>> {
        if !self.match_texts(&["PARTITION", "SUBPARTITION"]) {
            return Ok(None);
        }

        let subpartition = self.previous_text().eq_ignore_ascii_case("SUBPARTITION");

        if !self.match_token(TokenType::LParen) {
            return Ok(Some(Expression::Partition(Box::new(Partition {
                expressions: Vec::new(),
                subpartition,
            }))));
        }

        let mut expressions: Vec<Expression> = Vec::new();
        loop {
            if let Some(expr) = self.parse_disjunction()? {
                expressions.push(expr);
            } else {
                break;
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        self.match_token(TokenType::RParen);

        Ok(Some(Expression::Partition(Box::new(Partition {
            expressions,
            subpartition,
        }))))
    }

    /// Parse wrapped comma-separated expressions: (expr, expr, ...)
    pub(crate) fn parse_wrapped_csv_expressions(&mut self) -> Result<Vec<Expression>> {
        self.expect_token(TokenType::LParen)?;
        let mut exprs: Vec<Expression> = Vec::new();
        if !self.check(TokenType::RParen) {
            loop {
                if self.match_token(TokenType::Maxvalue) {
                    exprs.push(Expression::Var(Box::new(Var {
                        this: "MAXVALUE".to_string(),
                    })));
                } else {
                    exprs.push(self.parse_expression()?);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(exprs)
    }

    /// Parses a placeholder, `?` or `:name`.
    pub(crate) fn parse_placeholder(&mut self) -> Result<Option<Expression>> {
        if self.match_token(TokenType::Placeholder) {
            return Ok(Some(Expression::Placeholder(Placeholder { index: None })));
        }
        if self.match_token(TokenType::Parameter) {
            let text = self.previous_text().to_string();
            return Ok(Some(Expression::Parameter(Box::new(Parameter {
                name: Some(text),
                index: None,
                style: ParameterStyle::Colon,
                quoted: false,
                string_quoted: false,
                expression: None,
                span: SourceSpan::default(),
            }))));
        }
        Ok(None)
    }

    /// Parse ClickHouse query parameter syntax: {name: Type}
    pub(crate) fn parse_clickhouse_braced_parameter(&mut self) -> Result<Option<Expression>> {
        Ok(None)
    }

    /// Parses POSITION(substr IN str) or POSITION(needle, haystack, start).
    pub(crate) fn parse_position(&mut self) -> Result<Option<Expression>> {
        let mut args: Vec<Expression> = Vec::new();

        match self.parse_bitwise() {
            Ok(Some(expr)) => {
                let expr = self.maybe_clickhouse_alias(expr);
                let expr = self.try_clickhouse_func_arg_alias(expr);
                args.push(expr);
            }
            Ok(None) => return Ok(None),
            Err(e) => return Err(e),
        }

        if self.match_token(TokenType::In) {
            match self.parse_bitwise() {
                Ok(Some(haystack)) => {
                    let haystack = self.maybe_clickhouse_alias(haystack);
                    let haystack = self.try_clickhouse_func_arg_alias(haystack);
                    return Ok(Some(Expression::StrPosition(Box::new(StrPosition {
                        this: Box::new(haystack),
                        substr: Some(Box::new(args.remove(0))),
                        position: None,
                        occurrence: None,
                    }))));
                }
                Ok(None) => {
                    return Err(self.parse_error("Expected expression after IN in POSITION"))
                }
                Err(e) => return Err(e),
            }
        }

        while self.match_token(TokenType::Comma) {
            match self.parse_bitwise() {
                Ok(Some(expr)) => {
                    let expr = self.maybe_clickhouse_alias(expr);
                    let expr = self.try_clickhouse_func_arg_alias(expr);
                    args.push(expr);
                }
                Ok(None)
                    if self.config.dialect
                        == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake) =>
                {
                    return Err(self.parse_error("Expected an argument after the comma"));
                }
                Ok(None) => break,
                Err(e) => return Err(e),
            }
        }

        let position = args.get(2).cloned();
        let (haystack, needle) = { (args.get(1).cloned(), args.first().cloned()) };

        Ok(Some(Expression::StrPosition(Box::new(StrPosition {
            this: Box::new(
                haystack.unwrap_or_else(|| {
                    Expression::Literal(Box::new(Literal::String("".to_string())))
                }),
            ),
            substr: needle.map(Box::new),
            position: position.map(Box::new),
            occurrence: None,
        }))))
    }

    /// Parses PRIMARY KEY, as a column or table constraint.
    pub(crate) fn parse_primary_key(&mut self) -> Result<Option<Expression>> {
        self.parse_primary_key_impl(false, false)
    }

    /// Implementation of parse_primary_key with options
    pub(crate) fn parse_primary_key_impl(
        &mut self,
        wrapped_optional: bool,
        in_props: bool,
    ) -> Result<Option<Expression>> {
        let desc = if self.match_token(TokenType::Asc) {
            false
        } else {
            self.match_token(TokenType::Desc)
        };

        let this = if (self.check(TokenType::Identifier) || self.check(TokenType::Var))
            && self.check_next(TokenType::LParen)
        {
            self.parse_id_var()?
        } else {
            None
        };

        if !in_props && !self.check(TokenType::LParen) {
            let options = self.parse_key_constraint_options_list()?;
            return Ok(Some(Expression::PrimaryKeyColumnConstraint(Box::new(
                PrimaryKeyColumnConstraint {
                    desc: if desc {
                        Some(Box::new(Expression::Boolean(BooleanLiteral {
                            value: true,
                        })))
                    } else {
                        None
                    },
                    options,
                },
            ))));
        }

        let expressions = if self.match_token(TokenType::LParen) {
            let mut exprs: Vec<Expression> = Vec::new();
            loop {
                if let Some(part) = self.parse_primary_key_part()? {
                    exprs.push(part);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            exprs
        } else if wrapped_optional {
            Vec::new()
        } else {
            return Err(self.parse_error("Expected '(' for PRIMARY KEY column list"));
        };

        let include = self.parse_index_params()?;

        let options = self.parse_key_constraint_options_list()?;

        Ok(Some(Expression::PrimaryKey(Box::new(PrimaryKey {
            this: this.map(Box::new),
            expressions,
            options,
            include: include.map(Box::new),
        }))))
    }

    /// Parse key constraint options as a list of expressions
    pub(crate) fn parse_key_constraint_options_list(&mut self) -> Result<Vec<Expression>> {
        let mut options: Vec<Expression> = Vec::new();

        loop {
            if self.is_at_end() {
                break;
            }

            if self.match_token(TokenType::On) {
                let on_what = if !self.is_at_end() {
                    let token = self.advance()?;
                    token.text.to_string()
                } else {
                    break;
                };

                let action = if self.match_text_seq(&["NO", "ACTION"]) {
                    "NO ACTION"
                } else if self.match_text_seq(&["CASCADE"]) {
                    "CASCADE"
                } else if self.match_text_seq(&["RESTRICT"]) {
                    "RESTRICT"
                } else if self.match_token(TokenType::Set) && self.match_token(TokenType::Null) {
                    "SET NULL"
                } else if self.match_token(TokenType::Set) && self.match_token(TokenType::Default) {
                    "SET DEFAULT"
                } else {
                    break;
                };

                options.push(Expression::Var(Box::new(Var {
                    this: format!("ON {} {}", on_what, action),
                })));
            } else if self.match_text_seq(&["NOT", "ENFORCED"]) {
                options.push(Expression::Var(Box::new(Var {
                    this: "NOT ENFORCED".to_string(),
                })));
            } else if self.match_text_seq(&["DEFERRABLE"]) {
                options.push(Expression::Var(Box::new(Var {
                    this: "DEFERRABLE".to_string(),
                })));
            } else if self.match_text_seq(&["INITIALLY", "DEFERRED"]) {
                options.push(Expression::Var(Box::new(Var {
                    this: "INITIALLY DEFERRED".to_string(),
                })));
            } else if self.match_text_seq(&["NORELY"]) {
                options.push(Expression::Var(Box::new(Var {
                    this: "NORELY".to_string(),
                })));
            } else if self.match_text_seq(&["RELY"]) {
                options.push(Expression::Var(Box::new(Var {
                    this: "RELY".to_string(),
                })));
            } else {
                break;
            }
        }

        Ok(options)
    }

    /// parse_primary_key_part - Delegates to parse_field
    pub(crate) fn parse_primary_key_part(&mut self) -> Result<Option<Expression>> {
        if (self.is_identifier_token() || self.is_safe_keyword_as_identifier())
            && self.check_next(TokenType::LParen)
        {
            return self.parse_expression().map(Some);
        }
        if let Some(field) = self.parse_field()? {
            Ok(Some(field))
        } else {
            self.parse_expression().map(Some)
        }
    }

    /// Parses a primary expression, else a variable from any token.
    pub(crate) fn parse_primary_or_var(&mut self) -> Result<Option<Expression>> {
        let saved_pos = self.current;
        match self.parse_primary() {
            Ok(expr) => return Ok(Some(expr)),
            Err(_) => {
                self.current = saved_pos;
            }
        }

        self.parse_var()
    }

    /// Parses a property.
    pub(crate) fn parse_property(&mut self) -> Result<Option<Expression>> {
        if self.match_text_seq(&["COMPOUND", "SORTKEY"]) {
            return Ok(Some(Expression::Identifier(Identifier {
                name: String::new(),
                quoted: false,
                trailing_comments: Vec::new(),
                span: None,
            })));
        }
        if self.match_text_seq(&["SQL", "SECURITY"]) {
            return Ok(None);
        }
        if self.match_texts(&["DEFINER", "INVOKER"]) {
            return Ok(None);
        }
        Ok(None)
    }

    /// parse_on_cluster_clause - Parse ClickHouse ON CLUSTER clause
    pub(crate) fn parse_on_cluster_clause(&mut self) -> Result<Option<OnCluster>> {
        Ok(None)
    }

    /// Parses a ClickHouse implicit argument alias `expr name` before a delimiter.
    pub(crate) fn try_clickhouse_implicit_alias(&mut self, expr: Expression) -> Expression {
        expr
    }

    pub(crate) fn try_parse_date_part_unit_expr(&self, expr: &Expression) -> Option<IntervalUnit> {
        let upper = self.date_part_expr_name(expr)?.to_ascii_uppercase();
        let canonical = match upper.as_str() {
            "Y" | "YY" | "YYY" | "YYYY" | "YR" | "YEARS" | "YRS" => "YEAR",
            "Q" | "QTR" | "QTRS" | "QUARTERS" | "QQ" => "QUARTER",
            "MM" | "MON" | "MONS" | "MONTHS" | "M" => "MONTH",
            "W" | "WK" | "WEEKOFYEAR" | "WOY" | "WY" | "WW" | "WEEKS" => "WEEK",
            "D" | "DD" | "DAYS" | "DAYOFMONTH" => "DAY",
            "H" | "HH" | "HR" | "HOURS" | "HRS" => "HOUR",
            "MI" | "MIN" | "MINUTES" | "MINS" | "N" => "MINUTE",
            "S" | "SEC" | "SECONDS" | "SECS" | "SS" => "SECOND",
            "MS" | "MSEC" | "MSECS" | "MSECOND" | "MSECONDS" | "MILLISEC" | "MILLISECS"
            | "MILLISECON" | "MILLISECONDS" => "MILLISECOND",
            "US" | "USEC" | "USECS" | "MICROSEC" | "MICROSECS" | "USECOND" | "USECONDS"
            | "MICROSECONDS" | "MCS" => "MICROSECOND",
            "NS" | "NSEC" | "NANOSEC" | "NSECOND" | "NSECONDS" | "NANOSECS" => "NANOSECOND",
            _ => upper.as_str(),
        };

        parse_interval_unit_from_string(canonical)
    }

    pub(crate) fn try_parse_date_part_unit_identifier_expr(
        &self,
        expr: &Expression,
    ) -> Option<IntervalUnit> {
        let upper = self
            .date_part_identifier_expr_name(expr)?
            .to_ascii_uppercase();
        let canonical = match upper.as_str() {
            "Y" | "YY" | "YYY" | "YYYY" | "YR" | "YEARS" | "YRS" => "YEAR",
            "Q" | "QTR" | "QTRS" | "QUARTERS" | "QQ" => "QUARTER",
            "MM" | "MON" | "MONS" | "MONTHS" | "M" => "MONTH",
            "W" | "WK" | "WEEKOFYEAR" | "WOY" | "WY" | "WW" | "WEEKS" => "WEEK",
            "D" | "DD" | "DAYS" | "DAYOFMONTH" => "DAY",
            "H" | "HH" | "HR" | "HOURS" | "HRS" => "HOUR",
            "MI" | "MIN" | "MINUTES" | "MINS" | "N" => "MINUTE",
            "S" | "SEC" | "SECONDS" | "SECS" | "SS" => "SECOND",
            "MS" | "MSEC" | "MSECS" | "MSECOND" | "MSECONDS" | "MILLISEC" | "MILLISECS"
            | "MILLISECON" | "MILLISECONDS" => "MILLISECOND",
            "US" | "USEC" | "USECS" | "MICROSEC" | "MICROSECS" | "USECOND" | "USECONDS"
            | "MICROSECONDS" | "MCS" => "MICROSECOND",
            "NS" | "NSEC" | "NANOSEC" | "NSECOND" | "NSECONDS" | "NANOSECS" => "NANOSECOND",
            _ => upper.as_str(),
        };

        parse_interval_unit_from_string(canonical)
    }

    pub(crate) fn try_parse_date_part_field_identifier_expr(
        &self,
        expr: &Expression,
    ) -> Option<DateTimeField> {
        let upper = self
            .date_part_identifier_expr_name(expr)?
            .to_ascii_uppercase();
        Some(match upper.as_str() {
            "YEAR" | "Y" | "YY" | "YYY" | "YYYY" | "YR" | "YEARS" | "YRS" => DateTimeField::Year,
            "MONTH" | "MM" | "MON" | "MONS" | "MONTHS" => DateTimeField::Month,
            "DAY" | "D" | "DD" | "DAYS" | "DAYOFMONTH" => DateTimeField::Day,
            "HOUR" | "H" | "HH" | "HR" | "HOURS" | "HRS" => DateTimeField::Hour,
            "MINUTE" | "MI" | "MIN" | "MINUTES" | "MINS" => DateTimeField::Minute,
            "SECOND" | "S" | "SEC" | "SECONDS" | "SECS" => DateTimeField::Second,
            "MILLISECOND" | "MS" | "MSEC" | "MILLISECONDS" => DateTimeField::Millisecond,
            "MICROSECOND" | "US" | "USEC" | "MICROSECONDS" => DateTimeField::Microsecond,
            "DOW" | "DAYOFWEEK" | "DW" => DateTimeField::DayOfWeek,
            "DOY" | "DAYOFYEAR" | "DY" => DateTimeField::DayOfYear,
            "WEEK" | "W" | "WK" | "WEEKOFYEAR" | "WOY" | "WW" => DateTimeField::Week,
            "QUARTER" | "Q" | "QTR" | "QTRS" | "QUARTERS" => DateTimeField::Quarter,
            "EPOCH" | "EPOCH_SECOND" | "EPOCH_SECONDS" => DateTimeField::Epoch,
            "TIMEZONE" => DateTimeField::Timezone,
            "TIMEZONE_HOUR" | "TZH" => DateTimeField::TimezoneHour,
            "TIMEZONE_MINUTE" | "TZM" => DateTimeField::TimezoneMinute,
            "DATE" => DateTimeField::Date,
            "TIME" => DateTimeField::Time,
            other => DateTimeField::Custom(other.to_string()),
        })
    }

    pub(crate) fn convert_date_part_identifier_expr_to_var(&self, expr: Expression) -> Expression {
        match expr {
            Expression::Var(_) => expr,
            Expression::Column(c) if c.table.is_none() => {
                Expression::Var(Box::new(Var { this: c.name.name }))
            }
            Expression::Identifier(id) => Expression::Var(Box::new(Var { this: id.name })),
            _ => expr,
        }
    }

    /// Reads date-part keywords (DAY, WEEK(MONDAY), ...) in date-part functions as Vars.
    pub(crate) fn normalize_date_part_arg(
        &self,
        name: &str,
        mut args: Vec<Expression>,
    ) -> Vec<Expression> {
        use sqlquack_syntax::tokens::models::DialectType as DT;
        let dialect = match self.config.dialect {
            Some(d) => d,
            None => return args,
        };
        let upper = name.to_ascii_uppercase();
        let unit_index: Option<usize> = match dialect {
            DT::Snowflake => match upper.as_str() {
                "DATE_TRUNC" | "DATETRUNC" if !args.is_empty() => Some(0),
                "DATEDIFF" | "TIMEDIFF" | "TIMESTAMPDIFF" if args.len() == TERNARY_ARITY => Some(0),
                "EXTRACT" if args.len() == BINARY_ARITY => Some(0),
                _ => None,
            },
            _ => None,
        };
        if let Some(idx) = unit_index {
            let taken = std::mem::replace(&mut args[idx], Expression::Null(Null));
            args[idx] = Self::date_part_arg_to_var(taken);
        }
        args
    }

    pub(crate) fn date_part_arg_to_var(expr: Expression) -> Expression {
        match expr {
            Expression::Column(c) if c.table.is_none() => {
                Expression::Var(Box::new(Var { this: c.name.name }))
            }
            Expression::Identifier(id) => Expression::Var(Box::new(Var { this: id.name })),
            Expression::Function(mut f) if !f.args.is_empty() => {
                let inner = std::mem::replace(&mut f.args[0], Expression::Null(Null));
                f.args[0] = Self::date_part_arg_to_var(inner);
                Expression::Function(f)
            }
            other => other,
        }
    }

    pub(crate) fn date_part_identifier_expr_name<'a>(
        &self,
        expr: &'a Expression,
    ) -> Option<&'a str> {
        match expr {
            Expression::Var(v) => Some(v.this.as_str()),
            Expression::Column(c) if c.table.is_none() => Some(c.name.name.as_str()),
            Expression::Identifier(id) => Some(id.name.as_str()),
            _ => None,
        }
    }
}
