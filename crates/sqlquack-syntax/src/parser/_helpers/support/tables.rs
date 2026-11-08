//! Parser support: tables.

use crate::expressions::columns::models::UniqueColumnConstraint;
use crate::expressions::functions::models::XMLElement;
use crate::expressions::functions::models::XMLTable;
use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::names::models::Null;
use crate::expressions::names::models::TableRef;
use crate::expressions::operators::models::ArraySlice;
use crate::expressions::operators::models::Subscript;
use crate::expressions::queries::models::Ordered;
use crate::expressions::queries::models::WindowSpec;
use crate::expressions::statements::models::Command;
use crate::expressions::statements::models::UnpivotColumns;
use crate::expressions::statements::models::Var;
use crate::expressions::statements::models::VerticaMatch;
use crate::expressions::statements::models::VerticaTimeseries;
use crate::expressions::tables::models::Property;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::Tuple;
use crate::parser::constants::COLUMNS;
use crate::parser::constants::RETURNING;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parses `catalog.schema.table`, `schema.table` or `table`.
    pub(crate) fn parse_table_parts(&mut self) -> Result<Option<Expression>> {
        let first = self.parse_table_part()?;
        if first.is_none() {
            return Ok(None);
        }

        let mut parts = vec![first.ok_or_else(|| {
            crate::tokens::errors::Error::internal("a value the parser checked is missing")
        })?];

        while self.match_token(TokenType::Dot) {
            if let Some(part) = self.parse_table_part()? {
                parts.push(part);
            } else {
                break;
            }
        }

        let (catalog, schema, name) = match parts.len() {
            1 => (
                None,
                None,
                parts.pop().ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?,
            ),
            2 => {
                let table = parts.pop().ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?;
                let schema = parts.pop().ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?;
                (None, Some(schema), table)
            }
            _ => {
                let table = parts.pop().ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?;
                let schema = parts.pop().ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?;
                let catalog = parts.pop();
                (catalog, Some(schema), table)
            }
        };

        let name_ident = match name {
            Expression::Identifier(id) => id,
            _ => Identifier::new(String::new()),
        };
        let schema_ident = schema.map(|s| match s {
            Expression::Identifier(id) => id,
            _ => Identifier::new(String::new()),
        });
        let catalog_ident = catalog.map(|c| match c {
            Expression::Identifier(id) => id,
            _ => Identifier::new(String::new()),
        });

        Ok(Some(Expression::boxed_table(TableRef {
            sample_call: None,
            name: name_ident,
            schema: schema_ident,
            catalog: catalog_ident,
            alias: None,
            alias_explicit_as: false,
            column_aliases: Vec::new(),
            leading_comments: Vec::new(),
            trailing_comments: Vec::new(),
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
            whole_quoted: false,
            catalog_parts: Vec::new(),
        })))
    }

    /// Parses types.
    pub(crate) fn parse_types(&mut self) -> Result<Option<Expression>> {
        if self.match_text_seq(&["SYSUDTLIB", "."]) {
            return Ok(Some(Expression::Identifier(Identifier {
                name: String::new(),
                quoted: false,
                trailing_comments: Vec::new(),
                span: None,
            })));
        }
        if self.match_text_seq(&["WITH", "TIME", "ZONE"]) {
            return Ok(None);
        }
        if self.match_text_seq(&["WITH", "LOCAL", "TIME", "ZONE"]) {
            return Ok(None);
        }
        Ok(None)
    }

    /// Parses UNIQUE [KEY|INDEX] [NULLS NOT DISTINCT] [(columns)] [USING type].
    pub(crate) fn parse_unique(&mut self) -> Result<Option<Expression>> {
        let _ = self.match_texts(&["KEY", "INDEX"]);

        let nulls = if self.match_text_seq(&["NULLS", "NOT", "DISTINCT"]) {
            Some(Box::new(Expression::Boolean(BooleanLiteral {
                value: true,
            })))
        } else {
            None
        };

        let unique_key = self.parse_unique_key()?;
        let this = self.parse_schema_with_this(unique_key)?;

        let index_type = if self.match_token(TokenType::Using) {
            self.skip();
            Some(Box::new(Expression::Var(Box::new(Var {
                this: self.previous_text().to_string(),
            }))))
        } else {
            None
        };

        Ok(Some(Expression::UniqueColumnConstraint(Box::new(
            UniqueColumnConstraint {
                this: this.map(Box::new),
                index_type,
                on_conflict: None,
                nulls,
                options: Vec::new(),
            },
        ))))
    }

    /// Parses a UNIQUE constraint's key name.
    pub(crate) fn parse_unique_key(&mut self) -> Result<Option<Expression>> {
        self.parse_id_var()
    }

    /// Parses UNPIVOT INTO NAME column VALUE col, ...
    pub(crate) fn parse_unpivot_columns(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::Into) {
            return Ok(None);
        }

        let this = if self.match_text_seq(&["NAME"]) {
            self.parse_column()?
        } else {
            None
        };

        let expressions = if self.match_text_seq(&["VALUE"]) {
            let mut cols: Vec<Expression> = Vec::new();
            loop {
                if let Some(col) = self.parse_column()? {
                    cols.push(col);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            cols
        } else {
            Vec::new()
        };

        if this.is_some() || !expressions.is_empty() {
            Ok(Some(Expression::UnpivotColumns(Box::new(UnpivotColumns {
                this: Box::new(this.unwrap_or(Expression::Null(Null))),
                expressions,
            }))))
        } else {
            Ok(None)
        }
    }

    /// Parses an INSERT VALUES tuple, or a single value.
    pub(crate) fn parse_value(&mut self) -> Result<Option<Expression>> {
        if self.match_token(TokenType::LParen) {
            let mut expressions: Vec<Expression> = Vec::new();

            if !self.check(TokenType::RParen) {
                loop {
                    if self.match_texts(&["DEFAULT"]) {
                        let text = self.previous_text().to_ascii_uppercase();
                        expressions.push(Expression::Var(Box::new(Var { this: text })));
                    } else {
                        let saved_pos = self.current;
                        match self.parse_expression() {
                            Ok(expr) => expressions.push(expr),
                            Err(_) => {
                                self.current = saved_pos;
                            }
                        }
                    }

                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }

            self.match_token(TokenType::RParen);
            return Ok(Some(Expression::Tuple(Box::new(Tuple { expressions }))));
        }

        let saved_pos = self.current;
        match self.parse_expression() {
            Ok(expr) => {
                return Ok(Some(Expression::Tuple(Box::new(Tuple {
                    expressions: vec![expr],
                }))));
            }
            Err(_) => {
                self.current = saved_pos;
            }
        }

        Ok(None)
    }

    /// Parses a variable reference (unquoted identifier).
    pub(crate) fn parse_var(&mut self) -> Result<Option<Expression>> {
        if self.match_token(TokenType::Var) {
            let text = self.previous_text().to_string();
            return Ok(Some(Expression::Var(Box::new(Var { this: text }))));
        }
        self.parse_placeholder()
    }

    /// Parse window spec inner (without parentheses)
    pub(crate) fn parse_window_spec_inner(&mut self) -> Result<Option<Expression>> {
        let _base = if (self.check(TokenType::Identifier)
            || self.check(TokenType::QuotedIdentifier))
            && !self.check(TokenType::Partition)
            && !self.check(TokenType::Order)
            && !self.check(TokenType::Distribute)
            && !self.check(TokenType::Sort)
        {
            self.parse_identifier()?
        } else {
            None
        };

        let partition_by = if self.match_keywords(&[TokenType::Partition, TokenType::By])
            || self.match_keywords(&[TokenType::Distribute, TokenType::By])
        {
            self.parse_expression_list()?
        } else {
            Vec::new()
        };

        let order_by = if self.match_token(TokenType::Order) || self.match_token(TokenType::Sort) {
            self.match_token(TokenType::By);
            let mut orders: Vec<Ordered> = Vec::new();
            loop {
                if let Some(ordered) = self.parse_ordered_item()? {
                    orders.push(ordered);
                } else {
                    break;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            orders
        } else {
            Vec::new()
        };

        let frame = self.parse_window_frame()?;

        Ok(Some(Expression::WindowSpec(Box::new(WindowSpec {
            partition_by,
            order_by,
            frame,
        }))))
    }

    /// Parses an expression in parentheses.
    pub(crate) fn parse_wrapped(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::LParen) {
            return Ok(None);
        }

        let result = self.parse_disjunction()?;
        self.match_token(TokenType::RParen);

        Ok(result)
    }

    /// Parses comma-separated identifiers in parentheses.
    pub(crate) fn parse_wrapped_id_vars(&mut self) -> Result<Option<Expression>> {
        if !self.match_token(TokenType::LParen) {
            return Ok(None);
        }

        let mut identifiers: Vec<Expression> = Vec::new();
        loop {
            if let Some(id) = self.parse_id_var()? {
                identifiers.push(id);
            } else {
                break;
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        self.match_token(TokenType::RParen);

        if identifiers.is_empty() {
            return Ok(None);
        }

        Ok(Some(Expression::Tuple(Box::new(Tuple {
            expressions: identifiers,
        }))))
    }

    /// Parses space-separated KEY=VALUE options in parentheses, as Snowflake STAGE_FILE_FORMAT.
    pub(crate) fn parse_wrapped_options(&mut self) -> Result<Option<Expression>> {
        self.match_token(TokenType::Eq);

        if !self.match_token(TokenType::LParen) {
            return Ok(None);
        }

        let mut properties: Vec<Expression> = Vec::new();
        while !self.check(TokenType::RParen) && !self.is_at_end() {
            if let Some(prop) = self.parse_option_property()? {
                properties.push(prop);
            } else {
                break;
            }
        }

        self.match_token(TokenType::RParen);

        if properties.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Expression::Tuple(Box::new(Tuple {
                expressions: properties,
            }))))
        }
    }

    /// Parses one KEY=VALUE option; values may be names, literals or parenthesized lists.
    pub(crate) fn parse_option_property(&mut self) -> Result<Option<Expression>> {
        let index = self.current;

        let key = if self.check(TokenType::Identifier)
            || self.check(TokenType::Var)
            || self
                .peek()
                .text
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            let name = self.peek_text().to_string();
            self.skip();
            Some(Expression::Var(Box::new(Var { this: name })))
        } else {
            None
        };

        let key = match key {
            Some(k) => k,
            None => {
                self.current = index;
                return Ok(None);
            }
        };

        if !self.match_token(TokenType::Eq) {
            self.current = index;
            return Ok(None);
        }

        let value = if self.check(TokenType::LParen) {
            self.skip();
            let mut inner_exprs: Vec<Expression> = Vec::new();
            while !self.check(TokenType::RParen) && !self.is_at_end() {
                let start = self.current;
                if let Some(expr) = self.parse_primary_for_option()? {
                    inner_exprs.push(expr);
                }
                self.match_token(TokenType::Comma);
                if self.current == start {
                    break;
                }
            }
            self.match_token(TokenType::RParen);
            Expression::Tuple(Box::new(Tuple {
                expressions: inner_exprs,
            }))
        } else if let Some(primary) = self.parse_primary_for_option()? {
            primary
        } else {
            let text = self.peek_text().to_string();
            self.skip();
            Expression::Var(Box::new(Var { this: text }))
        };

        Ok(Some(Expression::Property(Box::new(Property {
            this: Box::new(key),
            value: Some(Box::new(value)),
        }))))
    }

    /// Parses an option value: a string, number, name, TRUE or FALSE.
    pub(crate) fn parse_primary_for_option(&mut self) -> Result<Option<Expression>> {
        if self.check(TokenType::String) {
            let text = self.peek_text().to_string();
            self.skip();
            return Ok(Some(Expression::Literal(Box::new(Literal::String(text)))));
        }

        if self.check(TokenType::Number) {
            let text = self.peek_text().to_string();
            self.skip();
            return Ok(Some(Expression::Literal(Box::new(Literal::Number(text)))));
        }

        if self.check(TokenType::True) {
            self.skip();
            return Ok(Some(Expression::Boolean(BooleanLiteral { value: true })));
        }
        if self.check(TokenType::False) {
            self.skip();
            return Ok(Some(Expression::Boolean(BooleanLiteral { value: false })));
        }

        if self.check(TokenType::Identifier)
            || self.check(TokenType::Var)
            || !self.check(TokenType::RParen)
                && !self.check(TokenType::Comma)
                && !self.check(TokenType::Eq)
                && !self.is_at_end()
        {
            let text = self.peek_text().to_string();
            if self.check(TokenType::RParen) {
                return Ok(None);
            }
            if self.check_next(TokenType::Eq) {
                return Ok(None);
            }
            self.skip();
            return Ok(Some(Expression::Var(Box::new(Var { this: text }))));
        }

        Ok(None)
    }

    /// Parses a BigQuery OPTIONS (key = value, ...) list.
    pub(crate) fn parse_options_list(&mut self) -> Result<Vec<Expression>> {
        if !self.match_token(TokenType::LParen) {
            return Ok(Vec::new());
        }

        let mut options: Vec<Expression> = Vec::new();
        loop {
            if self.check(TokenType::RParen) {
                break;
            }

            if let Some(opt) = self.parse_assignment()? {
                options.push(opt);
            } else {
                break;
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        self.expect_token(TokenType::RParen)?;

        Ok(options)
    }

    /// Parses a Databricks ENVIRONMENT (key = 'value', ...) list.
    pub(crate) fn parse_environment_list(&mut self) -> Result<Vec<Expression>> {
        if !self.match_token(TokenType::LParen) {
            return Ok(Vec::new());
        }

        let mut env_items: Vec<Expression> = Vec::new();
        loop {
            if self.check(TokenType::RParen) {
                break;
            }

            if let Some(opt) = self.parse_assignment()? {
                env_items.push(opt);
            } else {
                break;
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        self.expect_token(TokenType::RParen)?;

        Ok(env_items)
    }

    /// Parses XMLELEMENT(NAME name [, expr ...]) or XMLELEMENT(EVALNAME expr ...).
    pub(crate) fn parse_xml_element(&mut self) -> Result<Option<Expression>> {
        let (this, evalname) = if self.match_text_seq(&["EVALNAME"]) {
            let expr = self.parse_bitwise()?;
            (
                expr,
                Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: true,
                }))),
            )
        } else {
            self.match_text_seq(&["NAME"]);
            let id = self.parse_id_var()?;
            (id, None)
        };

        let expressions = if self.match_token(TokenType::Comma) {
            self.parse_expression_list()?
        } else {
            Vec::new()
        };

        match this {
            Some(t) => Ok(Some(Expression::XMLElement(Box::new(XMLElement {
                this: Box::new(t),
                expressions,
                evalname,
            })))),
            None => Ok(None),
        }
    }

    /// Parses XML namespace declarations.
    pub(crate) fn parse_xml_namespace(&mut self) -> Result<Option<Expression>> {
        let mut namespaces: Vec<Expression> = Vec::new();

        loop {
            let is_default = self.match_text_seq(&["DEFAULT"]);

            let uri = if is_default {
                self.parse_string()?
            } else {
                let uri_expr = self.parse_string()?;
                if let Some(u) = uri_expr {
                    self.parse_alias_with_expr(Some(u))?
                } else {
                    None
                }
            };

            if let Some(u) = uri {
                namespaces.push(u);
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        if namespaces.is_empty() {
            return Ok(None);
        }

        Ok(Some(Expression::Tuple(Box::new(Tuple {
            expressions: namespaces,
        }))))
    }

    /// Parses XMLTABLE(xpath PASSING doc COLUMNS ...).
    pub(crate) fn parse_xml_table(&mut self) -> Result<Option<Expression>> {
        let namespaces = if self.match_text_seq(&["XMLNAMESPACES", "("]) {
            let ns = self.parse_xml_namespace()?;
            self.match_text_seq(&[")", ","]);
            ns.map(Box::new)
        } else {
            None
        };

        let this = self.parse_string()?;
        if this.is_none() {
            return Ok(None);
        }

        let passing = if self.match_text_seq(&["PASSING"]) {
            self.match_text_seq(&["BY", "VALUE"]);
            let mut cols: Vec<Expression> = Vec::new();
            loop {
                if !self.is_at_end() {
                    let next_text = self.peek_text().to_ascii_uppercase();
                    if next_text == COLUMNS || next_text == RETURNING {
                        break;
                    }
                    if self.check(TokenType::RParen) {
                        break;
                    }
                }
                if let Some(col) = self.parse_assignment()? {
                    cols.push(col);
                } else {
                    break;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            if cols.is_empty() {
                None
            } else {
                Some(Box::new(Expression::Tuple(Box::new(Tuple {
                    expressions: cols,
                }))))
            }
        } else {
            None
        };

        let by_ref = if self.match_text_seq(&["RETURNING", "SEQUENCE", "BY", "REF"]) {
            Some(Box::new(Expression::Boolean(BooleanLiteral {
                value: true,
            })))
        } else {
            None
        };

        let columns = if self.match_text_seq(&["COLUMNS"]) {
            let mut cols: Vec<Expression> = Vec::new();
            loop {
                if self.check(TokenType::RParen) {
                    break;
                }
                if self.match_token(TokenType::Comma) {
                    continue;
                }
                if let Some(col_def) = self.parse_field_def()? {
                    cols.push(col_def);
                } else {
                    break;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            cols
        } else {
            Vec::new()
        };

        Ok(Some(Expression::XMLTable(Box::new(XMLTable {
            this: Box::new(this.ok_or_else(|| {
                crate::tokens::errors::Error::internal("a value the parser checked is missing")
            })?),
            namespaces,
            passing,
            columns,
            by_ref,
        }))))
    }

    /// Parses UNLOAD (SELECT ...) TO 'location' WITH (options) (Athena, Presto, Redshift).
    pub(crate) fn parse_unload(&mut self) -> Result<Expression> {
        let mut parts: Vec<String> = Vec::new();
        parts.push(self.advance_text()?);
        parts.push(" ".to_string());

        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let token_type = self.peek().token_type;
            let token_text = self.peek_text().to_string();

            if token_type == TokenType::String {
                parts.push(format!("'{}'", token_text.replace('\'', "''")));
                self.skip();
                if !self.is_at_end() {
                    let next_type = self.peek().token_type;
                    if !matches!(
                        next_type,
                        TokenType::Comma | TokenType::RParen | TokenType::Semicolon
                    ) {
                        parts.push(" ".to_string());
                    }
                }
                continue;
            }

            if token_text.eq_ignore_ascii_case("ARRAY")
                && self
                    .peek_nth(1)
                    .is_some_and(|t| t.token_type == TokenType::LBracket)
            {
                parts.push(token_text);
                self.skip();
                parts.push("[".to_string());
                self.skip();
                while !self.is_at_end() && !self.check(TokenType::RBracket) {
                    let inner_type = self.peek().token_type;
                    let inner_text = self.peek_text().to_string();
                    if inner_type == TokenType::String {
                        parts.push(format!("'{}'", inner_text.replace('\'', "''")));
                    } else {
                        parts.push(inner_text);
                    }
                    self.skip();
                    if self.check(TokenType::Comma) {
                        parts.push(", ".to_string());
                        self.skip();
                    }
                }
                if self.check(TokenType::RBracket) {
                    parts.push("]".to_string());
                    self.skip();
                }
                continue;
            }

            parts.push(token_text);
            self.skip();

            if !self.is_at_end() {
                let next_type = self.peek().token_type;
                let no_space_before = matches!(
                    next_type,
                    TokenType::Comma
                        | TokenType::RParen
                        | TokenType::RBracket
                        | TokenType::Semicolon
                        | TokenType::LBracket
                );
                let no_space_after = matches!(token_type, TokenType::LParen | TokenType::LBracket);
                if !no_space_before && !no_space_after {
                    parts.push(" ".to_string());
                }
            }
        }

        Ok(Expression::Command(Box::new(Command {
            this: parts.join(""),
        })))
    }

    /// Parses Athena USING EXTERNAL FUNCTION name(params) RETURNS type LAMBDA 'arn' SELECT ...
    pub(crate) fn parse_using_external_function(&mut self) -> Result<Expression> {
        let start_pos = self.peek().span.start;

        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            self.skip();
        }

        let end_pos = if self.current > 0 {
            self.tokens[self.current - 1].span.end
        } else {
            start_pos
        };

        let command_text = if self.source.is_some() {
            self.source_text_range(start_pos, end_pos)
        } else {
            let mut parts: Vec<String> = Vec::new();
            for i in 0..self.current {
                if self.tokens[i].span.start >= start_pos && self.tokens[i].span.end <= end_pos {
                    if self.tokens[i].token_type == TokenType::String {
                        parts.push(format!("'{}'", self.tokens[i].text.replace('\'', "''")));
                    } else {
                        parts.push(self.tokens[i].text.to_string());
                    }
                    if i + 1 < self.current {
                        parts.push(" ".to_string());
                    }
                }
            }
            parts.join("")
        };

        Ok(Expression::Command(Box::new(Command {
            this: command_text,
        })))
    }
}

impl Parser {
    pub(crate) fn parse_vertica_timeseries(&mut self) -> Result<Option<VerticaTimeseries>> {
        Ok(None)
    }

    pub(crate) fn parse_vertica_match(&mut self) -> Result<Option<VerticaMatch>> {
        Ok(None)
    }

    pub(crate) fn make_subscript(&self, this: Expression, index: Expression) -> Expression {
        {
            Expression::Subscript(Box::new(Subscript { this, index }))
        }
    }

    pub(crate) fn make_array_slice(
        &self,
        this: Expression,
        start: Option<Expression>,
        end: Option<Expression>,
    ) -> Expression {
        {
            Expression::ArraySlice(Box::new(ArraySlice { this, start, end }))
        }
    }

    pub(crate) fn parse_vertica_data_type(&mut self) -> Result<Option<DataType>> {
        Ok(None)
    }
}
