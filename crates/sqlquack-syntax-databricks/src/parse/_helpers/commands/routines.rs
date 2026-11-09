//! `CREATE` and `DROP` of schemas and functions, with Databricks' routine characteristics.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::schema_objects::models::{
    CreateFunction, CreateSchema, DropFunction, DropSchema, FunctionBody, FunctionParameter,
    FunctionSecurity, FunctionUsingResource, SqlDataAccess,
};
use sqlquack_syntax::expressions::tables::models::{
    LocationProperty, Properties, SchemaCommentProperty,
};
use sqlquack_syntax::tokens::models::SourceSpan;

use crate::parse::_helpers::commands::creates::{Head, property};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::{joined, string, table, var};
use crate::parse::constants::SQL_LANGUAGE;
use crate::scan::models::{NumberKind, TokenKind};

impl Parser<'_> {
    /// `createNamespace` after `CREATE`: `SCHEMA [IF NOT EXISTS] name` and its clauses.
    pub(crate) fn create_schema(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let mut node: CreateSchema = CreateSchema::new(String::new());
        node.if_not_exists = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        node.name = self.multipart_identifier()?;
        loop {
            if self.eat_kw("COMMENT") {
                let text: Expression = self.string_literal()?;
                node.properties
                    .push(Expression::SchemaCommentProperty(Box::new(
                        SchemaCommentProperty {
                            this: Box::new(text),
                        },
                    )));
            } else if self.eat_kw("LOCATION") {
                let path: Expression = self.string_literal()?;
                node.properties
                    .push(Expression::LocationProperty(Box::new(LocationProperty {
                        this: Box::new(path),
                    })));
            } else if self.eat_kws(&["MANAGED", "LOCATION"]) {
                let path: Expression = self.string_literal()?;
                node.properties.push(property("MANAGED LOCATION", path));
            } else if self.eat_kws(&["DEFAULT", "COLLATION"]) {
                let name: Identifier = self.ident()?;
                node.properties
                    .push(property("DEFAULT COLLATION", string(&name.name)));
            } else if self.kw("WITH") && self.kw_in(1, &["DBPROPERTIES", "PROPERTIES"]) {
                self.at += 2;
                let expressions: Vec<Expression> = self.property_list()?;
                node.properties
                    .push(Expression::Properties(Box::new(Properties { expressions })));
            } else if self.eat_kws(&["RETAIN", "DROPPED", "FOR"]) {
                let count: String = self
                    .expect_token(|token| token.kind == TokenKind::Number(NumberKind::Integer))?
                    .value;
                let unit: String = match self.word_at(0) {
                    Some(unit)
                        if ["HOUR", "HOURS", "DAY", "DAYS", "WEEK", "WEEKS"]
                            .contains(&unit.as_str()) =>
                    {
                        unit
                    }
                    _ => return Err(self.error()),
                };
                self.at += 1;
                node.properties.push(property(
                    "RETAIN DROPPED FOR",
                    var(&format!("{count} {unit}")),
                ));
            } else {
                return Ok(Expression::CreateSchema(Box::new(node)));
            }
        }
    }

    /// `dropNamespace` after `DROP`: `SCHEMA [IF EXISTS] name [RESTRICT | CASCADE]`.
    pub(crate) fn drop_schema(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let mut node: DropSchema = DropSchema::new(String::new());
        node.if_exists = self.eat_kws(&["IF", "EXISTS"]);
        node.name = joined(self.multipart_identifier()?);
        node.cascade = self.eat_kw("CASCADE");
        node.restrict = !node.cascade && self.eat_kw("RESTRICT");
        Ok(Expression::DropSchema(Box::new(node)))
    }

    /// `dropFunction` after `DROP [TEMPORARY]`; a temporary function's drop is a gap.
    pub(crate) fn drop_function(&mut self, temporary: bool) -> Parsing<Expression> {
        let start: usize = self.tokens[self.at.saturating_sub(1 + usize::from(temporary))].start;
        self.expect_kw("FUNCTION")?;
        let mut node: DropFunction = DropFunction::new(String::new());
        node.if_exists = self.eat_kws(&["IF", "EXISTS"]);
        node.name = self.table_reference()?;
        if temporary {
            self.gap("drop_temporary_function", start);
        }
        Ok(Expression::DropFunction(Box::new(node)))
    }

    /// `colDefinition` as a routine parameter: `name type [NOT NULL] [DEFAULT expr] [COMMENT 'x']`.
    fn function_parameter(&mut self) -> Parsing<FunctionParameter> {
        let name: Identifier = self.ident()?;
        let data_type: DataType = self.data_type()?;
        let mut default: Option<Expression> = None;
        loop {
            if (self.kw("NOT") || self.is_char('!')) && self.kw_at(1, "NULL") {
                self.at += 2;
            } else if self.eat_kw("DEFAULT") {
                default = Some(self.expression()?);
            } else if self.eat_kw("COMMENT") {
                self.string_lit()?;
            } else {
                break;
            }
        }
        Ok(FunctionParameter {
            name: Some(name),
            data_type,
            mode: None,
            default,
            mode_text: None,
            span: SourceSpan::default(),
        })
    }

    /// `createFunction` and `createUserDefinedFunction` after `CREATE [OR REPLACE] [TEMPORARY]`.
    pub(crate) fn create_function(&mut self, head: Head) -> Parsing<Expression> {
        self.expect_kw("FUNCTION")?;
        let mut node: CreateFunction = CreateFunction::new(String::new());
        node.or_replace = head.or_replace;
        node.temporary = head.temporary;
        node.if_not_exists = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        if node.or_replace && node.if_not_exists {
            return Err(self.error_with(
                "CREATE FUNCTION with both IF NOT EXISTS and REPLACE is not allowed",
                self.offset(),
                true,
            ));
        }
        node.name = table(self.multipart_identifier()?);
        if self.eat_kw("AS") {
            node.has_parens = false;
            node.body = Some(FunctionBody::StringLiteral(self.string_lit()?));
            if self.eat_kw("USING") {
                node.using_resources = self.comma_list(|parser| {
                    let kind: Identifier = parser.ident()?;
                    Ok(FunctionUsingResource {
                        kind: kind.name.to_ascii_uppercase(),
                        uri: parser.string_lit()?,
                    })
                })?;
            }
            return Ok(Expression::CreateFunction(Box::new(node)));
        }
        self.expect_char('(')?;
        if !self.is_char(')') {
            node.parameters = self.comma_list(|parser| parser.function_parameter())?;
        }
        self.expect_char(')')?;
        if self.eat_kw("RETURNS") {
            if self.eat_kw("TABLE") {
                node.is_table_function = true;
                let first: usize = self.at;
                if self.is_char('(') {
                    self.parenthesized(|parser| parser.function_parameter())?;
                    let from: usize = self.tokens[first].start;
                    node.returns_table_body =
                        Some(self.sql[from..self.tokens[self.at - 1].end].to_owned());
                }
            } else {
                node.return_type = Some(self.data_type()?);
            }
        }
        node = self.routine_characteristics(node)?;
        if self.eat_kw("RETURN") {
            let body: Expression = match self.starts_query_at(0) {
                true => self.query()?,
                false => self.expression()?,
            };
            node.body = Some(FunctionBody::Return(body));
        } else {
            let foreign: bool = node
                .language
                .as_deref()
                .is_some_and(|language| language != SQL_LANGUAGE);
            if foreign && self.eat_kw("HANDLER") {
                node.handler = Some(joined(self.multipart_identifier()?).name);
                if !self.kw("AS") {
                    return Ok(Expression::CreateFunction(Box::new(node)));
                }
            }
            self.expect_kw("AS")?;
            let token = self.expect_token(|token| token.kind == TokenKind::DollarString)?;
            node.body = Some(FunctionBody::DollarQuoted {
                content: token.value,
                tag: None,
            });
        }
        node = self.routine_characteristics(node)?;
        Ok(Expression::CreateFunction(Box::new(node)))
    }

    /// `routineCharacteristics`, with Databricks' own, such as `ENVIRONMENT` and `SECRETS`.
    fn routine_characteristics(&mut self, mut node: CreateFunction) -> Parsing<CreateFunction> {
        loop {
            let start: usize = self.offset();
            if self.eat_kw("LANGUAGE") {
                let language: String = self
                    .expect_token(|token| token.kind == TokenKind::Word)?
                    .value
                    .to_ascii_uppercase();
                node.language = Some(language);
            } else if self.eat_kw("DETERMINISTIC") {
                node.deterministic = Some(true);
            } else if self.kw("NOT") && self.kw_at(1, "DETERMINISTIC") {
                self.at += 2;
                node.deterministic = Some(false);
            } else if self.eat_kw("COMMENT") {
                let text: Expression = self.string_literal()?;
                node.options.push(property("COMMENT", text));
            } else if self.eat_kws(&["CONTAINS", "SQL"]) {
                node.sql_data_access = Some(SqlDataAccess::ContainsSql);
            } else if self.eat_kws(&["READS", "SQL", "DATA"]) {
                node.sql_data_access = Some(SqlDataAccess::ReadsSqlData);
            } else if self.eat_kws(&["MODIFIES", "SQL", "DATA"]) {
                node.sql_data_access = Some(SqlDataAccess::ModifiesSqlData);
            } else if self.eat_kws(&["NO", "SQL"]) {
                node.sql_data_access = Some(SqlDataAccess::NoSql);
            } else if self.eat_kws(&["DEFAULT", "COLLATION"]) {
                let name: Identifier = self.ident()?;
                node.options
                    .push(property("DEFAULT COLLATION", string(&name.name)));
            } else if self.eat_kws(&["PARAMETER", "STYLE"]) {
                node.parameter_style = Some(self.ident()?.name.to_ascii_uppercase());
            } else if self.eat_kw("SPECIFIC") {
                let name: Identifier = self.ident()?;
                node.options.push(property("SPECIFIC", string(&name.name)));
            } else if self.eat_kws(&["RETURNS", "NULL", "ON", "NULL", "INPUT"]) {
                node.returns_null_on_null_input = Some(true);
            } else if self.eat_kws(&["CALLED", "ON", "NULL", "INPUT"]) {
                node.returns_null_on_null_input = Some(false);
            } else if self.eat_kws(&["SQL", "SECURITY"]) {
                node.security = Some(match self.word_at(0).as_deref() {
                    Some("DEFINER") => FunctionSecurity::Definer,
                    Some("INVOKER") => FunctionSecurity::Invoker,
                    _ => return Err(self.error()),
                });
                self.at += 1;
            } else if self.kw("ENVIRONMENT") && self.char_at(1, '(') {
                self.at += 1;
                node.environment = self.property_list()?;
            } else if self.kw("CREDENTIALS") && self.char_at(1, '(') {
                self.at += 1;
                self.parenthesized(|parser| {
                    parser.multipart_identifier()?;
                    if parser.eat_kw("AS") {
                        parser.ident()?;
                    } else {
                        parser.eat_kw("DEFAULT");
                    }
                    Ok(())
                })?;
                self.gap("function_credentials", start);
            } else if self.kw("SECRETS") && self.char_at(1, '(') {
                self.at += 1;
                self.parenthesized(|parser| parser.multipart_identifier())?;
                self.gap("function_secrets", start);
            } else {
                return Ok(node);
            }
        }
    }
}
