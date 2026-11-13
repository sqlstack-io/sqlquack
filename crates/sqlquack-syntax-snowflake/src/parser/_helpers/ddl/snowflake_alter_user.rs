//! Snowflake ALTER USER (credentials, MFA, delegated authorizations, policies, properties, tags) and ALTER SHARE.

use crate::parser::_helpers::ddl::snowflake_alter::{alter_object, property, property_kind};
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::TableRef;
use sqlquack_syntax::expressions::schema_objects::models::ObjectProperty;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The words that open an ALTER USER action; a user name is absent when one of them comes first.
const USER_ACTIONS: &[&str] = &[
    "ABORT", "ADD", "ENROLL", "MODIFY", "REMOVE", "RENAME", "RESET", "ROTATE", "SET", "UNSET",
];

/// The credential kinds ADD, MODIFY, REMOVE and ROTATE take, longest first; PAT is the short spelling.
const CREDENTIALS: &[&[&str]] = &[
    &["PROGRAMMATIC", "ACCESS", "TOKEN"],
    &["WORKLOAD", "IDENTITY"],
    &["KEY", "PAIR"],
    &["PAT"],
];

/// The policies a user SET or UNSET names.
const USER_POLICIES: &[&[&str]] = &[
    &["AUTHENTICATION", "POLICY"],
    &["INGRESS", "POLICY"],
    &["PASSWORD", "POLICY"],
    &["SESSION", "POLICY"],
];

/// The operands and properties of one action.
type ActionParts = (Vec<Expression>, Vec<ObjectProperty>);

/// An action's words with its operands and properties.
type Action = (String, ActionParts);

impl Parser {
    /// Parse ALTER USER after its kind; the name may be left out for the current user, except to rename, enroll MFA, or set or unset a policy or tags.
    pub(super) fn parse_alter_user(&mut self) -> Result<Expression> {
        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let unnamed = self
            .text_at(0)
            .is_some_and(|word| USER_ACTIONS.contains(&word.as_str()))
            && !self
                .text_at(1)
                .is_some_and(|word| USER_ACTIONS.contains(&word.as_str()));
        let name = match unnamed {
            true => None,
            false => Some(self.parse_object_name()?),
        };
        let action = self.expect_alter_words(&[
            &["ABORT", "ALL", "QUERIES"],
            &["ENROLL", "MFA"],
            &["RESET", "PASSWORD"],
            &["RENAME", "TO"],
            &["ADD"],
            &["MODIFY"],
            &["REMOVE"],
            &["ROTATE"],
            &["SET"],
            &["UNSET"],
        ])?;
        let tag = self.check_identifier("TAG")
            && !self.check_next(TokenType::Eq)
            && !self.is_statement_end_at(1);
        let policy = USER_POLICIES.iter().any(|words| self.check_text_seq(words));
        let named_only = match action.as_str() {
            "RENAME TO" | "ENROLL MFA" => true,
            "SET" | "UNSET" => tag || policy,
            _ => false,
        };
        if unnamed && named_only {
            return Err(self.parse_error(format!("Expected a user name before {action}")));
        }
        let mut existence = (false, false);
        let (action, parts): Action = match action.as_str() {
            "RENAME TO" => (action, (vec![self.parse_user_operand()?], vec![])),
            "ADD" | "MODIFY" | "REMOVE" | "ROTATE" => {
                self.parse_user_credential_action(action, &mut existence)?
            }
            "SET" | "UNSET" => self.parse_user_set(action)?,
            _ => (action, (vec![], vec![])),
        };
        let Expression::AlterObject(mut alter) =
            alter_object("USER", (TableRef::new(""), if_exists), action, parts)
        else {
            return Err(self.parse_error("Expected an ALTER statement"));
        };
        alter.name = name;
        (alter.operand_if_exists, alter.operand_if_not_exists) = existence;
        Ok(Expression::AlterObject(alter))
    }

