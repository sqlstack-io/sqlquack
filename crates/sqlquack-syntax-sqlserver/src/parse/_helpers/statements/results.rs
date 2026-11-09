//! Result clauses after a query: `FOR XML`, `FOR JSON` and `FOR BROWSE`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::queries::models::Select;
use sqlquack_syntax::expressions::statements::models::Lock;
use sqlquack_syntax::expressions::tables::models::QueryOption;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::{literal, var};
use crate::parse::constants::{
    BROWSE_IN_SET_OPERATION, BROWSE_NESTED, BROWSE_WITH_WRITE, JSON_MODES, JSON_ROOT,
    JSON_ROOT_WITHOUT_WRAPPER, JSON_WITHOUT_ARRAY_WRAPPER, NAMED_XML_MODES, XML_MODES,
};
use crate::scan::models::TokenKind;

/// Which select of `query` a result clause belongs to: the query's own, or the one it parenthesises.
fn result_select(query: &mut Expression) -> Option<&mut Select> {
    match query {
        Expression::Select(select) => Some(select),
        Expression::Paren(paren) => result_select(&mut paren.this),
        Expression::Subquery(subquery) if subquery.alias.is_none() => {
            result_select(&mut subquery.this)
        }
        _ => None,
    }
}

/// The result clause `query` ends in, `XML` or `JSON`; absent when it ends in neither.
pub(crate) fn xml_or_json_clause_name(query: &Expression) -> Option<&'static str> {
    match query {
        Expression::Select(select) if !select.for_xml.is_empty() => Some("XML"),
        Expression::Select(select) if !select.for_json.is_empty() => Some("JSON"),
        Expression::Paren(paren) => xml_or_json_clause_name(&paren.this),
        Expression::Subquery(subquery) if subquery.alias.is_none() => {
            xml_or_json_clause_name(&subquery.this)
        }
        _ => None,
    }
}

/// Whether `query` ends in `FOR BROWSE`.
pub(crate) fn has_browse(query: &Expression) -> bool {
    match query {
        Expression::Select(select) => !select.locks.is_empty(),
        Expression::Paren(paren) => has_browse(&paren.this),
        Expression::Subquery(subquery) if subquery.alias.is_none() => has_browse(&subquery.this),
        _ => false,
    }
}

impl<'sql> Parser<'sql> {
    /// `FOR XML mode [, directive ...]` or `FOR JSON mode [, option ...]` after `query`, when it comes next.
    pub(crate) fn xml_or_json_clause(&mut self, mut query: Expression) -> Parsing<Expression> {
        if !(self.kw("FOR") && self.kw_in(1, &["XML", "JSON"])) {
            return Ok(query);
        }
        let start: usize = self.offset();
        let json: bool = self.kw_at(1, "JSON");
        self.at += 2;
        let options: Vec<Expression> = match json {
            true => self.json_options()?,
            false => self.xml_options()?,
        };
        let Some(select) = result_select(&mut query) else {
            return Err(self.gap("set_operation_for_xml_json", start));
        };
        if select.into.is_some() {
            let clause: &str = if json { "JSON" } else { "XML" };
            return Err(self.error_with(&format!(
                "The FOR {clause} clause is not allowed in a SELECT INTO statement."
            )));
        }
        match json {
            true => select.for_json = options,
            false => select.for_xml = options,
        }
        Ok(query)
    }

    /// `FOR BROWSE` after a statement's `query`, when it comes next.
    pub(crate) fn browse_clause(&mut self, mut query: Expression) -> Parsing<Expression> {
        if !(self.kw("FOR") && self.kw_at(1, "BROWSE")) {
            return Ok(query);
        }
        if xml_or_json_clause_name(&query).is_some() {
            return Err(self.error());
        }
        let Some(select) = result_select(&mut query) else {
            return Err(self.error_with(BROWSE_IN_SET_OPERATION));
        };
        if select.into.is_some() {
            return Err(self.error_with(BROWSE_WITH_WRITE));
        }
        self.at += 2;
        select.locks = vec![Lock {
            ignore_locked: false,
            update: Some(Box::new(var("BROWSE"))),
            expressions: Vec::new(),
            wait: None,
            key: None,
        }];
        Ok(query)
    }

    /// Refuses a `FOR BROWSE` where only a statement's own query takes one.
    pub(crate) fn refuse_nested_browse(&self) -> Parsing<()> {
        match self.kw("FOR") && self.kw_at(1, "BROWSE") {
            true => Err(self.error_with(BROWSE_NESTED)),
            false => Ok(()),
        }
    }

