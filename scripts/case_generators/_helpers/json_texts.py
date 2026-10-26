"""JSON text grids: PARSE_JSON's lenience, how it keeps numbers, and JSON text as times."""

from __future__ import annotations

import json
from decimal import Decimal

from scripts.case_generators._helpers.json_paths import divergence_for, fidelity_trigger
from scripts.case_generators.constants import (
    AUTO_TIMESTAMP_TEXTS,
    DATE_PROJECTIONS,
    GUARD_BOUNDARY_TOKENS,
    GUARD_STRING_DOCUMENTS,
    GUARD_TRIGGER_DOCUMENTS,
    JSON_EPOCH_DOCUMENT,
    JSON_FIDELITY,
    JSON_LENIENCE,
    JSON_NUMBERS,
    JSON_TEMPORAL,
    JSON_TEMPORAL_CASTS,
    JSON_TEMPORAL_DOCUMENTS,
    LENIENCE_PROJECTIONS,
    LENIENT_JSON_TEXTS,
    NESTED_TEMPORAL_DOCUMENTS,
    NULL_CELL,
    NUMBER_PROJECTIONS,
    NUMBER_TOKENS,
    SNOWFLAKE_REJECTED_JSON_TEXTS,
    TYPE_PROJECTION,
    UNENCODABLE_JSON_TEXTS,
    UNSURE_TIMESTAMP_TEXTS,
)
from scripts.case_generators.models import (
    Case,
    CaseFile,
    Column,
    NumberProjection,
    Projection,
)
from scripts.case_generators.types import Row


def text_literal(text: str) -> str:
    """A Snowflake string literal holding `text` exactly."""
    return "'" + text.replace("\\", "\\\\").replace("'", "''") + "'"


def lenient_parse_case(index: int) -> Case:
    """PARSE_JSON of one text alone, which Snowflake may read leniently or reject."""
    text: str = LENIENT_JSON_TEXTS[index]
    projection: str = "TYPEOF" if text in UNENCODABLE_JSON_TEXTS else "TO_JSON"
    return Case(
        id=f"parse_json_lenient_{index + 1}",
        description=f"PARSE_JSON of the text {text_literal(text)} raises or converts.",
        columns=(Column("x", "VARCHAR"),),
        rows=((text_literal(text),),),
        expression=f"{projection}(PARSE_JSON(x))",
        tags=("invalid",),
        expected_divergence=divergence_for(text=text, accepted=False),
    )


def lenient_attempt_case(projection: Projection) -> Case:
    """A projection of TRY_PARSE_JSON over every lenient text."""
    expression: str = projection.template.format(path="TRY_PARSE_JSON(x)")
    return Case(
        id=f"try_parse_json_lenient_{projection.slug}",
        description=f"{expression} over text Snowflake may read leniently or reject.",
        columns=(Column("x", "VARCHAR"),),
        rows=(
            *(
                (text_literal(text),)
                for text in LENIENT_JSON_TEXTS
                if not fidelity_trigger(text)
                and (projection.slug == TYPE_PROJECTION or text not in UNENCODABLE_JSON_TEXTS)
            ),
            (NULL_CELL,),
        ),
        expression=expression,
        tags=("null", "invalid"),
    )


def lenient_guard_cases() -> list[Case]:
    """TYPEOF(TRY_PARSE_JSON(x)) of each lenient text the JSON guard raises for, alone."""
    triggered: list[str] = [text for text in LENIENT_JSON_TEXTS if fidelity_trigger(text)]
    return [
        Case(
            id=f"try_parse_json_lenient_guard_{index + 1}",
            description=(
                f"TRY_PARSE_JSON of the text {text_literal(text)}, which the JSON guard raises for."
            ),
            columns=(Column("x", "VARCHAR"),),
            rows=((text_literal(text),),),
            expression="TYPEOF(TRY_PARSE_JSON(x))",
            tags=("invalid",),
            expected_divergence=JSON_FIDELITY,
        )
        for index, text in enumerate(triggered)
    ]


