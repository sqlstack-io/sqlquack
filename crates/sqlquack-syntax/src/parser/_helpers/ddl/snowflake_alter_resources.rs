//! Parser ddl: Snowflake ALTER of schemas, stages, file formats, routines, services and project objects.

use crate::expressions::names::models::Literal;
use crate::expressions::schema_objects::models::ObjectProperty;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::Tuple;
use crate::parser::_helpers::ddl::snowflake_alter::alter_object;
use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

/// The object kinds this module parses, as the words that follow ALTER.
const RESOURCE_KINDS: &[&[&str]] = &[
    &["MULTI", "PARTY", "APPROVAL", "POLICY"],
    &["STORAGE", "LIFECYCLE", "POLICY"],
    &["DATA", "MOVEMENT", "POLICY"],
    &["AGGREGATION", "POLICY"],
    &["AUTHENTICATION", "POLICY"],
    &["BACKUP", "POLICY"],
    &["FEATURE", "POLICY"],
    &["INGRESS", "POLICY"],
    &["JOIN", "POLICY"],
    &["MAINTENANCE", "POLICY"],
    &["MASKING", "POLICY"],
    &["NETWORK", "POLICY"],
    &["NETWORK", "RULE"],
    &["PACKAGES", "POLICY"],
    &["PASSWORD", "POLICY"],
    &["SESSION", "POLICY"],
    &["SNAPSHOT", "POLICY"],
    &["RESTRICTED", "SESSION", "SCOPE"],
    &["ARTIFACT", "REPOSITORY"],
    &["EXTERNAL", "VOLUME"],
    &["MATERIALIZED", "VIEW"],
    &["COMPUTE", "POOL"],
    &["DBT", "PROJECT"],
    &["DCM", "PROJECT"],
    &["EVENT", "ROUTING", "TABLE"],
    &["EXTERNAL", "AGENT"],
    &["EXTERNAL", "TABLE"],
    &["FILE", "FORMAT"],
    &["GIT", "REPOSITORY"],
    &["MODEL", "MONITOR"],
    &["AGENT"],
    &["DATASET"],
    &["PIPE"],
    &["RSS"],
    &["STREAM"],
    &["TYPE"],
    &["CONTACT"],
    &["GATEWAY"],
    &["LISTING"],
    &["MODEL"],
    &["NOTEBOOK"],
    &["PROCEDURE"],
    &["SCHEMA"],
    &["SERVICE"],
    &["STAGE"],
];

/// The operands and properties of one ALTER action.
type ActionParts = (Vec<Expression>, Vec<ObjectProperty>);

