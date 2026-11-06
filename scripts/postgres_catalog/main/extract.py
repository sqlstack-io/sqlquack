"""Catalogue extract entry point: pg_proc, pg_cast, pg_type and pg_operator as CSV, plus a manifest."""

from __future__ import annotations

import argparse

from scripts.postgres_catalog._helpers.psql import image_digest, psql
from scripts.postgres_catalog.constants import DATA_DIR, QUERIES


def run_catalog_extract(argv: list[str] | None = None) -> int:
    """Write the catalogue extract read from a container of the pinned image."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", required=True)
    parser.add_argument(
        "--docker", default="docker", help="the docker command, such as 'sudo docker'"
    )
    options: argparse.Namespace = parser.parse_args(argv)
    docker: list[str] = options.docker.split()
    for name, sql in QUERIES.items():
        csv: str = psql(container=options.container, docker=docker, arguments=["--csv"], sql=sql)
        (DATA_DIR / f"{name}.csv").write_text(csv)
    version: str = psql(
        container=options.container, docker=docker, arguments=["-At"], sql="SHOW server_version"
    ).strip()
    image: str = image_digest(container=options.container, docker=docker)
    (DATA_DIR / "manifest.toml").write_text(
        'engine = "postgres"\n'
        f'engine_version = "{version}"\n'
        f'engine_image_sha256 = "{image}"\n'
        'method = "pg_catalog read with format_type; written by scripts/extract_postgres_catalog.py"\n'
    )
    return 0
