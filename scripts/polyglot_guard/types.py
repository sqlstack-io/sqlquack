"""The parts of `cargo metadata` output the guard reads."""

from __future__ import annotations

from typing import TypedDict


class DependencyKind(TypedDict):
    """`null` for a normal dependency, otherwise `dev` or `build`."""

    kind: str | None


class NodeDependency(TypedDict):
    """One resolved dependency of a package."""

    pkg: str
    dep_kinds: list[DependencyKind]


class ResolveNode(TypedDict):
    """A package in the resolved graph."""

    id: str
    deps: list[NodeDependency]


class Resolve(TypedDict):
    """The resolved dependency graph."""

    nodes: list[ResolveNode]


class Package(TypedDict):
    """A package and its name."""

    id: str
    name: str


class Metadata(TypedDict):
    """`cargo metadata --format-version 1` output."""

    packages: list[Package]
    workspace_members: list[str]
    resolve: Resolve
