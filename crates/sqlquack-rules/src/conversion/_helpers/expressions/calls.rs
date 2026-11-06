//! Generic function calls and aggregates.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::aggregates::constants::EXTREME_KEY_TIES;
use sqlquack_duckdb::aggregates::main::json_variant_extreme::json_variant_extreme;
use sqlquack_duckdb::arrays::constants::ARRAY_SORT_ORDER;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::blocks::main::exact_decimal_divide::exact_decimal_divide;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_duckdb::floats::constants::FLOAT_SUM_ORDER;
use sqlquack_duckdb::floats::main::exact_float_sum::exact_float_sum;
use sqlquack_snowflake::typing::constants::{COUNT_PRECISION, MAX_PRECISION, SUM_EXTRA_DIGITS};
use sqlquack_snowflake::typing::main::average::average;
use sqlquack_snowflake::typing::main::count::count;
use sqlquack_snowflake::typing::main::framed_average::framed_average;
use sqlquack_snowflake::typing::main::sum::sum;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::arrays::array_function;
use crate::conversion::_helpers::expressions::casts::zoned_text;
use crate::conversion::_helpers::expressions::casts::{
    to_boolean_call, to_double_call, to_number_call, to_varchar_call,
};
use crate::conversion::_helpers::expressions::conditionals::unified;
use crate::conversion::_helpers::expressions::conditionals::{
    decode, div0, null_if_zero, zero_if_null,
};
use crate::conversion::_helpers::expressions::hashing::hash_call;
use crate::conversion::_helpers::expressions::lambdas::{lambda_call, reduce_call};
use crate::conversion::_helpers::expressions::numeric_functions::{
    absolute, bitwise, logarithm, power, rounding, sign,
};
use crate::conversion::_helpers::expressions::objects::{
    object_construct, object_delete, object_insert,
};
use crate::conversion::_helpers::expressions::operators::{comparable, equal_null, mod_call};
use crate::conversion::_helpers::expressions::project_functions::{override_call, project_call};
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::semi_structured::{
    parse_json, variant_get, variant_inspect,
};
use crate::conversion::_helpers::expressions::strings::{
    bytes_as_text, chr, concatenate, concatenate_with_separator, edit_distance, hex_encode,
    initcap, jaro_winkler, repeat, text_contains, text_md5, text_operand, text_sha2, trim_call,
    try_hex_decode,
};
use crate::conversion::_helpers::expressions::temporal_functions::{
    convert_timezone, current_time, date_add, date_diff_call, date_from_parts_call, date_part,
    date_trunc, day_name, last_day, temporal_text, timestamp_from_parts_call, to_time,
    to_timestamp, to_timestamp_tz,
};
use crate::conversion::_helpers::expressions::text_functions::{
    left_right, pad, regexp, replace, split_part, stripped_type_name, suffixed_column,
    system_typeof_call, translate, type_name,
};
use crate::conversion::_helpers::expressions::text_search::charindex_call;
use crate::conversion::_helpers::expressions::variants::variant_function;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Context, Emitted};
use crate::conversion::_helpers::support::site::{Placement, Site};
use crate::conversion::_helpers::support::tree::{
    Payload, field, flag, function_construct, list, node_construct, node_kind, payload_object,
    require_understood,
};
use crate::conversion::_helpers::support::typed_leaves::compared_by_text;
use crate::conversion::constants::MAX_KIND;
use crate::conversion::constants::{
    AVERAGE, AVG_NAME, COUNT, GENERATOR_RULE, GENERATOR_SEQUENCE_COLUMN, LEAST_GREATEST,
    LOCAL_ZONE_SUFFIX, MIN_MAX, MIN_MAX_FLOAT, NOT_SNOWFLAKE_FUNCTIONS, NULLIF,
    PART_FUNCTIONS_PAIR, REGEXP_LIKE_NAME, RLIKE_PAIR, SORTED_ARRAY_FUNCTION, STAR_KIND, SUM,
    SUM_FLOAT, TIMESTAMPDIFF_PAIR, TO_DATE_NAME, TRIM, TRY_PREFIX, VARIANT_MAX,
};
use sqlquack_policy::policy::constants::NUMERIC_RANGE;

