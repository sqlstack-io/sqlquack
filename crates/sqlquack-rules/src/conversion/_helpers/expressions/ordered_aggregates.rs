//! WITHIN GROUP aggregates: LISTAGG, ARRAY_AGG, PERCENTILE_CONT and PERCENTILE_DISC.

use serde_json::Value;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::blocks::main::string_literal::string_literal;
use sqlquack_duckdb::raises::main::raised::raised;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::objects::as_json;
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::strings::text_operand;
use crate::conversion::_helpers::expressions::windows::order_item;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Emitted, Literal};
use crate::conversion::_helpers::support::site::{Placement, Site};
use crate::conversion::_helpers::support::tree::{
    Payload, field, flag, function_construct, list, missing, node_construct, outside_conditions,
    payload_object, require_understood,
};
use crate::conversion::constants::{
    ARRAY_AGG_RULE, LISTAGG_RULE, PERCENTILE_DECIMALS, PERCENTILE_EXTRA_DIGITS,
    PERCENTILE_FRACTION_DIGITS, PERCENTILE_INTERPOLATION, PERCENTILE_MAX_PRECISION,
    PERCENTILE_RULE,
};

/// An aggregate WITHIN GROUP (ORDER BY ...), alone or under `over`.
pub(crate) fn within_group(
    payload: &Value,
    site: Site<'_>,
    over: Option<&str>,
) -> Result<Emitted, Refusal> {
    let construct = node_construct("within_group");
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "order_by"], &construct)?;
    let inner = field(payload, "this", &construct)?;
    let call = inner
        .get("aggregate_function")
        .and_then(Value::as_object)
        .ok_or_else(|| missing(construct.clone(), "WITHIN GROUP of this call".to_owned()))?;
    let name = call
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_uppercase();
    let function = function_construct(&name);
    require_understood(call, &["args", "distinct", "name"], &function)?;
    let ordered = Ordered {
        call,
        order: list(payload, "order_by"),
        site: site.placed(Placement::ROW),
        over: over.map_or_else(String::new, |over| format!(" OVER ({over})")),
    };
    match name.as_str() {
        "LISTAGG" => ordered.listagg(),
        "ARRAY_AGG" => ordered.array_agg(),
        "PERCENTILE_CONT" => ordered.percentile(&name, true),
        "PERCENTILE_DISC" => ordered.percentile(&name, false),
        _ => Err(missing(
            function,
            format!("no rule converts {name} WITHIN GROUP"),
        )),
    }
}

struct Ordered<'a> {
    call: &'a Payload,
    order: &'a [Value],
    site: Site<'a>,
    over: String,
}

