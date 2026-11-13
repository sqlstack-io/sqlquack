//! Snowflake ALTER OPENFLOW CONNECTOR, DEPLOYMENT, RUNTIME and DATA PLANE INTEGRATION, ALTER DEPLOYMENT and ALTER ROLE.

use crate::parser::_helpers::ddl::snowflake_alter::{alter_object, property};
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The kinds this module reads, longest first.
const KINDS: &[&[&str]] = &[
    &["OPENFLOW", "DATA", "PLANE", "INTEGRATION"],
    &["OPENFLOW", "CONNECTOR"],
    &["OPENFLOW", "DEPLOYMENT"],
    &["OPENFLOW", "RUNTIME"],
    &["DEPLOYMENT"],
    &["ROLE"],
];

/// The actions of an Openflow connector, longest first.
const CONNECTOR_ACTIONS: &[&[&str]] = &[
    &["ADD", "LIVE", "VERSION", "FROM", "LAST"],
    &["ADD", "VERSION", "FROM"],
    &["RENAME", "TO"],
    &["ABORT"],
    &["COMMIT"],
    &["PULL"],
    &["PUSH"],
    &["SET"],
    &["START"],
    &["STOP"],
    &["TERMINATE"],
];

/// The actions of an Openflow deployment.
const DEPLOYMENT_ACTIONS: &[&[&str]] = &[
    &["RENAME", "TO"],
    &["SET"],
    &["UNSET"],
    &["TERMINATE"],
    &["UPGRADE"],
];

/// The actions of an Openflow runtime.
const RUNTIME_ACTIONS: &[&[&str]] = &[
    &["RENAME", "TO"],
    &["ADD"],
    &["REMOVE"],
    &["RESTART"],
    &["RESUME"],
    &["SET"],
    &["SUSPEND"],
    &["TERMINATE"],
    &["UNSET"],
    &["UPGRADE"],
];

/// The optional word after an action without operands, by the kinds it applies to.
const ACTION_SUFFIXES: &[(&[&str], &str, &[&str])] = &[
    (&["OPENFLOW CONNECTOR"], "TERMINATE", &["FORCE"]),
    (&["OPENFLOW RUNTIME"], "TERMINATE", &["CASCADE"]),
    (&["OPENFLOW RUNTIME"], "RESTART", &["RECOVERY"]),
    (&["OPENFLOW RUNTIME"], "RESUME", &["RECOVERY"]),
    (&["OPENFLOW RUNTIME"], "UPGRADE", &["RECOVERY", "FORCE"]),
];

/// The actions of a deployment without OPENFLOW, which Snowflake parses only to set or unset properties.
const BARE_DEPLOYMENT_ACTIONS: &[&[&str]] = &[&["SET"], &["UNSET"]];

/// The actions of a role.
const ROLE_ACTIONS: &[&[&str]] = &[&["RENAME", "TO"], &["SET"], &["UNSET"]];

impl Parser {
    /// Parse ALTER of an Openflow kind, DEPLOYMENT or ROLE, or None when the next words name another kind.
    pub(crate) fn parse_alter_openflow_or_role(&mut self) -> Result<Option<Expression>> {
        let Some(kind) = KINDS.iter().find(|words| self.match_text_seq(words)) else {
            return Ok(None);
        };
        let kind = kind.join(" ");
        let head = self.parse_alter_object_head()?;
        let actions = match kind.as_str() {
            "OPENFLOW CONNECTOR" => CONNECTOR_ACTIONS,
            "OPENFLOW DEPLOYMENT" => DEPLOYMENT_ACTIONS,
            "OPENFLOW RUNTIME" => RUNTIME_ACTIONS,
            "DEPLOYMENT" => BARE_DEPLOYMENT_ACTIONS,
            "ROLE" => ROLE_ACTIONS,
            _ => &[&["SET"][..]][..],
        };
        let mut action = self.expect_alter_words(actions)?;
        let parts = match action.as_str() {
            "RENAME TO" => (vec![self.object_operand()?], vec![]),
            "SET" => self.parse_alter_set(true)?,
            "UNSET" => self.parse_alter_set(false)?,
            "ADD VERSION FROM" => (vec![self.parse_string_value()?], vec![]),
            "ADD" | "REMOVE" => {
                self.expect_alter_words(&[&["EXTERNAL_ACCESS_INTEGRATIONS"]])?;
                self.expect_token(TokenType::Eq)?;
                if !self.check(TokenType::LParen) {
                    return Err(self.parse_error("Expected a parenthesized integration list"));
                }
                let integrations = self.parse_alter_value()?;
                (
                    vec![],
                    vec![property(
                        "EXTERNAL_ACCESS_INTEGRATIONS",
                        None,
                        Some(integrations),
                    )],
                )
            }
            "PUSH" => {
                let mut operands: Vec<Expression> = Vec::new();
                if self.match_token(TokenType::To) {
                    action.push_str(" TO");
                    operands.push(self.parse_string_value()?);
                }
                (operands, self.parse_alter_settings(true)?)
            }
            "COMMIT" | "ADD LIVE VERSION FROM LAST" if !self.is_statement_end_at(0) => {
                (vec![], self.parse_alter_settings(true)?)
            }
            _ => {
                let suffix = ACTION_SUFFIXES
                    .iter()
                    .find(|(kinds, name, _)| *name == action && kinds.contains(&kind.as_str()))
                    .map_or(&[][..], |(_, _, words)| *words);
                if let Some(word) = suffix.iter().find(|word| self.match_text_seq(&[word])) {
                    action.push(' ');
                    action.push_str(word);
                }
                (vec![], vec![])
            }
        };
        Ok(Some(alter_object(&kind, head, action, parts)))
    }
}
