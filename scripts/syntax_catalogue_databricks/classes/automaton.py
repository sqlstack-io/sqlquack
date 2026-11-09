"""A nondeterministic automaton over the words and punctuation of catalogued grammar lines."""

from __future__ import annotations

from scripts.syntax_catalogue_databricks.constants import ANY_RUN


class Automaton:
    """A nondeterministic automaton: labelled and empty edges between numbered states."""

    def __init__(self) -> None:
        self.edges: list[list[tuple[str, int]]] = []
        self.empty: list[list[int]] = []

    def state(self) -> int:
        self.edges.append([])
        self.empty.append([])
        return len(self.edges) - 1

    def closure(self, states: set[int]) -> set[int]:
        found: set[int] = set(states)
        frontier: list[int] = list(states)
        while frontier:
            for target in self.empty[frontier.pop()]:
                if target not in found:
                    found.add(target)
                    frontier.append(target)
        return found

    def reach(self, states: set[int]) -> set[int]:
        """The states reachable along any edges: what a rule too large to spell may read."""
        found: set[int] = set(states)
        frontier: list[int] = list(states)
        while frontier:
            state: int = frontier.pop()
            for target in [*self.empty[state], *(target for _, target in self.edges[state])]:
                if target not in found:
                    found.add(target)
                    frontier.append(target)
        return found

    def targets(self, label: str) -> set[int]:
        """Every state an edge labelled `label` leads to, from any state."""
        if not hasattr(self, "_targets"):
            index: dict[str, set[int]] = {}
            for edges in self.edges:
                for found, target in edges:
                    index.setdefault(found, set()).add(target)
            self._targets: dict[str, set[int]] = index
        return self._targets.get(label, set())

    def step(self, *, states: set[int], symbol: str) -> set[int]:
        """Every state an edge labelled `symbol` leads to from one of `states`."""
        found: set[int] = set()
        for state in states:
            found |= {target for label, target in self.edges[state] if label == symbol}
        return found

    def reads(self, *, sequence: tuple[str, ...], start: int | None) -> bool:
        """True when the sequence is read without a gap from `start`, or from any state."""
        current: set[int] | None = None if start is None else self.closure({start})
        for symbol in sequence:
            if symbol == ANY_RUN:
                if current is not None:
                    current = self.reach(current)
                continue
            moved: set[int] = (
                self.targets(symbol)
                if current is None
                else self.step(states=current, symbol=symbol)
            )
            if not moved:
                return False
            current = self.closure(moved)
        return True
