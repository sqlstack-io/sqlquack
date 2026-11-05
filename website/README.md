# Website

The SQLQuack website: the homepage at `/`, the documentation at `/docs/` and the playground at
`/playground/`, built as one [Astro Starlight](https://starlight.astro.build) site. It shares its
structure, styling and components with the SQLClean and SQLBuild websites.

- `public/index.html` and `public/styles.css`: the homepage, plain static HTML and CSS. Its terminal
  examples are real `sqlquack` output on a small orders example, trimmed where marked `…`.
- `src/content/docs/docs/`: the documentation pages (MDX). `src/sidebar.json` holds the sidebar
  and `src/navigation.mjs` the redirects. `STYLE.md` is the style guide for these pages.
- `src/content/docs/docs/diagnostics/reference.mdx` is generated from
  `crates/sqlquack-core/src/diagnostics/constants.rs` by `scripts/generate_diagnostics.py`;
  `--check` fails while the two differ.
- `src/components/`: the page layout, header and page title, replacing Starlight's defaults.
- `public/fonts/`: self-hosted fonts, each with its licence.
- `public/playground/`: the playground page. `wasm/` is its engine, a thin wasm-bindgen crate over
  the existing SQLQuack crates (parse, Snowflake checks, name checks, conversion under a policy and
  session), kept out of the main Cargo workspace as `fuzz/` is. `scripts/build_engine.sh` builds it
  into `public/playground/engine/`, which is not committed. The engine runs in a Web Worker.
  Translated SQL runs in DuckDB-WASM, served from the site: `scripts/prepare_playground.mjs` copies
  it from `node_modules` into `public/playground/duckdb/` on every `npm run build` and `npm run dev`.

DuckDB-WASM is pinned to `1.33.1-dev114.0`, a prerelease from npm's `next` tag and the first build
that bundles DuckDB 2.0. Swap it for a stable 2.x release once one ships: change the version in
`package.json`, run `npm install`, and check that stage 4 of the playground reports the expected
DuckDB version.

The logo uses the SQLQuack icon from `sqlstack-io/sqlstack` (`assets/icons/sqlquack.png`); the
footer's SQLStack symbols are unchanged copies from the same repository.

The Website workflow builds and tests the engine, builds the site and checks its links, on pull
requests that touch them and when run by hand. It never runs on push to main and does not deploy;
nothing publishes the site yet.

Requires Node 22.12 or newer.

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked wasm-bindgen-cli --version =0.2.130
scripts/build_engine.sh   # the playground engine; prints its size
npm install
python3 scripts/generate_diagnostics.py   # after changing the diagnostic registry
npm run dev      # live preview
npm run build    # static site in dist/, including the search index
npm run preview  # serve dist/
python3 scripts/check_links.py dist
npm run test:engine   # the engine and its spans, after build_engine.sh
```