    /// The object of ADD, MODIFY, REMOVE or ROTATE: a credential, an MFA method or a delegated authorization; a token's IF [NOT] EXISTS goes to `existence`.
    fn parse_user_credential_action(
        &mut self,
        mut action: String,
        existence: &mut (bool, bool),
    ) -> Result<Action> {
        let verb = action.clone();
        if self.match_text_seq(&["MFA", "METHOD"]) {
            action.push_str(" MFA METHOD");
            return self.parse_user_mfa_method(&verb, action);
        }
        if self.check_identifier("DELEGATED") && matches!(verb.as_str(), "ADD" | "REMOVE") {
            self.skip();
            action.push_str(" DELEGATED");
            return self.parse_user_delegation(&verb, action);
        }
        let kind = self.expect_alter_words(CREDENTIALS)?;
        let kind = match kind.as_str() {
            "PAT" => "PROGRAMMATIC ACCESS TOKEN".to_string(),
            _ => kind,
        };
        action.push(' ');
        action.push_str(&kind);
        let token = matches!(kind.as_str(), "PROGRAMMATIC ACCESS TOKEN");
        if token && matches!(verb.as_str(), "ADD") && self.check(TokenType::If) {
            self.expect_alter_words(&[&["IF", "NOT", "EXISTS"]])?;
            existence.1 = true;
        } else if token && self.match_keywords(&[TokenType::If, TokenType::Exists]) {
            existence.0 = true;
        }
        let operands = vec![self.parse_user_operand()?];
        let properties = match verb.as_str() {
            "MODIFY" => {
                let change = self.expect_alter_words(&[&["RENAME", "TO"], &["SET"], &["UNSET"]])?;
                action.push(' ');
                action.push_str(&change);
                let parts: ActionParts = match change.as_str() {
                    "RENAME TO" => (
                        vec![operands[0].clone(), self.parse_user_operand()?],
                        vec![],
                    ),
                    "SET" => (operands, self.parse_user_properties()?),
                    _ => (operands, self.parse_user_unset_names()?),
                };
                return Ok((action, parts));
            }
            "REMOVE" => vec![],
            _ if self.is_statement_end_at(0) => vec![],
            _ => self.parse_user_properties()?,
        };
        Ok((action, (operands, properties)))
    }

    /// ADD MFA METHOD OTP [ COUNT = n ], REMOVE MFA METHOD <m> or MODIFY MFA METHOD <m> SET COMMENT = '<string>'.
    fn parse_user_mfa_method(&mut self, verb: &str, mut action: String) -> Result<Action> {
        if matches!(verb, "ROTATE") {
            return Err(self.parse_error("Snowflake has no ROTATE MFA METHOD"));
        }
        let method = self.parse_user_operand()?;
        if matches!(verb, "MODIFY") {
            action.push_str(" SET");
            self.expect_alter_words(&[&["SET"]])?;
            return Ok((action, (vec![method], self.parse_user_properties()?)));
        }
        let properties = match verb {
            "ADD" if !self.is_statement_end_at(0) => self.parse_user_properties()?,
            _ => vec![],
        };
        Ok((action, (vec![method], properties)))
    }

    /// ADD DELEGATED AUTHORIZATION OF ROLE <r> TO SECURITY INTEGRATION <i>, or its REMOVE forms, AUTHORIZATIONS [ OF ANY ROLE ] included.
    fn parse_user_delegation(&mut self, verb: &str, mut action: String) -> Result<Action> {
        let mut operands: Vec<Expression> = Vec::new();
        let what = match verb {
            "ADD" => self.expect_alter_words(&[&["AUTHORIZATION", "OF", "ROLE"]])?,
            _ => self.expect_alter_words(&[
                &["AUTHORIZATION", "OF", "ROLE"],
                &["AUTHORIZATIONS", "OF", "ANY", "ROLE"],
                &["AUTHORIZATIONS"],
            ])?,
        };
        action.push(' ');
        action.push_str(&what);
        if matches!(what.as_str(), "AUTHORIZATION OF ROLE") {
            operands.push(self.parse_user_operand()?);
        }
        let preposition = match verb {
            "ADD" => self.expect_alter_words(&[&["TO", "SECURITY", "INTEGRATION"]])?,
            _ => self.expect_alter_words(&[&["FROM", "SECURITY", "INTEGRATION"]])?,
        };
        action.push(' ');
        action.push_str(&preposition);
        operands.push(self.parse_user_operand()?);
        Ok((action, (operands, vec![])))
    }