/// SEQ4() or SEQ8() in a query level reading only a generator: the row's number from 0.
fn generator_sequence(
    name: &str,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(GENERATOR_RULE);
    let [relation] = site.scope.relations.as_slice() else {
        return Err(outside_conditions(
            construct.clone(),
            format!("{name} in a query level not reading only a generator"),
        ));
    };
    let Some(column) = relation
        .columns
        .iter()
        .find(|column| column.name == GENERATOR_SEQUENCE_COLUMN)
    else {
        return Err(outside_conditions(
            construct.clone(),
            format!("{name} over a relation other than a generator"),
        ));
    };
    let precision = match name {
        "SEQ4" => 10,
        _ => 19,
    };
    let data_type = SnowflakeType::Number {
        precision,
        scale: 0,
    };
    let read = format!(
        "{}.{}",
        quoted_identifier(&relation.alias, log),
        quoted_identifier(&column.name, log)
    );
    Ok(Emitted::new(
        cast(&read, data_type, log)
            .map_err(|reason| outside_conditions(construct.clone(), reason))?,
        data_type,
    ))
}

/// NULLIF, LTRIM, RTRIM and CONCAT, which the parser keeps as generic calls.
pub(crate) fn convert_function(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let payload = payload_object(payload, &node_construct("function"))?;
    let name = payload
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_uppercase();
    let construct = function_construct(&name);
    if let Some((resolved, spec)) = site.context.schema.functions.get_key_value(&name) {
        require_understood(payload, &["name", "args"], &construct)?;
        return project_call((resolved, spec), list(payload, "args"), &construct, site);
    }
    if let Some(replacement) = site.context.options.overrides.get(&name) {
        require_understood(payload, &["name", "args"], &construct)?;
        return override_call(replacement, list(payload, "args"), &construct, site);
    }
    if NOT_SNOWFLAKE_FUNCTIONS.contains(&name.as_str()) {
        return Err(missing(
            construct,
            format!("not in Snowflake's function reference: {name}"),
        ));
    }
    let understood: &[&str] = match name.as_str() {
        "CURRENT_DATE" | "CURRENT_TIMESTAMP" => &["name", "args", "no_parens"],
        _ => &["name", "args"],
    };
    require_understood(payload, understood, &construct)?;
    let arguments = list(payload, "args");
    if name != SORTED_ARRAY_FUNCTION
        && let Some(converted) = array_function(&name, arguments, &construct, site)
    {
        return converted;
    }
    match (name.as_str(), arguments) {
        ("NULLIF", [left, right]) => nullif(left, right, site),
        ("LTRIM" | "RTRIM", [text, rest @ ..]) if rest.len() <= 1 => {
            let _rule = site.context.log.rule(TRIM);
            let text = text_operand(text, &construct, site)?;
            let characters = match rest.first() {
                Some(characters) => Some(text_operand(characters, &construct, site)?),
                None => None,
            };
            Ok(Emitted::new(
                trim_call(&name.to_ascii_lowercase(), &text, characters.as_deref()),
                SnowflakeType::Varchar,
            ))
        }
        ("CONCAT", parts) if !parts.is_empty() => {
            concatenate(&parts.iter().collect::<Vec<_>>(), &construct, site)
        }
        ("CONCAT_WS", parts) => concatenate_with_separator(parts, &construct, site),
        ("CONTAINS", parts) => text_contains(parts, &construct, site),
        ("EDITDISTANCE", parts) => edit_distance(parts, &construct, site),
        ("JAROWINKLER_SIMILARITY", parts) => jaro_winkler(parts, &construct, site),
        ("CHARINDEX" | "POSITION", parts) => charindex_call(&name, parts, &construct, site),
        ("MD5", parts) => text_md5(parts, &construct, site),
        ("MOD", parts) => mod_call(parts, &construct, site),
        ("FILTER" | "TRANSFORM", parts) => lambda_call(&name, parts, &construct, site),
        ("REDUCE", parts) => reduce_call(parts, &construct, site),
        ("INITCAP", parts) => initcap(parts, &construct, site),
        ("CHR", parts) => chr(parts, &construct, site),
        ("REPEAT", parts) => repeat(parts, &construct, site),
        ("SEQ4" | "SEQ8", []) => generator_sequence(&name, &construct, site),
        ("HEX_ENCODE", parts) => hex_encode(parts, &construct, site),
        ("TRY_HEX_DECODE_STRING" | "HEX_DECODE_STRING", parts) => {
            try_hex_decode(&name, parts, &construct, site)
        }
        ("SHA2", parts) => text_sha2(parts, &construct, site),
        ("TO_BOOLEAN" | "TRY_TO_BOOLEAN", parts) => to_boolean_call(&name, parts, &construct, site),
        (
            "TO_NUMBER" | "TO_DECIMAL" | "TO_NUMERIC" | "TRY_TO_NUMBER" | "TRY_TO_DECIMAL"
            | "TRY_TO_NUMERIC",
            parts,
        ) => to_number_call(&name, parts, &construct, site),
        ("TO_DOUBLE" | "TRY_TO_DOUBLE", parts) => to_double_call(&name, parts, &construct, site),
        ("TO_VARCHAR" | "TO_CHAR", [value]) => to_varchar_call(&name, value, &construct, site),
        ("TO_VARCHAR" | "TO_CHAR", [value, format]) => {
            match bytes_as_text((value, format), &construct, site)? {
                Some(text) => Ok(text),
                None => temporal_text((value, format), &construct, site),
            }
        }
        ("TRY_BASE64_DECODE_BINARY", _) => Err(outside_conditions(
            construct,
            "decoded bytes other than as the operand of HEX_ENCODE, TO_VARCHAR with UTF-8 or \
             SUBSTR inside them"
                .to_owned(),
        )),
        ("LPAD" | "RPAD", parts) => pad(&name, parts, &construct, site),
        ("ABS", parts) => absolute(parts, &construct, site),
        ("SIGN", parts) => sign(parts, &construct, site),
        ("DECODE", parts) => decode(parts, &construct, site),
        ("HASH", parts) => hash_call(parts, &construct, site),
        ("ZEROIFNULL", parts) => zero_if_null(parts, &construct, site),
        ("DIV0", parts) => div0(parts, false, &construct, site),
        ("DIV0NULL", parts) => div0(parts, true, &construct, site),
        ("NULLIFZERO", parts) => null_if_zero(parts, &construct, site),
        ("LN" | "EXP" | "SQRT", parts) => logarithm(&name, parts, &construct, site),
        ("POWER", parts) => power(parts, &construct, site),
        ("EQUAL_NULL", parts) => equal_null(parts, &construct, site),
        ("ROUND" | "CEIL" | "FLOOR" | "TRUNC" | "TRUNCATE", parts) => {
            rounding(&name, parts, &construct, site)
        }
        ("BITAND" | "BITOR" | "BITXOR", parts) => bitwise(&name, parts, &construct, site),
        ("LEFT" | "RIGHT", [text, length]) => left_right(&name, (text, length), &construct, site),
        ("REPLACE", parts) if (2..=3).contains(&parts.len()) => replace(parts, &construct, site),
        ("TRANSLATE", parts) => translate(parts, &construct, site),
        ("SPLIT_PART", parts) => split_part(parts, &construct, site),
        ("REGEXP_REPLACE", [typed, pattern, replacement])
            if system_typeof_call(typed).is_some() || suffixed_column(typed, site) =>
        {
            stripped_type_name((typed, pattern, replacement), site)
        }
        ("SYSTEM$TYPEOF", [operand]) => type_name(operand, site),
        ("SYSTEM$TYPEOF", _) => Err(outside_conditions(
            construct,
            "SYSTEM$TYPEOF with these arguments".to_owned(),
        )),
        (
            "REGEXP_LIKE" | "REGEXP_REPLACE" | "REGEXP_SUBSTR" | "REGEXP_SUBSTR_ALL"
            | "REGEXP_COUNT" | "REGEXP_INSTR",
            parts,
        ) => regexp(&name, parts, &construct, site),
        ("RLIKE", parts) => {
            site.context.log.equivalence(RLIKE_PAIR);
            regexp(REGEXP_LIKE_NAME, parts, &construct, site)
        }
        ("DATE_PART", [unit, value]) => date_part(unit, value, &construct, site),
        ("DAYOFWEEKISO", [value]) => {
            site.context.log.equivalence(PART_FUNCTIONS_PAIR);
            let unit = serde_json::json!({ "var": { "this": "DAYOFWEEKISO" } });
            date_part(&unit, value, &construct, site)
        }
        ("DATEDIFF", parts) => date_diff_call(parts, &construct, site),
        ("TIMESTAMPDIFF", parts) => {
            site.context.log.equivalence(TIMESTAMPDIFF_PAIR);
            date_diff_call(parts, &construct, site)
        }
        ("TO_TIME" | "TRY_TO_TIME" | "TIME", parts) => to_time(&name, parts, &construct, site),
        ("DATE_FROM_PARTS", parts) => date_from_parts_call(parts, &construct, site),
        ("TIMESTAMP_NTZ_FROM_PARTS", parts) => timestamp_from_parts_call(parts, &construct, site),
        ("CURRENT_DATE" | "CURRENT_TIMESTAMP", parts) => {
            current_time(&name, parts, &construct, site)
        }
        ("DATE_TRUNC", [unit, value]) => date_trunc(unit, value, &construct, site),
        ("DAYNAME", parts) => day_name(parts, &construct, site),
        ("LAST_DAY", parts) => last_day(parts, &construct, site),
        ("DATEADD" | "TIMEADD" | "TIMESTAMPADD", [unit, amount, value]) => {
            date_add((unit, amount, value), &construct, site)
        }
        ("CONVERT_TIMEZONE", parts) => convert_timezone(parts, &construct, site),
        ("PARSE_JSON" | "TRY_PARSE_JSON", parts) => parse_json(parts, &construct, site),
        ("GET" | "GET_PATH" | "GET_IGNORE_CASE", parts) => {
            variant_get(&name, parts, &construct, site)
        }
        ("OBJECT_DELETE", parts) => object_delete(parts, &construct, site),
        (
            "OBJECT_PICK"
            | "OBJECT_KEYS"
            | "ARRAY_CONSTRUCT"
            | "ARRAY_EXCEPT"
            | "ARRAY_POSITION"
            | "ARRAY_CAT"
            | "ARRAY_COMPACT"
            | "ARRAY_SIZE"
            | "ARRAY_SORT"
            | "ARRAY_CONTAINS"
            | "ARRAY_CONSTRUCT_COMPACT"
            | "ARRAY_TO_STRING"
            | "SPLIT"
            | "PARSE_URL"
            | "TO_VARIANT"
            | "IS_OBJECT"
            | "IS_ARRAY"
            | "IS_BOOLEAN"
            | "IS_DATE"
            | "IS_TIME"
            | "IS_TIMESTAMP_NTZ"
            | "IS_TIMESTAMP_LTZ"
            | "IS_TIMESTAMP_TZ"
            | "IS_BINARY",
            parts,
        ) => variant_function(&name, parts, &construct, site),
        ("OBJECT_INSERT", parts) => object_insert(parts, &construct, site),
        ("OBJECT_CONSTRUCT" | "OBJECT_CONSTRUCT_KEEP_NULL", parts) => {
            object_construct(&name, parts, &construct, site)
        }
        (
            "TYPEOF" | "IS_NULL_VALUE" | "STRIP_NULL_VALUE" | "TO_JSON" | "IS_DECIMAL"
            | "IS_INTEGER",
            parts,
        ) => variant_inspect(&name, parts, &construct, site),
        (
            "TO_TIMESTAMP"
            | "TO_TIMESTAMP_NTZ"
            | "TRY_TO_TIMESTAMP"
            | "TRY_TO_TIMESTAMP_NTZ"
            | "TO_DATE"
            | "TRY_TO_DATE",
            parts,
        ) => to_timestamp(&name, parts, &construct, site),
        ("DATE", parts) => to_timestamp(TO_DATE_NAME, parts, &construct, site),
        ("TRY_TO_TIMESTAMP_TZ", parts) => to_timestamp_tz(parts, &construct, site),
        ("TO_TIMESTAMP_TZ" | "TO_TIMESTAMP_LTZ" | "TRY_TO_TIMESTAMP_LTZ", [value]) => {
            zoned_text_call(&name, value, &construct, site)
        }
        ("LEAST" | "GREATEST", parts) if !parts.is_empty() => {
            least_greatest(&name, parts, &construct, site)
        }
        _ => Err(missing(construct, format!("no rule converts fn:{name}"))),
    }
}

