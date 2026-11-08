//! Every helper macro, each before the macros calling it, and the statement defining one.

use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::macros::_helpers::reading::{checked_body, read_body};
use crate::macros::_helpers::scalars::{number_body, scalar_body};
use crate::macros::_helpers::writing::{escaped_body, json_text_body, text_body};
use crate::macros::constants::{
    HELPER_CALL_PREFIX, JSON_CHECKED_V1, JSON_ESCAPED_V1, JSON_READ_V1, NUMBER_LARGEST_PARAMETER,
    NUMBER_TYPE_PARAMETER, OBJECTS_PARAMETER, SITE_PARAMETER, TEXT_PARAMETER, TRYING_PARAMETER,
    VALUE_PARAMETER, VALUES_PARAMETER, VARIANT_BOOLEAN_V1, VARIANT_DATE_V1, VARIANT_FLOAT_V1,
    VARIANT_JSON_TEXT_V1, VARIANT_NUMBER_V1, VARIANT_TEXT_V1, VARIANT_TIMESTAMP_NTZ_V1,
};
use crate::macros::models::HelperMacro;

/// The helper macros, dependencies first.
pub(crate) fn helper_macros() -> [HelperMacro; 10] {
    [
        HelperMacro {
            name: JSON_READ_V1,
            parameters: &[SITE_PARAMETER, TEXT_PARAMETER, TRYING_PARAMETER],
            body: read_body,
            explain: "JSON text read as a VARIANT, blank text as NULL. Raises json_fidelity where \
                      DuckDB might read the text differently from Snowflake: text that is not \
                      strict JSON, an exponent, -0.0, a word other than true, false or null, a \
                      trailing comma, a decimal longer than a double keeps, a UTF-16 surrogate \
                      escape or a key repeated in one object. When trying, as TRY_PARSE_JSON, \
                      unfinished text is NULL instead.",
        },
        HelperMacro {
            name: JSON_CHECKED_V1,
            parameters: &[
                SITE_PARAMETER,
                TEXT_PARAMETER,
                TRYING_PARAMETER,
                OBJECTS_PARAMETER,
                VALUES_PARAMETER,
            ],
            body: checked_body,
            explain: "JSON text read as json_read_v1 reads it, faster: text holding only plain \
                      strings, short numbers, true, false and null, where no object at a path \
                      in the fourth argument and no value at a path in the fifth repeats a key, \
                      is read as it is. Any other text goes through json_read_v1.",
        },
        HelperMacro {
            name: JSON_ESCAPED_V1,
            parameters: &[TEXT_PARAMETER],
            body: escaped_body,
            explain: "DuckDB's JSON text with Snowflake's escapes: DEL written as \\u007f and the \
                      hex digits of \\u escapes in lower case.",
        },
        HelperMacro {
            name: VARIANT_TEXT_V1,
            parameters: &[SITE_PARAMETER, VALUE_PARAMETER],
            body: text_body,
            explain: "A VARIANT cast to VARCHAR as Snowflake writes it: a string unquoted, JSON \
                      null as SQL NULL, and other values as JSON text with every object's keys \
                      in byte order. Raises json_fidelity where DuckDB could sort the keys only \
                      by rewriting a number.",
        },
        HelperMacro {
            name: VARIANT_JSON_TEXT_V1,
            parameters: &[SITE_PARAMETER, VALUE_PARAMETER],
            body: json_text_body,
            explain: "A VARIANT as Snowflake writes it as JSON text, as TO_JSON does: every \
                      object's keys in byte order at every depth. Raises json_fidelity where \
                      DuckDB could sort the keys only by rewriting a number.",
        },
        HelperMacro {
            name: VARIANT_NUMBER_V1,
            parameters: &[
                SITE_PARAMETER,
                VALUE_PARAMETER,
                NUMBER_TYPE_PARAMETER,
                NUMBER_LARGEST_PARAMETER,
            ],
            body: number_body,
            explain: "A VARIANT cast to NUMBER(p, s), the type of the typed NULL passed third: \
                      a number cast directly, a string read as Snowflake reads number text, a \
                      boolean as 1 or 0, JSON null as NULL. Raises Snowflake's errors for text it \
                      does not read (100071) and values out of range (100046), and json_fidelity \
                      for an object or array.",
        },
        HelperMacro {
            name: VARIANT_FLOAT_V1,
            parameters: &[SITE_PARAMETER, VALUE_PARAMETER],
            body: || scalar_body(SnowflakeType::Float),
            explain: "A VARIANT cast to FLOAT: a number as a double, a string read as Snowflake \
                      reads number text, JSON null as NULL. Raises Snowflake's error for text it \
                      does not read (100071), and json_fidelity for a boolean, object or array.",
        },
        HelperMacro {
            name: VARIANT_BOOLEAN_V1,
            parameters: &[SITE_PARAMETER, VALUE_PARAMETER],
            body: || scalar_body(SnowflakeType::Boolean),
            explain: "A VARIANT cast to BOOLEAN: a boolean as it is, a string read as Snowflake \
                      reads boolean text, JSON null as NULL. Raises Snowflake's error for text it \
                      does not read, and json_fidelity for a number, object or array.",
        },
        HelperMacro {
            name: VARIANT_DATE_V1,
            parameters: &[SITE_PARAMETER, VALUE_PARAMETER],
            body: || scalar_body(SnowflakeType::Date),
            explain: "A VARIANT cast to DATE: a string read as Snowflake reads date text, JSON \
                      null as NULL. Raises Snowflake's error for text it does not read, and \
                      json_fidelity for a number, boolean, object or array.",
        },
        HelperMacro {
            name: VARIANT_TIMESTAMP_NTZ_V1,
            parameters: &[SITE_PARAMETER, VALUE_PARAMETER],
            body: || scalar_body(SnowflakeType::TimestampNtz { precision: 9 }),
            explain: "A VARIANT cast to TIMESTAMP_NTZ: a string read as Snowflake reads timestamp \
                      text, an integer as seconds since 1970, JSON null as NULL. Raises \
                      Snowflake's error for text it does not read, and json_fidelity for a \
                      number with a fraction, a boolean, an object or an array.",
        },
    ]
}

/// The statement creating `helper` in the `sqlquack` schema.
pub(crate) fn definition(helper: &HelperMacro) -> String {
    format!(
        "CREATE OR REPLACE MACRO {HELPER_CALL_PREFIX}{}({}) AS {}",
        helper.name,
        helper.parameters.join(", "),
        (helper.body)()
    )
}
