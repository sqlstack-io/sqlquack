# CLI recordings

Terminal recordings of `sqlquack` for the README, rendered with
[VHS](https://github.com/charmbracelet/vhs) from the `.tape` scripts in `tapes/`. The tapes and
scripts are the source; the rendered GIF and PNG files go to `out/`, which is not committed.

| Demo | Shows |
|------|-------|
| `check-convert` | `sqlquack check` reports a QUALIFY clause Snowflake rejects, `sqlquack convert` converts a query to DuckDB SQL, and refuses one that needs `HASH`. |

The README uses `check-convert`, copied into `.github/demos/`.

## Requirements

- `vhs`, `ttyd` and `ffmpeg` on `PATH`, plus a Chromium or Chrome that VHS can find.
- DejaVu Sans Mono, the font the tapes use.
- `sqlquack` built with `cargo build --release -p sqlquack`, or `SQLQUACK_BIN` pointing at a
  directory that contains the `sqlquack` to record. `record.sh` puts the DuckDB library the build
  downloaded on the library path.

## Record

```bash
cd website/recordings
./record.sh check-convert     # out/check-convert.gif and out/check-convert.png
cp out/check-convert.gif ../../.github/demos/
```

`record.sh` runs `prepare.sh <demo>` to write the demo's SQL and schema under `scratch/<demo>/`,
then renders `tapes/<demo>.tape`. Shared terminal settings and the colour theme are in
`tapes/common.tape`.

Look at every recording before publishing it.
