//! `ExplainStatement`, `VacuumStatement`, `AnalyzeStatement`, `ExportStatement` and `ImportStatement`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::operators::models::Export;
use sqlquack_syntax::expressions::statements::models::{Analyze, Describe, Pragma};
use sqlquack_syntax::expressions::tables::models::Property;

use crate::parse::_helpers::commands::from::is_table_name;
use crate::parse::_helpers::commands::options::{CopyOption, CopyValue};
use crate::parse::_helpers::tree::cursor::{
    Parser, Parsing, is_col_label, is_identifier, is_reserved_word,
};
use crate::parse::_helpers::tree::nodes::{string, var};
use crate::parse::constants::{DEFAULT_WORD, EXPLAIN_SQL_KIND, EXPLAINABLE_STARTS, QUERY_STARTS};

/// The `Describe` styles that mark an `EXPLAIN`, with `ANALYZE` written before its options or not.
const EXPLAIN_STYLE: &str = "EXPLAIN";
const EXPLAIN_ANALYZE_STYLE: &str = "EXPLAIN ANALYZE";
/// The option names `TransformExplainStatement` knows, in lower case.
const FORMAT_OPTION: &str = "format";
const SQL_OPTION: &str = "sql";
const ANALYZE_OPTION: &str = "analyze";
/// The `Analyze` kind of a `VACUUM`; an `ANALYZE` has none.
const VACUUM_KIND: &str = "VACUUM";
/// `VacuumLegacyOptions`, in the order they must be written.
const VACUUM_LEGACY: [&str; 3] = ["FULL", "FREEZE", "VERBOSE"];
/// The vacuum options DuckDB refuses as not implemented, which a later stage reports.
const VACUUM_UNIMPLEMENTED: [&str; 4] = ["disable_page_skipping", "freeze", "full", "verbose"];
/// The pragma `IMPORT DATABASE` is, as DuckDB's transformer writes it.
const IMPORT_PRAGMA: &str = "import_database";
/// The transformer's refusals of an `EXPORT DATABASE` format.
const EXPORT_FORMAT_EXPRESSION: &str =
    "Unsupported parameter type for FORMAT: expected e.g. FORMAT 'csv', 'parquet'";
const EXPORT_FORMAT_MISSING: &str =
    "FORMAT requires a parameter, e.g. FORMAT 'csv' or FORMAT 'parquet'";

/// One `ExplainOption`: its name in lower case, and its value as written and as read.
struct ExplainOption {
    name: String,
    text: String,
    value: Option<Expression>,
}

