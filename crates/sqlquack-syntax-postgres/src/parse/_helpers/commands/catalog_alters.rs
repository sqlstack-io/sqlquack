//! The `ALTER` actions of operator families, text search, replication and foreign data (gram.y).

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};

use crate::parse::_helpers::commands::alters::name_from;
use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::commands::tables::{clause, name_value};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_id};
use crate::parse::_helpers::tree::nodes::{null, table, tuple};
use crate::scan::models::TokenKind;

/// `preprocess_pubobj_list`'s error when the list starts with a bare name (42601).
const INVALID_LIST: &str = "invalid publication object list";

/// `preprocess_pubobj_list`'s error for `CURRENT_SCHEMA` after `TABLE` (42601).
const INVALID_TABLE: &str = "invalid table name";

/// `preprocess_pubobj_list`'s error for a qualified or starred name after `TABLES IN SCHEMA` (42601).
const INVALID_SCHEMA: &str = "invalid schema name";

/// `preprocess_pubobj_list`'s error for a row filter after `TABLES IN SCHEMA` (42601).
const SCHEMA_WHERE: &str = "WHERE clause not allowed for schema";

/// `preprocess_pubobj_list`'s error for a column list after `TABLES IN SCHEMA` (42601).
const SCHEMA_COLUMNS: &str = "column specification not allowed for schema";

/// `fdw_option` keywords, each with or without `NO`.
const FDW_OPTIONS: [(&str, &str, &str); 2] = [
    ("handler", "HANDLER", "NO HANDLER"),
    ("validator", "VALIDATOR", "NO VALIDATOR"),
];

/// One `PublicationObjSpec` as written, before `preprocess_pubobj_list` gives it its kind.
struct PublicationObject {
    /// `Some(true)` after `TABLES IN SCHEMA`, `Some(false)` after `TABLE`, `None` when continuing.
    schema: Option<bool>,
    name: Option<Identifier>,
    relation: Option<TableRef>,
    columns: Option<Expression>,
    filter: Option<Expression>,
    start: usize,
}

