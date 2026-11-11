//! Primaries: literals, variables, names, calls, `CASE`, casts, subqueries and windows.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::functions::models::{Function, JSONArray, WithinGroup};
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, Star};
use sqlquack_syntax::expressions::operators::models::{Parameter, ParameterStyle};
use sqlquack_syntax::expressions::queries::models::{
    Ordered, Over, WindowFrame, WindowFrameBound, WindowFrameKind,
};
use sqlquack_syntax::expressions::statements::models::NextValueFor;
use sqlquack_syntax::expressions::typed_functions::models::{
    ConvertFunc, IfFunc, JsonNullHandling, JsonObjectFunc, TrimFunc, TrimPosition,
};
use sqlquack_syntax::tokens::models::{SourceSpan, Span};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_any_name, is_name};
use crate::parse::_helpers::tree::nodes::{
    aggregate, case, cast, column_from, function, literal, null, over, paren, subquery, var,
    window_function,
};
use crate::parse::constants::{
    AGGREGATES, BUILT_IN_FUNCTIONS, DATE_PARTS, DATEPART_FUNCTIONS, IIF_FUNCTION, MAX_NAME_PARTS,
    MIN_COALESCE_ARGUMENTS, NILADIC_FUNCTIONS, NULLIF_ARGUMENTS, RESERVED_CALLS,
    TRY_CONVERT_FUNCTION,
};
use crate::scan::models::{NumberKind, Token, TokenKind};

/// A `@name` or `@@name` variable as a parameter node.
pub(crate) fn variable(token: &Token) -> Expression {
    let (style, name) = match token.value.strip_prefix("@@") {
        Some(name) => (ParameterStyle::DoubleAt, name),
        None => (
            ParameterStyle::At,
            token.value.strip_prefix('@').unwrap_or(&token.value),
        ),
    };
    Expression::Parameter(Box::new(Parameter {
        name: Some(name.to_owned()),
        index: None,
        style,
        quoted: false,
        string_quoted: false,
        expression: None,
        span: SourceSpan(Some(token.span)),
    }))
}

impl<'sql> Parser<'sql> {
    /// Whether a query starts at the cursor: `SELECT`, `WITH`, or `(` before one.
    pub(crate) fn starts_query(&self) -> bool {
        let mut offset: usize = 0;
        while self.char_at(offset, '(') {
            offset += 1;
        }
        self.kw_at(offset, "SELECT") || (offset == 0 && self.kw("WITH"))
    }

    pub(crate) fn primary(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        let this: Expression = self.primary_value()?;
        if self.is_char('.') && self.is_at(1, is_any_name) {
            return Err(self.gap("method_call", start));
        }
        Ok(this)
    }

    pub(crate) fn primary_value(&mut self) -> Parsing<Expression> {
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        match token.kind {
            TokenKind::String { national } => {
                self.at += 1;
                Ok(literal(match national {
                    true => Literal::NationalString(token.value),
                    false => Literal::String(token.value),
                }))
            }
            TokenKind::Number(NumberKind::Binary) => {
                self.at += 1;
                Ok(literal(Literal::HexString(token.value[2..].to_owned())))
            }
            TokenKind::Number(_) => {
                self.at += 1;
                Ok(literal(Literal::Number(token.value)))
            }
            TokenKind::Variable => {
                self.at += 1;
                if self.is_char('.') {
                    return Err(self.gap("variable_method", token.start));
                }
                Ok(variable(&token))
            }
            TokenKind::Char('(') => self.parenthesized_primary(),
            TokenKind::Char('{') => Err(self.gap("odbc_escape", token.start)),
            TokenKind::Word | TokenKind::Quoted(_) => self.word_primary(&token),
            _ => Err(self.error()),
        }
    }

