"""Python functions in a DuckDB connection, as SQLQuack's converted SQL calls them.

`register_functions` registers each function a conversion report lists, wrapped so its values
cross as they do in Snowflake or the call raises. Run as a script, it replays one statement with
the functions a request file defines, for `sqlquack verify`.

ABI `scalar-v1`: arguments and the result are VARCHAR, NUMBER, FLOAT or BOOLEAN; a NUMBER of scale
0 reaches the handler as `int`, as in Snowflake. A result of another Python type raises, as do a
Decimal beyond the declared scale or in exponent form and an int beyond 2^53 as FLOAT. ABI
`variant-json-v1`: the handler returns a VARIANT as JSON text, which the converted SQL reads as
PARSE_JSON does; `None` is JSON null; `Decimal`, `date`, naive `datetime` and `bytes` are their
str() text and int keys their digits, as recorded. Floats, zoned datetimes, datetimes without a
fraction of a second and other types raise.
Volatile functions, Snowflake's default, are never folded or shared; `returns_null` ones give NULL
for a NULL argument without calling the handler. Only the pinned duckdb release is accepted.
"""

from __future__ import annotations

import datetime
import decimal
import inspect
import json
import sys
from collections.abc import Callable, Mapping, Sequence
from pathlib import Path
from typing import Any

SCALAR_ABI: str = "scalar-v1"
VARIANT_ABI: str = "variant-json-v1"
CROSSING_DIVERGENCE: str = "python_value_crossing"
# VARIANT and FLOAT results cross as text, which the converted SQL casts: DuckDB reads a returned
# NaN as NULL.
TEXT_HOLDING: str = "VARCHAR"
DOUBLE_TYPE: str = "DOUBLE"
WHOLE_SCALE_SUFFIX: str = ",0)"
DECIMAL_PREFIX: str = "DECIMAL("
# Python values Snowflake stores in a VARIANT as their str() text, as recorded.
TEXT_LEAVES: tuple[type, ...] = (decimal.Decimal, datetime.date, bytes)
# The DuckDB release SQLQuack links; values cross as recorded only under it.
PINNED_DUCKDB_VERSION: str = "2.0.0"
IMMUTABLE: str = "immutable"
RETURNS_NULL: str = "returns_null"
LARGEST_EXACT_FLOAT_INTEGER: int = 2**53
RESULT_TABLE: str = "result"
TEXT_ENCODING: str = "utf-8"

Handler = Callable[..., Any]


class CrossingError(ValueError):
    """A value whose crossing between Python and SQL no recording shows."""

    def __init__(self, detail: str) -> None:
        super().__init__(f"'{CROSSING_DIVERGENCE}: {detail}'")


def register_functions(
    connection: Any, functions: Sequence[Mapping[str, Any]], handlers: Mapping[str, Handler]
) -> None:
    """Register each listed function by its handler, volatile ones never folded or shared."""
    import duckdb

    if duckdb.__version__ != PINNED_DUCKDB_VERSION:
        raise RuntimeError(
            f"duckdb {duckdb.__version__} is installed; SQLQuack's runtime needs "
            f"{PINNED_DUCKDB_VERSION}, the release it links"
        )
    for function in functions:
        name: str = function["name"]
        abi: str = function["abi"]
        if abi not in (SCALAR_ABI, VARIANT_ABI):
            raise ValueError(f"function {name}: unknown ABI `{abi}`")
        arguments: list[str] = list(function["arguments"])
        declared: str = function["returns"]
        returns: str = (
            TEXT_HOLDING if abi == VARIANT_ABI or declared.upper() == DOUBLE_TYPE else declared
        )
        connection.create_function(
            name,
            wrapped(handlers[name], abi=abi, arguments=arguments, returns=declared),
            [duckdb.sqltype(argument) for argument in arguments],
            duckdb.sqltype(returns),
            type="native",
            null_handling="default" if function["null_input"] == RETURNS_NULL else "special",
            side_effects=function["volatility"] != IMMUTABLE,
        )


def wrapped(handler: Handler, *, abi: str, arguments: Sequence[str], returns: str) -> Handler:
    """`handler` with arguments read as Snowflake passes them and its result checked."""
    whole: list[bool] = [is_whole_number(argument) for argument in arguments]

    def call(*values: Any) -> Any:
        passed: list[Any] = [
            int(value) if is_whole and value is not None else value
            for value, is_whole in zip(values, whole, strict=True)
        ]
        result: Any = handler(*passed)
        if abi == VARIANT_ABI:
            return variant_text(result)
        checked: Any = scalar_result(result, returns)
        if checked is not None and returns.upper() == DOUBLE_TYPE:
            return repr(float(checked))
        return checked

    call.__signature__ = inspect.Signature(  # ty: ignore[unresolved-attribute]
        [
            inspect.Parameter(f"argument_{index}", inspect.Parameter.POSITIONAL_ONLY)
            for index in range(len(arguments))
        ]
    )
    return call


