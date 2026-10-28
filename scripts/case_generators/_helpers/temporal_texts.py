"""Text to DATE and TIMESTAMP (formats and AUTO detection), and CONVERT_TIMEZONE across zones."""

from __future__ import annotations

from dataclasses import replace

from scripts.case_generators.constants import (
    AUTO_DATE_ATTEMPT,
    AUTO_DATE_SPELLINGS,
    AUTO_DATE_TEXTS,
    AUTO_TIMESTAMP_ATTEMPT,
    AUTO_TIMESTAMP_SPELLINGS,
    AUTO_TIMESTAMP_TEXTS,
    CENTURY_PIVOT_INSTANTS,
    DATE_FORMAT_FUNCTIONS,
    DATE_FORMATS,
    DATE_TEXT_SPELLING,
    FORMAT_TOKENS,
    INSTANTS,
    INSTANTS_FOR_TEXT,
    LOCAL_TIMES,
    MONTH_NAMES,
    NANOSECOND_YEARS,
    NULL_CELL,
    QUOTE,
    SPELLED_DATE_FORMATS,
    SUB_MICROSECOND_FORMAT_TOKENS,
    TEMPORAL_TEXTS,
    TIME_ZONES,
    TIMESTAMP_FORMAT_FUNCTIONS,
    TIMESTAMP_FORMATS,
    TWELVE,
    UNSPELLED_DATE_TEXTS,
    UNSURE_DATE_TEXTS,
    UNSURE_TIMESTAMP_TEXTS,
    ZONES,
)
from scripts.case_generators.models import (
    Case,
    CaseFile,
    Column,
    Instant,
    Projection,
    TemporalFormat,
    TemporalFunction,
)
from scripts.case_generators.types import Row


def offset_text(minutes: int) -> str:
    """A UTC offset as +HH:MM."""
    sign: str = "-" if minutes < 0 else "+"
    hours, rest = divmod(abs(minutes), 60)
    return f"{sign}{hours:02d}:{rest:02d}"


def token_text(*, token: str, instant: Instant, padded: bool) -> str:
    """One format token rendered for `instant`."""
    width: int = 2 if padded else 1
    hour_12: int = instant.hour % TWELVE or TWELVE
    texts: dict[str, str] = {
        "TZH:TZM": offset_text(instant.offset_minutes),
        "YYYY": f"{instant.year:04d}",
        "HH24": f"{instant.hour:0{width}d}",
        "HH12": f"{hour_12:0{width}d}",
        "MON": MONTH_NAMES[instant.month - 1],
        "FF3": f"{instant.nanosecond // 1_000_000:03d}",
        "FF9": f"{instant.nanosecond:09d}",
        "YY": f"{instant.year % 100:02d}",
        "MM": f"{instant.month:0{width}d}",
        "DD": f"{instant.day:0{width}d}",
        "MI": f"{instant.minute:0{width}d}",
        "SS": f"{instant.second:0{width}d}",
        "AM": "PM" if instant.hour >= TWELVE else "AM",
    }
    return texts[token]


def render(*, format_text: str, instant: Instant, padded: bool = True) -> str:
    """`instant` written in a Snowflake format; double-quoted parts are literal."""
    out: list[str] = []
    index: int = 0
    while index < len(format_text):
        if format_text[index] == QUOTE:
            end: int = format_text.index(QUOTE, index + 1)
            out.append(format_text[index + 1 : end])
            index = end + 1
            continue
        token: str | None = next(
            (token for token in FORMAT_TOKENS if format_text.startswith(token, index)), None
        )
        if token is None:
            out.append(format_text[index])
            index += 1
        else:
            out.append(token_text(token=token, instant=instant, padded=padded))
            index += len(token)
    return "".join(out)


def literal(text: str) -> str:
    """A VARCHAR cell."""
    return "'" + text.replace("'", "''") + "'"