    /// `( query )`, `( condition )` or `( value )`, the query reading tried first and rewound.
    fn parenthesized_primary(&mut self) -> Parsing<Expression> {
        let open: usize = self.at;
        if self.char_at(0, '(') && !self.not_subquery.contains(&open) && {
            self.at += 1;
            let query: bool = self.starts_query();
            self.at -= 1;
            query
        } {
            let read: Parsing<Expression> = self.attempt(|parser| {
                parser.at += 1;
                let query: Expression = parser.query_expression()?;
                parser.expect_char(')')?;
                Ok(subquery(query, None, false))
            });
            match read {
                Ok(node) => return Ok(node),
                Err(error) if self.gapped() || !error.syntax => return Err(error),
                Err(_) => {
                    self.not_subquery.insert(open);
                }
            }
        }
        self.at += 1;
        let this: Expression = self.any_expression()?;
        self.expect_char(')')?;
        Ok(paren(this))
    }

    fn word_primary(&mut self, token: &Token) -> Parsing<Expression> {
        let upper: String = token.value.to_ascii_uppercase();
        let quoted: bool = token.kind != TokenKind::Word;
        let call: bool = self.char_at(1, '(');
        if !quoted {
            match upper.as_str() {
                "NULL" => {
                    self.at += 1;
                    return Ok(null());
                }
                "CASE" => return self.case_expression(),
                "CAST" | "TRY_CAST" if call => return self.cast_expression(&upper),
                "CONVERT" | "TRY_CONVERT" if call => return self.convert_expression(&upper),
                "PARSE" | "TRY_PARSE" if call => {
                    return Err(self.gap("parse_function", token.start));
                }
                "NEXT" if self.kw_at(1, "VALUE") && self.kw_at(2, "FOR") => {
                    return self.next_value_for();
                }
                "$ACTION" | "$IDENTITY" | "$ROWGUID" => {
                    self.at += 1;
                    return Ok(column_from(vec![identifier(token)]));
                }
                _ => {}
            }
            if upper.starts_with('$') {
                return Err(self.gap("pseudo_column", token.start));
            }
            match upper.as_str() {
                "CONTAINS" | "FREETEXT" if call => {
                    return Err(self.gap("full_text_predicate", token.start));
                }
                "MATCH" if call => return Err(self.gap("graph_match", token.start)),
                "IDENTITY" if call => return Err(self.gap("identity_function", token.start)),
                _ => {}
            }
            if NILADIC_FUNCTIONS.contains(&upper.as_str()) && !call {
                self.at += 1;
                let mut node: Function = function(vec![identifier(token)], Vec::new());
                node.no_parens = true;
                return Ok(Expression::Function(Box::new(node)));
            }
            if !is_name(token) && !(call && RESERVED_CALLS.contains(&upper.as_str())) {
                return Err(self.error());
            }
        }
        let start: usize = token.start;
        let mut parts: Vec<Identifier> = vec![identifier(&self.advance()?)];
        while self.is_char('.') {
            if self.op_at(1, "::") {
                return Err(self.gap("static_method", start));
            }
            self.at += 1;
            if self.is_char('*') {
                return Err(self.error());
            }
            if self.is_char('.') {
                parts.push(Identifier::empty());
                continue;
            }
            if self
                .peek()
                .is_some_and(|token| token.kind == TokenKind::Word && token.value.starts_with('$'))
            {
                return Err(self.gap("pseudo_column", start));
            }
            parts.push(identifier(&self.expect_token(is_name)?));
        }
        if self.is_op("::") {
            return Err(self.gap("static_method", start));
        }
        if !self.is_char('(') {
            if parts.len() > MAX_NAME_PARTS {
                return Err(self.error());
            }
            return Ok(column_from(parts));
        }
        self.call(parts)
    }

