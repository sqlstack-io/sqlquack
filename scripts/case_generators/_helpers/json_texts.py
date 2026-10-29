"""JSON text grids: PARSE_JSON's lenience, numbers, times, and how JSON is written as text."""

from __future__ import annotations

import json
from decimal import Decimal

from scripts.case_generators._helpers.json_paths import (
    divergence_for,
    fidelity_trigger,
    pattern_trigger,
)
from scripts.case_generators.constants import (
    AUTO_TIMESTAMP_TEXTS,
    COMPOSED_DEPTHS,
    COMPOSED_ORDER_DOCUMENTS,
    COMPOSED_RESORT_DOCUMENTS,
    COMPOSED_UNPACK_QUERY,
    DATE_PROJECTIONS,
    DATE_TEXT_SPELLING,
    DECIMAL_TOKENS,
    DOUBLE_PROJECTIONS,
    DOUBLE_TOKENS,
    FLOAT_RAISING_TOKENS,
    FLOAT_TEXT,
    FLOAT_VARIANT_PROJECTIONS,
    FLOAT_WRITTEN_PROJECTIONS,
    GUARD_BOUNDARY_TOKENS,
    GUARD_STRING_DOCUMENTS,
    GUARD_TRIGGER_DOCUMENTS,
    INT64_MIN,
    JSON_COMPOSED,
    JSON_DOUBLES,
    JSON_EPOCH_DOCUMENT,
    JSON_FIDELITY,
    JSON_KEY_ORDER,
    JSON_LENIENCE,
    JSON_NUMBERS,
    JSON_TEMPORAL,
    JSON_TEMPORAL_CASTS,
    JSON_TEMPORAL_DIVERGENCES,
    JSON_TEMPORAL_DOCUMENTS,
    KEY_ORDER_COMPARISONS,
    KEY_ORDER_CONSTRUCTED,
    KEY_ORDER_DOCUMENTS,
    KEY_ORDER_PROJECTIONS,
    LENIENCE_PROJECTIONS,
    LENIENT_JSON_TEXTS,
    MIXED_NUMBER_DOCUMENTS,
    NESTED_LEVELS,
    NESTED_TEMPORAL_DOCUMENTS,
    NULL_CELL,
    NUMBER_PROJECTIONS,
    NUMBER_TOKENS,
    ROUND_TRIP_PROJECTIONS,
    ROUND_TRIP_TOKENS,
    SNOWFLAKE_REJECTED_JSON_TEXTS,
    TYPE_PROJECTION,
    UINT64_MAX,
    UNENCODABLE_JSON_TEXTS,
    UNSURE_TIMESTAMP_TEXTS,
    WIDE_KEYS,
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
                if not pattern_trigger(text)
                and (projection.slug == TYPE_PROJECTION or text not in UNENCODABLE_JSON_TEXTS)
            ),
            (NULL_CELL,),
        ),
        expression=expression,
        tags=("null", "invalid"),
    )