/// The actions an ALTER of `kind` takes, each as its words.
fn resource_actions(kind: &str) -> &'static [&'static [&'static str]] {
    match kind {
        "AGENT" => &[&["MODIFY", "LIVE", "VERSION", "SET"], &["SET"], &["UNSET"]],
        "COMPUTE POOL" => &[
            &["STOP", "ALL"],
            &["SUSPEND"],
            &["RESUME"],
            &["SET"],
            &["UNSET"],
        ],
        "DBT PROJECT" => &[&["DEPLOY", "FROM"], &["RENAME", "TO"], &["SET"], &["UNSET"]],
        "DCM PROJECT" => &[&["DROP", "DEPLOYMENT"], &["SET"], &["UNSET"]],
        "EVENT ROUTING TABLE" => &[
            &["MODIFY", "RULE"],
            &["RENAME", "TO"],
            &["FORCE", "SET", "RULES"],
            &["FORCE", "SET", "RULE"],
            &["FORCE", "UNSET", "RULE"],
            &["SET", "RULES"],
            &["SET", "RULE"],
            &["UNSET", "RULE"],
        ],
        "EXTERNAL AGENT" => &[&["ADD", "VERSION"], &["SET"], &["UNSET"]],
        "EXTERNAL TABLE" => &[
            &["ADD", "FILES"],
            &["REMOVE", "FILES"],
            &["ADD", "PARTITION"],
            &["DROP", "PARTITION", "LOCATION"],
            &["REFRESH"],
            &["SET"],
            &["UNSET"],
        ],
        "GATEWAY" => &[&["FROM", "SPECIFICATION"]],
        "GIT REPOSITORY" => &[&["FETCH"], &["SET"], &["UNSET"]],
        "LISTING" => &[
            &["PUBLISH"],
            &["UNPUBLISH"],
            &["REVIEW"],
            &["ADD", "TARGETS"],
            &["REMOVE", "TARGETS"],
            &["ADD", "VERSION"],
            &["AS"],
            &["RENAME", "TO"],
            &["SET"],
            &["UNSET"],
        ],
        "MODEL" => &[
            &["ADD", "VERSION"],
            &["DROP", "VERSION"],
            &["MODIFY", "VERSION"],
            &["VERSION"],
            &["RENAME", "TO"],
            &["SET"],
            &["UNSET"],
        ],
        "MODEL MONITOR" => &[
            &["ADD"],
            &["DROP"],
            &["SUSPEND"],
            &["RESUME"],
            &["SET"],
            &["UNSET"],
        ],
        "PROCEDURE" => &[&["EXECUTE", "AS"], &["RENAME", "TO"], &["SET"], &["UNSET"]],
        "SCHEMA" => &[
            &["ENABLE", "MANAGED", "ACCESS"],
            &["DISABLE", "MANAGED", "ACCESS"],
            &["RENAME", "TO"],
            &["SWAP", "WITH"],
            &["SET"],
            &["UNSET"],
        ],
        "SERVICE" => &[
            &["FROM"],
            &["RESTORE", "VOLUME"],
            &["SUSPEND"],
            &["RESUME"],
            &["SET"],
            &["UNSET"],
        ],
        "STAGE" | "PIPE" => &[&["REFRESH"], &["RENAME", "TO"], &["SET"], &["UNSET"]],
        "RESTRICTED SESSION SCOPE" | "RSS" => &[
            &["ADD"],
            &["REMOVE"],
            &["AS"],
            &["RENAME", "TO"],
            &["SET"],
            &["UNSET"],
        ],
        "EXTERNAL VOLUME" => &[
            &["ADD", "STORAGE_LOCATION"],
            &["REMOVE", "STORAGE_LOCATION"],
            &["UPDATE", "STORAGE_LOCATION"],
            &["RENAME", "TO"],
            &["SET"],
            &["UNSET"],
        ],
        "DATASET" => &[&["ADD", "VERSION"], &["DROP", "VERSION"]],
        "FEATURE POLICY" | "INGRESS POLICY" | "MULTI PARTY APPROVAL POLICY" => {
            &[&["AS"], &["RENAME", "TO"], &["SET"], &["UNSET"]]
        }
        "NETWORK POLICY" | "DATA MOVEMENT POLICY" => &[
            &["ADD"],
            &["REMOVE"],
            &["RENAME", "TO"],
            &["SET"],
            &["UNSET"],
        ],
        "MATERIALIZED VIEW" => &[
            &["CLUSTER", "BY"],
            &["DROP", "CLUSTERING", "KEY"],
            &["SUSPEND", "RECLUSTER"],
            &["RESUME", "RECLUSTER"],
            &["SUSPEND"],
            &["RESUME"],
            &["RENAME", "TO"],
            &["SET"],
            &["UNSET"],
        ],
        _ => &[&["RENAME", "TO"], &["SET"], &["UNSET"]],
    }
}

impl Parser {
    /// Parse a Snowflake ALTER of a kind in RESOURCE_KINDS, or None when the next words name another kind.
    pub(super) fn parse_alter_snowflake_resource(&mut self) -> Result<Option<Expression>> {
        let model_named_monitor = self.check_identifier("MODEL")
            && self.check_next_identifier("MONITOR")
            && matches!(self.text_at(2).as_deref(), Some("SET" | "UNSET"));
        let words: &[&str] = if model_named_monitor && self.match_identifier("MODEL") {
            &["MODEL"]
        } else if let Some(words) = RESOURCE_KINDS
            .iter()
            .find(|words| self.match_text_seq(words))
        {
            words
        } else if let Some(kind) = self.partial_resource_kind() {
            return Err(self.parse_error(format!("Snowflake has no ALTER {kind}")));
        } else {
            return Ok(None);
        };
        let kind = words.join(" ");
        let head = self.parse_alter_object_head()?;
        let arguments = match kind.as_str() {
            "PROCEDURE" => self.parse_routine_signature()?,
            _ => Vec::new(),
        };
        let action = self.expect_alter_words(resource_actions(&kind))?;
        let (action, parts) = self.parse_resource_action(&kind, action)?;
        let Expression::AlterObject(mut alter) = alter_object(&kind, head, action, parts) else {
            return Err(self.parse_error("Expected an ALTER statement"));
        };
        alter.arguments = arguments;
        Ok(Some(Expression::AlterObject(alter)))
    }