def lenience_files() -> list[CaseFile]:
    """PARSE_JSON of each lenient text alone, and TRY_PARSE_JSON over all of them."""
    null_case: Case = Case(
        id="parse_json_lenient_null",
        description="PARSE_JSON of SQL NULL text.",
        columns=(Column("x", "VARCHAR"),),
        rows=((NULL_CELL,),),
        expression="TO_JSON(PARSE_JSON(x))",
        tags=("null",),
    )
    return [
        CaseFile(
            generator=JSON_LENIENCE,
            name="parse_json",
            category="json-text",
            construct="fn:PARSE_JSON",
            cases=(
                *(lenient_parse_case(index) for index in range(len(LENIENT_JSON_TEXTS))),
                null_case,
            ),
            header=("PARSE_JSON of each text a strict JSON parser rejects or reads differently.",),
        ),
        CaseFile(
            generator=JSON_LENIENCE,
            name="try_parse_json",
            category="json-text",
            construct="fn:TRY_PARSE_JSON",
            cases=(
                *(lenient_attempt_case(projection) for projection in LENIENCE_PROJECTIONS),
                *lenient_guard_cases(),
            ),
            header=("TRY_PARSE_JSON over text a strict JSON parser rejects or reads differently.",),
        ),
    ]


def all_tokens() -> tuple[str, ...]:
    """The number tokens and the guard's boundary tokens, without repeats."""
    return tuple(dict.fromkeys((*NUMBER_TOKENS, *GUARD_BOUNDARY_TOKENS)))


def number_document(token: str) -> str:
    """A document holding `token` at key n."""
    return f'{{"n":{token}}}'


def variant_case(*, case_id: str, document: str, expression: str, divergent: bool) -> Case:
    """One document alone under `expression`, declaring json_fidelity when the guard raises."""
    return Case(
        id=case_id,
        description=f"{expression} of {document}, which the JSON guard raises for.",
        columns=(Column("v", "VARIANT"),),
        rows=((f"PARSE_JSON({text_literal(document)})",),),
        expression=expression,
        tags=("boundary",),
        expected_divergence=JSON_FIDELITY if divergent else None,
    )


def guard_cases() -> list[Case]:
    """Clean documents written exactly, and each document the guard raises for, alone."""
    clean: Case = Case(
        id="guard_passes",
        description="TO_JSON of documents the guard passes: guard-like strings, sorted keys.",
        columns=(Column("v", "VARIANT"),),
        rows=(
            *(
                (f"PARSE_JSON({text_literal(document)})",)
                for document in (*GUARD_STRING_DOCUMENTS, '{"n":null}', "{}")
            ),
            (NULL_CELL,),
        ),
        expression="TO_JSON(v)",
        tags=("null", "json-null", "missing-key", "boundary"),
    )
    triggered: list[str] = [
        token for token in all_tokens() if fidelity_trigger(number_document(token))
    ]
    numbers: list[Case] = [
        variant_case(
            case_id=f"guard_number_{index + 1}",
            document=number_document(token),
            expression="v:n::VARCHAR",
            divergent=True,
        )
        for index, token in enumerate(triggered)
    ]
    documents: list[Case] = [
        variant_case(
            case_id=f"guard_document_{index + 1}",
            document=document,
            expression="TO_JSON(v)",
            divergent=document not in SNOWFLAKE_REJECTED_JSON_TEXTS,
        )
        for index, document in enumerate(GUARD_TRIGGER_DOCUMENTS)
    ]
    return [clean, *numbers, *documents]


def number_rows(projection: NumberProjection) -> tuple[Row, ...]:
    """Documents holding each number token the projection's target holds, JSON null and none."""
    limit: Decimal | None = projection.limit
    tokens: tuple[str, ...] = tuple(
        token
        for token in all_tokens()
        if not fidelity_trigger(number_document(token))
        and (limit is None or Decimal(token).copy_abs() < limit)
    )
    documents: tuple[str, ...] = (*(number_document(token) for token in tokens), '{"n":null}', "{}")
    return (*((f"PARSE_JSON({text_literal(document)})",) for document in documents), (NULL_CELL,))


def number_case(projection: NumberProjection) -> Case:
    """One projection of the number at `v:n` over every token."""
    return Case(
        id=f"number_{projection.slug}",
        description=f"{projection.expression} over integer, decimal and exponent number tokens.",
        columns=(Column("v", "VARIANT"),),
        rows=number_rows(projection),
        expression=projection.expression,
        tags=("null", "json-null", "missing-key", "boundary"),
    )


