"""Write a one-crate workspace with the repository's fensu policy rules, and run fensu over it."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

REPOSITORY: Path = Path(__file__).resolve().parents[4]
WORKSPACE_MANIFEST: str = '[workspace]\nmembers = ["crates/sqlquack-semantics"]\nresolver = "2"\n'
CRATE_MANIFEST: str = (
    '[package]\nname = "sqlquack-semantics"\nversion = "0.1.0"\nedition = "2021"\n\n'
    "[features]\ndefault = []\nsnowflake = []\n"
)
LIB: str = '#[cfg(feature = "snowflake")]\nmod snowflake;\nmod schemas;\n'


def write_semantics_crate(root: Path, config: str) -> None:
    """Write the semantics crate, fensu.toml from `config` and the policy rules under `root`."""
    crate: Path = root / "crates" / "sqlquack-semantics"
    for folder in ("snowflake", "schemas"):
        (crate / "src" / folder).mkdir(parents=True)
        (crate / "src" / folder / "mod.rs").write_text("pub fn column() {}\n")
    (root / "Cargo.toml").write_text(WORKSPACE_MANIFEST)
    (crate / "Cargo.toml").write_text(CRATE_MANIFEST)
    (crate / "src" / "lib.rs").write_text(LIB)
    (root / "fensu.toml").write_text(config)
    shutil.copytree(
        REPOSITORY / "scripts" / "fensu_policy",
        root / "scripts" / "fensu_policy",
        ignore=shutil.ignore_patterns("__pycache__"),
    )
    (root / "scripts" / "__init__.py").write_text("")


def run_fensu_check(root: Path, path: str) -> subprocess.CompletedProcess[str]:
    """Run `fensu check` in `root` with `path` as PATH."""
    return subprocess.run(
        [str(Path(sys.executable).with_name("fensu")), "check", "--no-cache"],
        cwd=root,
        env={**os.environ, "PATH": path, "PYTHONPATH": str(root)},
        capture_output=True,
        text=True,
        check=False,
    )
