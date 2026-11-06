//! SQL/XML calls and `XMLTABLE` as `KeywordCall`s, `XMLEXISTS` as `xmlexists`, and `TREAT`, checked only.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;

use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_col_id, is_col_label};
use crate::parse::_helpers::tree::nodes::{boolean, function, keyword_call, null, tuple};
use crate::parse::_helpers::values::expressions::Flavor;
use crate::parse::constants::PRESERVE_WHITESPACE;
use crate::scan::models::TokenKind;

/// An `XMLTABLE` column's `PATH`, `DEFAULT` and `[NOT] NULL` flag, as declared so far.
#[derive(Default)]
struct ColumnOptions {
    default: Option<Expression>,
    path: Option<Expression>,
    nullability: Option<bool>,
}

/// A clause as a tuple led by its keywords.
fn clause(keywords: &str, values: Vec<Expression>) -> Expression {
    tuple(std::iter::once(var(keywords)).chain(values).collect())
}

/// `NAME ColLabel` as a clause.
fn named(name: Identifier) -> Expression {
    clause("NAME", vec![Expression::Identifier(name)])
}

impl Parser<'_> {
    /// An `XML…` call after its name, through its closing parenthesis; `TREAT` is checked only.
    pub(crate) fn xml_call(&mut self, name: &str) -> Parsing<Option<Expression>> {
        self.expect_char('(')?;
        let kind: String = name.to_uppercase();
        let node: Option<Expression> = match name {
            "xmlconcat" => Some(keyword_call(&kind, self.expr_list()?, Vec::new())),
            "xmlelement" => Some(self.xml_element()?),
            "xmlexists" => {
                let row: Expression = self.c_expr(Flavor::B)?;
                let document: Expression = self.xml_passing()?;
                Some(Expression::Function(Box::new(function(
                    vec![Identifier::new("xmlexists")],
                    vec![row, document],
                ))))
            }
            "xmlforest" => {
                let attributes: Vec<Expression> =
                    self.comma_list(|parser| parser.xml_attribute())?;
                Some(keyword_call(&kind, attributes, Vec::new()))
            }
            "xmlparse" => {
                let option: Expression = self.document_or_content()?;
                let value: Expression = self.a_expr()?;
                let preserve: bool = match self.keyword_at(0) {
                    Some(word @ ("preserve" | "strip")) => {
                        self.at += 1;
                        self.expect_kw("whitespace")?;
                        word == PRESERVE_WHITESPACE
                    }
                    _ => false,
                };
                Some(keyword_call(
                    &kind,
                    vec![value, boolean(preserve)],
                    vec![option],
                ))
            }
            "xmlpi" => {
                self.expect_kw("name")?;
                let label: Identifier = identifier(&self.expect_token(is_col_label)?);
                let operands: Vec<Expression> = match self.eat_char(',') {
                    true => vec![self.a_expr()?],
                    false => Vec::new(),
                };
                Some(keyword_call(&kind, operands, vec![named(label)]))
            }
            "xmlroot" => Some(self.xml_root()?),
            "xmlserialize" => {
                let option: Expression = self.document_or_content()?;
                let value: Expression = self.a_expr()?;
                self.expect_kw("as")?;
                let to: Expression = Expression::DataType(self.simple_typename()?);
                let mut options: Vec<Expression> = vec![option, clause("AS", vec![to])];
                let indent: bool = match self.eat_kw("no") {
                    true => {
                        self.expect_kw("indent")?;
                        false
                    }
                    false => self.eat_kw("indent"),
                };
                if indent {
                    options.push(var("INDENT"));
                }
                Some(keyword_call(&kind, vec![value], options))
            }
            _ => {
                self.a_expr()?;
                self.expect_kw("as")?;
                self.typename()?;
                None
            }
        };
        self.expect_char(')')?;
        Ok(node)
    }

    /// `XMLTABLE ( [XMLNAMESPACES (...),] row PASSING doc COLUMNS column, ... )`.
    pub(crate) fn xml_table(&mut self) -> Parsing<Expression> {
        self.expect_char('(')?;
        let mut options: Vec<Expression> = Vec::new();
        if self.kw("xmlnamespaces") && self.char_at(1, '(') {
            self.at += 1;
            let namespaces: Vec<Expression> = self.parenthesized(|parser| {
                if parser.eat_kw("default") {
                    return Ok(tuple(vec![parser.b_expr()?]));
                }
                let uri: Expression = parser.b_expr()?;
                parser.expect_kw("as")?;
                let name: Identifier = identifier(&parser.expect_token(is_col_label)?);
                Ok(tuple(vec![uri, Expression::Identifier(name)]))
            })?;
            options.push(clause("XMLNAMESPACES", namespaces));
            self.expect_char(',')?;
        }
        let row: Expression = self.c_expr(Flavor::B)?;
        let document: Expression = self.xml_passing()?;
        self.expect_kw("columns")?;
        options.push(clause(
            "COLUMNS",
            self.comma_list(|parser| parser.xml_table_column())?,
        ));
        self.expect_char(')')?;
        Ok(keyword_call("XMLTABLE", vec![row, document], options))
    }

    /// `NAME label [, XMLATTRIBUTES (...)] [, content, ...]`.
    fn xml_element(&mut self) -> Parsing<Expression> {
        self.expect_kw("name")?;
        let label: Identifier = identifier(&self.expect_token(is_col_label)?);
        let mut options: Vec<Expression> = vec![named(label)];
        if !self.eat_char(',') {
            return Ok(keyword_call("XMLELEMENT", Vec::new(), options));
        }
        if self.kw("xmlattributes") && self.char_at(1, '(') {
            self.at += 1;
            let attributes: Vec<Expression> =
                self.parenthesized(|parser| parser.xml_attribute())?;
            options.push(clause("XMLATTRIBUTES", attributes));
            if !self.eat_char(',') {
                return Ok(keyword_call("XMLELEMENT", Vec::new(), options));
            }
        }
        Ok(keyword_call("XMLELEMENT", self.expr_list()?, options))
    }

    /// `xml_attribute_el`: `a_expr [AS ColLabel]`, as `(value)` or `(value, name)`.
    fn xml_attribute(&mut self) -> Parsing<Expression> {
        let mut parts: Vec<Expression> = vec![self.a_expr()?];
        if self.eat_kw("as") {
            parts.push(Expression::Identifier(identifier(
                &self.expect_token(is_col_label)?,
            )));
        }
        Ok(tuple(parts))
    }

    /// `XMLROOT`'s value, version (`NO VALUE` is NULL) and standalone setting (`OMITTED` unless said).
    fn xml_root(&mut self) -> Parsing<Expression> {
        let value: Expression = self.a_expr()?;
        self.expect_char(',')?;
        self.expect_kw("version")?;
        let version: Expression = match self.kw("no") && self.kw_at(1, "value") {
            true => {
                self.at += 2;
                null()
            }
            false => self.a_expr()?,
        };
        let mut standalone: &str = "OMITTED";
        if self.eat_char(',') {
            self.expect_kw("standalone")?;
            standalone = match self.eat_kw("yes") {
                true => "YES",
                false => {
                    self.expect_kw("no")?;
                    match self.eat_kw("value") {
                        true => "NO VALUE",
                        false => "NO",
                    }
                }
            };
        }
        Ok(keyword_call(
            "XMLROOT",
            vec![value, version],
            vec![clause("STANDALONE", vec![var(standalone)])],
        ))
    }

    /// `DOCUMENT` or `CONTENT`.
    fn document_or_content(&mut self) -> Parsing<Expression> {
        let option: &str = match self.keyword_at(0) {
            Some("document") => "DOCUMENT",
            Some("content") => "CONTENT",
            _ => return Err(self.error()),
        };
        self.at += 1;
        Ok(var(option))
    }

    /// `xmlexists_argument`: `PASSING [BY REF|VALUE] c_expr [BY REF|VALUE]`, as its `c_expr`.
    fn xml_passing(&mut self) -> Parsing<Expression> {
        self.expect_kw("passing")?;
        self.passing_mechanism()?;
        let document: Expression = self.c_expr(Flavor::B)?;
        self.passing_mechanism()?;
        Ok(document)
    }

    fn passing_mechanism(&mut self) -> Parsing<()> {
        if self.eat_kw("by") && !(self.eat_kw("ref") || self.eat_kw("value")) {
            return Err(self.error());
        }
        Ok(())
    }

    /// `xmltable_column_el`: ordinality, or `(name, type)` then `PATH`, `DEFAULT` and `NOT NULL`.
    fn xml_table_column(&mut self) -> Parsing<Expression> {
        let name: Identifier = self.name(is_col_id)?;
        let column: String = name.name.clone();
        let name: Expression = Expression::Identifier(name);
        if self.kw("for") && self.kw_at(1, "ordinality") {
            self.at += 2;
            return Ok(keyword_call("FOR ORDINALITY", vec![name], Vec::new()));
        }
        let to: Expression = Expression::DataType(self.typename()?);
        let mut seen: ColumnOptions = ColumnOptions::default();
        let mut refusal: Option<(String, usize)> = None;
        loop {
            let position: usize = self.offset();
            let repeated: bool = match self.peek().map(|token| token.kind) {
                Some(TokenKind::Ident | TokenKind::QuotedIdent) => {
                    let option: String = self.advance()?.value;
                    self.b_expr()?;
                    refusal.get_or_insert((
                        format!("unrecognized column option \"{option}\""),
                        position,
                    ));
                    false
                }
                _ if self.eat_kw("default") => seen.default.replace(self.b_expr()?).is_some(),
                _ if self.eat_kw("path") => seen.path.replace(self.b_expr()?).is_some(),
                _ if self.kw("not") && self.kw_at(1, "null") => {
                    self.at += 2;
                    seen.nullability.replace(true).is_some()
                }
                _ if self.eat_kw("null") => seen.nullability.replace(false).is_some(),
                _ => break,
            };
            if repeated {
                refusal.get_or_insert((
                    format!("conflicting or redundant options for column \"{column}\""),
                    position,
                ));
            }
        }
        if let Some((message, position)) = refusal {
            return Err(self.error_with(&message, position));
        }
        let mut options: Vec<Expression> = Vec::new();
        options.extend(seen.path.map(|path| clause("PATH", vec![path])));
        options.extend(seen.default.map(|value| clause("DEFAULT", vec![value])));
        if seen.nullability == Some(true) {
            options.push(var("NOT NULL"));
        }
        Ok(keyword_call("COLUMN", vec![name, to], options))
    }
}