/// LEAST or GREATEST, NULL when any argument is NULL, as in Snowflake.
fn least_greatest(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(LEAST_GREATEST);
    let mut converted: Vec<Emitted> = Vec::with_capacity(arguments.len());
    for argument in arguments {
        converted.push(convert_expression(argument, site)?);
    }
    for other in converted.iter().skip(1) {
        comparable(converted[0].clone(), other.clone(), construct, site.context)?;
    }
    let (branches, target) = unified(&converted, construct, site.context)?;
    let any_null: Vec<String> = branches
        .iter()
        .map(|branch| format!("{branch} IS NULL"))
        .collect();
    Ok(Emitted::new(
        format!(
            "(CASE WHEN {} THEN NULL ELSE {}({}) END)",
            any_null.join(" OR "),
            name.to_ascii_lowercase(),
            branches.join(", ")
        ),
        target,
    ))
}

fn nullif(left: &Value, right: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = function_construct("NULLIF");
    let _rule = site.context.log.rule(NULLIF);
    let left = convert_expression(left, site)?;
    let right = convert_expression(right, site)?;
    comparable(left.clone(), right.clone(), &construct, site.context)?;
    let (branches, target) = unified(&[left, right], &construct, site.context)?;
    Ok(Emitted::new(
        format!("NULLIF({}, {})", branches[0], branches[1]),
        target,
    ))
}

