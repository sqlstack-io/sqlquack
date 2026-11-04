//! Snowflake anonymous procedures, `WITH <name> AS PROCEDURE ... CALL <name>(...)`, as documented.

use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::TableRef;
use crate::expressions::operators::models::Call;
use crate::expressions::queries::models::With;
use crate::expressions::schema_objects::models::CreateProcedure;
use crate::expressions::schema_objects::models::FunctionBody;
use crate::expressions::schema_objects::models::FunctionParameter;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::StructField;
use crate::parser::models::Dialect;
use crate::parser::models::Parser;
use crate::tokens::models::CallMode;
use crate::tokens::models::DialectType;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

/// Handler languages Snowflake accepts for an anonymous procedure.
const LANGUAGES: [&str; 5] = ["SQL", "JAVASCRIPT", "PYTHON", "JAVA", "SCALA"];

impl Parser {
    /// Whether the CTE body after `AS` is a Snowflake anonymous procedure.
    pub(crate) fn at_anonymous_procedure(&self) -> bool {
        matches!(self.config.dialect, Some(DialectType::Snowflake))
            && self.check(TokenType::Procedure)
    }

    /// The procedure after `<name> AS`: parameters, RETURNS, LANGUAGE, handler options and body.
    pub(crate) fn parse_anonymous_procedure(&mut self, name: &Identifier) -> Result<Expression> {
        self.expect_token(TokenType::Procedure)?;
        let parameters = self.parse_anonymous_procedure_parameters()?;
        if !self.match_token(TokenType::Returns) {
            return Err(self.parse_error("Expected RETURNS after the procedure parameters"));
        }
        let (return_type, returns_table) = if self.match_token(TokenType::Table) {
            (None, Some(self.parse_procedure_table_columns()?))
        } else {
            let data_type = self.parse_data_type()?;
            if !self.match_keywords(&[TokenType::Not, TokenType::Null]) {
                self.match_token(TokenType::Null);
            }
            (Some(data_type), None)
        };
        if !self.match_token(TokenType::Language) {
            return Err(self.parse_error("Expected LANGUAGE after the procedure return type"));
        }
        let language = self.expect_identifier_or_keyword()?.to_ascii_uppercase();
        if !LANGUAGES.contains(&language.as_str()) {
            return Err(self.parse_error(format!("Unknown procedure language {language}")));
        }
        let mut procedure = CreateProcedure::new("");
        procedure.name = TableRef::new("");
        procedure.name.name = name.clone();
        procedure.parameters = parameters;
        procedure.return_type = return_type;
        procedure.returns_table = returns_table;
        procedure.language = Some(language);
        let procedure = self.parse_anonymous_procedure_options(procedure)?;
        Ok(Expression::CreateProcedure(Box::new(procedure)))
    }

    /// `( [<name> <type> [DEFAULT <expr>]] [, ...] )` with no empty or trailing entries.
    fn parse_anonymous_procedure_parameters(&mut self) -> Result<Vec<FunctionParameter>> {
        self.expect_token(TokenType::LParen)?;
        let mut parameters: Vec<FunctionParameter> = Vec::new();
        while !self.check(TokenType::RParen) {
            if !parameters.is_empty() {
                self.expect_token(TokenType::Comma)?;
            }
            let name = self.expect_procedure_part_name()?;
            let data_type = self.parse_data_type()?;
            let default = if self.match_token(TokenType::Default) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            parameters.push(FunctionParameter {
                name: Some(name),
                data_type,
                mode: None,
                default,
                mode_text: None,
            });
        }
        self.expect_token(TokenType::RParen)?;
        Ok(parameters)
    }

    /// `( [<column> <type>] [, ...] )` after `RETURNS TABLE`.
    fn parse_procedure_table_columns(&mut self) -> Result<Vec<StructField>> {
        self.expect_token(TokenType::LParen)?;
        let mut columns: Vec<StructField> = Vec::new();
        while !self.check(TokenType::RParen) {
            if !columns.is_empty() {
                self.expect_token(TokenType::Comma)?;
            }
            let name = self.expect_procedure_part_name()?;
            columns.push(StructField::new(name.name, self.parse_data_type()?));
        }
        self.expect_token(TokenType::RParen)?;
        Ok(columns)
    }

    /// A parameter or column name: an identifier, quoted or not a reserved word.
    fn expect_procedure_part_name(&mut self) -> Result<Identifier> {
        if !(self.check(TokenType::Identifier)
            || self.check(TokenType::QuotedIdentifier)
            || self.check(TokenType::Var)
            || self.check_keyword())
        {
            return Err(self.parse_error("Expected a parameter or column name"));
        }
        let name = self.expect_identifier_or_keyword_with_quoted()?;
        self.reject_reserved_name_part(&name)?;
        Ok(name)
    }

