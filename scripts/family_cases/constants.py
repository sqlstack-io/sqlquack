"""The values each family's grid spans."""

from __future__ import annotations

ZONED_TEXTS: str = "zoned_texts@1"
ZONED_WALL_TIMES: tuple[str, ...] = ("2023-12-31 22:30:15", "2024-03-05 06:07:08")
ZONED_FRACTIONS: tuple[str, ...] = ("", ".123", ".123456789")
ZONED_SEPARATORS: tuple[str, ...] = (" ", "T")
ZONED_ATTACHED_OFFSETS: tuple[str, ...] = ("", "Z", "+00:00", "+09:00", "-05:30")
ZONED_SPACED_OFFSETS: tuple[str, ...] = (" +00:00", " +0000", " +09:00")
ZONED_DATES: tuple[str, ...] = ("2023-12-31", "2024-02-29")
ZONED_EDGE_TEXTS: tuple[tuple[str, str], ...] = (
    ("empty", ""),
    ("padded", "  2024-03-05 06:07:08  "),
    ("no_seconds", "2024-03-05 06:07"),
    ("t_spaced_offset", "2024-03-05T06:07:08 +00:00"),
    ("t_compact_offset", "2024-03-05T06:07:08+0000"),
    ("lower_t", "2024-03-05t06:07:08"),
    ("lower_z", "2024-03-05 06:07:08z"),
    ("words", "not a time"),
    ("impossible_day", "2024-02-30 00:00:00"),
    ("hour_24", "2024-03-05 24:00:00"),
    ("offset_beyond", "2024-03-05 06:07:08 +26:00"),
    ("epoch_seconds", "1709618828"),
    ("five_digit_year", "12024-03-05 06:07:08"),
)
ZONED_STRICT_EDGES_DEFERRED: frozenset[str] = frozenset({"five_digit_year"})
ZONED_STRICT_FORMS: tuple[tuple[str, str, str], ...] = (
    ("cast_tz", "CAST(s AS TIMESTAMP_TZ)", "node:cast"),
    ("cast_ltz", "s::TIMESTAMP_LTZ", "node:cast"),
    ("to_tz", "TO_TIMESTAMP_TZ(s)", "fn:TO_TIMESTAMP_TZ"),
    ("to_ltz", "TO_TIMESTAMP_LTZ(s)", "fn:TO_TIMESTAMP_LTZ"),
)
ZONED_LENIENT_FORMS: tuple[tuple[str, str, str], ...] = (
    ("try_cast_tz", "TRY_CAST(s AS TIMESTAMP_TZ)", "node:try_cast"),
    ("try_to_ltz", "TRY_TO_TIMESTAMP_LTZ(s)", "fn:TRY_TO_TIMESTAMP_LTZ"),
)
ZONED_PROJECTIONS: tuple[tuple[str, str], ...] = (
    ("kept", "v"),
    ("instant", "CONVERT_TIMEZONE('UTC', v)::TIMESTAMP_NTZ"),
    ("text", "TO_CHAR(v, 'YYYY-MM-DD HH24:MI:SS.FF9 TZH:TZM')"),
    ("varchar", "v::VARCHAR"),
    ("wall", "v::TIMESTAMP_NTZ"),
    ("day", "TO_DATE(v)"),
    ("year", "YEAR(v)"),
    ("day_start", "DATE_TRUNC('day', v)"),
    ("after_text", "v >= '2024-01-01 00:00:00'"),
)
ZONED_LITERALS: tuple[tuple[str, str], ...] = (
    ("iso_t_utc", "2024-03-05T06:07:08+00:00"),
    ("iso_t_z", "2024-03-05T06:07:08Z"),
    ("iso_t_naive", "2024-03-05T06:07:08"),
    ("spaced_utc", "2024-03-05 06:07:08 +00:00"),
    ("iso_t_offset", "2023-12-31T22:30:15-05:30"),
    ("fraction_utc", "2024-03-05T06:07:08.123456+00:00"),
)
ZONED_LITERAL_TYPES: tuple[str, ...] = ("TIMESTAMP_TZ", "TIMESTAMP_LTZ")
ZONED_COLUMN_TYPES: tuple[str, ...] = ("TIMESTAMP_TZ(9)", "TIMESTAMP_LTZ(9)")
ZONED_COLUMN_VALUES: tuple[str, ...] = (
    "'2023-12-31 22:30:15 +00:00'",
    "'2023-12-31 22:30:15 -05:30'",
    "'2024-01-01 00:00:00 +00:00'",
    "'2024-01-01 08:59:59 +09:00'",
    "'2024-03-05 06:07:08.123456789 +00:00'",
    "NULL",
)
ZONED_COMPARISONS: tuple[tuple[str, str], ...] = (
    ("gte_naive_text", "z >= '2024-01-01 00:00:00'"),
    ("lt_date_text", "z < '2024-01-01'"),
    ("eq_offset_text", "z = '2024-01-01 00:00:00 +00:00'"),
    ("eq_t_text", "z = '2024-01-01T00:00:00Z'"),
    ("between_texts", "z BETWEEN '2023-12-31 23:00:00' AND '2024-01-01 00:00:00'"),
    ("eq_ntz", "z = '2024-01-01 00:00:00'::TIMESTAMP_NTZ"),
    ("gte_date", "z >= '2024-01-01'::DATE"),
)
VARIANT_ORDER: str = "variant_order@1"
VARIANT_ORDER_GROUPS: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("numbers", ("10", "2", "-1", "1.5", "0", "1e2", "12345678901234567890", "-0.5")),
    ("texts", ('"b"', '"B"', '"a"', '"10"', '"2"', '""', '"é"', '"a b"')),
    ("booleans", ("true", "false")),
    ("containers", ("[2]", "[1,9]", "[]", '{"b":1}', '{"a":2}', "{}")),
    ("mixed", ("1", '"1"', "true", '{"a":1}', "[1]", '"a"', "-3", "false")),
)
VARIANT_EQUAL_PAIRS: tuple[tuple[str, str], ...] = (
    ("1", "1"),
    ("1", "1.0"),
    ("1", '"1"'),
    ("100", "1e2"),
    ('"a"', '"A"'),
    ("true", '"true"'),
    ('{"a":1,"b":2}', '{"b":2,"a":1}'),
    ("[1,2]", "[1,2]"),
    ("[1,2]", "[2,1]"),
    ("null", "null"),
)
VARIANT_NUMBER_TEXTS: tuple[str, ...] = ("1", "1.005", "12.345", '"12.5"', "true", "1e3", "-7.25")
VARIANT_NUMBER_EDGES: tuple[tuple[str, str], ...] = (
    ("text_word", '"abc"'),
    ("too_wide", "123456789012"),
    ("object", '{"a":1}'),
    ("array", "[1]"),
    ("empty_text", '""'),
    ("false", "false"),
)
VARIANT_NUMBER_FORMS: tuple[tuple[str, str], ...] = (
    ("to_decimal_10_2", "TO_DECIMAL(v, 10, 2)"),
    ("to_number", "TO_NUMBER(v)"),
    ("to_number_10_0", "TO_NUMBER(v, 10, 0)"),
)
VARIANT_AGGREGATED_TYPES: tuple[tuple[str, str, tuple[str, ...]], ...] = (
    ("ntz", "TIMESTAMP_NTZ(9)", ("'2024-03-05 06:07:08.123456789'", "'2023-12-31 22:30:15'")),
    ("tz", "TIMESTAMP_TZ(9)", ("'2024-03-05 06:07:08 +00:00'", "'2023-12-31 22:30:15 +00:00'")),
    ("ltz", "TIMESTAMP_LTZ(9)", ("'2024-03-05 06:07:08 +00:00'", "'2023-12-31 22:30:15 +00:00'")),
    ("date", "DATE", ("'2024-03-05'", "'2023-12-31'")),
    ("time", "TIME(9)", ("'06:07:08.123'", "'22:30:15'")),
)
ZONED_OFFSET_DIVERGENCE: str = "timestamp_tz_offset"
ZONED_SUBMICRO_DIVERGENCE: str = "timestamp_tz_submicro"
ZONED_SPELLING_DIVERGENCE: str = "date_text_spelling"
ZONED_OFFSET_PROJECTIONS: frozenset[str] = frozenset(
    {"kept", "instant", "wall", "day", "year", "day_start", "after_text"}
)
ZONED_SUBMICRO_PROJECTIONS: frozenset[str] = frozenset(
    {"kept", "instant", "wall", "year", "after_text"}
)
ZONED_SPELLING_EDGES: frozenset[str] = frozenset({"offset_beyond"})
ZONED_OFFSET_LITERALS: frozenset[str] = frozenset({"iso_t_offset"})
LOCAL_ZONE_MARK: str = "LTZ"
VARIANT_ORDER_DIVERGENCES: dict[str, str] = {
    "numbers": "json_fidelity",
    "containers": "array_sort_order",
    "mixed": "array_sort_order",
}
VARIANT_EXTREME_DIVERGENCES: dict[str, str] = {
    "numbers": "json_fidelity",
    "containers": "array_sort_order",
}
VARIANT_SPELLING_DIVERGENCE: str = "json_fidelity"
VARIANT_AGGREGATED_DIVERGENCES: dict[str, str] = {
    "date": "variant_typed_values",
    "ntz": "variant_typed_values",
}
ZONED_TEXT_COMPARISONS: frozenset[str] = frozenset(
    {"gte_naive_text", "lt_date_text", "eq_offset_text", "between_texts"}
)
ZONED_NAIVE_COMPARISONS: frozenset[tuple[str, str]] = frozenset({("TIMESTAMP_LTZ(9)", "eq_ntz")})
