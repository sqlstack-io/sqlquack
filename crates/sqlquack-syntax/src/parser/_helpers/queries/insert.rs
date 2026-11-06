//! Parser queries: INSERT statements.

use crate::expressions::functions::models::Function;
use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Column;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::TableRef;
use crate::expressions::operators::models::BinaryOp;
use crate::expressions::operators::models::OnConflict;
use crate::expressions::queries::models::RowFormat;
use crate::expressions::statements::models::DirectoryInsert;
use crate::expressions::statements::models::Insert;
use crate::expressions::tree::models::Expression;
use crate::parser::models::InsertPrefix;
use crate::parser::models::Parser;
use crate::parser::types::InsertPartition;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parse INSERT statement
    pub(crate) fn parse_insert(&mut self) -> Result<Expression> {
        let insert_token = self.expect_token(TokenType::Insert)?;
        let leading_comments = insert_token.comments;

        let hint = if self.check(TokenType::Hint) {
            Some(self.parse_hint()?)
        } else {
            None
        };

        let conflict_action = self.parse_insert_conflict_action()?;

        let ignore = conflict_action.is_none() && self.match_token(TokenType::Ignore);

        let overwrite = self.match_token(TokenType::Overwrite);

        if self.match_token(TokenType::All) || self.match_token(TokenType::First) {
            if let Some(multi_insert) =
                self.parse_multitable_inserts(leading_comments.clone(), overwrite)?
            {
                return Ok(multi_insert);
            }
        }

        let local_directory = overwrite && self.match_token(TokenType::Local);
        let is_directory = (overwrite || local_directory) && self.match_identifier("DIRECTORY");

        if is_directory {
            return self.parse_insert_directory(
                InsertPrefix {
                    leading_comments,
                    hint,
                    conflict_action,
                    ignore,
                    overwrite,
                },
                local_directory,
            );
        }

        if overwrite {
            self.match_token(TokenType::Into);
            self.match_token(TokenType::Table);
        } else {
            self.expect_token(TokenType::Into)?;
            self.match_token(TokenType::Table);
        }

        let function_target = self.parse_insert_function_target()?;

        let table_name = if function_target.is_some() {
            Identifier::new(String::new())
        } else {
            self.expect_identifier_or_keyword_with_quoted()?
        };
        let table = self.parse_insert_table(table_name)?;

        let (alias, alias_explicit_as) = self.parse_insert_alias()?;

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);

        let replace_where =
            if self.match_token(TokenType::Replace) && self.match_token(TokenType::Where) {
                Some(Box::new(self.parse_or()?))
            } else {
                None
            };

        let (partition_by_expr, partition) = self.parse_insert_partition()?;

        let insert_settings = if self.match_token(TokenType::Settings) {
            let mut settings: Vec<Expression> = Vec::new();
            loop {
                settings.push(self.parse_expression()?);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            settings
        } else {
            Vec::new()
        };

        let mut columns = if self.check(TokenType::LParen) {
            if self
                .peek_nth(1)
                .map(|t| t.token_type == TokenType::Select || t.token_type == TokenType::With)
                .unwrap_or(false)
            {
                Vec::new()
            } else {
                self.skip();
                let cols = self.parse_identifier_list()?;
                self.expect_token(TokenType::RParen)?;
                cols
            }
        } else {
            Vec::new()
        };

        let insert_set_values;
        (insert_set_values, columns) = self.parse_insert_set_values(columns)?;

        let output = if self.match_token(TokenType::Output) {
            Some(self.parse_output_clause()?)
        } else {
            None
        };

        let by_name = self.match_token(TokenType::By) && self.match_identifier("NAME");

        let default_values =
            self.match_token(TokenType::Default) && self.match_token(TokenType::Values);

        let (values, query) =
            self.parse_insert_values_or_query(default_values, insert_set_values)?;

        let source = self.parse_insert_source()?;

        let source_alias = if self.match_token(TokenType::As) {
            Some(self.expect_identifier_or_keyword_with_quoted()?)
        } else {
            None
        };

        let on_conflict = self.parse_insert_on_conflict()?;

        let returning = if self.match_token(TokenType::Returning) {
            self.parse_select_expressions()?
        } else {
            Vec::new()
        };

        Ok(Expression::Insert(Box::new(Insert {
            column_targets: Vec::new(),
            table,
            columns,
            values,
            query,
            overwrite,
            partition,
            directory: None,
            returning,
            output,
            on_conflict,
            leading_comments,
            if_exists,
            with: None,
            ignore,
            source_alias,
            alias,
            alias_explicit_as,
            default_values,
            by_name,
            conflict_action,
            is_replace: false,
            replace_where,
            source: source.map(Box::new),
            hint,
            function_target,
            partition_by: partition_by_expr,
            settings: insert_settings,
            overriding: None,
            returning_aliases: Vec::new(),
        })))
    }

    pub(crate) fn parse_insert_alias(&mut self) -> Result<(Option<Identifier>, bool)> {
        Ok(if self.match_token(TokenType::As) {
            (
                Some(self.parse_unquoted_identifier(Self::expect_identifier)?),
                true,
            )
        } else if self.is_identifier_token()
            && !self.check(TokenType::Values)
            && !self.check(TokenType::Select)
            && !self.check(TokenType::Default)
            && !self.check(TokenType::By)
            && !self.check(TokenType::Partition)
            && !self.check(TokenType::Output)
            && !self.check(TokenType::If)
            && !self.check(TokenType::Replace)
            && !self.check(TokenType::Table)
            && !self.check(TokenType::LParen)
        {
            (
                Some(self.parse_unquoted_identifier(Self::expect_identifier)?),
                false,
            )
        } else {
            (None, false)
        })
    }

    pub(crate) fn parse_insert_set_values(
        &mut self,
        mut columns: Vec<Identifier>,
    ) -> Result<(Option<Vec<Expression>>, Vec<Identifier>)> {
        let insert_set_values = if self.match_token(TokenType::Set) {
            let mut values: Vec<Expression> = Vec::new();
            loop {
                let column = self.expect_identifier_with_quoted()?;
                self.expect_token(TokenType::Eq)?;
                let value = if self.match_token(TokenType::Default) {
                    Expression::Command(Box::new(crate::expressions::statements::models::Command {
                        this: "DEFAULT".to_string(),
                    }))
                } else {
                    self.parse_disjunction()?.ok_or_else(|| {
                        self.parse_error("Expected column assignment in INSERT ... SET")
                    })?
                };
                columns.push(column);
                values.push(value);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            Some(values)
        } else {
            None
        };
        Ok((insert_set_values, columns))
    }

    pub(crate) fn parse_insert_conflict_action(&mut self) -> Result<Option<String>> {
        Ok(if self.match_token(TokenType::Or) {
            if self.match_identifier("ABORT") {
                Some("ABORT".to_string())
            } else if self.match_identifier("FAIL") {
                Some("FAIL".to_string())
            } else if self.match_token(TokenType::Ignore) {
                Some("IGNORE".to_string())
            } else if self.match_token(TokenType::Replace) {
                Some("REPLACE".to_string())
            } else if self.match_token(TokenType::Rollback) {
                Some("ROLLBACK".to_string())
            } else {
                return Err(self.parse_error(
                    "Expected ABORT, FAIL, IGNORE, REPLACE, or ROLLBACK after INSERT OR",
                ));
            }
        } else {
            None
        })
    }

    pub(crate) fn parse_insert_directory(
        &mut self,
        prefix: InsertPrefix,
        local_directory: bool,
    ) -> Result<Expression> {
        let InsertPrefix {
            leading_comments,
            hint,
            conflict_action,
            ignore,
            overwrite,
        } = prefix;

        let path = self.expect_string()?;
        let row_format = if self.match_keywords(&[TokenType::Row, TokenType::Format]) {
            let delimited = self.match_identifier("DELIMITED");
            let mut fields_terminated_by = None;
            let mut collection_items_terminated_by = None;
            let mut map_keys_terminated_by = None;
            let mut lines_terminated_by = None;
            let mut null_defined_as = None;

            loop {
                if self.match_identifier("FIELDS") || self.match_identifier("FIELD") {
                    self.match_identifier("TERMINATED");
                    self.match_token(TokenType::By);
                    fields_terminated_by = Some(self.expect_string()?);
                } else if self.match_identifier("COLLECTION") {
                    self.match_identifier("ITEMS");
                    self.match_identifier("TERMINATED");
                    self.match_token(TokenType::By);
                    collection_items_terminated_by = Some(self.expect_string()?);
                } else if self.match_identifier("MAP") {
                    self.match_identifier("KEYS");
                    self.match_identifier("TERMINATED");
                    self.match_token(TokenType::By);
                    map_keys_terminated_by = Some(self.expect_string()?);
                } else if self.match_identifier("LINES") {
                    self.match_identifier("TERMINATED");
                    self.match_token(TokenType::By);
                    lines_terminated_by = Some(self.expect_string()?);
                } else if self.match_token(TokenType::Null) {
                    self.match_identifier("DEFINED");
                    self.match_token(TokenType::As);
                    null_defined_as = Some(self.expect_string()?);
                } else {
                    break;
                }
            }

            Some(RowFormat {
                delimited,
                fields_terminated_by,
                collection_items_terminated_by,
                map_keys_terminated_by,
                lines_terminated_by,
                null_defined_as,
            })
        } else {
            None
        };

        let stored_as = if self.match_identifier("STORED") {
            self.expect_token(TokenType::As)?;
            Some(self.expect_identifier()?)
        } else {
            None
        };

        let query = self.parse_statement()?;

        Ok(Expression::Insert(Box::new(Insert {
            column_targets: Vec::new(),
            table: TableRef::new(""),
            columns: Vec::new(),
            values: Vec::new(),
            query: Some(query),
            overwrite,
            partition: Vec::new(),
            directory: Some(DirectoryInsert {
                local: local_directory,
                path,
                row_format,
                stored_as,
            }),
            returning: Vec::new(),
            output: None,
            on_conflict: None,
            leading_comments,
            if_exists: false,
            with: None,
            ignore,
            source_alias: None,
            alias: None,
            alias_explicit_as: false,
            default_values: false,
            by_name: false,
            conflict_action: conflict_action.clone(),
            is_replace: false,
            replace_where: None,
            source: None,
            hint: hint.clone(),
            function_target: None,
            partition_by: None,
            settings: Vec::new(),
            overriding: None,
            returning_aliases: Vec::new(),
        })))
    }

    pub(crate) fn parse_insert_function_target(&mut self) -> Result<Option<Box<Expression>>> {
        let mut function_target: Option<Box<Expression>> = None;
        if self.match_token(TokenType::Function) {
            let func_name = self.expect_identifier_or_keyword()?;
            self.expect_token(TokenType::LParen)?;
            let args = if self.check(TokenType::RParen) {
                Vec::new()
            } else {
                self.parse_expression_list()?
            };
            self.expect_token(TokenType::RParen)?;
            function_target = Some(Box::new(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: func_name,
                args,
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: SourceSpan::default(),
                inferred_type: None,
            }))));
        }
        Ok(function_target)
    }

    pub(crate) fn parse_insert_table(&mut self, table_name: Identifier) -> Result<TableRef> {
        Ok(if self.match_token(TokenType::Dot) {
            let mut schema = table_name;
            let mut name = self.expect_identifier_or_keyword_with_quoted()?;
            let catalog = if self.match_token(TokenType::Dot) {
                let catalog = schema;
                schema = name;
                name = self.expect_identifier_or_keyword_with_quoted()?;
                Some(catalog)
            } else {
                None
            };
            let trailing_comments = self.previous_trailing_comments().to_vec();
            TableRef {
                sample_call: None,
                name,
                schema: Some(schema),
                catalog,
                alias: None,
                alias_explicit_as: false,
                column_aliases: Vec::new(),
                leading_comments: Vec::new(),
                trailing_comments,
                when: None,
                only: false,
                final_: false,
                table_sample: None,
                hints: Vec::new(),
                system_time: None,
                partitions: Vec::new(),
                identifier_func: None,
                changes: None,
                version: None,
                span: None,
            }
        } else {
            let trailing_comments = self.previous_trailing_comments().to_vec();
            TableRef {
                sample_call: None,
                name: table_name,
                schema: None,
                catalog: None,
                alias: None,
                alias_explicit_as: false,
                column_aliases: Vec::new(),
                leading_comments: Vec::new(),
                when: None,
                only: false,
                final_: false,
                table_sample: None,
                hints: Vec::new(),
                system_time: None,
                trailing_comments,
                partitions: Vec::new(),
                identifier_func: None,
                changes: None,
                version: None,
                span: None,
            }
        })
    }

    pub(crate) fn parse_insert_partition(&mut self) -> Result<InsertPartition> {
        let mut partition_by_expr: Option<Box<Expression>> = None;
        let partition = if self.check(TokenType::Partition) && self.check_next(TokenType::By) {
            self.skip();
            self.skip();
            partition_by_expr = Some(Box::new(self.parse_expression()?));
            Vec::new()
        } else if self.match_token(TokenType::Partition) {
            self.expect_token(TokenType::LParen)?;
            let mut parts: Vec<(Identifier, Option<Expression>)> = Vec::new();
            loop {
                let col = self.parse_unquoted_identifier(Self::expect_identifier)?;
                let value = if self.match_token(TokenType::Eq) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                parts.push((col, value));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            parts
        } else {
            Vec::new()
        };
        Ok((partition_by_expr, partition))
    }

    pub(crate) fn parse_insert_values_or_query(
        &mut self,
        default_values: bool,
        insert_set_values: Option<Vec<Expression>>,
    ) -> Result<(Vec<Vec<Expression>>, Option<Expression>)> {
        Ok(if let Some(values) = insert_set_values {
            (vec![values], None)
        } else if default_values {
            (Vec::new(), None)
        } else {
            {
                if self.match_token(TokenType::Values) {
                    let mut all_values: Vec<Vec<Expression>> = Vec::new();

                    loop {
                        self.expect_token(TokenType::LParen)?;
                        let row = if self.check(TokenType::RParen)
                            && self.config.dialect
                                == Some(crate::tokens::models::DialectType::Snowflake)
                        {
                            return Err(self.parse_error("Expected a value in the VALUES row"));
                        } else if self.check(TokenType::RParen) {
                            Vec::new()
                        } else {
                            self.parse_values_expression_list()?
                        };
                        self.expect_token(TokenType::RParen)?;
                        all_values.push(row);

                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }

                    (all_values, None)
                } else if self.check(TokenType::Table) {
                    (Vec::new(), None)
                } else {
                    if self.is_snowflake() && self.check(TokenType::LParen) {
                        let mut offset = 1;
                        while self
                            .peek_nth(offset)
                            .is_some_and(|token| token.token_type == TokenType::LParen)
                        {
                            offset += 1;
                        }
                        if !self.peek_nth(offset).is_some_and(|token| {
                            matches!(
                                token.token_type,
                                TokenType::Select | TokenType::With | TokenType::Values
                            )
                        }) {
                            return Err(self.parse_error("Expected a query in parentheses"));
                        }
                    }
                    (Vec::new(), Some(self.parse_statement()?))
                }
            }
        })
    }

    pub(crate) fn parse_insert_source(&mut self) -> Result<Option<Expression>> {
        Ok(if self.match_token(TokenType::Table) {
            let source_name = self.expect_identifier_with_quoted()?;
            let source_table = if self.match_token(TokenType::Dot) {
                let schema = source_name;
                let name = self.expect_identifier_with_quoted()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                TableRef {
                    sample_call: None,
                    name,
                    schema: Some(schema),
                    catalog: None,
                    alias: None,
                    alias_explicit_as: false,
                    column_aliases: Vec::new(),
                    leading_comments: Vec::new(),
                    trailing_comments,
                    when: None,
                    only: false,
                    final_: false,
                    table_sample: None,
                    hints: Vec::new(),
                    system_time: None,
                    partitions: Vec::new(),
                    identifier_func: None,
                    changes: None,
                    version: None,
                    span: None,
                }
            } else {
                let trailing_comments = self.previous_trailing_comments().to_vec();
                TableRef {
                    sample_call: None,
                    name: source_name,
                    schema: None,
                    catalog: None,
                    alias: None,
                    alias_explicit_as: false,
                    column_aliases: Vec::new(),
                    leading_comments: Vec::new(),
                    trailing_comments,
                    when: None,
                    only: false,
                    final_: false,
                    table_sample: None,
                    hints: Vec::new(),
                    system_time: None,
                    partitions: Vec::new(),
                    identifier_func: None,
                    changes: None,
                    version: None,
                    span: None,
                }
            };
            Some(Expression::Table(Box::new(source_table)))
        } else {
            None
        })
    }

    pub(crate) fn parse_insert_on_conflict(&mut self) -> Result<Option<Box<Expression>>> {
        Ok(if self.match_token(TokenType::On) {
            if self.match_identifier("CONFLICT") {
                Some(Box::new(self.parse_on_conflict()?))
            } else if self.match_identifier("DUPLICATE") {
                self.expect_token(TokenType::Key)?;
                self.expect_token(TokenType::Update)?;

                let mut sets: Vec<Expression> = Vec::new();
                loop {
                    let col_name = self.expect_identifier_with_quoted()?;
                    let column = if self.match_token(TokenType::Dot) {
                        let col = self.expect_identifier_with_quoted()?;
                        Self::parsed_column(Column {
                            name: col,
                            table: Some(col_name),
                            join_mark: false,
                            trailing_comments: Vec::new(),
                            span: None,
                            inferred_type: None,
                        })
                    } else {
                        Expression::Identifier(col_name)
                    };
                    self.expect_token(TokenType::Eq)?;
                    let value = self.parse_expression()?;
                    sets.push(Expression::Eq(Box::new(BinaryOp {
                        left: column,
                        right: value,
                        left_comments: Vec::new(),
                        operator_comments: Vec::new(),
                        trailing_comments: Vec::new(),
                        inferred_type: None,
                        span: SourceSpan::default(),
                    })));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }

                Some(Box::new(Expression::OnConflict(Box::new(OnConflict {
                    duplicate: Some(Box::new(Expression::Boolean(BooleanLiteral {
                        value: true,
                    }))),
                    expressions: sets,
                    action: None,
                    conflict_keys: None,
                    index_predicate: None,
                    constraint: None,
                    where_: None,
                }))))
            } else {
                return Err(self.parse_error("Expected CONFLICT or DUPLICATE after ON"));
            }
        } else {
            None
        })
    }
}
