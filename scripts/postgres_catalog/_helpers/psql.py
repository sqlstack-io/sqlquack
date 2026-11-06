"""Commands run against the PostgreSQL container."""

from __future__ import annotations

import subprocess


def psql(*, container: str, docker: list[str], arguments: list[str], sql: str) -> str:
    """The output of one psql command run in the container."""
    command: list[str] = [
        *docker,
        "exec",
        "-i",
        container,
        "psql",
        "-U",
        "postgres",
        "-X",
        "-q",
        *arguments,
        "-c",
        sql,
    ]
    return subprocess.run(command, check=True, capture_output=True, text=True).stdout


def image_digest(*, container: str, docker: list[str]) -> str:
    """The sha256 of the image the container runs."""
    command: list[str] = [*docker, "inspect", "--format", "{{.Image}}", container]
    image: str = subprocess.run(command, check=True, capture_output=True, text=True).stdout
    return image.strip().removeprefix("sha256:")


def psql_script(*, container: str, docker: list[str], script: str) -> str:
    """The output of a psql script read from standard input, with errors interleaved in order."""
    shell: str = "psql -U postgres -X -q -At 2>&1"
    command: list[str] = [*docker, "exec", "-i", container, "sh", "-c", shell]
    return subprocess.run(command, input=script, check=True, capture_output=True, text=True).stdout
