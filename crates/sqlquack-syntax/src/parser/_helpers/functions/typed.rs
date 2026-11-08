//! Parser functions: typed.

use crate::expressions::functions::main::make_unquoted_function::make_unquoted_function;
use crate::expressions::functions::models::AggregateFunction;
use crate::expressions::functions::models::Function;
use crate::expressions::functions::models::JSONArray;
use crate::expressions::functions::models::JSONArrayAgg;
use crate::expressions::functions::models::JSONFormat;
use crate::expressions::functions::models::JSONKeyValue;
use crate::expressions::functions::models::JSONObjectAgg;
use crate::expressions::functions::models::JSONTable;
use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Literal;
use crate::expressions::names::models::Null;
use crate::expressions::names::models::Star;
use crate::expressions::operators::models::Cast;
use crate::expressions::queries::models::Limit;
use crate::expressions::queries::models::Offset;
use crate::expressions::queries::models::OrderBy;
use crate::expressions::queries::models::Ordered;
use crate::expressions::queries::models::Subquery;
use crate::expressions::statements::models::Var;
use crate::expressions::tree::models::Expression;
use crate::expressions::typed_functions::main::new_unary_func::new_unary_func;
use crate::expressions::typed_functions::models::AggFunc;
use crate::expressions::typed_functions::models::CountFunc;
use crate::expressions::typed_functions::models::CumeDist;
use crate::expressions::typed_functions::models::DenseRank;
use crate::expressions::typed_functions::models::GroupConcatFunc;
use crate::expressions::typed_functions::models::JsonExtractFunc;
use crate::expressions::typed_functions::models::JsonNullHandling;
use crate::expressions::typed_functions::models::JsonObjectFunc;
use crate::expressions::typed_functions::models::LeadLagFunc;
use crate::expressions::typed_functions::models::ListAggFunc;
use crate::expressions::typed_functions::models::ListAggOverflow;
use crate::expressions::typed_functions::models::NTileFunc;
use crate::expressions::typed_functions::models::NthValueFunc;
use crate::expressions::typed_functions::models::PercentRank;
use crate::expressions::typed_functions::models::Rank;
use crate::expressions::typed_functions::models::RowNumber;
use crate::expressions::typed_functions::models::StringAggFunc;
use crate::expressions::typed_functions::models::ValueFunc;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::Tuple;
use crate::parser::constants::BINARY_ARITY;
use crate::parser::constants::FIRST_VALUE;
use crate::parser::constants::LEAD;
use crate::parser::constants::TIME_TO_TIME_STR;
use crate::parser::constants::{HASH_AGG, OBJECT_AGG};
use crate::parser::models::Parser;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::Span;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parse phase-3 typed-function slices that are straightforward pass-throughs.
    pub(crate) fn try_parse_phase3_typed_function(
        &mut self,
        name: &str,
        _upper_name: &str,
        canonical_upper_name: &str,
    ) -> Result<Option<Expression>> {
        let Some(behavior) =
            crate::functions::dispatch::main::parser_dispatch_behavior_by_name_upper::parser_dispatch_behavior_by_name_upper(canonical_upper_name)
        else {
            return Ok(None);
        };

        match behavior {
            crate::functions::dispatch::models::ParserDispatchBehavior::ExprListFunction => {
                let args = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(make_unquoted_function(name, args)))
            }
            crate::functions::dispatch::models::ParserDispatchBehavior::OptionalExprListFunction => {
                let args = if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    self.parse_expression_list()?
                };
                self.expect_token(TokenType::RParen)?;
                Ok(Some(make_unquoted_function(name, args)))
            }
            crate::functions::dispatch::models::ParserDispatchBehavior::FunctionArgumentsFunction => {
                let args = self.parse_function_arguments()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(make_unquoted_function(name, args)))
            }
            crate::functions::dispatch::models::ParserDispatchBehavior::ZeroArgFunction => {
                self.expect_token(TokenType::RParen)?;
                Ok(Some(make_unquoted_function(name, Vec::new())))
            }
            crate::functions::dispatch::models::ParserDispatchBehavior::ExprListMaybeAggregateByFilter => {
                let args = if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    self.parse_expression_list()?
                };
                self.expect_token(TokenType::RParen)?;
                let filter = self.parse_filter_clause()?;
                if filter.is_some() {
                    Ok(Some(Self::make_simple_aggregate(name, args, false, filter)))
                } else {
                    Ok(Some(make_unquoted_function(name, args)))
                }
            }
            crate::functions::dispatch::models::ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix => {
                let args = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                let filter = self.parse_filter_clause()?;
                if canonical_upper_name.ends_with("_AGG") || filter.is_some() {
                    Ok(Some(Self::make_simple_aggregate(name, args, false, filter)))
                } else {
                    Ok(Some(make_unquoted_function(name, args)))
                }
            }
            crate::functions::dispatch::models::ParserDispatchBehavior::HashLike => {
                let distinct =
                    canonical_upper_name == HASH_AGG && self.match_token(TokenType::Distinct);
                let args = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                let filter = self.parse_filter_clause()?;
                if canonical_upper_name == HASH_AGG || filter.is_some() {
                    Ok(Some(Self::make_simple_aggregate(name, args, distinct, filter)))
                } else {
                    Ok(Some(make_unquoted_function(name, args)))
                }
            }
            crate::functions::dispatch::models::ParserDispatchBehavior::HllAggregate => {
                let distinct = self.match_token(TokenType::Distinct);
                let args = if self.match_token(TokenType::Star) {
                    vec![Expression::Star(Star {
                        table: None,
                        ilike: None,
                        except: None,
                        replace: None,
                        rename: None,
                        trailing_comments: Vec::new(),
                        span: None,
                        qualified: None,
                    })]
                } else if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    self.parse_expression_list()?
                };
                self.expect_token(TokenType::RParen)?;
                let filter = self.parse_filter_clause()?;
                Ok(Some(Self::make_simple_aggregate(
                    name, args, distinct, filter,
                )))
            }
            crate::functions::dispatch::models::ParserDispatchBehavior::PercentileAggregate => {
                let distinct = self.match_token(TokenType::Distinct);
                if !distinct {
                    self.match_token(TokenType::All);
                }
                let args = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                let filter = self.parse_filter_clause()?;
                Ok(Some(Self::make_simple_aggregate(
                    name, args, distinct, filter,
                )))
            }
            crate::functions::dispatch::models::ParserDispatchBehavior::ExprListAggregate => {
                let distinct =
                    canonical_upper_name == OBJECT_AGG && self.match_token(TokenType::Distinct);
                let args = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                let filter = self.parse_filter_clause()?;
                Ok(Some(Self::make_simple_aggregate(name, args, distinct, filter)))
            }
            crate::functions::dispatch::models::ParserDispatchBehavior::UnaryAggregate => {
                let this = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                let filter = self.parse_filter_clause()?;
                Ok(Some(Self::make_simple_aggregate(
                    name,
                    vec![this],
                    false,
                    filter,
                )))
            }
            crate::functions::dispatch::models::ParserDispatchBehavior::TranslateNonTeradata => {
                let args = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(make_unquoted_function(name, args)))
            }
        }
    }

    pub(crate) fn parse_typed_function(
        &mut self,
        name: &str,
        upper_name: &str,
        quoted: bool,
    ) -> Result<Expression> {
        if self.config.faithful_calls
            && self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake)
        {
            return self.parse_faithful_call(name, upper_name, quoted);
        }
        self.parse_typed_function_lowered(name, upper_name, quoted)
    }

    /// polyglot's typed parse of a call, lowering it into a cross-dialect node where it can.
    pub(crate) fn parse_typed_function_lowered(
        &mut self,
        name: &str,
        upper_name: &str,
        quoted: bool,
    ) -> Result<Expression> {
        let canonical_upper_name =
            crate::functions::typed::main::canonical_typed_function_name_upper::canonical_typed_function_name_upper(upper_name);

        if canonical_upper_name == TIME_TO_TIME_STR {
            let arg = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Expression::Cast(Box::new(Cast {
                this: arg,
                to: DataType::Text,
                trailing_comments: Vec::new(),
                double_colon_syntax: false,
                format: None,
                default: None,
                inferred_type: None,
                span: SourceSpan::default(),
            })));
        }

        if let Some(expr) = self.try_parse_simple_unary_typed_function(canonical_upper_name)? {
            return Ok(expr);
        }

        if let Some(expr) =
            self.try_parse_registry_typed_function(name, upper_name, canonical_upper_name, quoted)?
        {
            return Ok(expr);
        }
        if let Some(expr) =
            self.try_parse_phase3_typed_function(name, upper_name, canonical_upper_name)?
        {
            return Ok(expr);
        }

        self.parse_generic_function(name, quoted)
    }

    /// SQLQuack: whether a faithful parse keeps a path key in brackets, so the path stays unambiguous.
    pub(crate) fn faithful_path_key_needs_brackets(&self, key: &str) -> bool {
        self.config.faithful_calls && key.contains(['.', '[', ']', '"', ':'])
    }

    /// SQLQuack: a call as written, unless polyglot keeps it or the typed parse is the call itself.
    pub(crate) fn parse_faithful_call(
        &mut self,
        name: &str,
        upper_name: &str,
        quoted: bool,
    ) -> Result<Expression> {
        let saved = self.current;
        if let Some((expr, end)) = self.faithful_calls.get(&saved) {
            let expr = expr.clone();
            self.current = *end;
            return Ok(expr);
        }
        let comments = self.call_comments(saved);
        let typed = self.parse_typed_function_lowered(name, upper_name, quoted)?;
        let mut expr = match Self::call_name(&typed) {
            Some(kept) if kept.eq_ignore_ascii_case(name) => typed,
            _ if crate::functions::names::main::faithful_keyword_syntax_name::faithful_keyword_syntax_name(upper_name)
                && self.tokens_use_keyword_syntax(saved) =>
            {
                typed
            }
            _ => {
                let typed_end = self.current;
                let typed_comments = self.call_comments(saved);
                self.restore_comments(comments);
                self.current = saved;
                match self.parse_generic_function(name, quoted) {
                    Ok(generic) if self.current == typed_end => generic,
                    _ => {
                        self.restore_comments(typed_comments);
                        self.current = typed_end;
                        typed
                    }
                }
            }
        };
        if self.config.record_call_spans {
            self.record_call_span(&mut expr, saved);
        }
        self.faithful_calls
            .insert(saved, (expr.clone(), self.current));
        Ok(expr)
    }

    /// SQLQuack: comments on the tokens of the call starting at `start`, so a parse can be undone.
    pub(crate) fn call_comments(&self, start: usize) -> Vec<(usize, Vec<String>, Vec<String>)> {
        let mut depth = 0_usize;
        let mut saved: Vec<(usize, Vec<String>, Vec<String>)> = Vec::new();
        for index in start.saturating_sub(2)..self.tokens.len() {
            let token = &self.tokens[index];
            if !token.comments.is_empty() || !token.trailing_comments.is_empty() {
                saved.push((
                    index,
                    token.comments.clone(),
                    token.trailing_comments.clone(),
                ));
            }
            match token.token_type {
                TokenType::LParen | TokenType::LBracket | TokenType::LBrace if index >= start => {
                    depth += 1
                }
                TokenType::RParen | TokenType::RBracket | TokenType::RBrace if index >= start => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
        saved
    }

    /// SQLQuack: puts back comments saved by [`Self::call_comments`].
    pub(crate) fn restore_comments(&mut self, saved: Vec<(usize, Vec<String>, Vec<String>)>) {
        let indices: Vec<usize> = saved.iter().map(|(index, _, _)| *index).collect();
        if let (Some(first), Some(last)) = (indices.first(), indices.last()) {
            for index in *first..=*last {
                self.tokens[index].comments.clear();
                self.tokens[index].trailing_comments.clear();
            }
        }
        for (index, comments, trailing) in saved {
            self.tokens[index].comments = comments;
            self.tokens[index].trailing_comments = trailing;
        }
    }

    /// The name of a plain or aggregate call.
    pub(crate) fn call_name(expr: &Expression) -> Option<&str> {
        match expr {
            Expression::Function(function) => Some(&function.name),
            Expression::AggregateFunction(function) => Some(&function.name),
            _ => None,
        }
    }

    /// SQLQuack: records, on a call node, the source span from its name to its last token.
    pub(crate) fn record_call_span(&self, expr: &mut Expression, arguments_start: usize) {
        let Some(name_token) = arguments_start
            .checked_sub(2)
            .and_then(|index| self.tokens.get(index))
        else {
            return;
        };
        if self.tokens[arguments_start - 1].token_type != TokenType::LParen {
            return;
        }
        let span = Span {
            start: name_token.span.start,
            end: self.tokens[self.current - 1].span.end,
            line: 0,
            column: 0,
        };
        match expr {
            Expression::Function(function) => function.span = SourceSpan(Some(span)),
            Expression::AggregateFunction(function) => function.span = SourceSpan(Some(span)),
            _ => {}
        }
    }

    /// SQLQuack: whether arguments from `start` use top-level keywords, as `EXTRACT(YEAR FROM d)`.
    pub(crate) fn tokens_use_keyword_syntax(&self, start: usize) -> bool {
        let mut depth = 0_usize;
        for token in &self.tokens[start..] {
            match token.token_type {
                TokenType::LParen | TokenType::LBracket | TokenType::LBrace => depth += 1,
                TokenType::RParen | TokenType::RBracket | TokenType::RBrace => {
                    if depth == 0 {
                        return false;
                    }
                    depth -= 1;
                }
                TokenType::From
                | TokenType::In
                | TokenType::For
                | TokenType::Both
                | TokenType::Leading
                | TokenType::Trailing
                | TokenType::Placing
                | TokenType::As
                | TokenType::Using
                | TokenType::Separator
                | TokenType::Returning
                | TokenType::On
                | TokenType::Key
                    if depth == 0 =>
                {
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    pub(crate) fn parse_typed_aggregate_family(
        &mut self,
        name: &str,
        upper_name: &str,
        canonical_upper_name: &str,
    ) -> Result<Expression> {
        match canonical_upper_name {
            "COUNT" => self.parse_typed_count(name),

            "LIST" => self.parse_typed_list(name),

            "MAP" => {
                let args = if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    self.parse_function_arguments()?
                };
                self.expect_token(TokenType::RParen)?;
                Ok(Expression::Function(Box::new(Function {
                    on_error: None,
                    qualified_name: Vec::new(),
                    source_dialect: None,

                    name: name.to_string(),
                    args,
                    distinct: false,
                    trailing_comments: Vec::new(),
                    use_bracket_syntax: false,
                    no_parens: false,
                    quoted: false,
                    span: SourceSpan::default(),
                    inferred_type: None,
                    export_state: false,
                })))
            }

            "ARRAY" => self.parse_typed_array_function(name),

            "SUM"
            | "AVG"
            | "MIN"
            | "MAX"
            | "ARRAY_AGG"
            | "ARRAY_CONCAT_AGG"
            | "STDDEV"
            | "STDDEV_POP"
            | "STDDEV_SAMP"
            | "VARIANCE"
            | "VAR_POP"
            | "VAR_SAMP"
            | "BOOL_AND"
            | "BOOL_OR"
            | "MEDIAN"
            | "MODE"
            | "FIRST"
            | "LAST"
            | "ANY_VALUE"
            | "APPROX_DISTINCT"
            | "APPROX_COUNT_DISTINCT"
            | "BIT_AND"
            | "BIT_OR"
            | "BIT_XOR" => {
                self.parse_typed_simple_aggregate(canonical_upper_name, name, upper_name)
            }

            "STRING_AGG" => self.parse_typed_string_agg(),

            "GROUP_CONCAT" => self.parse_typed_group_concat(),

            "LISTAGG" => self.parse_typed_listagg(),
            _ => Err(crate::tokens::errors::Error::internal(format!(
                "phase-6 aggregate parser called with non-aggregate family name '{}'",
                canonical_upper_name
            ))),
        }
    }

    fn parse_typed_count(&mut self, name: &str) -> Result<Expression> {
        let count_start = self.tokens[self.current.saturating_sub(2)].span;
        let (this, star, distinct) = if self.check(TokenType::RParen) {
            (None, false, false)
        } else if self.check(TokenType::Star) && !self.check_next(TokenType::RParen) {
            (Some(self.parse_expression()?), false, false)
        } else if self.match_token(TokenType::Star) {
            (None, true, false)
        } else if self.match_token(TokenType::All) {
            (Some(self.parse_expression()?), false, false)
        } else if self.match_token(TokenType::Distinct) {
            let first_expr = self.parse_expression()?;
            if self.match_token(TokenType::Comma) {
                let mut args = vec![first_expr];
                loop {
                    args.push(self.parse_expression()?);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                (
                    Some(Expression::Tuple(Box::new(Tuple { expressions: args }))),
                    false,
                    true,
                )
            } else {
                (Some(first_expr), false, true)
            }
        } else {
            let first_expr = self.parse_expression()?;
            let first_expr = { first_expr };
            if self.match_token(TokenType::Comma) {
                let mut args = vec![first_expr];
                loop {
                    args.push(self.parse_expression()?);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
                if self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake) {
                    let filter = self.parse_filter_clause()?;
                    return Ok(Self::make_simple_aggregate(name, args, false, filter));
                }
                return Ok(make_unquoted_function(name, args));
            }
            (Some(first_expr), false, false)
        };
        let ignore_nulls =
            if self.match_token(TokenType::Ignore) && self.match_token(TokenType::Nulls) {
                Some(true)
            } else if self.match_token(TokenType::Respect) && self.match_token(TokenType::Nulls) {
                Some(false)
            } else {
                None
            };
        self.expect_token(TokenType::RParen)?;
        let filter = self.parse_filter_clause()?;
        let ignore_nulls = if ignore_nulls.is_some() {
            ignore_nulls
        } else if self.match_keywords(&[TokenType::Ignore, TokenType::Nulls]) {
            Some(true)
        } else if self.match_keywords(&[TokenType::Respect, TokenType::Nulls]) {
            Some(false)
        } else {
            None
        };
        Ok(Expression::Count(Box::new(CountFunc {
            span: Some(count_start.through(self.previous().span)),
            this,
            star,
            distinct,
            filter,
            ignore_nulls,
            original_name: Some(name.to_string()),
            inferred_type: None,
        })))
    }

    fn parse_typed_list(&mut self, name: &str) -> Result<Expression> {
        let distinct = self.match_token(TokenType::Distinct);
        let args = if self.check(TokenType::RParen) {
            Vec::new()
        } else {
            self.parse_function_arguments()?
        };
        let order_by = if self.match_token(TokenType::Order) {
            self.expect_token(TokenType::By)?;
            self.parse_order_by_list()?
        } else {
            Vec::new()
        };
        let limit = if self.match_token(TokenType::Limit) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        let filter = self.parse_filter_clause()?;

        if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::DuckDB)
        ) || distinct
            || !order_by.is_empty()
            || limit.is_some()
            || filter.is_some()
        {
            Ok(Expression::AggregateFunction(Box::new(AggregateFunction {
                source_dialect: None,

                name: name.to_string(),
                args,
                distinct,
                filter,
                order_by,
                limit,
                ignore_nulls: None,
                inferred_type: None,
                span: SourceSpan::default(),
                export_state: false,
                qualified_name: Vec::new(),
                quoted: false,
            })))
        } else {
            Ok(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: name.to_string(),
                args,
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: SourceSpan::default(),
                inferred_type: None,
                export_state: false,
            })))
        }
    }

    fn parse_typed_array_function(&mut self, name: &str) -> Result<Expression> {
        if self.check(TokenType::Select) {
            let query = self.parse_select()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: name.to_string(),
                args: vec![query],
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: SourceSpan::default(),
                inferred_type: None,
                export_state: false,
            })));
        }
        if self.check(TokenType::LParen) {
            let saved_pos = self.current;
            self.skip();

            if self.check(TokenType::Select) || self.check(TokenType::With) {
                let inner_query = self.parse_statement()?;
                self.expect_token(TokenType::RParen)?;

                let limit = if self.match_token(TokenType::Limit) {
                    let expr = self.parse_expression()?;
                    Some(Limit {
                        this: expr,
                        percent: false,
                        comments: Vec::new(),
                    })
                } else {
                    None
                };

                let offset = if self.match_token(TokenType::Offset) {
                    let expr = self.parse_expression()?;
                    let rows =
                        if self.match_token(TokenType::Row) || self.match_token(TokenType::Rows) {
                            Some(true)
                        } else {
                            None
                        };
                    Some(Offset { this: expr, rows })
                } else {
                    None
                };

                self.expect_token(TokenType::RParen)?;

                let subquery = Expression::Subquery(Box::new(Subquery {
                    this: inner_query,
                    alias: None,
                    column_aliases: Vec::new(),
                    alias_explicit_as: false,
                    alias_keyword: None,
                    order_by: None,
                    limit,
                    offset,
                    lateral: false,
                    modifiers_inside: false,
                    trailing_comments: Vec::new(),
                    distribute_by: None,
                    sort_by: None,
                    cluster_by: None,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }));

                return Ok(Expression::Function(Box::new(Function {
                    on_error: None,
                    qualified_name: Vec::new(),
                    source_dialect: None,

                    name: name.to_string(),
                    args: vec![subquery],
                    distinct: false,
                    trailing_comments: Vec::new(),
                    use_bracket_syntax: false,
                    no_parens: false,
                    quoted: false,
                    span: SourceSpan::default(),
                    inferred_type: None,
                    export_state: false,
                })));
            } else {
                self.current = saved_pos;
            }
        }
        let args = if self.check(TokenType::RParen) {
            Vec::new()
        } else {
            self.parse_function_arguments()?
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Function(Box::new(Function {
            on_error: None,
            qualified_name: Vec::new(),
            source_dialect: None,

            name: name.to_string(),
            args,
            distinct: false,
            trailing_comments: Vec::new(),
            use_bracket_syntax: false,
            no_parens: false,
            quoted: false,
            span: SourceSpan::default(),
            inferred_type: None,
            export_state: false,
        })))
    }

    fn parse_typed_simple_aggregate(
        &mut self,
        canonical_upper_name: &str,
        name: &str,
        upper_name: &str,
    ) -> Result<Expression> {
        let distinct = if self.match_token(TokenType::Distinct) {
            true
        } else {
            self.match_token(TokenType::All);
            false
        };

        if self.check(TokenType::RParen) {
            self.expect_token(TokenType::RParen)?;
            let filter = self.parse_filter_clause()?;
            let agg = AggFunc {
                ignore_nulls: None,
                this: Expression::Null(Null {}),
                distinct: false,
                filter,
                order_by: Vec::new(),
                having_max: None,
                name: Some(name.to_string()),
                limit: None,
                inferred_type: None,
            };
            return Ok(match upper_name {
                "MODE" => Expression::Mode(Box::new(agg)),
                _ if self.is_snowflake() => {
                    Expression::Function(Box::new(Function::new(name, Vec::new())))
                }
                _ => {
                    return Err(
                        self.parse_error(format!("{} cannot have zero arguments", upper_name))
                    );
                }
            });
        }

        let first_arg = self.parse_expression_with_clickhouse_alias()?;

        if self.match_token(TokenType::Comma) {
            let is_ignore_nulls_func = matches!(upper_name, "FIRST" | "LAST" | "ANY_VALUE");

            let second_arg = self.parse_expression()?;

            if is_ignore_nulls_func && self.check(TokenType::RParen) {
                if let Expression::Boolean(BooleanLiteral { value: true }) = &second_arg {
                    self.expect_token(TokenType::RParen)?;
                    let filter = self.parse_filter_clause()?;
                    let agg = AggFunc {
                        ignore_nulls: Some(true),
                        this: first_arg,
                        distinct,
                        filter,
                        order_by: Vec::new(),
                        having_max: None,
                        name: Some(name.to_string()),
                        limit: None,
                        inferred_type: None,
                    };
                    return Ok(match upper_name {
                        "FIRST" => Expression::First(Box::new(agg)),
                        "LAST" => Expression::Last(Box::new(agg)),
                        "ANY_VALUE" => Expression::AnyValue(Box::new(agg)),
                        _ => {
                            return Err(crate::tokens::errors::Error::internal(
                                "function name already matched by is_ignore_nulls_func guard",
                            ))
                        }
                    });
                }
            }

            let mut args = vec![first_arg, second_arg];
            while self.match_token(TokenType::Comma) {
                args.push(self.parse_expression()?);
            }
            let is_duckdb_top_n = matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::DuckDB)
            ) && matches!(canonical_upper_name, "MIN" | "MAX")
                && args.len() == BINARY_ARITY;
            let order_by =
                if is_duckdb_top_n && self.match_keywords(&[TokenType::Order, TokenType::By]) {
                    self.parse_order_by_list()?
                } else {
                    Vec::new()
                };
            self.expect_token(TokenType::RParen)?;

            if is_duckdb_top_n {
                let filter = self.parse_filter_clause()?;
                Ok(Expression::AggregateFunction(Box::new(AggregateFunction {
                    source_dialect: None,

                    name: name.to_string(),
                    args,
                    distinct,
                    filter,
                    order_by,
                    limit: None,
                    ignore_nulls: None,
                    inferred_type: None,
                    span: SourceSpan::default(),
                    export_state: false,
                    qualified_name: Vec::new(),
                    quoted: false,
                })))
            } else {
                Ok(Expression::Function(Box::new(Function {
                    on_error: None,
                    qualified_name: Vec::new(),
                    source_dialect: None,

                    name: name.to_string(),
                    args,
                    distinct,
                    trailing_comments: Vec::new(),
                    use_bracket_syntax: false,
                    no_parens: false,
                    quoted: false,
                    span: SourceSpan::default(),
                    inferred_type: None,
                    export_state: false,
                })))
            }
        } else {
            let ignore_nulls = if self.match_token(TokenType::Ignore)
                && self.match_token(TokenType::Nulls)
            {
                Some(true)
            } else if self.match_token(TokenType::Respect) && self.match_token(TokenType::Nulls) {
                Some(false)
            } else {
                None
            };

            let having_max = if self.match_token(TokenType::Having) {
                let is_max = if self.check_keyword_text("MAX") {
                    self.skip();
                    true
                } else if self.check_keyword_text("MIN") {
                    self.skip();
                    false
                } else {
                    return Err(self.parse_error("Expected MAX or MIN after HAVING in aggregate"));
                };
                let expr = self.parse_expression()?;
                Some((Box::new(expr), is_max))
            } else {
                None
            };

            let order_by = if self.match_keywords(&[TokenType::Order, TokenType::By]) {
                self.parse_order_by_list()?
            } else {
                Vec::new()
            };
            let limit = if self.match_token(TokenType::Limit) {
                let first = self.parse_expression()?;
                if self.match_token(TokenType::Comma) {
                    let second = self.parse_expression()?;
                    Some(Box::new(Expression::Tuple(Box::new(Tuple {
                        expressions: vec![first, second],
                    }))))
                } else {
                    Some(Box::new(first))
                }
            } else {
                None
            };
            self.expect_token(TokenType::RParen)?;
            let filter = self.parse_filter_clause()?;
            let ignore_nulls = if ignore_nulls.is_some() {
                ignore_nulls
            } else if self.match_keywords(&[TokenType::Ignore, TokenType::Nulls]) {
                Some(true)
            } else if self.match_keywords(&[TokenType::Respect, TokenType::Nulls]) {
                Some(false)
            } else {
                None
            };
            let agg = AggFunc {
                ignore_nulls,
                this: first_arg,
                distinct,
                filter,
                order_by,
                having_max,
                name: Some(name.to_string()),
                limit,
                inferred_type: None,
            };
            Ok(match canonical_upper_name {
                "SUM" => Expression::Sum(Box::new(agg)),
                "AVG" => Expression::Avg(Box::new(agg)),
                "MIN" => Expression::Min(Box::new(agg)),
                "MAX" => Expression::Max(Box::new(agg)),
                "ARRAY_AGG" => Expression::ArrayAgg(Box::new(agg)),
                "ARRAY_CONCAT_AGG" => Expression::ArrayConcatAgg(Box::new(agg)),
                "STDDEV" => Expression::Stddev(Box::new(agg)),
                "STDDEV_POP" => Expression::StddevPop(Box::new(agg)),
                "STDDEV_SAMP" => Expression::StddevSamp(Box::new(agg)),
                "VARIANCE" => Expression::Variance(Box::new(agg)),
                "VAR_POP" => Expression::VarPop(Box::new(agg)),
                "VAR_SAMP" => Expression::VarSamp(Box::new(agg)),
                "BOOL_AND" => Expression::LogicalAnd(Box::new(agg)),
                "BOOL_OR" => Expression::LogicalOr(Box::new(agg)),
                "MEDIAN" => Expression::Median(Box::new(agg)),
                "MODE" => Expression::Mode(Box::new(agg)),
                "FIRST" => Expression::First(Box::new(agg)),
                "LAST" => Expression::Last(Box::new(agg)),
                "ANY_VALUE" => Expression::AnyValue(Box::new(agg)),
                "APPROX_DISTINCT" => Expression::ApproxDistinct(Box::new(agg)),
                "APPROX_COUNT_DISTINCT" => Expression::ApproxCountDistinct(Box::new(agg)),
                "BIT_AND" => Expression::BitwiseAndAgg(Box::new(agg)),
                "BIT_OR" => Expression::BitwiseOrAgg(Box::new(agg)),
                "BIT_XOR" => Expression::BitwiseXorAgg(Box::new(agg)),
                _ => {
                    return Err(crate::tokens::errors::Error::internal(
                        "aggregate function name already matched in caller",
                    ))
                }
            })
        }
    }

    fn parse_typed_string_agg(&mut self) -> Result<Expression> {
        let distinct = self.match_token(TokenType::Distinct);
        let this = self.parse_expression()?;
        let separator = if self.match_token(TokenType::Comma) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        let order_by = if self.match_keywords(&[TokenType::Order, TokenType::By]) {
            Some(self.parse_order_by_list()?)
        } else {
            None
        };
        let limit = if self.match_token(TokenType::Limit) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        let filter = self.parse_filter_clause()?;
        Ok(Expression::StringAgg(Box::new(StringAggFunc {
            this,
            separator,
            order_by,
            distinct,
            filter,
            limit,
            inferred_type: None,
        })))
    }

    fn parse_typed_group_concat(&mut self) -> Result<Expression> {
        let distinct = self.match_token(TokenType::Distinct);
        let first = self.parse_expression()?;
        let mut exprs = vec![first];
        while self.match_token(TokenType::Comma) {
            if self.check(TokenType::Order) || self.check(TokenType::Separator) {
                break;
            }
            exprs.push(self.parse_expression()?);
        }
        let this = if exprs.len() == 1 {
            exprs.pop().ok_or_else(|| {
                crate::tokens::errors::Error::internal("a value the parser checked is missing")
            })?
        } else {
            Expression::Function(Box::new(Function::new("CONCAT".to_string(), exprs)))
        };
        let order_by = if self.match_keywords(&[TokenType::Order, TokenType::By]) {
            Some(self.parse_order_by_list()?)
        } else {
            None
        };
        let separator = if self.match_token(TokenType::Separator) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        let limit = if self.match_token(TokenType::Limit) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        let filter = self.parse_filter_clause()?;
        Ok(Expression::GroupConcat(Box::new(GroupConcatFunc {
            this,
            separator,
            order_by,
            distinct,
            filter,
            limit,
            inferred_type: None,
        })))
    }

    fn parse_typed_listagg(&mut self) -> Result<Expression> {
        let distinct = self.match_token(TokenType::Distinct);
        let this = self.parse_expression()?;
        let separator = if self.match_token(TokenType::Comma) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        let max_length = None;
        let vertica_overflow = None;

        let on_overflow = if vertica_overflow.is_some() {
            vertica_overflow
        } else if self.match_token(TokenType::On) {
            if self.match_identifier("OVERFLOW") {
                if self.match_identifier("ERROR") {
                    Some(ListAggOverflow::Error)
                } else if self.match_token(TokenType::Truncate) {
                    let filler = if self.check(TokenType::String) {
                        Some(self.parse_expression()?)
                    } else {
                        None
                    };
                    let with_count = if self.match_token(TokenType::With) {
                        self.match_identifier("COUNT");
                        true
                    } else if self.match_identifier("WITHOUT") {
                        self.match_identifier("COUNT");
                        false
                    } else {
                        true
                    };
                    Some(ListAggOverflow::Truncate {
                        filler: filler.map(Box::new),
                        with_count,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::ListAgg(Box::new(ListAggFunc {
            source_dialect: None,
            this,
            separator,
            on_overflow,
            max_length,
            order_by: None,
            distinct,
            filter: None,
            inferred_type: None,
        })))
    }

    pub(crate) fn try_parse_simple_unary_typed_function(
        &mut self,
        canonical_upper_name: &str,
    ) -> Result<Option<Expression>> {
        let use_clickhouse_alias = matches!(canonical_upper_name, "ABS" | "LOWER" | "UPPER");
        let this = match canonical_upper_name {
            "ABS" | "LOWER" | "UPPER" | "SQRT" | "EXP" | "LN" | "SIN" | "COS" | "TAN" | "ASIN"
            | "ACOS" | "RADIANS" | "DEGREES" | "DAYOFWEEK" | "DAYOFYEAR" | "DAYOFMONTH"
            | "WEEKOFYEAR" | "YEAR" | "MONTH" | "DAY" | "HOUR" | "MINUTE" | "SECOND"
            | "DAYOFWEEK_ISO" | "QUARTER" | "EPOCH" | "EPOCH_MS" | "CARDINALITY"
            | "ARRAY_REVERSE" | "ARRAY_DISTINCT" | "ARRAY_COMPACT" | "EXPLODE"
            | "EXPLODE_OUTER" | "MAP_FROM_ENTRIES" | "MAP_KEYS" | "MAP_VALUES" => {
                if use_clickhouse_alias {
                    self.parse_expression_with_clickhouse_alias()?
                } else {
                    self.parse_expression()?
                }
            }
            _ => return Ok(None),
        };
        self.expect_token(TokenType::RParen)?;

        let func = new_unary_func(this);
        let expr = match canonical_upper_name {
            "ABS" => Expression::Abs(Box::new(func)),
            "LOWER" => Expression::Lower(Box::new(func)),
            "UPPER" => Expression::Upper(Box::new(func)),
            "SQRT" => Expression::Sqrt(Box::new(func)),
            "EXP" => Expression::Exp(Box::new(func)),
            "LN" => Expression::Ln(Box::new(func)),
            "SIN" => Expression::Sin(Box::new(func)),
            "COS" => Expression::Cos(Box::new(func)),
            "TAN" => Expression::Tan(Box::new(func)),
            "ASIN" => Expression::Asin(Box::new(func)),
            "ACOS" => Expression::Acos(Box::new(func)),
            "RADIANS" => Expression::Radians(Box::new(func)),
            "DEGREES" => Expression::Degrees(Box::new(func)),
            "DAYOFWEEK" => Expression::DayOfWeek(Box::new(func)),
            "DAYOFYEAR" => Expression::DayOfYear(Box::new(func)),
            "DAYOFMONTH" => Expression::DayOfMonth(Box::new(func)),
            "WEEKOFYEAR" => Expression::WeekOfYear(Box::new(func)),
            "YEAR" => Expression::Year(Box::new(func)),
            "MONTH" => Expression::Month(Box::new(func)),
            "DAY" => Expression::Day(Box::new(func)),
            "HOUR" => Expression::Hour(Box::new(func)),
            "MINUTE" => Expression::Minute(Box::new(func)),
            "SECOND" => Expression::Second(Box::new(func)),
            "DAYOFWEEK_ISO" => Expression::DayOfWeekIso(Box::new(func)),
            "QUARTER" => Expression::Quarter(Box::new(func)),
            "EPOCH" => Expression::Epoch(Box::new(func)),
            "EPOCH_MS" => Expression::EpochMs(Box::new(func)),
            "CARDINALITY" => Expression::Cardinality(Box::new(func)),
            "ARRAY_REVERSE" => Expression::ArrayReverse(Box::new(func)),
            "ARRAY_DISTINCT" => Expression::ArrayDistinct(Box::new(func)),
            "ARRAY_COMPACT" => Expression::ArrayCompact(Box::new(func)),
            "EXPLODE" => Expression::Explode(Box::new(func)),
            "EXPLODE_OUTER" => Expression::ExplodeOuter(Box::new(func)),
            "MAP_FROM_ENTRIES" => Expression::MapFromEntries(Box::new(func)),
            "MAP_KEYS" => Expression::MapKeys(Box::new(func)),
            "MAP_VALUES" => Expression::MapValues(Box::new(func)),
            _ => {
                return Err(crate::tokens::errors::Error::internal(
                    "simple unary typed function already matched",
                ))
            }
        };

        Ok(Some(expr))
    }

    pub(crate) fn parse_typed_window_family(
        &mut self,
        name: &str,
        upper_name: &str,
        canonical_upper_name: &str,
    ) -> Result<Expression> {
        match canonical_upper_name {
            "ROW_NUMBER" => {
                if self.check(TokenType::RParen) {
                    self.skip();
                    Ok(Expression::RowNumber(RowNumber))
                } else {
                    let args = self.parse_function_args_list()?;
                    self.expect_token(TokenType::RParen)?;
                    let trailing_comments = self.previous_trailing_comments().to_vec();
                    Ok(Expression::Function(Box::new(Function {
                        on_error: None,
                        qualified_name: Vec::new(),
                        source_dialect: None,

                        name: name.to_string(),
                        args,
                        distinct: false,
                        trailing_comments,
                        use_bracket_syntax: false,
                        no_parens: false,
                        quoted: false,
                        span: SourceSpan::default(),
                        inferred_type: None,
                        export_state: false,
                    })))
                }
            }
            "RANK" => self.parse_typed_rank(),
            "DENSE_RANK" => {
                let args = if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    let mut args = vec![self.parse_expression()?];
                    while self.match_token(TokenType::Comma) {
                        args.push(self.parse_expression()?);
                    }
                    args
                };
                self.expect_token(TokenType::RParen)?;
                Ok(Expression::DenseRank(DenseRank { args }))
            }
            "PERCENT_RANK" => self.parse_typed_percent_rank(),
            "CUME_DIST" => self.parse_typed_cume_dist(),

            "NTILE" => self.parse_typed_ntile(),

            "LEAD" | "LAG" => self.parse_typed_lead_lag(upper_name),

            "FIRST_VALUE" | "LAST_VALUE" => self.parse_typed_first_last_value(upper_name),

            "NTH_VALUE" => self.parse_typed_nth_value(),
            _ => Err(crate::tokens::errors::Error::internal(format!(
                "phase-6 window parser called with non-window family name '{}'",
                canonical_upper_name
            ))),
        }
    }

    fn parse_typed_rank(&mut self) -> Result<Expression> {
        let (order_by, args) = if self.check(TokenType::RParen) {
            (None, Vec::new())
        } else if self.match_token(TokenType::Order) {
            self.expect_token(TokenType::By)?;
            (Some(self.parse_order_by()?.expressions), Vec::new())
        } else {
            let mut args = vec![self.parse_expression()?];
            while self.match_token(TokenType::Comma) {
                args.push(self.parse_expression()?);
            }
            (None, args)
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Rank(Rank { order_by, args }))
    }

    fn parse_typed_percent_rank(&mut self) -> Result<Expression> {
        let (order_by, args) = if self.check(TokenType::RParen) {
            (None, Vec::new())
        } else if self.match_token(TokenType::Order) {
            self.expect_token(TokenType::By)?;
            (Some(self.parse_order_by()?.expressions), Vec::new())
        } else {
            let mut args = vec![self.parse_expression()?];
            while self.match_token(TokenType::Comma) {
                args.push(self.parse_expression()?);
            }
            (None, args)
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::PercentRank(PercentRank { order_by, args }))
    }

    fn parse_typed_cume_dist(&mut self) -> Result<Expression> {
        let (order_by, args) = if self.check(TokenType::RParen) {
            (None, Vec::new())
        } else if self.match_token(TokenType::Order) {
            self.expect_token(TokenType::By)?;
            (Some(self.parse_order_by()?.expressions), Vec::new())
        } else {
            let mut args = vec![self.parse_expression()?];
            while self.match_token(TokenType::Comma) {
                args.push(self.parse_expression()?);
            }
            (None, args)
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::CumeDist(CumeDist { order_by, args }))
    }

    fn parse_typed_ntile(&mut self) -> Result<Expression> {
        let num_buckets = if self.check(TokenType::RParen) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        while false {
            let _ = self.parse_expression()?;
        }

        let order_by = if self.match_token(TokenType::Order) {
            self.expect_token(TokenType::By)?;
            Some(self.parse_order_by()?.expressions)
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::NTile(Box::new(NTileFunc {
            num_buckets,
            order_by,
        })))
    }

    fn parse_typed_lead_lag(&mut self, upper_name: &str) -> Result<Expression> {
        let this = self.parse_expression()?;
        let (offset, default) = if self.match_token(TokenType::Comma) {
            let off = self.parse_expression()?;
            let def = if self.match_token(TokenType::Comma) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            (Some(off), def)
        } else {
            (None, None)
        };
        let ignore_nulls_inside =
            if self.match_token(TokenType::Ignore) && self.match_token(TokenType::Nulls) {
                Some(true)
            } else if self.match_token(TokenType::Respect) && self.match_token(TokenType::Nulls) {
                Some(false)
            } else {
                None
            };
        self.expect_token(TokenType::RParen)?;
        let ignore_nulls = if ignore_nulls_inside.is_some() {
            ignore_nulls_inside
        } else if self.match_keywords(&[TokenType::Ignore, TokenType::Nulls]) {
            Some(true)
        } else if self.match_keywords(&[TokenType::Respect, TokenType::Nulls]) {
            Some(false)
        } else {
            None
        };
        let func = LeadLagFunc {
            this,
            offset,
            default,
            ignore_nulls,
        };
        Ok(if upper_name == LEAD {
            Expression::Lead(Box::new(func))
        } else {
            Expression::Lag(Box::new(func))
        })
    }

    fn parse_typed_first_last_value(&mut self, upper_name: &str) -> Result<Expression> {
        let this = self.parse_expression()?;
        let order_by = if self.match_keywords(&[TokenType::Order, TokenType::By]) {
            self.parse_order_by_list()?
        } else {
            Vec::new()
        };
        let mut ignore_nulls_inside =
            if self.match_token(TokenType::Ignore) && self.match_token(TokenType::Nulls) {
                Some(true)
            } else if self.match_token(TokenType::Respect) && self.match_token(TokenType::Nulls) {
                Some(false)
            } else {
                None
            };
        if ignore_nulls_inside.is_none() && self.match_token(TokenType::Comma) {
            let second_arg = self.parse_expression()?;
            if let Expression::Boolean(BooleanLiteral { value: true }) = &second_arg {
                ignore_nulls_inside = Some(true);
            }
        }
        self.expect_token(TokenType::RParen)?;
        let ignore_nulls: Option<bool> = if ignore_nulls_inside.is_some() {
            ignore_nulls_inside
        } else if self.match_keywords(&[TokenType::Ignore, TokenType::Nulls]) {
            Some(true)
        } else if self.match_keywords(&[TokenType::Respect, TokenType::Nulls]) {
            Some(false)
        } else {
            None
        };
        let func = ValueFunc {
            this,
            ignore_nulls,
            order_by,
        };
        Ok(if upper_name == FIRST_VALUE {
            Expression::FirstValue(Box::new(func))
        } else {
            Expression::LastValue(Box::new(func))
        })
    }

    fn parse_typed_nth_value(&mut self) -> Result<Expression> {
        let this = self.parse_expression()?;
        self.expect_token(TokenType::Comma)?;
        let offset = self.parse_expression()?;
        let ignore_nulls_inside =
            if self.match_token(TokenType::Ignore) && self.match_token(TokenType::Nulls) {
                Some(true)
            } else if self.match_token(TokenType::Respect) && self.match_token(TokenType::Nulls) {
                Some(false)
            } else {
                None
            };
        self.expect_token(TokenType::RParen)?;
        let from_first = if self.match_keywords(&[TokenType::From, TokenType::First]) {
            Some(true)
        } else if self.match_keywords(&[TokenType::From, TokenType::Last]) {
            Some(false)
        } else {
            None
        };
        let ignore_nulls: Option<bool> = if ignore_nulls_inside.is_some() {
            ignore_nulls_inside
        } else if self.match_keywords(&[TokenType::Ignore, TokenType::Nulls]) {
            Some(true)
        } else if self.match_keywords(&[TokenType::Respect, TokenType::Nulls]) {
            Some(false)
        } else {
            None
        };
        Ok(Expression::NthValue(Box::new(NthValueFunc {
            this,
            offset,
            ignore_nulls,
            from_first,
        })))
    }

    pub(crate) fn parse_typed_json_family(
        &mut self,
        name: &str,
        upper_name: &str,
        canonical_upper_name: &str,
    ) -> Result<Expression> {
        match canonical_upper_name {
            "JSON_EXTRACT" | "JSON_EXTRACT_SCALAR" | "JSON_QUERY" | "JSON_VALUE" => {
                self.parse_typed_json_extract(name, upper_name)
            }
            "JSON_ARRAY_LENGTH" | "JSON_KEYS" | "JSON_TYPE" | "TO_JSON" | "PARSE_JSON" => {
                self.parse_typed_json_unary(canonical_upper_name, name)
            }

            "JSON_OBJECT" => self.parse_typed_json_object(),

            "JSON_ARRAY" => self.parse_typed_json_array(),

            "JSON_ARRAYAGG" => self.parse_typed_json_arrayagg(),

            "JSON_OBJECTAGG" => self.parse_typed_json_objectagg(),

            "JSON_TABLE" => self.parse_typed_json_table(),
            _ => Err(crate::tokens::errors::Error::internal(format!(
                "phase-6 json parser called with non-json family name '{}'",
                canonical_upper_name
            ))),
        }
    }

    fn parse_typed_json_extract(&mut self, name: &str, upper_name: &str) -> Result<Expression> {
        let this = self.parse_expression()?;
        let path = if self.match_token(TokenType::Comma) {
            self.parse_expression()?
        } else {
            Expression::Literal(Box::new(Literal::String("$".to_string())))
        };

        if self.check(TokenType::Comma)
            && !self.check_identifier("WITH")
            && !self.check_identifier("WITHOUT")
            && !self.check_identifier("KEEP")
            && !self.check_identifier("OMIT")
            && !self.check_identifier("NULL")
            && !self.check_identifier("ERROR")
            && !self.check_identifier("EMPTY")
            && !self.check(TokenType::Returning)
        {
            let mut args = vec![this, path];
            while self.match_token(TokenType::Comma) {
                args.push(self.parse_expression()?);
            }
            self.expect_token(TokenType::RParen)?;
            let func_expr = Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: name.to_string(),
                args,
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: SourceSpan::default(),
                inferred_type: None,
                export_state: false,
            }));

            return Ok(func_expr);
        }

        let mut wrapper_option: Option<String> = None;
        let mut quotes_option: Option<String> = None;
        let mut on_scalar_string = false;
        let mut on_error: Option<String> = None;
        let mut returning: Option<DataType> = None;

        while !self.check(TokenType::RParen) {
            if self.match_text_seq(&["WITH", "UNCONDITIONAL", "ARRAY", "WRAPPER"]) {
                wrapper_option = Some("WITH UNCONDITIONAL ARRAY WRAPPER".to_string());
            } else if self.match_text_seq(&["WITH", "CONDITIONAL", "ARRAY", "WRAPPER"]) {
                wrapper_option = Some("WITH CONDITIONAL ARRAY WRAPPER".to_string());
            } else if self.match_text_seq(&["WITH", "UNCONDITIONAL", "WRAPPER"]) {
                wrapper_option = Some("WITH UNCONDITIONAL WRAPPER".to_string());
            } else if self.match_text_seq(&["WITH", "CONDITIONAL", "WRAPPER"]) {
                wrapper_option = Some("WITH CONDITIONAL WRAPPER".to_string());
            } else if self.match_text_seq(&["WITH", "ARRAY", "WRAPPER"]) {
                wrapper_option = Some("WITH ARRAY WRAPPER".to_string());
            } else if self.match_text_seq(&["WITH", "WRAPPER"]) {
                wrapper_option = Some("WITH WRAPPER".to_string());
            } else if self.match_text_seq(&["WITHOUT", "CONDITIONAL", "ARRAY", "WRAPPER"]) {
                wrapper_option = Some("WITHOUT CONDITIONAL ARRAY WRAPPER".to_string());
            } else if self.match_text_seq(&["WITHOUT", "CONDITIONAL", "WRAPPER"]) {
                wrapper_option = Some("WITHOUT CONDITIONAL WRAPPER".to_string());
            } else if self.match_text_seq(&["WITHOUT", "ARRAY", "WRAPPER"]) {
                wrapper_option = Some("WITHOUT ARRAY WRAPPER".to_string());
            } else if self.match_text_seq(&["WITHOUT", "WRAPPER"]) {
                wrapper_option = Some("WITHOUT WRAPPER".to_string());
            } else if self.match_text_seq(&["KEEP", "QUOTES"]) {
                quotes_option = Some("KEEP QUOTES".to_string());
            } else if self.match_text_seq(&["OMIT", "QUOTES", "ON", "SCALAR", "STRING"]) {
                quotes_option = Some("OMIT QUOTES".to_string());
                on_scalar_string = true;
            } else if self.match_text_seq(&["OMIT", "QUOTES"]) {
                quotes_option = Some("OMIT QUOTES".to_string());
            } else if self.match_text_seq(&["NULL", "ON", "ERROR"]) {
                on_error = Some("NULL ON ERROR".to_string());
            } else if self.match_text_seq(&["ERROR", "ON", "ERROR"]) {
                on_error = Some("ERROR ON ERROR".to_string());
            } else if self.match_text_seq(&["EMPTY", "ON", "ERROR"]) {
                on_error = Some("EMPTY ON ERROR".to_string());
            } else if self.match_token(TokenType::Returning) {
                returning = Some(self.parse_data_type()?);
            } else {
                break;
            }
        }

        self.expect_token(TokenType::RParen)?;
        let func = JsonExtractFunc {
            options: None,
            source_dialect: None,

            this,
            path,
            returning,
            arrow_syntax: false,
            hash_arrow_syntax: false,
            wrapper_option,
            quotes_option,
            on_scalar_string,
            on_error,
        };
        Ok(match upper_name {
            "JSON_EXTRACT" => Expression::JsonExtract(Box::new(func)),
            "JSON_EXTRACT_SCALAR" => Expression::JsonExtractScalar(Box::new(func)),
            "JSON_QUERY" => Expression::JsonQuery(Box::new(func)),
            "JSON_VALUE" => Expression::JsonValue(Box::new(func)),
            _ => {
                return Err(crate::tokens::errors::Error::internal(
                    "JSON function name already matched in caller",
                ))
            }
        })
    }

    fn parse_typed_json_unary(
        &mut self,
        canonical_upper_name: &str,
        name: &str,
    ) -> Result<Expression> {
        let this = self.parse_expression()?;
        let this = self.maybe_clickhouse_alias(this);

        if self.match_token(TokenType::Comma) {
            let mut all_args = vec![this];
            let remaining = self.parse_function_arguments()?;
            all_args.extend(remaining);
            self.expect_token(TokenType::RParen)?;
            Ok(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: name.to_string(),
                args: all_args,
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: SourceSpan::default(),
                inferred_type: None,
                export_state: false,
            })))
        } else {
            self.expect_token(TokenType::RParen)?;
            let func = new_unary_func(this);
            Ok(match canonical_upper_name {
                "JSON_ARRAY_LENGTH" => Expression::JsonArrayLength(Box::new(func)),
                "JSON_KEYS" => Expression::JsonKeys(Box::new(func)),
                "JSON_TYPE" => Expression::JsonType(Box::new(func)),
                "TO_JSON" => Expression::ToJson(Box::new(func)),
                "PARSE_JSON" => Expression::ParseJson(Box::new(func)),
                _ => {
                    return Err(crate::tokens::errors::Error::internal(
                        "JSON function name already matched in caller",
                    ))
                }
            })
        }
    }

    fn parse_typed_json_object(&mut self) -> Result<Expression> {
        let mut pairs: Vec<(Expression, Expression)> = Vec::new();
        let mut star = false;
        if !self.check(TokenType::RParen) {
            if self.check(TokenType::Star) && self.check_next(TokenType::RParen) {
                self.skip();
                star = true;
            } else {
                loop {
                    let has_key_keyword = self.match_token(TokenType::Key);
                    let key = if let Some(s) = self.parse_string()? {
                        s
                    } else {
                        self.parse_primary()?
                    };

                    let has_separator = self.match_token(TokenType::Colon)
                        || self.match_identifier("VALUE")
                        || has_key_keyword && self.match_token(TokenType::Is);

                    if has_separator {
                        let value = self.parse_bitwise()?.ok_or_else(|| {
                            self.parse_error("Expected value expression in JSON_OBJECT")
                        })?;
                        let value_with_format = if self.match_text_seq(&["FORMAT", "JSON"]) {
                            Expression::JSONFormat(Box::new(JSONFormat {
                                this: Some(Box::new(value)),
                                options: Vec::new(),
                                is_json: None,
                                to_json: None,
                            }))
                        } else {
                            value
                        };
                        pairs.push((key, value_with_format));
                    } else {
                        if self.match_token(TokenType::Comma) {
                            let value = self.parse_bitwise()?.ok_or_else(|| {
                                self.parse_error("Expected value expression in JSON_OBJECT")
                            })?;
                            pairs.push((key, value));
                        } else {
                            return Err(
                                self.parse_error("Expected value expression in JSON_OBJECT")
                            );
                        }
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
        }
        let null_handling = if self.match_token(TokenType::Null) {
            self.match_token(TokenType::On);
            self.match_token(TokenType::Null);
            Some(JsonNullHandling::NullOnNull)
        } else if self.match_identifier("ABSENT") {
            self.match_token(TokenType::On);
            self.match_token(TokenType::Null);
            Some(JsonNullHandling::AbsentOnNull)
        } else {
            None
        };
        let with_unique_keys = if self.match_token(TokenType::With) {
            self.match_token(TokenType::Unique);
            self.match_identifier("KEYS");
            true
        } else {
            false
        };
        let (returning_type, format_json, encoding) = if self.match_token(TokenType::Returning) {
            let return_type = self.parse_data_type()?;
            let has_format_json = if self.match_token(TokenType::Format) {
                let _ = self.match_token(TokenType::Json) || self.match_identifier("JSON");
                true
            } else {
                false
            };
            let enc = if self.match_identifier("ENCODING") {
                Some(self.expect_identifier_or_keyword()?)
            } else {
                None
            };
            (Some(return_type), has_format_json, enc)
        } else {
            (None, false, None)
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::JsonObject(Box::new(JsonObjectFunc {
            pairs,
            null_handling,
            with_unique_keys,
            returning_type,
            format_json,
            encoding,
            star,
        })))
    }

    fn parse_typed_json_array(&mut self) -> Result<Expression> {
        let mut expressions: Vec<Expression> = Vec::new();
        if !self.check(TokenType::RParen) {
            loop {
                let expr = self.parse_bitwise()?.unwrap_or(Expression::Null(Null));
                let expr_with_format = if self.match_text_seq(&["FORMAT", "JSON"]) {
                    Expression::JSONFormat(Box::new(JSONFormat {
                        this: Some(Box::new(expr)),
                        options: Vec::new(),
                        is_json: None,
                        to_json: None,
                    }))
                } else {
                    expr
                };
                expressions.push(expr_with_format);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        let null_handling = if self.match_text_seq(&["NULL", "ON", "NULL"]) {
            Some(Box::new(Expression::Var(Box::new(Var {
                this: "NULL ON NULL".to_string(),
            }))))
        } else if self.match_text_seq(&["ABSENT", "ON", "NULL"]) {
            Some(Box::new(Expression::Var(Box::new(Var {
                this: "ABSENT ON NULL".to_string(),
            }))))
        } else {
            None
        };
        let return_type = if self.match_token(TokenType::Returning) {
            let dt = self.parse_data_type()?;
            Some(Box::new(Expression::DataType(dt)))
        } else {
            None
        };
        let strict = if self.match_identifier("STRICT") {
            Some(Box::new(Expression::Boolean(BooleanLiteral {
                value: true,
            })))
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::JSONArray(Box::new(JSONArray {
            expressions,
            null_handling,
            return_type,
            strict,
        })))
    }

    fn parse_typed_json_arrayagg(&mut self) -> Result<Expression> {
        let this = self.parse_bitwise()?.unwrap_or(Expression::Null(Null));
        let this_with_format = if self.match_text_seq(&["FORMAT", "JSON"]) {
            Expression::JSONFormat(Box::new(JSONFormat {
                this: Some(Box::new(this)),
                options: Vec::new(),
                is_json: None,
                to_json: None,
            }))
        } else {
            this
        };
        let order = if self.match_token(TokenType::Order) {
            self.match_token(TokenType::By);
            let mut order_exprs: Vec<Ordered> = Vec::new();
            loop {
                if let Some(ordered) = self.parse_ordered_item()? {
                    order_exprs.push(ordered);
                } else {
                    break;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            if !order_exprs.is_empty() {
                Some(Box::new(Expression::OrderBy(Box::new(OrderBy {
                    expressions: order_exprs,
                    siblings: false,
                    comments: Vec::new(),
                    all: None,
                }))))
            } else {
                None
            }
        } else {
            None
        };
        let null_handling = if self.match_text_seq(&["NULL", "ON", "NULL"]) {
            Some(Box::new(Expression::Var(Box::new(Var {
                this: "NULL ON NULL".to_string(),
            }))))
        } else if self.match_text_seq(&["ABSENT", "ON", "NULL"]) {
            Some(Box::new(Expression::Var(Box::new(Var {
                this: "ABSENT ON NULL".to_string(),
            }))))
        } else {
            None
        };
        let return_type = if self.match_token(TokenType::Returning) {
            let dt = self.parse_data_type()?;
            Some(Box::new(Expression::DataType(dt)))
        } else {
            None
        };
        let strict = if self.match_identifier("STRICT") {
            Some(Box::new(Expression::Boolean(BooleanLiteral {
                value: true,
            })))
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::JSONArrayAgg(Box::new(JSONArrayAgg {
            this: Box::new(this_with_format),
            order,
            null_handling,
            return_type,
            strict,
        })))
    }

    fn parse_typed_json_objectagg(&mut self) -> Result<Expression> {
        let _has_key_keyword = self.match_token(TokenType::Key);
        let key = self.parse_column()?.unwrap_or(Expression::Null(Null));

        let _ = self.match_token(TokenType::Colon)
            || self.match_token(TokenType::Comma)
            || self.match_identifier("VALUE");

        let value = self.parse_bitwise()?.unwrap_or(Expression::Null(Null));
        let value_with_format = if self.match_text_seq(&["FORMAT", "JSON"]) {
            Expression::JSONFormat(Box::new(JSONFormat {
                this: Some(Box::new(value)),
                options: Vec::new(),
                is_json: None,
                to_json: None,
            }))
        } else {
            value
        };
        let null_handling = if self.match_text_seq(&["NULL", "ON", "NULL"]) {
            Some(Box::new(Expression::Var(Box::new(Var {
                this: "NULL ON NULL".to_string(),
            }))))
        } else if self.match_text_seq(&["ABSENT", "ON", "NULL"]) {
            Some(Box::new(Expression::Var(Box::new(Var {
                this: "ABSENT ON NULL".to_string(),
            }))))
        } else {
            None
        };
        let unique_keys = if self.match_text_seq(&["WITH", "UNIQUE"]) {
            self.match_identifier("KEYS");
            Some(Box::new(Expression::Boolean(BooleanLiteral {
                value: true,
            })))
        } else if self.match_text_seq(&["WITHOUT", "UNIQUE"]) {
            self.match_identifier("KEYS");
            Some(Box::new(Expression::Boolean(BooleanLiteral {
                value: false,
            })))
        } else {
            None
        };
        let return_type = if self.match_token(TokenType::Returning) {
            let dt = self.parse_data_type()?;
            Some(Box::new(Expression::DataType(dt)))
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::JSONObjectAgg(Box::new(JSONObjectAgg {
            expressions: vec![Expression::JSONKeyValue(Box::new(JSONKeyValue {
                this: Box::new(key),
                expression: Box::new(value_with_format),
            }))],
            null_handling,
            unique_keys,
            return_type,
            encoding: None,
        })))
    }

    fn parse_typed_json_table(&mut self) -> Result<Expression> {
        let this = self.parse_bitwise()?.unwrap_or(Expression::Null(Null));
        let this_with_format = if self.match_text_seq(&["FORMAT", "JSON"]) {
            Expression::JSONFormat(Box::new(JSONFormat {
                this: Some(Box::new(this)),
                options: Vec::new(),
                is_json: None,
                to_json: None,
            }))
        } else {
            this
        };

        let path = if self.match_token(TokenType::Comma) {
            self.parse_string()?.map(Box::new)
        } else {
            None
        };

        let error_handling =
            if self.match_identifier("ERROR") && self.match_text_seq(&["ON", "ERROR"]) {
                Some(Box::new(Expression::Var(Box::new(Var {
                    this: "ERROR ON ERROR".to_string(),
                }))))
            } else if self.match_text_seq(&["NULL", "ON", "ERROR"]) {
                Some(Box::new(Expression::Var(Box::new(Var {
                    this: "NULL ON ERROR".to_string(),
                }))))
            } else {
                None
            };

        let empty_handling =
            if self.match_identifier("ERROR") && self.match_text_seq(&["ON", "EMPTY"]) {
                Some(Box::new(Expression::Var(Box::new(Var {
                    this: "ERROR ON EMPTY".to_string(),
                }))))
            } else if self.match_text_seq(&["NULL", "ON", "EMPTY"]) {
                Some(Box::new(Expression::Var(Box::new(Var {
                    this: "NULL ON EMPTY".to_string(),
                }))))
            } else {
                None
            };

        let schema = self.parse_json_table_columns()?;

        self.expect_token(TokenType::RParen)?;

        Ok(Expression::JSONTable(Box::new(JSONTable {
            options: None,
            source_dialect: None,

            this: Box::new(this_with_format),
            schema: schema.map(Box::new),
            path,
            error_handling,
            empty_handling,
        })))
    }
}
