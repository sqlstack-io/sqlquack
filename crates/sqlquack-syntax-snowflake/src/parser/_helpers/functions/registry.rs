//! Parser functions: registry.

use crate::parser::constants::BINARY_ARITY;
use crate::parser::constants::TERNARY_ARITY;
use crate::parser::models::FunctionCallNames;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::functions::models::AggregateFunction;
use sqlquack_syntax::expressions::functions::models::CurrentSchemas;
use sqlquack_syntax::expressions::functions::models::Function;
use sqlquack_syntax::expressions::operators::models::Cast;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::typed_functions::main::new_unary_func::new_unary_func;
use sqlquack_syntax::expressions::typed_functions::models::AggFunc;
use sqlquack_syntax::expressions::typed_functions::models::BinaryFunc;
use sqlquack_syntax::expressions::typed_functions::models::LastDayFunc;
use sqlquack_syntax::expressions::typed_functions::models::Normalize;
use sqlquack_syntax::expressions::typed_functions::models::Pi;
use sqlquack_syntax::expressions::typed_functions::models::Rand;
use sqlquack_syntax::expressions::typed_functions::models::VarArgFunc;
use sqlquack_syntax::functions::typed::models::TypedFunctionSpec;
use sqlquack_syntax::tokens::models::SourceSpan;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Parse the subset of typed functions that are handled via function-registry metadata.
    pub(crate) fn try_parse_registry_typed_function(
        &mut self,
        name: &str,
        upper_name: &str,
        canonical_upper_name: &str,
        quoted: bool,
    ) -> Result<Option<Expression>> {
        let Some(spec) =
            sqlquack_syntax::functions::typed::main::typed_function_spec_by_canonical_upper::typed_function_spec_by_canonical_upper(canonical_upper_name)
        else {
            return Ok(None);
        };

        match (spec.parse_kind, spec.canonical_name) {
            (
                sqlquack_syntax::functions::typed::models::TypedParseKind::AggregateLike,
                "COUNT_IF",
            ) => {
                let distinct = self.match_token(TokenType::Distinct);
                let this = self.parse_expression()?;
                let this = { this };

                self.expect_token(TokenType::RParen)?;
                let filter = self.parse_filter_clause()?;
                Ok(Some(Expression::CountIf(Box::new(AggFunc {
                    ignore_nulls: None,
                    this,
                    distinct,
                    filter,
                    order_by: Vec::new(),
                    having_max: None,
                    name: Some(name.to_string()),
                    limit: None,
                    inferred_type: None,
                }))))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Binary, "STARTS_WITH")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Binary, "ENDS_WITH") => {
                self.parse_registry_starts_with(spec)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Binary, "ATAN2") => {
                let this = self.parse_expression()?;
                self.expect_token(TokenType::Comma)?;
                let expression = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Atan2(Box::new(BinaryFunc {
                    original_name: None,
                    this,
                    expression,
                    inferred_type: None,
                }))))
            }
            (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Binary,
                "MAP_FROM_ARRAYS",
            )
            | (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Binary,
                "MAP_CONTAINS_KEY",
            )
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Binary, "ELEMENT_AT") => {
                self.parse_registry_map_from_arrays(spec)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Binary, "CONTAINS")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Binary, "MOD")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Binary, "POW") => {
                let this = self.parse_expression()?;
                self.expect_token(TokenType::Comma)?;
                let expression = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                let expr = match spec.canonical_name {
                    "CONTAINS" => Expression::Contains(Box::new(BinaryFunc {
                        original_name: None,
                        this,
                        expression,
                        inferred_type: None,
                    })),
                    "MOD" => Expression::ModFunc(Box::new(BinaryFunc {
                        original_name: None,
                        this,
                        expression,
                        inferred_type: None,
                    })),
                    "POW" => Expression::Power(Box::new(BinaryFunc {
                        original_name: None,
                        this,
                        expression,
                        inferred_type: None,
                    })),
                    _ => {
                        return Err(sqlquack_syntax::tokens::errors::Error::internal(
                            "binary scalar parse kind already matched in caller",
                        ))
                    }
                };
                Ok(Some(expr))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Binary, "ADD_MONTHS")
            | (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Binary,
                "MONTHS_BETWEEN",
            )
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Binary, "NEXT_DAY") => {
                self.parse_registry_add_months(spec)
            }
            (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Binary,
                "ARRAY_CONTAINS",
            )
            | (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Binary,
                "ARRAY_POSITION",
            )
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Binary, "ARRAY_APPEND")
            | (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Binary,
                "ARRAY_PREPEND",
            )
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Binary, "ARRAY_UNION")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Binary, "ARRAY_EXCEPT")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Binary, "ARRAY_REMOVE") => {
                self.parse_registry_array_contains(spec)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "LENGTH") => {
                let this = self.parse_expression()?;
                if self.match_token(TokenType::Comma) {
                    let encoding = self.parse_expression()?;
                    self.expect_token(TokenType::RParen)?;
                    Ok(Some(Expression::Function(Box::new(Function::new(
                        upper_name,
                        vec![this, encoding],
                    )))))
                } else {
                    self.expect_token(TokenType::RParen)?;
                    Ok(Some(Expression::Length(Box::new(new_unary_func(this)))))
                }
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "LOWER") => {
                let this = self.parse_expression_with_clickhouse_alias()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Lower(Box::new(new_unary_func(this)))))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "UPPER") => {
                let this = self.parse_expression_with_clickhouse_alias()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Upper(Box::new(new_unary_func(this)))))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "TYPEOF") => {
                self.parse_registry_typeof(name)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "DAYOFWEEK")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "DAYOFYEAR")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "DAYOFMONTH")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "WEEKOFYEAR") => {
                let this = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                let func = new_unary_func(this);
                let expr = match spec.canonical_name {
                    "DAYOFWEEK" => Expression::DayOfWeek(Box::new(func)),
                    "DAYOFYEAR" => Expression::DayOfYear(Box::new(func)),
                    "DAYOFMONTH" => Expression::DayOfMonth(Box::new(func)),
                    "WEEKOFYEAR" => Expression::WeekOfYear(Box::new(func)),
                    _ => {
                        return Err(sqlquack_syntax::tokens::errors::Error::internal(
                            "date-part unary parse kind already matched in caller",
                        ))
                    }
                };
                Ok(Some(expr))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "SIN")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "COS")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "TAN")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "ASIN")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "ACOS")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "ATAN")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "RADIANS")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "DEGREES") => {
                self.parse_registry_sin(spec)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "YEAR")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "MONTH")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "DAY")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "HOUR")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "MINUTE")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "SECOND")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "DAYOFWEEK_ISO")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "QUARTER")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "EPOCH")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "EPOCH_MS") => {
                let this = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                let func = new_unary_func(this);
                let expr = match spec.canonical_name {
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
                    _ => {
                        return Err(sqlquack_syntax::tokens::errors::Error::internal(
                            "date unary parse kind already matched in caller",
                        ))
                    }
                };
                Ok(Some(expr))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "ARRAY_LENGTH")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "ARRAY_SIZE")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "CARDINALITY")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "ARRAY_REVERSE")
            | (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Unary,
                "ARRAY_DISTINCT",
            )
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "ARRAY_COMPACT")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "EXPLODE")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "EXPLODE_OUTER") => {
                self.parse_registry_array_length(name, spec)
            }
            (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Unary,
                "MAP_FROM_ENTRIES",
            )
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "MAP_KEYS")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "MAP_VALUES") => {
                let this = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                let func = new_unary_func(this);
                let expr = match spec.canonical_name {
                    "MAP_FROM_ENTRIES" => Expression::MapFromEntries(Box::new(func)),
                    "MAP_KEYS" => Expression::MapKeys(Box::new(func)),
                    "MAP_VALUES" => Expression::MapValues(Box::new(func)),
                    _ => {
                        return Err(sqlquack_syntax::tokens::errors::Error::internal(
                            "map unary parse kind already matched in caller",
                        ))
                    }
                };
                Ok(Some(expr))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "ABS") => {
                let this = self.parse_expression_with_clickhouse_alias()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Abs(Box::new(new_unary_func(this)))))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "SQRT")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "EXP")
            | (sqlquack_syntax::functions::typed::models::TypedParseKind::Unary, "LN") => {
                let this = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                let expr = match spec.canonical_name {
                    "SQRT" => Expression::Sqrt(Box::new(new_unary_func(this))),
                    "EXP" => Expression::Exp(Box::new(new_unary_func(this))),
                    "LN" => Expression::Ln(Box::new(new_unary_func(this))),
                    _ => {
                        return Err(sqlquack_syntax::tokens::errors::Error::internal(
                            "math unary parse kind already matched in caller",
                        ))
                    }
                };
                Ok(Some(expr))
            }
            _ => self.try_parse_registry_variadic_function(
                FunctionCallNames {
                    name,
                    upper_name,
                    canonical_upper_name,
                },
                quoted,
                spec,
            ),
        }
    }

    /// Parse the first half of the variadic typed functions handled via function-registry metadata.
    fn try_parse_registry_variadic_function(
        &mut self,
        names: FunctionCallNames<'_>,
        quoted: bool,
        spec: &TypedFunctionSpec,
    ) -> Result<Option<Expression>> {
        let FunctionCallNames {
            name,
            upper_name,
            canonical_upper_name,
        } = names;
        match (spec.parse_kind, spec.canonical_name) {
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "TO_NUMBER")
            | (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic,
                "TRY_TO_NUMBER",
            ) => self.parse_registry_to_number(spec),
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "SUBSTRING") => {
                self.parse_registry_substring(name)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "DATE_PART") => {
                self.parse_registry_date_part()
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "DATEADD") => {
                self.parse_registry_dateadd(name)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "DATEDIFF") => {
                self.parse_registry_datediff(name)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "RANDOM") => {
                self.parse_registry_random()
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "RAND") => {
                let seed = if self.check(TokenType::RParen) {
                    None
                } else {
                    Some(Box::new(self.parse_expression()?))
                };
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Rand(Box::new(Rand {
                    seed,
                    lower: None,
                    upper: None,
                }))))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "PI") => {
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Pi(Pi)))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "LAST_DAY") => {
                let this = self.parse_expression()?;
                let unit = if self.match_token(TokenType::Comma) {
                    Some(self.parse_datetime_field()?)
                } else {
                    None
                };
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::LastDay(Box::new(LastDayFunc {
                    this,
                    unit,
                }))))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "POSITION") => {
                let expr = self
                    .parse_position()?
                    .ok_or_else(|| self.parse_error("Expected expression in POSITION"))?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(expr))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "STRPOS") => {
                self.parse_registry_strpos()
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "LOCATE") => {
                self.parse_registry_locate(name)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "INSTR") => {
                self.parse_registry_instr()
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "NORMALIZE") => {
                let this = self.parse_expression()?;
                let form = if self.match_token(TokenType::Comma) {
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                };
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Normalize(Box::new(Normalize {
                    this: Box::new(this),
                    form,
                    is_casefold: None,
                }))))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "INITCAP") => {
                self.parse_registry_initcap()
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "FLOOR") => {
                self.parse_registry_floor(name)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "LOG") => {
                self.parse_registry_log()
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "FLATTEN") => {
                let args = match self.check(TokenType::RParen) {
                    true => Vec::new(),
                    false => self.parse_function_arguments()?,
                };
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Function(Box::new(Function {
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
                }))))
            }
            (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic,
                "ARRAY_INTERSECT",
            ) => {
                let mut expressions = vec![self.parse_expression()?];
                while self.match_token(TokenType::Comma) {
                    expressions.push(self.parse_expression()?);
                }
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::ArrayIntersect(Box::new(VarArgFunc {
                    expressions,
                    original_name: Some(name.to_string()),
                    inferred_type: None,
                }))))
            }
            (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic,
                "CURRENT_SCHEMAS",
            ) => {
                let args = if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    vec![self.parse_expression()?]
                };
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::CurrentSchemas(Box::new(CurrentSchemas {
                    this: args.into_iter().next().map(Box::new),
                }))))
            }
            _ => self.try_parse_registry_late_variadic_function(
                FunctionCallNames {
                    name,
                    upper_name,
                    canonical_upper_name,
                },
                quoted,
                spec,
            ),
        }
    }

    /// Parse the remaining variadic, cast-like and conditional typed functions handled via function-registry metadata.
    fn try_parse_registry_late_variadic_function(
        &mut self,
        names: FunctionCallNames<'_>,
        quoted: bool,
        spec: &TypedFunctionSpec,
    ) -> Result<Option<Expression>> {
        let FunctionCallNames {
            name,
            upper_name,
            canonical_upper_name,
        } = names;
        match (spec.parse_kind, spec.canonical_name) {
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "COALESCE") => {
                let args = if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    self.parse_expression_list()?
                };
                self.expect_token(TokenType::RParen)?;
                let original_name = { None };
                Ok(Some(Expression::Coalesce(Box::new(
                    sqlquack_syntax::expressions::typed_functions::models::VarArgFunc {
                        original_name,
                        expressions: args,
                        inferred_type: None,
                    },
                ))))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "IFNULL") => {
                let args = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                if args.len() >= BINARY_ARITY {
                    let original_name = { "IFNULL".to_string() };
                    Ok(Some(Expression::Coalesce(Box::new(
                        sqlquack_syntax::expressions::typed_functions::models::VarArgFunc {
                            original_name: Some(original_name),
                            expressions: args,
                            inferred_type: None,
                        },
                    ))))
                } else {
                    Ok(Some(Expression::Function(Box::new(Function {
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
                    }))))
                }
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "NVL") => {
                let args = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                if args.len() > BINARY_ARITY {
                    Ok(Some(Expression::Function(Box::new(Function {
                        on_error: None,
                        qualified_name: Vec::new(),
                        source_dialect: None,

                        name: "COALESCE".to_string(),
                        args,
                        distinct: false,
                        trailing_comments: Vec::new(),
                        use_bracket_syntax: false,
                        no_parens: false,
                        quoted: false,
                        span: SourceSpan::default(),
                        inferred_type: None,
                        export_state: false,
                    }))))
                } else if args.len() == BINARY_ARITY {
                    Ok(Some(Expression::Nvl(Box::new(
                        sqlquack_syntax::expressions::typed_functions::models::BinaryFunc {
                            original_name: Some("NVL".to_string()),
                            this: args[0].clone(),
                            expression: args[1].clone(),
                            inferred_type: None,
                        },
                    ))))
                } else {
                    Ok(Some(Expression::Function(Box::new(Function {
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
                    }))))
                }
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "NVL2") => {
                let args = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                if args.len() >= TERNARY_ARITY {
                    Ok(Some(Expression::Nvl2(Box::new(
                        sqlquack_syntax::expressions::typed_functions::models::Nvl2Func {
                            this: args[0].clone(),
                            true_value: args[1].clone(),
                            false_value: args[2].clone(),
                            inferred_type: None,
                        },
                    ))))
                } else {
                    Ok(Some(Expression::Function(Box::new(Function {
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
                    }))))
                }
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "EXTRACT") => {
                self.parse_registry_extract(name)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "STRUCT") => {
                let args = if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    self.parse_struct_args()?
                };
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Function(Box::new(Function {
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
                }))))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "CHAR") => {
                self.parse_registry_char(name)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "CHR") => {
                self.parse_registry_chr(name)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "RANGE_N") => {
                self.parse_registry_range_n()
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "XMLTABLE") => {
                if let Some(xml_table) = self.parse_xml_table()? {
                    self.expect_token(TokenType::RParen)?;
                    Ok(Some(xml_table))
                } else {
                    Err(self.parse_error("Failed to parse XMLTABLE"))
                }
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "XMLELEMENT") => {
                if let Some(elem) = self.parse_xml_element()? {
                    self.expect_token(TokenType::RParen)?;
                    Ok(Some(elem))
                } else {
                    self.expect_token(TokenType::RParen)?;
                    Ok(Some(Expression::Function(Box::new(Function {
                        on_error: None,
                        qualified_name: Vec::new(),
                        source_dialect: None,

                        name: name.to_string(),
                        args: Vec::new(),
                        distinct: false,
                        trailing_comments: Vec::new(),
                        use_bracket_syntax: false,
                        no_parens: false,
                        quoted: false,
                        span: SourceSpan::default(),
                        inferred_type: None,
                        export_state: false,
                    }))))
                }
            }
            (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic,
                "XMLATTRIBUTES",
            ) => self.parse_registry_xmlattributes(),
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "XMLCOMMENT") => {
                let args = if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    self.parse_expression_list()?
                };
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Function(Box::new(Function {
                    on_error: None,
                    qualified_name: Vec::new(),
                    source_dialect: None,

                    name: "XMLCOMMENT".to_string(),
                    args,
                    distinct: false,
                    trailing_comments: Vec::new(),
                    use_bracket_syntax: false,
                    no_parens: false,
                    quoted: false,
                    span: SourceSpan::default(),
                    inferred_type: None,
                    export_state: false,
                }))))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "MATCH") => {
                self.parse_registry_match()
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "TRANSFORM") => {
                self.parse_registry_transform(name, quoted)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "CONVERT") => {
                self.parse_registry_convert(upper_name)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "TRIM") => {
                self.parse_registry_trim()
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "OVERLAY") => {
                self.parse_registry_overlay(name)
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic, "CEIL") => {
                self.parse_registry_ceil()
            }
            (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic,
                "TIMESTAMP_FROM_PARTS",
            )
            | (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic,
                "TIMESTAMP_NTZ_FROM_PARTS",
            )
            | (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic,
                "TIMESTAMP_LTZ_FROM_PARTS",
            )
            | (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic,
                "TIMESTAMP_TZ_FROM_PARTS",
            )
            | (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic,
                "DATE_FROM_PARTS",
            )
            | (
                sqlquack_syntax::functions::typed::models::TypedParseKind::Variadic,
                "TIME_FROM_PARTS",
            ) => {
                let args = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Function(Box::new(Function {
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
                }))))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::CastLike, "TRY_CAST") => {
                let this = self.parse_expression()?;
                self.expect_token(TokenType::As)?;
                let to = self.parse_data_type()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::TryCast(Box::new(Cast {
                    this,
                    to,
                    trailing_comments: Vec::new(),
                    double_colon_syntax: false,
                    typed_literal: false,
                    format: None,
                    default: None,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))))
            }
            (sqlquack_syntax::functions::typed::models::TypedParseKind::Conditional, "IF") => {
                self.parse_registry_if(name, upper_name)
            }
            _ => {
                self.try_parse_registry_grouped_typed_family(name, upper_name, canonical_upper_name)
            }
        }
    }

    /// Route heavy typed-function families via registry metadata groups.
    pub(crate) fn try_parse_registry_grouped_typed_family(
        &mut self,
        name: &str,
        upper_name: &str,
        canonical_upper_name: &str,
    ) -> Result<Option<Expression>> {
        use sqlquack_syntax::functions::typed::models::TypedDispatchGroup;

        match sqlquack_syntax::functions::typed::main::typed_dispatch_group_by_name_upper::typed_dispatch_group_by_name_upper(canonical_upper_name) {
            Some(TypedDispatchGroup::Aggregate) => self
                .parse_typed_aggregate_family(name, upper_name, canonical_upper_name)
                .map(Some),
            Some(TypedDispatchGroup::Window) => self
                .parse_typed_window_family(name, upper_name, canonical_upper_name)
                .map(Some),
            Some(TypedDispatchGroup::Json) => self
                .parse_typed_json_family(name, upper_name, canonical_upper_name)
                .map(Some),
            Some(TypedDispatchGroup::TranslateTeradata) => Ok(None),
            None => Ok(None),
        }
    }

    pub(crate) fn make_simple_aggregate(
        name: &str,
        args: Vec<Expression>,
        distinct: bool,
        filter: Option<Expression>,
    ) -> Expression {
        Expression::AggregateFunction(Box::new(AggregateFunction {
            source_dialect: None,

            name: name.to_string(),
            args,
            distinct,
            filter,
            order_by: Vec::new(),
            limit: None,
            ignore_nulls: None,
            inferred_type: None,
            span: SourceSpan::default(),
            export_state: false,
            qualified_name: Vec::new(),
            quoted: false,
        }))
    }
}