    /// The words of an incomplete multi-word kind, such as `COMPUTE` without `POOL`, if the next words are one.
    fn partial_resource_kind(&self) -> Option<String> {
        let first = self.text_at(0)?;
        let second = self.text_at(1).unwrap_or_default();
        let complete = match (first.as_str(), second.as_str()) {
            ("COMPUTE", "POOL")
            | ("DBT" | "DCM", "PROJECT")
            | ("GIT", "REPOSITORY")
            | ("FILE", "FORMAT")
            | ("ARTIFACT", "REPOSITORY")
            | ("MATERIALIZED", "VIEW")
            | ("EXTERNAL", "AGENT" | "TABLE" | "VOLUME" | "CONSUMER" | "ACCESS") => true,
            ("EVENT", "ROUTING") => matches!(self.text_at(2).as_deref(), Some("TABLE")),
            ("RESTRICTED", "SESSION") => matches!(self.text_at(2).as_deref(), Some("SCOPE")),
            ("STORAGE", "INTEGRATION") => return None,
            ("STORAGE", "LIFECYCLE") => matches!(self.text_at(2).as_deref(), Some("POLICY")),
            ("STORAGE", _) => false,
            ("BACKUP", "SET") => return None,
            (
                "COMPUTE" | "DBT" | "DCM" | "GIT" | "FILE" | "EXTERNAL" | "EVENT" | "ARTIFACT"
                | "MATERIALIZED" | "RESTRICTED" | "VOLUME" | "AGGREGATION" | "AUTHENTICATION"
                | "BACKUP" | "DATA" | "INGRESS" | "JOIN" | "MAINTENANCE" | "MASKING" | "MULTI"
                | "NETWORK" | "PACKAGES" | "PASSWORD",
                _,
            ) => false,
            _ => return None,
        };
        (!complete).then(|| format!("{first} {second}").trim_end().to_owned())
    }