impl Ordered<'_> {
    /// LISTAGG of VARCHAR with a literal delimiter: the empty string when no value is non-NULL.
    fn listagg(&self) -> Result<Emitted, Refusal> {
        let construct = function_construct("LISTAGG");
        let log = self.site.context.log;
        let _rule = log.rule(LISTAGG_RULE);
        let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
        let (value, delimiter) = match list(self.call, "args") {
            [value] => (value, String::new()),
            [value, delimiter] => match convert_expression(delimiter, self.site)?.literal {
                Some(Literal::Text(text)) => (value, text),
                _ => return Err(refuse("a LISTAGG delimiter other than a text literal")),
            },
            _ => return Err(refuse("LISTAGG takes a value and a delimiter")),
        };
        let value = text_operand(value, &construct, self.site)?;
        let mut keys: Vec<String> = Vec::new();
        for item in self.order {
            keys.push(order_item(item, self.site)?);
        }
        let distinct = flag(self.call, "distinct");
        if distinct && !self.order_is_value(&value)? {
            return Err(refuse("LISTAGG DISTINCT ordered by anything but its value"));
        }
        let distinct = if distinct { "DISTINCT " } else { "" };
        Ok(Emitted::new(
            format!(
                "coalesce(string_agg({distinct}{value}, {} ORDER BY {}){}, '')",
                string_literal(&delimiter, log),
                keys.join(", "),
                self.over
            ),
            SnowflakeType::Varchar,
        ))
    }

    /// ARRAY_AGG of the non-NULL values in order, as a JSON array; `[]` when there are none.
    fn array_agg(&self) -> Result<Emitted, Refusal> {
        let construct = function_construct("ARRAY_AGG");
        let log = self.site.context.log;
        let _rule = log.rule(ARRAY_AGG_RULE);
        let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
        if !self.over.is_empty() {
            return Err(refuse("ARRAY_AGG as a window function"));
        }
        let [value] = list(self.call, "args") else {
            return Err(refuse("ARRAY_AGG takes one value"));
        };
        let emitted = convert_expression(value, self.site)?;
        let mut keys: Vec<String> = Vec::new();
        for item in self.order {
            keys.push(order_item(item, self.site)?);
        }
        let distinct = flag(self.call, "distinct");
        let text = emitted.data_type == SnowflakeType::Varchar;
        if distinct && !(text && self.order_is_value(&emitted.sql)?) {
            return Err(refuse(
                "ARRAY_AGG DISTINCT of other than text ordered by its value",
            ));
        }
        let present = emitted.sql.clone();
        let element = match text {
            true => emitted.sql,
            false => as_json(emitted, &construct, self.site.context)?,
        };
        let distinct = if distinct { "DISTINCT " } else { "" };
        Ok(Emitted::new(
            format!(
                "coalesce(CAST(to_json(list({distinct}{element} ORDER BY {}) \
                 FILTER (WHERE {present} IS NOT NULL)) AS JSON), CAST('[]' AS JSON))",
                keys.join(", ")
            ),
            SnowflakeType::SemiStructured,
        ))
    }

    fn order_is_value(&self, value: &str) -> Result<bool, Refusal> {
        let construct = node_construct("ordered");
        let [item] = self.order else {
            return Ok(false);
        };
        let key = convert_expression(
            field(payload_object(item, &construct)?, "this", &construct)?,
            self.site,
        )?;
        Ok(key.sql == value)
    }

    /// PERCENTILE_CONT or _DISC of one NUMBER key at a literal fraction of up to three decimals.
    fn percentile(&self, name: &str, continuous: bool) -> Result<Emitted, Refusal> {
        let construct = function_construct(name);
        let log = self.site.context.log;
        let _rule = log.rule(PERCENTILE_RULE);
        let refuse = |reason: String| outside_conditions(construct.clone(), reason);
        if flag(self.call, "distinct") {
            return Err(refuse(format!("{name} with DISTINCT")));
        }
        let [fraction] = list(self.call, "args") else {
            return Err(refuse(format!("{name} takes one fraction")));
        };
        let fraction = match convert_expression(fraction, self.site)?.literal {
            Some(Literal::Number(digits)) => thousandths(&digits).ok_or_else(|| {
                refuse(format!(
                    "the fraction {digits} is not 0 to 1 in thousandths"
                ))
            })?,
            _ => return Err(refuse(format!("a {name} fraction other than a literal"))),
        };
        let [item] = self.order else {
            return Err(refuse(format!("{name} ordered by other than one key")));
        };
        let ordered = payload_object(item, &node_construct("ordered"))?;
        let key = convert_expression(field(ordered, "this", &construct)?, self.site)?;
        let SnowflakeType::Number { precision, scale } = key.data_type else {
            return Err(refuse(format!("{name} of a {} key", key.data_type)));
        };
        let descending = flag(ordered, "desc");
        let (operand, data_type) = match continuous {
            true if precision + PERCENTILE_EXTRA_DIGITS > PERCENTILE_MAX_PRECISION => {
                return Err(refuse(format!(
                    "PERCENTILE_CONT of NUMBER({precision},{scale}) overflows NUMBER(38)"
                )));
            }
            true => {
                diverge(self.site.context, PERCENTILE_INTERPOLATION, &construct)?;
                let widened = SnowflakeType::Number {
                    precision: precision + PERCENTILE_EXTRA_DIGITS,
                    scale: scale + PERCENTILE_EXTRA_DIGITS,
                };
                (cast(&key.sql, widened, log).map_err(refuse)?, widened)
            }
            false if descending => {
                return Err(refuse("PERCENTILE_DISC in descending order".to_owned()));
            }
            false => (key.sql, key.data_type),
        };
        let fraction = match descending {
            true => PERCENTILE_DECIMALS - fraction,
            false => fraction,
        };
        let function = if continuous {
            "quantile_cont"
        } else {
            "quantile_disc"
        };
        let sql = cast(
            &format!(
                "{function}({operand}, {fraction}/{PERCENTILE_DECIMALS}){}",
                self.over
            ),
            data_type,
            log,
        )
        .map_err(refuse)?;
        let sql = match continuous {
            true => format!(
                "(CASE WHEN {sql} IS NULL OR list_contains(list({operand}){over}, {sql}) THEN {sql} \
                 ELSE {} END)",
                raised(&format!(
                    "{PERCENTILE_INTERPOLATION}: PERCENTILE_CONT between two different values, \
                     which Snowflake interpolates inexactly"
                )),
                over = self.over
            ),
            false => sql,
        };
        Ok(Emitted::new(sql, data_type))
    }
}

/// A literal from 0 to 1 with at most three decimals, in thousandths.
fn thousandths(digits: &str) -> Option<u32> {
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let plain = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if whole.is_empty() && fraction.is_empty()
        || !plain(whole)
        || !plain(fraction)
        || fraction.len() > PERCENTILE_FRACTION_DIGITS
    {
        return None;
    }
    let padded = format!(
        "{whole}{fraction:0<width$}",
        width = PERCENTILE_FRACTION_DIGITS
    );
    let value: u32 = match padded.trim_start_matches('0') {
        "" => 0,
        significant if significant.len() > PERCENTILE_FRACTION_DIGITS + 1 => return None,
        significant => match significant.parse() {
            Ok(value) => value,
            Err(_) => return None,
        },
    };
    (value <= PERCENTILE_DECIMALS).then_some(value)
}