    /// SET or UNSET: a policy, tags, or properties and parameters.
    fn parse_user_set(&mut self, mut action: String) -> Result<Action> {
        let set = matches!(action.as_str(), "SET");
        if let Some(policy) = USER_POLICIES
            .iter()
            .find(|words| self.match_text_seq(words))
        {
            action.push(' ');
            action.push_str(&policy.join(" "));
            if !set {
                return Ok((action, (vec![], vec![])));
            }
            let operands = vec![self.parse_user_operand()?];
            if self.match_identifier("FORCE") {
                action.push_str(" FORCE");
            }
            return Ok((action, (operands, vec![])));
        }
        if self.check_identifier("TAG") && !self.check_next(TokenType::Eq) {
            action.push_str(" TAG");
            let tags = self.parse_alter_object_properties(set, false)?;
            return Ok((action, (vec![], tags)));
        }
        let properties = match set {
            true => self.parse_user_properties()?,
            false => self.parse_user_unset_names()?,
        };
        Ok((action, (vec![], properties)))
    }

    /// `<name> = <value>` properties separated by blanks or commas; a value may be a parenthesized list.
    fn parse_user_properties(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let name = self.parse_simple_name()?;
            self.expect_token(TokenType::Eq)?;
            let value = match self.check(TokenType::LParen) {
                true => self.parse_routine_value_list()?,
                false => self.parse_object_value()?,
            };
            properties.push(property(&property_kind(&name), None, Some(value)));
            let comma = self.match_token(TokenType::Comma);
            if self.is_statement_end_at(0) {
                if comma {
                    return Err(self.parse_error("Expected a property after the comma"));
                }
                return Ok(properties);
            }
        }
    }

    /// UNSET names, separated by commas.
    fn parse_user_unset_names(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let name = self.parse_simple_name()?;
            properties.push(property(&property_kind(&name), None, None));
            if !self.match_token(TokenType::Comma) {
                return Ok(properties);
            }
        }
    }

    /// A user operand: a credential, method, role, integration or policy name.
    fn parse_user_operand(&mut self) -> Result<Expression> {
        Ok(Expression::Table(Box::new(self.parse_object_name()?)))
    }

    /// Parse ALTER SHARE after its kind: ADD or REMOVE ACCOUNTS, SET and UNSET.
    pub(super) fn parse_alter_share(&mut self) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let mut action = self.expect_alter_words(&[&["ADD"], &["REMOVE"], &["SET"], &["UNSET"]])?;
        let parts: ActionParts = match action.as_str() {
            "ADD" | "REMOVE" => {
                self.expect_alter_words(&[&["ACCOUNTS"]])?;
                action.push_str(" ACCOUNTS");
                self.expect_token(TokenType::Eq)?;
                let accounts = self.parse_account_names()?;
                let mut properties: Vec<ObjectProperty> = Vec::new();
                if matches!(action.as_str(), "ADD ACCOUNTS")
                    && self.match_identifier("SHARE_RESTRICTIONS")
                {
                    self.expect_token(TokenType::Eq)?;
                    properties.push(property(
                        "SHARE_RESTRICTIONS",
                        None,
                        Some(self.parse_object_value()?),
                    ));
                }
                (accounts, properties)
            }
            "SET" if self.check_identifier("TAG") && !self.check_next(TokenType::Eq) => {
                action.push_str(" TAG");
                (vec![], self.parse_alter_object_properties(true, false)?)
            }
            "SET" => (vec![], self.parse_share_set()?),
            _ => (vec![], self.parse_alter_object_properties(false, false)?),
        };
        Ok(alter_object("SHARE", head, action, parts))
    }

    /// A share's SET properties; ACCOUNTS takes a list of accounts.
    fn parse_share_set(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let name = self.parse_simple_name()?;
            self.expect_token(TokenType::Eq)?;
            let kind = property_kind(&name);
            let value = match kind.as_str() {
                "ACCOUNTS" => Expression::Tuple(Box::new(Tuple {
                    expressions: self.parse_account_names()?,
                })),
                _ => self.parse_object_value()?,
            };
            properties.push(property(&kind, None, Some(value)));
            self.match_token(TokenType::Comma);
            if self.is_statement_end_at(0) {
                return Ok(properties);
            }
        }
    }
}
