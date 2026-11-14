//! Snowflake `EXECUTE JOB SERVICE`, `INFERENCE JOB SERVICE`, `NOTEBOOK [ PROJECT ]`, `DBT PROJECT` and `CODE BUNDLE`.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::TableRef;
use sqlquack_syntax::expressions::schema_objects::models::ExecuteObject;
use sqlquack_syntax::expressions::schema_objects::models::ObjectProperty;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// Options of `EXECUTE JOB SERVICE` after its source.
const JOB_SERVICE_OPTIONS: &[&str] = &[
    "NAME",
    "ASYNC",
    "REPLICAS",
    "QUERY_WAREHOUSE",
    "COMMENT",
    "EXTERNAL_ACCESS_INTEGRATIONS",
];

/// Options of `EXECUTE NOTEBOOK [ PROJECT ]` after its four required ones.
const NOTEBOOK_OPTIONS: &[&str] = &[
    "ARGUMENTS",
    "REQUIREMENTS_FILE",
    "ARTIFACT_REPOSITORIES",
    "EXTERNAL_ACCESS_INTEGRATIONS",
    "SECRETS",
];

/// The options `EXECUTE NOTEBOOK [ PROJECT ]` requires, in order.
const NOTEBOOK_REQUIRED: &[&str] = &["MAIN_FILE", "COMPUTE_POOL", "QUERY_WAREHOUSE", "RUNTIME"];

/// Options of `EXECUTE DBT PROJECT`.
const DBT_OPTIONS: &[&str] = &[
    "ARGS",
    "DBT_VERSION",
    "EXTERNAL_ACCESS_INTEGRATIONS",
    "ENVIRONMENT",
    "ENV_VARS",
    "IMPORTS",
    "WRITEBACK",
    "PROJECT_ROOT",
];

/// Options of `EXECUTE INFERENCE JOB SERVICE` after MODEL.
const INFERENCE_OPTIONS: &[&str] = &["VERSION", "FUNCTION", "NAME", "ASYNC", "REPLICAS"];

/// Options whose value is a parenthesized list of values.
const LIST_OPTIONS: &[&str] = &[
    "EXTERNAL_ACCESS_INTEGRATIONS",
    "ARTIFACT_REPOSITORIES",
    "SECRETS",
];

impl Parser {
    /// The job, notebook, dbt and code bundle forms of Snowflake EXECUTE, or none.
    pub(super) fn parse_snowflake_execute_job(&mut self) -> Result<Option<Expression>> {
        if !self.applies_snowflake_context_rules() {
            return Ok(None);
        }
        let kind = if self.match_text_seq(&["JOB", "SERVICE"]) {
            "JOB SERVICE"
        } else if self.match_text_seq(&["INFERENCE", "JOB", "SERVICE"]) {
            "INFERENCE JOB SERVICE"
        } else if self.match_text_seq(&["NOTEBOOK", "PROJECT"]) {
            "NOTEBOOK PROJECT"
        } else if self.match_text_seq(&["NOTEBOOK"]) {
            "NOTEBOOK"
        } else if self.match_text_seq(&["DBT", "PROJECT"]) {
            "DBT PROJECT"
        } else if self.match_text_seq(&["CODE", "BUNDLE"]) {
            "CODE BUNDLE"
        } else {
            return Ok(None);
        };
        let mut node = ExecuteObject {
            kind: kind.to_owned(),
            name: TableRef::new(""),
            action: None,
            group: None,
            alias: None,
            config: None,
            properties: Vec::new(),
        };
        node.properties = match kind {
            "JOB SERVICE" => self.parse_execute_job_service()?,
            "INFERENCE JOB SERVICE" => self.parse_execute_inference()?,
            "DBT PROJECT" => {
                if self.match_text_seq(&["FROM", "WORKSPACE"]) {
                    node.action = Some("FROM WORKSPACE".to_owned());
                }
                node.name = self.parse_object_name()?;
                self.parse_execute_options(DBT_OPTIONS, &[])?
            }
            "CODE BUNDLE" => {
                node.name = self.parse_object_name()?;
                self.parse_execute_code_bundle()?
            }
            "NOTEBOOK" if self.check_next(TokenType::LParen) => {
                node.name = self.parse_object_name()?;
                let arguments = self.parse_execute_list(Self::parse_execute_value)?;
                vec![property("PARAMETERS", None, Some(arguments))]
            }
            _ => {
                node.name = self.parse_object_name()?;
                let mut properties = self.parse_execute_required(NOTEBOOK_REQUIRED)?;
                let options = self.parse_execute_options(NOTEBOOK_OPTIONS, &properties)?;
                properties.extend(options);
                properties
            }
        };
        Ok(Some(Expression::ExecuteObject(Box::new(node))))
    }

