"""The benchmarked workloads, the headroom over recorded ratios and how long each query is timed."""

from __future__ import annotations

from scripts.sql_benchmark.models import Workload

BUDGET_FORMAT: str = "sqlquack-sql-benchmark/1"
RATIO_HEADROOM: float = 2.0
MIN_REPEATS: int = 3
MAX_REPEATS: int = 30
TIMED_SECONDS: float = 1.0
ROWS_PLACEHOLDER: str = "{rows}"

SCHEMA: dict[str, list[list[str]]] = {
    "EVENTS": [["PAYLOAD", "VARIANT"]],
    "ORDERS": [["TOTAL", "NUMBER(12,2)"]],
}

EVENTS_ROWS: str = """
CREATE TABLE "EVENTS" AS SELECT CAST(printf(
    '{"order_id":%d,"customer_id":"C%07d","amount":%d.%02d,"status":"%s","shipping":{"country":"%s"}}',
    i, i % 100003, (i * 37) % 100000, i % 100, ['placed', 'shipped', 'returned'][1 + i % 3],
    ['GB', 'FR', 'US', 'DE'][1 + i % 4]) AS JSON) AS "PAYLOAD"
FROM range({rows}) AS t(i)
"""

ORDERS_ROWS: str = """
CREATE TABLE "ORDERS" AS
SELECT CAST(((i * 7919) % 2000000000) / 100.0 - 10000000 AS DECIMAL(12,2)) AS "TOTAL"
FROM range({rows}) AS t(i)
"""

WORKLOADS: tuple[Workload, ...] = (
    Workload(
        name="variant_one_field",
        rows=EVENTS_ROWS,
        snowflake="SELECT payload:customer_id::VARCHAR AS customer_id FROM events",
        reference="""SELECT json_extract_string("PAYLOAD", '$.customer_id') AS customer_id
FROM "EVENTS\"""",
    ),
    Workload(
        name="variant_staging",
        rows=EVENTS_ROWS,
        snowflake="""SELECT
    payload:customer_id::VARCHAR AS customer_id,
    payload:order_id::NUMBER AS order_id,
    payload:amount::NUMBER(12,2) AS amount,
    payload:status::VARCHAR AS status,
    payload:shipping.country::VARCHAR AS country
FROM events""",
        reference="""SELECT
    json_extract_string("PAYLOAD", '$.customer_id') AS customer_id,
    CAST(json_extract("PAYLOAD", '$.order_id') AS DECIMAL(38,0)) AS order_id,
    CAST(json_extract("PAYLOAD", '$.amount') AS DECIMAL(12,2)) AS amount,
    json_extract_string("PAYLOAD", '$.status') AS status,
    json_extract_string("PAYLOAD", '$.shipping.country') AS country
FROM "EVENTS\"""",
    ),
    Workload(
        name="decimal_divide",
        rows=ORDERS_ROWS,
        snowflake="SELECT total / 3 AS third FROM orders",
        reference="""SELECT CAST("TOTAL" AS DECIMAL(18,8)) / 3 AS third FROM "ORDERS\"""",
        row_factor=10,
    ),
)
