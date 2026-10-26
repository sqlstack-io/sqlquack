"""Fuzz corpus seeding entry point."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from scripts.fuzz_corpus.constants import CORPUS_GLOB, CORPUS_SOURCES, ENTRIES_KEY, SQL_KEY


def run_seed(argv: list[str] | None = None) -> int:
    """Write each exact-check statement to the corpus directory, named by its SHA-1."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    parser.add_argument("--out", type=Path, required=True)
    arguments: argparse.Namespace = parser.parse_args(argv)
    arguments.out.mkdir(parents=True, exist_ok=True)
    written: int = 0
    for path in sorted((arguments.repository / CORPUS_SOURCES).glob(CORPUS_GLOB)):
        for entry in json.loads(path.read_text())[ENTRIES_KEY]:
            data: bytes = entry[SQL_KEY].encode()
            (arguments.out / hashlib.sha1(data).hexdigest()).write_bytes(data)
            written += 1
    print(f"fuzz corpus: done, {written} statements in {arguments.out}", file=sys.stderr)
    return 0
