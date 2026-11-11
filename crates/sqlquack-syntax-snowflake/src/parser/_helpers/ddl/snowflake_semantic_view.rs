//! Parser ddl: Snowflake CREATE SEMANTIC VIEW and DEFINE SEMANTIC VIEW, built against their grammar.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::constants::SNOWFLAKE_VERIFIED_QUERY_FIELDS;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// Parses one item of a semantic view list.
type SemanticItem = fn(&mut Parser) -> Result<Expression>;

/// `<key> = <value>`: one named part of a semantic view entry.
pub(super) fn part(key: &str, value: Expression) -> Expression {
    Expression::PropertyEQ(Box::new(BinaryOp::new(
        Expression::Identifier(Identifier::new(key)),
        value,
    )))
}

/// The parts of one entry, in the order they are written.
pub(super) fn tuple(expressions: Vec<Expression>) -> Expression {
    Expression::Tuple(Box::new(Tuple { expressions }))
}

impl Parser {
    /// A Snowflake CREATE [ OR REPLACE | OR ALTER ] SEMANTIC VIEW or DEFINE SEMANTIC VIEW; None for any other CREATE.
    pub(crate) fn parse_snowflake_create_semantic_view(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.check_text_seq(&["SEMANTIC", "VIEW"]) {
            return Ok(None);
        }
        self.config.in_semantic_view = true;
        let view = self.parse_semantic_view_definition(or_replace, or_alter);
        self.config.in_semantic_view = false;
        view.map(Some)
    }

    /// Skips EXCLUDING after a window's PARTITION BY inside a semantic view.
    pub(crate) fn skip_semantic_view_excluding(&mut self) {
        if self.config.in_semantic_view {
            self.match_text_seq(&["EXCLUDING"]);
        }
    }

    fn parse_semantic_view_definition(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Expression> {
        self.match_text_seq(&["SEMANTIC", "VIEW"]);
        let if_not_exists = self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_object_name()?;
        if !self.match_text_seq(&["TABLES"]) {
            return Err(self.parse_error("Expected TABLES after the semantic view name"));
        }
        let mut properties = vec![property(
            "TABLES",
            None,
            Some(self.semantic_list(Self::parse_semantic_table)?),
        )];
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            properties.push(self.parse_semantic_view_clause()?);
        }
        Ok(Expression::CreateObject(Box::new(CreateObject {
            kind: "SEMANTIC VIEW".to_owned(),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: String::new(),
            operands: Vec::new(),
            properties,
        })))
    }

    fn parse_semantic_view_clause(&mut self) -> Result<ObjectProperty> {
        let lists: [(&str, SemanticItem); 5] = [
            ("RELATIONSHIPS", Self::parse_semantic_relationship),
            ("FACTS", Self::parse_semantic_item),
            ("DIMENSIONS", Self::parse_semantic_item),
            ("METRICS", Self::parse_semantic_item),
            ("AI_VERIFIED_QUERIES", Self::parse_semantic_verified_query),
        ];
        if let Some((kind, item)) = lists.iter().find(|(kind, _)| self.match_text_seq(&[kind])) {
            return Ok(property(kind, None, Some(self.semantic_list(*item)?)));
        }
        for kind in ["AI_SQL_GENERATION", "AI_QUESTION_CATEGORIZATION"] {
            if self.match_text_seq(&[kind]) {
                return Ok(property(kind, None, Some(self.parse_semantic_string()?)));
            }
        }
        if self.match_text_seq(&["COPY", "GRANTS"]) {
            return Ok(property("COPY GRANTS", None, None));
        }
        if self.match_text_seq(&["COMMENT"]) {
            self.expect_token(TokenType::Eq)?;
            return Ok(property(
                "COMMENT",
                None,
                Some(self.parse_semantic_string()?),
            ));
        }
        if self.match_text_seq(&["WITH", "TAG"]) || self.match_text_seq(&["TAG"]) {
            return Ok(property("TAG", None, Some(self.parse_semantic_tags()?)));
        }
        Err(self.parse_error("Unexpected clause in CREATE SEMANTIC VIEW"))
    }

    /// `( <item> [ , ... ] )`, which needs at least one item.
    fn semantic_list(&mut self, item: SemanticItem) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut items: Vec<Expression> = Vec::new();
        loop {
            items.push(item(self)?);
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(tuple(items))
    }

    fn parse_semantic_string(&mut self) -> Result<Expression> {
        match self.check(TokenType::String) {
            true => self.parse_primary(),
            false => Err(self.parse_error("Expected a string")),
        }
    }

    /// One unqualified name: an identifier or a keyword Snowflake does not reserve.
    fn parse_semantic_name(&mut self) -> Result<Expression> {
        Ok(Expression::Identifier(self.parse_external_name()?))
    }