def odd_texts(temporal_format: TemporalFormat) -> tuple[str, ...]:
    """Texts near the format: padded, lower-case, unpadded, impossible, trailing junk, empty."""
    first: str = render(format_text=temporal_format.format, instant=INSTANTS_FOR_TEXT[0])
    unpadded: str = render(
        format_text=temporal_format.format, instant=INSTANTS_FOR_TEXT[5], padded=False
    )
    impossible: Instant = Instant(2023, 2, 29, 0, 0, 0, 0, 0)
    return (
        f" {first} ",
        first.lower(),
        unpadded,
        render(format_text=temporal_format.format, instant=impossible),
        f"{first}x",
        "",
    )


def valid_texts(
    *, temporal_format: TemporalFormat, instants: tuple[Instant, ...] = INSTANTS_FOR_TEXT
) -> tuple[str, ...]:
    """Every instant rendered in the format, without repeats."""
    texts: list[str] = []
    for instant in instants:
        text: str = render(format_text=temporal_format.format, instant=instant)
        if text not in texts:
            texts.append(text)
    return tuple(texts)


def isolated_years(*, function: TemporalFunction, temporal_format: TemporalFormat) -> bool:
    """Whether instants outside the nanosecond range need their own case, sparing other rows."""
    return function in TIMESTAMP_FORMAT_FUNCTIONS and any(
        token in temporal_format.format for token in SUB_MICROSECOND_FORMAT_TOKENS
    )


def format_cases(*, function: TemporalFunction, temporal_format: TemporalFormat) -> list[Case]:
    """TRY_ form over valid and odd texts; plain form over valid texts and an impossible date."""
    pattern: str = literal(temporal_format.format)
    isolated: bool = isolated_years(function=function, temporal_format=temporal_format)
    kept: tuple[Instant, ...] = tuple(
        instant for instant in INSTANTS_FOR_TEXT if not isolated or instant.year in NANOSECOND_YEARS
    )
    outside: tuple[Instant, ...] = tuple(
        instant for instant in INSTANTS_FOR_TEXT if instant not in kept
    )
    extreme_texts: tuple[str, ...] = valid_texts(temporal_format=temporal_format, instants=outside)
    extreme: tuple[Row, ...] = tuple((literal(text),) for text in extreme_texts)
    kept_texts: tuple[str, ...] = valid_texts(temporal_format=temporal_format, instants=kept)
    valid: tuple[Row, ...] = (*((literal(text),) for text in kept_texts), (NULL_CELL,))
    odd: tuple[Row, ...] = tuple((literal(text),) for text in odd_texts(temporal_format))
    plain: str = f"{function.name}(x, {pattern})"
    attempt: str = f"TRY_{function.name}(x, {pattern})"
    column: tuple[Column, ...] = (Column("x", "VARCHAR"),)
    spelled: bool = (
        function in DATE_FORMAT_FUNCTIONS and temporal_format.format in SPELLED_DATE_FORMATS
    )
    return [
        Case(
            id=f"try_{function.slug}_{temporal_format.slug}",
            description=f"{attempt} over rendered instants and near-miss texts.",
            columns=column,
            rows=(*valid, *odd),
            expression=attempt,
            tags=("null", "invalid", "boundary"),
            expected_divergence=DATE_TEXT_SPELLING
            if spelled or function in TIMESTAMP_FORMAT_FUNCTIONS
            else None,
        ),
        *spelled_cases(
            spelled=spelled, attempt=attempt, temporal_format=temporal_format, valid=valid
        ),
        Case(
            id=f"{function.slug}_{temporal_format.slug}",
            description=f"{plain} over instants rendered in the format.",
            columns=column,
            rows=valid,
            expression=plain,
            tags=("null", "boundary"),
        ),
        *(
            Case(
                id=f"{case_id}_extreme_years",
                description=f"{expression} over instants outside the nanosecond range, alone.",
                columns=column,
                rows=extreme,
                expression=expression,
                tags=("boundary",),
                expected_divergence=divergence,
            )
            for case_id, expression, divergence in (
                (f"try_{function.slug}_{temporal_format.slug}", attempt, DATE_TEXT_SPELLING),
                (f"{function.slug}_{temporal_format.slug}", plain, None),
            )
            if extreme
        ),
        Case(
            id=f"{function.slug}_{temporal_format.slug}_invalid",
            description=f"{plain} of an impossible date raises or converts.",
            columns=column,
            rows=(odd[3],),
            expression=plain,
            tags=("invalid",),
        ),
    ]