    /// A `FOR XML` mode and its directives, each directive at most once and XMLDATA or XMLSCHEMA, not both.
    fn xml_options(&mut self) -> Parsing<Vec<Expression>> {
        let mode: String = self.mode(&XML_MODES)?;
        let named: bool = NAMED_XML_MODES.contains(&mode.as_str());
        if !named && self.is_char('(') {
            return Err(self.error_with(&format!("Parameterized FOR XML {mode} is not supported.")));
        }
        let argument: Option<Expression> = match named {
            true => self.string_argument()?,
            false => None,
        };
        let mut options: Vec<Expression> = vec![option(&mode, argument)];
        let mut seen: Vec<&'static str> = Vec::new();
        while self.eat_char(',') {
            let (kind, text, argument): (&'static str, String, Option<Expression>) =
                match self.word_at(0).as_deref() {
                    Some("BINARY") if self.kw_at(1, "BASE64") => {
                        self.at += 2;
                        ("BINARY", "BINARY BASE64".to_owned(), None)
                    }
                    Some("TYPE") => {
                        self.at += 1;
                        ("TYPE", "TYPE".to_owned(), None)
                    }
                    Some("ROOT") => {
                        self.at += 1;
                        ("ROOT", "ROOT".to_owned(), self.string_argument()?)
                    }
                    Some("ELEMENTS") => {
                        self.at += 1;
                        let text: String = match self.word_at(0).as_deref() {
                            Some(qualifier @ ("XSINIL" | "ABSENT")) => {
                                let text: String = format!("ELEMENTS {qualifier}");
                                self.at += 1;
                                text
                            }
                            _ => "ELEMENTS".to_owned(),
                        };
                        ("ELEMENTS", text, None)
                    }
                    Some("XMLDATA") => {
                        self.at += 1;
                        ("SCHEMA", "XMLDATA".to_owned(), None)
                    }
                    Some("XMLSCHEMA") => {
                        self.at += 1;
                        ("SCHEMA", "XMLSCHEMA".to_owned(), self.string_argument()?)
                    }
                    _ => return Err(self.error()),
                };
            if seen.contains(&kind) {
                return Err(self.error_at(self.at - 1));
            }
            seen.push(kind);
            options.push(option(&text, argument));
        }
        Ok(options)
    }

    /// A `FOR JSON` mode and its options, each at most once and ROOT or WITHOUT_ARRAY_WRAPPER, not both.
    fn json_options(&mut self) -> Parsing<Vec<Expression>> {
        let mode: String = self.mode(&JSON_MODES)?;
        let mut options: Vec<Expression> = vec![option(&mode, None)];
        let mut seen: Vec<String> = Vec::new();
        while self.eat_char(',') {
            let word: String = match self.word_at(0) {
                Some(word)
                    if matches!(
                        word.as_str(),
                        "ROOT" | "INCLUDE_NULL_VALUES" | "WITHOUT_ARRAY_WRAPPER"
                    ) =>
                {
                    word
                }
                _ => return Err(self.error()),
            };
            if seen.contains(&word) {
                return Err(self.error());
            }
            self.at += 1;
            let argument: Option<Expression> = match word.as_str() {
                "ROOT" => self.string_argument()?,
                _ => None,
            };
            seen.push(word.clone());
            options.push(option(&word, argument));
        }
        if seen.iter().any(|word| word == JSON_ROOT)
            && seen.iter().any(|word| word == JSON_WITHOUT_ARRAY_WRAPPER)
        {
            return Err(self.error_with(JSON_ROOT_WITHOUT_WRAPPER));
        }
        Ok(options)
    }

    /// The mode word after `FOR XML` or `FOR JSON`, one of `modes`.
    fn mode(&mut self, modes: &[&str]) -> Parsing<String> {
        match self.word_at(0) {
            Some(word) if modes.contains(&word.as_str()) => {
                self.at += 1;
                Ok(word)
            }
            _ => Err(self.error_at(self.at - 1)),
        }
    }

    /// A `('string')` argument, when one comes next.
    fn string_argument(&mut self) -> Parsing<Option<Expression>> {
        if !self.eat_char('(') {
            return Ok(None);
        }
        let value: Expression = match self.peek().cloned() {
            Some(token) if matches!(token.kind, TokenKind::String { .. }) => {
                self.at += 1;
                literal(match token.kind {
                    TokenKind::String { national: true } => Literal::NationalString(token.value),
                    _ => Literal::String(token.value),
                })
            }
            _ => return Err(self.error()),
        };
        self.expect_char(')')?;
        Ok(Some(value))
    }
}

/// One option of a result clause: its words, with the string it names when it takes one.
fn option(text: &str, argument: Option<Expression>) -> Expression {
    Expression::QueryOption(Box::new(QueryOption {
        this: Box::new(var(text)),
        expression: argument.map(Box::new),
    }))
}