    /// `<alias> AS`, when the name is followed by AS.
    fn parse_semantic_alias(&mut self) -> Result<Option<Expression>> {
        if !self
            .peek_nth(1)
            .is_some_and(|token| token.token_type == TokenType::As)
        {
            return Ok(None);
        }
        let alias = self.parse_semantic_name()?;
        self.skip();
        Ok(Some(part("ALIAS", alias)))
    }

    /// `[<alias> AS] <table> [ PRIMARY KEY ( ... ) | UNIQUE ( ... ) | CONSTRAINT ... | <property> ... ]`.
    fn parse_semantic_table(&mut self) -> Result<Expression> {
        let mut parts: Vec<Expression> = self.parse_semantic_alias()?.into_iter().collect();
        parts.push(Expression::Table(Box::new(self.parse_object_name()?)));
        while !self.check(TokenType::Comma) && !self.check(TokenType::RParen) {
            if self.match_text_seq(&["PRIMARY", "KEY"]) {
                parts.push(part(
                    "PRIMARY KEY",
                    self.semantic_list(Self::parse_semantic_name)?,
                ));
            } else if self.match_text_seq(&["UNIQUE"]) {
                parts.push(part(
                    "UNIQUE",
                    self.semantic_list(Self::parse_semantic_name)?,
                ));
            } else if self.match_text_seq(&["CONSTRAINT"]) {
                let constraint = self.parse_semantic_name()?;
                if !self.match_text_seq(&["DISTINCT", "RANGE"]) {
                    return Err(self.parse_error("Expected DISTINCT RANGE"));
                }
                let range = self.parse_semantic_range()?;
                parts.push(part("CONSTRAINT", tuple(vec![constraint, range])));
            } else {
                parts.push(self.parse_semantic_property(false)?);
            }
        }
        Ok(tuple(parts))
    }

    /// `BETWEEN <start> AND <end> EXCLUSIVE`.
    fn parse_semantic_range(&mut self) -> Result<Expression> {
        if !self.match_text_seq(&["BETWEEN"]) {
            return Err(self.parse_error("Expected BETWEEN"));
        }
        let start = self.parse_semantic_name()?;
        if !self.match_text_seq(&["AND"]) {
            return Err(self.parse_error("Expected AND"));
        }
        let end = self.parse_semantic_name()?;
        match self.match_text_seq(&["EXCLUSIVE"]) {
            true => Ok(part("DISTINCT RANGE", tuple(vec![start, end]))),
            false => Err(self.parse_error("Expected EXCLUSIVE")),
        }
    }

    /// COMMENT, SYNONYMS or TAG, and with `item` also a Cortex Search service.
    fn parse_semantic_property(&mut self, item: bool) -> Result<Expression> {
        if self.match_text_seq(&["COMMENT"]) {
            self.expect_token(TokenType::Eq)?;
            Ok(part("COMMENT", self.parse_semantic_string()?))
        } else if self.match_text_seq(&["WITH", "SYNONYMS"]) {
            self.match_token(TokenType::Eq);
            Ok(part(
                "SYNONYMS",
                self.semantic_list(Self::parse_semantic_string)?,
            ))
        } else if item && self.match_text_seq(&["WITH", "CORTEX", "SEARCH", "SERVICE"]) {
            let mut service = vec![Expression::Table(Box::new(self.parse_object_name()?))];
            if self.match_token(TokenType::Using) {
                service.push(self.parse_semantic_name()?);
            }
            Ok(part("CORTEX SEARCH SERVICE", tuple(service)))
        } else if self.match_text_seq(&["WITH", "TAG"]) || self.match_text_seq(&["TAG"]) {
            Ok(part("TAG", self.parse_semantic_tags()?))
        } else {
            Err(self.parse_error("Unexpected property in CREATE SEMANTIC VIEW"))
        }
    }

    /// `( <tag> = <value> [ , ... ] )`, the value a string or a name.
    fn parse_semantic_tags(&mut self) -> Result<Expression> {
        self.semantic_list(|parser| {
            let tag = Expression::Table(Box::new(parser.parse_object_name()?));
            parser.expect_token(TokenType::Eq)?;
            let value = match parser.check(TokenType::String) {
                true => parser.parse_semantic_string()?,
                false => parser.parse_semantic_name()?,
            };
            Ok(Expression::PropertyEQ(Box::new(BinaryOp::new(tag, value))))
        })
    }

