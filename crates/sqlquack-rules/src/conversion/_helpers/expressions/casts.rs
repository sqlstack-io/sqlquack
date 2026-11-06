//! CAST, `::` and TRY_CAST.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::provenance::models::{ProvenanceLog, RuleScope};
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::boolean_to_number::boolean_to_number;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::blocks::main::number_to_boolean::number_to_boolean;
use sqlquack_duckdb::blocks::main::string_literal::string_literal;
use sqlquack_duckdb::blocks::main::text_to_boolean::text_to_boolean;
use sqlquack_duckdb::blocks::main::text_to_float::text_to_float;
use sqlquack_duckdb::blocks::main::text_to_number::text_to_number;
use sqlquack_duckdb::blocks::main::varchar_length_check::varchar_length_check;
use sqlquack_duckdb::blocks::main::varchar_length_try::varchar_length_try;
use sqlquack_duckdb::floats::constants::FLOAT_DECIMAL_DIGITS;
use sqlquack_duckdb::floats::main::float_to_decimal::float_to_decimal;
use sqlquack_duckdb::floats::main::float_to_text::float_to_text;
use sqlquack_duckdb::raises::main::raised::raised;
use sqlquack_duckdb::semi_structured::constants::JSON_FIDELITY;
use sqlquack_duckdb::semi_structured::main::json_text::json_text;
use sqlquack_duckdb::temporal::main::text_to_timestamp::text_to_timestamp;
use sqlquack_duckdb::temporal::main::text_to_zoned::text_to_zoned;
use sqlquack_duckdb::temporal::main::utc_to_zone::utc_to_zone;
use sqlquack_duckdb::temporal::main::utc_wall_time::utc_wall_time;
use sqlquack_duckdb::text::main::binary_length_check::binary_length_check;
use sqlquack_duckdb::text::main::hex_bytes::hex_bytes;
use sqlquack_duckdb::times::main::text_to_time::text_to_time;
use sqlquack_duckdb::times::main::time_text::time_text;
use sqlquack_duckdb::times::main::timestamp_time::timestamp_time;
use sqlquack_duckdb::times::main::truncate_time::truncate_time;
use sqlquack_duckdb::times::models::TimeFormat;
use sqlquack_duckdb::variants::main::json_kind_cast::json_kind_cast;
use sqlquack_duckdb::zones::main::text_to_utc_zoned::text_to_utc_zoned;
use sqlquack_semantics::types::main::zoned_in_utc::zoned_in_utc;
use sqlquack_snowflake::typing::constants::CAST_TARGET;
use sqlquack_snowflake::typing::constants::STRUCTURED_UNSUPPORTED;
use sqlquack_snowflake::typing::main::cast_target::cast_target;
use sqlquack_snowflake::typing::models::{CastTarget, SnowflakeType};

use crate::conversion::_helpers::expressions::objects::as_json;
use crate::conversion::_helpers::expressions::operators::is_iso_date;
use crate::conversion::_helpers::expressions::scalar::{convert_expression, convert_held};
use crate::conversion::_helpers::expressions::semi_structured::{
    held_cast, variant_cast, variant_null,
};
use crate::conversion::_helpers::expressions::temporal::{
    is_temporal, literal_date, proven_time_text, temporal_cast, time_submicro, timestamp_literal,
    zoned_literal_utc,
};
use crate::conversion::_helpers::expressions::variants::{to_variant, written_array};
use crate::conversion::_helpers::support::declared::rely_on_declared;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::failure::text_failure;
use crate::conversion::_helpers::support::scope::{Context, Emitted, Literal, VariantRoot};
use crate::conversion::_helpers::support::shapes::type_family;
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    LITERAL_NUMBER, field, node_construct, node_kind, payload_object, require_understood,
};
use crate::conversion::constants::{
    CAST_BOOLEAN, CAST_HEX_BINARY, CAST_NUMERIC, CAST_PARSED_TEXT, CAST_SAME_TYPE, CAST_TEMPORAL,
    CAST_TEXT_LITERAL, CAST_TEXT_TIME, CAST_TEXT_TIMESTAMP, CAST_TO_TEXT, CAST_UTC_ZONED,
    FLOAT_SCALE_FACET, FLOAT_TO_DOUBLE, KEEP_NULL_CONSTRUCTOR, KIND_CAST_TARGETS,
    MICROSECOND_DIGITS, NTZ_OUTPUT_PATTERN, OBJECT_CONSTRUCTOR, OBJECT_TYPE_NAME, TARGET_FACET,
    TEXT_TYPE_PAIR, TEXT_TYPE_SPELLINGS, TIME_CAST, TIMESTAMP_TIME_RULE, TO_BOOLEAN_RULE,
    TO_DOUBLE_PAIR, TO_NUMBER_PAIR, TO_VARCHAR_PAIR, TO_VARIANT_RULE, VARIANT_CAST,
    VARIANT_KIND_CAST, VARIANT_NAME, VARIANT_TEXT_FUNCTION, VARIANT_TO_VARCHAR,
};
use sqlquack_policy::policy::constants::{
    DATE_TEXT_SPELLING, SESSION_TIMEZONE_UTC, TIMESTAMP_RANGE, TIMESTAMP_TEXT_ROUNDING,
    TIMESTAMP_TZ_OFFSET,
};

