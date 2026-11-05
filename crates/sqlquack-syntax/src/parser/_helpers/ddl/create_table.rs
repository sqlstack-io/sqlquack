//! Parser ddl: create table.

use crate::expressions::columns::models::ColumnConstraint;
use crate::expressions::columns::models::ColumnDef;
use crate::expressions::columns::models::TableConstraint;
use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Column;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::names::models::TableRef;
use crate::expressions::statements::main::snowflake_table_option_to_raw::snowflake_table_option_to_raw;
use crate::expressions::statements::models::Raw;
use crate::expressions::statements::models::Var;
use crate::expressions::tables::models::BackupProperty;
use crate::expressions::tables::models::CollateProperty;
use crate::expressions::tables::models::ConstraintType;
use crate::expressions::tables::models::CreateTable;
use crate::expressions::tables::models::DistKeyProperty;
use crate::expressions::tables::models::DistStyleProperty;
use crate::expressions::tables::models::OnCommit;
use crate::expressions::tables::models::OnProperty;
use crate::expressions::tables::models::PartitionBoundSpec;
use crate::expressions::tables::models::Properties;
use crate::expressions::tables::models::RollupIndex;
use crate::expressions::tables::models::RollupProperty;
use crate::expressions::tables::models::SchemaCommentProperty;
use crate::expressions::tables::models::SortKeyProperty;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::Tuple;
use crate::parser::constants::COMPOUND;
use crate::parser::models::CreateTableCloneKind;
use crate::parser::models::CreateTableHead;
use crate::parser::models::CreateTableLeadingOptions;
use crate::parser::models::CreateTableNameSuffix;
use crate::parser::models::Parser;
use crate::parser::types::CreateTableInlineProperties;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parse CREATE TABLE
    pub(crate) fn parse_create_table(
        &mut self,
        or_replace: bool,
        temporary: bool,
        leading_comments: Vec<String>,
        table_modifier: Option<&str>,
    ) -> Result<Expression> {
        if table_modifier == Some("DICTIONARY") {
            let _ = self.match_token(TokenType::Dictionary);
        } else {
            self.expect_token(TokenType::Table)?;
        }

        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);

        let is_special_modifier = matches!(
            table_modifier,
            Some(
                "DYNAMIC"
                    | "ICEBERG"
                    | "EXTERNAL"
                    | "HYBRID"
                    | "UNLOGGED"
                    | "DICTIONARY"
                    | "MATERIALIZED"
            )
        );

        let name = match self.is_snowflake() {
            true => self.parse_object_name()?,
            false => self.parse_table_ref()?,
        };

        let uuid = { None };

        let on_cluster = self.parse_on_cluster_clause()?;

        let teradata_post_name_options = { Vec::new() };

        if self.match_keywords(&[TokenType::Partition, TokenType::Of]) {
            return self.parse_create_table_partition_of(CreateTableHead {
                name,
                if_not_exists,
                temporary,
                or_replace,
                table_modifier,
                leading_comments,
            });
        }

        let shallow_clone = self.check_identifier("SHALLOW");
        let deep_clone = self.check_identifier("DEEP");
        if shallow_clone || deep_clone {
            self.skip();
        }
        let is_copy = self.check(TokenType::Copy) && !self.check_next_identifier("GRANTS");
        if self.check_identifier("CLONE") || is_copy {
            return self.parse_create_table_clone(
                CreateTableHead {
                    name,
                    if_not_exists,
                    temporary,
                    or_replace,
                    table_modifier,
                    leading_comments,
                },
                CreateTableNameSuffix {
                    on_cluster,
                    uuid,
                    teradata_post_name_options,
                },
                CreateTableCloneKind {
                    shallow_clone,
                    deep_clone,
                    is_copy,
                },
            );
        }

        self.parse_create_table_after_name(
            CreateTableHead {
                name,
                if_not_exists,
                temporary,
                or_replace,
                table_modifier,
                leading_comments,
            },
            CreateTableNameSuffix {
                on_cluster,
                uuid,
                teradata_post_name_options,
            },
            is_special_modifier,
        )
    }

    fn parse_create_table_after_name(
        &mut self,
        head: CreateTableHead<'_>,
        suffix: CreateTableNameSuffix,
        is_special_modifier: bool,
    ) -> Result<Expression> {
        let CreateTableHead {
            name,
            if_not_exists,
            temporary,
            or_replace,
            table_modifier,
            leading_comments,
        } = head;
        let CreateTableNameSuffix {
            on_cluster,
            uuid,
            teradata_post_name_options,
        } = suffix;
        let mut with_properties = if self.match_token(TokenType::With) {
            self.parse_with_properties()?
        } else {
            Vec::new()
        };

        let copy_grants = self.match_text_seq(&["COPY", "GRANTS"]);

        let using_template = self.parse_using_template()?;

        if using_template.is_some() {
            return self.parse_create_table_using_template(
                CreateTableHead {
                    name,
                    if_not_exists,
                    temporary,
                    or_replace,
                    table_modifier,
                    leading_comments,
                },
                CreateTableNameSuffix {
                    on_cluster,
                    uuid,
                    teradata_post_name_options,
                },
                CreateTableLeadingOptions {
                    copy_grants,
                    with_properties,
                },
                using_template,
            );
        }

        let mut snowflake_inline_properties;
        (snowflake_inline_properties, with_properties) =
            self.parse_create_table_inline_properties(is_special_modifier, with_properties)?;

        let redshift_ctas_properties = self.parse_create_table_redshift_properties()?;

        if self.match_token(TokenType::As) {
            return self.parse_create_table_as_select(
                CreateTableHead {
                    name,
                    if_not_exists,
                    temporary,
                    or_replace,
                    table_modifier,
                    leading_comments,
                },
                CreateTableNameSuffix {
                    on_cluster,
                    uuid,
                    teradata_post_name_options,
                },
                CreateTableLeadingOptions {
                    copy_grants,
                    with_properties,
                },
                redshift_ctas_properties,
            );
        }

        let snowflake_hybrid = self.check_snowflake_hybrid_body(table_modifier)?;
        if !self.check(TokenType::LParen) && is_special_modifier && !snowflake_hybrid {
            return self.parse_create_table_special_without_columns(
                CreateTableHead {
                    name,
                    if_not_exists,
                    temporary,
                    or_replace,
                    table_modifier,
                    leading_comments,
                },
                CreateTableNameSuffix {
                    on_cluster,
                    uuid,
                    teradata_post_name_options,
                },
                CreateTableLeadingOptions {
                    copy_grants,
                    with_properties,
                },
            );
        }

        if self.check(TokenType::Like) {
            return self.parse_create_table_like(
                CreateTableHead {
                    name,
                    if_not_exists,
                    temporary,
                    or_replace,
                    table_modifier,
                    leading_comments,
                },
                CreateTableNameSuffix {
                    on_cluster,
                    uuid,
                    teradata_post_name_options,
                },
                CreateTableLeadingOptions {
                    copy_grants,
                    with_properties,
                },
            );
        }

        if self.match_keyword("TAG")
            || self.match_token(TokenType::With) && self.match_keyword("TAG")
        {
            return self.parse_create_table_tags_only(
                CreateTableHead {
                    name,
                    if_not_exists,
                    temporary,
                    or_replace,
                    table_modifier,
                    leading_comments,
                },
                CreateTableNameSuffix {
                    on_cluster,
                    uuid,
                    teradata_post_name_options,
                },
                CreateTableLeadingOptions {
                    copy_grants,
                    with_properties,
                },
            );
        }

        if self.check_identifier("TBLPROPERTIES")
            || self.check_identifier("LOCATION")
            || self.check_identifier("STORED")
            || self.check(TokenType::Row)
            || self.check(TokenType::Using)
            || self.check_identifier("CLUSTERED")
            || self.check_identifier("PARTITIONED")
            || self.check_identifier("COMMENT")
        {
            return self.parse_create_table_tblproperties_only(
                CreateTableHead {
                    name,
                    if_not_exists,
                    temporary,
                    or_replace,
                    table_modifier,
                    leading_comments,
                },
                CreateTableNameSuffix {
                    on_cluster,
                    uuid,
                    teradata_post_name_options,
                },
                CreateTableLeadingOptions {
                    copy_grants,
                    with_properties,
                },
            );
        }

        if self.check(TokenType::LParen) {
            let saved = self.current;
            self.skip();
            let is_ctas = self.check(TokenType::Select) || self.check(TokenType::With);
            self.current = saved;
            if is_ctas {
                return self.parse_create_table_parenthesized_as_select(
                    CreateTableHead {
                        name,
                        if_not_exists,
                        temporary,
                        or_replace,
                        table_modifier,
                        leading_comments,
                    },
                    CreateTableNameSuffix {
                        on_cluster,
                        uuid,
                        teradata_post_name_options,
                    },
                    CreateTableLeadingOptions {
                        copy_grants,
                        with_properties,
                    },
                );
            }
        }

        let no_column_defs = !self.check(TokenType::LParen)
            && (self.check(TokenType::Partition)
                || self.check(TokenType::PartitionBy)
                || self.check(TokenType::Cluster)
                || self.check_identifier("OPTIONS")
                || self.check(TokenType::As));

        if !no_column_defs {
            self.expect_token(TokenType::LParen)?;
        }

        let (columns, constraints) =
            self.parse_create_table_columns(&name, no_column_defs, table_modifier)?;

        if !no_column_defs {
            self.expect_token(TokenType::RParen)?;
        }

        let pre_with_comment = self.parse_create_table_pre_with_comment()?;

        let with_properties_after = self.parse_create_table_with_properties_after()?;

        let mut all_with_properties = with_properties;
        all_with_properties.extend(with_properties_after);

        all_with_properties =
            self.parse_create_table_special_options(all_with_properties, is_special_modifier)?;

        if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        ) {
            snowflake_inline_properties.extend(
                self.parse_snowflake_inline_table_options()?
                    .into_iter()
                    .map(snowflake_table_option_to_raw),
            );
        }

        let (mysql_table_options, tidb_table_options) = { self.parse_mysql_table_options()? };

        let rollup = self.parse_create_table_rollup(&name)?;

        let hive_properties = self.parse_hive_table_properties()?;

        let (on_commit, on_property) = self.parse_create_table_on_commit()?;

        let mut table_properties = self.parse_create_table_trailing_properties(
            hive_properties,
            pre_with_comment,
            snowflake_inline_properties,
        )?;

        let as_select = self.parse_create_table_column_list_query(
            no_column_defs,
            &columns,
            table_modifier == Some("EXTERNAL"),
        )?;

        table_properties = self.parse_create_table_partitioning(table_properties)?;

        table_properties = self.parse_create_table_cluster_by(&name, table_properties)?;

        if no_column_defs {
            {
                if self.match_identifier("OPTIONS") {
                    let options = self.parse_options_list()?;
                    table_properties.push(Expression::Properties(Box::new(Properties {
                        expressions: options,
                    })));
                }
            }
        }

        let as_select = if no_column_defs && self.match_token(TokenType::As) {
            Some(self.parse_statement()?)
        } else {
            as_select
        };

        all_with_properties = self.parse_create_table_special_trailing_options(
            all_with_properties,
            is_special_modifier,
        )?;

        table_properties = self.parse_create_table_stage_file_formats(table_properties)?;

        let post_table_properties = self.parse_post_table_properties()?;

        let inherits = self.parse_create_table_inherits()?;

        Ok(Expression::CreateTable(Box::new(CreateTable {
            name,
            on_cluster,
            columns,
            constraints,
            if_not_exists,
            temporary,
            or_replace,
            table_modifier: table_modifier.map(|s| s.to_string()),
            source_dialect: None,
            as_select,
            as_select_parenthesized: false,
            on_commit,
            clone_source: None,
            clone_at_clause: None,
            shallow_clone: false,
            deep_clone: false,
            is_copy: false,
            leading_comments,
            with_properties: all_with_properties,
            teradata_post_name_options: teradata_post_name_options.clone(),
            with_data: None,
            with_statistics: None,
            teradata_indexes: Vec::new(),
            with_cte: None,
            properties: table_properties,
            partition_of: None,
            post_table_properties,
            mysql_table_options,
            tidb_table_options,
            inherits,
            on_property,
            copy_grants,
            using_template: None,
            rollup,
            uuid,
            with_partition_columns: Vec::new(),
            with_connection: None,
        })))
    }

    fn parse_create_table_inherits(&mut self) -> Result<Vec<TableRef>> {
        Ok(if self.match_identifier("INHERITS") {
            self.expect_token(TokenType::LParen)?;
            let mut parents: Vec<TableRef> = Vec::new();
            loop {
                parents.push(self.parse_table_ref()?);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            parents
        } else {
            Vec::new()
        })
    }

    fn parse_create_table_trailing_properties(
        &mut self,
        hive_properties: Vec<Expression>,
        pre_with_comment: Option<String>,
        snowflake_inline_properties: Vec<Expression>,
    ) -> Result<Vec<Expression>> {
        let mut table_properties = hive_properties;
        table_properties.extend(snowflake_inline_properties);

        if let Some(comment_text) = pre_with_comment {
            table_properties.push(Expression::SchemaCommentProperty(Box::new(
                SchemaCommentProperty {
                    this: Box::new(Expression::Literal(Box::new(Literal::String(comment_text)))),
                },
            )));
        }

        if self.match_token(TokenType::Default) && self.match_token(TokenType::Collate) {
            let collation = self.parse_primary()?;
            table_properties.push(Expression::CollateProperty(Box::new(CollateProperty {
                this: Box::new(collation),
                default: Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: true,
                }))),
            })));
        }

        if self.match_identifier("OPTIONS") {
            let options = self.parse_options_list()?;
            table_properties.push(Expression::Properties(Box::new(Properties {
                expressions: options,
            })));
        }

        loop {
            if self.match_identifier("DISTKEY") {
                if let Some(distkey) = self.parse_distkey()? {
                    table_properties.push(distkey);
                }
            } else if self.match_text_seq(&["COMPOUND", "SORTKEY"]) {
                if let Some(Expression::SortKeyProperty(mut skp)) = self.parse_sortkey()? {
                    skp.compound = Some(Box::new(Expression::Boolean(BooleanLiteral {
                        value: true,
                    })));
                    table_properties.push(Expression::SortKeyProperty(skp));
                }
            } else if self.match_identifier("SORTKEY") {
                if let Some(sortkey) = self.parse_sortkey()? {
                    table_properties.push(sortkey);
                }
            } else if self.match_identifier("DISTSTYLE") {
                if self.match_texts(&["ALL", "EVEN", "AUTO", "KEY"]) {
                    let style = self.previous_text().to_ascii_uppercase();
                    table_properties.push(Expression::DistStyleProperty(Box::new(
                        DistStyleProperty {
                            this: Box::new(Expression::Var(Box::new(Var { this: style }))),
                        },
                    )));
                }
            } else if self.match_identifier("BACKUP") {
                if self.match_texts(&["YES", "NO"]) {
                    let value = self.previous_text().to_ascii_uppercase();
                    table_properties.push(Expression::BackupProperty(Box::new(BackupProperty {
                        this: Box::new(Expression::Var(Box::new(Var { this: value }))),
                    })));
                }
            } else {
                break;
            }
        }
        Ok(table_properties)
    }

    /// The `AS <query>` after a column list; Snowflake requires one when a non-external table's column has no type.
    fn parse_create_table_column_list_query(
        &mut self,
        no_column_defs: bool,
        columns: &[ColumnDef],
        external: bool,
    ) -> Result<Option<Expression>> {
        let as_select = if !no_column_defs && self.match_token(TokenType::As) {
            Some(self.parse_statement()?)
        } else {
            None
        };
        if self.is_snowflake()
            && !external
            && as_select.is_none()
            && columns.iter().any(|c| c.no_type)
        {
            return Err(self.parse_error("Expected a data type for the column"));
        }
        Ok(as_select)
    }

    fn parse_create_table_columns(
        &mut self,
        _name: &TableRef,
        no_column_defs: bool,
        table_modifier: Option<&str>,
    ) -> Result<(Vec<ColumnDef>, Vec<TableConstraint>)> {
        Ok(if no_column_defs {
            (Vec::new(), Vec::new())
        } else if table_modifier == Some("DYNAMIC") {
            let saved = self.current;
            let is_name_only_list =
                if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                    self.skip();
                    let result = self.check(TokenType::Comma) || self.check(TokenType::RParen);
                    self.current = saved;
                    result
                } else {
                    false
                };

            if is_name_only_list {
                let mut cols: Vec<ColumnDef> = Vec::new();
                loop {
                    let name = self.expect_identifier_or_safe_keyword_with_quoted()?;
                    let mut col_def = ColumnDef::new(
                        name.name.clone(),
                        DataType::Custom {
                            name: String::new(),
                        },
                    );
                    col_def.name = name;
                    cols.push(col_def);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                (cols, Vec::new())
            } else {
                self.parse_column_definitions()?
            }
        } else {
            self.parse_column_definitions()?
        })
    }

    fn parse_create_table_pre_with_comment(&mut self) -> Result<Option<String>> {
        Ok(if self.check(TokenType::Comment) {
            let saved = self.current;
            self.skip();
            if self.check(TokenType::String) {
                let comment_text = self.advance_text()?;
                Some(comment_text)
            } else {
                self.current = saved;
                None
            }
        } else {
            None
        })
    }

    fn parse_create_table_with_properties_after(&mut self) -> Result<Vec<(String, String)>> {
        Ok(if self.check(TokenType::With) {
            let saved = self.current;
            self.skip();
            let is_system_versioning = if self.check(TokenType::LParen) {
                let saved2 = self.current;
                self.skip();
                let result = self.check_identifier("SYSTEM_VERSIONING");
                self.current = saved2;
                result
            } else {
                false
            };
            let is_row_access_policy = self.check_text_seq(&["ROW", "ACCESS", "POLICY"]);
            if is_system_versioning || is_row_access_policy {
                self.current = saved;
                Vec::new()
            } else {
                self.parse_with_properties()?
            }
        } else {
            Vec::new()
        })
    }

    fn parse_create_table_rollup(&mut self, _name: &TableRef) -> Result<Option<RollupProperty>> {
        Ok(if self.match_token(TokenType::Rollup) {
            self.expect_token(TokenType::LParen)?;
            let mut indices: Vec<RollupIndex> = Vec::new();
            loop {
                let name = self.expect_identifier_or_keyword_with_quoted()?;
                let cols = if self.match_token(TokenType::LParen) {
                    let mut col_list: Vec<Identifier> = Vec::new();
                    loop {
                        col_list.push(self.expect_identifier_or_keyword_with_quoted()?);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    self.expect_token(TokenType::RParen)?;
                    col_list
                } else {
                    Vec::new()
                };
                indices.push(crate::expressions::tables::models::RollupIndex {
                    name,
                    expressions: cols,
                });
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            Some(crate::expressions::tables::models::RollupProperty {
                expressions: indices,
            })
        } else {
            None
        })
    }

    fn parse_create_table_on_commit(&mut self) -> Result<(Option<OnCommit>, Option<OnProperty>)> {
        Ok({
            if self.match_token(TokenType::On) {
                if self.match_token(TokenType::Commit) {
                    if self.match_keywords(&[TokenType::Preserve, TokenType::Rows]) {
                        (Some(OnCommit::PreserveRows), None)
                    } else if self.match_keywords(&[TokenType::Delete, TokenType::Rows]) {
                        (Some(OnCommit::DeleteRows), None)
                    } else {
                        return Err(self
                            .parse_error("Expected PRESERVE ROWS or DELETE ROWS after ON COMMIT"));
                    }
                } else {
                    let filegroup = self.parse_schema_identifier()?;
                    (
                        None,
                        Some(OnProperty {
                            this: Box::new(filegroup),
                        }),
                    )
                }
            } else {
                (None, None)
            }
        })
    }

    fn parse_create_table_inline_properties(
        &mut self,
        is_special_modifier: bool,
        mut with_properties: Vec<(String, String)>,
    ) -> Result<CreateTableInlineProperties> {
        let mut snowflake_inline_properties: Vec<Expression> = Vec::new();
        if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        ) {
            let pre_column_options = self.parse_snowflake_inline_table_options()?;
            if is_special_modifier {
                with_properties.extend(pre_column_options);
            } else {
                snowflake_inline_properties.extend(
                    pre_column_options
                        .into_iter()
                        .map(snowflake_table_option_to_raw),
                );
            }
        }
        Ok((snowflake_inline_properties, with_properties))
    }

    fn parse_create_table_redshift_properties(&mut self) -> Result<Vec<Expression>> {
        let mut redshift_ctas_properties: Vec<Expression> = Vec::new();
        loop {
            if self.match_identifier("DISTKEY") {
                if self.match_token(TokenType::LParen) {
                    let col = self.expect_identifier()?;
                    self.expect_token(TokenType::RParen)?;
                    redshift_ctas_properties.push(Expression::DistKeyProperty(Box::new(
                        DistKeyProperty {
                            this: Box::new(Self::parsed_column(Column {
                                name: Identifier::new(col),
                                table: None,
                                join_mark: false,
                                trailing_comments: Vec::new(),
                                span: None,
                                inferred_type: None,
                            })),
                        },
                    )));
                }
            } else if self.check_identifier("COMPOUND") || self.check_identifier("INTERLEAVED") {
                let modifier = self.advance_text()?.to_ascii_uppercase();
                if self.match_identifier("SORTKEY") && self.match_token(TokenType::LParen) {
                    let mut cols: Vec<Expression> = Vec::new();
                    loop {
                        let col = self.expect_identifier()?;
                        cols.push(Self::parsed_column(Column {
                            name: Identifier::new(col),
                            table: None,
                            join_mark: false,
                            trailing_comments: Vec::new(),
                            span: None,
                            inferred_type: None,
                        }));
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    self.expect_token(TokenType::RParen)?;
                    let compound_value = if modifier == COMPOUND {
                        Some(Box::new(Expression::Boolean(BooleanLiteral {
                            value: true,
                        })))
                    } else {
                        None
                    };
                    redshift_ctas_properties.push(Expression::SortKeyProperty(Box::new(
                        SortKeyProperty {
                            this: Box::new(Expression::Tuple(Box::new(Tuple {
                                expressions: cols,
                            }))),
                            compound: compound_value,
                        },
                    )));
                }
            } else if self.match_identifier("SORTKEY") {
                if self.match_token(TokenType::LParen) {
                    let mut cols: Vec<Expression> = Vec::new();
                    loop {
                        let col = self.expect_identifier()?;
                        cols.push(Self::parsed_column(Column {
                            name: Identifier::new(col),
                            table: None,
                            join_mark: false,
                            trailing_comments: Vec::new(),
                            span: None,
                            inferred_type: None,
                        }));
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    self.expect_token(TokenType::RParen)?;
                    redshift_ctas_properties.push(Expression::SortKeyProperty(Box::new(
                        SortKeyProperty {
                            this: Box::new(Expression::Tuple(Box::new(Tuple {
                                expressions: cols,
                            }))),
                            compound: None,
                        },
                    )));
                }
            } else if self.match_identifier("DISTSTYLE") {
                if self.match_texts(&["ALL", "EVEN", "AUTO", "KEY"]) {
                    let style = self.previous_text().to_ascii_uppercase();
                    redshift_ctas_properties.push(Expression::DistStyleProperty(Box::new(
                        DistStyleProperty {
                            this: Box::new(Expression::Var(Box::new(Var { this: style }))),
                        },
                    )));
                }
            } else if self.match_identifier("BACKUP") {
                if self.match_texts(&["YES", "NO"]) {
                    let value = self.previous_text().to_ascii_uppercase();
                    redshift_ctas_properties.push(Expression::BackupProperty(Box::new(
                        BackupProperty {
                            this: Box::new(Expression::Var(Box::new(Var { this: value }))),
                        },
                    )));
                }
            } else {
                break;
            }
        }
        Ok(redshift_ctas_properties)
    }

    fn parse_create_table_partitioning(
        &mut self,
        mut table_properties: Vec<Expression>,
    ) -> Result<Vec<Expression>> {
        if self.check(TokenType::Partition) || self.check(TokenType::PartitionBy) {
            let parsed_bigquery_partition = { false };

            if !parsed_bigquery_partition {
                let saved = self.current;
                let is_partition_by = if self.match_token(TokenType::PartitionBy) {
                    true
                } else if self.match_token(TokenType::Partition) {
                    self.match_token(TokenType::By)
                } else {
                    false
                };
                if is_partition_by {
                    let partition_kind = if self.check(TokenType::Range) {
                        self.skip();
                        Some("RANGE".to_string())
                    } else if self.check(TokenType::List) {
                        self.skip();
                        Some("LIST".to_string())
                    } else if (self.check(TokenType::Identifier) || self.check(TokenType::Var))
                        && self.check_next(TokenType::LParen)
                    {
                        Some(self.advance_text()?.to_ascii_uppercase())
                    } else {
                        None
                    };

                    {
                        {
                            let no_partition_kind = partition_kind.is_none();
                            let mut raw_sql = match partition_kind {
                                Some(kind) => format!("PARTITION BY {}", kind),
                                None => "PARTITION BY ".to_string(),
                            };
                            if self.check(TokenType::LParen) {
                                self.skip();
                                raw_sql.push('(');
                                let mut depth = 1;
                                let mut last_tok_type: Option<TokenType> = None;
                                while !self.is_at_end() && depth > 0 {
                                    let tok = self.advance()?;
                                    if tok.token_type == TokenType::LParen {
                                        depth += 1;
                                    } else if tok.token_type == TokenType::RParen {
                                        depth -= 1;
                                        if depth == 0 {
                                            break;
                                        }
                                    }
                                    let needs_space = matches!(
                                        (&last_tok_type, &tok.token_type),
                                        (Some(TokenType::Comma), _)
                                            | (Some(TokenType::Identifier), TokenType::Identifier)
                                    );
                                    if needs_space {
                                        raw_sql.push(' ');
                                    }
                                    if tok.token_type == TokenType::String {
                                        raw_sql.push('\'');
                                        raw_sql.push_str(&tok.text);
                                        raw_sql.push('\'');
                                    } else {
                                        raw_sql.push_str(&tok.text);
                                    }
                                    last_tok_type = Some(tok.token_type);
                                }
                                raw_sql.push(')');
                            } else if no_partition_kind {
                                let mut first = true;
                                while !self.is_at_end()
                                    && !self.check(TokenType::Cluster)
                                    && !self.check(TokenType::As)
                                    && !self.check(TokenType::Semicolon)
                                    && !self.check(TokenType::RParen)
                                    && !self.check_identifier("OPTIONS")
                                {
                                    if !first {
                                        raw_sql.push_str(", ");
                                    }
                                    first = false;
                                    let tok = self.advance()?;
                                    raw_sql.push_str(&tok.text);
                                    if self.check(TokenType::LParen) {
                                        self.skip();
                                        raw_sql.push('(');
                                        let mut depth = 1;
                                        while !self.is_at_end() && depth > 0 {
                                            let t = self.advance()?;
                                            if t.token_type == TokenType::LParen {
                                                depth += 1;
                                            } else if t.token_type == TokenType::RParen {
                                                depth -= 1;
                                                if depth == 0 {
                                                    break;
                                                }
                                            }
                                            raw_sql.push_str(&t.text);
                                        }
                                        raw_sql.push(')');
                                    }
                                    if !self.match_token(TokenType::Comma) {
                                        break;
                                    }
                                }
                            }
                            table_properties.push(Expression::Raw(Raw { sql: raw_sql }));
                        }
                    }
                } else {
                    self.current = saved;
                }
            }
        }
        Ok(table_properties)
    }

    fn parse_create_table_cluster_by(
        &mut self,
        _name: &TableRef,
        mut table_properties: Vec<Expression>,
    ) -> Result<Vec<Expression>> {
        if self.match_keywords(&[TokenType::Cluster, TokenType::By]) {
            let parens = self.match_token(TokenType::LParen);
            let mut cluster_names: Vec<String> = Vec::new();
            loop {
                let name = self.expect_identifier_or_keyword()?;
                cluster_names.push(name);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            if parens {
                self.expect_token(TokenType::RParen)?;
            }
            let inner = cluster_names.join(", ");
            let sql = if parens {
                format!("CLUSTER BY ({})", inner)
            } else {
                format!("CLUSTER BY {}", inner)
            };
            table_properties.push(Expression::Raw(Raw { sql }));
        }
        Ok(table_properties)
    }

    fn parse_create_table_special_trailing_options(
        &mut self,
        mut all_with_properties: Vec<(String, String)>,
        is_special_modifier: bool,
    ) -> Result<Vec<(String, String)>> {
        if is_special_modifier {
            while !self.is_at_end()
                && !self.check(TokenType::As)
                && !self.check(TokenType::Semicolon)
            {
                let is_snowflake_option = self.check(TokenType::Warehouse)
                    || self.check_identifier("TARGET_LAG")
                    || self.check_identifier("CATALOG")
                    || self.check_identifier("EXTERNAL_VOLUME")
                    || self.check_identifier("BASE_LOCATION")
                    || self.check_identifier("REFRESH_MODE")
                    || self.check_identifier("INITIALIZE")
                    || self.check_identifier("DATA_RETENTION_TIME_IN_DAYS")
                    || self.check_identifier("LOCATION")
                    || self.check_identifier("PARTITION_TYPE")
                    || self.check_identifier("FILE_FORMAT")
                    || self.check_identifier("AUTO_REFRESH");
                if is_snowflake_option {
                    let key = self.advance_text()?;
                    if self.match_token(TokenType::Eq) {
                        let value = if self.check(TokenType::LParen) {
                            self.skip();
                            let mut options = String::from("(");
                            let mut depth = 1;
                            while !self.is_at_end() && depth > 0 {
                                let tok = self.advance()?;
                                if tok.token_type == TokenType::LParen {
                                    depth += 1;
                                } else if tok.token_type == TokenType::RParen {
                                    depth -= 1;
                                }
                                if !options.ends_with('(')
                                    && !options.ends_with(' ')
                                    && tok.token_type != TokenType::RParen
                                {
                                    options.push(' ');
                                }
                                options.push_str(&tok.text);
                            }
                            options
                        } else if self.check(TokenType::String) {
                            let v = format!("'{}'", self.peek_text());
                            self.skip();
                            v
                        } else if self.check(TokenType::DAt) {
                            self.skip();
                            let mut path = String::from("@");
                            if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                                path.push_str(&self.advance_text()?);
                            }
                            while self.check(TokenType::Slash) {
                                if self.current + 1 < self.tokens.len() {
                                    let next = &self.tokens[self.current + 1];
                                    if next.text.eq_ignore_ascii_case("FILE_FORMAT")
                                        || next.text.eq_ignore_ascii_case("PARTITION_TYPE")
                                        || next.text.eq_ignore_ascii_case("AUTO_REFRESH")
                                        || next.text.eq_ignore_ascii_case("LOCATION")
                                        || next.text.eq_ignore_ascii_case("PARTITION")
                                        || next.text.eq_ignore_ascii_case("WAREHOUSE")
                                    {
                                        self.skip();
                                        path.push('/');
                                        break;
                                    }
                                }
                                self.skip();
                                path.push('/');
                                if self.is_identifier_token()
                                    || self.is_safe_keyword_as_identifier()
                                {
                                    path.push_str(&self.advance_text()?);
                                }
                            }
                            path
                        } else if self.check(TokenType::Var) && self.peek_text().starts_with('@') {
                            let mut path = self.advance_text()?;
                            while self.check(TokenType::Slash) {
                                if self.current + 1 < self.tokens.len() {
                                    let next = &self.tokens[self.current + 1];
                                    if next.text.eq_ignore_ascii_case("FILE_FORMAT")
                                        || next.text.eq_ignore_ascii_case("PARTITION_TYPE")
                                        || next.text.eq_ignore_ascii_case("AUTO_REFRESH")
                                        || next.text.eq_ignore_ascii_case("LOCATION")
                                        || next.text.eq_ignore_ascii_case("PARTITION")
                                        || next.text.eq_ignore_ascii_case("WAREHOUSE")
                                    {
                                        self.skip();
                                        path.push('/');
                                        break;
                                    }
                                }
                                self.skip();
                                path.push('/');
                                if self.is_identifier_token()
                                    || self.is_safe_keyword_as_identifier()
                                {
                                    path.push_str(&self.advance_text()?);
                                }
                            }
                            path
                        } else if self.check(TokenType::Warehouse)
                            || self.is_identifier_token()
                            || self.is_safe_keyword_as_identifier()
                        {
                            self.advance_text()?
                        } else {
                            break;
                        };
                        all_with_properties.push((key, value));
                    } else if self.is_identifier_token()
                        || self.is_safe_keyword_as_identifier()
                        || self.check(TokenType::Warehouse)
                    {
                        let value = self.advance_text()?;
                        all_with_properties.push((key, value));
                    }
                } else {
                    break;
                }
            }
        }
        Ok(all_with_properties)
    }

    fn parse_create_table_stage_file_formats(
        &mut self,
        mut table_properties: Vec<Expression>,
    ) -> Result<Vec<Expression>> {
        while self.check_identifier("STAGE_FILE_FORMAT")
            || self.check_identifier("STAGE_COPY_OPTIONS")
        {
            let key = self.advance_text()?.to_ascii_uppercase();
            self.match_token(TokenType::Eq);
            if self.match_token(TokenType::LParen) {
                let mut raw = format!("{} = (", key);
                let mut paren_depth = 1i32;
                while !self.is_at_end() && paren_depth > 0 {
                    let token = self.advance()?;
                    if token.token_type == TokenType::LParen {
                        paren_depth += 1;
                    } else if token.token_type == TokenType::RParen {
                        paren_depth -= 1;
                        if paren_depth == 0 {
                            break;
                        }
                    }
                    if token.token_type == TokenType::String {
                        raw.push_str(&format!("'{}'", token.text));
                    } else {
                        raw.push_str(&token.text);
                    }
                    if paren_depth > 0 {
                        raw.push(' ');
                    }
                }
                raw.push(')');
                table_properties.push(Expression::Raw(Raw { sql: raw }));
            }
        }
        Ok(table_properties)
    }

    fn parse_create_table_special_options(
        &mut self,
        mut all_with_properties: Vec<(String, String)>,
        is_special_modifier: bool,
    ) -> Result<Vec<(String, String)>> {
        if is_special_modifier {
            while !self.is_at_end()
                && !self.check(TokenType::As)
                && !self.check(TokenType::Semicolon)
            {
                let is_snowflake_option = self.check(TokenType::Warehouse)
                    || self.check_identifier("TARGET_LAG")
                    || self.check_identifier("CATALOG")
                    || self.check_identifier("EXTERNAL_VOLUME")
                    || self.check_identifier("BASE_LOCATION")
                    || self.check_identifier("REFRESH_MODE")
                    || self.check_identifier("INITIALIZE")
                    || self.check_identifier("DATA_RETENTION_TIME_IN_DAYS")
                    || self.check_identifier("LOCATION")
                    || self.check_identifier("PARTITION")
                    || self.check_identifier("FILE_FORMAT")
                    || self.check_identifier("AUTO_REFRESH");
                if is_snowflake_option {
                    let saved = self.current;
                    let key = self.advance_text()?;
                    if self.match_token(TokenType::Eq) {
                        let value = if self.check(TokenType::LParen) {
                            self.skip();
                            let mut options = String::from("(");
                            let mut depth = 1;
                            while !self.is_at_end() && depth > 0 {
                                let tok = self.advance()?;
                                if tok.token_type == TokenType::LParen {
                                    depth += 1;
                                } else if tok.token_type == TokenType::RParen {
                                    depth -= 1;
                                }
                                if !options.ends_with('(')
                                    && !options.ends_with(' ')
                                    && tok.token_type != TokenType::RParen
                                {
                                    options.push(' ');
                                }
                                options.push_str(&tok.text);
                            }
                            options
                        } else if self.check(TokenType::String) {
                            let v = format!("'{}'", self.peek_text());
                            self.skip();
                            v
                        } else if self.check(TokenType::DAt) {
                            self.skip();
                            let mut path = String::from("@");
                            if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                                path.push_str(&self.advance_text()?);
                            }
                            while self.check(TokenType::Slash) {
                                if self.current + 1 < self.tokens.len() {
                                    let next = &self.tokens[self.current + 1];
                                    if next.text.eq_ignore_ascii_case("FILE_FORMAT")
                                        || next.text.eq_ignore_ascii_case("PARTITION_TYPE")
                                        || next.text.eq_ignore_ascii_case("AUTO_REFRESH")
                                        || next.text.eq_ignore_ascii_case("LOCATION")
                                        || next.text.eq_ignore_ascii_case("PARTITION")
                                        || next.text.eq_ignore_ascii_case("WAREHOUSE")
                                    {
                                        self.skip();
                                        path.push('/');
                                        break;
                                    }
                                }
                                self.skip();
                                path.push('/');
                                if self.is_identifier_token()
                                    || self.is_safe_keyword_as_identifier()
                                {
                                    path.push_str(&self.advance_text()?);
                                }
                            }
                            path
                        } else if self.check(TokenType::Var) && self.peek_text().starts_with('@') {
                            let mut path = self.advance_text()?;
                            while self.check(TokenType::Slash) {
                                if self.current + 1 < self.tokens.len() {
                                    let next = &self.tokens[self.current + 1];
                                    if next.text.eq_ignore_ascii_case("FILE_FORMAT")
                                        || next.text.eq_ignore_ascii_case("PARTITION_TYPE")
                                        || next.text.eq_ignore_ascii_case("AUTO_REFRESH")
                                        || next.text.eq_ignore_ascii_case("LOCATION")
                                        || next.text.eq_ignore_ascii_case("PARTITION")
                                        || next.text.eq_ignore_ascii_case("WAREHOUSE")
                                    {
                                        self.skip();
                                        path.push('/');
                                        break;
                                    }
                                }
                                self.skip();
                                path.push('/');
                                if self.is_identifier_token()
                                    || self.is_safe_keyword_as_identifier()
                                {
                                    path.push_str(&self.advance_text()?);
                                }
                            }
                            path
                        } else if self.check(TokenType::Warehouse)
                            || self.is_identifier_token()
                            || self.is_safe_keyword_as_identifier()
                        {
                            self.advance_text()?
                        } else {
                            self.current = saved;
                            break;
                        };
                        all_with_properties.push((key, value));
                    } else if self.is_identifier_token()
                        || self.is_safe_keyword_as_identifier()
                        || self.check(TokenType::Warehouse)
                    {
                        let value = self.advance_text()?;
                        all_with_properties.push((key, value));
                    } else {
                        self.current = saved;
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        Ok(all_with_properties)
    }

    /// Parse partition bound spec for PARTITION OF: IN (...), FROM (...) TO (...), or WITH (MODULUS n, REMAINDER n)
    pub(crate) fn parse_partition_bound_spec(&mut self) -> Result<Expression> {
        if self.match_token(TokenType::In) {
            self.expect_token(TokenType::LParen)?;
            let mut values: Vec<Expression> = Vec::new();
            loop {
                let val = self.parse_expression()?;
                values.push(val);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            let this_expr = if values.len() == 1 {
                values.into_iter().next().ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?
            } else {
                Expression::Tuple(Box::new(Tuple {
                    expressions: values,
                }))
            };
            Ok(Expression::PartitionBoundSpec(Box::new(
                PartitionBoundSpec {
                    this: Some(Box::new(this_expr)),
                    expression: None,
                    from_expressions: None,
                    to_expressions: None,
                },
            )))
        } else if self.match_token(TokenType::From) {
            self.expect_token(TokenType::LParen)?;
            let mut from_vals: Vec<Expression> = Vec::new();
            loop {
                let val = self.parse_partition_bound_value()?;
                from_vals.push(val);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;

            self.expect_token(TokenType::To)?;
            self.expect_token(TokenType::LParen)?;
            let mut to_vals: Vec<Expression> = Vec::new();
            loop {
                let val = self.parse_partition_bound_value()?;
                to_vals.push(val);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;

            let from_expr = if from_vals.len() == 1 {
                from_vals.into_iter().next().ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?
            } else {
                Expression::Tuple(Box::new(Tuple {
                    expressions: from_vals,
                }))
            };
            let to_expr = if to_vals.len() == 1 {
                to_vals.into_iter().next().ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?
            } else {
                Expression::Tuple(Box::new(Tuple {
                    expressions: to_vals,
                }))
            };

            Ok(Expression::PartitionBoundSpec(Box::new(
                PartitionBoundSpec {
                    this: None,
                    expression: None,
                    from_expressions: Some(Box::new(from_expr)),
                    to_expressions: Some(Box::new(to_expr)),
                },
            )))
        } else if self.match_token(TokenType::With) {
            self.expect_token(TokenType::LParen)?;
            self.match_text_seq(&["MODULUS"]);
            let modulus = self.parse_expression()?;
            self.expect_token(TokenType::Comma)?;
            self.match_text_seq(&["REMAINDER"]);
            let remainder = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;

            Ok(Expression::PartitionBoundSpec(Box::new(
                PartitionBoundSpec {
                    this: Some(Box::new(modulus)),
                    expression: Some(Box::new(remainder)),
                    from_expressions: None,
                    to_expressions: None,
                },
            )))
        } else {
            Err(self.parse_error("Expected IN, FROM, or WITH after FOR VALUES in PARTITION OF"))
        }
    }

    /// Parse a single partition bound value (number, string, MINVALUE, MAXVALUE)
    pub(crate) fn parse_partition_bound_value(&mut self) -> Result<Expression> {
        if self.match_token(TokenType::Minvalue) {
            Ok(Expression::Var(Box::new(Var {
                this: "MINVALUE".to_string(),
            })))
        } else if self.match_token(TokenType::Maxvalue) {
            Ok(Expression::Var(Box::new(Var {
                this: "MAXVALUE".to_string(),
            })))
        } else {
            self.parse_expression()
        }
    }

    /// Parses PostgreSQL PARTITION OF column specs: names with constraint overrides, no types.
    pub(crate) fn parse_partition_column_specs(
        &mut self,
    ) -> Result<(Vec<ColumnDef>, Vec<TableConstraint>)> {
        let mut columns: Vec<ColumnDef> = Vec::new();
        let mut constraints: Vec<TableConstraint> = Vec::new();

        loop {
            if self.check(TokenType::Constraint)
                || self.check(TokenType::PrimaryKey)
                || self.check(TokenType::ForeignKey)
                || self.check(TokenType::Unique)
                || self.check(TokenType::Check)
                || self.check(TokenType::Exclude)
            {
                constraints.push(self.parse_table_constraint()?);
            } else {
                columns.push(self.parse_partition_column_spec()?);
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok((columns, constraints))
    }

    /// Parse a single partition column specification: column_name [DEFAULT value] [NOT NULL] [NULL] [WITH OPTIONS ...]
    pub(crate) fn parse_partition_column_spec(&mut self) -> Result<ColumnDef> {
        let name = self.expect_identifier_or_safe_keyword_with_quoted()?;

        let mut col_def = ColumnDef::new(name.name.clone(), DataType::Unknown);
        col_def.name = name;

        loop {
            if self.match_token(TokenType::Default) {
                let default_val = self.parse_expression()?;
                col_def.default = Some(default_val.clone());
                col_def
                    .constraints
                    .push(ColumnConstraint::Default(default_val));
                col_def.constraint_order.push(ConstraintType::Default);
            } else if self.match_keywords(&[TokenType::Not, TokenType::Null]) {
                col_def.nullable = Some(false);
                col_def.constraint_order.push(ConstraintType::NotNull);
            } else if self.match_token(TokenType::Null) {
                col_def.nullable = Some(true);
                col_def.constraint_order.push(ConstraintType::Null);
            } else if self.match_token(TokenType::Constraint) {
                let constraint_name = self.expect_identifier_or_safe_keyword()?;
                if self.match_keywords(&[TokenType::Not, TokenType::Null]) {
                    col_def.nullable = Some(false);
                    col_def.not_null_constraint_name = Some(constraint_name);
                    col_def.constraint_order.push(ConstraintType::NotNull);
                } else if self.match_token(TokenType::Check) {
                    col_def.check_constraint_name = Some(constraint_name);
                    if self.match_token(TokenType::LParen) {
                        let check_expr = self.parse_expression()?;
                        self.expect_token(TokenType::RParen)?;
                        col_def
                            .constraints
                            .push(ColumnConstraint::Check(check_expr));
                    }
                    col_def.constraint_order.push(ConstraintType::Check);
                } else if self.match_token(TokenType::Default) {
                    let default_val = self.parse_expression()?;
                    col_def.default = Some(default_val.clone());
                    col_def
                        .constraints
                        .push(ColumnConstraint::Default(default_val));
                    col_def.constraint_order.push(ConstraintType::Default);
                }
            } else {
                self.match_text_seq(&["WITH", "OPTIONS"]);
                break;
            }
        }

        Ok(col_def)
    }

    pub(crate) fn is_snowflake_inline_table_option_start(&self) -> bool {
        self.check(TokenType::Warehouse)
            || self.check_identifier("TARGET_LAG")
            || self.check_identifier("CHANGE_TRACKING")
            || self.check_identifier("DATA_RETENTION_TIME_IN_DAYS")
            || self.check_identifier("CATALOG")
            || self.check_identifier("EXTERNAL_VOLUME")
            || self.check_identifier("BASE_LOCATION")
            || self.check_identifier("REFRESH_MODE")
            || self.check_identifier("INITIALIZE")
            || self.check_identifier("LOCATION")
            || self.check_identifier("PARTITION")
            || self.check_identifier("PARTITION_TYPE")
            || self.check_identifier("FILE_FORMAT")
            || self.check_identifier("AUTO_REFRESH")
    }

    pub(crate) fn parse_snowflake_inline_table_options(&mut self) -> Result<Vec<(String, String)>> {
        let mut options: Vec<(String, String)> = Vec::new();

        while self.is_snowflake_inline_table_option_start() {
            let saved = self.current;
            let key = self.advance_text()?;
            let value = if self.match_token(TokenType::Eq)
                || self.is_identifier_token()
                || self.is_safe_keyword_as_identifier()
                || self.check_keyword()
                || self.check(TokenType::Number)
                || self.check(TokenType::String)
            {
                self.parse_snowflake_inline_table_option_value()?
            } else {
                self.current = saved;
                break;
            };
            options.push((key, value));
        }

        Ok(options)
    }
}

impl Parser {
    /// Parses `USING TEMPLATE`; Snowflake takes only a string or a parenthesized query.
    fn parse_using_template(&mut self) -> Result<Option<Box<Expression>>> {
        if !self.match_text_seq(&["USING", "TEMPLATE"]) {
            return Ok(None);
        }
        let query = self.check(TokenType::String)
            || self.check(TokenType::LParen)
                && matches!(
                    self.peek_nth(1).map(|token| token.token_type),
                    Some(TokenType::Select | TokenType::With)
                );
        if self.is_snowflake() && !query {
            return Err(
                self.parse_error("Expected a string or a parenthesized query after USING TEMPLATE")
            );
        }
        Ok(Some(Box::new(self.parse_primary()?)))
    }

    /// Reports whether this is a Snowflake hybrid table, which needs columns, AS or LIKE.
    fn check_snowflake_hybrid_body(&mut self, table_modifier: Option<&str>) -> Result<bool> {
        let hybrid = self.is_snowflake() && matches!(table_modifier, Some("HYBRID"));
        if hybrid && !self.check(TokenType::LParen) && !self.check(TokenType::Like) {
            return Err(
                self.parse_error("Expected columns, AS or LIKE after the hybrid table name")
            );
        }
        Ok(hybrid)
    }
}
