"""Engine-type entry point: the 18.6 server's result type for each matrix or syntax case."""

from __future__ import annotations

import argparse

from scripts.postgres_catalog._helpers.matrix import expression_cases, syntax_cases
from scripts.postgres_catalog._helpers.psql import image_digest, psql, psql_script
from scripts.postgres_catalog._helpers.recording import (
    recorded_cases,
    recording_script,
    write_fixture,
)
from scripts.postgres_catalog.constants import (
    MATRIX_METHOD,
    SYNTAX_CASES,
    SYNTAX_METHOD,
    SYNTAX_TYPES_FIXTURE,
    TYPE_FIELD,
    TYPES_FIXTURE,
)


def run_type_recording(argv: list[str] | None = None) -> int:
    """Write each case's type, read through a temporary view, into a semantics test fixture."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", required=True)
    parser.add_argument(
        "--docker", default="docker", help="the docker command, such as 'sudo docker'"
    )
    parser.add_argument(
        "--syntax-cases",
        action="store_true",
        help="record the accepted query-shaped syntax cases instead of the expression matrix",
    )
    options: argparse.Namespace = parser.parse_args(argv)
    docker: list[str] = options.docker.split()
    cases: list[tuple[str, str, str]] = (
        syntax_cases(SYNTAX_CASES) if options.syntax_cases else expression_cases()
    )
    script: str = recording_script(cases=cases)
    output: str = psql_script(container=options.container, docker=docker, script=script)
    records: list[dict[str, str]] = recorded_cases(cases=cases, output=output)
    version: str = psql(
        container=options.container, docker=docker, arguments=["-At"], sql="SHOW server_version"
    ).strip()
    image: str = image_digest(container=options.container, docker=docker)
    write_fixture(
        records=records,
        version=version,
        image=image,
        path=SYNTAX_TYPES_FIXTURE if options.syntax_cases else TYPES_FIXTURE,
        method=SYNTAX_METHOD if options.syntax_cases else MATRIX_METHOD,
    )
    typed: int = sum(TYPE_FIELD in record for record in records)
    print(f"{len(records)} cases, {typed} typed, {len(records) - typed} engine errors")
    return 0
