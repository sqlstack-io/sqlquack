"""Engine-type entry point: the 18.6 server's result type for each expression of the matrix."""

from __future__ import annotations

import argparse

from scripts.postgres_catalog._helpers.matrix import expression_cases
from scripts.postgres_catalog._helpers.psql import image_digest, psql, psql_script
from scripts.postgres_catalog._helpers.recording import (
    recorded_cases,
    recording_script,
    write_fixture,
)
from scripts.postgres_catalog.constants import TYPE_FIELD


def run_type_recording(argv: list[str] | None = None) -> int:
    """Write each case's type, read through a temporary view, into the semantics test fixture."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", required=True)
    parser.add_argument(
        "--docker", default="docker", help="the docker command, such as 'sudo docker'"
    )
    options: argparse.Namespace = parser.parse_args(argv)
    docker: list[str] = options.docker.split()
    cases: list[tuple[str, str, str]] = expression_cases()
    script: str = recording_script(cases=cases)
    output: str = psql_script(container=options.container, docker=docker, script=script)
    records: list[dict[str, str]] = recorded_cases(cases=cases, output=output)
    version: str = psql(
        container=options.container, docker=docker, arguments=["-At"], sql="SHOW server_version"
    ).strip()
    image: str = image_digest(container=options.container, docker=docker)
    write_fixture(records=records, version=version, image=image)
    typed: int = sum(TYPE_FIELD in record for record in records)
    print(f"{len(records)} cases, {typed} typed, {len(records) - typed} engine errors")
    return 0
