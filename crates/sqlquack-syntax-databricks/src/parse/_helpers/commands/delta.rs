//! Databricks' table maintenance and loading statements, such as `COPY INTO` and `VACUUM`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, TableRef};
use sqlquack_syntax::expressions::queries::models::{From, Select};
use sqlquack_syntax::expressions::statements::models::{
    CopyParameter, CopyStmt, Credentials, LoadData,
};
use sqlquack_syntax::expressions::tables::models::Undrop;

use crate::parse::_helpers::commands::alters::{property_pair, property_text};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::{literal, null, subquery, var};
use crate::parse::constants::MSCK;
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
        };
        if self.eat_kw("CREDENTIAL") {
            match self.is_char('(') {
                true => {
                    for option in self.option_list()? {
                        let (key, value) = property_pair(option);
                        credentials
                            .credentials
                            .push((property_text(&key), property_text(&value)));
                    }
                }
                false => {
                    let name: Identifier = self.ident()?;
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
        let start: usize = self.offset();
        if self.kw("WITH") && self.char_at(1, '(') {
            self.copy_credentials()?;
            self.gap("copy_target_credential", start);
        }
        let start: usize = self.offset();
        if self.eat_kws(&["BY", "POSITION"]) {
            self.gap("copy_by_position", start);
        } else if self.is_char('(') {
            self.identifier_list()?;
            self.gap("copy_columns", start);
        }
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
            Some(var(&format.name)),
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
            this: Expression::Table(Box::new(target)),
            kind: true,
            files: vec![source],
            params,
            credentials,
            is_into: true,
            with_wrapped: false,
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

    /// `OPTIMIZE table [FULL] [WHERE predicate] [ZORDER BY (columns)]`, kept as a gap.
    pub(crate) fn optimize_statement(&mut self) -> Parsing<Expression> {
        self.checked_gap("optimize", |parser| {
            match parser.string_at(0) {
                true => {
                    parser.string_lit()?;
                }
                false => {
                    parser.table_reference()?;
                }
            }
            parser.eat_kw("FULL");
            if parser.eat_kw("WHERE") {
                parser.expression()?;
            }
            if parser.eat_kw("ZORDER") {
                parser.expect_kw("BY")?;
                match parser.is_char('(') {
                    true => {
                        parser.parenthesized(|parser| parser.multipart_identifier())?;
                    }
                    false => {
                        parser.comma_list(|parser| parser.multipart_identifier())?;
                    }
                }
            }
            Ok(())
        })
    }

    /// `VACUUM table [USING INVENTORY ...] [RETAIN n HOURS] [DRY RUN] [FULL | LITE]`, as a gap.
    pub(crate) fn vacuum_statement(&mut self) -> Parsing<Expression> {
        self.checked_gap("vacuum", |parser| {
            match parser.string_at(0) {
                true => {
                    parser.string_lit()?;
                }
                false => {
                    parser.table_reference()?;
                }
            }
            if parser.eat_kws(&["USING", "INVENTORY"]) {
                match parser.eat_char('(') {
                    true => {
                        parser.query()?;
                        parser.expect_char(')')?;
                    }
                    false => {
                        parser.multipart_identifier()?;
                    }
                }
            }
            loop {
                if parser.eat_kw("RETAIN") {
                    parser.expect_token(|token| {
                        matches!(
                            token.kind,
                            TokenKind::Number(NumberKind::Integer | NumberKind::Decimal)
                        )
                    })?;
                    parser.expect_kw("HOURS")?;
                } else if parser.eat_kw("DRY") {
                    parser.expect_kw("RUN")?;
                } else if !(parser.eat_kw("FULL") || parser.eat_kw("LITE")) {
                    return Ok(());
                }
            }
        })
    }

    /// `RESTORE [TABLE] table [TO] {VERSION | TIMESTAMP} AS OF value`, kept as a gap.
    pub(crate) fn restore_statement(&mut self) -> Parsing<Expression> {
        self.checked_gap("restore", |parser| {
            if parser.kw("TABLE") && !parser.kw_at(1, "TO") {
                parser.at += 1;
            }
            parser.table_reference()?;
            parser.eat_kw("TO");
            let version: bool = parser.eat_kw("VERSION");
            if !version {
                parser.expect_kw("TIMESTAMP")?;
            }
            parser.expect_kw("AS")?;
            parser.expect_kw("OF")?;
            match version {
                true => {
                    parser.expect_token(is_integer)?;
                }
                false => {
                    parser.expression()?;
                }
            }
            Ok(())
        })
    }

    /// `[MSCK] REPAIR TABLE table [{ADD | DROP | SYNC} PARTITIONS | SYNC METADATA]`, as a gap.
    pub(crate) fn repair_statement(&mut self) -> Parsing<Expression> {
        let construct: &'static str = match self.kw("MSCK") {
            true => MSCK,
            false => "repair",
        };
        self.checked_gap(construct, |parser| {
            if construct == MSCK {
                parser.expect_kw("REPAIR")?;
            } else if parser.eat_kw("PRIVILEGES") {
                return parser.skip_statement_rest();
            }
            parser.expect_kw("TABLE")?;
            parser.table_reference()?;
            if parser.kw("SYNC") && parser.kw_at(1, "METADATA") {
                parser.at += 2;
            } else if construct == MSCK && parser.kw("PRIVILEGES") {
                parser.at += 1;
            } else if parser.kw_in(0, &["ADD", "DROP", "SYNC"]) {
                parser.at += 1;
                parser.expect_kw("PARTITIONS")?;
            }
            Ok(())
        })
    }

    /// `UNDROP {TABLE | MATERIALIZED VIEW | VOLUME} name`; `WITH ID 'id'` is a gap.
    pub(crate) fn undrop_statement(&mut self) -> Parsing<Expression> {
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
            self.at += 2;
            self.string_lit()?;
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