def number_files() -> list[CaseFile]:
    """The number tokens under each projection."""
    return [
        CaseFile(
            generator=JSON_NUMBERS,
            name="numbers",
            category="json-number",
            construct="fn:GET_PATH",
            cases=tuple(number_case(projection) for projection in NUMBER_PROJECTIONS),
            header=("How PARSE_JSON keeps each number token, seen through each projection.",),
        ),
        CaseFile(
            generator=JSON_NUMBERS,
            name="guard",
            category="json-number",
            construct="fn:PARSE_JSON",
            cases=tuple(guard_cases()),
            header=(
                "Each trigger of the JSON guard, its boundary, and text it must not trigger on.",
            ),
        ),
    ]


def document_cell(document: str) -> str:
    """A VARIANT cell parsing `document`."""
    return "PARSE_JSON('" + document.replace("\\", "\\\\").replace("'", "''") + "')"


def text_document(text: str) -> str:
    """A document holding `text` at key t."""
    return json.dumps({"t": text}, ensure_ascii=False)


def sure_case(projection: Projection) -> Case:
    """A cast of v:t over standard texts, JSON null, a missing key, a number and a cased key."""
    documents: tuple[str, ...] = (
        *(text_document(text) for text in AUTO_TIMESTAMP_TEXTS),
        *(
            document
            for document in JSON_TEMPORAL_DOCUMENTS
            if projection.slug not in DATE_PROJECTIONS or document != JSON_EPOCH_DOCUMENT
        ),
    )
    rows: tuple[Row, ...] = (*((document_cell(document),) for document in documents), (NULL_CELL,))
    return Case(
        id=projection.slug,
        description=f"{projection.template} over standard texts, JSON null, no key and a number.",
        columns=(Column("v", "VARIANT"),),
        rows=rows,
        expression=projection.template,
        tags=("null", "json-null", "missing-key", "case-sensitive-key"),
    )


def unsure_case(*, projection: Projection, index: int) -> Case:
    """A cast of v:t over one text AUTO detection may reject."""
    text: str = UNSURE_TIMESTAMP_TEXTS[index]
    return Case(
        id=f"{projection.slug}_unsure_{index + 1}",
        description=f"{projection.template} of the JSON text {json.dumps(text)}: raises or not.",
        columns=(Column("v", "VARIANT"),),
        rows=((document_cell(text_document(text)),),),
        expression=projection.template,
        tags=("invalid",),
    )


def json_temporal_files() -> list[CaseFile]:
    """Each cast over the sure texts; the NTZ cast over each unsure text alone."""
    cases: list[Case] = [sure_case(projection) for projection in JSON_TEMPORAL_CASTS]
    cases.append(
        Case(
            id=f"{JSON_TEMPORAL_CASTS[3].slug}_unsure",
            description=f"{JSON_TEMPORAL_CASTS[3].template} over texts AUTO detection may reject.",
            columns=(Column("v", "VARIANT"),),
            rows=tuple((document_cell(text_document(text)),) for text in UNSURE_TIMESTAMP_TEXTS),
            expression=JSON_TEMPORAL_CASTS[3].template,
            tags=("invalid",),
        )
    )
    cases.append(
        Case(
            id="nested_as_timestamp_ntz",
            description="v:e[0].t::TIMESTAMP_NTZ through object arrays, past the end, JSON null.",
            columns=(Column("v", "VARIANT"),),
            rows=(
                *((document_cell(document),) for document in NESTED_TEMPORAL_DOCUMENTS),
                (NULL_CELL,),
            ),
            expression="v:e[0].t::TIMESTAMP_NTZ",
            tags=("null", "json-null", "nested", "array-index", "missing-key"),
        )
    )
    cases.extend(
        unsure_case(projection=JSON_TEMPORAL_CASTS[1], index=index)
        for index in range(len(UNSURE_TIMESTAMP_TEXTS))
    )
    return [
        CaseFile(
            generator=JSON_TEMPORAL,
            name="casts",
            category="semi-structured",
            construct="fn:GET_PATH",
            cases=tuple(cases),
            header=("JSON text values cast to DATE, TIMESTAMP_NTZ and TIMESTAMP_TZ.",),
        )
    ]
