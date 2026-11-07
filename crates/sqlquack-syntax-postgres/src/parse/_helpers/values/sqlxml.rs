//! SQL/XML (gram.y): the `XML…` calls, `TREAT`, and `XMLTABLE`, checked only.

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_id, is_col_label};
use crate::parse::_helpers::values::expressions::Flavor;
use crate::scan::models::TokenKind;

/// What an `XMLTABLE` column has declared so far, to refuse a second of any kind.
#[derive(Default)]
struct ColumnOptions {
    default: bool,
    path: bool,
    nullability: bool,
}

impl Parser<'_> {
    /// An `XML…` call or `TREAT` after its name, through its closing parenthesis.
    pub(crate) fn xml_call(&mut self, name: &str) -> Parsing<()> {
        self.expect_char('(')?;
        match name {
            "xmlconcat" => {
                self.expr_list()?;
            }
            "xmlelement" => self.xml_element()?,
            "xmlexists" => {
                self.c_expr(Flavor::B)?;
                self.xml_passing()?;
            }
            "xmlforest" => {
                self.comma_list(|parser| parser.xml_attribute())?;
            }
            "xmlparse" => {
                self.document_or_content()?;
                self.a_expr()?;
                if self.eat_kw("preserve") || self.eat_kw("strip") {
                    self.expect_kw("whitespace")?;
                }
            }
            "xmlpi" => {
                self.expect_kw("name")?;
                self.expect_token(is_col_label)?;
                if self.eat_char(',') {
                    self.a_expr()?;
                }
            }
            "xmlroot" => self.xml_root()?,
            "xmlserialize" => {
                self.document_or_content()?;
                self.a_expr()?;
                self.expect_kw("as")?;
                self.simple_typename()?;
                if self.eat_kw("no") {
                    self.expect_kw("indent")?;
                } else {
                    self.eat_kw("indent");
                }
            }
            _ => {
                self.a_expr()?;
                self.expect_kw("as")?;
                self.typename()?;
            }
        }
        self.expect_char(')')
    }

    /// `XMLTABLE ( [XMLNAMESPACES (...),] row PASSING doc COLUMNS column, ... )`.
    pub(crate) fn xml_table(&mut self) -> Parsing<()> {
        self.expect_char('(')?;
        if self.kw("xmlnamespaces") && self.char_at(1, '(') {
            self.at += 1;
            self.parenthesized(|parser| {
                if parser.eat_kw("default") {
                    return parser.b_expr().map(drop);
                }
                parser.b_expr()?;
                parser.expect_kw("as")?;
                parser.expect_token(is_col_label).map(drop)
            })?;
            self.expect_char(',')?;
        }
        self.c_expr(Flavor::B)?;
        self.xml_passing()?;
        self.expect_kw("columns")?;
        self.comma_list(|parser| parser.xml_table_column())?;
        self.expect_char(')')
    }

    /// `NAME label [, XMLATTRIBUTES (...)] [, content, ...]`.
    fn xml_element(&mut self) -> Parsing<()> {
        self.expect_kw("name")?;
        self.expect_token(is_col_label)?;
        if !self.eat_char(',') {
            return Ok(());
        }
        if self.kw("xmlattributes") && self.char_at(1, '(') {
            self.at += 1;
            self.parenthesized(|parser| parser.xml_attribute())?;
            if !self.eat_char(',') {
                return Ok(());
            }
        }
        self.expr_list().map(drop)
    }

    /// `xml_attribute_el`: `a_expr [AS ColLabel]`.
    fn xml_attribute(&mut self) -> Parsing<()> {
        self.a_expr()?;
        if self.eat_kw("as") {
            self.expect_token(is_col_label)?;
        }
        Ok(())
    }

    /// `XMLROOT`'s arguments: `xml, VERSION {a_expr | NO VALUE} [, STANDALONE {YES | NO [VALUE]}]`.
    fn xml_root(&mut self) -> Parsing<()> {
        self.a_expr()?;
        self.expect_char(',')?;
        self.expect_kw("version")?;
        if self.kw("no") && self.kw_at(1, "value") {
            self.at += 2;
        } else {
            self.a_expr()?;
        }
        if self.eat_char(',') {
            self.expect_kw("standalone")?;
            if !self.eat_kw("yes") {
                self.expect_kw("no")?;
                self.eat_kw("value");
            }
        }
        Ok(())
    }

    fn document_or_content(&mut self) -> Parsing<()> {
        match self.eat_kw("document") || self.eat_kw("content") {
            true => Ok(()),
            false => Err(self.error()),
        }
    }

    /// `xmlexists_argument`: `PASSING [BY REF|VALUE] c_expr [BY REF|VALUE]`.
    fn xml_passing(&mut self) -> Parsing<()> {
        self.expect_kw("passing")?;
        self.passing_mechanism()?;
        self.c_expr(Flavor::B)?;
        self.passing_mechanism()
    }

    fn passing_mechanism(&mut self) -> Parsing<()> {
        if self.eat_kw("by") && !(self.eat_kw("ref") || self.eat_kw("value")) {
            return Err(self.error());
        }
        Ok(())
    }

    /// `xmltable_column_el`: `name FOR ORDINALITY`, or `name type` and options, each at most once.
    fn xml_table_column(&mut self) -> Parsing<()> {
        let name: String = self.name(is_col_id)?.name;
        if self.kw("for") && self.kw_at(1, "ordinality") {
            self.at += 2;
            return Ok(());
        }
        self.typename()?;
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
                _ if self.eat_kw("default") => {
                    self.b_expr()?;
                    std::mem::replace(&mut seen.default, true)
                }
                _ if self.eat_kw("path") => {
                    self.b_expr()?;
                    std::mem::replace(&mut seen.path, true)
                }
                _ if self.kw("not") && self.kw_at(1, "null") => {
                    self.at += 2;
                    std::mem::replace(&mut seen.nullability, true)
                }
                _ if self.eat_kw("null") => std::mem::replace(&mut seen.nullability, true),
                _ => break,
            };
            if repeated {
                refusal.get_or_insert((
                    format!("conflicting or redundant options for column \"{name}\""),
                    position,
                ));
            }
        }
        match refusal {
            None => Ok(()),
            Some((message, position)) => Err(self.error_with(&message, position)),
        }
    }
}
