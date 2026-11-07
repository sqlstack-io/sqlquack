//! `function_parameters`, `procedure_parameters` and `type_or_tvf_schema` (googlesql.tm).

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;

use crate::parse::_helpers::commands::alters::{clause, named};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{tuple, var};
use crate::scan::models::TokenKind;

/// The modes a procedure parameter may have.
const PROCEDURE_MODES: [&str; 3] = ["IN", "OUT", "INOUT"];

/// A parameter's type from its tuple: the first data type in it.
pub(crate) fn parameter_type(parameter: &Expression) -> Option<DataType> {
    let Expression::Tuple(parts) = parameter else {
        return None;
    };
    parts.expressions.iter().find_map(|part| match part {
        Expression::DataType(data_type) => Some(data_type.clone()),
        _ => None,
    })
}

impl Parser<'_> {
    /// `function_parameters`: `( [function_parameter, ...] )`.
    pub(crate) fn function_parameters(&mut self) -> Parsing<Vec<Expression>> {
        self.expect_char('(')?;
        if self.eat_char(')') {
            return Ok(Vec::new());
        }
        let parameters: Vec<Expression> = self.comma_list(|parser| parser.function_parameter())?;
        self.expect_char(')')?;
        Ok(parameters)
    }

    /// `function_parameter`: `[name] type [AS alias] [DEFAULT expr] [NOT AGGREGATE]`.
    fn function_parameter(&mut self) -> Parsing<Expression> {
        let named_parameter: bool = self.is(is_identifier)
            && self
                .peek_at(1)
                .is_some_and(|token| matches!(token.kind, TokenKind::Word | TokenKind::Quoted))
            && !self.kw_in(1, &["AS", "DEFAULT", "NOT"]);
        let mut parts: Vec<Expression> = Vec::new();
        if named_parameter {
            parts.push(named(self.ident()?));
        }
        parts.push(Expression::DataType(self.type_or_tvf_schema()?));
        if self.eat_kw("AS") {
            parts.push(clause("AS", vec![named(self.ident()?)]));
        }
        if self.kw("DEFAULT") {
            if !named_parameter {
                return Err(self.error_with(
                    "Syntax error: Default expression is only allowed for named parameters",
                    self.offset(),
                    true,
                ));
            }
            self.at += 1;
            parts.push(clause("DEFAULT", vec![self.expression()?]));
        }
        if self.eat_kws(&["NOT", "AGGREGATE"]) {
            parts.push(var("NOT AGGREGATE"));
        }
        Ok(tuple(parts))
    }

    /// `procedure_parameters`: `( [[IN | OUT | INOUT] name type, ...] )`.
    pub(crate) fn procedure_parameters(&mut self) -> Parsing<Vec<Expression>> {
        self.expect_char('(')?;
        if self.eat_char(')') {
            return Ok(Vec::new());
        }
        let parameters: Vec<Expression> = self.comma_list(|parser| {
            let mut parts: Vec<Expression> = Vec::new();
            if parser.kw_in(0, &PROCEDURE_MODES) && parser.is_at(1, is_identifier) {
                parts.push(var(&parser.word_at(0).unwrap_or_default()));
                parser.at += 1;
            }
            parts.push(named(parser.ident()?));
            if parser.is_char(')') || parser.is_char(',') {
                return Err(parser.error_with(
                    "Syntax error: Unexpected end of parameter. Parameters should be in the \
                     format [<parameter mode>] <parameter name> <type>.",
                    parser.offset(),
                    true,
                ));
            }
            parts.push(Expression::DataType(parser.type_or_tvf_schema()?));
            Ok(tuple(parts))
        })?;
        self.expect_char(')')?;
        Ok(parameters)
    }

    /// `type_or_tvf_schema`: a type, `ANY kind`, or `TABLE< [name] type, ... >`.
    pub(crate) fn type_or_tvf_schema(&mut self) -> Parsing<DataType> {
        if self.kw("ANY") {
            self.at += 1;
            let kind: String = match self.word_at(0) {
                Some(word) => {
                    self.at += 1;
                    word
                }
                None => self.ident()?.name,
            };
            return Ok(DataType::Custom {
                name: format!("ANY {kind}"),
            });
        }
        if self.kw("TABLE") && self.char_at(1, '<') {
            self.at += 2;
            let fields: Vec<(String, DataType, bool)> = self.comma_list(|parser| {
                let named_column: bool = parser.is(is_identifier)
                    && !parser.char_at(1, '<')
                    && parser.peek_at(1).is_some_and(|token| {
                        matches!(token.kind, TokenKind::Word | TokenKind::Quoted)
                    });
                let name: String = match named_column {
                    true => parser.ident()?.name,
                    false => String::new(),
                };
                Ok((name, parser.data_type()?, false))
            })?;
            self.expect_char('>')?;
            return Ok(DataType::Object {
                fields,
                modifier: Some("TABLE".to_owned()),
            });
        }
        self.data_type()
    }
}