def spelled_cases(
    *, spelled: bool, attempt: str, temporal_format: TemporalFormat, valid: tuple[Row, ...]
) -> list[Case]:
    """For a format read exactly: the texts spelled in it, an impossible date and empty text."""
    impossible: Instant = Instant(2023, 2, 29, 0, 0, 0, 0, 0)
    pivots: tuple[Row, ...] = tuple(
        (literal(render(format_text=temporal_format.format, instant=instant)),)
        for instant in CENTURY_PIVOT_INSTANTS
    )
    rows: tuple[Row, ...] = (
        *valid,
        *pivots,
        (literal(render(format_text=temporal_format.format, instant=impossible)),),
        (literal(""),),
    )
    return [
        Case(
            id=f"try_to_date_{temporal_format.slug}_spelled",
            description=f"{attempt} over texts spelled exactly in the format.",
            columns=(Column("x", "VARCHAR"),),
            rows=rows,
            expression=attempt,
            tags=("null", "invalid", "boundary", "empty"),
        )
        for _ in range(spelled)
    ]


def format_file(*, function: TemporalFunction, formats: tuple[TemporalFormat, ...]) -> CaseFile:
    """One function's format cases."""
    cases: list[Case] = []
    for temporal_format in formats:
        cases.extend(format_cases(function=function, temporal_format=temporal_format))
    return CaseFile(
        generator=TEMPORAL_TEXTS,
        name=f"{function.slug}_formats",
        category="conversion",
        construct=function.construct,
        cases=tuple(cases),
        header=(f"{function.name} and TRY_{function.name} with explicit format strings.",),
    )


def auto_cases(
    *,
    spellings: tuple[Projection, ...],
    attempt: Projection,
    sure: tuple[str, ...],
    unsure: tuple[str, ...],
    unspelled: frozenset[str] = frozenset(),
) -> list[Case]:
    """AUTO detection: TRY_ over every text, each spelling over sure texts, the cast per unsure."""
    column: tuple[Column, ...] = (Column("x", "VARCHAR"),)
    sure_rows: tuple[Row, ...] = (*((literal(text),) for text in sure), (NULL_CELL,))
    cases: list[Case] = [
        Case(
            id=f"{attempt.slug}_auto",
            description=f"{attempt.template} over the texts AUTO detection may read.",
            columns=column,
            rows=(*sure_rows, *((literal(text),) for text in unsure if text not in unspelled)),
            expression=attempt.template,
            tags=("null", "invalid", "boundary"),
        ),
        *(
            _unspelled_case(attempt=attempt, number=index + 1, text=text)
            for index, text in enumerate(sorted(unspelled))
        ),
    ]
    cases.extend(
        Case(
            id=f"{spelling.slug}_auto",
            description=f"{spelling.template} over texts in the standard formats.",
            columns=column,
            rows=sure_rows,
            expression=spelling.template,
            tags=("null", "boundary"),
        )
        for spelling in spellings
    )
    cast: Projection = spellings[0]
    cases.extend(
        Case(
            id=f"{cast.slug}_auto_{index + 1}",
            description=f"{cast.template} of the text {literal(text)} raises or converts.",
            columns=column,
            rows=((literal(text),),),
            expression=cast.template,
            tags=("invalid",),
        )
        for index, text in enumerate(unsure)
    )
    return cases


