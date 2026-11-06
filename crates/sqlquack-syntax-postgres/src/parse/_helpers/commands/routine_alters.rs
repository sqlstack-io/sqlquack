//! `AlterFunctionStmt` and the statements that rename, move, re-own or tie a routine or aggregate (gram.y), as `AlterObject`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;

use crate::parse::_helpers::commands::alters::{alter, name_from};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_id};
use crate::parse::_helpers::tree::nodes::table;

/// The routine kinds `ALTER` names, as gram.y spells them after `ALTER`.
const ROUTINE_KINDS: [(&str, &str); 4] = [
    ("function", "FUNCTION"),
    ("procedure", "PROCEDURE"),
    ("routine", "ROUTINE"),
    ("aggregate", AGGREGATE_KIND),
];

const AGGREGATE_KIND: &str = "AGGREGATE";

/// The name parts of a `function_with_argtypes` or `aggregate_with_argtypes` signature.
fn signature_parts(signature: &Expression) -> Vec<Identifier> {
    match signature {
        Expression::Function(call) if call.qualified_name.is_empty() => {
            let mut name: Identifier = Identifier::new(call.name.clone());
            name.quoted = call.quoted;
            vec![name]
        }
        Expression::Function(call) => call.qualified_name.clone(),
        Expression::Dot(dot) => {
            let mut parts: Vec<Identifier> = signature_parts(&dot.this);
            parts.push(dot.field.clone());
            parts
        }
        Expression::Identifier(name) => vec![name.clone()],
        _ => Vec::new(),
    }
}

impl Parser<'_> {
    /// The routine kind `ALTER` alters here, if it is a function, procedure, routine or aggregate.
    pub(crate) fn altered_routine(&self) -> Option<&'static str> {
        match self.kw("alter") {
            true => ROUTINE_KINDS
                .iter()
                .find(|(word, _)| self.kw_at(1, word))
                .map(|(_, kind)| *kind),
            false => None,
        }
    }

    /// `ALTER kind signature` then a rename, an owner, a schema, an extension dependency or options.
    pub(crate) fn alter_routine(&mut self, kind: &'static str) -> Parsing<Expression> {
        self.at += 2;
        let aggregate: bool = kind == AGGREGATE_KIND;
        let signature: Expression = match aggregate {
            true => self.aggregate_with_argtypes()?,
            false => self.function_with_argtypes()?,
        };
        let target = (kind, table(signature_parts(&signature)), false);
        let moved: bool =
            self.kw("set") && self.kw_at(1, "schema") && self.peek_at(2).is_some_and(is_col_id);
        let depends: bool =
            !aggregate && (self.kw("depends") || (self.kw("no") && self.kw_at(1, "depends")));
        let (action, values): (&str, Vec<Expression>) = if self.eat_kw("rename") {
            self.expect_kw("to")?;
            ("RENAME", vec![name_from(self.name(is_col_id)?)])
        } else if self.eat_kw("owner") {
            self.expect_kw("to")?;
            ("OWNER TO", vec![self.role_spec()?])
        } else if moved {
            self.at += 2;
            ("SET SCHEMA", vec![name_from(self.name(is_col_id)?)])
        } else if depends {
            let action: &str = match self.eat_kw("no") {
                true => "NO DEPENDS ON EXTENSION",
                false => "DEPENDS ON EXTENSION",
            };
            self.expect_kw("depends")?;
            self.expect_kw("on")?;
            self.expect_kw("extension")?;
            (action, vec![name_from(self.name(is_col_id)?)])
        } else if aggregate {
            return Err(self.error());
        } else {
            let options: Vec<Expression> = self.common_routine_options()?;
            self.eat_kw("restrict");
            ("", options)
        };
        let mut operands: Vec<Expression> = vec![signature];
        operands.extend(values);
        Ok(alter(target, action, operands))
    }
}