    /// A call of `name`, the cursor at its `(`.
    fn call(&mut self, name: Vec<Identifier>) -> Parsing<Expression> {
        let upper: String = name
            .last()
            .map(|part| part.name.to_ascii_uppercase())
            .unwrap_or_default();
        let bare: bool = name.len() == 1 && !name.iter().any(|part| part.quoted);
        if name.len() == 1 && !bare {
            return Err(self.error());
        }
        match upper.as_str() {
            "TRIM" if bare => {
                let this: Expression = self.trim_call(name)?;
                return self.window_tail(this);
            }
            "JSON_OBJECT" if bare => return self.json_object(),
            "JSON_ARRAY" if bare => return self.json_array(),
            _ => {}
        }
        if bare && let Some(marker) = self.typed_marker(&upper)? {
            return Ok(marker);
        }
        if bare
            && upper != IIF_FUNCTION
            && BUILT_IN_FUNCTIONS.binary_search(&upper.as_str()).is_err()
            && !RESERVED_CALLS.contains(&upper.as_str())
        {
            return Err(self.error_with("not a recognized built-in function name"));
        }
        if bare && upper == IIF_FUNCTION {
            self.expect_char('(')?;
            let condition: Expression = self.condition()?;
            self.expect_char(',')?;
            let true_value: Expression = self.scalar()?;
            self.expect_char(',')?;
            let false_value: Expression = self.scalar()?;
            self.expect_char(')')?;
            return Ok(Expression::IfFunc(Box::new(IfFunc {
                condition,
                true_value,
                false_value: Some(false_value),
                original_name: Some("IIF".to_owned()),
                inferred_type: None,
            })));
        }
        self.expect_char('(')?;
        let aggregate_call: bool = bare && AGGREGATES.contains(&upper.as_str());
        let mut distinct: bool = false;
        let mut args: Vec<Expression> = Vec::new();
        let star_call: bool =
            aggregate_call || (bare && matches!(upper.as_str(), "CHECKSUM" | "BINARY_CHECKSUM"));
        if star_call && self.is_char('*') && self.char_at(1, ')') {
            self.at += 1;
            args.push(Expression::Star(Box::new(Star {
                table: None,
                ilike: None,
                except: None,
                replace: None,
                rename: None,
                trailing_comments: Vec::new(),
                span: SourceSpan::default(),
                qualified: None,
            })));
        } else if !self.is_char(')') {
            if aggregate_call {
                distinct = self.eat_kw("DISTINCT");
                if !distinct {
                    self.eat_kw("ALL");
                }
            }
            if bare && DATEPART_FUNCTIONS.contains(&upper.as_str()) {
                let part: Token = self.expect_token(|token| {
                    matches!(token.kind, TokenKind::Word | TokenKind::Quoted(_))
                })?;
                if !DATE_PARTS.contains(&part.value.to_ascii_lowercase().as_str()) {
                    self.at -= 1;
                    return Err(self.error());
                }
                args.push(var(&part.value));
                while self.eat_char(',') {
                    args.push(self.scalar()?);
                }
            } else {
                args = self.comma_list(Self::scalar)?;
            }
        }
        let arity_ok: bool = match (bare, upper.as_str()) {
            (true, "COALESCE") => args.len() >= MIN_COALESCE_ARGUMENTS,
            (true, "NULLIF") => args.len() == NULLIF_ARGUMENTS,
            _ => true,
        };
        if !arity_ok {
            return Err(self.error());
        }
        self.expect_char(')')?;
        let name_start: Option<usize> = name
            .first()
            .and_then(|part| part.span)
            .map(|span| span.start);
        let mut node: Function = function(name, args);
        node.span = SourceSpan(name_start.map(|start| Span {
            start,
            end: self.tokens[self.at - 1].span.end,
            line: 0,
            column: 0,
        }));
        let ignore_nulls: Option<bool> = match (self.word_at(0).as_deref(), self.kw_at(1, "NULLS"))
        {
            (Some("IGNORE"), true) => Some(true),
            (Some("RESPECT"), true) => Some(false),
            _ => None,
        };
        if ignore_nulls.is_some() {
            self.at += 2;
        }
        let mut this: Expression = match aggregate_call || ignore_nulls.is_some() {
            true => aggregate(node, distinct, None, ignore_nulls),
            false => Expression::Function(Box::new(node)),
        };
        if self.kw_in(0, &["WITHIN", "GROUP"]) && self.char_at(1, '(')
            || self.kw("WITHIN") && self.kw_at(1, "GROUP") && self.kw_at(3, "GRAPH")
        {
            return Err(self.gap("within_group", self.offset()));
        }
        if self.kw("WITHIN") && self.kw_at(1, "GROUP") {
            self.at += 2;
            self.expect_char('(')?;
            let order_by: Vec<Ordered> = self.order_by_clause()?;
            self.expect_char(')')?;
            this = Expression::WithinGroup(Box::new(WithinGroup { this, order_by }));
            if self.is_char('(') {
                return Err(self.gap("within_group", self.offset()));
            }
        }
        self.window_tail(this)
    }