impl Parser<'_> {
    /// `ALTER OPERATOR FAMILY name USING method` then `ADD opclass_item_list` or `DROP opclass_drop_list`.
    pub(crate) fn operator_family_action(&mut self) -> Parsing<(&'static str, Vec<Expression>)> {
        if self.eat_kw("add") {
            return Ok((
                "ADD",
                self.comma_list(|parser| parser.operator_class_item())?,
            ));
        }
        self.expect_kw("drop")?;
        let items: Vec<Expression> = self.comma_list(|parser| {
            let keyword: &str = match parser.eat_kw("operator") {
                true => "OPERATOR",
                false => {
                    parser.expect_kw("function")?;
                    "FUNCTION"
                }
            };
            let number: Expression = parser.iconst_value()?;
            Ok(clause(keyword, vec![number, parser.type_list()?]))
        })?;
        Ok(("DROP", items))
    }

    /// One `opclass_item`: an operator, a support function, or `STORAGE`.
    fn operator_class_item(&mut self) -> Parsing<Expression> {
        if self.eat_kw("storage") {
            return Ok(clause(
                "STORAGE",
                vec![Expression::DataType(self.typename()?)],
            ));
        }
        if self.eat_kw("function") {
            let mut values: Vec<Expression> = vec![self.iconst_value()?];
            if self.is_char('(') {
                values.push(self.type_list()?);
            }
            values.push(self.function_with_argtypes()?);
            return Ok(clause("FUNCTION", values));
        }
        self.expect_kw("operator")?;
        let mut values: Vec<Expression> = vec![self.iconst_value()?];
        let operator: Expression = match self.attempt(|parser| parser.operator_with_argtypes()) {
            Ok(signature) => signature,
            Err(_) => self.any_operator()?,
        };
        values.push(operator);
        if self.eat_kw("for") && !self.eat_kw("search") {
            self.expect_kw("order")?;
            self.expect_kw("by")?;
            values.push(clause(
                "FOR ORDER BY",
                vec![name_value(&self.qualified_name()?)],
            ));
        }
        Ok(clause("OPERATOR", values))
    }

    /// `Iconst`: an integer, as its value.
    fn iconst_value(&mut self) -> Parsing<Expression> {
        match self.peek().map(|token| token.kind) {
            Some(TokenKind::Integer) => self.numeric_only(),
            _ => Err(self.error()),
        }
    }

    /// `'(' type_list ')'`, as a tuple of types.
    fn type_list(&mut self) -> Parsing<Expression> {
        let types: Vec<Expression> =
            self.parenthesized(|parser| Ok(Expression::DataType(parser.typename()?)))?;
        Ok(tuple(types))
    }

    /// `name_list` without parentheses: `name, ...`.
    fn bare_name_list(&mut self) -> Parsing<Expression> {
        let names: Vec<Expression> =
            self.comma_list(|parser| Ok(name_from(parser.name(is_col_id)?)))?;
        Ok(tuple(names))
    }

    /// `any_name_list`: `any_name, ...`.
    fn any_name_list(&mut self) -> Parsing<Expression> {
        let names: Vec<Expression> =
            self.comma_list(|parser| Ok(name_value(&parser.qualified_name()?)))?;
        Ok(tuple(names))
    }

    /// `AlterTSConfigurationStmt`'s mapping commands.
    pub(crate) fn mapping_action(&mut self) -> Parsing<(&'static str, Vec<Expression>)> {
        if self.eat_kw("add") {
            self.expect_kw("mapping")?;
            self.expect_kw("for")?;
            let tokens: Expression = self.bare_name_list()?;
            self.expect_kw("with")?;
            return Ok(("ADD MAPPING", vec![tokens, self.any_name_list()?]));
        }
        if self.eat_kw("drop") {
            self.expect_kw("mapping")?;
            let if_exists: bool = self.eat_kw("if");
            if if_exists {
                self.expect_kw("exists")?;
            }
            self.expect_kw("for")?;
            let tokens: Expression = self.bare_name_list()?;
            return Ok(match if_exists {
                true => ("DROP MAPPING IF EXISTS", vec![tokens]),
                false => ("DROP MAPPING", vec![tokens]),
            });
        }
        self.expect_kw("alter")?;
        self.expect_kw("mapping")?;
        let mut values: Vec<Expression> = Vec::new();
        if self.eat_kw("for") {
            values.push(self.bare_name_list()?);
            if self.eat_kw("with") {
                values.push(self.any_name_list()?);
                return Ok(("ALTER MAPPING", values));
            }
        }
        self.expect_kw("replace")?;
        values.push(name_value(&self.qualified_name()?));
        self.expect_kw("with")?;
        values.push(name_value(&self.qualified_name()?));
        Ok(("ALTER MAPPING REPLACE", values))
    }

    /// `AlterPublicationStmt`: `SET definition`, or `ADD`, `SET` or `DROP` a `pub_obj_list`.
    pub(crate) fn publication_action(&mut self) -> Parsing<(&'static str, Vec<Expression>)> {
        if self.kw("set") && self.char_at(1, '(') {
            self.at += 1;
            return Ok(("SET OPTIONS", self.options(false)?));
        }
        let action: &str = match self.keyword_at(0) {
            Some("add") => "ADD",
            Some("set") => "SET",
            Some("drop") => "DROP",
            _ => return Err(self.error()),
        };
        self.at += 1;
        let objects: Vec<PublicationObject> =
            self.comma_list(|parser| parser.publication_object())?;
        Ok((action, self.publication_objects(objects)?))
    }

    /// One `PublicationObjSpec`, as written.
    fn publication_object(&mut self) -> Parsing<PublicationObject> {
        let mut object = PublicationObject {
            schema: None,
            name: None,
            relation: None,
            columns: None,
            filter: None,
            start: self.offset(),
        };
        if self.kw("tables") && self.kw_at(1, "in") && self.kw_at(2, "schema") {
            self.at += 3;
            object.schema = Some(true);
            if !self.eat_kw("current_schema") {
                object.name = Some(self.name(is_col_id)?);
            }
            return Ok(object);
        }
        if self.eat_kw("current_schema") {
            return Ok(object);
        }
        let explicit: bool = self.eat_kw("table");
        let bare: bool =
            !explicit && self.is(is_col_id) && !self.char_at(1, '.') && !self.char_at(1, '*');
        let relation: TableRef = match bare {
            true => table(vec![self.name(is_col_id)?]),
            false => self.relation()?,
        };
        if explicit {
            object.schema = Some(false);
        }
        if self.is_char('(') {
            object.columns = Some(self.column_list(false)?);
        }
        if self.eat_kw("where") {
            object.filter = Some(self.parenthesized_condition()?);
        }
        match bare && object.columns.is_none() && object.filter.is_none() {
            true => object.name = Some(relation.name),
            false => object.relation = Some(relation),
        }
        Ok(object)
    }

    /// `preprocess_pubobj_list`: each object takes the kind before it, then is checked against it.
    fn publication_objects(&self, objects: Vec<PublicationObject>) -> Parsing<Vec<Expression>> {
        let mut previous: Option<bool> = None;
        let mut items: Vec<Expression> = Vec::new();
        for object in objects {
            let Some(schema) = object.schema.or(previous) else {
                return Err(self.error_with(INVALID_LIST, object.start));
            };
            previous = Some(schema);
            items.push(match schema {
                true => self.publication_schema(object)?,
                false => self.publication_table(object)?,
            });
        }
        Ok(items)
    }

    /// A `TABLE` object: its relation, column list and row filter.
    fn publication_table(&self, object: PublicationObject) -> Parsing<Expression> {
        let relation: TableRef = match (object.relation, object.name) {
            (Some(relation), _) => relation,
            (None, Some(name)) => table(vec![name]),
            (None, None) => return Err(self.error_with(INVALID_TABLE, object.start)),
        };
        let mut values: Vec<Expression> = vec![Expression::Table(Box::new(relation))];
        values.extend(object.columns);
        if let Some(filter) = object.filter {
            values.push(clause("WHERE", vec![filter]));
        }
        Ok(clause("TABLE", values))
    }

    /// A `TABLES IN SCHEMA` object: a schema name or `CURRENT_SCHEMA`.
    fn publication_schema(&self, object: PublicationObject) -> Parsing<Expression> {
        if object.filter.is_some() {
            return Err(self.error_with(SCHEMA_WHERE, object.start));
        }
        if object.columns.is_some() {
            return Err(self.error_with(SCHEMA_COLUMNS, object.start));
        }
        let schema: Expression = match (object.name, object.relation) {
            (Some(name), _) => name_from(name),
            (None, None) => var("CURRENT_SCHEMA"),
            (None, Some(_)) => return Err(self.error_with(INVALID_SCHEMA, object.start)),
        };
        Ok(clause("TABLES IN SCHEMA", vec![schema]))
    }

    /// `AlterSubscriptionStmt`'s actions.
    pub(crate) fn subscription_action(&mut self) -> Parsing<(&'static str, Vec<Expression>)> {
        let Some(word) = self.keyword_at(0) else {
            return Err(self.error());
        };
        self.at += 1;
        match word {
            "set" if self.is_char('(') => Ok(("SET OPTIONS", self.options(false)?)),
            "connection" => Ok(("CONNECTION", vec![self.sconst()?])),
            "enable" => Ok(("ENABLE", Vec::new())),
            "disable" => Ok(("DISABLE", Vec::new())),
            "skip" => Ok(("SKIP", self.options(false)?)),
            "refresh" => {
                self.expect_kw("publication")?;
                Ok(("REFRESH PUBLICATION", self.with_definition()?))
            }
            "add" | "drop" | "set" => {
                self.expect_kw("publication")?;
                let action: &str = match word {
                    "add" => "ADD PUBLICATION",
                    "drop" => "DROP PUBLICATION",
                    _ => "SET PUBLICATION",
                };
                let mut values: Vec<Expression> = vec![self.bare_name_list()?];
                values.extend(self.with_definition()?);
                Ok((action, values))
            }
            _ => {
                self.at -= 1;
                Err(self.error())
            }
        }
    }

    /// `opt_definition`: `WITH definition`, as one `WITH` clause, if written.
    fn with_definition(&mut self) -> Parsing<Vec<Expression>> {
        match self.eat_kw("with") {
            true => Ok(vec![clause("WITH", self.options(false)?)]),
            false => Ok(Vec::new()),
        }
    }

    /// `AlterFdwStmt` (`fdw` true) or `AlterForeignServerStmt`: their options, then `OPTIONS`.
    pub(crate) fn foreign_action(&mut self, fdw: bool) -> Parsing<(&'static str, Vec<Expression>)> {
        let mut values: Vec<Expression> = match fdw {
            true => self.fdw_options()?,
            false => self.server_version()?,
        };
        if self.kw("options") {
            values.push(clause("OPTIONS", self.generic_options()?));
        }
        match values.is_empty() {
            true => Err(self.error()),
            false => Ok(("", values)),
        }
    }

    /// `opt_fdw_options`: `[NO] HANDLER` and `[NO] VALIDATOR`, in order.
    fn fdw_options(&mut self) -> Parsing<Vec<Expression>> {
        let mut values: Vec<Expression> = Vec::new();
        loop {
            let negated: bool = self.kw("no");
            let offset: usize = usize::from(negated);
            let Some((word, set, unset)) = FDW_OPTIONS
                .iter()
                .find(|(word, _, _)| self.kw_at(offset, word))
            else {
                return Ok(values);
            };
            self.at += offset;
            self.expect_kw(word)?;
            values.push(match negated {
                true => clause(unset, Vec::new()),
                false => clause(set, vec![name_value(&self.qualified_name()?)]),
            });
        }
    }

    /// `foreign_server_version`: `VERSION 'text'` or `VERSION NULL`, if written.
    fn server_version(&mut self) -> Parsing<Vec<Expression>> {
        if !self.eat_kw("version") {
            return Ok(Vec::new());
        }
        let version: Expression = match self.eat_kw("null") {
            true => null(),
            false => self.sconst()?,
        };
        Ok(vec![clause("VERSION", vec![version])])
    }
}