/// COUNT, SUM, AVG, MIN and MAX; `window` is the OVER clause and whether it is not running.
pub(crate) fn convert_aggregate(
    kind: &str,
    payload: &Value,
    site: Site<'_>,
    window: Option<(&str, bool)>,
) -> Result<Emitted, Refusal> {
    let construct = node_construct(kind);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "star", "distinct", "name"], &construct)?;
    let placement = match window.is_some() && site.scope.grouped {
        true => Placement::GROUPED,
        false => Placement::ROW,
    };
    let call = AggregateCall {
        payload,
        site: site.placed(placement),
        over: window.map_or_else(String::new, |(window, _)| format!(" OVER ({window})")),
        framed: window.is_some_and(|(_, framed)| framed),
    };
    match kind {
        "count" => call.count(),
        "sum" => call.sum(),
        "avg" => call.average(),
        _ => call.min_max(kind),
    }
}

struct AggregateCall<'a> {
    payload: &'a Payload,
    site: Site<'a>,
    over: String,
    /// Whether the OVER clause has an explicit frame or no order, rather than a running frame.
    framed: bool,
}

impl AggregateCall<'_> {
    fn distinct(&self) -> &'static str {
        match flag(self.payload, "distinct") {
            true => "DISTINCT ",
            false => "",
        }
    }

    fn argument(&self, kind: &str) -> Result<Emitted, Refusal> {
        convert_expression(
            field(self.payload, "this", &node_construct(kind))?,
            self.site,
        )
    }

    fn count(&self) -> Result<Emitted, Refusal> {
        let log = self.site.context.log;
        let _rule = log.rule(COUNT);
        let starred = self
            .payload
            .get("this")
            .and_then(node_kind)
            .is_some_and(|(kind, _)| kind == STAR_KIND);
        if starred || (flag(self.payload, "star") && flag(self.payload, "distinct")) {
            return Err(outside_conditions(
                node_construct("count"),
                "COUNT over a qualified star or a star with EXCLUDE, ILIKE, RENAME, REPLACE or DISTINCT, which counts other rows than COUNT(*)".to_owned(),
            ));
        }
        let inner = match flag(self.payload, "star") {
            true => "*".to_owned(),
            false => {
                let argument = self.argument("count")?;
                match argument.data_type {
                    SnowflakeType::SemiStructured if flag(self.payload, "distinct") => {
                        return Err(outside_conditions(
                            node_construct("count"),
                            "COUNT(DISTINCT) of a VARIANT".to_owned(),
                        ));
                    }
                    SnowflakeType::SemiStructured => {
                        format!("CASE WHEN json_type({}) <> 'NULL' THEN 1 END", argument.sql)
                    }
                    _ => format!("{}{}", self.distinct(), argument.sql),
                }
            }
        };
        let data_type = count(log);
        let sql = cast(&format!("count({inner}){}", self.over), data_type, log)
            .map_err(|reason| outside_conditions(node_construct("count"), reason))?;
        Ok(Emitted::new(sql, data_type))
    }

    fn sum(&self) -> Result<Emitted, Refusal> {
        let log = self.site.context.log;
        let refuse = |reason: String| outside_conditions(node_construct("sum"), reason);
        let argument = self.argument("sum")?;
        if argument.data_type == SnowflakeType::Float {
            let _float = log.rule(SUM_FLOAT);
            diverge(
                self.site.context,
                FLOAT_SUM_ORDER,
                &function_construct("SUM"),
            )?;
            return Ok(Emitted::new(
                exact_float_sum(&argument.sql, self.distinct(), &self.over, log),
                SnowflakeType::Float,
            ));
        }
        let _rule = log.rule(SUM);
        let data_type = sum_type(argument.data_type, self.site.context, "sum").map_err(refuse)?;
        let sql = cast(
            &format!("sum({}{}){}", self.distinct(), argument.sql, self.over),
            data_type,
            log,
        )
        .map_err(refuse)?;
        Ok(Emitted::new(sql, data_type))
    }

    fn average(&self) -> Result<Emitted, Refusal> {
        let log = self.site.context.log;
        let _rule = log.rule(AVERAGE);
        let refuse = |reason: String| outside_conditions(node_construct("avg"), reason);
        let argument = self.argument("avg")?;
        if !self.framed
            && flag(self.payload, "distinct")
            && matches!(argument.data_type, SnowflakeType::Number { .. })
        {
            return Err(refuse(
                "AVG(DISTINCT) of a NUMBER, which Snowflake types with 3 more decimal places, not 6, and no recording shows how it rounds".to_owned(),
            ));
        }
        let total = sum_type(argument.data_type, self.site.context, "avg").map_err(refuse)?;
        let data_type = match self.framed {
            true => framed_average(argument.data_type, log),
            false => average(argument.data_type, log),
        }
        .map_err(refuse)?;
        let SnowflakeType::Number { precision, scale } = total else {
            return Err(refuse(format!(
                "AVG of {} has no exact conversion",
                argument.data_type
            )));
        };
        let distinct = self.distinct();
        let counted = cast(
            &format!("count({distinct}{}){}", argument.sql, self.over),
            count(log),
            log,
        )
        .map_err(refuse)?;
        let total_sql = cast(
            &format!("sum({distinct}{}){}", argument.sql, self.over),
            total,
            log,
        )
        .map_err(refuse)?;
        let (divided, guarded) = exact_decimal_divide(
            (&total_sql, precision, scale),
            (&counted, COUNT_PRECISION, 0),
            (data_type, self.framed),
            log,
        )
        .map_err(refuse)?;
        if guarded {
            diverge(
                self.site.context,
                NUMERIC_RANGE,
                &function_construct(AVG_NAME),
            )?;
        }
        Ok(Emitted::new(
            format!("(CASE WHEN {counted} = 0 THEN NULL ELSE {divided} END)"),
            data_type,
        ))
    }

    fn min_max(&self, kind: &str) -> Result<Emitted, Refusal> {
        let _rule = self.site.context.log.rule(MIN_MAX);
        let argument = self.argument(kind)?;
        match argument.data_type {
            SnowflakeType::Number { .. }
            | SnowflakeType::Varchar
            | SnowflakeType::Boolean
            | SnowflakeType::Date
            | SnowflakeType::Time { .. }
            | SnowflakeType::TimestampNtz { .. }
            | SnowflakeType::TimestampTz { .. }
            | SnowflakeType::TimestampLtz { .. } => Ok(Emitted {
                utc_offset: argument.utc_offset,
                declared: argument.declared.clone(),
                ..Emitted::new(
                    format!("{kind}({}{}){}", self.distinct(), argument.sql, self.over),
                    argument.data_type,
                )
            }),
            SnowflakeType::SemiStructured
                if self.over.is_empty()
                    && self.distinct().is_empty()
                    && !argument.is_variant_held() =>
            {
                let context = self.site.context;
                let construct = node_construct(kind);
                let _variant = context.log.rule(VARIANT_MAX);
                diverge(context, EXTREME_KEY_TIES, &construct)?;
                diverge(context, ARRAY_SORT_ORDER, &construct)?;
                compared_by_text((argument.untainted, argument.sourced), &construct, context)?;
                Ok(Emitted::new(
                    json_variant_extreme(&argument.sql, kind == MAX_KIND, context.log),
                    SnowflakeType::SemiStructured,
                ))
            }
            SnowflakeType::Float => {
                let _float = self.site.context.log.rule(MIN_MAX_FLOAT);
                Ok(Emitted::new(
                    format!("{kind}({}{}){}", self.distinct(), argument.sql, self.over),
                    SnowflakeType::Float,
                ))
            }
            other => Err(outside_conditions(
                node_construct(kind),
                format!("{} of {other}", kind.to_ascii_uppercase()),
            )),
        }
    }
}

