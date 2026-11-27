//! Databricks type names read from the tree, and the casts DuckDB performs the same way.

use serde_json::Value;
use sqlquack_diagnostics::classification::models::{Construct, ConstructFamily};
use sqlquack_diagnostics::refusals::main::missing::missing;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::support::tree::{field, understood};
use crate::databricks::conversion::constants::{
    DEFAULT_DECIMAL, MAX_DECIMAL_PRECISION, TIMESTAMP_NTZ_NAME,
};
use crate::databricks::conversion::models::{DatabricksType, Element};

/// The Databricks type a serialised data type names, or a refusal for a type no rule maps.
pub(crate) fn databricks_type(node: &Value) -> Result<DatabricksType, Refusal> {
    let name = node
        .get("data_type")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    let refusal = |reason: &str| {
        missing(
            Construct::new(ConstructFamily::Cast, &name),
            reason.to_owned(),
        )
    };
    let fields: &[&str] = match name.as_str() {
        "boolean" | "date" => &["data_type"],
        "tiny_int" | "small_int" | "int" | "big_int" | "string" => {
            &["data_type", "length", "spelling"]
        }
        "float" | "double" => &["data_type", "precision", "scale", "real_spelling"],
        "decimal" => &["data_type", "precision", "scale", "spelling"],
        "timestamp" => &["data_type", "timezone"],
        "array" => {
            let payload = understood(node, "data_type", &["data_type", "element_type"])?;
            let element = databricks_type(field(payload, "element_type"))?;
            return match Element::of(element) {
                Some(element) if element != Element::Null => Ok(DatabricksType::Array(element)),
                _ => Err(refusal(
                    "no rule maps arrays of this element type to DuckDB",
                )),
            };
        }
        "custom" if node.get("name").and_then(Value::as_str) == Some(TIMESTAMP_NTZ_NAME) => {
            return understood(node, "data_type", &["data_type", "name"])
                .map(|_| DatabricksType::TimestampNtz);
        }
        _ => return Err(refusal("no rule maps this type to DuckDB")),
    };
    let payload = understood(node, "data_type", fields)?;
    let number = |name: &str| field(payload, name).as_u64();
    if !field(payload, "length").is_null() {
        return Err(refusal("a length on this type has no DuckDB counterpart"));
    }
    match name.as_str() {
        "boolean" => Ok(DatabricksType::Boolean),
        "date" => Ok(DatabricksType::Date),
        "tiny_int" => Ok(DatabricksType::TinyInt),
        "small_int" => Ok(DatabricksType::SmallInt),
        "int" => Ok(DatabricksType::Int),
        "big_int" => Ok(DatabricksType::BigInt),
        "string" => Ok(DatabricksType::String),
        "float" | "double" if number("precision").is_some() || number("scale").is_some() => Err(
            refusal("a precision on a float type has no DuckDB counterpart"),
        ),
        "float" => Ok(DatabricksType::Float),
        "double" => Ok(DatabricksType::Double),
        "decimal" => decimal(number("precision"), number("scale")).ok_or_else(|| {
            refusal("the decimal's precision and scale are outside 1 to 38 and 0 to precision")
        }),
        "timestamp" => match field(payload, "timezone").as_bool() {
            Some(true) => Ok(DatabricksType::Timestamp),
            _ => Ok(DatabricksType::TimestampNtz),
        },
        _ => Err(refusal("no rule maps this type to DuckDB")),
    }
}

/// `DECIMAL(p, s)`, `DECIMAL(p)` meaning scale 0, or plain `DECIMAL(10, 0)`.
fn decimal(precision: Option<u64>, scale: Option<u64>) -> Option<DatabricksType> {
    let (precision, scale) = match (precision, scale) {
        (None, None) => return Some(DEFAULT_DECIMAL),
        (Some(precision), scale) => (precision, scale.unwrap_or(0)),
        (None, Some(_)) => return None,
    };
    match (u8::try_from(precision), u8::try_from(scale)) {
        (Ok(precision), Ok(scale))
            if (1..=MAX_DECIMAL_PRECISION).contains(&precision) && scale <= precision =>
        {
            Some(DatabricksType::Decimal { precision, scale })
        }
        _ => None,
    }
}

/// Whether DuckDB casts every value of `from` to `to` exactly as Databricks does, raising on overflow alike.
pub(crate) fn same_cast(from: DatabricksType, to: DatabricksType) -> bool {
    use DatabricksType as T;
    match (from, to) {
        (T::Null, _) => true,
        (from, to) if from == to => true,
        (from, to) if from.is_integral() && to.is_integral() => true,
        (from, T::Double) if from.is_integral() => true,
        (from, T::Decimal { precision, scale }) if from.is_integral() => from
            .integer_digits()
            .is_some_and(|digits| precision - scale >= digits),
        (T::Int | T::BigInt | T::SmallInt | T::TinyInt, T::String) => true,
        (T::Date, T::Timestamp | T::TimestampNtz) => true,
        (T::Array(Element::Null), T::Array(_)) => true,
        (T::Array(from), T::Array(to)) => same_cast(from.scalar(), to.scalar()),
        _ => false,
    }
}

/// The wider of two integer types, `None` unless both are integers.
pub(crate) fn wider_integer(left: DatabricksType, right: DatabricksType) -> Option<DatabricksType> {
    match (left.integer_digits(), right.integer_digits()) {
        (Some(left_digits), Some(right_digits)) if left_digits >= right_digits => Some(left),
        (Some(_), Some(_)) => Some(right),
        _ => None,
    }
}