    /// `[<alias> AS] <table> ( <columns> ) REFERENCES <table> [ ( <columns> | <range> ) ]`.
    fn parse_semantic_relationship(&mut self) -> Result<Expression> {
        let mut parts: Vec<Expression> = self.parse_semantic_alias()?.into_iter().collect();
        parts.push(Expression::Table(Box::new(self.parse_object_name()?)));
        parts.push(self.semantic_list(Self::parse_semantic_name)?);
        if !self.match_text_seq(&["REFERENCES"]) {
            return Err(self.parse_error("Expected REFERENCES"));
        }
        let mut references = vec![Expression::Table(Box::new(self.parse_object_name()?))];
        if self.match_token(TokenType::LParen) {
            if self.check_text_seq(&["BETWEEN"]) {
                references.push(self.parse_semantic_range()?);
            } else {
                loop {
                    references.push(match self.match_text_seq(&["ASOF"]) {
                        true => part("ASOF", self.parse_semantic_name()?),
                        false => self.parse_semantic_name()?,
                    });
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            self.expect_token(TokenType::RParen)?;
        }
        parts.push(part("REFERENCES", tuple(references)));
        Ok(tuple(parts))
    }

    /// A fact, dimension or metric: `[PUBLIC | PRIVATE] <table>.<name> [...] AS <expr> [<property> ...]`.
    fn parse_semantic_item(&mut self) -> Result<Expression> {
        let mut parts: Vec<Expression> = Vec::new();
        if !self.check_next(TokenType::Dot) {
            for visibility in ["PUBLIC", "PRIVATE"] {
                if self.match_text_seq(&[visibility]) {
                    parts.push(Expression::Identifier(Identifier::new(visibility)));
                    break;
                }
            }
        }
        let table = self.parse_semantic_name()?;
        self.expect_token(TokenType::Dot)?;
        parts.push(tuple(vec![table, self.parse_semantic_name()?]));
        loop {
            if self.match_text_seq(&["LABELS"]) {
                self.expect_token(TokenType::Eq)?;
                parts.push(part(
                    "LABELS",
                    self.semantic_list(Self::parse_semantic_name)?,
                ));
            } else if self.match_token(TokenType::Using) {
                parts.push(part(
                    "USING",
                    self.semantic_list(Self::parse_semantic_name)?,
                ));
            } else if self.match_text_seq(&["NON", "ADDITIVE", "BY"]) {
                parts.push(part(
                    "NON ADDITIVE BY",
                    self.semantic_list(Self::parse_semantic_ordered_name)?,
                ));
            } else {
                break;
            }
        }
        self.expect_token(TokenType::As)?;
        parts.push(part("AS", self.parse_expression()?));
        while !self.check(TokenType::Comma) && !self.check(TokenType::RParen) {
            parts.push(self.parse_semantic_property(true)?);
        }
        Ok(tuple(parts))
    }

    /// `<name> [ ASC | DESC ] [ NULLS { FIRST | LAST } ]`.
    fn parse_semantic_ordered_name(&mut self) -> Result<Expression> {
        let mut parts = vec![self.parse_semantic_name()?];
        for direction in ["ASC", "DESC"] {
            if self.match_text_seq(&[direction]) {
                parts.push(Expression::Identifier(Identifier::new(direction)));
                break;
            }
        }
        if self.match_text_seq(&["NULLS"]) {
            let order = self.expect_alter_words(&[&["FIRST"], &["LAST"]])?;
            parts.push(part(
                "NULLS",
                Expression::Identifier(Identifier::new(order)),
            ));
        }
        Ok(tuple(parts))
    }

    /// `<name> AS ( <key> <value> ... )`, which needs QUESTION and SQL.
    fn parse_semantic_verified_query(&mut self) -> Result<Expression> {
        let mut parts = vec![self.parse_semantic_name()?];
        self.expect_token(TokenType::As)?;
        self.expect_token(TokenType::LParen)?;
        let mut keys: Vec<String> = Vec::new();
        while !self.check(TokenType::RParen) {
            if !matches!(
                self.peek().token_type,
                TokenType::Identifier | TokenType::Var
            ) {
                return Err(self.parse_error("Expected a verified query field"));
            }
            let key = self.advance_text()?.to_ascii_uppercase();
            let literal = matches!(
                self.peek().token_type,
                TokenType::String | TokenType::Number | TokenType::True | TokenType::False
            );
            let value = match literal && !self.is_at_end() {
                true => self.parse_primary()?,
                false => self.parse_semantic_name()?,
            };
            parts.push(part(&key, value));
            keys.push(key);
        }
        if !SNOWFLAKE_VERIFIED_QUERY_FIELDS
            .iter()
            .all(|field| keys.contains(&field.to_string()))
        {
            return Err(self.parse_error("Expected QUESTION and SQL in a verified query"));
        }
        self.expect_token(TokenType::RParen)?;
        Ok(tuple(parts))
    }
}