    /// `this`, or `this OVER window` when an `OVER` clause follows the call.
    fn window_tail(&mut self, this: Expression) -> Parsing<Expression> {
        if !self.kw("OVER") {
            return Ok(this);
        }
        self.at += 1;
        let window: Over = match self.is(is_name) {
            true => over(Some(self.name()?), Vec::new(), Vec::new(), None),
            false => self.window_spec()?,
        };
        Ok(window_function(this, window))
    }

    /// `TRIM ( [[LEADING|TRAILING|BOTH] characters FROM] value )`, the cursor at the `(`.
    fn trim_call(&mut self, name: Vec<Identifier>) -> Parsing<Expression> {
        self.expect_char('(')?;
        let position: Option<TrimPosition> = match self.word_at(0).as_deref() {
            Some("LEADING") => Some(TrimPosition::Leading),
            Some("TRAILING") => Some(TrimPosition::Trailing),
            Some("BOTH") => Some(TrimPosition::Both),
            _ => None,
        };
        if position.is_some() {
            self.at += 1;
        } else if self.eat_char(')') {
            return Ok(Expression::Function(Box::new(function(name, Vec::new()))));
        }
        let first: Expression = self.scalar()?;
        let (this, characters) = match position.is_some() || self.kw("FROM") {
            true => {
                self.expect_kw("FROM")?;
                (self.scalar()?, Some(first))
            }
            false if self.is_char(',') => {
                let mut args: Vec<Expression> = vec![first];
                while self.eat_char(',') {
                    args.push(self.scalar()?);
                }
                self.expect_char(')')?;
                return Ok(Expression::Function(Box::new(function(name, args))));
            }
            false => (first, None),
        };
        self.expect_char(')')?;
        Ok(Expression::Trim(Box::new(TrimFunc {
            this,
            sql_standard_syntax: characters.is_some(),
            characters,
            position: position.unwrap_or(TrimPosition::Both),
            position_explicit: position.is_some(),
            inferred_type: None,
        })))
    }

    /// `NULL ON NULL` or `ABSENT ON NULL` at the cursor, consumed.
    fn json_null_clause(&mut self) -> Option<JsonNullHandling> {
        let handling: JsonNullHandling = match self.word_at(0).as_deref() {
            Some("NULL") => JsonNullHandling::NullOnNull,
            Some("ABSENT") => JsonNullHandling::AbsentOnNull,
            _ => return None,
        };
        if !(self.kw_at(1, "ON") && self.kw_at(2, "NULL")) {
            return None;
        }
        self.at += 3;
        Some(handling)
    }

    /// `JSON_OBJECT ( [key : value [, ...]] [NULL|ABSENT ON NULL] )`, the cursor at the `(`.
    fn json_object(&mut self) -> Parsing<Expression> {
        self.expect_char('(')?;
        let mut pairs: Vec<(Expression, Expression)> = Vec::new();
        let mut null_handling: Option<JsonNullHandling> = self.json_null_clause();
        if null_handling.is_none() && !self.is_char(')') {
            loop {
                let key: Expression = self.scalar()?;
                self.expect_char(':')?;
                pairs.push((key, self.scalar()?));
                if !self.eat_char(',') {
                    null_handling = self.json_null_clause();
                    break;
                }
            }
        }
        self.expect_char(')')?;
        Ok(Expression::JsonObject(Box::new(JsonObjectFunc {
            pairs,
            null_handling,
            with_unique_keys: false,
            returning_type: None,
            format_json: false,
            encoding: None,
            star: false,
        })))
    }

