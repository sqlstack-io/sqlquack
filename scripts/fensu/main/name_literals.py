"""Entry point: name compared string literals in domain constants (fensu FPRSH007)."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.fensu._helpers.rewrites.domain_constants import named_in_file, write_constants
from scripts.fensu._helpers.rust.files import rust_files
from scripts.fensu.constants import CONSTANTS_FILE, DOMAIN_FILE_DEPTH, TEST_PATH_PARTS, TESTS_FILE


def run_name_literals(argv: list[str] | None = None) -> int:
    """Replace each compared literal in a crate's domains with a constant of its domain."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("src", type=Path)
    arguments: argparse.Namespace = parser.parse_args(argv)
    print("name-literals: rewriting Rust sources", file=sys.stderr)
    named: dict[str, dict[str, str]] = {}
    for path in rust_files(roots=[arguments.src]):
        relative: Path = path.relative_to(arguments.src)
        if _skipped(relative):
            continue
        domain: str = relative.parts[0]
        named[domain] = named_in_file(path=path, domain=domain, values=named.get(domain, {}))
    for domain, values in sorted(named.items()):
        if not values:
            continue
        write_constants(directory=arguments.src / domain, values=values)
    count: int = sum(len(values) for values in named.values())
    print(f"name-literals: done, {count} constants named", file=sys.stderr)
    return 0


def _skipped(relative: Path) -> bool:
    return (
        len(relative.parts) < DOMAIN_FILE_DEPTH
        or relative.name in {TESTS_FILE, CONSTANTS_FILE}
        or bool(TEST_PATH_PARTS.intersection(relative.parts[:-1]))
    )
