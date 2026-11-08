"""The automaton strict containment reads a parse.y keyword sequence with."""

from __future__ import annotations

from scripts.syntax_catalogue_sqlite.constants import ANY_RUN


class Automaton:
    """A nondeterministic automaton: labelled and empty edges between numbered states."""

    def __init__(self) -> None:
        self.edges: list[list[tuple[str, int]]] = []
        self.empty: list[list[int]] = []

    def state(self) -> int:
        """A new state."""
        self.edges.append([])
        self.empty.append([])
        return len(self.edges) - 1

    def closure(self, states: set[int]) -> set[int]:
        """The states reachable by empty edges."""
        found: set[int] = set(states)
        frontier: list[int] = list(states)
        while frontier:
            for target in self.empty[frontier.pop()]:
                if target not in found:
                    found.add(target)
                    frontier.append(target)
        return found

    def reach(self, states: set[int]) -> set[int]:
        """The states reachable along any edges: what a parse.y rule too large to spell may read."""
        found: set[int] = set(states)
        frontier: list[int] = list(states)
        while frontier:
            state: int = frontier.pop()
            for target in [*self.empty[state], *(target for _, target in self.edges[state])]:
                if target not in found:
                    found.add(target)
                    frontier.append(target)
        return found

    def reads(self, *, sequence: tuple[str, ...], start: int | None) -> bool:
        """True when the sequence is read without a gap from `start`, or from any state."""
        current: set[int] = self.closure(set(range(len(self.edges))) if start is None else {start})
        for symbol in sequence:
            if symbol == ANY_RUN:
                current = self.reach(current)
                continue
            moved: set[int] = set()
            for state in current:
                moved |= {target for label, target in self.edges[state] if label == symbol}
            if not moved:
                return False
            current = self.closure(moved)
        return True