    /// Handler options and the `AS` body, in any order, each at most once.
    fn parse_anonymous_procedure_options(
        &mut self,
        mut procedure: CreateProcedure,
    ) -> Result<CreateProcedure> {
        loop {
            let option = if self.match_identifier("RUNTIME_VERSION")
                || self.match_identifier("HANDLER")
            {
                let key = self.previous_text().to_ascii_uppercase();
                self.expect_token(TokenType::Eq)?;
                format!("{key} = '{}'", self.expect_string()?)
            } else if self.match_identifier("PACKAGES") || self.match_identifier("IMPORTS") {
                let key = self.previous_text().to_ascii_uppercase();
                self.expect_token(TokenType::Eq)?;
                format!(
                    "{key} = ({})",
                    self.parse_procedure_string_list()?.join(", ")
                )
            } else if self.match_text_seq(&["CALLED", "ON", "NULL", "INPUT"]) {
                "CALLED ON NULL INPUT".to_string()
            } else if self.match_text_seq(&["RETURNS", "NULL", "ON", "NULL", "INPUT"]) {
                "RETURNS NULL ON NULL INPUT".to_string()
            } else if self.match_identifier("STRICT") {
                "STRICT".to_string()
            } else if procedure.body.is_none() && self.match_token(TokenType::As) {
                let body = self.parse_anonymous_procedure_body(procedure.language.as_deref())?;
                procedure.body = Some(body);
                continue;
            } else {
                return Ok(procedure);
            };
            let key = option.split(' ').next().unwrap_or_default().to_string();
            if procedure
                .with_options
                .iter()
                .any(|seen| seen.starts_with(&key))
            {
                return Err(self.parse_error(format!("{key} is given more than once")));
            }
            procedure.with_options.push(option);
        }
    }

    /// `( '<value>' [, ...] )`, possibly empty.
    fn parse_procedure_string_list(&mut self) -> Result<Vec<String>> {
        self.expect_token(TokenType::LParen)?;
        let mut values: Vec<String> = Vec::new();
        while !self.check(TokenType::RParen) {
            if !values.is_empty() {
                self.expect_token(TokenType::Comma)?;
            }
            values.push(format!("'{}'", self.expect_string()?));
        }
        self.expect_token(TokenType::RParen)?;
        Ok(values)
    }

    /// The body string; a LANGUAGE SQL body must parse as Snowflake Scripting.
    fn parse_anonymous_procedure_body(&mut self, language: Option<&str>) -> Result<FunctionBody> {
        if !(self.check(TokenType::String) || self.check(TokenType::DollarString)) {
            return Err(self.parse_error("Expected the procedure body as a string after AS"));
        }
        let body = self.advance_text()?;
        if language != Some("SQL") {
            return Ok(FunctionBody::StringLiteral(body));
        }
        let mode = if self.config.faithful_calls {
            CallMode::Faithful
        } else {
            CallMode::Lowered
        };
        let statements = Dialect::get(DialectType::Snowflake)
            .parse_with_config(&body, self.config.complexity_guard.clone(), mode)
            .map_err(|error| self.parse_error(format!("In the procedure body: {error}")))?;
        if statements.is_empty() {
            return Err(self.parse_error("Expected a procedure body"));
        }
        Ok(FunctionBody::Statements(statements))
    }

    /// `CALL <name> ( [[<arg> =>] <value>, ...] ) [INTO :<variable>]` after a WITH clause.
    pub(crate) fn parse_call_with(&mut self, with: With) -> Result<Expression> {
        if !self.check_identifier("CALL") {
            return Err(self.parse_error("Expected CALL after an anonymous procedure"));
        }
        self.skip();
        let mut name = vec![self.expect_identifier_or_keyword_with_quoted()?];
        while self.match_token(TokenType::Dot) {
            name.push(self.expect_identifier_or_keyword_with_quoted()?);
        }
        for part in &name {
            self.reject_reserved_name_part(part)?;
        }
        self.expect_token(TokenType::LParen)?;
        let arguments = if self.check(TokenType::RParen) {
            Vec::new()
        } else {
            self.parse_function_arguments()?
        };
        self.expect_token(TokenType::RParen)?;
        let into = if self.match_token(TokenType::Into) {
            self.expect_token(TokenType::Colon)?;
            let variable = self.expect_identifier_or_keyword_with_quoted()?;
            self.reject_reserved_name_part(&variable)?;
            Some(variable)
        } else {
            None
        };
        Ok(Expression::Call(Box::new(Call {
            source_dialect: None,
            name,
            member: None,
            arguments,
            asynchronous: false,
            hints: Vec::new(),
            with: Some(with),
            into,
        })))
    }
}
