"""A two-crate cargo workspace with path dependencies only, so cargo needs no network."""

from __future__ import annotations

import subprocess
from pathlib import Path

ORDERS_MANIFEST: str = '[package]\nname = "orders"\nversion = "0.1.0"\nedition = "2021"\n'
ORDERS_WITH_CUSTOMERS: str = (
    ORDERS_MANIFEST + '\n[dependencies]\ncustomers = { path = "../customers" }\n'
)
CUSTOMERS_MANIFEST: str = '[package]\nname = "customers"\nversion = "0.1.0"\nedition = "2021"\n'


def write_locked_workspace(*, directory: Path, manifest_after_lock: str) -> Path:
    """Lock a workspace of two independent crates, then give orders `manifest_after_lock`."""
    workspace: Path = directory / "workspace"
    for crate, manifest in (("orders", ORDERS_MANIFEST), ("customers", CUSTOMERS_MANIFEST)):
        (workspace / crate / "src").mkdir(parents=True)
        (workspace / crate / "Cargo.toml").write_text(manifest)
        (workspace / crate / "src" / "lib.rs").write_text("")
    (workspace / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["orders", "customers"]\nresolver = "2"\n'
    )
    subprocess.run(["cargo", "generate-lockfile", "--offline"], cwd=workspace, check=True)
    (workspace / "orders" / "Cargo.toml").write_text(manifest_after_lock)
    return workspace
