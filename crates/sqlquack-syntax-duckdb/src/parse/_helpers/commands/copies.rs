//! `CopyStatement` (copy.gram): a table or a query to or from a file, and `COPY FROM DATABASE`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::statements::models::{CopyParameter, CopyStmt, Schema};

use crate::parse::_helpers::commands::options::CopyOption;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label, is_identifier};
use crate::parse::_helpers::tree::nodes::{column, literal, subquery, var};

/// The options `SetCopyOptions` needs a column list or `*` for.
const LIST_OPTIONS: [&str; 4] = [
    "partition_by",
    "force_quote",
    "force_not_null",
    "force_null",
];
const FORMAT_OPTION: &str = "format";
const PARTITION_OPTION: &str = "partition_by";
const PARTITIONED_OPTION: &str = "partitioned_by";
const FORMAT_NEEDS_VALUE: &str =
    "Unsupported parameter type for FORMAT: expected e.g. FORMAT 'csv', 'parquet'";
/// `SingleOption`: a word alone and the option it stands for.
const SINGLE_OPTIONS: [(&str, &str); 5] = [
    ("BINARY", "format"),
    ("FREEZE", "freeze"),
    ("OIDS", "oids"),
    ("CSV", "format"),
    ("HEADER", "header"),
];
/// The options that take a string: the word, whether `AS` may follow, and the option.
const STRING_OPTIONS: [(&str, bool, &str); 5] = [
    ("NULL", true, "null"),
    ("DELIMITER", true, "delimiter"),
    ("QUOTE", true, "quote"),
    ("ESCAPE", true, "escape"),
    ("ENCODING", false, "encoding"),
];
/// `CopyDatabaseFlag`.
const DATABASE_FLAGS: [&str; 2] = ["SCHEMA", "DATA"];

/// One parameter of the tree.
fn parameter(name: &str, value: Option<Expression>, values: Vec<Expression>) -> CopyParameter {
    CopyParameter {
        name: name.to_owned(),
        value,
        values,
        eq: false,
    }
}

/// What `SetCopyOptions` reads of one option: its name and whether it has a value.
struct Setting {
    name: String,
    valued: bool,
}