    /// `JSON_ARRAY ( [value [, ...]] [NULL|ABSENT ON NULL] )`, the cursor at the `(`.
    fn json_array(&mut self) -> Parsing<Expression> {
        self.expect_char('(')?;
        let mut expressions: Vec<Expression> = Vec::new();
        let mut null_handling: Option<JsonNullHandling> = self.json_null_clause();
        if null_handling.is_none() && !self.is_char(')') {
            loop {
                expressions.push(self.scalar()?);
                if !self.eat_char(',') {
                    null_handling = self.json_null_clause();
                    break;
                }
            }
        }
        self.expect_char(')')?;
        let null_handling: Option<Box<Expression>> = null_handling.map(|handling| {
            Box::new(var(match handling {
                JsonNullHandling::NullOnNull => "NULL ON NULL",
                JsonNullHandling::AbsentOnNull => "ABSENT ON NULL",
            }))
        });
        Ok(Expression::JSONArray(Box::new(JSONArray {
            expressions,
            null_handling,
            return_type: None,
            strict: None,
        })))
    }

    /// `( [PARTITION BY ...] [ORDER BY ...] [ROWS|RANGE frame] )`, or a window name inside it.
    pub(crate) fn window_spec(&mut self) -> Parsing<Over> {
        self.expect_char('(')?;
        let base: Option<Identifier> =
            match self.is(is_name) && !(self.kw_in(0, &["PARTITION", "ORDER", "ROWS", "RANGE"])) {
                true => Some(self.name()?),
                false => None,
            };
        let mut partition: Vec<Expression> = Vec::new();
        if self.kw("PARTITION") && self.kw_at(1, "BY") {
            self.at += 2;
            partition = self.comma_list(Self::scalar)?;
        }
        let order: Vec<Ordered> = match self.kw("ORDER") {
            true => self.order_by_clause()?,
            false => Vec::new(),
        };
        let frame: Option<WindowFrame> = self.window_frame()?;
        self.expect_char(')')?;
        Ok(over(base, partition, order, frame))
    }

    fn window_frame(&mut self) -> Parsing<Option<WindowFrame>> {
        let kind: WindowFrameKind = match self.word_at(0).as_deref() {
            Some("RANGE") => WindowFrameKind::Range,
            Some("ROWS") => WindowFrameKind::Rows,
            _ => return Ok(None),
        };
        let kind_text: String = self.advance()?.value.to_ascii_uppercase();
        let (start, end) = match self.eat_kw("BETWEEN") {
            true => {
                let start: WindowFrameBound = self.frame_bound()?;
                self.expect_kw("AND")?;
                (start, Some(self.frame_bound()?))
            }
            false => (self.frame_bound()?, None),
        };
        Ok(Some(WindowFrame {
            kind,
            start,
            end,
            exclude: None,
            kind_text: Some(kind_text),
            start_side_text: None,
            end_side_text: None,
        }))
    }

    fn frame_bound(&mut self) -> Parsing<WindowFrameBound> {
        if self.kw("UNBOUNDED") && self.kw_in(1, &["PRECEDING", "FOLLOWING"]) {
            let preceding: bool = self.kw_at(1, "PRECEDING");
            self.at += 2;
            return Ok(match preceding {
                true => WindowFrameBound::UnboundedPreceding,
                false => WindowFrameBound::UnboundedFollowing,
            });
        }
        if self.kw("CURRENT") && self.kw_at(1, "ROW") {
            self.at += 2;
            return Ok(WindowFrameBound::CurrentRow);
        }
        let offset: Expression = self.scalar()?;
        if self.eat_kw("PRECEDING") {
            return Ok(WindowFrameBound::Preceding(Box::new(offset)));
        }
        self.expect_kw("FOLLOWING")?;
        Ok(WindowFrameBound::Following(Box::new(offset)))
    }

