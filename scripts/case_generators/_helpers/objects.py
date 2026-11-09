"""OBJECT_DELETE, OBJECT_INSERT and OBJECT_CONSTRUCT grids, typed values in VARIANTs, outputs."""

from __future__ import annotations

from scripts.case_generators._helpers.json_texts import text_literal
from scripts.case_generators.constants import (
    CONSTRUCT_GRIDS,
    CONVERTED_TYPED_SOURCES,
    ELEMENT_TYPED_WRAPPERS,
    EXCEPT_PAIRS,
    EXCEPT_UNSURE_PAIRS,
    FLOAT_NUMBER_VALUES,
    FLOAT_TEXT,
    FLOAT_TEXT_TYPED_READS,
    FLOAT_TEXT_VALUES,
    INSERT_KEYS,
    INSERT_TARGETS,
    JSON_FIDELITY,
    KIND_CAST_TARGETS,
    KIND_CHECKS,
    KIND_MISMATCH_DOCUMENTS,
    NOT_OBJECT_DOCUMENTS,
    NULL_CELL,
    OBJECT_DOCUMENTS,
    OBJECT_FUNCTIONS,
    OBJECT_KEYS,
    OUTPUT_DOCUMENTS,
    PICK_KEY_LISTS,
    RAISING_TYPED_READS,
    RAISING_TYPED_SOURCES,
    TYPED_CHECKS,
    TYPED_READS,
    TYPED_SOURCES,
    TYPED_WRAPPERS,
    UNSORTED_OUTPUT_DOCUMENTS,
    VALUE_GRIDS,
    VALUE_WRAPPERS,
    VARIANT_CAST_GRIDS,
    VARIANT_FUNCTIONS,
    VARIANT_OUTPUT,
    VARIANT_TYPED,
    VARIANT_TYPED_VALUES,
    ZONED_VARIANT_TEXTS,
)
from scripts.case_generators.models import Case, CaseFile, Column
from scripts.case_generators.types import Row


def variant_column() -> tuple[Column, ...]:
    """The one VARIANT input column `v`."""
    return (Column("v", "VARIANT"),)


def parsed(document: str) -> str:
    """A cell holding `document` parsed as a VARIANT."""
    return f"PARSE_JSON({text_literal(document)})"


def document_rows(documents: tuple[str, ...]) -> tuple[Row, ...]:
    """Each document, JSON null and SQL NULL."""
    return (*((parsed(document),) for document in documents), (parsed("null"),), (NULL_CELL,))


def variant_query(expression: str) -> str:
    """`expression` as JSON text and as a VARIANT result column, in input order."""
    return f"SELECT TO_JSON({expression}) AS j, {expression} AS v FROM input ORDER BY ordinal"


def delete_cases() -> list[Case]:
    """OBJECT_DELETE of each key over objects holding it or not, and non-objects alone."""
    cases: list[Case] = [
        Case(
            id=f"object_delete_key_{slug}",
            description=f"OBJECT_DELETE of the key {text_literal(key)}: exact, never a path.",
            columns=variant_column(),
            rows=document_rows(OBJECT_DOCUMENTS),
            query=variant_query(f"OBJECT_DELETE(v, {text_literal(key)})"),
            order="ordered",
            tags=("null", "json-null", "missing-key", "nested"),
        )
        for slug, key in OBJECT_KEYS
    ]
    cases.extend(
        Case(
            id=f"object_delete_not_object_{slug}",
            description=f"OBJECT_DELETE of the {slug} {document}: raises or converts.",
            columns=variant_column(),
            rows=((parsed(document),),),
            query=variant_query("OBJECT_DELETE(v, 'a')"),
            order="ordered",
            tags=("invalid",),
        )
        for slug, document in NOT_OBJECT_DOCUMENTS
    )
    cases.extend(
        (
            Case(
                id="object_delete_repeated_keys",
                description="OBJECT_DELETE of a key named twice and a missing key.",
                columns=variant_column(),
                rows=document_rows(OBJECT_DOCUMENTS),
                query=variant_query("OBJECT_DELETE(v, 'a', 'a', 'missing', 'A')"),
                order="ordered",
                tags=("duplicates", "missing-key"),
            ),
            Case(
                id="object_delete_null_key",
                description="OBJECT_DELETE with a SQL NULL key: raises or converts.",
                columns=variant_column(),
                rows=document_rows(OBJECT_DOCUMENTS),
                query=variant_query("OBJECT_DELETE(v, NULL)"),
                order="ordered",
                tags=("null",),
            ),
            Case(
                id="object_delete_column_key",
                description="OBJECT_DELETE with keys from a column, NULL included.",
                columns=(Column("v", "VARIANT"), Column("k", "VARCHAR")),
                rows=tuple(
                    (parsed(OBJECT_DOCUMENTS[0]), key)
                    for key in ("'a'", "'a.b'", "'missing'", NULL_CELL)
                ),
                query=variant_query("OBJECT_DELETE(v, k)"),
                order="ordered",
                tags=("null", "missing-key"),
            ),
            Case(
                id="object_delete_nested",
                description="OBJECT_DELETE of a nested object reached by a path.",
                columns=variant_column(),
                rows=document_rows(OBJECT_DOCUMENTS),
                query=variant_query("OBJECT_DELETE(v:o, 'k')"),
                order="ordered",
                tags=("nested", "missing-key"),
            ),
        )
    )
    return cases


