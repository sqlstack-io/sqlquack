"""A one-form coverage root, checker results and tree dumps for the re-bound review check."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

from scripts.rebound_reviews.constants import REBOUND_NOTE

_FORM: str = "sf.functions.count"
_CASE: str = "sf.functions.count.k1"
_SQL_SHA256: str = "a" * 64


def call(quantifier: str) -> str:
    """A one-statement tree whose call carries `quantifier`, a field such as `"distinct":true`."""
    function: str = f'{{"name":"COUNT",{quantifier},"args":[{{"column":"c1"}}]}}'
    return f'[{{"select":{{"expressions":[{{"function":{function}}}]}}}}]'


def fingerprint(tree: str) -> str:
    """The fingerprint `syntax-coverage` gives the form whose one accepted case parsed to `tree`."""
    tree_hash: str = hashlib.sha256(tree.encode()).hexdigest()
    line: str = f'{_CASE}\t{_SQL_SHA256}\taccepted\t{tree_hash}\t"accept"\tnull\t"accepted"\n'
    return hashlib.sha256(line.encode()).hexdigest()


def write_inputs(
    root: Path, *, sql: str, old_tree: str, new_tree: str, reviewed_tree: str, note_field: str
) -> list[str]:
    """The check's arguments over a form reviewed at `reviewed_tree` and re-bound to `new_tree`."""
    (root / "reviews").mkdir()
    (root / "cases").mkdir()
    (root / "cases" / "functions.jsonl").write_text(
        json.dumps({"case_id": _CASE, "sql": sql, "expect": "accept"}) + "\n"
    )
    (root / "reviews" / "functions.toml").write_text(
        f'[[review]]\nid = "r-001"\nverdict = "approved"\nforms = ["{_FORM}"]\n'
        f'[review.fingerprints]\n"{_FORM}" = "{fingerprint(reviewed_tree)}"\n\n'
        f'[[review]]\nid = "r-002"\nverdict = "approved"\nforms = ["{_FORM}"]\n'
        f'{note_field} = "{REBOUND_NOTE}"\n'
        f'[review.fingerprints]\n"{_FORM}" = "{fingerprint(new_tree)}"\n'
    )
    case: dict[str, object] = {
        "case_id": _CASE,
        "sql_sha256": _SQL_SHA256,
        "phase": "accepted",
        "local": {
            "verdict": "accepted",
            "opaque": False,
            "tree_sha256": hashlib.sha256(new_tree.encode()).hexdigest(),
        },
    }
    (root / "family.json").write_text(
        json.dumps([{"id": _FORM, "obligations": [{"cases": [case]}]}])
    )
    (root / "old.tsv").write_text(f"{_CASE}\t{old_tree}\n")
    (root / "new.tsv").write_text(f"{_CASE}\t{new_tree}\n")
    return [
        "--reviews", str(root / "reviews"), "--cases", str(root / "cases"),
        "--family", str(root / "family.json"),
        "--old", str(root / "old.tsv"), "--new", str(root / "new.tsv"),
    ]  # fmt: skip
