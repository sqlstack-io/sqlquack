//! The `DropEntries` other than tables and views (drop.gram), and `DropExtensionRepositoryStmt` (load.gram).

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::schema_objects::models::{
    DropFunction, DropSchema, DropSequence, DropTrigger, DropType,
};
use sqlquack_syntax::expressions::tables::models::DropObject;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label, is_identifier};
use crate::parse::_helpers::tree::nodes::{joined, null, table};
use crate::parse::constants::{
    DROP_COLLATION, DROP_ONE, DROP_SECRET_BEHAVIOR, DROP_SECRET_STORAGE, SECRET_KIND,
};
use crate::scan::models::Token;

/// `CASCADE` or `RESTRICT` after the dropped names.
struct Behavior {
    cascade: bool,
    restrict: bool,
}

impl Parser<'_> {
    /// A `DropEntries` kind other than `TABLE` and `VIEW`, with the cursor on the kind's first word.
    pub(crate) fn drop_object(&mut self, start: usize) -> Parsing<Expression> {
        if self.kw_in(0, &["TEMP", "TEMPORARY", "PERSISTENT"]) || self.kw(SECRET_KIND) {
            return self.drop_secret(start);
        }
        if self.kw("EXTENSION") && self.kw_at(1, "REPOSITORY") {
            self.at += 2;
            self.eat_kws(&["IF", "EXISTS"]);
            self.col_id_or_string()?;
            self.gap("extension_repository", start);
            return Ok(null());
        }
        if self.eat_kw("TRIGGER") {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let name: Identifier = self.name(is_identifier)?;
            self.expect_kw("ON")?;
            let target: TableRef = self.base_table_name()?;
            let behavior: Behavior = self.drop_behavior();
            return Ok(Expression::DropTrigger(Box::new(DropTrigger {
                name,
                table: Some(target),
                if_exists,
                cascade: behavior.cascade,
            })));
        }
        let Some(kind) = self.word_at(0) else {
            return Err(self.error());
        };
        let macro_table: bool = self.kw("MACRO") && self.kw_at(1, "TABLE");
        let collation: bool = self.kw("COLLATION");
        self.at += 1 + usize::from(macro_table);
        let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        let names: Vec<Vec<Identifier>> = match (kind.as_str(), macro_table) {
            ("MACRO", true) => self.comma_list(|parser| parser.dotted_name(is_identifier, None))?,
            ("MACRO" | "FUNCTION" | "TYPE", _) => {
                self.comma_list(|parser| parser.dotted_name(is_col_label, None))?
            }
            ("SCHEMA", _) => self.comma_list(|parser| parser.qualified_name())?,
            ("INDEX", _) => self.comma_list(|parser| parser.dotted_name(is_col_label, Some(3)))?,
            ("SEQUENCE", _) => {
                self.comma_list(|parser| parser.dotted_name(is_identifier, Some(3)))?
            }
            ("COLLATION", _) => self.comma_list(|parser| Ok(vec![parser.name(is_identifier)?]))?,
            _ => return Err(self.error()),
        };
        let behavior: Behavior = self.drop_behavior();
        if collation {
            self.refuse_later(DROP_COLLATION, start);
        }
        if names.len() > 1 {
            self.refuse_later(DROP_ONE, start);
        }
        let Some(name) = names.into_iter().next() else {
            return Err(self.error());
        };
        Ok(match (kind.as_str(), macro_table) {
            ("MACRO", true) => {
                self.gap("drop_table_macro", start);
                null()
            }
            ("MACRO" | "FUNCTION", _) => Expression::DropFunction(Box::new(DropFunction {
                name: table(name),
                parameters: None,
                if_exists,
                cascade: behavior.cascade,
            })),
            ("TYPE", _) => Expression::DropType(Box::new(DropType {
                name: table(name),
                if_exists,
                cascade: behavior.cascade,
            })),
            ("SCHEMA", _) => Expression::DropSchema(Box::new(DropSchema {
                name: joined(name),
                if_exists,
                cascade: behavior.cascade,
                restrict: behavior.restrict,
            })),
            ("INDEX", _) => Expression::DropObject(Box::new(DropObject {
                kind: kind.clone(),
                name: table(name),
                if_exists,
                cascade: behavior.cascade,
                restrict: behavior.restrict,
                force: false,
                objects: Vec::new(),
                concurrently: false,
            })),
            _ => Expression::DropSequence(Box::new(DropSequence {
                name: table(name),
                if_exists,
                cascade: behavior.cascade,
                restrict: behavior.restrict,
            })),
        })
    }

    /// `DropSecret`: `[TEMPORARY | PERSISTENT] SECRET [IF EXISTS] name [FROM storage]`, a gap.
    fn drop_secret(&mut self, start: usize) -> Parsing<Expression> {
        let temporary: bool = self.eat_kw("TEMP") || self.eat_kw("TEMPORARY");
        if !temporary {
            self.eat_kw("PERSISTENT");
        }
        self.expect_kw(SECRET_KIND)?;
        self.eat_kws(&["IF", "EXISTS"]);
        self.name(is_identifier)?;
        let storage: bool = self.eat_kw("FROM");
        if storage {
            self.name(is_identifier)?;
        }
        let behavior: Behavior = self.drop_behavior();
        if behavior.cascade || behavior.restrict {
            self.refuse_later(DROP_SECRET_BEHAVIOR, start);
        }
        if temporary && storage {
            self.refuse_later(DROP_SECRET_STORAGE, start);
        }
        self.gap("drop_secret", start);
        Ok(null())
    }

    /// `DropBehavior`.
    fn drop_behavior(&mut self) -> Behavior {
        let cascade: bool = self.eat_kw("CASCADE");
        let restrict: bool = !cascade && self.eat_kw("RESTRICT");
        Behavior { cascade, restrict }
    }

    /// A dotted name of at most `limit` parts: a first `Identifier`, then parts `rest` accepts.
    pub(crate) fn dotted_name(
        &mut self,
        rest: fn(&Token) -> bool,
        limit: Option<usize>,
    ) -> Parsing<Vec<Identifier>> {
        let mut parts: Vec<Identifier> = vec![self.name(is_identifier)?];
        while self.is_char('.') && limit.is_none_or(|limit| parts.len() < limit) {
            self.at += 1;
            parts.push(self.name(rest)?);
        }
        Ok(parts)
    }
}