/// Serialised kinds of Snowflake's structured types.
const STRUCTURED_KINDS: &[&str] = &["array", "object", "map"];
/// Text Snowflake and DuckDB both read as `true`.
const TRUE_TEXT: &str = "true";
/// Text Snowflake and DuckDB both read as `false`.
const FALSE_TEXT: &str = "false";

/// A cast to a Snowflake type, or a refusal when the conversion's semantics are unproven.
pub(crate) fn convert_cast(
    kind: &str,
    payload: &Value,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let construct = node_construct(kind);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "to"], &construct)?;
    let to = field(payload, "to", &construct)?;
    if to
        .get("data_type")
        .and_then(Value::as_str)
        .is_some_and(|written| TEXT_TYPE_SPELLINGS.contains(&written))
    {
        site.context.log.equivalence(TEXT_TYPE_PAIR);
    }
    let type_name = data_type_name(to).ok_or_else(|| {
        missing(
            construct.clone(),
            "a cast target type has no name".to_owned(),
        )
    })?;
    if is_structured(to) {
        return Err(missing(
            construct,
            format!("casting to the structured type {type_name} {STRUCTURED_UNSUPPORTED}"),
        ));
    }
    let log = site.context.log;
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let target = cast_target(&type_name, &ProvenanceLog::default()).map_err(refuse)?;
    log.facet(TARGET_FACET, type_family(target.data_type));
    let operand_node = field(payload, "this", &construct)?;
    if let Some(digits) = number_literal_text(operand_node)
        && type_name != VARIANT_NAME
    {
        let _rule = cast_rule(CAST_NUMERIC, log);
        let sql = number_literal_cast(&digits, target.data_type, log).map_err(refuse)?;
        return Ok(Emitted::new(bounded(sql, target, log), target.data_type));
    }
    let trying = matches!(construct.name.as_str(), "try_cast");
    if target.data_type == SnowflakeType::Varchar
        && !trying
        && let Some(written) = written_array(operand_node, None, site)
    {
        let _rule = cast_rule(VARIANT_CAST, log);
        let written = written?;
        return Ok(match target.max_length {
            Some(max_length) => Emitted::new(
                varchar_length_check(&written.sql, max_length, log),
                SnowflakeType::Varchar,
            ),
            None => written,
        });
    }
    let operand = convert_held(operand_node, site)?;
    if operand.is_variant_held() {
        if type_name == OBJECT_TYPE_NAME
            && !built_object(operand_node)
            && operand.root != VariantRoot::Element
        {
            return Err(refuse(
                "a VARIANT built from a FLOAT cast to OBJECT, other than an OBJECT_CONSTRUCT or an \
                 element read from one, which Snowflake may reject as it compiles"
                    .to_owned(),
            ));
        }
        return held_cast(
            operand,
            (&type_name, target),
            trying,
            (site.context, &construct),
        );
    }
    if operand.data_type == SnowflakeType::Float && type_name == VARIANT_NAME && !trying {
        let _rule = cast_rule(TO_VARIANT_RULE, log);
        return to_variant(operand, &construct, site.context);
    }
    if let (SnowflakeType::Float, SnowflakeType::Number { scale, .. }) =
        (operand.data_type, target.data_type)
    {
        log.facet(FLOAT_SCALE_FACET, &scale.to_string());
    }
    if operand.data_type == SnowflakeType::Null
        && (type_name == VARIANT_NAME
            || !trying && KIND_CAST_TARGETS.iter().any(|(name, _)| *name == type_name))
    {
        let _rule = cast_rule(CAST_SAME_TYPE, log);
        return Ok(Emitted::new(
            variant_null(log),
            SnowflakeType::SemiStructured,
        ));
    }
    if let Some((_, kind)) = KIND_CAST_TARGETS
        .iter()
        .find(|(name, _)| *name == type_name)
        .filter(|_| operand.data_type == SnowflakeType::SemiStructured && !trying)
    {
        let _rule = cast_rule(VARIANT_KIND_CAST, log);
        return Ok(Emitted::new(
            json_kind_cast(&operand.sql, kind, log),
            SnowflakeType::SemiStructured,
        ));
    }
    if type_name == VARIANT_NAME
        && !trying
        && (matches!(
            operand.data_type,
            SnowflakeType::Varchar
                | SnowflakeType::Boolean
                | SnowflakeType::Number { scale: 0, .. }
                | SnowflakeType::Date
                | SnowflakeType::TimestampNtz { .. }
                | SnowflakeType::TimestampLtz { .. }
        ) || (matches!(operand.data_type, SnowflakeType::TimestampTz { .. })
            && operand.utc_offset))
    {
        let _rule = cast_rule(TO_VARIANT_RULE, log);
        return Ok(Emitted::new(
            as_json(operand, &construct, site.context)?,
            SnowflakeType::SemiStructured,
        ));
    }
    if operand.data_type == SnowflakeType::Varchar
        && matches!(
            target.data_type,
            SnowflakeType::TimestampTz { .. } | SnowflakeType::TimestampLtz { .. }
        )
    {
        return zoned_text(
            &operand,
            target.data_type,
            trying,
            (site.context, &construct),
        )
        .map_err(refuse);
    }
    if operand.data_type == SnowflakeType::SemiStructured {
        let sql = variant_cast(
            &operand,
            (&type_name, target),
            trying,
            (site.context, &construct),
        )
        .map_err(refuse)?;
        return Ok(Emitted::new(sql, target.data_type));
    }
    let sql = cast_sql(&operand, target, trying, (site.context, &construct)).map_err(refuse)?;
    Ok(Emitted {
        utc_offset: zoned_in_utc((operand.data_type, operand.utc_offset), target.data_type),
        declared: operand.declared.clone(),
        not_null: (operand.not_null || operand.literal.is_some()) && !trying,
        ..Emitted::new(sql, target.data_type)
    })
}