    /// `ORDER BY item [ASC|DESC], ...`.
    pub(crate) fn order_by_clause(&mut self) -> Parsing<Vec<Ordered>> {
        self.expect_kws(&["ORDER", "BY"])?;
        self.comma_list(|parser| {
            let this: Expression = parser.scalar()?;
            let desc: bool = parser.eat_kw("DESC");
            let explicit_asc: bool = !desc && parser.eat_kw("ASC");
            Ok(Ordered {
                nulls_auto: false,
                this,
                desc,
                nulls_first: None,
                explicit_asc,
                with_fill: None,
            })
        })
    }

    /// `CASE [operand] WHEN ... THEN ... [ELSE ...] END`.
    fn case_expression(&mut self) -> Parsing<Expression> {
        self.expect_kw("CASE")?;
        let operand: Option<Expression> = match self.kw("WHEN") {
            true => None,
            false => Some(self.scalar()?),
        };
        let mut whens: Vec<(Expression, Expression)> = Vec::new();
        while self.eat_kw("WHEN") {
            let test: Expression = match operand {
                Some(_) => self.scalar()?,
                None => self.condition()?,
            };
            self.expect_kw("THEN")?;
            whens.push((test, self.scalar()?));
        }
        if whens.is_empty() {
            return Err(self.error());
        }
        let else_: Option<Expression> = match self.eat_kw("ELSE") {
            true => Some(self.scalar()?),
            false => None,
        };
        self.expect_kw("END")?;
        Ok(case(operand, whens, else_))
    }

    /// `CAST ( value AS type )` and `TRY_CAST`.
    fn cast_expression(&mut self, name: &str) -> Parsing<Expression> {
        self.at += 2;
        let this: Expression = self.scalar()?;
        self.expect_kw("AS")?;
        let to = self.data_type()?;
        self.expect_char(')')?;
        let node = Box::new(cast(this, to, false));
        Ok(match name {
            "TRY_CAST" => Expression::TryCast(node),
            _ => Expression::Cast(node),
        })
    }

    /// `CONVERT ( type, value [, style] )` and `TRY_CONVERT`.
    fn convert_expression(&mut self, name: &str) -> Parsing<Expression> {
        let name_token: Token = self.peek().cloned().ok_or_else(|| self.error())?;
        self.at += 2;
        let to = self.data_type()?;
        self.expect_char(',')?;
        let this: Expression = self.scalar()?;
        let style: Option<Expression> = match self.eat_char(',') {
            true => Some(self.scalar()?),
            false => None,
        };
        self.expect_char(')')?;
        if name == TRY_CONVERT_FUNCTION {
            let mut args: Vec<Expression> = vec![Expression::DataType(to), this];
            args.extend(style);
            let mut node: Function = function(vec![identifier(&name_token)], args);
            node.span = SourceSpan(Some(Span {
                start: name_token.span.start,
                end: self.tokens[self.at - 1].span.end,
                line: 0,
                column: 0,
            }));
            return Ok(Expression::Function(Box::new(node)));
        }
        Ok(Expression::Convert(Box::new(ConvertFunc {
            this,
            to,
            style,
        })))
    }

    /// `NEXT VALUE FOR sequence`.
    fn next_value_for(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.at += 3;
        let name: Vec<Identifier> = self.multipart(3)?;
        if self.kw("OVER") {
            return Err(self.gap("next_value_over", start));
        }
        Ok(Expression::NextValueFor(Box::new(NextValueFor {
            this: Box::new(column_from(name)),
            order: None,
        })))
    }
}