def lenient_guard_cases() -> list[Case]:
    """TYPEOF(TRY_PARSE_JSON(x)) of each lenient text the JSON guard raises for, alone."""
    triggered: list[str] = [text for text in LENIENT_JSON_TEXTS if pattern_trigger(text)]
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
        token for token in all_tokens() if pattern_trigger(number_document(token))
    ]
    numbers: list[Case] = [
        variant_case(
            case_id=f"guard_number_{index + 1}",
            document=number_document(token),
            expression="v:n::VARCHAR",
            divergent=fidelity_trigger(number_document(token)),
        )
        for index, token in enumerate(triggered)
    ]
    documents: list[Case] = [
        variant_case(
            case_id=f"guard_document_{index + 1}",
            document=document,
            expression="TO_JSON(v)",
            divergent=document not in SNOWFLAKE_REJECTED_JSON_TEXTS and fidelity_trigger(document),
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
        if not pattern_trigger(number_document(token))
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
        expected_divergence=JSON_TEMPORAL_DIVERGENCES.get(projection.slug),
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
            expected_divergence=DATE_TEXT_SPELLING,
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


def parsed_document(document: str) -> str:
    """A cell holding `document` parsed as a VARIANT."""
    return f"PARSE_JSON({text_literal(document)})"


def deep_document() -> str:
    """Objects nested `NESTED_LEVELS` deep, each listing key b before key a."""
    document: str = '{"b":1,"a":2}'
    for _ in range(NESTED_LEVELS):
        document = f'{{"b":{document},"a":0}}'
    return document


def wide_document() -> str:
    """One object of `WIDE_KEYS` keys written in descending order."""
    members: list[str] = [f'"k{index:02d}":{index}' for index in reversed(range(WIDE_KEYS))]
    return "{" + ",".join(members) + "}"


def documents() -> tuple[tuple[str, str], ...]:
    """Every document of the key-order family, by name."""
    return (*KEY_ORDER_DOCUMENTS, ("deep_nesting", deep_document()), ("wide", wide_document()))


def document_case(*, name: str, document: str) -> Case:
    """One document alone, written as text every way Snowflake writes a VARIANT."""
    return Case(
        id=f"key_order_{name}",
        description=f"The document {text_literal(document)} written as text.",
        columns=(Column("v", "VARIANT"),),
        rows=((parsed_document(document),),),
        query=KEY_ORDER_PROJECTIONS,
        order="ordered",
        tags=("nested",),
        expected_divergence=JSON_FIDELITY if resort_rewrites(document) else None,
    )


def resort_rewrites(document: str) -> bool:
    """Whether resorting `document`'s keys through DuckDB's VARIANT would rewrite an integer."""
    value: object = json.loads(document, parse_int=int)
    return keys_out_of_order(value) and integers_beyond_64_bits(value)


def keys_out_of_order(value: object) -> bool:
    """Whether some object within `value` lists its keys out of UTF-8 byte order."""
    if isinstance(value, dict):
        keys: list[bytes] = [key.encode() for key in value]
        return keys != sorted(keys) or any(keys_out_of_order(item) for item in value.values())
    if isinstance(value, list):
        return any(keys_out_of_order(item) for item in value)
    return False


def integers_beyond_64_bits(value: object) -> bool:
    """Whether `value` holds an integer outside the signed and unsigned 64-bit ranges."""
    if isinstance(value, dict):
        return any(integers_beyond_64_bits(item) for item in value.values())
    if isinstance(value, list):
        return any(integers_beyond_64_bits(item) for item in value)
    return (
        isinstance(value, int)
        and not isinstance(value, bool)
        and not INT64_MIN <= value <= UINT64_MAX
    )


def null_case() -> Case:
    """JSON null and SQL NULL written as text."""
    return Case(
        id="key_order_nulls",
        description="JSON null and SQL NULL written as text.",
        columns=(Column("v", "VARIANT"),),
        rows=((parsed_document("null"),), (NULL_CELL,)),
        query=KEY_ORDER_PROJECTIONS,
        order="ordered",
        tags=("null", "json-null"),
    )


def constructed_case(*, name: str, expression: str) -> Case:
    """An object built in SQL, written as text."""
    return Case(
        id=f"key_order_{name}",
        description=f"{expression} written as text.",
        columns=(Column("x", "NUMBER(1,0)"),),
        rows=(("1",),),
        query=(
            f"SELECT TO_VARCHAR({expression}) AS t, TO_JSON({expression}) AS j, "
            f"{expression} AS raw FROM input ORDER BY ordinal"
        ),
        order="ordered",
        tags=("nested",),
    )


def null_value_case() -> Case:
    """SQL NULL values dropped from a built object, or kept as JSON null."""
    return Case(
        id="key_order_null_values",
        description="OBJECT_CONSTRUCT drops a NULL value; OBJECT_CONSTRUCT_KEEP_NULL keeps null.",
        columns=(Column("x", "NUMBER(1,0)"),),
        rows=(("NULL",), ("1",)),
        query=(
            "SELECT TO_VARCHAR(OBJECT_CONSTRUCT('b', x, 'a', 1)) AS t, "
            "TO_JSON(OBJECT_CONSTRUCT_KEEP_NULL('b', x, 'a', 1)) AS j FROM input ORDER BY ordinal"
        ),
        order="ordered",
        tags=("null", "json-null", "nested"),
    )


def comparison_case(*, name: str, projections: str) -> Case:
    """Comparisons of documents written with keys in different orders."""
    return Case(
        id=f"key_order_{name}",
        description=f"Comparisons of reordered documents: {name.replace('_', ' ')}.",
        columns=(Column("x", "NUMBER(1,0)"),),
        rows=(("1",),),
        query=f"SELECT {projections} FROM input ORDER BY ordinal",
        order="ordered",
        tags=("nested",),
    )


def join_case() -> Case:
    """Rows joined and grouped on documents whose keys are written in different orders."""
    rows: tuple[tuple[str, ...], ...] = (
        ("'l'", "1", parsed_document('{"b":1,"a":2}')),
        ("'l'", "2", parsed_document('{"a":[{"d":1,"c":2}]}')),
        ("'r'", "1", parsed_document('{"a":2,"b":1}')),
        ("'r'", "2", parsed_document('{"a":[{"c":2,"d":1}]}')),
        ("'r'", "3", parsed_document('{"a":2,"b":1}')),
    )
    return Case(
        id="key_order_join",
        description="A join on the text of reordered documents, and a count of distinct texts.",
        columns=(Column("side", "VARCHAR"), Column("k", "NUMBER(2,0)"), Column("v", "VARIANT")),
        rows=rows,
        query=(
            "SELECT l.k AS lk, r.k AS rk, TO_JSON(l.v) AS j FROM input AS l JOIN input AS r "
            "ON TO_JSON(l.v) = TO_JSON(r.v) WHERE l.side = 'l' AND r.side = 'r' "
            "ORDER BY lk, rk"
        ),
        order="ordered",
        tags=("nested",),
    )


def key_order_files() -> list[CaseFile]:
    """Documents, built objects and comparisons whose text depends on the order of keys."""
    return [
        CaseFile(
            generator=JSON_KEY_ORDER,
            name="documents",
            category="semi-structured-output",
            construct="fn:TO_JSON",
            cases=(
                *(document_case(name=name, document=document) for name, document in documents()),
                null_case(),
            ),
            header=("Each document alone as TO_VARCHAR, ::VARCHAR, TO_JSON, MD5 and a result.",),
        ),
        CaseFile(
            generator=JSON_KEY_ORDER,
            name="constructed",
            category="semi-structured-output",
            construct="fn:OBJECT_CONSTRUCT",
            cases=(
                *(
                    constructed_case(name=name, expression=expression)
                    for name, expression in KEY_ORDER_CONSTRUCTED
                ),
                *(
                    comparison_case(name=name, projections=projections)
                    for name, projections in KEY_ORDER_COMPARISONS
                ),
                join_case(),
                null_value_case(),
            ),
            header=("Objects built in SQL, comparisons and joins over reordered documents.",),
        ),
    ]


def double_case(*, name: str, token: str, projections: str = DOUBLE_PROJECTIONS) -> Case:
    """One number token, at key n and nested in an array, under every projection."""
    documents: tuple[str, str] = (f'{{"n":{token}}}', f'[1,{{"m":[{token},2]}}]')
    return Case(
        id=f"number_{name}",
        description=f"The JSON number {token} read and written as text.",
        columns=(Column("v", "VARIANT"), Column("w", "VARIANT")),
        rows=(tuple(parsed_document(document) for document in documents),),
        query=projections,
        order="ordered",
        tags=("boundary",),
        expected_divergence=JSON_FIDELITY if any(map(fidelity_trigger, documents)) else None,
    )


def mixed_number_case(*, name: str, document: str) -> Case:
    """A document holding long and short numbers together, written as text."""
    return Case(
        id=f"number_mixed_{name}",
        description=f"The document {text_literal(document)} written as text.",
        columns=(Column("v", "VARIANT"),),
        rows=((parsed_document(document),),),
        query="SELECT TO_JSON(v) AS j, v::VARCHAR AS c, v AS raw FROM input ORDER BY ordinal",
        order="ordered",
        tags=("boundary", "nested"),
        expected_divergence=JSON_FIDELITY if fidelity_trigger(document) else None,
    )


def missing_case(nested: str) -> Case:
    """JSON null, a missing key and SQL NULL under every projection, `nested` beside them."""
    return Case(
        id="number_missing",
        description="JSON null, a missing key and SQL NULL read and written as text.",
        columns=(Column("v", "VARIANT"), Column("w", "VARIANT")),
        rows=(
            (parsed_document('{"n":null}'), parsed_document(nested)),
            (parsed_document("{}"), parsed_document("[]")),
            (NULL_CELL, NULL_CELL),
        ),
        query=DOUBLE_PROJECTIONS,
        order="ordered",
        tags=("null", "json-null", "missing-key"),
    )


def float_variant_case(*, name: str, token: str) -> Case:
    """A SQL FLOAT put into a VARIANT, an object and an array, written as text."""
    return Case(
        id=f"float_{name}",
        description=f"The FLOAT {token} inside a VARIANT written as text.",
        columns=(Column("x", "FLOAT"),),
        rows=((token,),),
        query=FLOAT_VARIANT_PROJECTIONS,
        order="ordered",
        tags=("boundary",),
        expected_divergence=JSON_FIDELITY if name in FLOAT_RAISING_TOKENS else FLOAT_TEXT,
    )


def float_written_case() -> Case:
    """Every FLOAT that converts, over rows Snowflake cannot fold, written in VARIANT documents."""
    return Case(
        id="float_written_rows",
        description="FLOAT values inside a VARIANT, an object and an array, written as JSON.",
        columns=(Column("x", "FLOAT"),),
        rows=tuple((token,) for name, token in DOUBLE_TOKENS if name not in FLOAT_RAISING_TOKENS),
        query=FLOAT_WRITTEN_PROJECTIONS,
        order="ordered",
        tags=("boundary",),
    )


def float_null_case() -> Case:
    """A NULL FLOAT put into a VARIANT, an object and an array."""
    return Case(
        id="float_null",
        description="A NULL FLOAT inside a VARIANT, an object and an array, written as text.",
        columns=(Column("x", "FLOAT"),),
        rows=((NULL_CELL,),),
        query=FLOAT_VARIANT_PROJECTIONS,
        order="ordered",
        tags=("null", "json-null", "missing-key"),
        expected_divergence=JSON_FIDELITY,
    )


def double_files() -> list[CaseFile]:
    """Number tokens Snowflake keeps as doubles, and long decimals it may keep exactly."""
    return [
        CaseFile(
            generator=JSON_DOUBLES,
            name="doubles",
            category="json-number",
            construct="fn:PARSE_JSON",
            cases=(
                *(double_case(name=name, token=token) for name, token in DOUBLE_TOKENS),
                missing_case("[null]"),
            ),
            header=("Exponent number tokens, which Snowflake keeps as doubles, written as text.",),
        ),
        CaseFile(
            generator=JSON_DOUBLES,
            name="decimals",
            category="json-number",
            construct="fn:PARSE_JSON",
            cases=(
                *(double_case(name=name, token=token) for name, token in DECIMAL_TOKENS),
                missing_case('[{"m":null}]'),
            ),
            header=("Integers and decimals of 17 to 41 digits, written as text.",),
        ),
        CaseFile(
            generator=JSON_DOUBLES,
            name="floats",
            category="json-number",
            construct="fn:TO_VARIANT",
            cases=(
                *(float_variant_case(name=name, token=token) for name, token in DOUBLE_TOKENS),
                float_null_case(),
                float_written_case(),
            ),
            header=("SQL FLOAT values inside VARIANTs, written as text.",),
        ),
        CaseFile(
            generator=JSON_DOUBLES,
            name="round_trip",
            category="json-number",
            construct="fn:PARSE_JSON",
            cases=(
                *(
                    double_case(name=name, token=token, projections=ROUND_TRIP_PROJECTIONS)
                    for name, token in ROUND_TRIP_TOKENS
                ),
                *(
                    mixed_number_case(name=name, document=document)
                    for name, document in MIXED_NUMBER_DOCUMENTS
                ),
                missing_case('[{"m":null}]'),
            ),
            header=("Decimals of 16 and 17 digits, most written back unchanged by a double.",),
        ),
    ]


def chained(*, depth: int, writer: str) -> str:
    """`v` written as text by `writer` and parsed again, `depth` times over."""
    expression: str = "v"
    for _ in range(depth):
        expression = f"PARSE_JSON({writer}({expression}))"
    return expression


def nesting_case(depth: int) -> Case:
    """Order documents through `depth` rounds of PARSE_JSON over TO_VARCHAR and TO_JSON."""
    by_text: str = chained(depth=depth, writer="TO_VARCHAR")
    by_json: str = chained(depth=depth, writer="TO_JSON")
    return Case(
        id=f"composed_nesting_{depth}",
        description=f"Order documents written and parsed again {depth} times, then as text.",
        columns=(Column("v", "VARIANT"),),
        rows=document_rows_with_nulls(
            tuple(document for document in COMPOSED_ORDER_DOCUMENTS if not document.startswith('"'))
        ),
        query=(
            f"SELECT TO_JSON({by_text}) AS j, {by_text}::VARCHAR AS c, "
            f"TO_VARCHAR({by_json}) AS t, MD5(TO_JSON({by_json})) AS m "
            "FROM input ORDER BY ordinal"
        ),
        order="ordered",
        tags=("null", "json-null", "nested"),
    )


def document_rows_with_nulls(documents: tuple[str, ...]) -> tuple[Row, ...]:
    """Each document parsed, then JSON null and SQL NULL."""
    return (
        *((parsed_document(document),) for document in documents),
        (parsed_document("null"),),
        (NULL_CELL,),
    )


def building_case() -> Case:
    """Objects built over parsed order documents, then written as text."""
    return Case(
        id="composed_built_objects",
        description="OBJECT_CONSTRUCT and OBJECT_INSERT over parsed order documents, as text.",
        columns=(Column("v", "VARIANT"),),
        rows=document_rows_with_nulls(COMPOSED_ORDER_DOCUMENTS[:2]),
        query=(
            "SELECT TO_VARCHAR(OBJECT_CONSTRUCT('status', v:status, 'Customer', v:customer, "
            "'items', v:items, 'all', v)) AS t, "
            "TO_JSON(OBJECT_INSERT(v, 'Zone', v:customer)) AS j, "
            "OBJECT_INSERT(PARSE_JSON(TO_VARCHAR(v)), 'aa', v:items)::VARCHAR AS c, "
            "TO_JSON(OBJECT_CONSTRUCT('z', OBJECT_INSERT(v, 'B', v:order_id), 'a', 1)) AS n "
            "FROM input ORDER BY ordinal"
        ),
        order="ordered",
        tags=("null", "json-null", "nested"),
    )


def unpacking_case() -> Case:
    """Many VARIANT columns unpacked from flattened order lines, some written as text."""
    return Case(
        id="composed_unpacking",
        description="Order documents unpacked into many columns over their flattened items.",
        columns=(Column("v", "VARIANT"),),
        rows=document_rows_with_nulls(COMPOSED_ORDER_DOCUMENTS[:2]),
        query=COMPOSED_UNPACK_QUERY,
        order="ordered",
        tags=("null", "json-null", "nested"),
    )


def composed_files() -> list[CaseFile]:
    """VARIANT text written through composed expressions, and through DuckDB's resorting."""
    return [
        CaseFile(
            generator=JSON_COMPOSED,
            name="composed",
            category="semi-structured-output",
            construct="fn:TO_VARCHAR",
            cases=(
                *(nesting_case(depth) for depth in COMPOSED_DEPTHS),
                building_case(),
                unpacking_case(),
            ),
            header=("VARIANT text written through nested and composed expressions.",),
        ),
        CaseFile(
            generator=JSON_COMPOSED,
            name="resorted",
            category="semi-structured-output",
            construct="fn:TO_JSON",
            cases=(
                *(
                    document_case(name=name, document=document)
                    for name, document in COMPOSED_RESORT_DOCUMENTS
                ),
                null_case(),
            ),
            header=("Documents whose keys are resorted, beside numbers kept exactly or not.",),
        ),
    ]
