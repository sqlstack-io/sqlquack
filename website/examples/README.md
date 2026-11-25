# Website examples

Each folder holds the input files of one example the website shows: SQL files, `sources/*.yml`
and `sqlquack.toml`. `make website-examples` copies the folder to a scratch directory, runs the
commands a page shows there with the built `sqlquack` first on `PATH`, and fails where the page
shows anything other than the real output. It also fails on any homepage terminal or docs
`console` block that names no example.

A page names the example a block shows:

- On the homepage, on the `<pre>`: `<pre class="terminal" data-example="division">`.
- In a docs page, on the line before the block: `{/* example: quickstart */}`.

After the name, the marker says how to read the block:

| Marker | The block shows |
|--------|-----------------|
| `name` | A session: each `$ command` line, then what it prints (standard output and error). |
| `name preamble=FILE` | The same, after the contents of `FILE`. |
| `name file=FILE` | The contents of `FILE`. |
| `name output: COMMAND` | What `COMMAND` prints on standard output. |
| `name python` | Python code: each `expression  # value` line shows the value it gives. |

`no-inputs` is the name for an example that needs no files, such as `echo "…" | sqlquack check`.

The page may shorten what it shows:

- A line of only `…` stands for one or more lines.
- `…` within a line stands for any text on that line.
- A first line that comments the file's name (`-- div.sql, …` or `# sources/orders.yml`) labels the
  file and is not part of it.
- A command's leading progress line, such as `sqlquack convert: converting…`, may be left out.

`pip install sqlquack` is shown but not run.