/// The SQL casting `operand` to the type named `type_name`, as CAST or TRY_CAST does.
pub(crate) fn cast_to(
    operand: &Emitted,
    type_name: &str,
    trying: bool,
    (context, construct): (&Context<'_>, &Construct),
) -> Result<String, String> {
    let target = cast_target(type_name, &ProvenanceLog::default())?;
    cast_sql(operand, target, trying, (context, construct))
}

/// Activates a cast rule, which relies on the target type `cast_target` resolved.
fn cast_rule<'a>(id: &str, log: &'a ProvenanceLog) -> RuleScope<'a> {
    let scope = log.rule(id);
    log.typing(CAST_TARGET);
    scope
}

fn cast_sql(
    operand: &Emitted,
    target: CastTarget,
    trying: bool,
    (context, construct): (&Context<'_>, &Construct),
) -> Result<String, String> {
    let log = context.log;
    let to = target.data_type;
    match (operand.data_type, to, &operand.literal) {
        (SnowflakeType::Null, _, _) => {
            let _rule = cast_rule(CAST_SAME_TYPE, log);
            Ok(bounded(cast("NULL", to, log)?, target, log))
        }
        (from @ SnowflakeType::Number { .. }, to, _) if trying && from == to => {
            let _rule = cast_rule(CAST_SAME_TYPE, log);
            Ok(operand.sql.clone())
        }
        (SnowflakeType::Varchar, SnowflakeType::Varchar, _) if trying => {
            let _rule = cast_rule(CAST_SAME_TYPE, log);
            Ok(match target.max_length {
                Some(max_length) => varchar_length_try(&operand.sql, max_length, log),
                None => operand.sql.clone(),
            })
        }
        (SnowflakeType::Varchar, SnowflakeType::Varchar, _) => {
            let _rule = cast_rule(CAST_SAME_TYPE, log);
            Ok(bounded(operand.sql.clone(), target, log))
        }
        (SnowflakeType::Varchar, SnowflakeType::Number { .. }, _) => {
            let _rule = cast_rule(CAST_PARSED_TEXT, log);
            text_to_number(&operand.sql, to, text_failure(trying), log)
        }
        (SnowflakeType::Varchar, SnowflakeType::Float, _) => {
            let _rule = cast_rule(CAST_PARSED_TEXT, log);
            Ok(text_to_float(&operand.sql, text_failure(trying), log))
        }
        (SnowflakeType::Varchar, SnowflakeType::Boolean, _) => {
            let _rule = cast_rule(CAST_PARSED_TEXT, log);
            Ok(text_to_boolean(&operand.sql, text_failure(trying), log))
        }
        (SnowflakeType::Varchar, SnowflakeType::Binary, _) => {
            let _rule = cast_rule(CAST_HEX_BINARY, log);
            let bytes = hex_bytes(&operand.sql, trying, log);
            match (target.max_length, trying) {
                (None, _) => Ok(bytes),
                (Some(max_length), false) => Ok(binary_length_check(&bytes, max_length, log)),
                (Some(_), true) => Err(
                    "TRY_CAST to BINARY(n), whose result for longer bytes no recording shows"
                        .to_owned(),
                ),
            }
        }
        (SnowflakeType::Varchar, SnowflakeType::Time { precision }, literal) => {
            let _rule = cast_rule(CAST_TEXT_TIME, log);
            time_submicro((operand.literal.as_ref(), precision), construct, context)
                .map_err(|refusal| refusal.reason)?;
            if !matches!(literal, Some(Literal::Text(text)) if proven_time_text(text)) {
                diverge(context, DATE_TEXT_SPELLING, construct)
                    .map_err(|refusal| refusal.reason)?;
            }
            Ok(text_to_time(
                &operand.sql,
                (TimeFormat::Auto, precision),
                text_failure(trying),
                log,
            ))
        }
        (SnowflakeType::Time { precision: from }, SnowflakeType::Time { precision }, _) => {
            let _rule = cast_rule(TIME_CAST, log);
            Ok(match precision < from {
                true => truncate_time(&operand.sql, precision, log),
                false => operand.sql.clone(),
            })
        }
        (SnowflakeType::Time { .. }, SnowflakeType::Varchar, _) => {
            let _rule = cast_rule(TIME_CAST, log);
            Ok(bounded(time_text(&operand.sql, log), target, log))
        }
        (SnowflakeType::Varchar, _, Some(Literal::Text(text)))
            if is_temporal(to) && to != SnowflakeType::Date =>
        {
            let _rule = cast_rule(CAST_TEXT_LITERAL, log);
            timestamp_literal(text, to, (context, construct))
        }
        (SnowflakeType::Varchar, _, Some(Literal::Text(text))) => {
            let _rule = cast_rule(CAST_TEXT_LITERAL, log);
            text_literal_cast(text, to, log)
        }
        (SnowflakeType::Varchar, SnowflakeType::Date, _) => {
            let _rule = cast_rule(CAST_PARSED_TEXT, log);
            diverge(context, DATE_TEXT_SPELLING, construct).map_err(|refusal| refusal.reason)?;
            Ok(text_to_timestamp(
                &operand.sql,
                (text_failure(trying), true),
                log,
            ))
        }
        (SnowflakeType::Varchar, SnowflakeType::TimestampNtz { .. }, _) => {
            let _rule = cast_rule(CAST_TEXT_TIMESTAMP, log);
            diverge(context, TIMESTAMP_RANGE, construct).map_err(|refusal| refusal.reason)?;
            diverge(context, DATE_TEXT_SPELLING, construct).map_err(|refusal| refusal.reason)?;
            let parsed = text_to_timestamp(&operand.sql, (text_failure(trying), false), log);
            temporal_cast(
                &parsed,
                (SnowflakeType::TimestampNtz { precision: 9 }, to),
                (context, construct),
            )
        }
        (from, _, _) if trying => Err(format!(
            "TRY_CAST of {from} to {to} has no proven conversion"
        )),
        (SnowflakeType::Float, SnowflakeType::Varchar, _) => {
            let _rule = cast_rule(CAST_TO_TEXT, log);
            Ok(bounded(float_to_text(&operand.sql, log), target, log))
        }
        (SnowflakeType::Float, SnowflakeType::Number { scale, .. }, _) if scale > 0 => {
            let _rule = cast_rule(CAST_NUMERIC, log);
            diverge(context, FLOAT_DECIMAL_DIGITS, construct).map_err(|refusal| refusal.reason)?;
            float_to_decimal(&operand.sql, to, log)
        }
        (SnowflakeType::TimestampNtz { precision: 9 }, SnowflakeType::Varchar, _) => {
            let _rule = cast_rule(CAST_TO_TEXT, log);
            diverge(context, TIMESTAMP_TEXT_ROUNDING, construct)
                .map_err(|refusal| refusal.reason)?;
            let unproven = raised(&format!(
                "{TIMESTAMP_TEXT_ROUNDING}: a timestamp whose dropped digits past the millisecond \
                 decide between truncating and rounding, which no recording shows"
            ));
            Ok(bounded(
                format!(
                    "list_transform([{}], lambda ntz_text: CASE WHEN ((epoch_ns(ntz_text) % 1000000) \
                     + 1000000) % 1000000 >= 500000 THEN CAST({unproven} AS VARCHAR) \
                     ELSE strftime(ntz_text, {}) END)[1]",
                    operand.sql,
                    string_literal(NTZ_OUTPUT_PATTERN, log)
                ),
                target,
                log,
            ))
        }
        (SnowflakeType::TimestampLtz { precision }, SnowflakeType::TimestampNtz { .. }, _) => {
            let _rule = cast_rule(CAST_UTC_ZONED, log);
            diverge(context, SESSION_TIMEZONE_UTC, construct).map_err(|refusal| refusal.reason)?;
            nanosecond_range(to, (context, construct))?;
            temporal_cast(
                &utc_wall_time(&operand.sql, log),
                (SnowflakeType::TimestampNtz { precision }, to),
                (context, construct),
            )
        }
        (SnowflakeType::TimestampTz { precision }, SnowflakeType::Time { precision: 9 }, _)
            if operand.utc_offset =>
        {
            let _rule = cast_rule(CAST_UTC_ZONED, log);
            let _time = log.rule(TIMESTAMP_TIME_RULE);
            rely_on_declared(&operand.declared, construct, context)
                .map_err(|refusal| refusal.reason)?;
            Ok(timestamp_time(
                &utc_wall_time(&operand.sql, log),
                precision > MICROSECOND_DIGITS,
                log,
            ))
        }
        (
            SnowflakeType::TimestampTz { precision },
            SnowflakeType::TimestampNtz { .. } | SnowflakeType::Date,
            _,
        ) if operand.utc_offset => {
            let _rule = cast_rule(CAST_UTC_ZONED, log);
            rely_on_declared(&operand.declared, construct, context)
                .map_err(|refusal| refusal.reason)?;
            nanosecond_range(to, (context, construct))?;
            temporal_cast(
                &utc_wall_time(&operand.sql, log),
                (SnowflakeType::TimestampNtz { precision }, to),
                (context, construct),
            )
        }
        (
            SnowflakeType::TimestampTz { precision },
            SnowflakeType::TimestampNtz { .. } | SnowflakeType::Date,
            _,
        ) if operand.zone.is_some() => {
            let _rule = cast_rule(CAST_UTC_ZONED, log);
            let zone = operand.zone.as_deref().unwrap_or_default();
            let wall = SnowflakeType::TimestampNtz {
                precision: precision.min(MICROSECOND_DIGITS),
            };
            temporal_cast(
                &utc_to_zone(&utc_wall_time(&operand.sql, log), zone, wall, log),
                (wall, to),
                (context, construct),
            )
        }
        (from, _, _) if from != to && is_temporal(from) && is_temporal(to) => {
            let _rule = cast_rule(CAST_TEMPORAL, log);
            temporal_cast(&operand.sql, (from, to), (context, construct))
        }
        (from, _, _) => strict_cast(operand, from, target, log),
    }
}

/// Relies on timestamp_range for a naive target held in DuckDB nanoseconds, whose range is narrower.
fn nanosecond_range(
    target: SnowflakeType,
    (context, construct): (&Context<'_>, &Construct),
) -> Result<(), String> {
    match target {
        SnowflakeType::TimestampNtz { precision } if precision > MICROSECOND_DIGITS => {
            diverge(context, TIMESTAMP_RANGE, construct).map_err(|refusal| refusal.reason)
        }
        _ => Ok(()),
    }
}

/// Text cut to a VARCHAR(n) target's length raises an error, as in Snowflake.
fn bounded(sql: String, target: CastTarget, log: &ProvenanceLog) -> String {
    match target.max_length {
        Some(max_length) if target.data_type == SnowflakeType::Varchar => {
            varchar_length_check(&sql, max_length, log)
        }
        _ => sql,
    }
}

fn strict_cast(
    operand: &Emitted,
    from: SnowflakeType,
    target: CastTarget,
    log: &ProvenanceLog,
) -> Result<String, String> {
    let to = target.data_type;
    match (from, to) {
        (
            SnowflakeType::Number { .. } | SnowflakeType::Boolean | SnowflakeType::Date,
            SnowflakeType::Varchar,
        ) => {
            let _rule = cast_rule(CAST_TO_TEXT, log);
            Ok(bounded(cast(&operand.sql, to, log)?, target, log))
        }
        _ if from == to => {
            let _rule = cast_rule(CAST_SAME_TYPE, log);
            cast(&operand.sql, to, log)
        }
        (SnowflakeType::Number { .. }, SnowflakeType::Number { .. } | SnowflakeType::Float)
        | (SnowflakeType::Float, SnowflakeType::Number { scale: 0, .. }) => {
            let _rule = cast_rule(CAST_NUMERIC, log);
            cast(&operand.sql, to, log)
        }
        (SnowflakeType::Boolean, SnowflakeType::Number { .. }) => {
            let _rule = cast_rule(CAST_BOOLEAN, log);
            boolean_to_number(&operand.sql, to, log)
        }
        (SnowflakeType::Number { .. }, SnowflakeType::Boolean) => {
            let _rule = cast_rule(CAST_BOOLEAN, log);
            Ok(number_to_boolean(&operand.sql, log))
        }
        _ => Err(format!("casting {from} to {to} has no proven conversion")),
    }
}

/// TO_NUMBER, TO_DECIMAL, TO_NUMERIC and their TRY_ forms of text, as CAST or TRY_CAST to NUMBER.
pub(crate) fn to_number_call(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [value, sizes @ ..] = arguments else {
        return Err(refuse(format!("{name} without an argument")));
    };
    let mut digits: Vec<String> = Vec::with_capacity(sizes.len());
    for size in sizes {
        match number_literal_text(size) {
            Some(text) if is_unsigned_integer(&text) => digits.push(text),
            _ => {
                return Err(refuse(format!(
                    "{name} with a format or a non-literal size"
                )));
            }
        }
    }
    let type_name = match digits.as_slice() {
        [] => "NUMBER(38,0)".to_owned(),
        [precision] => format!("NUMBER({precision},0)"),
        [precision, scale] => format!("NUMBER({precision},{scale})"),
        _ => return Err(refuse(format!("{name} with these arguments"))),
    };
    let target = cast_target(&type_name, &ProvenanceLog::default()).map_err(refuse)?;
    let operand = convert_expression(value, site)?;
    let trying = name.starts_with("TRY_");
    if operand.data_type == SnowflakeType::SemiStructured && !operand.is_variant_held() && !trying {
        let sql = variant_cast(
            &operand,
            (&type_name, target),
            false,
            (site.context, construct),
        )
        .map_err(refuse)?;
        return Ok(Emitted::new(sql, target.data_type));
    }
    if !matches!(
        operand.data_type,
        SnowflakeType::Varchar | SnowflakeType::Null
    ) {
        return Err(refuse(format!("{name} of {}", operand.data_type)));
    }
    site.context.log.equivalence(TO_NUMBER_PAIR);
    let sql = cast_sql(&operand, target, trying, (site.context, construct)).map_err(refuse)?;
    Ok(Emitted::new(sql, target.data_type))
}

/// TO_DOUBLE and TRY_TO_DOUBLE of text, as CAST or TRY_CAST to FLOAT.
pub(crate) fn to_double_call(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [value] = arguments else {
        return Err(refuse(format!("{name} with a format or these arguments")));
    };
    let operand = convert_expression(value, site)?;
    if operand.data_type == SnowflakeType::Float && name.starts_with("TRY_") {
        let _rule = site.context.log.rule(FLOAT_TO_DOUBLE);
        return Ok(operand);
    }
    if !matches!(
        operand.data_type,
        SnowflakeType::Varchar | SnowflakeType::Null
    ) {
        return Err(refuse(format!("{name} of {}", operand.data_type)));
    }
    site.context.log.equivalence(TO_DOUBLE_PAIR);
    let target = cast_target("FLOAT", &ProvenanceLog::default()).map_err(refuse)?;
    let trying = name.starts_with("TRY_");
    let sql = cast_sql(&operand, target, trying, (site.context, construct)).map_err(refuse)?;
    Ok(Emitted::new(sql, SnowflakeType::Float))
}

/// TO_BOOLEAN and TRY_TO_BOOLEAN of text, as CAST and TRY_CAST to BOOLEAN.
pub(crate) fn to_boolean_call(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TO_BOOLEAN_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [value] = arguments else {
        return Err(refuse(format!("{name} with these arguments")));
    };
    let operand = convert_expression(value, site)?;
    if !matches!(
        operand.data_type,
        SnowflakeType::Varchar | SnowflakeType::Null
    ) {
        return Err(refuse(format!("{name} of {}", operand.data_type)));
    }
    let target = cast_target("BOOLEAN", &ProvenanceLog::default()).map_err(refuse)?;
    let trying = name.starts_with("TRY_");
    let sql = cast_sql(&operand, target, trying, (site.context, construct)).map_err(refuse)?;
    Ok(Emitted::new(sql, SnowflakeType::Boolean))
}

/// TO_VARCHAR and TO_CHAR without a format, as CAST to VARCHAR; TO_VARCHAR of a VARIANT as its text.
pub(crate) fn to_varchar_call(
    name: &str,
    value: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let operand = convert_held(value, site)?;
    if operand.is_variant_held() && name == VARIANT_TEXT_FUNCTION {
        let target = cast_target("VARCHAR", &ProvenanceLog::default()).map_err(refuse)?;
        return held_cast(
            operand,
            ("VARCHAR", target),
            false,
            (site.context, construct),
        );
    }
    if operand.is_variant_held() {
        return Err(refuse(format!("{name} of a VARIANT has no recording")));
    }
    if operand.data_type == SnowflakeType::SemiStructured {
        if name != VARIANT_TEXT_FUNCTION {
            return Err(refuse(format!("{name} of a VARIANT has no recording")));
        }
        let _rule = site.context.log.rule(VARIANT_TO_VARCHAR);
        diverge(site.context, JSON_FIDELITY, construct)?;
        return Ok(Emitted::new(
            json_text(&operand.sql, site.context.log),
            SnowflakeType::Varchar,
        ));
    }
    site.context.log.equivalence(TO_VARCHAR_PAIR);
    let target = cast_target("VARCHAR", &ProvenanceLog::default()).map_err(refuse)?;
    let sql = cast_sql(&operand, target, false, (site.context, construct)).map_err(refuse)?;
    Ok(Emitted::new(sql, SnowflakeType::Varchar))
}

fn is_unsigned_integer(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// The digits of a number literal, with a leading minus when negated.
pub(crate) fn number_literal_text(node: &Value) -> Option<String> {
    let (kind, payload) = node_kind(node)?;
    match kind {
        "neg" => number_literal_text(payload.get("this")?).map(|digits| format!("-{digits}")),
        "literal" => match payload.get("literal_type")?.as_str()? {
            LITERAL_NUMBER => payload.get("value")?.as_str().map(str::to_owned),
            _ => None,
        },
        _ => None,
    }
}

/// An integer literal as Snowflake writes it as text: no plus sign, leading zero or negative zero.
fn is_written_integer(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    is_unsigned_integer(digits) && !(digits.starts_with('0') && text.len() > 1)
}

fn is_plain_decimal(text: &str) -> bool {
    let body = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
    (!whole.is_empty() || !fraction.is_empty())
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_float_spelling(text: &str) -> bool {
    let (mantissa, exponent) = text
        .split_once(['e', 'E'])
        .map_or((text, None), |(mantissa, exponent)| {
            (mantissa, Some(exponent))
        });
    let exponent_ok = exponent.is_none_or(|exponent| {
        let digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    });
    is_plain_decimal(mantissa) && exponent_ok
}

fn with_leading_zero(text: &str) -> String {
    let (sign, unsigned) = text
        .strip_prefix('-')
        .map_or(("", text), |unsigned| ("-", unsigned));
    match unsigned.strip_prefix('.') {
        Some(fraction) => format!("{sign}0.{fraction}"),
        None => text.to_owned(),
    }
}

fn number_literal_cast(
    text: &str,
    to: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<String, String> {
    match to {
        SnowflakeType::Number { .. } if is_plain_decimal(text) => {
            cast(&with_leading_zero(text), to, log)
        }
        SnowflakeType::Float if is_float_spelling(text) => cast(&with_leading_zero(text), to, log),
        SnowflakeType::Varchar if is_written_integer(text) => Ok(string_literal(text, log)),
        _ => Err(format!(
            "casting the literal {text} to {to} has no proven conversion"
        )),
    }
}

fn text_literal_cast(text: &str, to: SnowflakeType, log: &ProvenanceLog) -> Result<String, String> {
    let literal = string_literal(text, log);
    match to {
        SnowflakeType::Number { .. } if is_plain_decimal(text) => cast(&literal, to, log),
        SnowflakeType::Float if is_float_spelling(text) => cast(&literal, to, log),
        SnowflakeType::Boolean if text.eq_ignore_ascii_case(TRUE_TEXT) => Ok("TRUE".to_owned()),
        SnowflakeType::Boolean if text.eq_ignore_ascii_case(FALSE_TEXT) => Ok("FALSE".to_owned()),
        SnowflakeType::Date if is_iso_date(text) => cast(&literal, to, log),
        SnowflakeType::Date => match literal_date(text) {
            Some(date) => cast(&string_literal(date, log), to, log),
            None => Err(format!(
                "casting the text '{text}' to {to} has no proven conversion"
            )),
        },
        other => Err(format!(
            "casting the text '{text}' to {other} has no proven conversion"
        )),
    }
}

fn with_arguments(base: &str, arguments: &[Option<u64>]) -> String {
    let present: Vec<String> = arguments.iter().flatten().map(u64::to_string).collect();
    match present.is_empty() {
        true => base.to_owned(),
        false => format!("{base}({})", present.join(",")),
    }
}

/// Whether a serialised type is a structured ARRAY, OBJECT or MAP, with element or field types.
fn is_structured(data_type: &Value) -> bool {
    data_type
        .get("data_type")
        .and_then(Value::as_str)
        .is_some_and(|kind| STRUCTURED_KINDS.contains(&kind))
}

/// The Snowflake spelling of a structured type's fields, `<name> <type> [NOT NULL]`.
fn field_names(fields: &[Value]) -> Option<Vec<String>> {
    let mut names: Vec<String> = Vec::new();
    for field in fields {
        let [name, data_type, not_null] = field.as_array()?.as_slice() else {
            return None;
        };
        let suffix = match not_null.as_bool()? {
            true => " NOT NULL",
            false => "",
        };
        names.push(format!(
            "{} {}{suffix}",
            name.as_str()?,
            data_type_name(data_type)?
        ));
    }
    Some(names)
}

/// The Snowflake spelling of a serialised cast target type.
pub(crate) fn data_type_name(data_type: &Value) -> Option<String> {
    let object = data_type.as_object()?;
    let number = |name: &str| object.get(name).and_then(Value::as_u64);
    let zoned = object
        .get("timezone")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Some(match object.get("data_type")?.as_str()? {
        "custom" => object.get("name")?.as_str()?.to_owned(),
        "decimal" => with_arguments("NUMBER", &[number("precision"), number("scale")]),
        "int" | "big_int" | "small_int" | "tiny_int" => "INT".to_owned(),
        "float" | "double" => "FLOAT".to_owned(),
        "var_char" | "string" => with_arguments("VARCHAR", &[number("length")]),
        "text" => "VARCHAR".to_owned(),
        "char" => with_arguments("CHAR", &[number("length")]),
        "boolean" => "BOOLEAN".to_owned(),
        "binary" | "var_binary" => with_arguments("BINARY", &[number("length")]),
        "date" => "DATE".to_owned(),
        "time" if !zoned => with_arguments("TIME", &[number("precision")]),
        "timestamp" if zoned => with_arguments("TIMESTAMP_TZ", &[number("precision")]),
        "timestamp" => with_arguments("TIMESTAMP", &[number("precision")]),
        "array" => format!("ARRAY({})", data_type_name(object.get("element_type")?)?),
        "map" => format!(
            "MAP({}, {})",
            data_type_name(object.get("key_type")?)?,
            data_type_name(object.get("value_type")?)?
        ),
        "object" => format!(
            "OBJECT({})",
            field_names(object.get("fields")?.as_array()?)?.join(", ")
        ),
        _ => return None,
    })
}

/// Whether `node` is an OBJECT_CONSTRUCT call, which Snowflake types as OBJECT.
fn built_object(node: &Value) -> bool {
    node.get("function")
        .and_then(|function| function.get("name"))
        .and_then(Value::as_str)
        .is_some_and(|name| [OBJECT_CONSTRUCTOR, KEEP_NULL_CONSTRUCTOR].contains(&name))
}

/// Text to TIMESTAMP_TZ or _LTZ; a TIMESTAMP_TZ holds +00:00, raising for any other offset.
pub(crate) fn zoned_text(
    operand: &Emitted,
    to: SnowflakeType,
    trying: bool,
    (context, construct): (&Context<'_>, &Construct),
) -> Result<Emitted, String> {
    let log = context.log;
    let zoned = matches!(to, SnowflakeType::TimestampTz { .. });
    if let Some(Literal::Text(text)) = &operand.literal
        && let Some(utc) = zoned_literal_utc(text)
    {
        let _rule = cast_rule(CAST_TEXT_LITERAL, log);
        let sql = timestamp_literal(text, to, (context, construct))?;
        return Ok(Emitted {
            utc_offset: zoned && utc,
            ..Emitted::new(sql, to)
        });
    }
    let _rule = cast_rule(CAST_TEXT_TIMESTAMP, log);
    diverge(context, TIMESTAMP_RANGE, construct).map_err(|refusal| refusal.reason)?;
    diverge(context, DATE_TEXT_SPELLING, construct).map_err(|refusal| refusal.reason)?;
    let instant = match zoned {
        true => {
            diverge(context, TIMESTAMP_TZ_OFFSET, construct).map_err(|refusal| refusal.reason)?;
            text_to_utc_zoned(&operand.sql, text_failure(trying), log)
        }
        false => text_to_zoned(&operand.sql, text_failure(trying), log),
    };
    let sql = temporal_cast(
        &instant,
        (SnowflakeType::TimestampNtz { precision: 9 }, to),
        (context, construct),
    )?;
    Ok(Emitted {
        utc_offset: zoned,
        ..Emitted::new(sql, to)
    })
}
