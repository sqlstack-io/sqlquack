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
  the existing SQLQuack crates (parse, Snowflake checks, name checks, conversion under the built-in
  policy), kept out of the main Cargo workspace as `fuzz/` is. `scripts/build_engine.sh` builds it
  into `public/playground/engine/`, which is not committed. Translated SQL runs in DuckDB-WASM,
  loaded from jsDelivr on first use.

The logo uses the SQLQuack icon from `sqlstack-io/sqlstack` (`assets/icons/sqlquack.png`); the
footer's SQLStack symbols are unchanged copies from the same repository.

The Website workflow builds the site and checks its links when run by hand. It does not deploy,
and nothing publishes the site yet.

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
npm test         # Playwright: branding and navigation
```
