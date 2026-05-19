# SQLQuack

SQLQuack is an early-stage Rust SQL semantic engine for warehouse SQL analysis and fail-closed DuckDB transpilation.

The project is inspired by SQLGlot's feature breadth, but it is not intended to be an `N x N` best-effort transpiler. SQLQuack focuses on structured analysis first and only emits DuckDB SQL when supported semantics are expected to be preserved.

Status: early development. APIs are unstable.