/// The SUM type; above precision 26 a total may exceed 38 digits, which relies on `numeric_range`.
fn sum_type(
    argument: SnowflakeType,
    context: &Context<'_>,
    kind: &str,
) -> Result<SnowflakeType, String> {
    let safe_precision = MAX_PRECISION - SUM_EXTRA_DIGITS;
    match argument {
        SnowflakeType::Number { precision, .. } if precision <= safe_precision => {
            sum(argument, context.log)
        }
        SnowflakeType::Number { .. } => {
            diverge(
                context,
                NUMERIC_RANGE,
                &function_construct(&kind.to_ascii_uppercase()),
            )
            .map_err(|refusal| refusal.reason)?;
            sum(argument, context.log)
        }
        SnowflakeType::Float => Err(
            "SUM of FLOAT depends on summation order, which the engines do not share".to_owned(),
        ),
        other => sum(other, context.log),
    }
}

/// TO_TIMESTAMP_TZ, TO_TIMESTAMP_LTZ and TRY_TO_TIMESTAMP_LTZ of text, read as a cast of it is.
fn zoned_text_call(
    name: &str,
    value: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let operand = convert_expression(value, site)?;
    if operand.data_type != SnowflakeType::Varchar {
        return Err(outside_conditions(
            construct.clone(),
            format!("{name} of {}", operand.data_type),
        ));
    }
    let to = match name.ends_with(LOCAL_ZONE_SUFFIX) {
        true => SnowflakeType::TimestampLtz { precision: 9 },
        false => SnowflakeType::TimestampTz { precision: 9 },
    };
    zoned_text(
        &operand,
        to,
        name.starts_with(TRY_PREFIX),
        (site.context, construct),
    )
    .map_err(|reason| outside_conditions(construct.clone(), reason))
}