    /// Parse the operands and properties of `action` on `kind`, extending `action` with its later keywords.
    fn parse_resource_action(
        &mut self,
        kind: &str,
        action: String,
    ) -> Result<(String, ActionParts)> {
        let key = action.clone();
        let parts = match (kind, key.as_str()) {
            ("MATERIALIZED VIEW", "SET") if self.check_identifier("SECURE") => {
                self.skip();
                let mut properties = vec![property("SECURE", None, None)];
                properties.extend(self.parse_resource_properties()?);
                (vec![], properties)
            }
            ("DATASET", "DROP VERSION") | (_, "REMOVE STORAGE_LOCATION") => {
                (vec![self.parse_string_value()?], vec![])
            }
            ("AGENT", "SET") if !self.check_identifier("TAG") => {
                (vec![], vec![self.parse_literal_property()?])
            }
            ("STORAGE LIFECYCLE POLICY", "SET") if self.check_identifier("BODY") => {
                self.skip();
                self.expect_token(TokenType::Arrow)?;
                (
                    vec![],
                    vec![property("BODY", None, Some(self.parse_expression()?))],
                )
            }
            (_, "SET" | "MODIFY LIVE VERSION SET") => (vec![], self.parse_resource_set()?),
            (_, "UNSET") => (vec![], self.parse_resource_unset()?),
            (
                _,
                "RENAME TO" | "SWAP WITH" | "DROP DEPLOYMENT" | "DROP VERSION" | "UNSET RULE"
                | "FORCE UNSET RULE",
            )
            | ("EXTERNAL AGENT", "ADD VERSION") => (vec![self.parse_name_operand()?], vec![]),
            (_, "DEPLOY FROM" | "DROP PARTITION LOCATION") => {
                (vec![self.parse_location_value()?], vec![])
            }
            ("LISTING", "AS") => {
                let manifest = self.parse_location_value()?;
                (vec![manifest], self.parse_resource_properties()?)
            }
            (_, "FROM SPECIFICATION") => (vec![self.parse_object_value()?], vec![]),
            (_, "ADD FILES" | "REMOVE FILES") => (vec![self.parse_string_list()?], vec![]),
            (_, "ADD TARGETS" | "REMOVE TARGETS") => (vec![self.parse_resource_value()?], vec![]),
            ("STAGE" | "PIPE", "REFRESH") => (vec![], self.parse_resource_properties()?),
            ("NETWORK POLICY" | "DATA MOVEMENT POLICY", "ADD" | "REMOVE") => {
                (vec![], self.parse_resource_set()?)
            }
            (
                "RESTRICTED SESSION SCOPE"
                | "RSS"
                | "FEATURE POLICY"
                | "INGRESS POLICY"
                | "MULTI PARTY APPROVAL POLICY",
                "AS",
            )
            | ("RESTRICTED SESSION SCOPE" | "RSS", "ADD" | "REMOVE") => {
                (vec![self.parse_text_value()?], vec![])
            }
            ("DATASET", "ADD VERSION") => return self.parse_dataset_version(action),
            (_, "ADD STORAGE_LOCATION") => {
                self.expect_token(TokenType::Eq)?;
                (vec![self.parse_value_list()?], vec![])
            }
            (_, "UPDATE STORAGE_LOCATION") => {
                self.expect_token(TokenType::Eq)?;
                let location = self.parse_object_value()?;
                (vec![location], self.parse_resource_set()?)
            }
            (_, "CLUSTER BY") => (vec![self.parse_expression_list_in_parens()?], vec![]),
            (_, "REFRESH") if self.check(TokenType::String) => {
                (vec![self.parse_string_value()?], vec![])
            }
            ("LISTING", "ADD VERSION") => return self.parse_listing_version(action),
            (_, "ADD VERSION" | "MODIFY VERSION" | "VERSION") => {
                return self.parse_model_version(action)
            }
            (_, "FROM") => self.parse_service_from()?,
            (
                _,
                "MODIFY RULE" | "SET RULE" | "FORCE SET RULE" | "SET RULES" | "FORCE SET RULES",
            ) => return self.parse_routing_rule(action),
            _ => return self.parse_resource_clause(action),
        };
        Ok((action, parts))
    }

    /// Parse a dataset's ADD VERSION after its keywords: `'<version>' FROM ( <query> ) [ PARTITION BY <expr> ]` and properties.
    fn parse_dataset_version(&mut self, mut action: String) -> Result<(String, ActionParts)> {
        let mut operands = vec![self.parse_string_value()?];
        self.expect_token(TokenType::From)?;
        self.expect_token(TokenType::LParen)?;
        if !self.check(TokenType::Select) && !self.check(TokenType::With) {
            return Err(self.parse_error("Expected a query"));
        }
        operands.push(self.parse_statement()?);
        self.expect_token(TokenType::RParen)?;
        if self.match_keywords(&[TokenType::Partition, TokenType::By]) {
            action.push_str(" PARTITION BY");
            operands.push(self.parse_expression()?);
        }
        Ok((action, (operands, self.parse_resource_properties()?)))
    }

