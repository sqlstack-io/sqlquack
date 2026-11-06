"""Recorder identity, file formats and defaults."""

from __future__ import annotations

import re

RECORDER_NAME: str = "sqlquack-record"
RECORDER_VERSION: str = "0.1.0"
EVIDENCE_FORMAT: int = 1
DEFAULT_PENDING_DIR: str = "evidence/pending"
DEFAULT_RESULTS_DIR: str = "evidence/results"
REPOSITORY_MARKER: str = "sqlquack.toml"
DEFAULT_BATCH_SIZE: int = 20
QUERY_TAG: str = "sqlquack-record"
CONNECTION_ENVIRONMENT: str = "SQLQUACK_SNOWFLAKE_CONNECTION"
WAREHOUSE_ENVIRONMENT: str = "SQLQUACK_SNOWFLAKE_WAREHOUSE"
CONNECTION_OVERRIDES: dict[str, str] = {
    "SNOWFLAKE_ACCOUNT": "account",
    "SNOWFLAKE_USER": "user",
    "SNOWFLAKE_ROLE": "role",
    "SNOWFLAKE_AUTHENTICATOR": "authenticator",
    "SNOWFLAKE_HOST": "host",
    "SNOWFLAKE_DATABASE": "database",
    "SNOWFLAKE_SCHEMA": "schema",
}
RESULT_FORMAT_PARAMETER: str = "PYTHON_CONNECTOR_QUERY_RESULT_FORMAT"
ARROW_RESULT_FORMAT: str = "ARROW"
RECORDABLE_SQLSTATE_CLASSES: frozenset[str] = frozenset({"0A", "21", "22", "42"})
RECORDABLE_FUNCTION_ERRORS: frozenset[tuple[int, str]] = frozenset({(100357, "P0000")})
# Built-in function errors (SQLSTATE P0000): a case outcome only for a case expecting an error.
# Each pair was observed on the translation cases named above it.
EXPECTED_FUNCTION_ERRORS: frozenset[tuple[int, str]] = frozenset(
    {
        # unparsable or non-4326 WKB: st-geographyfromwkb t003/t004, st-geometryfromwkb t004
        (100254, "P0000"),
        # unparsable or non-4326 WKT: st-geographyfromwkt t003/t004, st-geometryfromwkt t004
        (100255, "P0000"),
        # unrecognised geo input: to-geography t005/t006, to-geometry t005
        (100256, "P0000"),
        # ST_X of a non-point: st-x t002
        (100281, "P0000"),
        # ST_Y of a non-point: st-y t002
        (100282, "P0000"),
        # geohash precision 0: st-geohash t003, st-geogfromgeohash t004, st-geomfromgeohash t003
        (100319, "P0000"),
        # ST_AZIMUTH of a non-point: st-azimuth t003
        (100344, "P0000"),
        # SRID mismatch: st-contains, -coveredby, -covers, -disjoint, -distance, -intersects and
        # -makeline t003
        (100388, "P0000"),
        # a polygon as a line: st-endpoint t002, st-startpoint t002, st-pointn t003
        (100401, "P0000"),
        # point index out of bounds: st-pointn t002
        (100402, "P0000"),
        # unsupported SRID: st-transform t003
        (100404, "P0000"),
        # H3 resolution 16: h3-coverage t004, h3-latlng-to-cell t003, h3-point-to-cell t002,
        # h3-polygon-to-cells t002, and their string forms
        (100410, "P0000"),
        # NaN latitude: h3-latlng-to-cell t004 and its string form
        (100411, "P0000"),
        # H3 unspecified error for a negative k: h3-grid-disk t003
        (100412, "P0000"),
        # H3 target resolution out of range: h3-cell-to-children t002 and its string form,
        # h3-cell-to-parent t003
        (100415, "P0000"),
        # too many H3 cells: h3-coverage t003 and its string form
        (100418, "P0000"),
        # mixed H3 resolutions: h3-grid-distance t003, h3-uncompact-cells t002/t003 and its string
        # form
        (100422, "P0000"),
    }
)
DATA_EXCEPTION_SQLSTATE_CLASSES: frozenset[str] = frozenset({"22"})
SESSION_DEPENDENT_SQLSTATES: frozenset[str] = frozenset({"42501"})
SESSION_FAILURE_SQLSTATE_CLASSES: frozenset[str] = frozenset({"08", "28", "53", "57", "XX"})
INTERNAL_SQLSTATE_CLASS: str = "XX"
HASH_NAME_LENGTH: int = 64
NANOSECOND_TYPE_NAMES: frozenset[str] = frozenset(
    {"TIMESTAMP_NTZ", "TIMESTAMP_LTZ", "TIMESTAMP_TZ", "TIME"}
)
MICROSECOND_SCALE: int = 6
NANOSECOND_UNIT: str = "ns"
NANOS_PER_MICRO: int = 1000
QUERY_ID_CHARACTERS: frozenset[str] = frozenset("0123456789abcdefABCDEF-")
HEX_DIGITS: frozenset[str] = frozenset("0123456789abcdef")
PENDING_KEYS: frozenset[str] = frozenset(
    {
        "format",
        "case_hash",
        "case_reference",
        "source_dialect",
        "session_parameters",
        "source_sql",
        "duckdb_version",
    }
)
VALUES_KEY: str = "values"
SETUP_KEY: str = "setup_sql"
PINNED_KEY: str = "pinned_parameters"
COMPILE_ERROR_KEY: str = "compile_error"
EXPECTED_ERROR_KEY: str = "expected_error"
OPTIONAL_PENDING_KEYS: frozenset[str] = frozenset(
    {VALUES_KEY, SETUP_KEY, PINNED_KEY, COMPILE_ERROR_KEY, EXPECTED_ERROR_KEY}
)
EXPLAIN_PREFIX: str = "EXPLAIN USING TEXT "
COMPILE_FAILURE_SQLSTATE_CLASSES: frozenset[str] = frozenset({"0A", "42"})
SHOW_SESSION_PARAMETERS: str = "SHOW PARAMETERS IN SESSION"
PARAMETER_KEY_COLUMN: str = "key"
PARAMETER_VALUE_COLUMN: str = "value"
SESSION_DEFAULTS_FORMAT: int = 1
ARGUMENT_SEPARATOR: str = ","
PARENTHESIS_DEPTH: dict[str, int] = {"(": 1, ")": -1}
SCRATCH_FUNCTION_SUFFIX: str = "__sqlquack_"
SCRATCH_HASH_LENGTH: int = 16
SCRATCH_NONCE_BYTES: int = 4
CREATE_FUNCTION_PATTERN: str = (
    r"CREATE FUNCTION (?P<name>[a-z_][a-z0-9_]*)\((?P<arguments>.*?)\)\s*RETURNS"
)
ROWS_OUTCOME: str = "rows"
ERROR_OUTCOME: str = "error"
TYPES_OUTCOME: str = "types"
ACCOUNT_ARGUMENT: str = "account"
WAREHOUSE_ARGUMENT: str = "warehouse"
SESSION_PARAMETERS_ARGUMENT: str = "session_parameters"
CREDENTIAL_CACHE_DEFAULTS: dict[str, bool] = {
    "client_store_temporary_credential": True,
    "oauth_enable_refresh_tokens": True,
}
MULTI_STATEMENT_FAILURE: tuple[int, str] = (100132, "P0000")
QUERY_ID_PATTERN: re.Pattern[str] = re.compile(
    r"\s*\(?(?:query id:\s*)?[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}\)?:?", re.IGNORECASE
)
ERROR_PREFIX_PATTERN: re.Pattern[str] = re.compile(r"^\d+ \([0-9A-Z]{5}\):\s*")