def temporal_text_files() -> list[CaseFile]:
    """Format files per function, and AUTO detection for DATE and TIMESTAMP_NTZ."""
    files: list[CaseFile] = [
        *(
            format_file(function=function, formats=DATE_FORMATS)
            for function in DATE_FORMAT_FUNCTIONS
        ),
        *(
            format_file(function=function, formats=TIMESTAMP_FORMATS)
            for function in TIMESTAMP_FORMAT_FUNCTIONS
        ),
    ]
    files.append(
        CaseFile(
            generator=TEMPORAL_TEXTS,
            name="date_auto",
            category="conversion",
            construct="cast:DATE",
            cases=tuple(
                auto_cases(
                    spellings=AUTO_DATE_SPELLINGS,
                    attempt=AUTO_DATE_ATTEMPT,
                    sure=AUTO_DATE_TEXTS,
                    unsure=UNSURE_DATE_TEXTS,
                    unspelled=UNSPELLED_DATE_TEXTS,
                )
            ),
            header=("Text to DATE without a format, under the session's AUTO input format.",),
        )
    )
    files.append(
        CaseFile(
            generator=TEMPORAL_TEXTS,
            name="timestamp_auto",
            category="conversion",
            construct="cast:TIMESTAMP_NTZ",
            cases=timestamp_auto_cases(),
            header=(
                "Text to TIMESTAMP_NTZ without a format, under the session's AUTO input format.",
            ),
        )
    )
    return files


def timestamp_auto_cases() -> tuple[Case, ...]:
    """AUTO detection to TIMESTAMP_NTZ; the TRY_ form raises for spellings no recording shows."""
    attempt, *spellings = auto_cases(
        spellings=AUTO_TIMESTAMP_SPELLINGS,
        attempt=AUTO_TIMESTAMP_ATTEMPT,
        sure=AUTO_TIMESTAMP_TEXTS,
        unsure=UNSURE_TIMESTAMP_TEXTS,
    )
    return (replace(attempt, expected_divergence=DATE_TEXT_SPELLING), *spellings)


def slug(zone: str) -> str:
    """A lower-case identifier fragment for a zone name."""
    return zone.split("/")[-1].lower()


def pair_case(*, source: str, target: str) -> Case:
    """Naive local times in `source` converted to `target`."""
    return Case(
        id=f"{slug(source)}_to_{slug(target)}",
        description=(
            f"Naive local times in {source}, including DST gaps and repeats, converted to {target}."
        ),
        columns=(Column("t", "TIMESTAMP_NTZ(9)"),),
        rows=tuple((value,) for value in LOCAL_TIMES),
        expression=f"CONVERT_TIMEZONE('{source}', '{target}', t)",
        tags=("null", "boundary"),
    )


def instant_case(target: str) -> Case:
    """Zoned instants converted to `target`."""
    return Case(
        id=f"instant_to_{slug(target)}",
        description=(f"Zoned instants either side of DST transitions, converted to {target}."),
        columns=(Column("t", "TIMESTAMP_TZ(9)"),),
        rows=tuple((value,) for value in INSTANTS),
        expression=f"CONVERT_TIMEZONE('{target}', t)",
        tags=("null", "boundary"),
    )


def zone_files() -> list[CaseFile]:
    """Three-argument conversion for every ordered zone pair, two-argument for every target."""
    cases: list[Case] = []
    for source in ZONES:
        cases.extend(
            pair_case(source=source, target=target) for target in ZONES if target != source
        )
    cases.extend(instant_case(target) for target in ZONES)
    return [
        CaseFile(
            generator=TIME_ZONES,
            name="convert_timezone",
            category="temporal",
            construct="fn:CONVERT_TIMEZONE",
            cases=tuple(cases),
            header=("CONVERT_TIMEZONE across zone pairs and DST transitions.",),
        )
    ]


def _unspelled_case(*, attempt: Projection, number: int, text: str) -> Case:
    """The AUTO attempt on one unrecorded text: raising with digits, NULL without."""
    digits: bool = any(character.isdigit() for character in text)
    outcome: str = "a spelling no recording shows, raises" if digits else "without a digit, is NULL"
    return Case(
        id=f"{attempt.slug}_auto_unspelled_{number}",
        description=f"{attempt.template} of the text {literal(text)}, {outcome}.",
        columns=(Column("x", "VARCHAR"),),
        rows=((literal(text),),),
        expression=attempt.template,
        tags=("invalid",),
        expected_divergence=DATE_TEXT_SPELLING if digits else None,
    )
