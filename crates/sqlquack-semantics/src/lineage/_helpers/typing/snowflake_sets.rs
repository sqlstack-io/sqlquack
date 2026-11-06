//! Snowflake's set-operation types, directional as probed; OBJECT, never probed, stays untyped.

use std::rc::Rc;

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::main::cast_target::cast_target;
use sqlquack_snowflake::typing::models::{CastTarget, SnowflakeType};

use crate::dialects::main::canonical_type::canonical_type;
use crate::lineage::constants::{
    SNOWFLAKE_ARRAY_TYPE, SNOWFLAKE_MAX_PRECISION, SNOWFLAKE_TEXT_NUMBER, SNOWFLAKE_VARIANT_TYPE,
};
use crate::lineage::models::Lineage;

/// The kinds the probes distinguish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Boolean,
    Number,
    Text,
    Date,
    TimestampLtz,
    TimestampNtz,
    TimestampTz,
    Time,
    Variant,
    Array,
}

impl Kind {
    fn temporal(self) -> bool {
        matches!(
            self,
            Kind::Date | Kind::TimestampLtz | Kind::TimestampNtz | Kind::TimestampTz | Kind::Time
        )
    }
}

/// A number's precision and scale, or FLOAT.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Numeric {
    Fixed(u32, u32),
    Float,
}

/// A branch's type as the probes read it; `numeric` is unknown when the spelling hides it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SetType {
    kind: Kind,
    numeric: Option<Numeric>,
    precision: u32,
}

/// The type Snowflake gives a set operation's column, its branches' lineage in order.
pub(crate) fn snowflake_set_type(branches: &[&Lineage]) -> Option<Rc<str>> {
    let mut folded: Option<SetType> = None;
    for branch in branches {
        let own: SetType = set_type(branch.data_type.as_deref()?)?;
        folded = Some(match folded {
            None => own,
            Some(current) => fold(current, own)?,
        });
    }
    spelled(folded?).map(Rc::from)
}

fn set_type(spelling: &str) -> Option<SetType> {
    let target: CastTarget = match cast_target(spelling, &ProvenanceLog::default()) {
        Ok(target) => target,
        Err(_unknown) => return None,
    };
    let explicit: bool = spelling.contains('(');
    let (kind, numeric, precision): (Kind, Option<Numeric>, u32) = match target.data_type {
        SnowflakeType::Boolean => (Kind::Boolean, None, 0),
        SnowflakeType::Number { precision, scale } => (
            Kind::Number,
            explicit.then_some(Numeric::Fixed(u32::from(precision), u32::from(scale))),
            0,
        ),
        SnowflakeType::Float => (Kind::Number, Some(Numeric::Float), 0),
        SnowflakeType::Varchar => (Kind::Text, None, 0),
        SnowflakeType::Date => (Kind::Date, None, 0),
        SnowflakeType::Time { precision } => (Kind::Time, None, u32::from(precision)),
        SnowflakeType::TimestampNtz { precision } => {
            (Kind::TimestampNtz, None, u32::from(precision))
        }
        SnowflakeType::TimestampTz { precision } => (Kind::TimestampTz, None, u32::from(precision)),
        SnowflakeType::TimestampLtz { precision } => {
            (Kind::TimestampLtz, None, u32::from(precision))
        }
        SnowflakeType::SemiStructured
            if canonical_type(spelling).starts_with(SNOWFLAKE_ARRAY_TYPE) =>
        {
            (Kind::Array, None, 0)
        }
        SnowflakeType::SemiStructured if canonical_type(spelling) == SNOWFLAKE_VARIANT_TYPE => {
            (Kind::Variant, None, 0)
        }
        SnowflakeType::SemiStructured => return None,
        SnowflakeType::Binary | SnowflakeType::Null => return None,
    };
    Some(SetType {
        kind,
        numeric,
        precision,
    })
}