impl Parser<'_> {
    /// `CopyStatement`: `COPY` a table or a query, or `COPY FROM DATABASE`.
    pub(crate) fn copy_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("COPY")?;
        if self.kw("FROM") && self.kw_at(1, "DATABASE") {
            return self.copy_database();
        }
        if self.eat_char('(') {
            let query: Expression = self.query()?;
            self.expect_char(')')?;
            self.expect_kw("TO")?;
            let file: Expression = self.copy_file_name()?;
            let (params, settings, with_wrapped) = self.copy_options()?;
            self.check_copy_settings(&settings, start);
            return Ok(copy(
                subquery(query, None, false),
                false,
                file,
                (params, with_wrapped),
            ));
        }
        let relation = Expression::Table(Box::new(self.base_table_name()?));
        let this: Expression = match self.is_char('(') {
            true => {
                let names: Vec<Identifier> = self.comma_list_parenthesized()?;
                Expression::Schema(Box::new(Schema {
                    this: Some(Box::new(relation)),
                    expressions: names.into_iter().map(|name| column(None, name)).collect(),
                }))
            }
            false => relation,
        };
        let from: bool = self.eat_kw("FROM");
        if !from {
            self.expect_kw("TO")?;
        }
        let file: Expression = self.copy_file_name()?;
        let mut params: Vec<CopyParameter> = Vec::new();
        let mut settings: Vec<Setting> = Vec::new();
        if self.eat_kw("USING") || self.kw("DELIMITERS") {
            self.expect_kw("DELIMITERS")?;
            let delimiter: Expression = self.copy_string()?;
            params.push(parameter("delimiter", Some(delimiter), Vec::new()));
            settings.push(Setting {
                name: "delimiter".to_owned(),
                valued: true,
            });
        }
        let (options, option_settings, with_wrapped) = self.copy_options()?;
        params.extend(options);
        settings.extend(option_settings);
        self.check_copy_settings(&settings, start);
        Ok(copy(this, from, file, (params, with_wrapped)))
    }

    /// `InsertColumnList`: `(name, ...)`.
    fn comma_list_parenthesized(&mut self) -> Parsing<Vec<Identifier>> {
        self.parenthesized(|parser| parser.name(is_identifier))
    }

    /// `CopyFromDatabase`: `FROM DATABASE source TO target [(SCHEMA | DATA)]`.
    fn copy_database(&mut self) -> Parsing<Expression> {
        self.at += 2;
        let source: Identifier = self.name(is_identifier)?;
        self.expect_kw("TO")?;
        let target: Identifier = self.name(is_identifier)?;
        let mut params: Vec<CopyParameter> = Vec::new();
        if self.is_char('(') && DATABASE_FLAGS.iter().any(|flag| self.kw_at(1, flag)) {
            self.at += 1;
            let flag: String = self.word_at(0).unwrap_or_default();
            self.at += 1;
            self.expect_char(')')?;
            params.push(parameter(&flag.to_ascii_lowercase(), None, Vec::new()));
        }
        let this = Expression::Identifier(source);
        Ok(copy(
            this,
            false,
            Expression::Identifier(target),
            (params, false),
        ))
    }

    /// `CopyFileName`: a parameter, a parenthesized expression, a string, or a dotted name.
    fn copy_file_name(&mut self) -> Parsing<Expression> {
        if self.is_char('?') || self.is_char('$') {
            return self.primary_expression();
        }
        if self.eat_char('(') {
            let expression: Expression = self.expression()?;
            self.expect_char(')')?;
            return Ok(expression);
        }
        if self.string_at(0) {
            return self.copy_string();
        }
        let mut parts: Vec<String> = vec![self.name(is_identifier)?.name];
        while self.is_char('.') && self.is_at(1, is_identifier) {
            self.at += 1;
            parts.push(self.name(is_identifier)?.name);
        }
        Ok(literal(Literal::String(parts.join("."))))
    }

    /// A string literal.
    fn copy_string(&mut self) -> Parsing<Expression> {
        let text: String = self.expect_string()?;
        Ok(literal(Literal::String(text)))
    }

    /// `CopyOptions?`: `[WITH] (generic options)` or specialized options, and what each sets.
    fn copy_options(&mut self) -> Parsing<(Vec<CopyParameter>, Vec<Setting>, bool)> {
        let with: bool = self.kw("WITH");
        let offset: usize = usize::from(with);
        if self.char_at(offset, '(') {
            self.at += offset;
            let options: Vec<CopyOption> =
                self.parenthesized(|parser| parser.copy_generic_option())?;
            let settings: Vec<Setting> = options
                .iter()
                .map(|option| Setting {
                    name: option.name.name.to_lowercase(),
                    valued: option.value.is_some(),
                })
                .collect();
            let params: Vec<CopyParameter> = options
                .into_iter()
                .map(|option| {
                    parameter(
                        &option.name.name.to_lowercase(),
                        option.expression,
                        Vec::new(),
                    )
                })
                .collect();
            return Ok((params, settings, true));
        }
        let mut params: Vec<CopyParameter> = Vec::new();
        if self.specialized_option_at(offset) {
            self.at += offset;
            params.push(self.specialized_option()?);
            loop {
                let comma: usize = usize::from(self.is_char(','));
                if !self.specialized_option_at(comma) {
                    break;
                }
                self.at += comma;
                params.push(self.specialized_option()?);
            }
        }
        let settings: Vec<Setting> = params
            .iter()
            .map(|param| Setting {
                name: param.name.clone(),
                valued: true,
            })
            .collect();
        Ok((params, settings, false))
    }

    /// `CopyGenericOption`: `ORDER BY`, `PARTITION BY` or a named option, each with a value or not.
    fn copy_generic_option(&mut self) -> Parsing<CopyOption> {
        let start: usize = self.offset();
        let two_words: Option<&str> = match (self.word_at(0).as_deref(), self.kw_at(1, "BY")) {
            (Some("ORDER"), true) => Some("order_by"),
            (Some("PARTITION" | "PARTITIONED"), true) => Some(PARTITION_OPTION),
            _ => None,
        };
        let name: Identifier = match two_words {
            Some(name) => {
                self.at += 2;
                Identifier::new(name)
            }
            None => self.name(is_col_label)?,
        };
        self.copy_option_value(name, start)
    }

    /// Whether a `SpecializedOption` starts `offset` tokens ahead.
    fn specialized_option_at(&self, offset: usize) -> bool {
        let single: bool = SINGLE_OPTIONS
            .iter()
            .any(|(word, _)| self.kw_at(offset, word));
        let string: bool = STRING_OPTIONS
            .iter()
            .any(|(word, _, _)| self.kw_at(offset, word));
        let force: bool = self.kw_at(offset, "FORCE");
        let partition: bool =
            self.kw_in(offset, &["PARTITION", "PARTITIONED"]) && self.kw_at(offset + 1, "BY");
        single || string || force || partition
    }

    /// `SpecializedOption`, as the option it sets.
    fn specialized_option(&mut self) -> Parsing<CopyParameter> {
        if let Some((_, name)) = SINGLE_OPTIONS.iter().find(|(word, _)| self.kw(word)) {
            self.at += 1;
            return Ok(parameter(name, None, Vec::new()));
        }
        if self.kw("QUOTE") && !(self.kw_at(1, "AS") || self.string_at(1)) {
            self.at += 1;
            return self.column_list_option("quote");
        }
        if let Some((_, takes_as, name)) = STRING_OPTIONS.iter().find(|(word, _, _)| self.kw(word))
        {
            self.at += 1;
            if *takes_as {
                self.eat_kw("AS");
            }
            let value: Expression = self.copy_string()?;
            return Ok(parameter(name, Some(value), Vec::new()));
        }
        if self.eat_kw("FORCE") {
            if self.eat_kw("QUOTE") {
                return self.column_list_option("force_quote");
            }
            let name: &str = match self.eat_kw("NOT") {
                true => "force_not_null",
                false => "force_null",
            };
            self.expect_kw("NULL")?;
            let columns: Vec<Identifier> = self.comma_list(|parser| parser.name(is_identifier))?;
            let values: Vec<Expression> =
                columns.into_iter().map(|name| column(None, name)).collect();
            return Ok(parameter(name, None, values));
        }
        self.at += 2;
        if self.is_char('(') {
            let columns: Vec<Identifier> = self.comma_list_parenthesized()?;
            let values: Vec<Expression> =
                columns.into_iter().map(|name| column(None, name)).collect();
            return Ok(parameter(PARTITION_OPTION, None, values));
        }
        self.column_list_option(PARTITION_OPTION)
    }

    /// `StarSymbolColumnList`: `*` or `name, ...`, for option `name`.
    fn column_list_option(&mut self, name: &str) -> Parsing<CopyParameter> {
        if self.eat_char('*') {
            return Ok(parameter(name, Some(var("*")), Vec::new()));
        }
        let single: bool = name == PARTITION_OPTION;
        let columns: Vec<Identifier> = match single {
            true => vec![self.name(is_identifier)?],
            false => self.comma_list(|parser| parser.name(is_identifier))?,
        };
        let values: Vec<Expression> = columns.into_iter().map(|name| column(None, name)).collect();
        Ok(parameter(name, None, values))
    }

    /// `SetCopyOptions`' refusals in order: a repeat, a list option without a list, a bare `FORMAT`.
    fn check_copy_settings(&mut self, settings: &[Setting], start: usize) {
        let mut seen: Vec<&str> = Vec::new();
        for setting in settings {
            let name: &str = match setting.name.as_str() {
                PARTITIONED_OPTION => PARTITION_OPTION,
                other => other,
            };
            if seen.contains(&name) {
                let message: String = format!("Unexpected duplicate option {}", quoted(name));
                self.refuse_later(&message, start);
                return;
            }
            seen.push(name);
            if !setting.valued && LIST_OPTIONS.contains(&name) {
                let message: String =
                    format!("{} expects a column list or * as parameter", quoted(name));
                self.refuse_later(&message, start);
                return;
            }
        }
        let format: bool = settings
            .iter()
            .any(|setting| setting.name == FORMAT_OPTION && !setting.valued);
        if format {
            self.refuse_later(FORMAT_NEEDS_VALUE, start);
        }
    }
}

/// `name` in double quotes, its quotes doubled, as the transformer prints an identifier.
fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// The `COPY` node; `options` is the parameters and whether they were parenthesized.
fn copy(
    this: Expression,
    from: bool,
    file: Expression,
    options: (Vec<CopyParameter>, bool),
) -> Expression {
    let (params, with_wrapped) = options;
    Expression::Copy(Box::new(CopyStmt {
        this,
        kind: from,
        files: vec![file],
        params,
        credentials: None,
        is_into: false,
        with_wrapped,
        where_clause: None,
    }))
}
