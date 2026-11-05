//! Parser ddl: the CREATE TABLE forms that end before a column list: clones, LIKE, AS SELECT, templates and property-only tables.

use crate::expressions::columns::models::TableConstraint;
use crate::expressions::names::models::Identifier;
use crate::expressions::statements::models::Raw;
use crate::expressions::statements::models::Var;
use crate::expressions::statements::models::Version;
use crate::expressions::tables::models::CreateTable;
use crate::expressions::tables::models::PartitionedOfProperty;
use crate::expressions::tree::models::Expression;
use crate::parser::_helpers::ddl::snowflake_create_clone::CloneOptions;
use crate::parser::models::CreateTableCloneKind;
use crate::parser::models::CreateTableHead;
use crate::parser::models::CreateTableLeadingOptions;
use crate::parser::models::CreateTableNameSuffix;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parse CREATE TABLE ... PARTITION OF parent_table [(cols)] [FOR VALUES spec | DEFAULT] [PARTITION BY ...]
    pub(crate) fn parse_create_table_partition_of(
        &mut self,
        head: CreateTableHead<'_>,
    ) -> Result<Expression> {
        let CreateTableHead {
            name,
            if_not_exists,
            temporary,
            or_replace,
            table_modifier,
            leading_comments,
        } = head;
        let parent_table = self.parse_table_ref()?;

        let (columns, constraints) = if self.check(TokenType::LParen) {
            let first_inside = self.current + 1;
            let is_column_defs = first_inside < self.tokens.len()
                && (self.tokens[first_inside].token_type == TokenType::Constraint
                    || (self.tokens[first_inside].token_type == TokenType::Var
                        || self.tokens[first_inside].token_type == TokenType::QuotedIdentifier
                        || self.tokens[first_inside].token_type == TokenType::Identifier)
                        && first_inside + 1 < self.tokens.len()
                        && self.tokens[first_inside + 1].token_type == TokenType::Default);

            if is_column_defs {
                self.skip();
                let (cols, constrs) = self.parse_partition_column_specs()?;
                self.expect_token(TokenType::RParen)?;
                (cols, constrs)
            } else {
                (Vec::new(), Vec::new())
            }
        } else {
            (Vec::new(), Vec::new())
        };

        let partition_bound: Expression = if self.match_token(TokenType::Default) {
            Expression::Var(Box::new(Var {
                this: "DEFAULT".to_string(),
            }))
        } else if self.match_token(TokenType::For) {
            self.expect_token(TokenType::Values)?;
            self.parse_partition_bound_spec()?
        } else {
            Expression::Var(Box::new(Var {
                this: "DEFAULT".to_string(),
            }))
        };

        let partition_of_expr =
            Expression::PartitionedOfProperty(Box::new(PartitionedOfProperty {
                this: Box::new(Expression::Table(Box::new(parent_table))),
                expression: Box::new(partition_bound),
            }));

        let mut table_properties: Vec<Expression> = Vec::new();
        if self.match_token(TokenType::Partition) || self.match_token(TokenType::PartitionBy) {
            if self.previous().token_type == TokenType::Partition {
                self.expect_token(TokenType::By)?;
            }
            let partition_kind = if self.check(TokenType::Identifier) || self.check(TokenType::Var)
            {
                self.advance_text()?.to_ascii_uppercase()
            } else if self.check(TokenType::Range) {
                self.skip();
                "RANGE".to_string()
            } else if self.check(TokenType::List) {
                self.skip();
                "LIST".to_string()
            } else {
                "RANGE".to_string()
            };
            let mut raw_sql = format!("PARTITION BY {}", partition_kind);
            if self.check(TokenType::LParen) {
                self.skip();
                raw_sql.push('(');
                let mut depth = 1;
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
                    raw_sql.push_str(&tok.text);
                }
                raw_sql.push(')');
            }
            table_properties.push(Expression::Raw(Raw { sql: raw_sql }));
        }

        Ok(Expression::CreateTable(Box::new(CreateTable {
            name,
            on_cluster: None,
            columns,
            constraints,
            if_not_exists,
            temporary,
            or_replace,
            table_modifier: table_modifier.map(|s| s.to_string()),
            source_dialect: None,
            as_select: None,
            as_select_parenthesized: false,
            on_commit: None,
            clone_source: None,
            clone_at_clause: None,
            shallow_clone: false,
            deep_clone: false,
            is_copy: false,
            leading_comments,
            with_properties: Vec::new(),
            teradata_post_name_options: Vec::new(),
            with_data: None,
            with_statistics: None,
            teradata_indexes: Vec::new(),
            with_cte: None,
            properties: table_properties,
            partition_of: Some(partition_of_expr),
            post_table_properties: Vec::new(),
            mysql_table_options: Vec::new(),
            tidb_table_options: Vec::new(),
            inherits: Vec::new(),
            on_property: None,
            copy_grants: false,
            using_template: None,
            rollup: None,
            uuid: None,
            with_partition_columns: Vec::new(),
            with_connection: None,
        })))
    }

    pub(crate) fn parse_create_table_clone(
        &mut self,
        head: CreateTableHead<'_>,
        suffix: CreateTableNameSuffix,
        clone_kind: CreateTableCloneKind,
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
        let CreateTableCloneKind {
            shallow_clone,
            deep_clone,
            is_copy,
        } = clone_kind;

        self.skip();

        let snowflake = self.is_snowflake();
        if snowflake {
            for part in [&name.catalog, &name.schema, &Some(name.name.clone())]
                .into_iter()
                .flatten()
            {
                self.reject_reserved_name_part(part)?;
            }
        }
        let source = if snowflake {
            self.parse_object_name()?
        } else {
            self.parse_table_ref()?
        };
        let at_clause = if self.is_snowflake() {
            self.parse_clone_time_travel()?
        } else if self.match_identifier("AT") || self.match_token(TokenType::Before) {
            let keyword = self.previous_text().to_ascii_uppercase();
            self.expect_token(TokenType::LParen)?;
            let mut result = format!("{} (", keyword);
            let mut prev_token_type: Option<TokenType> = None;
            let mut paren_depth = 1;
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
                let needs_space = !result.ends_with('(')
                    && prev_token_type != Some(TokenType::Arrow)
                    && prev_token_type != Some(TokenType::Dash)
                    && prev_token_type != Some(TokenType::LParen)
                    && prev_token_type != Some(TokenType::Comma)
                    && token.token_type != TokenType::LParen;
                if needs_space
                    && token.token_type != TokenType::RParen
                    && token.token_type != TokenType::Comma
                {
                    result.push(' ');
                }
                if token.token_type == TokenType::String {
                    result.push('\'');
                    result.push_str(&token.text.replace('\'', "''"));
                    result.push('\'');
                } else {
                    result.push_str(&token.text);
                }
                if token.token_type == TokenType::Arrow || token.token_type == TokenType::Comma {
                    result.push(' ');
                }
                prev_token_type = Some(token.token_type);
            }
            result.push(')');
            Some(Expression::Raw(Raw { sql: result }))
        } else if matches!(
            self.peek().token_type,
            TokenType::VersionSnapshot | TokenType::TimestampSnapshot
        ) || (self.check_keyword_text("VERSION") || self.check_keyword_text("TIMESTAMP"))
            && self
                .peek_nth(1)
                .is_some_and(|t| t.token_type == TokenType::As)
            && self
                .peek_nth(2)
                .is_some_and(|t| t.text.eq_ignore_ascii_case("OF"))
        {
            let version_kind = if self.match_token(TokenType::TimestampSnapshot) {
                "TIMESTAMP".to_string()
            } else if self.match_token(TokenType::VersionSnapshot) {
                "VERSION".to_string()
            } else {
                let version_kind = self.advance_text()?.to_ascii_uppercase();
                self.expect_token(TokenType::As)?;
                if !self.match_keyword("OF") {
                    return Err(self.parse_error("Expected OF after AS"));
                }
                version_kind
            };

            let value = self.parse_bitwise()?.ok_or_else(|| {
                self.parse_error(format!("Expected expression after {version_kind} AS OF"))
            })?;

            Some(Expression::Version(Box::new(Version {
                this: Box::new(Expression::Identifier(Identifier::new(&version_kind))),
                kind: "AS OF".to_string(),
                expression: Some(Box::new(value)),
            })))
        } else {
            None
        };
        let properties = if snowflake {
            Self::clone_options_as_properties(self.parse_clone_options(CloneOptions::Copy)?)
        } else {
            Vec::new()
        };
        Ok(Expression::CreateTable(Box::new(CreateTable {
            name,
            on_cluster: on_cluster.clone(),
            columns: Vec::new(),
            constraints: Vec::new(),
            if_not_exists,
            temporary,
            or_replace,
            table_modifier: table_modifier.map(|s| s.to_string()),
            source_dialect: None,
            as_select: None,
            as_select_parenthesized: false,
            on_commit: None,
            clone_source: Some(source),
            clone_at_clause: at_clause,
            shallow_clone,
            deep_clone,
            is_copy,
            leading_comments,
            with_properties: Vec::new(),
            teradata_post_name_options: teradata_post_name_options.clone(),
            with_data: None,
            with_statistics: None,
            teradata_indexes: Vec::new(),
            with_cte: None,
            properties,
            partition_of: None,
            post_table_properties: Vec::new(),
            mysql_table_options: Vec::new(),
            tidb_table_options: Vec::new(),
            inherits: Vec::new(),
            on_property: None,
            copy_grants: false,
            using_template: None,
            rollup: None,
            uuid: uuid.clone(),
            with_partition_columns: Vec::new(),
            with_connection: None,
        })))
    }

    pub(crate) fn parse_create_table_using_template(
        &mut self,
        head: CreateTableHead<'_>,
        suffix: CreateTableNameSuffix,
        leading: CreateTableLeadingOptions,
        using_template: Option<Box<Expression>>,
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
        let CreateTableLeadingOptions {
            copy_grants,
            with_properties,
        } = leading;

        Ok(Expression::CreateTable(Box::new(CreateTable {
            name,
            on_cluster: on_cluster.clone(),
            columns: Vec::new(),
            constraints: Vec::new(),
            if_not_exists,
            temporary,
            or_replace,
            table_modifier: table_modifier.map(|s| s.to_string()),
            source_dialect: None,
            as_select: None,
            as_select_parenthesized: false,
            on_commit: None,
            clone_source: None,
            clone_at_clause: None,
            shallow_clone: false,
            deep_clone: false,
            is_copy: false,
            leading_comments,
            with_properties,
            teradata_post_name_options: teradata_post_name_options.clone(),
            with_data: None,
            with_statistics: None,
            teradata_indexes: Vec::new(),
            with_cte: None,
            properties: Vec::new(),
            partition_of: None,
            post_table_properties: Vec::new(),
            mysql_table_options: Vec::new(),
            tidb_table_options: Vec::new(),
            inherits: Vec::new(),
            on_property: None,
            copy_grants,
            using_template,
            rollup: None,
            uuid: uuid.clone(),
            with_partition_columns: Vec::new(),
            with_connection: None,
        })))
    }

    pub(crate) fn parse_create_table_as_select(
        &mut self,
        head: CreateTableHead<'_>,
        suffix: CreateTableNameSuffix,
        leading: CreateTableLeadingOptions,
        redshift_ctas_properties: Vec<Expression>,
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
        let CreateTableLeadingOptions {
            copy_grants,
            with_properties,
        } = leading;

        let mut as_select_parenthesized = self.check(TokenType::LParen);
        let query = if as_select_parenthesized {
            self.expect_token(TokenType::LParen)?;
            let lparen_comments = self.previous_trailing_comments().to_vec();
            let subquery = self.parse_parenthesized_primary(lparen_comments, true)?;
            if matches!(
                &subquery,
                Expression::Union(_) | Expression::Intersect(_) | Expression::Except(_)
            ) {
                as_select_parenthesized = false;
                subquery
            } else {
                if let Expression::Subquery(ref sq) = subquery {
                    if sq.limit.is_some() || sq.offset.is_some() || sq.order_by.is_some() {
                        subquery
                    } else {
                        if let Expression::Subquery(sq) = subquery {
                            sq.this
                        } else {
                            subquery
                        }
                    }
                } else if let Expression::Paren(p) = subquery {
                    p.this
                } else {
                    subquery
                }
            }
        } else if self.check(TokenType::With) {
            self.parse_statement()?
        } else if self.is_identifier_token() {
            Expression::Table(Box::new(self.parse_table_ref()?))
        } else {
            self.parse_select()?
        };

        let (with_data, with_statistics, teradata_indexes) = self.parse_teradata_table_options()?;
        let on_commit = { None };

        Ok(Expression::CreateTable(Box::new(CreateTable {
            name,
            on_cluster: on_cluster.clone(),
            columns: Vec::new(),
            constraints: Vec::new(),
            if_not_exists,
            temporary,
            or_replace,
            table_modifier: table_modifier.map(|s| s.to_string()),
            source_dialect: None,
            as_select: Some(query),
            as_select_parenthesized,
            on_commit,
            clone_source: None,
            clone_at_clause: None,
            shallow_clone: false,
            deep_clone: false,
            is_copy: false,
            leading_comments,
            with_properties,
            teradata_post_name_options: teradata_post_name_options.clone(),
            with_data,
            with_statistics,
            teradata_indexes,
            with_cte: None,
            properties: Vec::new(),
            partition_of: None,
            post_table_properties: redshift_ctas_properties,
            mysql_table_options: Vec::new(),
            tidb_table_options: Vec::new(),
            inherits: Vec::new(),
            on_property: None,
            copy_grants,
            using_template: None,
            rollup: None,
            uuid: uuid.clone(),
            with_partition_columns: Vec::new(),
            with_connection: None,
        })))
    }

    pub(crate) fn parse_create_table_special_without_columns(
        &mut self,
        head: CreateTableHead<'_>,
        suffix: CreateTableNameSuffix,
        leading: CreateTableLeadingOptions,
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
        let CreateTableLeadingOptions { copy_grants, .. } = leading;

        let mut extra_options: Vec<(String, String)> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::As) && !self.check(TokenType::Semicolon) {
            if self.is_identifier_token()
                || self.is_safe_keyword_as_identifier()
                || self.check(TokenType::Warehouse)
            {
                let key = self.advance_text()?;
                if self.match_token(TokenType::Eq) {
                    let value = if self.check(TokenType::String) {
                        let v = format!("'{}'", self.peek_text());
                        self.skip();
                        v
                    } else if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                        self.advance_text()?
                    } else {
                        break;
                    };
                    extra_options.push((key, value));
                } else {
                    if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                        let value = self.advance_text()?;
                        extra_options.push((key, value));
                    }
                }
            } else {
                break;
            }
        }
        let as_select = if self.match_token(TokenType::As) {
            Some(self.parse_statement()?)
        } else {
            None
        };
        Ok(Expression::CreateTable(Box::new(CreateTable {
            name,
            on_cluster: on_cluster.clone(),
            columns: Vec::new(),
            constraints: Vec::new(),
            if_not_exists,
            temporary,
            or_replace,
            table_modifier: table_modifier.map(|s| s.to_string()),
            source_dialect: None,
            as_select,
            as_select_parenthesized: false,
            on_commit: None,
            clone_source: None,
            clone_at_clause: None,
            shallow_clone: false,
            deep_clone: false,
            is_copy: false,
            leading_comments,
            with_properties: extra_options,
            teradata_post_name_options: teradata_post_name_options.clone(),
            with_data: None,
            with_statistics: None,
            teradata_indexes: Vec::new(),
            with_cte: None,
            properties: Vec::new(),
            partition_of: None,
            post_table_properties: Vec::new(),
            mysql_table_options: Vec::new(),
            tidb_table_options: Vec::new(),
            inherits: Vec::new(),
            on_property: None,
            copy_grants,
            using_template: None,
            rollup: None,
            uuid: uuid.clone(),
            with_partition_columns: Vec::new(),
            with_connection: None,
        })))
    }

    pub(crate) fn parse_create_table_like(
        &mut self,
        head: CreateTableHead<'_>,
        suffix: CreateTableNameSuffix,
        leading: CreateTableLeadingOptions,
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
        let CreateTableLeadingOptions {
            copy_grants,
            with_properties,
        } = leading;

        self.skip();
        let source_ref = self.parse_table_ref()?;
        Ok(Expression::CreateTable(Box::new(CreateTable {
            name,
            on_cluster: on_cluster.clone(),
            columns: Vec::new(),
            constraints: vec![TableConstraint::Like {
                source: source_ref,
                options: Vec::new(),
            }],
            if_not_exists,
            temporary,
            or_replace,
            table_modifier: table_modifier.map(|s| s.to_string()),
            source_dialect: None,
            as_select: None,
            as_select_parenthesized: false,
            on_commit: None,
            clone_source: None,
            clone_at_clause: None,
            shallow_clone: false,
            deep_clone: false,
            is_copy: false,
            leading_comments,
            with_properties,
            teradata_post_name_options: teradata_post_name_options.clone(),
            with_data: None,
            with_statistics: None,
            teradata_indexes: Vec::new(),
            with_cte: None,
            properties: Vec::new(),
            partition_of: None,
            post_table_properties: Vec::new(),
            mysql_table_options: Vec::new(),
            tidb_table_options: Vec::new(),
            inherits: Vec::new(),
            on_property: None,
            copy_grants,
            using_template: None,
            rollup: None,
            uuid: uuid.clone(),
            with_partition_columns: Vec::new(),
            with_connection: None,
        })))
    }

    pub(crate) fn parse_create_table_tags_only(
        &mut self,
        head: CreateTableHead<'_>,
        suffix: CreateTableNameSuffix,
        leading: CreateTableLeadingOptions,
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
        let CreateTableLeadingOptions {
            copy_grants,
            with_properties,
        } = leading;

        let tags = self.parse_tags()?;
        Ok(Expression::CreateTable(Box::new(CreateTable {
            name,
            on_cluster: on_cluster.clone(),
            columns: Vec::new(),
            constraints: vec![TableConstraint::Tags(tags)],
            if_not_exists,
            temporary,
            or_replace,
            table_modifier: table_modifier.map(|s| s.to_string()),
            source_dialect: None,
            as_select: None,
            as_select_parenthesized: false,
            on_commit: None,
            clone_source: None,
            clone_at_clause: None,
            shallow_clone: false,
            deep_clone: false,
            is_copy: false,
            leading_comments,
            with_properties,
            teradata_post_name_options: teradata_post_name_options.clone(),
            with_data: None,
            with_statistics: None,
            teradata_indexes: Vec::new(),
            with_cte: None,
            properties: Vec::new(),
            partition_of: None,
            post_table_properties: Vec::new(),
            mysql_table_options: Vec::new(),
            tidb_table_options: Vec::new(),
            inherits: Vec::new(),
            on_property: None,
            copy_grants,
            using_template: None,
            rollup: None,
            uuid: uuid.clone(),
            with_partition_columns: Vec::new(),
            with_connection: None,
        })))
    }

    pub(crate) fn parse_create_table_tblproperties_only(
        &mut self,
        head: CreateTableHead<'_>,
        suffix: CreateTableNameSuffix,
        leading: CreateTableLeadingOptions,
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
        let CreateTableLeadingOptions {
            copy_grants,
            with_properties,
        } = leading;

        let hive_properties = self.parse_hive_table_properties()?;

        let as_select = if self.match_token(TokenType::As) {
            Some(self.parse_statement()?)
        } else {
            None
        };

        Ok(Expression::CreateTable(Box::new(CreateTable {
            name,
            on_cluster: on_cluster.clone(),
            columns: Vec::new(),
            constraints: Vec::new(),
            if_not_exists,
            temporary,
            or_replace,
            table_modifier: table_modifier.map(|s| s.to_string()),
            source_dialect: None,
            as_select,
            as_select_parenthesized: false,
            on_commit: None,
            clone_source: None,
            clone_at_clause: None,
            shallow_clone: false,
            deep_clone: false,
            is_copy: false,
            leading_comments,
            with_properties,
            teradata_post_name_options: teradata_post_name_options.clone(),
            with_data: None,
            with_statistics: None,
            teradata_indexes: Vec::new(),
            with_cte: None,
            properties: hive_properties,
            partition_of: None,
            post_table_properties: Vec::new(),
            mysql_table_options: Vec::new(),
            tidb_table_options: Vec::new(),
            inherits: Vec::new(),
            on_property: None,
            copy_grants,
            using_template: None,
            rollup: None,
            uuid: uuid.clone(),
            with_partition_columns: Vec::new(),
            with_connection: None,
        })))
    }

    pub(crate) fn parse_create_table_parenthesized_as_select(
        &mut self,
        head: CreateTableHead<'_>,
        suffix: CreateTableNameSuffix,
        leading: CreateTableLeadingOptions,
    ) -> Result<Expression> {
        if self.is_snowflake() {
            return Err(self.parse_error("Expected column definitions or AS before the query"));
        }
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
        let CreateTableLeadingOptions {
            copy_grants,
            with_properties,
        } = leading;

        let subquery = self.parse_primary()?;
        let query = if let Expression::Subquery(sq) = subquery {
            sq.this
        } else if let Expression::Paren(p) = subquery {
            p.this
        } else {
            subquery
        };
        Ok(Expression::CreateTable(Box::new(CreateTable {
            name,
            on_cluster: on_cluster.clone(),
            columns: Vec::new(),
            constraints: Vec::new(),
            if_not_exists,
            temporary,
            or_replace,
            table_modifier: table_modifier.map(|s| s.to_string()),
            source_dialect: None,
            as_select: Some(query),
            as_select_parenthesized: true,
            on_commit: None,
            clone_source: None,
            clone_at_clause: None,
            shallow_clone: false,
            deep_clone: false,
            is_copy: false,
            leading_comments,
            with_properties,
            teradata_post_name_options: teradata_post_name_options.clone(),
            with_data: None,
            with_statistics: None,
            teradata_indexes: Vec::new(),
            with_cte: None,
            properties: Vec::new(),
            partition_of: None,
            post_table_properties: Vec::new(),
            mysql_table_options: Vec::new(),
            tidb_table_options: Vec::new(),
            inherits: Vec::new(),
            on_property: None,
            copy_grants,
            using_template: None,
            rollup: None,
            uuid: uuid.clone(),
            with_partition_columns: Vec::new(),
            with_connection: None,
        })))
    }
}