/// Whether a later branch of kind `later` binds under a first branch of kind `first`.
fn compatible(first: Kind, later: Kind) -> bool {
    use Kind::Variant;
    use Kind::{Array, Boolean, Date, Number, Text, Time, TimestampLtz, TimestampNtz, TimestampTz};
    if first == later {
        return true;
    }
    match (first, later) {
        (Boolean, Number) | (Boolean | Number, Text) => true,
        (Text, Boolean | Variant | Number) => true,
        (Text, value) | (value, Text) if value.temporal() => true,
        (Boolean | Date | Time, Variant) => true,
        (Number | TimestampLtz | TimestampNtz | TimestampTz, Variant) => true,
        (Variant, Boolean | Number | Array) | (Array, Variant) => true,
        (Date, TimestampLtz | TimestampNtz | TimestampTz)
        | (TimestampLtz | TimestampNtz | TimestampTz, Date)
        | (TimestampTz, TimestampNtz)
        | (TimestampLtz, TimestampNtz | TimestampTz)
        | (TimestampNtz | TimestampTz, TimestampLtz) => true,
        _ => false,
    }
}

fn fold(first: SetType, later: SetType) -> Option<SetType> {
    if !compatible(first.kind, later.kind) {
        return None;
    }
    let text_number: Option<Numeric> = Some(Numeric::Fixed(
        SNOWFLAKE_TEXT_NUMBER.0,
        SNOWFLAKE_TEXT_NUMBER.1,
    ));
    let mut result: SetType = match (first.kind, later.kind) {
        (Kind::Number, Kind::Number) => SetType {
            numeric: combined(first.numeric, later.numeric),
            ..first
        },
        (Kind::Number, Kind::Text) => SetType {
            numeric: combined(first.numeric, text_number),
            ..first
        },
        (Kind::Text, Kind::Number) => SetType {
            numeric: combined(later.numeric, text_number),
            ..later
        },
        (Kind::Number, Kind::Variant) => later,
        (Kind::Text, kind) if kind.temporal() => later,
        (Kind::Date, Kind::TimestampLtz | Kind::TimestampNtz | Kind::TimestampTz) => later,
        _ => first,
    };
    if first.kind.temporal() && later.kind.temporal() {
        result.precision = first.precision.max(later.precision);
    }
    Some(result)
}

/// Two numbers combined: FLOAT wins; fixed numbers keep the widest integer part and scale.
fn combined(left: Option<Numeric>, right: Option<Numeric>) -> Option<Numeric> {
    match (left, right) {
        (Some(Numeric::Float), _) | (_, Some(Numeric::Float)) => Some(Numeric::Float),
        (
            Some(Numeric::Fixed(left_precision, left_scale)),
            Some(Numeric::Fixed(right_precision, right_scale)),
        ) => {
            let scale: u32 = left_scale.max(right_scale).min(SNOWFLAKE_MAX_PRECISION);
            let integral: u32 = left_precision
                .saturating_sub(left_scale)
                .max(right_precision.saturating_sub(right_scale));
            Some(Numeric::Fixed(
                (integral + scale).min(SNOWFLAKE_MAX_PRECISION),
                scale,
            ))
        }
        _ => None,
    }
}

fn spelled(folded: SetType) -> Option<String> {
    Some(match folded.kind {
        Kind::Boolean => "BOOLEAN".to_owned(),
        Kind::Number => match folded.numeric? {
            Numeric::Fixed(precision, scale) => format!("NUMBER({precision},{scale})"),
            Numeric::Float => "FLOAT".to_owned(),
        },
        Kind::Text => "VARCHAR".to_owned(),
        Kind::Date => "DATE".to_owned(),
        Kind::TimestampLtz => format!("TIMESTAMP_LTZ({})", folded.precision),
        Kind::TimestampNtz => format!("TIMESTAMP_NTZ({})", folded.precision),
        Kind::TimestampTz => format!("TIMESTAMP_TZ({})", folded.precision),
        Kind::Time => format!("TIME({})", folded.precision),
        Kind::Variant => SNOWFLAKE_VARIANT_TYPE.to_owned(),
        Kind::Array => SNOWFLAKE_ARRAY_TYPE.to_owned(),
    })
}