def insert_cases() -> list[Case]:
    """OBJECT_INSERT of each key: new, updated, and existing without update alone."""
    cases: list[Case] = []
    for slug, key in INSERT_KEYS:
        quoted: str = text_literal(key)
        cases.append(
            Case(
                id=f"object_insert_key_{slug}",
                description=f"OBJECT_INSERT of the new key {quoted}, kept in byte order.",
                columns=variant_column(),
                rows=document_rows(INSERT_TARGETS),
                query=variant_query(f"OBJECT_INSERT(v, {quoted}, 'new')"),
                order="ordered",
                tags=("null", "json-null", "empty", "missing-key"),
            )
        )
        cases.append(
            Case(
                id=f"object_insert_update_{slug}",
                description=f"OBJECT_INSERT updating the existing key {quoted}.",
                columns=variant_column(),
                rows=((parsed(OBJECT_DOCUMENTS[0]),),),
                query=variant_query(f"OBJECT_INSERT(v, {quoted}, 'new', TRUE)"),
                order="ordered",
                tags=("duplicates",),
            )
        )
    cases.append(
        Case(
            id="object_insert_existing_dotted",
            description="OBJECT_INSERT of the existing key 'a.b' without update: raises.",
            columns=variant_column(),
            rows=((parsed(OBJECT_DOCUMENTS[0]),),),
            query=variant_query("OBJECT_INSERT(v, 'a.b', 'new')"),
            order="ordered",
            tags=("invalid",),
        )
    )
    cases.extend(
        Case(
            id=f"object_insert_not_object_{slug}",
            description=f"OBJECT_INSERT into the {slug} {document}: raises or converts.",
            columns=variant_column(),
            rows=((parsed(document),),),
            query=variant_query("OBJECT_INSERT(v, 'k', 1)"),
            order="ordered",
            tags=("invalid",),
        )
        for slug, document in (*NOT_OBJECT_DOCUMENTS, ("json_null", "null"))
    )
    return cases


def value_cases() -> list[Case]:
    """Each value type in each object constructor: its JSON text, TYPEOF and the VARIANT."""
    cases: list[Case] = []
    for type_slug, data_type, values in VALUE_GRIDS:
        for wrapper_slug, wrapper in VALUE_WRAPPERS:
            cases.append(
                Case(
                    id=f"object_{wrapper_slug}_{type_slug}_values",
                    description=f"{wrapper} of {data_type} values: text, TYPEOF and VARIANT.",
                    columns=(Column("x", data_type),),
                    rows=tuple((value,) for value in values),
                    query=(
                        f"SELECT TO_JSON({wrapper}) AS j, TYPEOF({wrapper}:k) AS t, "
                        f"{wrapper} AS v FROM input ORDER BY ordinal"
                    ),
                    order="ordered",
                    tags=("null", "boundary"),
                )
            )
    return cases