impl Parser<'_> {
    /// `ExplainStatement`: `EXPLAIN [ANALYZE] [(options)] statement`, as a `Describe` styled `EXPLAIN`.
    pub(crate) fn explain_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("EXPLAIN")?;
        let analyze: bool = self.eat_kw("ANALYZE") || self.eat_kw("ANALYSE");
        let options: Vec<ExplainOption> = match self.is_char('(') {
            true => self
                .attempt(|parser| parser.parenthesized(|inner| inner.explain_option()))
                .unwrap_or_default(),
            false => Vec::new(),
        };
        let pivots: usize = self.pivot_entries;
        let target: Expression = self.explainable_statement()?;
        if explains_sql(&options, analyze) {
            self.pivot_entry_check(pivots, EXPLAIN_SQL_KIND, start);
        }
        let mut node: Describe = Describe::new(target);
        node.style = Some(
            match analyze {
                true => EXPLAIN_ANALYZE_STYLE,
                false => EXPLAIN_STYLE,
            }
            .to_owned(),
        );
        node.properties = options
            .into_iter()
            .map(|option| (option.name, option.text))
            .collect();
        Ok(Expression::Describe(Box::new(node)))
    }

    /// `ExplainOption`: a name, `ANALYZE` or any word but a reserved one, and an optional value.
    fn explain_option(&mut self) -> Parsing<ExplainOption> {
        let name: String = match self.eat_kw("ANALYZE") || self.eat_kw("ANALYSE") {
            true => ANALYZE_OPTION.to_owned(),
            false if self.is(is_reserved_word) => return Err(self.error()),
            false => self.name(is_col_label)?.name.to_lowercase(),
        };
        if self.terminal_at(0) {
            return Ok(ExplainOption {
                name,
                text: String::new(),
                value: None,
            });
        }
        let first: usize = self.offset();
        let value: Expression = self.expression()?;
        let end: usize = self.tokens[self.at - 1].end;
        Ok(ExplainOption {
            name,
            text: self.sql[first..end].to_owned(),
            value: Some(value),
        })
    }

    /// `ExplainableStatements`: a query or one of the statements `EXPLAIN` takes.
    fn explainable_statement(&mut self) -> Parsing<Expression> {
        if self.is_char('(') {
            return self.statement();
        }
        let word: String = self.word_at(0).unwrap_or_default();
        if self.kw("UPDATE") && self.kw_at(1, "EXTENSIONS") && !self.kw_at(2, "SET") {
            self.at += 1;
            return Err(self.error());
        }
        let explainable: bool = QUERY_STARTS.contains(&word.as_str())
            || EXPLAINABLE_STARTS.contains(&word.as_str())
            || (self.kw("FORCE") && self.kw_at(1, "CHECKPOINT"));
        match explainable {
            true => self.statement(),
            false => Err(self.error()),
        }
    }

    /// `VacuumStatement`: `VACUUM [(options) | legacy options] [table [(columns)]]`.
    pub(crate) fn vacuum_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("VACUUM")?;
        let options: Vec<String> = match self.is_char('(') {
            true => self.parenthesized(|parser| parser.vacuum_option())?,
            false => {
                let mut options: Vec<String> = VACUUM_LEGACY
                    .iter()
                    .filter(|word| self.eat_kw(word))
                    .map(|word| word.to_ascii_lowercase())
                    .collect();
                if self.eat_kw("ANALYZE") || self.eat_kw("ANALYSE") {
                    options.push(ANALYZE_OPTION.to_owned());
                }
                options
            }
        };
        for option in &options {
            let name: String = option.to_lowercase();
            if VACUUM_UNIMPLEMENTED.contains(&name.as_str()) {
                break;
            }
            if name != ANALYZE_OPTION {
                self.refuse_later(&format!("unrecognized VACUUM option \"{option}\""), start);
                break;
            }
        }
        self.analyze_node(Some(VACUUM_KIND), options)
    }

    /// `VacuumOption`: `ANALYZE`, `FREEZE`, `FULL`, `VERBOSE` or a name, as written.
    fn vacuum_option(&mut self) -> Parsing<String> {
        if self.eat_kw("ANALYZE") || self.eat_kw("ANALYSE") {
            return Ok(ANALYZE_OPTION.to_owned());
        }
        if let Some(word) = VACUUM_LEGACY.iter().find(|word| self.kw(word)) {
            self.at += 1;
            return Ok(word.to_ascii_lowercase());
        }
        Ok(self.name(is_identifier)?.name)
    }

    /// `AnalyzeStatement`: `ANALYZE [VERBOSE] [table [(columns)]]`.
    pub(crate) fn analyze_statement(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let options: Vec<String> = match self.eat_kw("VERBOSE") {
            true => vec!["verbose".to_owned()],
            false => Vec::new(),
        };
        self.analyze_node(None, options)
    }

    /// `AnalyzeTarget?` and the node of a `VACUUM` or an `ANALYZE` with its options.
    fn analyze_node(&mut self, kind: Option<&str>, options: Vec<String>) -> Parsing<Expression> {
        let (this, columns) = match self.is(is_table_name) {
            true => {
                let table = Expression::Table(Box::new(self.base_table_name()?));
                let columns: Vec<String> = match self.is_char('(') {
                    true => self
                        .parenthesized(|parser| parser.col_id_or_string())?
                        .into_iter()
                        .map(|column| column.name)
                        .collect(),
                    false => Vec::new(),
                };
                (Some(Box::new(table)), columns)
            }
            false => (None, Vec::new()),
        };
        Ok(Expression::Analyze(Box::new(Analyze {
            kind: kind.map(str::to_owned),
            this,
            options: options.iter().map(|option| var(option)).collect(),
            mode: None,
            partition: None,
            expression: None,
            properties: Vec::new(),
            columns,
        })))
    }

    /// `ExportStatement`: `EXPORT DATABASE [catalog TO] 'path' [(options)]`.
    pub(crate) fn export_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("EXPORT")?;
        self.expect_kw("DATABASE")?;
        let source: Option<Identifier> = match self.string_at(0) {
            true => None,
            false => {
                let name: Identifier = self.name(is_identifier)?;
                self.expect_kw("TO")?;
                Some(name)
            }
        };
        let path: String = self.string_lit()?;
        let options: Vec<CopyOption> = match self.is_char('(') {
            true => self.generic_copy_options()?,
            false => Vec::new(),
        };
        let format = options
            .iter()
            .find(|option| option.name.name.eq_ignore_ascii_case(FORMAT_OPTION));
        let refusal: Option<&str> = match format.map(|option| &option.value) {
            Some(Some(CopyValue::Expression)) => Some(EXPORT_FORMAT_EXPRESSION),
            Some(None) => Some(EXPORT_FORMAT_MISSING),
            _ => None,
        };
        if let Some(message) = refusal {
            self.refuse_later(message, start);
        }
        Ok(Expression::Export(Box::new(Export {
            this: Box::new(string(&path)),
            connection: source.map(|name| Box::new(Expression::Identifier(name))),
            options: options
                .into_iter()
                .map(|option| {
                    Expression::Property(Box::new(Property {
                        this: Box::new(Expression::Identifier(Identifier::new(
                            option.name.name.to_lowercase(),
                        ))),
                        value: option.expression.map(Box::new),
                    }))
                })
                .collect(),
        })))
    }

    /// `ImportStatement`: `IMPORT DATABASE 'path'`, the `import_database` pragma DuckDB reads it as.
    pub(crate) fn import_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("IMPORT")?;
        self.expect_kw("DATABASE")?;
        let path: String = self.string_lit()?;
        Ok(Expression::Pragma(Box::new(Pragma {
            schema: None,
            name: Identifier::new(IMPORT_PRAGMA),
            value: None,
            args: vec![string(&path)],
            use_assignment_syntax: false,
        })))
    }
}

/// Whether `TransformExplainStatement` reaches its `EXPLAIN (SQL)` check, with `SQL` alone and valid.
fn explains_sql(options: &[ExplainOption], analyze: bool) -> bool {
    let (mut format, mut sql, mut analyze) = (false, false, analyze);
    for option in options {
        match option.name.as_str() {
            FORMAT_OPTION if format || !format_value(option.value.as_ref()) => return false,
            FORMAT_OPTION => format = true,
            SQL_OPTION if sql || option.value.is_some() => return false,
            SQL_OPTION => sql = true,
            ANALYZE_OPTION => analyze = true,
            _ => return false,
        }
    }
    sql && !format && !analyze
}

/// Whether a `FORMAT` value is one the transformer takes: a string, a column name, or `DEFAULT`.
fn format_value(value: Option<&Expression>) -> bool {
    match value {
        Some(Expression::Literal(literal)) => matches!(literal.as_ref(), Literal::String(_)),
        Some(Expression::Column(_)) => true,
        Some(Expression::Var(word)) => word.this == DEFAULT_WORD,
        _ => false,
    }
}
