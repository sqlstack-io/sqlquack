<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/sqlstack-io/sqlquack/main/.github/sqlquack-logo-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/sqlstack-io/sqlquack/main/.github/sqlquack-logo-light.png">
    <img src="https://raw.githubusercontent.com/sqlstack-io/sqlquack/main/.github/sqlquack-logo-light.png" alt="SQLQuack" width="420">
  </picture>
  <br>
  <sub>by <a href="https://sqlstack.io">SQLStack</a></sub>
</p>

### Run warehouse SQL locally.

Translate warehouse SQL into DuckDB SQL for local testing and CI - without a running warehouse.

[Website](https://sqlquack.io/) · [Docs](https://sqlquack.io/docs/) · [Playground](https://sqlquack.io/playground/) · [Quickstart](https://sqlquack.io/docs/quickstart/)

![SQLQuack checks SQL and converts queries to DuckDB SQL](https://raw.githubusercontent.com/sqlstack-io/sqlquack/main/.github/demos/check-convert.gif)

## Why SQLQuack?

SQL that runs on two engines doesn't always return the same answer. NULLs, decimal arithmetic and casts can behave differently.

SQLQuack checks its conversion rules against real results from the source warehouse. It handles those differences deliberately and gives you precise diagnostics when it can't convert a query - not plausible-looking SQL and crossed fingers.

- **Check early.** Catch invalid SQL before it reaches the warehouse.
- **Test locally.** Convert queries to DuckDB SQL for tests against small fixtures.
- **See what's blocking you.** Scan a SQL project to find what converts and what needs attention.
- **Choose what to accept.** Control known differences through an explicit policy.

## Get started

```bash
pip install sqlquack

# Convert your first query
echo "SELECT 1 AS total" | sqlquack convert --builtin-policy

# Check a SQL file
sqlquack check query.sql

# Convert a query using your table definitions
sqlquack convert query.sql --schema schema.json
```

Or **[try the playground](https://sqlquack.io/playground/)** - no installation needed.

See the [supported dialects and coverage](https://sqlquack.io/docs/coverage/), [schema guide](https://sqlquack.io/docs/schema/) and [conversion policy](https://sqlquack.io/docs/policy/) for the details.

## Use it from Python

```python
import sqlquack

result = sqlquack.convert(
    sql="SELECT 1 AS total",
    options=sqlquack.ConvertOptions(builtin_policy=True),
)
```

[Python API](https://sqlquack.io/docs/python/)

---

Built in Rust. Free and open source. [Apache 2.0](https://github.com/sqlstack-io/sqlquack/blob/main/LICENSE).

Part of [SQLStack](https://sqlstack.io/).