def is_whole_number(duckdb_type: str) -> bool:
    """Whether `duckdb_type` is a DECIMAL of scale 0."""
    upper: str = duckdb_type.replace(" ", "").upper()
    return upper.startswith(DECIMAL_PREFIX) and upper.endswith(WHOLE_SCALE_SUFFIX)


def scalar_result(result: Any, returns: str) -> Any:
    """`result` when it crosses into `returns` as recorded, else raise."""
    if result is None:
        return None
    upper: str = returns.replace(" ", "").upper()
    is_int: bool = isinstance(result, int) and not isinstance(result, bool)
    crosses: bool = (
        isinstance(result, str)
        if upper == "VARCHAR"
        else isinstance(result, bool)
        if upper == "BOOLEAN"
        else isinstance(result, float) or (is_int and abs(result) <= LARGEST_EXACT_FLOAT_INTEGER)
        if upper == "DOUBLE"
        else is_int or exact_decimal(result, upper)
        if upper.startswith(DECIMAL_PREFIX)
        else False
    )
    if not crosses:
        raise CrossingError(f"a Python {type(result).__name__} {result!r} returned as {returns}")
    return result


def exact_decimal(result: Any, duckdb_type: str) -> bool:
    """Whether `result` is a finite Decimal within `duckdb_type`'s scale."""
    if not isinstance(result, decimal.Decimal) or not result.is_finite():
        return False
    scale: int = int(duckdb_type.rstrip(")").split(",")[1])
    exponent: int = int(result.as_tuple().exponent)
    return exponent <= 0 and -exponent <= scale


def variant_text(value: Any) -> str:
    """`value` as JSON text Snowflake reads as the VARIANT it stores, or raise."""
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, datetime.datetime) and value.tzinfo is not None:
        raise CrossingError("a Python datetime with a time zone in a VARIANT")
    if isinstance(value, datetime.datetime) and value.microsecond == 0:
        raise CrossingError("a Python datetime without a fraction of a second in a VARIANT")
    if isinstance(value, decimal.Decimal) and "E" in str(value).upper():
        raise CrossingError("a Python Decimal in exponent form in a VARIANT")
    if isinstance(value, TEXT_LEAVES):
        return json.dumps(str(value), ensure_ascii=False)
    if isinstance(value, list | tuple):
        return "[" + ",".join(variant_text(item) for item in value) + "]"
    if isinstance(value, dict):
        return object_text(value)
    raise CrossingError(f"a Python {type(value).__name__} in a VARIANT")


def object_text(value: dict[Any, Any]) -> str:
    """A dict as a JSON object: int keys as their digits, members sorted by their keys' bytes."""
    members: dict[str, Any] = {}
    for key, item in value.items():
        is_text: bool = isinstance(key, str)
        is_int: bool = isinstance(key, int) and not isinstance(key, bool)
        if not (is_text or is_int):
            raise CrossingError(f"a Python dict with a {type(key).__name__} key in a VARIANT")
        if str(key) in members:
            raise CrossingError("a Python dict with two keys of one spelling in a VARIANT")
        members[str(key)] = item
    keys: list[str] = sorted(members, key=lambda key: key.encode(TEXT_ENCODING))
    written: list[str] = [
        f"{json.dumps(key, ensure_ascii=False)}:{variant_text(members[key])}" for key in keys
    ]
    return "{" + ",".join(written) + "}"


def handler_from_source(source: str, handler: str) -> Handler:
    """The function named `handler` that Python `source` defines."""
    namespace: dict[str, Any] = {"__name__": f"sqlquack_function_{handler}"}
    exec(compile(source, f"<function {handler}>", "exec"), namespace)
    return namespace[handler]


def replay(request: Mapping[str, Any]) -> None:
    """Run the request's statement with its functions, into a table of the request's database."""
    import duckdb

    if request["duckdb_version"].lstrip("v") != duckdb.__version__:
        Path(request["error"]).write_text(
            f"the Python replay runs duckdb {duckdb.__version__}, not the linked "
            f"{request['duckdb_version']}",
            encoding=TEXT_ENCODING,
        )
        return
    functions: list[Mapping[str, Any]] = list(request["functions"])
    handlers: dict[str, Handler] = {
        function["name"]: handler_from_source(function["source"], function["handler"])
        for function in functions
    }
    connection = duckdb.connect(request["database"])
    try:
        connection.execute(f"SET TimeZone = '{request['time_zone']}'")
        connection.execute(f"SET default_null_order = '{request['null_order']}'")
        connection.execute("SET threads = 1")
        for statement in request["setup"]:
            connection.execute(statement)
        register_functions(connection, functions, handlers)
        try:
            connection.execute(f"CREATE TABLE {RESULT_TABLE} AS {request['sql']}")
        except duckdb.Error as error:
            Path(request["error"]).write_text(str(error), encoding=TEXT_ENCODING)
    finally:
        connection.close()


if __name__ == "__main__":
    replay(json.loads(Path(sys.argv[1]).read_text(encoding=TEXT_ENCODING)))
