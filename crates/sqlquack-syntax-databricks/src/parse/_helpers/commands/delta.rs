//! Databricks' table maintenance and loading statements, such as `COPY INTO` and `VACUUM`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, TableRef};
use sqlquack_syntax::expressions::queries::models::{From, Select};
use sqlquack_syntax::expressions::statements::models::{
    CopyParameter, CopyStmt, Credentials, LoadData,
};
use sqlquack_syntax::expressions::tables::models::Undrop;

use crate::parse::_helpers::commands::alters::property_text;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::{
    literal, null, property, subquery, too_deep_to_keep, utility, var,
};
use crate::parse::constants::{MSCK, UNDROP_WITH_ID};
use crate::scan::models::{NumberKind, TokenKind};

fn parameter(
    name: &str,
    value: Option<Expression>,
    values: Vec<Expression>,
    eq: bool,
) -> CopyParameter {
    CopyParameter {
        name: name.to_owned(),
        value,
        values,
        eq,
    }
}

fn is_integer(token: &crate::scan::models::Token) -> bool {
    token.kind == TokenKind::Number(NumberKind::Integer)
}

impl Parser<'_> {
    /// `propertyList` whose every key has a value, as `key = value` equalities.
    fn option_list(&mut self) -> Parsing<Vec<Expression>> {
        let at: usize = self.offset();
        let options: Vec<Expression> = self.property_list()?;
        if options
            .iter()
            .any(|option| !matches!(option, Expression::Eq(_)))
        {
            return Err(self.error_with(
                "Operation not allowed: Values must be specified for key(s)",
                at,
                true,
            ));
        }
        Ok(options)
    }

    /// `WITH ( [CREDENTIAL name | CREDENTIAL (options)] [ENCRYPTION (options)] )`.
    fn copy_credentials(&mut self) -> Parsing<Credentials> {
        self.expect_kw("WITH")?;
        self.expect_char('(')?;
        let mut credentials: Credentials = Credentials {
            credentials: Vec::new(),
            encryption: None,
            storage: None,
            name_quoted: false,
            without_equals: Vec::new(),
            quoted_keys: Vec::new(),
            string_keys: Vec::new(),
        };
        if self.eat_kw("CREDENTIAL") {
            match self.is_char('(') {
                true => {
                    let options = self.spelled_valued_property_list()?;
                    for (position, ((key, value), without_equals)) in
                        options.into_iter().enumerate()
                    {
                        if without_equals {
                            credentials.without_equals.push(position);
                        }
                        match &key {
                            Expression::Literal(_) => credentials.string_keys.push(position),
                            Expression::Identifier(name) if name.quoted => {
                                credentials.quoted_keys.push(position)
                            }
                            _ => {}
                        }
                        credentials
                            .credentials
                            .push((property_text(&key), property_text(&value)));
                    }
                }
                false => {
                    let name: Identifier = self.ident()?;
                    credentials.name_quoted = name.quoted;
                    credentials
                        .credentials
                        .push(("CREDENTIAL".to_owned(), name.name));
                }
            }
        }
        if self.eat_kw("ENCRYPTION") {
            let first: usize = self.at;
            self.option_list()?;
            let from: usize = self.tokens[first].start;
            credentials.encryption = Some(self.sql[from..self.tokens[self.at - 1].end].to_owned());
        }
        if credentials.credentials.is_empty() && credentials.encryption.is_none() {
            return Err(self.error());
        }
        self.expect_char(')')?;
        Ok(credentials)
    }

    /// `COPY INTO table … FROM source FILEFORMAT = format`, with its optional clauses.
    pub(crate) fn copy_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("COPY")?;
        self.expect_kw("INTO")?;
        let target: TableRef = self.table_reference()?;
        let target_credentials: Option<Box<Credentials>> =
            match self.kw("WITH") && self.char_at(1, '(') {
                true => Some(Box::new(self.copy_credentials()?)),
                false => None,
            };
        let by_position: bool = self.eat_kws(&["BY", "POSITION"]);
        let columns: Vec<Identifier> = match !by_position && self.is_char('(') {
            true => self.identifier_list()?,
            false => Vec::new(),
        };
        self.expect_kw("FROM")?;
        let mut credentials: Option<Box<Credentials>> = None;
        let source: Expression = match self.eat_char('(') {
            true => {
                self.expect_kw("SELECT")?;
                let expressions: Vec<Expression> =
                    self.comma_list(|parser| parser.named_expression())?;
                self.expect_kw("FROM")?;
                let location: Expression = self.string_literal()?;
                if self.kw("WITH") && self.char_at(1, '(') {
                    credentials = Some(Box::new(self.copy_credentials()?));
                }
                self.expect_char(')')?;
                let mut select: Select = Select::new();
                select.expressions = expressions;
                select.from = Some(From {
                    expressions: vec![location],
                });
                subquery(Expression::Select(Box::new(select)), None, false)
            }
            false => {
                let location: Expression = self.string_literal()?;
                if self.kw("WITH") && self.char_at(1, '(') {
                    credentials = Some(Box::new(self.copy_credentials()?));
                }
                location
            }
        };
        let mut params: Vec<CopyParameter> = Vec::new();
        self.expect_kw("FILEFORMAT")?;
        self.expect_char('=')?;
        let format: Identifier = self.ident()?;
        params.push(parameter(
            "FILEFORMAT",
            Some(match format.quoted {
                true => Expression::Identifier(format),
                false => var(&format.name),
            }),
            Vec::new(),
            true,
        ));
        if self.eat_kw("VALIDATE") {
            let value: Option<Expression> = if self.eat_kw("ALL") {
                Some(var("ALL"))
            } else if self.is(is_integer) {
                let rows: String = self.advance()?.value;
                self.expect_kw("ROWS")?;
                Some(literal(Literal::Number(rows)))
            } else {
                None
            };
            params.push(parameter("VALIDATE", value, Vec::new(), false));
        }
        if self.eat_kw("FILES") {
            self.expect_char('=')?;
            let files: Vec<Expression> = self.parenthesized(|parser| parser.string_literal())?;
            params.push(parameter("FILES", None, files, true));
        } else if self.eat_kw("PATTERN") {
            self.expect_char('=')?;
            let pattern: Expression = self.string_literal()?;
            params.push(parameter("PATTERN", Some(pattern), Vec::new(), true));
        }
        for name in ["FORMAT_OPTIONS", "COPY_OPTIONS"] {
            if self.eat_kw(name) {
                let options: Vec<Expression> = self.option_list()?;
                params.push(parameter(name, None, options, false));
            }
        }
        Ok(Expression::Copy(Box::new(CopyStmt {
            where_clause: None,
            copy_files: false,
            this: Expression::Table(Box::new(target)),
            kind: true,
            files: vec![source],
            params,
            credentials,
            is_into: true,
            with_wrapped: false,
            by_position,
            columns,
            target_credentials,
        })))
    }

    /// A statement with no shared-tree node, checked by `check`; a gap named by its first word.
    fn checked_gap(
        &mut self,
        construct: &'static str,
        check: impl FnOnce(&mut Self) -> Parsing<()>,
    ) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.at += 1;
        check(self)?;
        self.gap(construct, start);
        Ok(null())
    }

    /// `RESTORE [TABLE] table [TO] {VERSION | TIMESTAMP} AS OF value`: the table, then its version.
    pub(crate) fn restore_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("RESTORE")?;
        let mut kind: String = "RESTORE".to_owned();
        if self.kw("TABLE") && !self.kw_at(1, "TO") {
            self.at += 1;
            kind.push_str(" TABLE");
        }
        let target: Expression = Expression::Table(Box::new(self.table_reference()?));
        let mut words: Vec<&str> = self.eat_kw("TO").then_some("TO").into_iter().collect();
        let version: bool = self.eat_kw("VERSION");
        if !version {
            self.expect_kw("TIMESTAMP")?;
        }
        self.expect_kw("AS")?;
        self.expect_kw("OF")?;
        words.extend([if version { "VERSION" } else { "TIMESTAMP" }, "AS", "OF"]);
        let value: Expression = match version {
            true => literal(Literal::Number(self.expect_token(is_integer)?.value)),
            false => self.expression()?,
        };
        let restored: Expression =
            utility(kind, vec![target, property(&words.join(" "), Some(value))]);
        if too_deep_to_keep(&restored) {
            self.gap("restore", start);
            return Ok(null());
        }
        self.treed("restore", start);
        Ok(restored)
    }

    /// `REPAIR PRIVILEGES ...`, kept as a gap; `[MSCK] REPAIR TABLE` is a documented form.
    pub(crate) fn repair_statement(&mut self) -> Parsing<Expression> {
        let construct: &'static str = match self.kw("MSCK") {
            true => MSCK,
            false => "repair",
        };
        self.checked_gap(construct, |parser| {
            if construct == MSCK {
                parser.expect_kw("REPAIR")?;
                parser.expect_kw("TABLE")?;
            }
            parser.expect_kw("PRIVILEGES")?;
            parser.skip_statement_rest()
        })
    }

    /// `UNDROP {TABLE | MATERIALIZED VIEW | VOLUME} name`; `WITH ID 'id'` is a gap.
    pub(crate) fn undrop_statement(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        self.expect_kw("UNDROP")?;
        let kind: &str = match self.word_at(0).as_deref() {
            Some("TABLE") => "TABLE",
            Some("VOLUME") => "VOLUME",
            Some("MATERIALIZED") => {
                self.at += 1;
                if !self.kw("VIEW") {
                    return Err(self.error());
                }
                "MATERIALIZED VIEW"
            }
            _ => return Err(self.error()),
        };
        self.at += 1;
        let start: usize = self.offset();
        if self.kw("WITH") && self.kw_at(1, "ID") {
            let lead: usize = self.at - first;
            self.at += 2;
            self.string_lit()?;
            if let Some(tree) = self.tree_within(first, self.at, lead, UNDROP_WITH_ID) {
                self.treed("undrop_with_id", start);
                return Ok(tree);
            }
            self.gap("undrop_with_id", start);
            return Ok(null());
        }
        let name: TableRef = self.table_reference()?;
        Ok(Expression::Undrop(Box::new(Undrop {
            kind: kind.to_owned(),
            name,
            if_exists: false,
            rename_to: None,
        })))
    }

    /// `LOAD DATA [LOCAL] INPATH 'path' [OVERWRITE] INTO TABLE table [PARTITION (...)]`.
    pub(crate) fn load_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("LOAD")?;
        self.expect_kw("DATA")?;
        let local: bool = self.eat_kw("LOCAL");
        self.expect_kw("INPATH")?;
        let inpath: String = self.string_lit()?;
        let overwrite: bool = self.eat_kw("OVERWRITE");
        self.expect_kw("INTO")?;
        self.expect_kw("TABLE")?;
        let table: TableRef = self.table_reference()?;
        let partition: Vec<(Identifier, Expression)> = match self.kw("PARTITION") {
            true => self
                .partition_spec()?
                .into_iter()
                .map(|(name, value)| (name, value.unwrap_or_else(null)))
                .collect(),
            false => Vec::new(),
        };
        Ok(Expression::LoadData(Box::new(LoadData {
            local,
            inpath,
            overwrite,
            table: Expression::Table(Box::new(table)),
            partition,
            input_format: None,
            serde: None,
        })))
    }
}
