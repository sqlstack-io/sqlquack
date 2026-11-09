//! WITHIN GROUP aggregates: LISTAGG, ARRAY_AGG, PERCENTILE_CONT and PERCENTILE_DISC.

use serde_json::Value;
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_blocks::blocks::main::cast::cast;
use sqlquack_blocks::blocks::main::string_literal::string_literal;
use sqlquack_blocks::raises::main::raised::raised;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::held::{builds_held, built, leaf};
use crate::conversion::_helpers::expressions::scalar::{convert_expression, convert_held};
use crate::conversion::_helpers::expressions::strings::text_operand;
use crate::conversion::_helpers::expressions::windows::{order_item, order_key};
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Context, Emitted, Literal, VariantRoot};
use crate::conversion::_helpers::support::site::{Placement, Site};
use crate::conversion::_helpers::support::tree::{
    Payload, field, flag, function_construct, list, node_construct, payload_object,
    require_understood,
};
use crate::conversion::_helpers::support::typed_leaves::compared_value;
use crate::conversion::constants::DUPLICATE_VARIANTS;
use crate::conversion::constants::{
    ARRAY_AGG_RULE, LISTAGG_RULE, PERCENTILE_DECIMALS, PERCENTILE_EXTRA_DIGITS,
    PERCENTILE_FRACTION_DIGITS, PERCENTILE_MAX_PRECISION, PERCENTILE_RULE,
};
use sqlquack_policy::policy::constants::{
    ARRAY_AGG_ORDER, LISTAGG_ORDER, PERCENTILE_INTERPOLATION,
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
        by: Ordering::Written,
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
    by: Ordering,
    site: Site<'a>,
    over: String,
}

/// How ARRAY_AGG orders its elements.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ordering {
    /// By the WITHIN GROUP keys as written.
    Written,
    /// Ascending by the value itself, as ARRAY_SORT of the result orders it.
    Value,
    /// In no defined order, so only a result of at most one distinct element converts.
    Unordered,
}

/// ARRAY_AGG without WITHIN GROUP, whose order neither engine defines.
pub(crate) fn unordered_array_agg(call: &Payload, site: Site<'_>) -> Result<Emitted, Refusal> {
    Ordered {
        call,
        order: &[],
        by: Ordering::Unordered,
        site: site.placed(Placement::ROW),
        over: String::new(),
    }
    .array_agg()
}

/// ARRAY_SORT of ARRAY_AGG without WITHIN GROUP: the values ascending, text or whole numbers.
pub(crate) fn sorted_array_agg(call: &Payload, site: Site<'_>) -> Result<Emitted, Refusal> {
    Ordered {
        call,
        order: &[],
        by: Ordering::Value,
        site: site.placed(Placement::ROW),
        over: String::new(),
    }
    .array_agg()
}

