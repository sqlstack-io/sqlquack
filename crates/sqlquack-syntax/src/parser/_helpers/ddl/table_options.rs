//! Parser ddl: table options.

use crate::expressions::columns::models::ColumnConstraint;
use crate::expressions::columns::models::ColumnDef;
use crate::expressions::columns::models::ComputedColumn;
use crate::expressions::columns::models::GeneratedAsIdentity;
use crate::expressions::columns::models::GeneratedAsRow;
use crate::expressions::columns::models::LikeOptionAction;
use crate::expressions::columns::models::TableConstraint;
use crate::expressions::functions::models::Function;
use crate::expressions::names::main::identifier_from_token::identifier_from_token;
use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::names::models::Null;
use crate::expressions::operators::models::BinaryOp;
use crate::expressions::statements::models::Raw;
use crate::expressions::tables::models::ClusteredByProperty;
use crate::expressions::tables::models::ConstraintType;
use crate::expressions::tables::models::FileFormatProperty;
use crate::expressions::tables::models::InputOutputFormat;
use crate::expressions::tables::models::LocationProperty;
use crate::expressions::tables::models::PartitionedByProperty;
use crate::expressions::tables::models::Properties;
use crate::expressions::tables::models::SortOrder;
use crate::expressions::tables::models::StorageHandlerProperty;
use crate::expressions::tables::models::TeradataIndex;
use crate::expressions::tables::models::TeradataIndexKind;
use crate::expressions::tables::models::TiDBTableOption;
use crate::expressions::tables::models::WithSystemVersioningProperty;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::Tuple;
use crate::parser::constants::CHARSET;
use crate::parser::constants::UNSIGNED;
use crate::parser::constants::ZEROFILL;
use crate::parser::models::Parser;
use crate::parser::types::MySqlTableOptions;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    pub(crate) fn parse_snowflake_inline_table_option_value(&mut self) -> Result<String> {
        if self.check(TokenType::String) {
            return Ok(self.advance_quoted_string());
        }

        if self.check(TokenType::LParen) {
            let start = self.current;
            self.skip();
            let mut depth = 1;
            while !self.is_at_end() && depth > 0 {
                if self.check(TokenType::LParen) {
                    depth += 1;
                } else if self.check(TokenType::RParen) {
                    depth -= 1;
                    self.skip();
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
                self.skip();
            }
            return Ok(self.tokens_to_sql(start, self.current));
        }

        if self.check(TokenType::Number)
            || self.is_identifier_token()
            || self.is_safe_keyword_as_identifier()
            || self.check_keyword()
        {
            return self.advance_text();
        }

        Err(self.parse_error("Expected Snowflake table option value"))
    }

    /// Parses CREATE TABLE WITH (key = value, ...) properties.
    pub(crate) fn parse_with_properties(&mut self) -> Result<Vec<(String, String)>> {
        self.expect_token(TokenType::LParen)?;
        let mut properties: Vec<(String, String)> = Vec::new();

        loop {
            if self.check(TokenType::RParen) {
                break;
            }

            let mut key = self.expect_identifier_or_keyword()?;

            if key.eq_ignore_ascii_case("PARTITIONED") && self.check(TokenType::By) {
                self.skip();
                key = "PARTITIONED_BY".to_string();
            }

            self.expect_token(TokenType::Eq)?;

            let value = if self.check(TokenType::String) {
                self.advance_quoted_string()
            } else if self.match_token(TokenType::LParen) {
                let mut depth = 1;
                let mut result = String::from("(");
                let mut need_space = false;
                while !self.is_at_end() && depth > 0 {
                    if self.check(TokenType::LParen) {
                        depth += 1;
                    } else if self.check(TokenType::RParen) {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    let token = self.peek();
                    let text = &token.text;
                    let token_type = token.token_type;

                    let is_punctuation = matches!(
                        token_type,
                        TokenType::Comma | TokenType::LParen | TokenType::RParen
                    );
                    if need_space && !is_punctuation {
                        result.push(' ');
                    }

                    result.push_str(text);

                    need_space = token_type == TokenType::Comma
                        || !is_punctuation
                            && !matches!(
                                token_type,
                                TokenType::LParen | TokenType::RParen | TokenType::Comma
                            );
                    self.skip();
                }
                self.expect_token(TokenType::RParen)?;
                result.push(')');
                result
            } else if self.check_identifier("ARRAY")
                && self
                    .peek_nth(1)
                    .is_some_and(|t| t.token_type == TokenType::LBracket)
            {
                let mut result = self.advance_text()?;
                self.expect_token(TokenType::LBracket)?;
                result.push('[');
                let mut first = true;
                while !self.is_at_end() && !self.check(TokenType::RBracket) {
                    if !first {
                        if self.match_token(TokenType::Comma) {
                            result.push_str(", ");
                        } else {
                            break;
                        }
                    }
                    first = false;
                    if self.check(TokenType::String) {
                        result.push('\'');
                        result.push_str(&self.advance_text()?);
                        result.push('\'');
                    } else if self.is_identifier_token() {
                        result.push_str(&self.advance_text()?);
                    } else {
                        break;
                    }
                }
                self.expect_token(TokenType::RBracket)?;
                result.push(']');
                result
            } else if self.check(TokenType::Number) {
                self.advance_text()?
            } else {
                self.expect_identifier_or_keyword()?
            };

            properties.push((key, value));

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        self.expect_token(TokenType::RParen)?;
        Ok(properties)
    }

    /// Parse column definitions and table constraints
    pub(crate) fn parse_column_definitions(
        &mut self,
    ) -> Result<(Vec<ColumnDef>, Vec<TableConstraint>)> {
        let mut columns: Vec<ColumnDef> = Vec::new();
        let mut constraints: Vec<TableConstraint> = Vec::new();

        loop {
            if self.check(TokenType::RParen) {
                break;
            }
            if self.check(TokenType::Like) {
                constraints.push(self.parse_like_clause()?);
            } else if self.check(TokenType::Constraint)
                || self.check(TokenType::PrimaryKey)
                || self.check(TokenType::ForeignKey)
                || self.check(TokenType::Unique)
                || (self.check(TokenType::Check))
                    && self
                        .peek_nth(1)
                        .is_some_and(|t| t.token_type == TokenType::LParen)
                || self.check(TokenType::Exclude)
            {
                constraints.push(self.parse_table_constraint()?);
            } else {
                if self.check(TokenType::Index)
                    || self.check(TokenType::Key)
                    || self.check_identifier("FULLTEXT")
                    || self.check_identifier("SPATIAL")
                {
                    let looks_like_key_constraint = if self.check(TokenType::Key) {
                        self.check_next(TokenType::LParen)
                            || (self.check_next(TokenType::Identifier)
                                || self.check_next(TokenType::Var)
                                || self.check_next(TokenType::QuotedIdentifier))
                                && self.current + 2 < self.tokens.len()
                                && self.tokens[self.current + 2].token_type == TokenType::LParen
                    } else {
                        true
                    };

                    if looks_like_key_constraint {
                        constraints.push(self.parse_index_table_constraint()?);
                    } else {
                        columns.push(self.parse_column_def()?);
                    }
                } else if self.check_identifier("PERIOD") {
                    if let Some(period_constraint) =
                        self.parse_period_for_system_time_table_constraint()?
                    {
                        constraints.push(period_constraint);
                    } else {
                        columns.push(self.parse_column_def()?);
                    }
                } else if self.check_identifier("INITIALLY") {
                    self.skip();
                    if self.match_identifier("DEFERRED") {
                        constraints.push(TableConstraint::InitiallyDeferred { deferred: true });
                    } else if self.match_identifier("IMMEDIATE") {
                        constraints.push(TableConstraint::InitiallyDeferred { deferred: false });
                    } else {
                        return Err(
                            self.parse_error("Expected DEFERRED or IMMEDIATE after INITIALLY")
                        );
                    }
                } else {
                    columns.push(self.parse_column_def()?);
                }
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok((columns, constraints))
    }

    /// Parse LIKE clause in CREATE TABLE: LIKE source_table [INCLUDING|EXCLUDING options]
    pub(crate) fn parse_like_clause(&mut self) -> Result<TableConstraint> {
        self.expect_token(TokenType::Like)?;
        let source = self.parse_table_ref()?;
        let mut options: Vec<(LikeOptionAction, String)> = Vec::new();

        loop {
            if self.match_identifier("INCLUDING") {
                let prop = self.expect_identifier_or_keyword()?.to_ascii_uppercase();
                options.push((LikeOptionAction::Including, prop));
            } else if self.match_identifier("EXCLUDING") {
                let prop = self.expect_identifier_or_keyword()?.to_ascii_uppercase();
                options.push((LikeOptionAction::Excluding, prop));
            } else {
                break;
            }
        }

        Ok(TableConstraint::Like { source, options })
    }

    /// Parse a single column definition
    pub(crate) fn parse_column_def(&mut self) -> Result<ColumnDef> {
        let name = { self.expect_identifier_or_safe_keyword_with_quoted()? };

        if self.check(TokenType::As) {
            let mut col_def = ColumnDef::new(
                name.name.clone(),
                DataType::Custom {
                    name: String::new(),
                },
            );
            col_def.name = name;
            self.skip();
            if self.check(TokenType::LParen) {
                col_def = self.parse_as_computed_column(col_def)?;
            }
            return Ok(col_def);
        }

        let no_type = self.check(TokenType::Comma) || self.check(TokenType::RParen);
        let data_type = if no_type {
            DataType::Custom {
                name: String::new(),
            }
        } else {
            self.parse_data_type()?
        };

        let mut col_def = ColumnDef::new(name.name.clone(), data_type);
        col_def.name = name;
        col_def.no_type = no_type;

        while self.match_identifier("UNSIGNED")
            || self.match_identifier("ZEROFILL")
            || self.match_identifier("SIGNED")
        {
            let modifier = self.previous_text().to_ascii_uppercase();
            if modifier == UNSIGNED {
                col_def.unsigned = true;
            } else if modifier == ZEROFILL {
                col_def.zerofill = true;
            }
        }

        if self.match_identifier("OPTIONS") {
            col_def.options = self.parse_options_list()?;
        }

        loop {
            if self.match_keywords(&[TokenType::Not, TokenType::Null]) {
                col_def.nullable = Some(false);
                col_def.constraint_order.push(ConstraintType::NotNull);
            } else if self.match_token(TokenType::Null) {
                col_def.nullable = Some(true);
                col_def.constraint_order.push(ConstraintType::Null);
            } else if self.match_keywords(&[TokenType::PrimaryKey, TokenType::Key]) {
                col_def.primary_key = true;
                if self.match_token(TokenType::Asc) {
                    col_def.primary_key_order = Some(SortOrder::Asc);
                } else if self.match_token(TokenType::Desc) {
                    col_def.primary_key_order = Some(SortOrder::Desc);
                }
                col_def.constraint_order.push(ConstraintType::PrimaryKey);
            } else if self.match_token(TokenType::Constraint) {
                col_def = self.parse_column_named_constraint(col_def)?;
            } else if self.match_token(TokenType::Unique) {
                col_def.unique = true;
                if self.match_text_seq(&["NULLS", "NOT", "DISTINCT"]) {
                    col_def.unique_nulls_not_distinct = true;
                }
                col_def.constraint_order.push(ConstraintType::Unique);
            } else if self.match_token(TokenType::Check) {
                if self.match_token(TokenType::LParen) {
                    let check_expr = self.parse_expression()?;
                    self.expect_token(TokenType::RParen)?;
                    col_def
                        .constraints
                        .push(ColumnConstraint::Check(check_expr));
                    col_def.constraint_order.push(ConstraintType::Check);
                }
            } else {
                let (next, matched) = self.parse_column_option(col_def)?;
                col_def = next;
                if !matched {
                    {
                        if self.match_identifier("TTL") {
                            let expr = self.parse_expression()?;
                            col_def.ttl_expr = Some(Box::new(expr));
                        } else {
                            if self.skip_column_modifier() {
                                continue;
                            }
                            break;
                        }
                    }
                }
            }
        }

        Ok(col_def)
    }

    fn parse_column_option(&mut self, mut col_def: ColumnDef) -> Result<(ColumnDef, bool)> {
        if self.match_token(TokenType::AutoIncrement) || self.match_keyword("IDENTITY") {
            col_def = self.parse_column_identity(col_def)?;
        } else if self.match_token(TokenType::Default) {
            col_def.default = Some(self.parse_unary()?);
            col_def.constraint_order.push(ConstraintType::Default);
        } else if self.match_keywords(&[TokenType::ForeignKey, TokenType::Key]) {
            self.expect_token(TokenType::References)?;
            let mut fk_ref = self.parse_foreign_key_ref()?;
            fk_ref.has_foreign_key_keywords = true;
            col_def
                .constraints
                .push(ColumnConstraint::References(Box::new(fk_ref)));
            col_def.constraint_order.push(ConstraintType::References);
        } else if self.match_token(TokenType::References) {
            let fk_ref = self.parse_foreign_key_ref()?;
            col_def
                .constraints
                .push(ColumnConstraint::References(Box::new(fk_ref)));
            col_def.constraint_order.push(ConstraintType::References);
        } else if self.match_token(TokenType::Generated) {
            col_def = self.parse_generated_column_constraint(col_def)?;
        } else if self.match_token(TokenType::Collate) {
            col_def = self.parse_column_collate(col_def)?;
        } else if self.match_token(TokenType::Comment) {
            let comment_text = self.expect_string()?;
            col_def
                .constraints
                .push(ColumnConstraint::Comment(comment_text));
            col_def.constraint_order.push(ConstraintType::Comment);
        } else if self.match_keywords(&[TokenType::On, TokenType::Update]) {
            let expr = self.parse_unary()?;
            col_def.on_update = Some(expr);
            col_def.constraint_order.push(ConstraintType::OnUpdate);
        } else if self.match_identifier("VISIBLE") {
            col_def.visible = Some(true);
        } else if self.match_identifier("INVISIBLE") {
            col_def.visible = Some(false);
        } else if self.match_identifier("ENCODE") {
            let encoding = self.expect_identifier_or_keyword()?;
            col_def.encoding = Some(encoding);
            col_def.constraint_order.push(ConstraintType::Encode);
        } else if self.match_token(TokenType::Format) {
            let format_str = self.expect_string()?;
            col_def.format = Some(format_str);
        } else if self.match_identifier("TITLE") {
            let title_str = self.expect_string()?;
            col_def.title = Some(title_str);
        } else if self.match_identifier("INLINE") {
            self.match_identifier("LENGTH");
            let length = self.expect_number()?;
            col_def.inline_length = Some(length as u64);
        } else if self.match_identifier("COMPRESS") {
            col_def = self.parse_column_compress(col_def)?;
        } else if self.match_identifier("CHARACTER") {
            self.match_token(TokenType::Set);
            let charset = self.expect_identifier_or_keyword()?;
            col_def.character_set = Some(charset);
        } else if self.match_identifier("UPPERCASE") {
            col_def.uppercase = true;
        } else if self.match_identifier("CASESPECIFIC") {
            col_def.casespecific = Some(true);
        } else if self.match_text_seq(&["NOT", "FOR", "REPLICATION"]) {
            col_def.not_for_replication = true;
        } else if self.match_token(TokenType::Not) && self.match_identifier("CASESPECIFIC") {
            col_def.casespecific = Some(false);
        } else if self.match_keyword("TAG")
            || self.match_token(TokenType::With) && self.match_keyword("TAG")
        {
            let tags = self.parse_tags()?;
            col_def.constraints.push(ColumnConstraint::Tags(tags));
            col_def.constraint_order.push(ConstraintType::Tags);
        } else if self.match_token(TokenType::As) {
            if self.check(TokenType::LParen) {
                col_def = self.parse_as_computed_column(col_def)?;
            }
        } else if self.match_identifier("CODEC") {
            col_def = self.parse_column_codec(col_def)?;
        } else if self.match_identifier("STATISTICS") {
            self.parse_column_statistics()?;
        } else if self.match_identifier("EPHEMERAL") {
            col_def = self.parse_column_ephemeral(col_def)?;
        } else if self.check(TokenType::Materialized) && !self.check_next(TokenType::View) {
            self.skip();
            let expr = self.parse_or()?;
            col_def.materialized_expr = Some(Box::new(expr));
        } else if self.match_identifier("ALIAS") {
            let expr = self.parse_or()?;
            col_def.alias_expr = Some(Box::new(expr));
        } else {
            return Ok((col_def, false));
        }
        Ok((col_def, true))
    }

    fn parse_column_named_constraint(&mut self, mut col_def: ColumnDef) -> Result<ColumnDef> {
        let constraint_name = self.expect_identifier()?;
        if self.match_token(TokenType::References) {
            let mut fk_ref = self.parse_foreign_key_ref()?;
            fk_ref.constraint_name = Some(constraint_name);
            col_def
                .constraints
                .push(ColumnConstraint::References(Box::new(fk_ref)));
            col_def.constraint_order.push(ConstraintType::References);
        } else if self.match_keywords(&[TokenType::PrimaryKey, TokenType::Key]) {
            col_def.primary_key = true;
            col_def.primary_key_constraint_name = Some(constraint_name);
            col_def.constraint_order.push(ConstraintType::PrimaryKey);
        } else if self.match_token(TokenType::Unique) {
            col_def.unique = true;
            col_def.unique_constraint_name = Some(constraint_name);
            if self.match_text_seq(&["NULLS", "NOT", "DISTINCT"]) {
                col_def.unique_nulls_not_distinct = true;
            }
            col_def.constraint_order.push(ConstraintType::Unique);
        } else if self.match_keywords(&[TokenType::Not, TokenType::Null]) {
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
        }
        Ok(col_def)
    }

    fn parse_column_identity(&mut self, mut col_def: ColumnDef) -> Result<ColumnDef> {
        col_def.auto_increment = true;
        col_def.constraint_order.push(ConstraintType::AutoIncrement);
        if self.match_token(TokenType::LParen) {
            col_def.auto_increment_start = Some(Box::new(self.parse_primary()?));
            if self.match_token(TokenType::Comma) {
                col_def.auto_increment_increment = Some(Box::new(self.parse_primary()?));
            }
            self.expect_token(TokenType::RParen)?;
        }
        loop {
            if self.match_keyword("START") {
                col_def.auto_increment_start = Some(Box::new(self.parse_primary()?));
            } else if self.match_keyword("INCREMENT") {
                col_def.auto_increment_increment = Some(Box::new(self.parse_primary()?));
            } else if self.match_token(TokenType::Order) {
                col_def.auto_increment_order = Some(true);
            } else if self.match_identifier("NOORDER") {
                col_def.auto_increment_order = Some(false);
            } else {
                break;
            }
        }
        Ok(col_def)
    }

    fn parse_column_collate(&mut self, mut col_def: ColumnDef) -> Result<ColumnDef> {
        let mut collation = self.expect_identifier_or_keyword_with_quoted()?;
        while self.match_token(TokenType::Dot) {
            let next = self.expect_identifier_or_keyword_with_quoted()?;
            let sep = if next.quoted {
                format!("{}.\"{}\"", collation.name, next.name)
            } else {
                format!("{}.{}", collation.name, next.name)
            };
            collation = Identifier {
                name: sep,
                quoted: false,
                trailing_comments: Vec::new(),
                span: None,
            };
        }
        col_def
            .constraints
            .push(ColumnConstraint::Collate(collation));
        col_def.constraint_order.push(ConstraintType::Collate);
        Ok(col_def)
    }

    fn parse_column_compress(&mut self, mut col_def: ColumnDef) -> Result<ColumnDef> {
        if self.match_token(TokenType::LParen) {
            let values = self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
            col_def.compress = Some(values);
        } else if self.check(TokenType::String) {
            let value = self.parse_primary()?;
            col_def.compress = Some(vec![value]);
        } else {
            col_def.compress = Some(Vec::new());
        }
        Ok(col_def)
    }

    fn parse_column_codec(&mut self, mut col_def: ColumnDef) -> Result<ColumnDef> {
        self.expect_token(TokenType::LParen)?;
        let start = self.current;
        let mut depth = 1;
        while !self.is_at_end() && depth > 0 {
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
        let codec_text = self.tokens_to_sql(start, self.current);
        self.expect_token(TokenType::RParen)?;
        col_def.codec = Some(codec_text);
        Ok(col_def)
    }

    fn parse_column_statistics(&mut self) -> Result<()> {
        self.expect_token(TokenType::LParen)?;
        let mut depth = 1;
        while !self.is_at_end() && depth > 0 {
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
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }

    fn parse_column_ephemeral(&mut self, mut col_def: ColumnDef) -> Result<ColumnDef> {
        if !self.check(TokenType::Comma)
            && !self.check(TokenType::RParen)
            && !self.is_at_end()
            && !self.check_identifier("CODEC")
            && !self.check_identifier("TTL")
            && !self.check(TokenType::Comment)
        {
            let expr = self.parse_bitwise()?.unwrap_or(Expression::Null(Null));
            col_def.ephemeral = Some(Some(Box::new(expr)));
            if col_def.no_type
                && !self.check(TokenType::Comma)
                && !self.check(TokenType::RParen)
                && !self.is_at_end()
                && !self.check_identifier("CODEC")
                && !self.check_identifier("TTL")
                && !self.check(TokenType::Comment)
            {
                col_def.data_type = self.parse_data_type()?;
                col_def.no_type = false;
            }
        } else {
            col_def.ephemeral = Some(None);
        }
        Ok(col_def)
    }

    /// Skip optional column modifiers that we don't need to preserve
    pub(crate) fn skip_column_modifier(&mut self) -> bool {
        if self.check(TokenType::Not)
            && (self.check_next_identifier("DEFERRABLE")
                || self.check_next_identifier("CASESPECIFIC"))
        {
            self.skip();
            self.skip();
            return true;
        }
        if self.match_identifier("DEFERRABLE")
            || self.match_identifier("DEFERRED")
            || self.match_identifier("IMMEDIATE")
        {
            return true;
        }
        if self.match_identifier("CHARACTER") {
            self.match_token(TokenType::Set);
            let _ = self.match_token(TokenType::Var) || self.match_token(TokenType::Identifier);
            return true;
        }
        if self.match_identifier("UPPERCASE") || self.match_identifier("CASESPECIFIC") {
            return true;
        }
        false
    }

    /// Parses Teradata table options after CREATE TABLE AS.
    pub(crate) fn parse_teradata_table_options(
        &mut self,
    ) -> Result<(Option<bool>, Option<bool>, Vec<TeradataIndex>)> {
        let mut with_data = None;
        let mut with_statistics = None;
        let mut teradata_indexes: Vec<TeradataIndex> = Vec::new();

        loop {
            if self.match_token(TokenType::With) {
                let no = self.match_token(TokenType::No);
                self.match_identifier("DATA");
                with_data = Some(!no);
                if self.match_token(TokenType::And) {
                    let no_stats = self.match_token(TokenType::No);
                    self.match_identifier("STATISTICS");
                    with_statistics = Some(!no_stats);
                }
                continue;
            }
            if self.match_token(TokenType::No) {
                self.match_token(TokenType::PrimaryKey);
                self.match_token(TokenType::Index);
                teradata_indexes.push(TeradataIndex {
                    kind: TeradataIndexKind::NoPrimary,
                    name: None,
                    columns: Vec::new(),
                });
                self.match_token(TokenType::Comma);
                continue;
            }
            if self.match_token(TokenType::PrimaryKey) {
                let is_amp = self.match_identifier("AMP");
                self.match_token(TokenType::Index);
                let name = if self.is_identifier_token() && !self.check(TokenType::LParen) {
                    Some(self.advance_text()?)
                } else {
                    None
                };
                let columns = if self.match_token(TokenType::LParen) {
                    let cols = self.parse_identifier_list_raw();
                    self.match_token(TokenType::RParen);
                    cols
                } else {
                    Vec::new()
                };
                teradata_indexes.push(TeradataIndex {
                    kind: if is_amp {
                        TeradataIndexKind::PrimaryAmp
                    } else {
                        TeradataIndexKind::Primary
                    },
                    name,
                    columns,
                });
                self.match_token(TokenType::Comma);
                continue;
            }
            if self.match_token(TokenType::Unique) {
                let is_primary = self.match_token(TokenType::PrimaryKey);
                self.match_token(TokenType::Index);
                let name = if self.is_identifier_token() {
                    Some(self.advance_text()?)
                } else {
                    None
                };
                let columns = if self.match_token(TokenType::LParen) {
                    let cols = self.parse_identifier_list_raw();
                    self.match_token(TokenType::RParen);
                    cols
                } else {
                    Vec::new()
                };
                teradata_indexes.push(TeradataIndex {
                    kind: if is_primary {
                        TeradataIndexKind::UniquePrimary
                    } else {
                        TeradataIndexKind::Unique
                    },
                    name,
                    columns,
                });
                self.match_token(TokenType::Comma);
                continue;
            }
            if self.match_token(TokenType::Index) {
                let name = if self.is_identifier_token() && !self.check(TokenType::LParen) {
                    Some(self.advance_text()?)
                } else {
                    None
                };
                let columns = if self.match_token(TokenType::LParen) {
                    let cols = self.parse_identifier_list_raw();
                    self.match_token(TokenType::RParen);
                    cols
                } else {
                    Vec::new()
                };
                teradata_indexes.push(TeradataIndex {
                    kind: TeradataIndexKind::Secondary,
                    name,
                    columns,
                });
                self.match_token(TokenType::Comma);
                continue;
            }
            break;
        }

        Ok((with_data, with_statistics, teradata_indexes))
    }

    /// Parse identifier list for Teradata indexes, returning raw strings
    pub(crate) fn parse_identifier_list_raw(&mut self) -> Vec<String> {
        let mut identifiers: Vec<String> = Vec::new();
        loop {
            if self.is_identifier_token() || self.is_identifier_or_keyword_token() {
                let Ok(text) = self.advance_text() else { break };
                identifiers.push(text);
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        identifiers
    }

    /// Parses a GENERATED column constraint: identity, computed or row-start/end forms.
    pub(crate) fn parse_generated_column_constraint(
        &mut self,
        mut col_def: ColumnDef,
    ) -> Result<ColumnDef> {
        let always;
        let mut on_null = false;

        if self.match_token(TokenType::By) {
            self.expect_token(TokenType::Default)?;
            on_null = self.match_keywords(&[TokenType::On, TokenType::Null]);
            always = false;
        } else {
            self.expect_token(TokenType::Always)?;
            always = true;
        }

        self.expect_token(TokenType::As)?;

        if self.check(TokenType::Row) {
            self.skip();
            let start = if self.match_token(TokenType::Start) {
                true
            } else {
                self.expect_token(TokenType::End)?;
                false
            };
            let hidden = self.match_identifier("HIDDEN");
            col_def
                .constraints
                .push(ColumnConstraint::GeneratedAsRow(GeneratedAsRow {
                    start,
                    hidden,
                }));
            col_def
                .constraint_order
                .push(ConstraintType::GeneratedAsRow);
        } else if self.check(TokenType::Identity) {
            self.skip();

            let mut start = None;
            let mut increment = None;
            let mut minvalue = None;
            let mut maxvalue = None;
            let mut cycle = None;

            if self.match_token(TokenType::LParen) {
                loop {
                    if self.match_token(TokenType::Start) {
                        self.match_token(TokenType::With);
                        start = Some(Box::new(self.parse_unary()?));
                    } else if self.match_token(TokenType::Increment) {
                        self.match_token(TokenType::By);
                        increment = Some(Box::new(self.parse_unary()?));
                    } else if self.match_token(TokenType::Minvalue) {
                        minvalue = Some(Box::new(self.parse_unary()?));
                    } else if self.match_token(TokenType::Maxvalue) {
                        maxvalue = Some(Box::new(self.parse_unary()?));
                    } else if self.match_token(TokenType::Cycle) {
                        cycle = Some(true);
                    } else if self.match_keywords(&[TokenType::No, TokenType::Cycle]) {
                        cycle = Some(false);
                    } else if self.check(TokenType::RParen) || self.is_at_end() {
                        break;
                    } else {
                        self.skip();
                    }
                }
                self.expect_token(TokenType::RParen)?;
            }

            col_def
                .constraints
                .push(ColumnConstraint::GeneratedAsIdentity(GeneratedAsIdentity {
                    always,
                    on_null,
                    start,
                    increment,
                    minvalue,
                    maxvalue,
                    cycle,
                }));
            col_def
                .constraint_order
                .push(ConstraintType::GeneratedAsIdentity);
        } else if self.check(TokenType::LParen) {
            self.skip();
            let expr = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;

            let (persisted, persistence_kind) = if self.match_identifier("STORED") {
                (true, Some("STORED".to_string()))
            } else if self.match_identifier("VIRTUAL") {
                (false, Some("VIRTUAL".to_string()))
            } else {
                (false, None)
            };

            col_def
                .constraints
                .push(ColumnConstraint::ComputedColumn(ComputedColumn {
                    expression: Box::new(expr),
                    persisted,
                    not_null: false,
                    persistence_kind,
                    data_type: None,
                }));
            col_def
                .constraint_order
                .push(ConstraintType::ComputedColumn);
        } else {
            col_def
                .constraints
                .push(ColumnConstraint::GeneratedAsIdentity(GeneratedAsIdentity {
                    always,
                    on_null,
                    start: None,
                    increment: None,
                    minvalue: None,
                    maxvalue: None,
                    cycle: None,
                }));
            col_def
                .constraint_order
                .push(ConstraintType::GeneratedAsIdentity);
        }
        Ok(col_def)
    }

    /// Parses a computed column AS (expr) [STORED|VIRTUAL|PERSISTED] [type] [NOT NULL].
    pub(crate) fn parse_as_computed_column(&mut self, mut col_def: ColumnDef) -> Result<ColumnDef> {
        self.expect_token(TokenType::LParen)?;
        let expr = self.parse_expression()?;
        self.expect_token(TokenType::RParen)?;

        let (persisted, persistence_kind) = if self.match_identifier("STORED") {
            (true, Some("STORED".to_string()))
        } else if self.match_identifier("VIRTUAL") {
            (false, Some("VIRTUAL".to_string()))
        } else if self.match_identifier("PERSISTED") {
            (true, Some("PERSISTED".to_string()))
        } else {
            (false, None)
        };

        let data_type = if persistence_kind.as_deref() == Some("PERSISTED") {
            if !self.is_at_end()
                && !self.check(TokenType::Not)
                && !self.check(TokenType::Comma)
                && !self.check(TokenType::RParen)
                && !self.check(TokenType::Semicolon)
            {
                let tok = self.peek();
                if tok.text.eq_ignore_ascii_case("AUTO") {
                    self.skip();
                    None
                } else if tok.token_type.is_keyword()
                    || tok.token_type == TokenType::Identifier
                    || tok.token_type == TokenType::Var
                {
                    Some(self.parse_data_type()?)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let not_null = if persistence_kind.as_deref() == Some("PERSISTED") {
            self.match_keywords(&[TokenType::Not, TokenType::Null])
        } else {
            false
        };

        col_def
            .constraints
            .push(ColumnConstraint::ComputedColumn(ComputedColumn {
                expression: Box::new(expr),
                persisted,
                not_null,
                persistence_kind,
                data_type,
            }));
        col_def
            .constraint_order
            .push(ConstraintType::ComputedColumn);
        Ok(col_def)
    }

    /// Parses PERIOD FOR SYSTEM_TIME (start, end), or `None` for a column named PERIOD.
    pub(crate) fn parse_period_for_system_time_table_constraint(
        &mut self,
    ) -> Result<Option<TableConstraint>> {
        let saved = self.current;

        if self.match_identifier("PERIOD")
            && self.match_token(TokenType::For)
            && self.match_identifier("SYSTEM_TIME")
        {
            self.expect_token(TokenType::LParen)?;
            let start_name = self.expect_identifier_or_safe_keyword_with_quoted()?;
            self.expect_token(TokenType::Comma)?;
            let end_name = self.expect_identifier_or_safe_keyword_with_quoted()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Some(TableConstraint::PeriodForSystemTime {
                start_col: start_name,
                end_col: end_name,
            }));
        }

        self.current = saved;
        Ok(None)
    }

    /// Parses MySQL table options after the column list: ENGINE, CHARSET, COMMENT, ...
    fn advance_quoted_string(&mut self) -> String {
        let quoted = format!("'{}'", self.peek_text());
        self.skip();
        quoted
    }

    pub(crate) fn parse_mysql_table_options(&mut self) -> Result<MySqlTableOptions> {
        let mut options: Vec<(String, String)> = Vec::new();
        let tidb_options: Vec<TiDBTableOption> = Vec::new();
        loop {
            self.match_token(TokenType::Comma);

            if self.check(TokenType::Default) {
                let saved = self.current;
                self.skip();
                if self.check_identifier("CHARSET") || self.check_identifier("CHARACTER") {
                    let is_character = self.check_identifier("CHARACTER");
                    let key_part = self.advance_text()?.to_ascii_uppercase();
                    if is_character {
                        self.match_token(TokenType::Set);
                    }
                    if self.match_token(TokenType::Eq) {
                        let value = if self.check(TokenType::String) {
                            self.advance_quoted_string()
                        } else if self.is_identifier_token()
                            || self.is_safe_keyword_as_identifier()
                            || self.check(TokenType::Number)
                        {
                            self.advance_text()?
                        } else {
                            self.current = saved;
                            break;
                        };
                        let key = if is_character || key_part == CHARSET {
                            "DEFAULT CHARACTER SET".to_string()
                        } else {
                            format!("DEFAULT {}", key_part)
                        };
                        options.push((key, value));
                        continue;
                    }
                }
                self.current = saved;
                break;
            }

            let is_known_option = self.check_identifier("ENGINE")
                || self.check(TokenType::AutoIncrement)
                || self.check_identifier("ROW_FORMAT")
                || self.check(TokenType::Collate)
                || self.check_identifier("KEY_BLOCK_SIZE")
                || self.check_identifier("PACK_KEYS")
                || self.check_identifier("STATS_AUTO_RECALC")
                || self.check_identifier("STATS_PERSISTENT")
                || self.check_identifier("STATS_SAMPLE_PAGES")
                || self.check_identifier("MAX_ROWS")
                || self.check_identifier("MIN_ROWS")
                || self.check_identifier("CHECKSUM")
                || self.check_identifier("DELAY_KEY_WRITE")
                || self.check_identifier("COMPRESSION")
                || self.check_identifier("CONNECTION")
                || self.check_identifier("TABLESPACE")
                || self.check_identifier("ENCRYPTION");

            if is_known_option {
                let key = self.advance_text()?.to_ascii_uppercase();
                if self.match_token(TokenType::Eq) {
                    let value = if self.check(TokenType::String) {
                        self.advance_quoted_string()
                    } else if self.check(TokenType::Number)
                        || self.is_identifier_token()
                        || self.is_safe_keyword_as_identifier()
                    {
                        self.advance_text()?
                    } else {
                        break;
                    };
                    options.push((key, value));
                    continue;
                }
                break;
            }

            if self.check(TokenType::Comment) {
                let saved = self.current;
                self.skip();
                if self.match_token(TokenType::Eq) {
                    if self.check(TokenType::String) {
                        options.push(("COMMENT".to_string(), self.advance_quoted_string()));
                        continue;
                    }
                } else if self.check(TokenType::String) {
                    options.push(("COMMENT".to_string(), self.advance_quoted_string()));
                    continue;
                }
                self.current = saved;
                break;
            }

            if self.check_identifier("CHARACTER") || self.check_identifier("CHARSET") {
                let saved = self.current;
                let is_character = self.check_identifier("CHARACTER");
                self.skip();
                if is_character && !self.match_token(TokenType::Set) {
                    self.current = saved;
                    break;
                }
                if self.match_token(TokenType::Eq) {
                    let value = if self.check(TokenType::String) {
                        self.advance_quoted_string()
                    } else if self.is_identifier_token()
                        || self.is_safe_keyword_as_identifier()
                        || self.check(TokenType::Number)
                    {
                        self.advance_text()?
                    } else {
                        self.current = saved;
                        break;
                    };
                    options.push(("CHARACTER SET".to_string(), value));
                    continue;
                }
                self.current = saved;
                break;
            }

            break;
        }
        Ok((options, tidb_options))
    }

    /// Parses Hive table properties: ROW FORMAT, STORED AS/BY, LOCATION, TBLPROPERTIES.
    pub(crate) fn parse_hive_table_properties(&mut self) -> Result<Vec<Expression>> {
        let mut properties: Vec<Expression> = Vec::new();

        loop {
            if self.check(TokenType::Row) && self.check_next(TokenType::Format) {
                self.skip();
                if let Some(row_format) = self.parse_row()? {
                    properties.push(row_format);
                    continue;
                }
            }

            if self.match_identifier("STORED") {
                if self.match_token(TokenType::By) {
                    let handler = self.parse_string()?.unwrap_or(Expression::Null(Null));
                    properties.push(Expression::StorageHandlerProperty(Box::new(
                        StorageHandlerProperty {
                            this: Box::new(handler),
                        },
                    )));
                    continue;
                } else if self.match_token(TokenType::As) {
                    if self.match_token(TokenType::InputFormat) {
                        let input_format = self.parse_string()?;
                        let output_format = if self.match_identifier("OUTPUTFORMAT") {
                            self.parse_string()?
                        } else {
                            None
                        };
                        let io_format =
                            Expression::InputOutputFormat(Box::new(InputOutputFormat {
                                input_format: input_format.map(Box::new),
                                output_format: output_format.map(Box::new),
                            }));
                        properties.push(Expression::FileFormatProperty(Box::new(
                            FileFormatProperty {
                                this: Some(Box::new(io_format)),
                                expressions: vec![],
                                hive_format: Some(Box::new(Expression::Boolean(BooleanLiteral {
                                    value: true,
                                }))),
                            },
                        )));
                        continue;
                    } else {
                        let format = if self.check(TokenType::String) {
                            Expression::Literal(Box::new(Literal::String(self.advance_text()?)))
                        } else if self.is_identifier_token() || self.is_safe_keyword_as_identifier()
                        {
                            Expression::Identifier(identifier_from_token(self.advance()?, false))
                        } else {
                            break;
                        };
                        properties.push(Expression::FileFormatProperty(Box::new(
                            FileFormatProperty {
                                this: Some(Box::new(format)),
                                expressions: vec![],
                                hive_format: Some(Box::new(Expression::Boolean(BooleanLiteral {
                                    value: true,
                                }))),
                            },
                        )));
                        continue;
                    }
                }
            }

            if self.match_token(TokenType::Using) {
                let format = if self.check(TokenType::String) {
                    Expression::Literal(Box::new(Literal::String(self.advance_text()?)))
                } else if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                    Expression::Identifier(identifier_from_token(self.advance()?, false))
                } else {
                    break;
                };
                properties.push(Expression::FileFormatProperty(Box::new(
                    FileFormatProperty {
                        this: Some(Box::new(format)),
                        expressions: vec![],
                        hive_format: None,
                    },
                )));
                continue;
            }

            if self.match_identifier("LOCATION") {
                let path = self.parse_string()?.unwrap_or(Expression::Null(Null));
                properties.push(Expression::LocationProperty(Box::new(LocationProperty {
                    this: Box::new(path),
                })));
                continue;
            }

            if self.match_identifier("TBLPROPERTIES") {
                properties = self.parse_hive_tblproperties(properties)?;
                continue;
            }

            if self.match_identifier("DISTRIBUTED") {
                if let Some(dist_prop) = self.parse_distributed_property()? {
                    properties.push(dist_prop);
                    continue;
                }
            }

            if self.match_identifier("CLUSTERED") {
                properties = self.parse_hive_clustered_by(properties)?;
                continue;
            }

            if self.match_identifier("PARTITIONED") {
                properties = self.parse_hive_partitioned_by(properties)?;
                continue;
            }

            break;
        }

        Ok(properties)
    }

    fn parse_hive_tblproperties(
        &mut self,
        mut properties: Vec<Expression>,
    ) -> Result<Vec<Expression>> {
        self.expect_token(TokenType::LParen)?;
        let mut prop_exprs: Vec<Expression> = Vec::new();
        loop {
            if self.check(TokenType::RParen) {
                break;
            }
            let key = self.parse_primary()?;
            if self.match_token(TokenType::Eq) {
                let value = self.parse_primary()?;
                prop_exprs.push(Expression::Eq(Box::new(BinaryOp::new(key, value))));
            } else {
                prop_exprs.push(key);
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        properties.push(Expression::Properties(Box::new(Properties {
            expressions: prop_exprs,
        })));
        Ok(properties)
    }

    fn parse_hive_clustered_by(
        &mut self,
        mut properties: Vec<Expression>,
    ) -> Result<Vec<Expression>> {
        self.expect_token(TokenType::By)?;
        self.expect_token(TokenType::LParen)?;
        let expressions = self.parse_expression_list()?;
        self.expect_token(TokenType::RParen)?;

        let sorted_by = if self.match_identifier("SORTED") {
            self.expect_token(TokenType::By)?;
            self.expect_token(TokenType::LParen)?;
            let sorted_exprs = self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
            Some(Box::new(Expression::Tuple(Box::new(Tuple {
                expressions: sorted_exprs,
            }))))
        } else {
            None
        };

        let buckets = if self.match_token(TokenType::Into) {
            let num = self.parse_expression()?;
            if !self.match_identifier("BUCKETS") {
                return Err(self.parse_error("Expected BUCKETS after INTO <n>"));
            }
            Some(Box::new(num))
        } else {
            None
        };

        properties.push(Expression::ClusteredByProperty(Box::new(
            ClusteredByProperty {
                expressions,
                sorted_by,
                buckets,
            },
        )));
        Ok(properties)
    }

    fn parse_hive_partitioned_by(
        &mut self,
        mut properties: Vec<Expression>,
    ) -> Result<Vec<Expression>> {
        self.expect_token(TokenType::By)?;
        self.expect_token(TokenType::LParen)?;

        let mut partition_exprs: Vec<Expression> = Vec::new();
        loop {
            if self.check(TokenType::RParen) {
                break;
            }

            if self.check_identifier("BUCKET") || self.check_identifier("TRUNCATE") {
                let func_name = self.advance_text()?;
                self.expect_token(TokenType::LParen)?;
                let args = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;

                partition_exprs.push(Expression::Function(Box::new(Function {
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
                    span: None,
                    inferred_type: None,
                })));
            } else {
                let saved_pos = self.current;
                let mut parsed_as_column = false;
                if self.check(TokenType::Var)
                    || self.check(TokenType::Identifier)
                    || self.check(TokenType::Date)
                    || self.check(TokenType::Timestamp)
                    || self.check(TokenType::Int)
                    || self.check(TokenType::BigInt)
                    || self.check(TokenType::SmallInt)
                    || self.check(TokenType::TinyInt)
                    || self.check(TokenType::Float)
                    || self.check(TokenType::Double)
                    || self.check(TokenType::Boolean)
                {
                    let col_name = self.advance_text()?;
                    if self.check(TokenType::Var)
                        || self.check(TokenType::Identifier)
                        || self.check(TokenType::Int)
                        || self.check(TokenType::BigInt)
                        || self.check(TokenType::SmallInt)
                        || self.check(TokenType::TinyInt)
                        || self.check(TokenType::Float)
                        || self.check(TokenType::Double)
                        || self.check(TokenType::Boolean)
                        || self.check(TokenType::Date)
                        || self.check(TokenType::Timestamp)
                    {
                        let type_text = self.peek_text().to_ascii_uppercase();
                        let is_type = matches!(
                            type_text.as_str(),
                            "INT"
                                | "INTEGER"
                                | "BIGINT"
                                | "SMALLINT"
                                | "TINYINT"
                                | "FLOAT"
                                | "DOUBLE"
                                | "DECIMAL"
                                | "NUMERIC"
                                | "STRING"
                                | "VARCHAR"
                                | "CHAR"
                                | "BINARY"
                                | "BOOLEAN"
                                | "DATE"
                                | "TIMESTAMP"
                                | "DATETIME"
                                | "ARRAY"
                                | "MAP"
                                | "STRUCT"
                        );
                        if is_type {
                            let data_type = self.parse_data_type()?;
                            partition_exprs.push(Expression::ColumnDef(Box::new(
                                crate::expressions::columns::models::ColumnDef::new(
                                    col_name, data_type,
                                ),
                            )));
                            parsed_as_column = true;
                        }
                    }
                }
                if !parsed_as_column {
                    self.current = saved_pos;
                    partition_exprs.push(self.parse_expression()?);
                }
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;

        properties.push(Expression::PartitionedByProperty(Box::new(
            PartitionedByProperty {
                this: Box::new(Expression::Tuple(Box::new(Tuple {
                    expressions: partition_exprs,
                }))),
            },
        )));
        Ok(properties)
    }

    pub(crate) fn parse_snowflake_row_access_policy_clause(&mut self) -> Option<String> {
        let saved = self.current;
        let _ = self.match_token(TokenType::With);
        if !self.match_text_seq(&["ROW", "ACCESS", "POLICY"]) {
            self.current = saved;
            return None;
        }

        let body_start = self.current;
        let mut depth = 0usize;
        while !self.is_at_end() {
            if depth == 0
                && (self.check(TokenType::As)
                    || self.check(TokenType::Comment)
                    || self.check(TokenType::Semicolon)
                    || self.check_identifier("TAG")
                    || self.check_identifier("OPTIONS")
                    || self.check_identifier("BUILD")
                    || self.check_text_seq(&["AUTO", "REFRESH"])
                    || self.check_text_seq(&["COPY", "GRANTS"])
                    || self.check_identifier("SECURITY")
                    || self.check(TokenType::Refresh))
            {
                break;
            }

            if self.check(TokenType::LParen) {
                depth += 1;
            } else if self.check(TokenType::RParen) && depth > 0 {
                depth -= 1;
            }
            self.skip();
        }

        let body = self.tokens_to_sql(body_start, self.current);
        Some(if body.is_empty() {
            "ROW ACCESS POLICY".to_string()
        } else {
            format!("ROW ACCESS POLICY {}", body)
        })
    }

    /// Parses table properties after the column list, currently TSQL SYSTEM_VERSIONING.
    pub(crate) fn parse_post_table_properties(&mut self) -> Result<Vec<Expression>> {
        let mut properties: Vec<Expression> = Vec::new();

        if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        ) {
            if let Some(clause) = self.parse_snowflake_row_access_policy_clause() {
                properties.push(Expression::Raw(Raw {
                    sql: format!("WITH {}", clause),
                }));
            }
        }

        if self.check(TokenType::With) {
            let saved = self.current;
            if self.match_token(TokenType::With) {
                if self.match_token(TokenType::LParen) {
                    if self.check_identifier("SYSTEM_VERSIONING") {
                        self.skip();
                        self.expect_token(TokenType::Eq)?;

                        let on = if self.match_token(TokenType::On) {
                            true
                        } else if self.match_identifier("OFF") {
                            false
                        } else {
                            return Err(
                                self.parse_error("Expected ON or OFF after SYSTEM_VERSIONING=")
                            );
                        };

                        let mut history_table = None;
                        let mut data_consistency = None;

                        if on && self.match_token(TokenType::LParen) {
                            loop {
                                if self.check(TokenType::RParen) || self.is_at_end() {
                                    break;
                                }
                                if self.match_identifier("HISTORY_TABLE") {
                                    self.expect_token(TokenType::Eq)?;
                                    let table_ref = self.parse_table_ref()?;
                                    history_table = Some(Expression::Table(Box::new(table_ref)));
                                } else if self.match_identifier("DATA_CONSISTENCY_CHECK") {
                                    self.expect_token(TokenType::Eq)?;
                                    let val = self.expect_identifier_or_keyword()?;
                                    data_consistency = Some(Expression::Identifier(
                                        crate::expressions::names::models::Identifier::new(val),
                                    ));
                                } else if self.check(TokenType::RParen) {
                                    break;
                                } else {
                                    self.skip();
                                }
                                self.match_token(TokenType::Comma);
                            }
                            self.expect_token(TokenType::RParen)?;
                        }

                        self.expect_token(TokenType::RParen)?;

                        properties.push(Expression::WithSystemVersioningProperty(Box::new(
                            WithSystemVersioningProperty {
                                on: if on {
                                    Some(Box::new(Expression::Boolean(
                                        crate::expressions::names::models::BooleanLiteral {
                                            value: true,
                                        },
                                    )))
                                } else {
                                    None
                                },
                                this: history_table.map(Box::new),
                                data_consistency: data_consistency.map(Box::new),
                                retention_period: None,
                                with_: Some(Box::new(Expression::Boolean(
                                    crate::expressions::names::models::BooleanLiteral {
                                        value: true,
                                    },
                                ))),
                            },
                        )));
                    } else {
                        self.current = saved;
                    }
                } else {
                    self.current = saved;
                }
            }
        }

        Ok(properties)
    }

    /// Parses the columns of UNIQUE KEY (cols) or DUPLICATE KEY (cols).
    pub(crate) fn parse_composite_key_expressions(&mut self) -> Result<Vec<Expression>> {
        self.expect_token(TokenType::LParen)?;
        let mut expressions: Vec<Expression> = Vec::new();
        loop {
            if let Some(id) = self.parse_id_var()? {
                expressions.push(id);
            } else {
                break;
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(expressions)
    }

    /// Parse a table-level constraint
    pub(crate) fn parse_table_constraint(&mut self) -> Result<TableConstraint> {
        let name = if self.match_token(TokenType::Constraint) {
            Some(self.expect_identifier_or_safe_keyword_with_quoted()?)
        } else {
            None
        };

        self.parse_constraint_definition(name)
    }
}