def typed_cases() -> list[Case]:
    """Typed values put in a VARIANT, then read by TYPEOF, casts, text and comparison."""
    cases: list[Case] = []
    for slug, data_type, is_type, values in TYPED_SOURCES:
        rows: tuple[Row, ...] = (*((value,) for value in values), (NULL_CELL,))
        for wrapper_slug, wrapper in TYPED_WRAPPERS:
            for read_slug, read in TYPED_READS:
                expression: str = read.format(e=wrapper, is_type=is_type, type=data_type)
                cases.append(
                    Case(
                        id=f"typed_{slug}_{wrapper_slug}_{read_slug}",
                        description=f"{expression} of {data_type} values.",
                        columns=(Column("x", data_type),),
                        rows=rows,
                        query=f"SELECT {expression} AS r FROM input ORDER BY ordinal",
                        order="ordered",
                        tags=("null", "boundary"),
                        expected_divergence=typed_divergence(
                            source=slug, read=read_slug, wrapper=wrapper_slug
                        ),
                    )
                )
            cases.append(
                Case(
                    id=f"typed_{slug}_{wrapper_slug}_output",
                    description=f"{wrapper} of {data_type} values as a VARIANT result column.",
                    columns=(Column("x", data_type),),
                    rows=rows,
                    query=f"SELECT {wrapper} AS v FROM input ORDER BY ordinal",
                    order="ordered",
                    tags=("null", "boundary"),
                    expected_divergence=typed_divergence(source=slug, read="output"),
                )
            )
        for read_slug, read in TYPED_READS[:-1]:
            expression = read.format(e="v", is_type=is_type, type=data_type)
            cases.append(
                Case(
                    id=f"typed_{slug}_round_trip_{read_slug}",
                    description=f"{expression} of a VARIANT column written from {data_type}.",
                    columns=variant_column(),
                    rows=tuple((f"TO_VARIANT(CAST({value} AS {data_type}))",) for value in values),
                    query=f"SELECT {expression} AS r FROM input ORDER BY ordinal",
                    order="ordered",
                    tags=("boundary",),
                    expected_divergence=typed_divergence(source=slug, read=read_slug),
                )
            )
    return cases


def typed_divergence(*, source: str, read: str, wrapper: str | None = None) -> str | None:
    """The divergence a typed read raises: FLOAT text at the root, typed reads, or values."""
    if (source, read) in FLOAT_TEXT_TYPED_READS and wrapper not in ELEMENT_TYPED_WRAPPERS:
        return FLOAT_TEXT
    if source in RAISING_TYPED_SOURCES or (
        source in CONVERTED_TYPED_SOURCES and read in RAISING_TYPED_READS
    ):
        return VARIANT_TYPED_VALUES
    return None


def output_cases() -> list[Case]:
    """VARIANT result columns: documents together, objects with unsorted keys alone."""
    cases: list[Case] = [
        Case(
            id="variant_output_documents",
            description="PARSE_JSON results as a VARIANT result column, and paths into them.",
            columns=variant_column(),
            rows=document_rows(OUTPUT_DOCUMENTS),
            query="SELECT v, v:a AS a, v:c AS c, v[0] AS first FROM input ORDER BY ordinal",
            order="ordered",
            tags=("null", "json-null", "nested", "empty", "missing-key"),
        )
    ]
    cases.extend(
        Case(
            id=f"variant_output_unsorted_{index + 1}",
            description=f"The object {document}, whose keys Snowflake sorts, as a result.",
            columns=variant_column(),
            rows=((parsed(document),),),
            query="SELECT v FROM input",
            order="ordered",
            tags=("boundary",),
        )
        for index, document in enumerate(UNSORTED_OUTPUT_DOCUMENTS)
    )
    return cases


def object_files() -> list[CaseFile]:
    """Object functions, typed values in VARIANTs and VARIANT result columns."""
    return [
        CaseFile(
            generator=OBJECT_FUNCTIONS,
            name="object_delete",
            category="semi-structured-object",
            construct="fn:OBJECT_DELETE",
            cases=tuple(delete_cases()),
            header=("OBJECT_DELETE over keys a path would misread, and non-objects.",),
        ),
        CaseFile(
            generator=OBJECT_FUNCTIONS,
            name="object_insert",
            category="semi-structured-object",
            construct="fn:OBJECT_INSERT",
            cases=tuple(insert_cases()),
            header=("OBJECT_INSERT of new and existing keys, and non-objects.",),
        ),
        CaseFile(
            generator=OBJECT_FUNCTIONS,
            name="object_values",
            category="semi-structured-typed",
            construct="fn:OBJECT_CONSTRUCT",
            cases=tuple(value_cases()),
            header=("Values of each SQL type in OBJECT_CONSTRUCT and OBJECT_INSERT.",),
        ),
        CaseFile(
            generator=VARIANT_TYPED,
            name="typed_values",
            category="semi-structured-typed",
            construct="cast:VARIANT",
            cases=tuple(typed_cases()),
            header=("Dates, timestamps, times, binary and numbers inside a VARIANT, read back.",),
        ),
        CaseFile(
            generator=VARIANT_OUTPUT,
            name="variant_output",
            category="semi-structured-output",
            construct="node:select",
            cases=tuple(output_cases()),
            header=("VARIANT result columns, compared as Snowflake's JSON text.",),
        ),
    ]