    /// Parse `( <expr> [ , ... ] )` as one tuple.
    fn parse_expression_list_in_parens(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut expressions = vec![self.parse_expression()?];
        while self.match_token(TokenType::Comma) {
            expressions.push(self.parse_expression()?);
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions })))
    }

    /// Parse an agent's SET: one property with a literal value.
    fn parse_literal_property(&mut self) -> Result<ObjectProperty> {
        let name = self.parse_simple_name()?;
        self.expect_token(TokenType::Eq)?;
        if !self.is_literal_value() {
            return Err(self.parse_error("Expected a literal value"));
        }
        let value = self.parse_object_value()?;
        Ok(property(&name.name.to_ascii_uppercase(), None, Some(value)))
    }

    /// Parse a listing's ADD VERSION after its keywords: `[ [ IF NOT EXISTS ] <version> ] FROM <location>` and properties.
    fn parse_listing_version(&mut self, mut action: String) -> Result<(String, ActionParts)> {
        if self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]) {
            action.push_str(" IF NOT EXISTS");
        }
        let mut operands: Vec<Expression> = Vec::new();
        if !self.check(TokenType::From) {
            operands.push(self.parse_name_operand()?);
        }
        self.expect_token(TokenType::From)?;
        operands.push(self.parse_location_value()?);
        Ok((action, (operands, self.parse_resource_properties()?)))
    }

    /// Parse a model's ADD VERSION, MODIFY VERSION or VERSION after its keywords.
    fn parse_model_version(&mut self, mut action: String) -> Result<(String, ActionParts)> {
        let mut operands = vec![self.parse_name_operand()?];
        match action.as_str() {
            "MODIFY VERSION" => {
                self.expect_alter_words(&[&["SET"]])?;
                action.push_str(" SET");
                return Ok((action, (operands, self.parse_resource_set()?)));
            }
            "VERSION" => {
                let alias = self.expect_alter_words(&[&["SET", "ALIAS"], &["UNSET", "ALIAS"]])?;
                if let "SET ALIAS" = alias.as_str() {
                    self.expect_token(TokenType::Eq)?;
                    operands.push(self.parse_name_operand()?);
                }
                action.push(' ');
                action.push_str(&alias);
            }
            _ => {
                self.expect_token(TokenType::From)?;
                if self.match_identifier("MODEL") {
                    action.push_str(" FROM MODEL");
                    operands.push(self.parse_name_operand()?);
                    if self.match_identifier("VERSION") {
                        operands.push(self.parse_name_operand()?);
                    }
                } else {
                    operands.push(self.parse_location_value()?);
                }
            }
        }
        Ok((action, (operands, vec![])))
    }

    /// Parse a service's FROM: an inline specification, or an optional stage path and the source's properties.
    fn parse_service_from(&mut self) -> Result<ActionParts> {
        let staged = self.is_stage_path();
        if !staged && !self.check(TokenType::Identifier) && !self.check(TokenType::Var) {
            return Ok((vec![self.parse_object_value()?], vec![]));
        }
        let operands = if staged {
            vec![self.parse_stage_path()?]
        } else {
            Vec::new()
        };
        let properties = self.parse_service_source()?;
        if properties.is_empty() && operands.is_empty() {
            return Err(self.parse_error("Expected a specification"));
        }
        Ok((operands, properties))
    }

    /// Parse an event routing table's rule action after its keywords.
    fn parse_routing_rule(&mut self, mut action: String) -> Result<(String, ActionParts)> {
        if action.ends_with("RULES") {
            let mut properties: Vec<ObjectProperty> = Vec::new();
            loop {
                let rule = self.parse_simple_name()?;
                self.expect_token(TokenType::Eq)?;
                properties.push(property("RULE", Some(rule), Some(self.parse_value_list()?)));
                if !self.match_token(TokenType::Comma) {
                    return Ok((action, (vec![], properties)));
                }
            }
        }
        let mut operands = vec![self.parse_name_operand()?];
        if action.starts_with("MODIFY") {
            action.push(' ');
            action.push_str(&self.expect_alter_words(&[&["RENAME", "TO"]])?);
            operands.push(self.parse_name_operand()?);
            return Ok((action, (operands, vec![])));
        }
        Ok((action, (operands, self.parse_resource_set()?)))
    }

    /// Parse the rest of the remaining actions: STOP ALL, EXECUTE AS, segment columns, partitions and volumes.
    fn parse_resource_clause(&mut self, mut action: String) -> Result<(String, ActionParts)> {
        let mut operands: Vec<Expression> = Vec::new();
        let mut properties: Vec<ObjectProperty> = Vec::new();
        match action.as_str() {
            "STOP ALL" if self.match_text_seq(&["OF", "TYPE"]) => {
                action.push_str(" OF TYPE");
                operands = self.parse_object_name_list()?;
            }
            "EXECUTE AS" => {
                let caller =
                    self.expect_alter_words(&[&["OWNER"], &["CALLER"], &["RESTRICTED", "CALLER"]])?;
                action.push(' ');
                action.push_str(&caller);
            }
            "ADD" | "DROP" => {
                self.expect_alter_words(&[&["SEGMENT_COLUMN"]])?;
                action.push_str(" SEGMENT_COLUMN");
                self.expect_token(TokenType::Eq)?;
                operands.push(self.parse_object_value()?);
            }
            "ADD PARTITION" => {
                self.expect_token(TokenType::LParen)?;
                loop {
                    let column = self.parse_simple_name()?;
                    self.expect_token(TokenType::Eq)?;
                    let value = self.parse_object_value()?;
                    properties.push(property("PARTITION", Some(column), Some(value)));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
                self.expect_alter_words(&[&["LOCATION"]])?;
                action.push_str(" LOCATION");
                operands.push(self.parse_location_value()?);
            }
            "RESTORE VOLUME" => {
                operands.push(self.parse_name_operand()?);
                self.expect_alter_words(&[&["INSTANCES"]])?;
                let mut instances = vec![self.parse_object_value()?];
                while self.match_token(TokenType::Comma) {
                    instances.push(self.parse_object_value()?);
                }
                operands.push(Expression::Tuple(Box::new(Tuple {
                    expressions: instances,
                })));
                self.expect_alter_words(&[&["FROM", "SNAPSHOT"]])?;
                operands.push(self.parse_name_operand()?);
            }
            _ => {}
        }
        Ok((action, (operands, properties)))
    }

    /// Parse an object name operand.
    fn parse_name_operand(&mut self) -> Result<Expression> {
        Ok(Expression::Table(Box::new(self.parse_object_name()?)))
    }

    /// Parse a location: a stage path or a string.
    pub(super) fn parse_location_value(&mut self) -> Result<Expression> {
        if self.is_stage_path() {
            return self.parse_stage_path();
        }
        self.parse_string_value()
    }

    /// Parse a string or dollar-quoted string.
    fn parse_text_value(&mut self) -> Result<Expression> {
        if !self.check(TokenType::String) && !self.check(TokenType::DollarString) {
            return Err(self.parse_error("Expected a string"));
        }
        self.parse_primary()
    }

    /// Whether the current token starts a stage path.
    fn is_stage_path(&self) -> bool {
        !self.is_at_end() && self.peek_text().starts_with('@')
    }

    /// Parse a stage path, which like Snowflake's ends at the first whitespace: `@"s 1"` is not one path.
    fn parse_stage_path(&mut self) -> Result<Expression> {
        let mut path = self.peek_text().to_owned();
        let mut end = self.peek().span.end;
        self.skip();
        while !self.is_at_end()
            && self.peek().span.start == end
            && !matches!(
                self.peek().token_type,
                TokenType::Semicolon | TokenType::Comma | TokenType::LParen | TokenType::RParen
            )
        {
            if self.check(TokenType::String) || self.peek_text().chars().any(char::is_whitespace) {
                return Err(self.parse_error("A stage path ends at whitespace"));
            }
            path.push_str(self.peek_text());
            end = self.peek().span.end;
            self.skip();
        }
        Ok(Expression::Literal(Box::new(Literal::String(path))))
    }

    /// Parse a service's source after FROM: SPECIFICATION or SPECIFICATION_TEMPLATE with a value, `<name> = <value>` and USING ( ... ).
    fn parse_service_source(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.check(TokenType::Using) || self.check_identifier("USING") {
                self.skip();
                properties.push(property("USING", None, Some(self.parse_value_list()?)));
                continue;
            }
            let name = self.parse_simple_name()?;
            let upper = name.name.to_ascii_uppercase();
            let bare = matches!(upper.as_str(), "SPECIFICATION" | "SPECIFICATION_TEMPLATE")
                && !self.check(TokenType::Eq);
            if !bare {
                self.expect_token(TokenType::Eq)?;
            }
            properties.push(property(&upper, None, Some(self.parse_object_value()?)));
        }
        Ok(properties)
    }

    /// Whether the current token is a literal: a string, a number, TRUE or FALSE.
    fn is_literal_value(&self) -> bool {
        self.check(TokenType::String)
            || self.check(TokenType::DollarString)
            || self.check(TokenType::Number)
            || self.check(TokenType::True)
            || self.check(TokenType::False)
    }

    /// Parse the properties of a SET: at least one TAG pair, CONTACT pair or `<name> = <value>`.
    fn parse_resource_set(&mut self) -> Result<Vec<ObjectProperty>> {
        let properties = self.parse_resource_properties()?;
        if properties.is_empty() {
            return Err(self.parse_error("Expected a property to set"));
        }
        let pairs = properties
            .iter()
            .filter(|item| item.name.is_some())
            .map(|item| item.kind.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if !pairs.is_empty()
            && (pairs.len() > 1 || properties.iter().any(|item| item.name.is_none()))
        {
            return Err(self.parse_error("A SET of TAG or CONTACT pairs sets nothing else"));
        }
        Ok(properties)
    }

    /// Parse zero or more TAG pairs, CONTACT pairs and `<name> = <value>` properties, optionally comma-separated; a policy's `AS` body only follows another property.
    fn parse_resource_properties(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let pairs = ["TAG", "CONTACT"]
                .into_iter()
                .find(|word| self.check_identifier(word) && !self.check_next(TokenType::Eq));
            if let Some(word) = pairs {
                self.skip();
                loop {
                    let name = self.parse_simple_qualified_name()?;
                    self.expect_token(TokenType::Eq)?;
                    let value = match word {
                        "CONTACT" => self.parse_name_operand()?,
                        _ => self.parse_object_value()?,
                    };
                    properties.push(property(word, Some(name), Some(value)));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            } else if !properties.is_empty() && self.match_token(TokenType::As) {
                properties.push(property("AS", None, Some(self.parse_text_value()?)));
            } else {
                let name = self.parse_simple_name()?;
                let value = if self.match_token(TokenType::Arrow) {
                    self.parse_expression()?
                } else {
                    self.expect_token(TokenType::Eq)?;
                    self.parse_resource_value()?
                };
                properties.push(property(&name.name.to_ascii_uppercase(), None, Some(value)));
                self.match_token(TokenType::Comma);
            }
        }
        Ok(properties)
    }

    /// Parse the names an UNSET removes: TAG names, CONTACT purposes or properties, comma-separated.
    fn parse_resource_unset(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let pairs = ["TAG", "CONTACT"].into_iter().find(|word| {
                self.check_identifier(word)
                    && !self.check_next(TokenType::Comma)
                    && !self.check_next(TokenType::Semicolon)
                    && self.current + 1 < self.tokens.len()
            });
            if let Some(word) = pairs {
                self.skip();
                loop {
                    let name = self.parse_simple_qualified_name()?;
                    properties.push(property(word, Some(name), None));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            } else {
                let name = self.parse_simple_name()?;
                properties.push(property(&name.name.to_ascii_uppercase(), None, None));
            }
            if !self.match_token(TokenType::Comma) {
                return Ok(properties);
            }
        }
    }

    /// Parse a property value: a parenthesized list, a stage path or a single value.
    fn parse_resource_value(&mut self) -> Result<Expression> {
        if self.check(TokenType::LParen) {
            return self.parse_value_list();
        }
        if self.is_stage_path() {
            return self.parse_stage_path();
        }
        self.parse_object_value()
    }

    /// Parse `( [ <item> [ , ] ... ] )`, each item a value or a `<key> = <value>` or `<key> => <value>` pair.
    fn parse_value_list(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut items: Vec<Expression> = Vec::new();
        while !self.match_token(TokenType::RParen) {
            let item = self.parse_resource_value()?;
            items.push(
                if self.match_token(TokenType::Eq) || self.match_token(TokenType::FArrow) {
                    Expression::Tuple(Box::new(Tuple {
                        expressions: vec![item, self.parse_resource_value()?],
                    }))
                } else {
                    item
                },
            );
            self.match_token(TokenType::Comma);
            if self.is_at_end() {
                return Err(self.parse_error("Expected )"));
            }
        }
        Ok(Expression::Tuple(Box::new(Tuple { expressions: items })))
    }
}