    /// `IN COMPUTE POOL <name>` as a property.
    fn parse_execute_compute_pool(&mut self) -> Result<ObjectProperty> {
        if !self.match_text_seq(&["IN", "COMPUTE", "POOL"]) {
            return Err(self.parse_error("Expected IN COMPUTE POOL"));
        }
        let pool = Expression::Table(Box::new(self.parse_object_name()?));
        Ok(property("IN COMPUTE POOL", None, Some(pool)))
    }

    /// The entry point, arguments and specification of EXECUTE CODE BUNDLE.
    fn parse_execute_code_bundle(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties = self.parse_execute_required(&["ENTRYPOINT"])?;
        if self.match_text_seq(&["ARGUMENTS"]) {
            self.expect_token(TokenType::Eq)?;
            let list = self.parse_execute_list(Self::parse_execute_value)?;
            properties.push(property("ARGUMENTS", None, Some(list)));
        }
        if self.match_text_seq(&["WITH", "SPECIFICATION"]) {
            let text = self.parse_execute_specification()?;
            properties.push(property("WITH SPECIFICATION", None, Some(text)));
        }
        Ok(properties)
    }

    /// The pool, the specification source, its USING arguments and the options of EXECUTE JOB SERVICE.
    fn parse_execute_job_service(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties = vec![self.parse_execute_compute_pool()?];
        let options = self.parse_execute_options(JOB_SERVICE_OPTIONS, &properties)?;
        properties.extend(options);
        self.expect_token(TokenType::From)?;
        let template = if self.is_stage_path() {
            let stage = self.parse_stage_path()?;
            properties.push(property("FROM", None, Some(stage)));
            let template = self.check_text_seq(&["SPECIFICATION_TEMPLATE_FILE"]);
            let key = match template {
                true => "SPECIFICATION_TEMPLATE_FILE",
                false => "SPECIFICATION_FILE",
            };
            properties.extend(self.parse_execute_required(&[key])?);
            template
        } else {
            let template = self.match_text_seq(&["SPECIFICATION_TEMPLATE"]);
            if !template && !self.match_text_seq(&["SPECIFICATION"]) {
                return Err(
                    self.parse_error("Expected a stage, SPECIFICATION or SPECIFICATION_TEMPLATE")
                );
            }
            let key = match template {
                true => "FROM SPECIFICATION_TEMPLATE",
                false => "FROM SPECIFICATION",
            };
            let text = self.parse_execute_specification()?;
            properties.push(property(key, None, Some(text)));
            template
        };
        if template {
            if !self.match_token(TokenType::Using) && !self.match_text_seq(&["USING"]) {
                return Err(self.parse_error("Expected USING after the specification template"));
            }
            let arguments = self.parse_execute_using()?;
            properties.push(property("USING", None, Some(arguments)));
        }
        let options = self.parse_execute_options(JOB_SERVICE_OPTIONS, &properties)?;
        properties.extend(options);
        Ok(properties)
    }

    /// The pool, specification, input and model of EXECUTE INFERENCE JOB SERVICE, then its options.
    fn parse_execute_inference(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties = vec![self.parse_execute_compute_pool()?];
        if !self.match_text_seq(&["WITH", "SPECIFICATION"]) {
            return Err(self.parse_error("Expected WITH SPECIFICATION"));
        }
        let text = self.parse_execute_specification()?;
        properties.push(property("WITH SPECIFICATION", None, Some(text)));
        self.expect_token(TokenType::From)?;
        let source = if self.is_stage_path() {
            self.parse_stage_path()?
        } else {
            self.expect_token(TokenType::LParen)?;
            if !(self.check(TokenType::Select) || self.check(TokenType::With)) {
                return Err(self.parse_error("Expected a query in the parentheses after FROM"));
            }
            let query = self.parse_statement()?;
            self.expect_token(TokenType::RParen)?;
            query
        };
        properties.push(property("FROM", None, Some(source)));
        if !self.match_text_seq(&["MODEL"]) {
            return Err(self.parse_error("Expected MODEL = <model_name>"));
        }
        self.expect_token(TokenType::Eq)?;
        let model = Expression::Table(Box::new(self.parse_object_name()?));
        properties.push(property("MODEL", None, Some(model)));
        while let Some(key) = self.next_execute_option(INFERENCE_OPTIONS, &[], &properties) {
            let value = match key {
                "FUNCTION" => self.parse_string_value()?,
                "ASYNC" => self.parse_execute_boolean()?,
                "REPLICAS" if self.check(TokenType::Number) => self.parse_primary()?,
                "REPLICAS" => return Err(self.parse_error("Expected a number of replicas")),
                _ => Expression::Table(Box::new(self.parse_object_name()?)),
            };
            properties.push(property(key, None, Some(value)));
        }
        Ok(properties)
    }

