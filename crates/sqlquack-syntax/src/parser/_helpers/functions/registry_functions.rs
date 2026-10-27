//! Parser functions: the individual typed functions the function registry dispatches to.

use crate::expressions::functions::models::Function;
use crate::expressions::functions::models::ToNumber;
use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Literal;
use crate::expressions::names::models::Null;
use crate::expressions::operators::models::Alias;
use crate::expressions::operators::models::Cast;
use crate::expressions::statements::models::Var;
use crate::expressions::tables::models::QueryTransform;
use crate::expressions::tree::models::Expression;
use crate::expressions::typed_functions::main::new_unary_func::new_unary_func;
use crate::expressions::typed_functions::models::BinaryFunc;
use crate::expressions::typed_functions::models::CeilFunc;
use crate::expressions::typed_functions::models::DateDiffFunc;
use crate::expressions::typed_functions::models::ExtractFunc;
use crate::expressions::typed_functions::models::FloorFunc;
use crate::expressions::typed_functions::models::LogFunc;
use crate::expressions::typed_functions::models::MatchAgainst;
use crate::expressions::typed_functions::models::OverlayFunc;
use crate::expressions::typed_functions::models::Rand;
use crate::expressions::typed_functions::models::Random;
use crate::expressions::typed_functions::models::RangeN;
use crate::expressions::typed_functions::models::StrPosition;
use crate::expressions::typed_functions::models::SubstringFunc;
use crate::expressions::typed_functions::models::TrimFunc;
use crate::expressions::typed_functions::models::TrimPosition;
use crate::expressions::types::models::DataType;
use crate::functions::typed::models::TypedFunctionSpec;
use crate::parser::constants::ARRAY_LENGTH;
use crate::parser::constants::ARRAY_SIZE;
use crate::parser::constants::ATAN;
use crate::parser::constants::BINARY_ARITY;
use crate::parser::constants::MONTHS_BETWEEN;
use crate::parser::constants::TERNARY_ARITY;
use crate::parser::constants::TRY_CONVERT;
use crate::parser::constants::TRY_TO_NUMBER;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    pub(crate) fn parse_registry_starts_with(
        &mut self,
        spec: &TypedFunctionSpec,
    ) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;
        self.expect_token(TokenType::Comma)?;
        let expression = self.parse_expression()?;
        self.expect_token(TokenType::RParen)?;
        let func = BinaryFunc {
            original_name: None,
            this,
            expression,
            inferred_type: None,
        };
        let expr = match spec.canonical_name {
            "STARTS_WITH" => Expression::StartsWith(Box::new(func)),
            "ENDS_WITH" => Expression::EndsWith(Box::new(func)),
            _ => {
                return Err(crate::tokens::errors::Error::internal(
                    "binary typed parse kind already matched in caller",
                ))
            }
        };
        Ok(Some(expr))
    }

    pub(crate) fn parse_registry_map_from_arrays(
        &mut self,
        spec: &TypedFunctionSpec,
    ) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;
        self.expect_token(TokenType::Comma)?;
        let expression = self.parse_expression()?;
        self.expect_token(TokenType::RParen)?;
        let func = BinaryFunc {
            original_name: None,
            this,
            expression,
            inferred_type: None,
        };
        let expr = match spec.canonical_name {
            "MAP_FROM_ARRAYS" => Expression::MapFromArrays(Box::new(func)),
            "MAP_CONTAINS_KEY" => Expression::MapContainsKey(Box::new(func)),
            "ELEMENT_AT" => Expression::ElementAt(Box::new(func)),
            _ => {
                return Err(crate::tokens::errors::Error::internal(
                    "binary map parse kind already matched in caller",
                ))
            }
        };
        Ok(Some(expr))
    }

    pub(crate) fn parse_registry_add_months(
        &mut self,
        spec: &TypedFunctionSpec,
    ) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;
        self.expect_token(TokenType::Comma)?;
        let expression = self.parse_expression()?;
        if spec.canonical_name == MONTHS_BETWEEN && self.match_token(TokenType::Comma) {
            let round_off = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Some(Expression::Function(Box::new(
                crate::expressions::functions::models::Function::new(
                    "MONTHS_BETWEEN".to_string(),
                    vec![this, expression, round_off],
                ),
            ))));
        }
        self.expect_token(TokenType::RParen)?;
        let func = BinaryFunc {
            original_name: None,
            this,
            expression,
            inferred_type: None,
        };
        let expr = match spec.canonical_name {
            "ADD_MONTHS" => Expression::AddMonths(Box::new(func)),
            "MONTHS_BETWEEN" => Expression::MonthsBetween(Box::new(func)),
            "NEXT_DAY" => Expression::NextDay(Box::new(func)),
            _ => {
                return Err(crate::tokens::errors::Error::internal(
                    "date binary parse kind already matched in caller",
                ))
            }
        };
        Ok(Some(expr))
    }

    pub(crate) fn parse_registry_array_contains(
        &mut self,
        spec: &TypedFunctionSpec,
    ) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;
        self.expect_token(TokenType::Comma)?;
        let expression = self.parse_expression()?;
        self.expect_token(TokenType::RParen)?;
        let func = BinaryFunc {
            original_name: None,
            this,
            expression,
            inferred_type: None,
        };
        let expr = match spec.canonical_name {
            "ARRAY_CONTAINS" => Expression::ArrayContains(Box::new(func)),
            "ARRAY_POSITION" => Expression::ArrayPosition(Box::new(func)),
            "ARRAY_APPEND" => Expression::ArrayAppend(Box::new(func)),
            "ARRAY_PREPEND" => Expression::ArrayPrepend(Box::new(func)),
            "ARRAY_UNION" => Expression::ArrayUnion(Box::new(func)),
            "ARRAY_EXCEPT" => Expression::ArrayExcept(Box::new(func)),
            "ARRAY_REMOVE" => Expression::ArrayRemove(Box::new(func)),
            _ => {
                return Err(crate::tokens::errors::Error::internal(
                    "array binary parse kind already matched in caller",
                ))
            }
        };
        Ok(Some(expr))
    }

    pub(crate) fn parse_registry_typeof(&mut self, name: &str) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;
        let this = self.maybe_clickhouse_alias(this);
        if self.match_token(TokenType::Comma) {
            let mut all_args = vec![this];
            let remaining = self.parse_function_arguments()?;
            all_args.extend(remaining);
            self.expect_token(TokenType::RParen)?;
            Ok(Some(Expression::Function(Box::new(Function {
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
                span: None,
                inferred_type: None,
            }))))
        } else {
            self.expect_token(TokenType::RParen)?;
            Ok(Some(Expression::Typeof(Box::new(new_unary_func(this)))))
        }
    }

    pub(crate) fn parse_registry_sin(
        &mut self,
        spec: &TypedFunctionSpec,
    ) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;
        if spec.canonical_name == ATAN && self.match_token(TokenType::Comma) {
            let expression = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Some(Expression::Atan2(Box::new(BinaryFunc {
                original_name: Some("ATAN".to_string()),
                this,
                expression,
                inferred_type: None,
            }))));
        }
        self.expect_token(TokenType::RParen)?;
        let func = new_unary_func(this);
        let expr = match spec.canonical_name {
            "SIN" => Expression::Sin(Box::new(func)),
            "COS" => Expression::Cos(Box::new(func)),
            "TAN" => Expression::Tan(Box::new(func)),
            "ASIN" => Expression::Asin(Box::new(func)),
            "ACOS" => Expression::Acos(Box::new(func)),
            "ATAN" => Expression::Atan(Box::new(func)),
            "RADIANS" => Expression::Radians(Box::new(func)),
            "DEGREES" => Expression::Degrees(Box::new(func)),
            _ => {
                return Err(crate::tokens::errors::Error::internal(
                    "trig unary parse kind already matched in caller",
                ))
            }
        };
        Ok(Some(expr))
    }

    pub(crate) fn parse_registry_array_length(
        &mut self,
        name: &str,
        spec: &TypedFunctionSpec,
    ) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;
        if (spec.canonical_name == ARRAY_LENGTH || spec.canonical_name == ARRAY_SIZE)
            && self.match_token(TokenType::Comma)
        {
            let dimension = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Some(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: name.to_string(),
                args: vec![this, dimension],
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: None,
                inferred_type: None,
            }))));
        }
        self.expect_token(TokenType::RParen)?;
        let func = new_unary_func(this);
        let expr = match spec.canonical_name {
            "ARRAY_LENGTH" => Expression::ArrayLength(Box::new(func)),
            "ARRAY_SIZE" => Expression::ArraySize(Box::new(func)),
            "CARDINALITY" => Expression::Cardinality(Box::new(func)),
            "ARRAY_REVERSE" => Expression::ArrayReverse(Box::new(func)),
            "ARRAY_DISTINCT" => Expression::ArrayDistinct(Box::new(func)),
            "ARRAY_COMPACT" => Expression::ArrayCompact(Box::new(func)),
            "EXPLODE" => Expression::Explode(Box::new(func)),
            "EXPLODE_OUTER" => Expression::ExplodeOuter(Box::new(func)),
            _ => {
                return Err(crate::tokens::errors::Error::internal(
                    "array unary parse kind already matched in caller",
                ))
            }
        };
        Ok(Some(expr))
    }

    pub(crate) fn parse_registry_to_number(
        &mut self,
        spec: &TypedFunctionSpec,
    ) -> Result<Option<Expression>> {
        let args = self.parse_function_args_list()?;
        self.expect_token(TokenType::RParen)?;
        let this = args.first().cloned().unwrap_or(Expression::Null(Null {}));
        let (format, precision, scale) =
                    if self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake)
                        && args.len() <= TERNARY_ARITY
                        && args.get(1).is_some_and(|arg| {
                            matches!(arg, Expression::Literal(literal) if matches!(literal.as_ref(), Literal::Number(_)))
                        })
                    {
                        (
                            None,
                            args.get(1).cloned().map(Box::new),
                            args.get(2).cloned().map(Box::new),
                        )
                    } else {
                        (
                            args.get(1).cloned().map(Box::new),
                            args.get(2).cloned().map(Box::new),
                            args.get(3).cloned().map(Box::new),
                        )
                    };
        let safe = if spec.canonical_name == TRY_TO_NUMBER {
            Some(Box::new(Expression::Boolean(BooleanLiteral {
                value: true,
            })))
        } else {
            None
        };
        Ok(Some(Expression::ToNumber(Box::new(ToNumber {
            this: Box::new(this),
            format,
            nlsparam: None,
            precision,
            scale,
            safe,
            safe_name: None,
        }))))
    }

    pub(crate) fn parse_registry_substring(&mut self, name: &str) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;
        let this = self.try_clickhouse_func_arg_alias(this);

        if self.match_token(TokenType::From) {
            let start = self.parse_expression()?;
            let start = self.try_clickhouse_func_arg_alias(start);
            let length = if self.match_token(TokenType::For) {
                let len = self.parse_expression()?;
                Some(self.try_clickhouse_func_arg_alias(len))
            } else {
                None
            };
            self.expect_token(TokenType::RParen)?;
            Ok(Some(Expression::Substring(Box::new(SubstringFunc {
                this,
                start,
                length,
                from_for_syntax: true,
            }))))
        } else if self.match_token(TokenType::For) {
            let length_expr = self.parse_expression()?;
            let length_expr = self.try_clickhouse_func_arg_alias(length_expr);
            let start = if self.match_token(TokenType::From) {
                let s = self.parse_expression()?;
                self.try_clickhouse_func_arg_alias(s)
            } else {
                Expression::Literal(Box::new(Literal::Number("1".to_string())))
            };
            self.expect_token(TokenType::RParen)?;
            Ok(Some(Expression::Substring(Box::new(SubstringFunc {
                this,
                start,
                length: Some(length_expr),
                from_for_syntax: true,
            }))))
        } else if self.match_token(TokenType::Comma) {
            let start = self.parse_expression()?;
            let start = self.try_clickhouse_func_arg_alias(start);
            let length = if self.match_token(TokenType::Comma) {
                let len = self.parse_expression()?;
                Some(self.try_clickhouse_func_arg_alias(len))
            } else {
                None
            };
            self.expect_token(TokenType::RParen)?;
            Ok(Some(Expression::Substring(Box::new(SubstringFunc {
                this,
                start,
                length,
                from_for_syntax: false,
            }))))
        } else {
            self.expect_token(TokenType::RParen)?;
            Ok(Some(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: name.to_string(),
                args: vec![this],
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: None,
                inferred_type: None,
            }))))
        }
    }

    pub(crate) fn parse_registry_date_part(&mut self) -> Result<Option<Expression>> {
        let part = self.parse_expression()?;
        let mut part = { part };
        if !self.match_token(TokenType::From) && !self.match_token(TokenType::Comma) {
            return Err(self.parse_error("Expected FROM or comma in DATE_PART"));
        }
        let from_expr = self.parse_expression()?;
        self.expect_token(TokenType::RParen)?;
        if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        ) && self
            .try_parse_date_part_field_identifier_expr(&part)
            .is_some()
        {
            part = self.convert_date_part_identifier_expr_to_var(part);
        }
        let mut args = vec![part, from_expr];
        args = self.normalize_date_part_arg("DATE_PART", args);
        Ok(Some(Expression::Function(Box::new(Function {
            on_error: None,
            qualified_name: Vec::new(),
            source_dialect: None,

            name: "DATE_PART".to_string(),
            args,
            distinct: false,
            trailing_comments: Vec::new(),
            use_bracket_syntax: false,
            no_parens: false,
            quoted: false,
            span: None,
            inferred_type: None,
        }))))
    }

    pub(crate) fn parse_registry_dateadd(&mut self, name: &str) -> Result<Option<Expression>> {
        let mut first_arg = self.parse_expression()?;
        first_arg = self.try_clickhouse_func_arg_alias(first_arg);
        self.expect_token(TokenType::Comma)?;
        let second_arg = self.parse_expression()?;
        let second_arg = self.try_clickhouse_func_arg_alias(second_arg);

        if self.match_token(TokenType::Comma) {
            let third_arg = self.parse_expression()?;
            let third_arg = self.try_clickhouse_func_arg_alias(third_arg);
            self.expect_token(TokenType::RParen)?;
            if matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::Snowflake)
            ) && self
                .try_parse_date_part_unit_identifier_expr(&first_arg)
                .is_some()
            {
                first_arg = self.convert_date_part_identifier_expr_to_var(first_arg);
            }
            let mut args = vec![first_arg, second_arg, third_arg];
            args = self.normalize_date_part_arg(name, args);
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
                span: None,
                inferred_type: None,
            }))))
        } else {
            self.expect_token(TokenType::RParen)?;
            Ok(Some(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: name.to_string(),
                args: vec![first_arg, second_arg],
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: None,
                inferred_type: None,
            }))))
        }
    }

    pub(crate) fn parse_registry_datediff(&mut self, name: &str) -> Result<Option<Expression>> {
        let first_arg = self.parse_expression()?;
        let first_arg = self.try_clickhouse_func_arg_alias(first_arg);
        self.expect_token(TokenType::Comma)?;
        let second_arg = self.parse_expression()?;
        let second_arg = self.try_clickhouse_func_arg_alias(second_arg);
        let mut args = if self.match_token(TokenType::Comma) {
            let third_arg = self.parse_expression()?;
            let third_arg = self.try_clickhouse_func_arg_alias(third_arg);
            vec![first_arg, second_arg, third_arg]
        } else {
            vec![first_arg, second_arg]
        };
        while self.match_token(TokenType::Comma) {
            let arg = self.parse_expression()?;
            args.push(self.try_clickhouse_func_arg_alias(arg));
        }
        self.expect_token(TokenType::RParen)?;
        if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        ) && args.len() == TERNARY_ARITY
        {
            if let Some(unit) = self.try_parse_date_part_unit_expr(&args[0]) {
                return Ok(Some(Expression::DateDiff(Box::new(DateDiffFunc {
                    this: args[2].clone(),
                    expression: args[1].clone(),
                    unit: Some(unit),
                }))));
            }
        }
        args = self.normalize_date_part_arg(name, args);
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
            span: None,
            inferred_type: None,
        }))))
    }

    pub(crate) fn parse_registry_random(&mut self) -> Result<Option<Expression>> {
        if self.check(TokenType::RParen) {
            self.expect_token(TokenType::RParen)?;
            Ok(Some(Expression::Random(Random)))
        } else {
            let first = self.parse_expression()?;
            if self.match_token(TokenType::Comma) {
                let second = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Rand(Box::new(Rand {
                    seed: None,
                    lower: Some(Box::new(first)),
                    upper: Some(Box::new(second)),
                }))))
            } else {
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Rand(Box::new(Rand {
                    seed: Some(Box::new(first)),
                    lower: None,
                    upper: None,
                }))))
            }
        }
    }

    pub(crate) fn parse_registry_strpos(&mut self) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;
        self.expect_token(TokenType::Comma)?;
        let substr = self.parse_expression()?;
        let occurrence = if self.match_token(TokenType::Comma) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Some(Expression::StrPosition(Box::new(StrPosition {
            this: Box::new(this),
            substr: Some(Box::new(substr)),
            position: None,
            occurrence,
        }))))
    }

    pub(crate) fn parse_registry_locate(&mut self, name: &str) -> Result<Option<Expression>> {
        if self.check(TokenType::RParen) {
            self.skip();
            return Ok(Some(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: name.to_string(),
                args: vec![],
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: None,
                inferred_type: None,
            }))));
        }
        let first = self.parse_expression()?;
        if !self.check(TokenType::Comma) && self.check(TokenType::RParen) {
            self.skip();
            return Ok(Some(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: name.to_string(),
                args: vec![first],
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: None,
                inferred_type: None,
            }))));
        }
        self.expect_token(TokenType::Comma)?;
        let second = self.parse_expression()?;
        let position = if self.match_token(TokenType::Comma) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Some(Expression::StrPosition(Box::new(StrPosition {
            this: Box::new(second),
            substr: Some(Box::new(first)),
            position,
            occurrence: None,
        }))))
    }

    pub(crate) fn parse_registry_instr(&mut self) -> Result<Option<Expression>> {
        let first = self.parse_expression()?;
        self.expect_token(TokenType::Comma)?;
        let second = self.parse_expression()?;
        let position = if self.match_token(TokenType::Comma) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Some(Expression::StrPosition(Box::new(StrPosition {
            this: Box::new(first),
            substr: Some(Box::new(second)),
            position,
            occurrence: None,
        }))))
    }

    pub(crate) fn parse_registry_initcap(&mut self) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;
        let delimiter = if self.match_token(TokenType::Comma) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        if let Some(delim) = delimiter {
            Ok(Some(Expression::Function(Box::new(Function::new(
                "INITCAP".to_string(),
                vec![this, *delim],
            )))))
        } else {
            Ok(Some(Expression::Initcap(Box::new(new_unary_func(this)))))
        }
    }

    pub(crate) fn parse_registry_floor(&mut self, name: &str) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;
        let to = if self.match_token(TokenType::To) {
            self.parse_var()?
        } else {
            None
        };
        let scale = if to.is_none() && self.match_token(TokenType::Comma) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        if self.check(TokenType::Comma) {
            let mut args = vec![this];
            if let Some(s) = scale {
                args.push(s);
            }
            while self.match_token(TokenType::Comma) {
                args.push(self.parse_expression()?);
            }
            self.expect_token(TokenType::RParen)?;
            return Ok(Some(Expression::Function(Box::new(Function {
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
                span: None,
                inferred_type: None,
            }))));
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Some(Expression::Floor(Box::new(FloorFunc {
            this,
            scale,
            to,
        }))))
    }

    pub(crate) fn parse_registry_log(&mut self) -> Result<Option<Expression>> {
        let first = self.parse_expression()?;
        if self.match_token(TokenType::Comma) {
            let second = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            let (value, base) = if self.log_base_first() {
                (second, first)
            } else {
                (first, second)
            };
            Ok(Some(Expression::Log(Box::new(LogFunc {
                this: value,
                base: Some(base),
            }))))
        } else {
            self.expect_token(TokenType::RParen)?;
            {
                Ok(Some(Expression::Log(Box::new(LogFunc {
                    this: first,
                    base: None,
                }))))
            }
        }
    }

    pub(crate) fn parse_registry_extract(&mut self, name: &str) -> Result<Option<Expression>> {
        if self.check(TokenType::String) && !self.check_next(TokenType::From) {
            let args = self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Some(Expression::Function(Box::new(Function {
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
                span: None,
                inferred_type: None,
            }))));
        }

        let field = self.parse_datetime_field()?;
        if !self.match_token(TokenType::From) && !self.match_token(TokenType::Comma) {
            return Err(self.parse_error("Expected FROM or comma after EXTRACT field"));
        }
        let this = self.parse_expression()?;
        let this = self.try_clickhouse_func_arg_alias(this);
        self.expect_token(TokenType::RParen)?;
        Ok(Some(Expression::Extract(Box::new(ExtractFunc {
            this,
            field,
        }))))
    }

    pub(crate) fn parse_registry_char(&mut self, name: &str) -> Result<Option<Expression>> {
        let args = self.parse_expression_list()?;
        let charset = if self.match_token(TokenType::Using) {
            let charset = self.expect_identifier_or_keyword_with_quoted()?;
            Some(self.format_charset_for_raw_sql(&charset))
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        if charset.is_some() {
            Ok(Some(Expression::CharFunc(Box::new(
                crate::expressions::typed_functions::models::CharFunc {
                    args,
                    charset,
                    name: None,
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
                span: None,
                inferred_type: None,
            }))))
        }
    }

    pub(crate) fn parse_registry_chr(&mut self, name: &str) -> Result<Option<Expression>> {
        let args = self.parse_expression_list()?;
        let charset = if self.match_token(TokenType::Using) {
            let charset = self.expect_identifier_or_keyword_with_quoted()?;
            Some(self.format_charset_for_raw_sql(&charset))
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        if charset.is_some() {
            Ok(Some(Expression::CharFunc(Box::new(
                crate::expressions::typed_functions::models::CharFunc {
                    args,
                    charset,
                    name: Some("CHR".to_string()),
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
                span: None,
                inferred_type: None,
            }))))
        }
    }

    pub(crate) fn parse_registry_range_n(&mut self) -> Result<Option<Expression>> {
        let this = self.parse_bitwise_or()?;
        self.expect_token(TokenType::Between)?;
        let mut expressions: Vec<Expression> = Vec::new();
        while !self.check(TokenType::Each) && !self.check(TokenType::RParen) {
            expressions.push(self.parse_expression()?);
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        let each = if self.match_token(TokenType::Each) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Some(Expression::RangeN(Box::new(RangeN {
            this: Box::new(this),
            expressions,
            each,
        }))))
    }

    pub(crate) fn parse_registry_xmlattributes(&mut self) -> Result<Option<Expression>> {
        let mut attrs: Vec<Expression> = Vec::new();
        if !self.check(TokenType::RParen) {
            loop {
                let expr = self.parse_expression()?;
                if self.match_token(TokenType::As) {
                    let alias_ident = self.expect_identifier_or_keyword_with_quoted()?;
                    attrs.push(Expression::Alias(Box::new(Alias {
                        this: expr,
                        alias: alias_ident,
                        column_aliases: Vec::new(),
                        alias_explicit_as: false,
                        alias_keyword: None,
                        pre_alias_comments: Vec::new(),
                        trailing_comments: Vec::new(),
                        inferred_type: None,
                    })));
                } else {
                    attrs.push(expr);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Some(Expression::Function(Box::new(Function {
            on_error: None,
            qualified_name: Vec::new(),
            source_dialect: None,

            name: "XMLATTRIBUTES".to_string(),
            args: attrs,
            distinct: false,
            trailing_comments: Vec::new(),
            use_bracket_syntax: false,
            no_parens: false,
            quoted: false,
            span: None,
            inferred_type: None,
        }))))
    }

    pub(crate) fn parse_registry_match(&mut self) -> Result<Option<Expression>> {
        let expressions = if self.check(TokenType::Table) {
            self.skip();
            let table_name = self.expect_identifier_or_keyword()?;
            vec![Expression::Var(Box::new(Var {
                this: format!("TABLE {}", table_name),
            }))]
        } else {
            self.parse_expression_list()?
        };

        self.expect_token(TokenType::RParen)?;

        if !self.check_keyword_text("AGAINST") {
            return Ok(Some(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: "MATCH".to_string(),
                args: expressions,
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: None,
                inferred_type: None,
            }))));
        }

        self.skip();
        self.expect_token(TokenType::LParen)?;
        let search_expr = self.parse_primary()?;

        let modifier = if self.match_text_seq(&["IN", "NATURAL", "LANGUAGE", "MODE"]) {
            if self.match_text_seq(&["WITH", "QUERY", "EXPANSION"]) {
                Some(Box::new(Expression::Var(Box::new(Var {
                    this: "IN NATURAL LANGUAGE MODE WITH QUERY EXPANSION".to_string(),
                }))))
            } else {
                Some(Box::new(Expression::Var(Box::new(Var {
                    this: "IN NATURAL LANGUAGE MODE".to_string(),
                }))))
            }
        } else if self.match_text_seq(&["IN", "BOOLEAN", "MODE"]) {
            Some(Box::new(Expression::Var(Box::new(Var {
                this: "IN BOOLEAN MODE".to_string(),
            }))))
        } else if self.match_text_seq(&["WITH", "QUERY", "EXPANSION"]) {
            Some(Box::new(Expression::Var(Box::new(Var {
                this: "WITH QUERY EXPANSION".to_string(),
            }))))
        } else {
            None
        };

        self.expect_token(TokenType::RParen)?;

        Ok(Some(Expression::MatchAgainst(Box::new(MatchAgainst {
            this: Box::new(search_expr),
            expressions,
            modifier,
        }))))
    }

    pub(crate) fn parse_registry_transform(
        &mut self,
        name: &str,
        quoted: bool,
    ) -> Result<Option<Expression>> {
        let expressions = if self.check(TokenType::RParen) {
            Vec::new()
        } else {
            self.parse_function_args_with_lambda()?
        };
        self.expect_token(TokenType::RParen)?;

        let row_format_before = if self.match_token(TokenType::Row) {
            self.parse_row()?
        } else {
            None
        };

        let record_writer = if self.match_text_seq(&["RECORDWRITER"]) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        if self.match_token(TokenType::Using) {
            let command_script = Some(Box::new(self.parse_expression()?));
            let schema = if self.match_token(TokenType::As) {
                self.parse_schema()?
            } else {
                None
            };

            let row_format_after = if self.match_token(TokenType::Row) {
                self.parse_row()?
            } else {
                None
            };

            let record_reader = if self.match_text_seq(&["RECORDREADER"]) {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };

            Ok(Some(Expression::QueryTransform(Box::new(QueryTransform {
                expressions,
                command_script,
                schema: schema.map(Box::new),
                row_format_before: row_format_before.map(Box::new),
                record_writer,
                row_format_after: row_format_after.map(Box::new),
                record_reader,
            }))))
        } else {
            Ok(Some(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: name.to_string(),
                args: expressions,
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted,
                span: None,
                inferred_type: None,
            }))))
        }
    }

    pub(crate) fn parse_registry_convert(
        &mut self,
        upper_name: &str,
    ) -> Result<Option<Expression>> {
        let is_try = upper_name == TRY_CONVERT;

        let this = self.parse_expression()?;
        if self.match_token(TokenType::Using) {
            let charset = self.expect_identifier_or_keyword_with_quoted()?;
            self.expect_token(TokenType::RParen)?;
            Ok(Some(Expression::Cast(Box::new(Cast {
                this,
                to: DataType::CharacterSet {
                    name: self.format_charset_for_raw_sql(&charset),
                },
                trailing_comments: Vec::new(),
                double_colon_syntax: false,
                format: None,
                default: None,
                inferred_type: None,
            }))))
        } else if self.match_token(TokenType::Comma) {
            let mut args = vec![this];
            args.push(self.parse_expression()?);
            while self.match_token(TokenType::Comma) {
                args.push(self.parse_expression()?);
            }
            self.expect_token(TokenType::RParen)?;
            let func_name = if is_try { "TRY_CONVERT" } else { "CONVERT" };
            Ok(Some(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: func_name.to_string(),
                args,
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: None,
                inferred_type: None,
            }))))
        } else {
            self.expect_token(TokenType::RParen)?;
            let func_name = if is_try { "TRY_CONVERT" } else { "CONVERT" };
            Ok(Some(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: func_name.to_string(),
                args: vec![this],
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: None,
                inferred_type: None,
            }))))
        }
    }

    pub(crate) fn parse_registry_trim(&mut self) -> Result<Option<Expression>> {
        let (position, position_explicit) = if self.match_token(TokenType::Leading) {
            (TrimPosition::Leading, true)
        } else if self.match_token(TokenType::Trailing) {
            (TrimPosition::Trailing, true)
        } else if self.match_token(TokenType::Both) {
            (TrimPosition::Both, true)
        } else {
            (TrimPosition::Both, false)
        };

        if position_explicit || self.check(TokenType::From) {
            if self.match_token(TokenType::From) {
                let this = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Trim(Box::new(TrimFunc {
                    this,
                    characters: None,
                    position,
                    sql_standard_syntax: true,
                    position_explicit,
                    inferred_type: None,
                }))))
            } else {
                let first_expr = self.parse_bitwise_or()?;
                let first_expr = self.try_clickhouse_func_arg_alias(first_expr);
                if self.match_token(TokenType::From) {
                    let this = self.parse_bitwise_or()?;
                    let this = self.try_clickhouse_func_arg_alias(this);
                    self.expect_token(TokenType::RParen)?;
                    Ok(Some(Expression::Trim(Box::new(TrimFunc {
                        this,
                        characters: Some(first_expr),
                        position,
                        sql_standard_syntax: true,
                        position_explicit,
                        inferred_type: None,
                    }))))
                } else {
                    self.expect_token(TokenType::RParen)?;
                    Ok(Some(Expression::Trim(Box::new(TrimFunc {
                        this: first_expr,
                        characters: None,
                        position,
                        sql_standard_syntax: true,
                        position_explicit,
                        inferred_type: None,
                    }))))
                }
            }
        } else {
            let first_expr = self.parse_expression()?;
            let first_expr = self.try_clickhouse_func_arg_alias(first_expr);
            if self.match_token(TokenType::From) {
                let this = self.parse_expression()?;
                let this = self.try_clickhouse_func_arg_alias(this);
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Trim(Box::new(TrimFunc {
                    this,
                    characters: Some(first_expr),
                    position: TrimPosition::Both,
                    sql_standard_syntax: true,
                    position_explicit: false,
                    inferred_type: None,
                }))))
            } else if self.match_token(TokenType::Comma) {
                let second_expr = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                let (this, characters) = { (first_expr, second_expr) };
                Ok(Some(Expression::Trim(Box::new(TrimFunc {
                    this,
                    characters: Some(characters),
                    position: TrimPosition::Both,
                    sql_standard_syntax: false,
                    position_explicit: false,
                    inferred_type: None,
                }))))
            } else {
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Trim(Box::new(TrimFunc {
                    this: first_expr,
                    characters: None,
                    position: TrimPosition::Both,
                    sql_standard_syntax: false,
                    position_explicit: false,
                    inferred_type: None,
                }))))
            }
        }
    }

    pub(crate) fn parse_registry_overlay(&mut self, name: &str) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;
        if self.match_token(TokenType::Placing) {
            let replacement = self.parse_expression()?;
            self.expect_token(TokenType::From)?;
            let from = self.parse_expression()?;
            let length = if self.match_token(TokenType::For) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.expect_token(TokenType::RParen)?;
            Ok(Some(Expression::Overlay(Box::new(OverlayFunc {
                this,
                replacement,
                from,
                length,
            }))))
        } else if self.match_token(TokenType::Comma) {
            let replacement = self.parse_expression()?;
            if self.match_token(TokenType::Comma) {
                let from = self.parse_expression()?;
                let length = if self.match_token(TokenType::Comma) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Overlay(Box::new(OverlayFunc {
                    this,
                    replacement,
                    from,
                    length,
                }))))
            } else {
                self.expect_token(TokenType::RParen)?;
                Ok(Some(Expression::Function(Box::new(Function {
                    on_error: None,
                    qualified_name: Vec::new(),
                    source_dialect: None,

                    name: name.to_string(),
                    args: vec![this, replacement],
                    distinct: false,
                    trailing_comments: Vec::new(),
                    use_bracket_syntax: false,
                    no_parens: false,
                    quoted: false,
                    span: None,
                    inferred_type: None,
                }))))
            }
        } else {
            self.expect_token(TokenType::RParen)?;
            Ok(Some(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: name.to_string(),
                args: vec![this],
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: None,
                inferred_type: None,
            }))))
        }
    }

    pub(crate) fn parse_registry_ceil(&mut self) -> Result<Option<Expression>> {
        let this = self.parse_expression()?;
        let to = if self.match_token(TokenType::To) {
            self.parse_var()?
        } else {
            None
        };
        let decimals = if to.is_none() && self.match_token(TokenType::Comma) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Some(Expression::Ceil(Box::new(CeilFunc {
            this,
            decimals,
            to,
        }))))
    }

    pub(crate) fn parse_registry_if(
        &mut self,
        name: &str,
        upper_name: &str,
    ) -> Result<Option<Expression>> {
        if self.check(TokenType::RParen) {
            self.skip();
            return Ok(Some(Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: name.to_string(),
                args: vec![],
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: None,
                inferred_type: None,
            }))));
        }
        let args = self.parse_expression_list()?;
        self.expect_token(TokenType::RParen)?;
        let expr = if args.len() == TERNARY_ARITY {
            Expression::IfFunc(Box::new(
                crate::expressions::typed_functions::models::IfFunc {
                    original_name: Some(upper_name.to_string()),
                    condition: args[0].clone(),
                    true_value: args[1].clone(),
                    false_value: Some(args[2].clone()),
                    inferred_type: None,
                },
            ))
        } else if args.len() == BINARY_ARITY {
            Expression::IfFunc(Box::new(
                crate::expressions::typed_functions::models::IfFunc {
                    original_name: Some(upper_name.to_string()),
                    condition: args[0].clone(),
                    true_value: args[1].clone(),
                    false_value: None,
                    inferred_type: None,
                },
            ))
        } else {
            return Err(self.parse_error("IF function requires 2 or 3 arguments"));
        };
        Ok(Some(expr))
    }
}
