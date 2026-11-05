//! Snowflake data metric function actions in ALTER TABLE and ALTER VIEW.

use crate::expressions::columns::models::DataMetricExpectation;
use crate::expressions::columns::models::DataMetricFunctionAction;
use crate::expressions::columns::models::DataMetricFunctionClause;
use crate::expressions::columns::models::DataMetricFunctionItem;
use crate::expressions::tree::models::Expression;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// `{ ADD | DROP | MODIFY } DATA METRIC FUNCTION <item>, ...` in a Snowflake ALTER TABLE or VIEW, or none.
    pub(crate) fn parse_data_metric_function_action(
        &mut self,
    ) -> Result<Option<DataMetricFunctionAction>> {
        if !self.is_snowflake() {
            return Ok(None);
        }
        let kind = if self.check(TokenType::Add) {
            "ADD"
        } else if self.check(TokenType::Drop) {
            "DROP"
        } else if self.check_identifier("MODIFY") {
            "MODIFY"
        } else {
            return Ok(None);
        };
        let start = self.current;
        self.skip();
        if !self.match_text_seq(&["DATA", "METRIC", "FUNCTION"]) {
            self.current = start;
            return Ok(None);
        }
        let mut items = vec![self.parse_data_metric_function_item(kind)?];
        while self.match_token(TokenType::Comma) {
            items.push(self.parse_data_metric_function_item(kind)?);
        }
        Ok(Some(DataMetricFunctionAction {
            kind: kind.to_owned(),
            items,
        }))
    }

    /// `<name> ON ( [ <argument>, ... ] )` with the clauses its action allows.
    fn parse_data_metric_function_item(&mut self, kind: &str) -> Result<DataMetricFunctionItem> {
        let name = self.parse_object_name_ref()?;
        if !self.match_token(TokenType::On) {
            return Err(self.parse_error("Expected ON after a data metric function name"));
        }
        let arguments = self.parse_data_metric_arguments()?;
        let mut clauses: Vec<DataMetricFunctionClause> = Vec::new();
        match kind {
            "ADD" => {
                while let Some(clause) = self.parse_data_metric_add_clause()? {
                    clauses.push(clause);
                }
            }
            "MODIFY" => clauses.push(self.parse_data_metric_modify_clause()?),
            _ => {}
        }
        Ok(DataMetricFunctionItem {
            name,
            arguments,
            clauses,
        })
    }

    /// `( [ <column>, ... ] [ , TABLE ( <name> ( <column>, ... ) ) | , <lambda> ] )`; the column list may be empty.
    fn parse_data_metric_arguments(&mut self) -> Result<Vec<Expression>> {
        self.expect_token(TokenType::LParen)?;
        let leading_comma = self.match_token(TokenType::Comma);
        let arguments = if self.check(TokenType::RParen) && !leading_comma {
            Vec::new()
        } else {
            self.parse_function_arguments()?
        };
        self.expect_token(TokenType::RParen)?;
        let column = |argument: &Expression| matches!(argument, Expression::Column(_));
        let last_allowed = |argument: &Expression| match argument {
            Expression::Lambda(_) => true,
            Expression::Function(table) if table.name.eq_ignore_ascii_case("TABLE") => {
                matches!(table.args.as_slice(), [Expression::Function(source)]
                    if !source.args.is_empty() && source.args.iter().all(column))
            }
            _ => false,
        };
        let valid = match arguments.split_last() {
            None => true,
            Some((last, columns)) if leading_comma => columns.is_empty() && last_allowed(last),
            Some((last, columns)) => {
                columns.iter().all(column) && (column(last) || last_allowed(last))
            }
        };
        if !valid {
            return Err(self.parse_error(
                "A data metric function takes columns, then optionally TABLE ( <name> ( <column>, ... ) ) or a lambda",
            ));
        }
        Ok(arguments)
    }

    /// A clause of ADD DATA METRIC FUNCTION, or none.
    fn parse_data_metric_add_clause(&mut self) -> Result<Option<DataMetricFunctionClause>> {
        if self.check_identifier("SENSITIVITY") || self.check_identifier("ANOMALY_DETECTION") {
            let name = self.advance_text()?.to_ascii_uppercase();
            self.expect_token(TokenType::Eq)?;
            let value = self.parse_primary()?;
            return Ok(Some(DataMetricFunctionClause::Property { name, value }));
        }
        if self.match_identifier("EXPECTATION") {
            return Ok(Some(DataMetricFunctionClause::Expectations {
                action: None,
                expectations: self.parse_data_metric_expectations(true)?,
            }));
        }
        if self.match_text_seq(&["WITHIN", "GROUP"]) {
            let columns = self.parse_wrapped_csv_expressions()?;
            if columns.is_empty()
                || !columns
                    .iter()
                    .all(|column| matches!(column, Expression::Column(_)))
            {
                return Err(self.parse_error("WITHIN GROUP takes columns"));
            }
            let limit = if self.match_text_seq(&["GROUP", "LIMIT"]) {
                Some(self.parse_primary()?)
            } else {
                None
            };
            return Ok(Some(DataMetricFunctionClause::WithinGroup {
                columns,
                limit,
            }));
        }
        if self.match_text_seq(&["EXECUTE", "AS", "ROLE"]) {
            let role = self.expect_identifier_or_keyword_with_quoted()?;
            self.reject_reserved_name_part(&role)?;
            return Ok(Some(DataMetricFunctionClause::ExecuteAsRole(role)));
        }
        if self.check(TokenType::Filter) {
            let filter = self.parse_data_metric_filter()?;
            if filter == DataMetricFunctionClause::Filter(None) {
                return Err(
                    self.parse_error("ADD DATA METRIC FUNCTION ... FILTER needs a condition")
                );
            }
            return Ok(Some(filter));
        }
        Ok(None)
    }

    /// The one clause of a MODIFY DATA METRIC FUNCTION item.
    fn parse_data_metric_modify_clause(&mut self) -> Result<DataMetricFunctionClause> {
        if self.check_identifier("SUSPEND") || self.check_identifier("RESUME") {
            return Ok(DataMetricFunctionClause::State(
                self.advance_text()?.to_ascii_uppercase(),
            ));
        }
        if self.check(TokenType::Filter) {
            return self.parse_data_metric_filter();
        }
        let (action, with_condition) = if self.match_token(TokenType::Add) {
            ("ADD", true)
        } else if self.match_token(TokenType::Drop) {
            ("DROP", false)
        } else if self.match_identifier("MODIFY") {
            ("MODIFY", true)
        } else {
            return Err(self.parse_error(
                "Expected SUSPEND, RESUME, FILTER or an EXPECTATION change after the arguments",
            ));
        };
        if !self.match_identifier("EXPECTATION") {
            return Err(self.parse_error("Expected EXPECTATION"));
        }
        Ok(DataMetricFunctionClause::Expectations {
            action: Some(action.to_owned()),
            expectations: self.parse_data_metric_expectations(with_condition)?,
        })
    }

    /// `FILTER ( [ <condition> ] )`.
    fn parse_data_metric_filter(&mut self) -> Result<DataMetricFunctionClause> {
        self.skip();
        self.expect_token(TokenType::LParen)?;
        let condition = if self.check(TokenType::RParen) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect_token(TokenType::RParen)?;
        Ok(DataMetricFunctionClause::Filter(condition))
    }

    /// `<name> ( <condition> ), ...`, or bare names under DROP; a comma before `<name> ON` ends the list.
    fn parse_data_metric_expectations(
        &mut self,
        with_condition: bool,
    ) -> Result<Vec<DataMetricExpectation>> {
        let mut expectations: Vec<DataMetricExpectation> = Vec::new();
        loop {
            let name = self.expect_identifier_or_keyword_with_quoted()?;
            self.reject_reserved_name_part(&name)?;
            let condition = if with_condition {
                self.expect_token(TokenType::LParen)?;
                let condition = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                Some(condition)
            } else {
                None
            };
            expectations.push(DataMetricExpectation { name, condition });
            let next = self
                .tokens
                .get(self.current + 1)
                .map(|token| token.token_type);
            let after = self
                .tokens
                .get(self.current + 2)
                .map(|token| token.token_type);
            let continues = if with_condition {
                after == Some(TokenType::LParen)
            } else {
                !matches!(after, Some(TokenType::On) | Some(TokenType::Dot))
                    && next != Some(TokenType::Semicolon)
            };
            if !(self.check(TokenType::Comma) && continues) {
                return Ok(expectations);
            }
            self.skip();
        }
    }
}