    /// `<key> = <value>` for each of `keys`, in order.
    fn parse_execute_required(&mut self, keys: &[&str]) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        for key in keys {
            if !self.match_text_seq(&[key]) {
                return Err(self.parse_error(format!("Expected {key} =")));
            }
            self.expect_token(TokenType::Eq)?;
            let value = self.parse_execute_value()?;
            properties.push(property(key, None, Some(value)));
        }
        Ok(properties)
    }

    /// The next option among `keys`, followed by `=`, not given before; its key and `=` are consumed.
    fn next_execute_option(
        &mut self,
        keys: &[&'static str],
        given: &[ObjectProperty],
        properties: &[ObjectProperty],
    ) -> Option<&'static str> {
        let text = self.peek_text().to_ascii_uppercase();
        let key = keys.iter().find(|key| **key == text)?;
        let seen = given.iter().chain(properties).any(|seen| seen.kind == *key);
        if seen || !self.check_next(TokenType::Eq) {
            return None;
        }
        self.skip();
        self.skip();
        Some(key)
    }

    /// Options among `keys` in any order, each at most once, counting those already in `given`.
    fn parse_execute_options(
        &mut self,
        keys: &[&'static str],
        given: &[ObjectProperty],
    ) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while let Some(key) = self.next_execute_option(keys, given, &properties) {
            let value = match key {
                "ENV_VARS" => self.parse_execute_list(Self::parse_execute_pair)?,
                "IMPORTS" => self.parse_execute_list(Self::parse_execute_import)?,
                _ if LIST_OPTIONS.contains(&key) => {
                    self.parse_execute_list(Self::parse_execute_value)?
                }
                _ => self.parse_execute_value()?,
            };
            properties.push(property(key, None, Some(value)));
        }
        Ok(properties)
    }

    /// A value: a string, a number, TRUE or FALSE, or a name with no empty part.
    fn parse_execute_value(&mut self) -> Result<Expression> {
        let value = self.parse_object_value()?;
        if let Expression::Table(table) = &value {
            let parts = [
                Some(&table.name),
                table.schema.as_ref(),
                table.catalog.as_ref(),
            ];
            if table.schema_omitted || parts.into_iter().flatten().any(|part| part.name.is_empty())
            {
                return Err(self.parse_error("Expected a name with no empty part"));
            }
        }
        Ok(value)
    }

    /// TRUE or FALSE.
    fn parse_execute_boolean(&mut self) -> Result<Expression> {
        if !(self.check(TokenType::True) || self.check(TokenType::False)) {
            return Err(self.parse_error("Expected TRUE or FALSE"));
        }
        self.parse_primary()
    }

    /// The specification text: a dollar-quoted or quoted string.
    pub(super) fn parse_execute_specification(&mut self) -> Result<Expression> {
        if !(self.check(TokenType::DollarString) || self.check(TokenType::String)) {
            return Err(self.parse_error("Expected the specification text"));
        }
        self.parse_primary()
    }

    /// `<key> = <value>` inside ENV_VARS.
    fn parse_execute_pair(&mut self) -> Result<Expression> {
        let key = self.parse_execute_value()?;
        self.expect_token(TokenType::Eq)?;
        let value = self.parse_execute_value()?;
        Ok(Expression::Tuple(Box::new(Tuple {
            expressions: vec![key, value],
        })))
    }

    /// An IMPORTS entry: a location or a system function call, with an optional `AS '<alias>'`.
    fn parse_execute_import(&mut self) -> Result<Expression> {
        let source = match self.check_next(TokenType::LParen) {
            true => self.parse_primary()?,
            false => self.parse_execute_value()?,
        };
        if !self.match_token(TokenType::As) {
            return Ok(source);
        }
        let alias = self.parse_string_value()?;
        Ok(Expression::Tuple(Box::new(Tuple {
            expressions: vec![source, alias],
        })))
    }

    /// `( <name> => <value> [ , ... ] )` after USING.
    pub(super) fn parse_execute_using(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut items: Vec<Expression> = Vec::new();
        loop {
            let name = Expression::Identifier(self.parse_simple_name()?);
            self.expect_token(TokenType::FArrow)?;
            let value = self.parse_expression()?;
            items.push(Expression::Tuple(Box::new(Tuple {
                expressions: vec![name, value],
            })));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions: items })))
    }

    /// `( [ <item> [ , ... ] ] )`, possibly empty.
    fn parse_execute_list(
        &mut self,
        item: fn(&mut Self) -> Result<Expression>,
    ) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut items: Vec<Expression> = Vec::new();
        while !self.check(TokenType::RParen) {
            if !items.is_empty() {
                self.expect_token(TokenType::Comma)?;
            }
            items.push(item(self)?);
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions: items })))
    }
}