def single_rows(values: tuple[str, ...]) -> tuple[Row, ...]:
    """One row per value."""
    return tuple((value,) for value in values)


def check_cases() -> list[Case]:
    """IS_ kind and typed-value checks of documents, paths into them and NULLs."""
    return [
        Case(
            id=f"{function.lower()}_documents",
            description=f"{function} of documents, paths into them, JSON null and SQL NULL.",
            columns=variant_column(),
            rows=document_rows(OUTPUT_DOCUMENTS),
            query=(
                f"SELECT {function}(v) AS whole, {function}(v:a) AS a, {function}(v:c) AS c, "
                f"{function}(v[0]) AS first FROM input ORDER BY ordinal"
            ),
            order="ordered",
            tags=("null", "json-null", "missing-key"),
        )
        for function in KIND_CHECKS + TYPED_CHECKS
    ]


def keys_cases() -> list[Case]:
    """OBJECT_KEYS of objects together, of each non-object alone."""
    cases: list[Case] = [
        Case(
            id="object_keys_objects",
            description="OBJECT_KEYS of objects, nested objects, JSON null and SQL NULL.",
            columns=variant_column(),
            rows=document_rows(OBJECT_DOCUMENTS),
            query=variant_query("OBJECT_KEYS(v)").replace(
                "FROM input", ", OBJECT_KEYS(v:o) AS nested FROM input"
            ),
            order="ordered",
            tags=("null", "json-null", "missing-key"),
        )
    ]
    cases.extend(
        Case(
            id=f"object_keys_not_object_{slug}",
            description=f"OBJECT_KEYS of the {slug} {document}: raises or converts.",
            columns=variant_column(),
            rows=((parsed(document),),),
            query=variant_query("OBJECT_KEYS(v)"),
            order="ordered",
            tags=("invalid",),
        )
        for slug, document in NOT_OBJECT_DOCUMENTS
    )
    return cases


def pick_cases() -> list[Case]:
    """OBJECT_PICK of literal keys and arrays over objects; non-objects alone."""
    cases: list[Case] = [
        Case(
            id=f"object_pick_{slug}",
            description=f"OBJECT_PICK of {keys}: exact keys, never paths, byte order.",
            columns=variant_column(),
            rows=document_rows(OBJECT_DOCUMENTS),
            query=variant_query(f"OBJECT_PICK(v, {keys})"),
            order="ordered",
            tags=("null", "json-null", "missing-key"),
        )
        for slug, keys in PICK_KEY_LISTS
    ]
    cases.extend(
        Case(
            id=f"object_pick_not_object_{slug}",
            description=f"OBJECT_PICK of the {slug} {document}: raises or converts.",
            columns=variant_column(),
            rows=((parsed(document),),),
            query=variant_query("OBJECT_PICK(v, 'a')"),
            order="ordered",
            tags=("invalid",),
        )
        for slug, document in NOT_OBJECT_DOCUMENTS
    )
    cases.append(
        Case(
            id="object_pick_array_of_numbers",
            description="OBJECT_PICK of an array holding a number: raises or converts.",
            columns=variant_column(),
            rows=((parsed(OBJECT_DOCUMENTS[0]),),),
            query=variant_query("OBJECT_PICK(v, ARRAY_CONSTRUCT('a', 1))"),
            order="ordered",
            tags=("invalid",),
            expected_divergence=JSON_FIDELITY,
        )
    )
    return cases


