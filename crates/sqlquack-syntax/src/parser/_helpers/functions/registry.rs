//! Parser functions: registry.

use crate::expressions::functions::models::AggregateFunction;
use crate::expressions::functions::models::CurrentSchemas;
use crate::expressions::functions::models::Function;
use crate::expressions::operators::models::Cast;
use crate::expressions::tree::models::Expression;
use crate::expressions::typed_functions::main::new_unary_func::new_unary_func;
use crate::expressions::typed_functions::models::AggFunc;
use crate::expressions::typed_functions::models::BinaryFunc;
use crate::expressions::typed_functions::models::LastDayFunc;
use crate::expressions::typed_functions::models::Normalize;
use crate::expressions::typed_functions::models::Pi;
use crate::expressions::typed_functions::models::Rand;
use crate::expressions::typed_functions::models::VarArgFunc;
use crate::functions::typed::models::TypedFunctionSpec;
use crate::parser::constants::BINARY_ARITY;
use crate::parser::constants::TERNARY_ARITY;
use crate::parser::models::FunctionCallNames;
use crate::parser::models::Parser;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

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
            crate::functions::typed::main::typed_function_spec_by_canonical_upper::typed_function_spec_by_canonical_upper(canonical_upper_name)
        else {
            return Ok(None);
        };

        match (spec.parse_kind, spec.canonical_name) {
            (crate::functions::typed::models::TypedParseKind::AggregateLike, "COUNT_IF") => {
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
            (crate::functions::typed::models::TypedParseKind::Binary, "STARTS_WITH")
            | (crate::functions::typed::models::TypedParseKind::Binary, "ENDS_WITH") => {
                self.parse_registry_starts_with(spec)
            }
            (crate::functions::typed::models::TypedParseKind::Binary, "ATAN2") => {
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
            (crate::functions::typed::models::TypedParseKind::Binary, "MAP_FROM_ARRAYS")
            | (crate::functions::typed::models::TypedParseKind::Binary, "MAP_CONTAINS_KEY")
            | (crate::functions::typed::models::TypedParseKind::Binary, "ELEMENT_AT") => {
                self.parse_registry_map_from_arrays(spec)
            }
            (crate::functions::typed::models::TypedParseKind::Binary, "CONTAINS")
            | (crate::functions::typed::models::TypedParseKind::Binary, "MOD")
            | (crate::functions::typed::models::TypedParseKind::Binary, "POW") => {
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
                        return Err(crate::tokens::errors::Error::internal(
                            "binary scalar parse kind already matched in caller",
                        ))
                    }
                };
                Ok(Some(expr))
            }
            (crate::functions::typed::models::TypedParseKind::Binary, "ADD_MONTHS")
            | (crate::functions::typed::models::TypedParseKind::Binary, "MONTHS_BETWEEN")
            | (crate::functions::typed::models::TypedParseKind::Binary, "NEXT_DAY") => {
                self.parse_registry_add_months(spec)
            }
            (crate::functions::typed::models::TypedParseKind::Binary, "ARRAY_CONTAINS")
            | (crate::functions::typed::models::TypedParseKind::Binary, "ARRAY_POSITION")
            | (crate::functions::typed::models::TypedParseKind::Binary, "ARRAY_APPEND")
            | (crate::functions::typed::models::TypedParseKind::Binary, "ARRAY_PREPEND")
            | (crate::functions::typed::models::TypedParseKind::Binary, "ARRAY_UNION")
            | (crate::functions::typed::models::TypedParseKind::Binary, "ARRAY_EXCEPT")
            | (crate::functions::typed::models::TypedParseKind::Binary, "ARRAY_REMOVE") => {
                self.parse_registry_array_contains(spec)
            }
            (crate::functions::typed::models::TypedParseKind::Unary, "LENGTH") => {
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
            (crate::functions::typed::models::TypedParseKind::Unary, "LOWER") => {
                let this = self.parse_expression_with_clickhouse_alias()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Lower(Box::new(new_unary_func(this)))))
            }
            (crate::functions::typed::models::TypedParseKind::Unary, "UPPER") => {
                let this = self.parse_expression_with_clickhouse_alias()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Upper(Box::new(new_unary_func(this)))))
            }
            (crate::functions::typed::models::TypedParseKind::Unary, "TYPEOF") => {
                self.parse_registry_typeof(name)
            }
            (crate::functions::typed::models::TypedParseKind::Unary, "DAYOFWEEK")
            | (crate::functions::typed::models::TypedParseKind::Unary, "DAYOFYEAR")
            | (crate::functions::typed::models::TypedParseKind::Unary, "DAYOFMONTH")
            | (crate::functions::typed::models::TypedParseKind::Unary, "WEEKOFYEAR") => {
                let this = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                let func = new_unary_func(this);
                let expr = match spec.canonical_name {
                    "DAYOFWEEK" => Expression::DayOfWeek(Box::new(func)),
                    "DAYOFYEAR" => Expression::DayOfYear(Box::new(func)),
                    "DAYOFMONTH" => Expression::DayOfMonth(Box::new(func)),
                    "WEEKOFYEAR" => Expression::WeekOfYear(Box::new(func)),
                    _ => {
                        return Err(crate::tokens::errors::Error::internal(
                            "date-part unary parse kind already matched in caller",
                        ))
                    }
                };
                Ok(Some(expr))
            }
            (crate::functions::typed::models::TypedParseKind::Unary, "SIN")
            | (crate::functions::typed::models::TypedParseKind::Unary, "COS")
            | (crate::functions::typed::models::TypedParseKind::Unary, "TAN")
            | (crate::functions::typed::models::TypedParseKind::Unary, "ASIN")
            | (crate::functions::typed::models::TypedParseKind::Unary, "ACOS")
            | (crate::functions::typed::models::TypedParseKind::Unary, "ATAN")
            | (crate::functions::typed::models::TypedParseKind::Unary, "RADIANS")
            | (crate::functions::typed::models::TypedParseKind::Unary, "DEGREES") => {
                self.parse_registry_sin(spec)
            }
            (crate::functions::typed::models::TypedParseKind::Unary, "YEAR")
            | (crate::functions::typed::models::TypedParseKind::Unary, "MONTH")
            | (crate::functions::typed::models::TypedParseKind::Unary, "DAY")
            | (crate::functions::typed::models::TypedParseKind::Unary, "HOUR")
            | (crate::functions::typed::models::TypedParseKind::Unary, "MINUTE")
            | (crate::functions::typed::models::TypedParseKind::Unary, "SECOND")
            | (crate::functions::typed::models::TypedParseKind::Unary, "DAYOFWEEK_ISO")
            | (crate::functions::typed::models::TypedParseKind::Unary, "QUARTER")
            | (crate::functions::typed::models::TypedParseKind::Unary, "EPOCH")
            | (crate::functions::typed::models::TypedParseKind::Unary, "EPOCH_MS") => {
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
                        return Err(crate::tokens::errors::Error::internal(
                            "date unary parse kind already matched in caller",
                        ))
                    }
                };
                Ok(Some(expr))
            }
            (crate::functions::typed::models::TypedParseKind::Unary, "ARRAY_LENGTH")
            | (crate::functions::typed::models::TypedParseKind::Unary, "ARRAY_SIZE")
            | (crate::functions::typed::models::TypedParseKind::Unary, "CARDINALITY")
            | (crate::functions::typed::models::TypedParseKind::Unary, "ARRAY_REVERSE")
            | (crate::functions::typed::models::TypedParseKind::Unary, "ARRAY_DISTINCT")
            | (crate::functions::typed::models::TypedParseKind::Unary, "ARRAY_COMPACT")
            | (crate::functions::typed::models::TypedParseKind::Unary, "EXPLODE")
            | (crate::functions::typed::models::TypedParseKind::Unary, "EXPLODE_OUTER") => {
                self.parse_registry_array_length(name, spec)
            }
            (crate::functions::typed::models::TypedParseKind::Unary, "MAP_FROM_ENTRIES")
            | (crate::functions::typed::models::TypedParseKind::Unary, "MAP_KEYS")
            | (crate::functions::typed::models::TypedParseKind::Unary, "MAP_VALUES") => {
                let this = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                let func = new_unary_func(this);
                let expr = match spec.canonical_name {
                    "MAP_FROM_ENTRIES" => Expression::MapFromEntries(Box::new(func)),
                    "MAP_KEYS" => Expression::MapKeys(Box::new(func)),
                    "MAP_VALUES" => Expression::MapValues(Box::new(func)),
                    _ => {
                        return Err(crate::tokens::errors::Error::internal(
                            "map unary parse kind already matched in caller",
                        ))
                    }
                };
                Ok(Some(expr))
            }
            (crate::functions::typed::models::TypedParseKind::Unary, "ABS") => {
                let this = self.parse_expression_with_clickhouse_alias()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Abs(Box::new(new_unary_func(this)))))
            }
            (crate::functions::typed::models::TypedParseKind::Unary, "SQRT")
            | (crate::functions::typed::models::TypedParseKind::Unary, "EXP")
            | (crate::functions::typed::models::TypedParseKind::Unary, "LN") => {
                let this = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                let expr = match spec.canonical_name {
                    "SQRT" => Expression::Sqrt(Box::new(new_unary_func(this))),
                    "EXP" => Expression::Exp(Box::new(new_unary_func(this))),
                    "LN" => Expression::Ln(Box::new(new_unary_func(this))),
                    _ => {
                        return Err(crate::tokens::errors::Error::internal(
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
            (crate::functions::typed::models::TypedParseKind::Variadic, "TO_NUMBER")
            | (crate::functions::typed::models::TypedParseKind::Variadic, "TRY_TO_NUMBER") => {
                self.parse_registry_to_number(spec)
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "SUBSTRING") => {
                self.parse_registry_substring(name)
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "DATE_PART") => {
                self.parse_registry_date_part()
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "DATEADD") => {
                self.parse_registry_dateadd(name)
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "DATEDIFF") => {
                self.parse_registry_datediff(name)
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "RANDOM") => {
                self.parse_registry_random()
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "RAND") => {
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
            (crate::functions::typed::models::TypedParseKind::Variadic, "PI") => {
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Pi(Pi)))
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "LAST_DAY") => {
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
            (crate::functions::typed::models::TypedParseKind::Variadic, "POSITION") => {
                let expr = self
                    .parse_position()?
                    .ok_or_else(|| self.parse_error("Expected expression in POSITION"))?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(expr))
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "STRPOS") => {
                self.parse_registry_strpos()
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "LOCATE") => {
                self.parse_registry_locate(name)
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "INSTR") => {
                self.parse_registry_instr()
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "NORMALIZE") => {
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
            (crate::functions::typed::models::TypedParseKind::Variadic, "INITCAP") => {
                self.parse_registry_initcap()
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "FLOOR") => {
                self.parse_registry_floor(name)
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "LOG") => {
                self.parse_registry_log()
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "FLATTEN") => {
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
            (crate::functions::typed::models::TypedParseKind::Variadic, "ARRAY_INTERSECT") => {
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
            (crate::functions::typed::models::TypedParseKind::Variadic, "CURRENT_SCHEMAS") => {
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
            (crate::functions::typed::models::TypedParseKind::Variadic, "COALESCE") => {
                let args = if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    self.parse_expression_list()?
                };
                self.expect_token(TokenType::RParen)?;
                let original_name = { None };
                Ok(Some(Expression::Coalesce(Box::new(
                    crate::expressions::typed_functions::models::VarArgFunc {
                        original_name,
                        expressions: args,
                        inferred_type: None,
                    },
                ))))
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "IFNULL") => {
                let args = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                if args.len() >= BINARY_ARITY {
                    let original_name = { "IFNULL".to_string() };
                    Ok(Some(Expression::Coalesce(Box::new(
                        crate::expressions::typed_functions::models::VarArgFunc {
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
            (crate::functions::typed::models::TypedParseKind::Variadic, "NVL") => {
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
                        crate::expressions::typed_functions::models::BinaryFunc {
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
            (crate::functions::typed::models::TypedParseKind::Variadic, "NVL2") => {
                let args = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                if args.len() >= TERNARY_ARITY {
                    Ok(Some(Expression::Nvl2(Box::new(
                        crate::expressions::typed_functions::models::Nvl2Func {
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
            (crate::functions::typed::models::TypedParseKind::Variadic, "EXTRACT") => {
                self.parse_registry_extract(name)
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "STRUCT") => {
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
            (crate::functions::typed::models::TypedParseKind::Variadic, "CHAR") => {
                self.parse_registry_char(name)
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "CHR") => {
                self.parse_registry_chr(name)
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "RANGE_N") => {
                self.parse_registry_range_n()
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "XMLTABLE") => {
                if let Some(xml_table) = self.parse_xml_table()? {
                    self.expect_token(TokenType::RParen)?;
                    Ok(Some(xml_table))
                } else {
                    Err(self.parse_error("Failed to parse XMLTABLE"))
                }
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "XMLELEMENT") => {
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
            (crate::functions::typed::models::TypedParseKind::Variadic, "XMLATTRIBUTES") => {
                self.parse_registry_xmlattributes()
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "XMLCOMMENT") => {
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
            (crate::functions::typed::models::TypedParseKind::Variadic, "MATCH") => {
                self.parse_registry_match()
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "TRANSFORM") => {
                self.parse_registry_transform(name, quoted)
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "CONVERT") => {
                self.parse_registry_convert(upper_name)
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "TRIM") => {
                self.parse_registry_trim()
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "OVERLAY") => {
                self.parse_registry_overlay(name)
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "CEIL") => {
                self.parse_registry_ceil()
            }
            (crate::functions::typed::models::TypedParseKind::Variadic, "TIMESTAMP_FROM_PARTS")
            | (
                crate::functions::typed::models::TypedParseKind::Variadic,
                "TIMESTAMP_NTZ_FROM_PARTS",
            )
            | (
                crate::functions::typed::models::TypedParseKind::Variadic,
                "TIMESTAMP_LTZ_FROM_PARTS",
            )
            | (
                crate::functions::typed::models::TypedParseKind::Variadic,
                "TIMESTAMP_TZ_FROM_PARTS",
            )
            | (crate::functions::typed::models::TypedParseKind::Variadic, "DATE_FROM_PARTS")
            | (crate::functions::typed::models::TypedParseKind::Variadic, "TIME_FROM_PARTS") => {
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
            (crate::functions::typed::models::TypedParseKind::CastLike, "TRY_CAST") => {
                let this = self.parse_expression()?;
                self.expect_token(TokenType::As)?;
                let to = self.parse_data_type()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::TryCast(Box::new(Cast {
                    this,
                    to,
                    trailing_comments: Vec::new(),
                    double_colon_syntax: false,
                    format: None,
                    default: None,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))))
            }
            (crate::functions::typed::models::TypedParseKind::Conditional, "IF") => {
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
        use crate::functions::typed::models::TypedDispatchGroup;

        match crate::functions::typed::main::typed_dispatch_group_by_name_upper::typed_dispatch_group_by_name_upper(canonical_upper_name) {
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
