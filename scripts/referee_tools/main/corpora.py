"""Referee corpus import entry point: compiled SQLBuild projects and sqlglot's Snowflake tests."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.referee_tools._helpers.corpus_files import (
    write_project_corpus,
    write_sqlglot_corpus,
)
from scripts.referee_tools.constants import SQLGLOT_CORPUS
from scripts.referee_tools.models import ProjectImport


def run_corpora(argv: list[str] | None = None) -> int:
    """Write referee/corpora/ files and their environments from a project or sqlglot's tests."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    commands: argparse._SubParsersAction[argparse.ArgumentParser] = parser.add_subparsers(
        dest="command", required=True
    )
    project: argparse.ArgumentParser = commands.add_parser(
        "project", help="a compiled SQLBuild project"
    )
    project.add_argument("--name", required=True, help="the corpus and environment name")
    project.add_argument("--compiled", type=Path, required=True, help="target/compiled")
    project.add_argument("--dag", type=Path, required=True, help="`sqb dag --json` output")
    project.add_argument("--origin", required=True, help="where the project came from")
    project.add_argument("--split-tag", help="write models with this tag to a second file")
    project.add_argument("--every", type=int, default=1, help="keep every Nth model by name")
    sqlglot: argparse.ArgumentParser = commands.add_parser(
        SQLGLOT_CORPUS, help="sqlglot's tests/dialects/test_snowflake.py"
    )
    sqlglot.add_argument("--tests", type=Path, required=True)
    sqlglot.add_argument("--revision", required=True, help="the sqlglot commit the file is from")
    arguments: argparse.Namespace = parser.parse_args(argv)
    print(f"referee corpora: importing {arguments.command}…", file=sys.stderr)
    summary: str = (
        write_sqlglot_corpus(
            repository=arguments.repository, tests=arguments.tests, revision=arguments.revision
        )
        if arguments.command == SQLGLOT_CORPUS
        else write_project_corpus(
            repository=arguments.repository,
            project=ProjectImport(
                name=arguments.name,
                compiled=arguments.compiled,
                dag=arguments.dag,
                origin=arguments.origin,
                split_tag=arguments.split_tag,
                every=arguments.every,
            ),
        )
    )
    print(f"referee corpora: done, {summary}", file=sys.stderr)
    return 0