def array_cases() -> list[Case]:
    """ARRAY_CONSTRUCT of each scalar type and VARIANTs; ARRAY_EXCEPT of string arrays."""
    cases: list[Case] = [
        Case(
            id=f"array_construct_{slug}",
            description=f"ARRAY_CONSTRUCT of {data_type} values, its size and TYPEOF.",
            columns=(Column("x", data_type),),
            rows=single_rows(values),
            query=(
                "SELECT TO_JSON(ARRAY_CONSTRUCT(x, x)) AS j, ARRAY_CONSTRUCT(x) AS v, "
                "TYPEOF(ARRAY_CONSTRUCT(x)[0]) AS t FROM input ORDER BY ordinal"
            ),
            order="ordered",
            tags=("boundary",),
        )
        for slug, data_type, values in CONSTRUCT_GRIDS
    ]
    cases.append(
        Case(
            id="array_construct_variants",
            description="ARRAY_CONSTRUCT of VARIANT documents, JSON null and paths.",
            columns=variant_column(),
            rows=tuple((parsed(document),) for document in OUTPUT_DOCUMENTS),
            query=variant_query("ARRAY_CONSTRUCT(v, v:a, PARSE_JSON('null'))"),
            order="ordered",
            tags=("json-null", "nested"),
            expected_divergence=JSON_FIDELITY,
        )
    )
    cases.append(
        Case(
            id="array_construct_null_element",
            description="ARRAY_CONSTRUCT of SQL NULL: raises or converts.",
            columns=(Column("x", "VARCHAR"),),
            rows=((NULL_CELL,),),
            query=variant_query("ARRAY_CONSTRUCT('a', x)"),
            order="ordered",
            tags=("null",),
            expected_divergence=JSON_FIDELITY,
        )
    )
    cases.extend(
        Case(
            id=f"array_except_{slug}",
            description=f"ARRAY_EXCEPT({left}, {right}).",
            columns=(Column("k", "NUMBER(1,0)"),),
            rows=(("1",),),
            query=variant_query(f"ARRAY_EXCEPT({left}, {right})"),
            order="ordered",
            tags=("duplicates", "empty"),
        )
        for slug, left, right in EXCEPT_PAIRS
    )
    cases.extend(
        Case(
            id=f"array_except_unsure_{slug}",
            description=f"ARRAY_EXCEPT({left}, {right}): raises or converts.",
            columns=(Column("k", "NUMBER(1,0)"),),
            rows=(("1",),),
            query=variant_query(f"ARRAY_EXCEPT({left}, {right})"),
            order="ordered",
            tags=("invalid",),
            expected_divergence=JSON_FIDELITY,
        )
        for slug, left, right in EXCEPT_UNSURE_PAIRS
    )
    cases.append(
        Case(
            id="array_except_null",
            description="ARRAY_EXCEPT with a SQL NULL array on either side.",
            columns=variant_column(),
            rows=((parsed('["a"]'),), (NULL_CELL,)),
            query=(
                "SELECT TO_JSON(ARRAY_EXCEPT(v, ARRAY_CONSTRUCT('a'))) AS l, "
                "TO_JSON(ARRAY_EXCEPT(ARRAY_CONSTRUCT('a', 'b'), v)) AS r FROM input "
                "ORDER BY ordinal"
            ),
            order="ordered",
            tags=("null",),
        )
    )
    return cases


def cast_cases() -> list[Case]:
    """Casts and TO_VARIANT of text, booleans and integers; VARIANT casts to OBJECT and ARRAY."""
    cases: list[Case] = []
    for slug, data_type, values in VARIANT_CAST_GRIDS:
        for spelling, expression in (("cast", "x::VARIANT"), ("to_variant", "TO_VARIANT(x)")):
            cases.append(
                Case(
                    id=f"{spelling}_{slug}_to_variant",
                    description=f"{expression} of {data_type}: TYPEOF, text and the VARIANT.",
                    columns=(Column("x", data_type),),
                    rows=(*((value,) for value in values), (NULL_CELL,)),
                    query=(
                        f"SELECT TYPEOF({expression}) AS t, TO_JSON({expression}) AS j, "
                        f"{expression}::VARCHAR AS s, {expression} AS v FROM input "
                        "ORDER BY ordinal"
                    ),
                    order="ordered",
                    tags=("null", "boundary"),
                )
            )
    for target in KIND_CAST_TARGETS:
        cases.append(
            Case(
                id=f"variant_as_{target.lower()}",
                description=f"VARIANT values of that kind, JSON null and NULL cast to {target}.",
                columns=variant_column(),
                rows=(
                    *((parsed(document),) for document in OUTPUT_DOCUMENTS[:3]),
                    (parsed("null"),),
                    (NULL_CELL,),
                ),
                query=(
                    f"SELECT TO_JSON(CASE WHEN IS_{target}(v) OR v IS NULL "
                    f"OR IS_NULL_VALUE(v) THEN v::{target} END) AS j FROM input ORDER BY ordinal"
                ),
                order="ordered",
                tags=("null", "json-null"),
            )
        )
        cases.extend(
            Case(
                id=f"variant_as_{target.lower()}_mismatch_{index + 1}",
                description=f"The VARIANT {document} cast to {target}: raises or converts.",
                columns=variant_column(),
                rows=((parsed(document),),),
                query=f"SELECT TO_JSON(v::{target}) AS j FROM input",
                order="ordered",
                tags=("invalid",),
            )
            for index, document in enumerate(KIND_MISMATCH_DOCUMENTS[target])
        )
    return cases


