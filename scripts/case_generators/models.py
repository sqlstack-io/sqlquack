"""Generated cases and case files, and the grid axes they are built from."""

from __future__ import annotations

from dataclasses import dataclass, field
from decimal import Decimal

from scripts.case_generators.types import Operator, Target


@dataclass(frozen=True)
class Column:
    """An input column: a lower-case name and its Snowflake type."""

    name: str
    data_type: str


@dataclass(frozen=True)
class Case:
    """One generated case: typed input rows and an expression or an ordered query."""

    id: str
    description: str
    columns: tuple[Column, ...]
    rows: tuple[tuple[str, ...], ...]
    tags: tuple[str, ...]
    expression: str | None = None
    query: str | None = None
    order: str | None = None
    expected_divergence: str | None = None


@dataclass(frozen=True)
class CaseFile:
    """One generated case file, written to `cases/generated/<generator>/<name>.toml`."""

    generator: str
    name: str
    category: str
    construct: str
    cases: tuple[Case, ...]
    session_profile: str = "utc-v1"
    header: tuple[str, ...] = field(default=())


@dataclass(frozen=True)
class NumberType:
    """`NUMBER(precision, scale)`, or FLOAT when `precision` is 0."""

    precision: int
    scale: int

    @property
    def is_float(self) -> bool:
        """Whether this is FLOAT."""
        return self.precision == 0

    @property
    def name(self) -> str:
        """The Snowflake spelling."""
        return "FLOAT" if self.is_float else f"NUMBER({self.precision},{self.scale})"

    @property
    def slug(self) -> str:
        """A lower-case identifier fragment, such as `n10_2` or `float`."""
        return "float" if self.is_float else f"n{self.precision}_{self.scale}"


@dataclass(frozen=True)
class OperatorFile:
    """An arithmetic operator, the file it is written to and its construct."""

    operator: Operator
    slug: str
    construct: str


@dataclass(frozen=True)
class SamplePair:
    """Names of the left and right samples to combine; None stands for NULL."""

    left: str | None
    right: str | None


@dataclass(frozen=True)
class CastType:
    """A cast source or target: its Snowflake spelling and an identifier fragment."""

    name: str
    slug: str


@dataclass(frozen=True)
class Variant:
    """Argument columns and the non-NULL value each takes."""

    slug: str
    columns: tuple[Column, ...]
    values: tuple[str, ...]


@dataclass(frozen=True)
class Function:
    """An n-ary construct, its category and the argument variants to try."""

    slug: str
    construct: str
    category: str
    expression: str
    variants: tuple[Variant, ...]
    extra_rows: tuple[tuple[str, ...], ...] = ()
    extra_tags: tuple[str, ...] = ()


@dataclass(frozen=True)
class Path:
    """A Snowflake path expression on `v` and the steps it takes."""

    slug: str
    expression: str
    steps: tuple[str | int, ...]
    tags: tuple[str, ...]


@dataclass(frozen=True)
class Projection:
    """A projection of a VARIANT to a scalar: its slug, template and construct or target."""

    slug: str
    template: str
    construct: str
    target: Target | None = None


@dataclass(frozen=True)
class NumberProjection:
    """A projection of the number at `v:n`; `limit` bounds the values a NUMBER target holds."""

    slug: str
    expression: str
    limit: Decimal | None


@dataclass(frozen=True)
class Instant:
    """A wall-clock time with nanoseconds and a UTC offset in minutes, to render as text."""

    year: int
    month: int
    day: int
    hour: int
    minute: int
    second: int
    nanosecond: int
    offset_minutes: int


@dataclass(frozen=True)
class TemporalFormat:
    """A Snowflake format string and an identifier for it."""

    slug: str
    format: str


@dataclass(frozen=True)
class TemporalFunction:
    """A text-to-temporal function: its id, its name, and the constructs it and its TRY form are."""

    slug: str
    name: str
    construct: str
    try_construct: str