/// The call of `node` when it is ARRAY_AGG without WITHIN GROUP.
pub(crate) fn unordered_array_agg_call(node: &Value) -> Option<&Payload> {
    let call = node.get("aggregate_function")?.as_object()?;
    let name = call.get("name")?.as_str()?;
    name.eq_ignore_ascii_case("ARRAY_AGG").then_some(call)
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
        let by_value = self.order_is_value(&value)?;
        if distinct && !by_value {
            return Err(refuse("LISTAGG DISTINCT ordered by anything but its value"));
        }
        let listed = format!(
            "coalesce(string_agg({}{value}, {} ORDER BY {}){}, '')",
            if distinct { "DISTINCT " } else { "" },
            string_literal(&delimiter, log),
            keys.join(", "),
            self.over
        );
        if by_value {
            return Ok(Emitted::new(listed, SnowflakeType::Varchar));
        }
        diverge(self.site.context, LISTAGG_ORDER, &construct)?;
        let mut tie_keys: Vec<String> = Vec::new();
        for (position, item) in self.order.iter().enumerate() {
            tie_keys.push(format!("k{position} := {}", order_key(item, self.site)?));
        }
        Ok(Emitted::new(
            tied_listagg(&listed, &value, (&tie_keys, &self.over)),
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
        let emitted = convert_held(value, self.site)?;
        let text = emitted.data_type == SnowflakeType::Varchar;
        let mut keys: Vec<String> = Vec::new();
        for item in self.order {
            keys.push(order_item(item, self.site)?);
        }
        if self.by == Ordering::Value {
            if !matches!(
                emitted.data_type,
                SnowflakeType::Varchar | SnowflakeType::Number { scale: 0, .. }
            ) {
                return Err(refuse(
                    "ARRAY_SORT of ARRAY_AGG of other than text or whole numbers",
                ));
            }
            keys.push(format!("{} ASC", emitted.sql));
        }
        let distinct = flag(self.call, "distinct");
        if distinct && emitted.data_type == SnowflakeType::SemiStructured {
            return Err(refuse(&format!("ARRAY_AGG DISTINCT {DUPLICATE_VARIANTS}")));
        }
        if distinct
            && self.by == Ordering::Written
            && !(text && self.order_is_value(&emitted.sql)?)
        {
            return Err(refuse(
                "ARRAY_AGG DISTINCT of other than text ordered by its value",
            ));
        }
        let present = emitted.sql.clone();
        let variant_element = emitted.data_type == SnowflakeType::SemiStructured;
        let held = builds_held(std::slice::from_ref(&emitted));
        if self.by == Ordering::Unordered {
            compared_value(&emitted, &construct, self.site.context)?;
        }
        let element = match text {
            true => emitted.sql,
            false => leaf(emitted, held, &construct, self.site.context)?,
        };
        let distinct = if distinct { "DISTINCT " } else { "" };
        let order = match keys.is_empty() {
            true => String::new(),
            false => format!(" ORDER BY {}", keys.join(", ")),
        };
        let array = format!(
            "coalesce(CAST(to_json(list({distinct}{element}{order}) \
             FILTER (WHERE {present} IS NOT NULL)) AS JSON), CAST('[]' AS JSON))"
        );
        let array = match self.by {
            Ordering::Unordered if !held && !variant_element => {
                diverge(self.site.context, ARRAY_AGG_ORDER, &construct)?;
                let sorted = format!(
                    "coalesce(CAST(to_json(list_sort(list({distinct}{element}) \
                     FILTER (WHERE {present} IS NOT NULL))) AS JSON), CAST('[]' AS JSON))"
                );
                return Ok(Emitted {
                    unordered: true,
                    ..built(sorted, held, VariantRoot::Container, self.site.context)
                });
            }
            Ordering::Unordered => {
                diverge(self.site.context, ARRAY_AGG_ORDER, &construct)?;
                unordered_array(&element, &present, distinct)
            }
            Ordering::Written if distinct.is_empty() => {
                diverge(self.site.context, ARRAY_AGG_ORDER, &construct)?;
                let mut tie_keys: Vec<String> = Vec::new();
                for (position, item) in self.order.iter().enumerate() {
                    tie_keys.push(format!("k{position} := {}", order_key(item, self.site)?));
                }
                tied_array(&element, &present, (&tie_keys, &order))
            }
            Ordering::Written | Ordering::Value => array,
        };
        Ok(built(
            array,
            held,
            VariantRoot::Container,
            self.site.context,
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

/// ARRAY_AGG of `element` in `order`, raising where rows tied on `tie_keys` hold different values.
fn tied_array(element: &str, present: &str, (tie_keys, order): (&[String], &str)) -> String {
    let tied = raised(&format!(
        "{ARRAY_AGG_ORDER}: ARRAY_AGG WITHIN GROUP of different values tied on its order"
    ));
    format!(
        "list_transform([list(struct_pack(k := struct_pack({}), v := {element}){order}) \
         FILTER (WHERE {present} IS NOT NULL)], lambda array_agg_rows: CASE WHEN array_agg_rows \
         IS NULL THEN CAST('[]' AS JSON) WHEN len(list_distinct(list_transform(array_agg_rows, \
         lambda array_agg_row: array_agg_row['k']))) = len(list_distinct(array_agg_rows)) THEN \
         CAST(to_json(list_transform(array_agg_rows, lambda array_agg_row: array_agg_row['v'])) AS JSON) \
         ELSE CAST({tied} AS JSON) END)[1]",
        tie_keys.join(", ")
    )
}

/// `listed`, raising where rows tied on `tie_keys` hold different non-NULL values of `value`.
fn tied_listagg(listed: &str, value: &str, (tie_keys, over): (&[String], &str)) -> String {
    let tied = raised(&format!(
        "{LISTAGG_ORDER}: LISTAGG of different values tied on its order"
    ));
    let keys = format!("struct_pack({})", tie_keys.join(", "));
    let count = |element: &str| {
        format!(
            "coalesce(len(list_distinct(list({element}) FILTER (WHERE {value} IS NOT NULL){over})), 0)"
        )
    };
    format!(
        "CASE WHEN {} = {} THEN {listed} ELSE CAST({tied} AS VARCHAR) END",
        count(&keys),
        count(&format!("struct_pack(k := {keys}, v := {value})"))
    )
}

/// An unordered array read where order matters: raising for several distinct elements.
pub(crate) fn order_checked(emitted: Emitted, context: &Context<'_>) -> Result<Emitted, Refusal> {
    let construct = function_construct("ARRAY_AGG");
    if emitted.data_type != SnowflakeType::SemiStructured || emitted.is_variant_held() {
        return Err(outside_conditions(
            construct,
            "a value that depends on the order of an ARRAY_AGG, which Snowflake leaves undefined"
                .to_owned(),
        ));
    }
    diverge(context, ARRAY_AGG_ORDER, &construct)?;
    let unordered = raised(&format!(
        "{ARRAY_AGG_ORDER}: ARRAY_AGG of several values in no defined order"
    ));
    Ok(Emitted {
        sql: format!(
            "list_transform([{}], lambda ordered_array: CASE WHEN ordered_array IS NULL \
             OR len(list_distinct(CAST(ordered_array AS JSON[]))) <= 1 THEN ordered_array \
             ELSE CAST({unordered} AS JSON) END)[1]",
            emitted.sql
        ),
        unordered: false,
        ..emitted
    })
}

/// An unordered ARRAY_AGG of `element` where `present`, raising for more than one distinct value.
fn unordered_array(element: &str, present: &str, distinct: &str) -> String {
    let unordered = raised(&format!(
        "{ARRAY_AGG_ORDER}: ARRAY_AGG of several values in no defined order"
    ));
    let listed = match distinct.is_empty() {
        true => "array_agg_values",
        false => "list_distinct(array_agg_values)",
    };
    format!(
        "list_transform([list({element}) FILTER (WHERE {present} IS NOT NULL)], \
         lambda array_agg_values: CASE WHEN array_agg_values IS NULL THEN CAST('[]' AS JSON) \
         WHEN len(list_distinct(array_agg_values)) <= 1 THEN CAST(to_json({listed}) AS JSON) \
         ELSE CAST({unordered} AS JSON) END)[1]"
    )
}