def variant_function_files() -> list[CaseFile]:
    """IS_ checks, OBJECT_KEYS, OBJECT_PICK, arrays and casts to and from VARIANT."""
    return [
        CaseFile(
            generator=VARIANT_FUNCTIONS,
            name="kind_checks",
            category="semi-structured-check",
            construct="fn:IS_OBJECT",
            cases=tuple(check_cases()),
            header=("IS_OBJECT, IS_ARRAY, IS_BOOLEAN and the typed-value IS_ checks of VARIANTs.",),
        ),
        CaseFile(
            generator=VARIANT_FUNCTIONS,
            name="object_keys_pick",
            category="semi-structured-object",
            construct="fn:OBJECT_PICK",
            cases=tuple(keys_cases() + pick_cases()),
            header=("OBJECT_KEYS and OBJECT_PICK over keys a path would misread.",),
        ),
        CaseFile(
            generator=VARIANT_FUNCTIONS,
            name="arrays",
            category="semi-structured-array",
            construct="fn:ARRAY_CONSTRUCT",
            cases=tuple(array_cases()),
            header=("ARRAY_CONSTRUCT of each value type, and ARRAY_EXCEPT of string arrays.",),
        ),
        CaseFile(
            generator=VARIANT_FUNCTIONS,
            name="variant_casts",
            category="semi-structured-cast",
            construct="node:cast",
            cases=(*cast_cases(), *probe_cases()),
            header=("Casts and TO_VARIANT to VARIANT; VARIANT casts to OBJECT and ARRAY.",),
        ),
    ]


def probe_cases() -> list[Case]:
    """Casts the census still refuses: zoned VARIANT text, FLOAT to text and to NUMBER(10,2)."""
    cases: list[Case] = [
        Case(
            id=f"variant_as_timestamp_tz_{index + 1}",
            description=f"The VARIANT {document} cast to TIMESTAMP_TZ: raises or converts.",
            columns=variant_column(),
            rows=((parsed(document),),),
            query=(
                "SELECT v::TIMESTAMP_TZ AS t, TO_VARCHAR(v::TIMESTAMP_TZ) AS s, "
                "v::TIMESTAMP_TZ::TIMESTAMP_NTZ AS n FROM input"
            ),
            order="ordered",
            tags=("invalid", "boundary"),
        )
        for index, document in enumerate(ZONED_VARIANT_TEXTS)
    ]
    cases.append(
        Case(
            id="variant_as_timestamp_tz_nulls",
            description="JSON null and SQL NULL cast to TIMESTAMP_TZ.",
            columns=variant_column(),
            rows=((parsed("null"),), (NULL_CELL,)),
            query="SELECT v::TIMESTAMP_TZ AS t FROM input ORDER BY ordinal",
            order="ordered",
            tags=("null", "json-null"),
        )
    )
    cases.extend(
        Case(
            id=f"float_as_varchar_{index + 1}",
            description=f"The FLOAT {value} cast to VARCHAR.",
            columns=(Column("k", "NUMBER(1,0)"),),
            rows=(("1",),),
            query=f"SELECT ({value})::FLOAT::VARCHAR AS s FROM input",
            order="ordered",
            tags=("boundary",),
        )
        for index, value in enumerate(FLOAT_TEXT_VALUES)
    )
    cases.append(
        Case(
            id="float_as_number_10_2",
            description="FLOAT values at rounding midpoints cast to NUMBER(10,2), and NULL.",
            columns=(Column("x", "FLOAT"),),
            rows=(*single_rows(FLOAT_NUMBER_VALUES), (NULL_CELL,)),
            query="SELECT x::NUMBER(10,2) AS n FROM input ORDER BY ordinal",
            order="ordered",
            tags=("null", "boundary"),
        )
    )
    cases.append(
        Case(
            id="float_as_number_10_2_overflow",
            description="A FLOAT beyond NUMBER(10,2) cast to it: raises or converts.",
            columns=(Column("x", "FLOAT"),),
            rows=(("1e9",),),
            query="SELECT x::NUMBER(10,2) AS n FROM input",
            order="ordered",
            tags=("invalid",),
        )
    )
    return cases
