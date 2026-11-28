# Changelog

## [0.28.0](https://github.com/sqlstack-io/sqlbabel/compare/v0.27.0...v0.28.0) (2026-11-28)


### Features

* **databricks-syntax:** tree query tags and NOT NULL on Hive CHANGE and parameters ([38b908f](https://github.com/sqlstack-io/sqlbabel/commit/38b908fa67eab303852b84949dd4855acb87df57))
* **databricks:** convert window functions and QUALIFY ([596d8f5](https://github.com/sqlstack-io/sqlbabel/commit/596d8f5f89d860e093503bf48c1c51956fad2e98))

## [0.27.0](https://github.com/sqlstack-io/sqlquack/compare/v0.26.0...v0.27.0) (2026-11-28)


### Features

* **convert:** add a Databricks dialect option ([1d2de48](https://github.com/sqlstack-io/sqlquack/commit/1d2de48b7d11a1102bc6c8c71e09829a71d792a2))
* **coverage:** sign off Databricks forms with nothing in DuckDB to translate to ([6cc8b86](https://github.com/sqlstack-io/sqlquack/commit/6cc8b86faf8291c8a11dea1a314cad34030695c6))
* **databricks:** convert CONV and DATE_FORMAT with a constant pattern ([728da59](https://github.com/sqlstack-io/sqlquack/commit/728da593170c96041278d180d62a6c1b16a6c678))
* **databricks:** convert functions whose arguments Databricks casts or narrows ([f1618a2](https://github.com/sqlstack-io/sqlquack/commit/f1618a21d0e198e55e8b992be4ddd754cee99a58))
* **databricks:** convert mixed-type IN lists and TO_DATE, TRY_TO_DATE and TO_TIMESTAMP ([f9814ab](https://github.com/sqlstack-io/sqlquack/commit/f9814ab10923b84fa139f5df6f4280526f7a8a87))
* **databricks:** convert more string, timestamp and boolean forms ([269bc8c](https://github.com/sqlstack-io/sqlquack/commit/269bc8c0b3f5c2bb008aedda363aea328e99fdaa))
* **databricks:** convert PERCENTILE_CONT and PERCENTILE_DISC WITHIN GROUP ([4ad436c](https://github.com/sqlstack-io/sqlquack/commit/4ad436c683608f46346d18bdab55d9f99c27d0f5))
* **databricks:** convert the REGEXP functions for constant patterns RE2 reads as Java does ([5abb009](https://github.com/sqlstack-io/sqlquack/commit/5abb009152f057527e310fba8aba43c7b33ca7f5))
* **databricks:** read STRING arguments and casts as Databricks casts them ([d8c1d8b](https://github.com/sqlstack-io/sqlquack/commit/d8c1d8bf2c3c82ff1ee10e74539c2313d020e438))


### Bug Fixes

* **coverage:** keep two Databricks groups unsigned until their reasons are checked ([6264c67](https://github.com/sqlstack-io/sqlquack/commit/6264c678c68529807c71c04c59e33d7f890beaac))
* **coverage:** state what two not-translatable Databricks groups call, and sign them off ([f53fd28](https://github.com/sqlstack-io/sqlquack/commit/f53fd28a05a05002e1ccdc15234821342822ac53))
* **databricks:** match what the new Databricks recordings show ([03bdd0c](https://github.com/sqlstack-io/sqlquack/commit/03bdd0c7ae1b683737468eccca4827c8f5ad465b))
* **databricks:** refuse or raise where Databricks reads the arguments differently ([4f87534](https://github.com/sqlstack-io/sqlquack/commit/4f87534e75822d888b9cb90b21c0937a58c07b9d))

## [0.26.0](https://github.com/sqlstack-io/sqlquack/compare/v0.25.1...v0.26.0) (2026-11-28)


### Features

* **coverage:** measure Databricks translation against recorded results ([d2c43cb](https://github.com/sqlstack-io/sqlquack/commit/d2c43cb8ce25917bfd1fd99f7d5bbaa0d25f90b9))
* **databricks:** translate Databricks SQL to DuckDB with fail-closed rules ([20e410b](https://github.com/sqlstack-io/sqlquack/commit/20e410bf30d8ba09c415991ffdba1ff717070506))
* **evidence:** load and verify cases recorded on Databricks SQL ([0483c16](https://github.com/sqlstack-io/sqlquack/commit/0483c16fafbf4c15ed1a2ad74d13434afdf7f1cf))

## [0.25.1](https://github.com/sqlstack-io/sqlquack/compare/v0.25.0...v0.25.1) (2026-11-27)


### Bug Fixes

* **databricks:** keep FROM-first and TABLE marks through a PIVOT pipe ([d4a9a0d](https://github.com/sqlstack-io/sqlquack/commit/d4a9a0d1206b82934166a18eb3a4252c0fbcdb38))
* **databricks:** read deferral options only where constraints document them ([3143419](https://github.com/sqlstack-io/sqlquack/commit/31434195d3ad08e47d9c74921c7a7ff3a7dedfd4))

## [0.25.0](https://github.com/sqlstack-io/sqlquack/compare/v0.24.1...v0.25.0) (2026-11-27)


### Features

* **api:** allow and allow_location on ConvertOptions ([583440b](https://github.com/sqlstack-io/sqlquack/commit/583440bc17286c638cbbca8285b8c5ead4e0b0bb))


### Bug Fixes

* **convert:** TRY_PARSE_JSON of text without brackets no longer raises list_reduce's error ([a9bd593](https://github.com/sqlstack-io/sqlquack/commit/a9bd5937f3f2d02eb9460cf4e82a1189d1d8119e))
* **overrides:** errors loading override files name the files ([aa77f06](https://github.com/sqlstack-io/sqlquack/commit/aa77f06bb03b3cc5d5a11efaf7feb4d23eecbb27))

## [0.24.1](https://github.com/sqlstack-io/sqlquack/compare/v0.24.0...v0.24.1) (2026-11-25)


### Performance Improvements

* **convert:** bind a computed SPLIT operand once in DuckDB ([01b4e4e](https://github.com/sqlstack-io/sqlquack/commit/01b4e4e759068005597e74c5e7bc1186de5388c5))
* **convert:** keep computed arguments out of DuckDB's repeating macros ([43bc619](https://github.com/sqlstack-io/sqlquack/commit/43bc6196e4a684c488a7e99e3075fe5e9b94b50b))
* **convert:** look up UPPER's multi-character mappings in two strings ([be4d4fc](https://github.com/sqlstack-io/sqlquack/commit/be4d4fc690f8328bd55f8db2b4ba51239089472d))
* **convert:** read text as FLOAT with one literal for its whitespace, numbers first ([da5adcf](https://github.com/sqlstack-io/sqlquack/commit/da5adcf21211f152024274e85f0be26e37109e83))
* **convert:** write VARIANT text through version-2 macros that escape once ([7ce81e2](https://github.com/sqlstack-io/sqlquack/commit/7ce81e27510078f45ad5cf3f40b4728a185515d0))

## [0.24.0](https://github.com/sqlstack-io/sqlquack/compare/v0.23.2...v0.24.0) (2026-11-25)


### Features

* **convert:** let callers skip DuckDB's bind check of the converted SQL ([55d45ae](https://github.com/sqlstack-io/sqlquack/commit/55d45ae660e23ee9e136bd9952d2d486bb1784f3))


### Bug Fixes

* **convert:** bind a simple CASE's computed operand once in DuckDB ([b9fd94c](https://github.com/sqlstack-io/sqlquack/commit/b9fd94cac9094abf3a315fab04b089c21d458d97))
* **convert:** cast UNION ALL literals once only in starless chains, and meet the lint rules ([dbd7784](https://github.com/sqlstack-io/sqlquack/commit/dbd778428f848d5cfd758b4f70276b4e52cfb698))
* **convert:** cast UNION ALL literals once only where items are columns ([25a7c2f](https://github.com/sqlstack-io/sqlquack/commit/25a7c2f547990061a1a9bfd711908e42561994a2))
* **convert:** slice non-ASCII Snowflake source once per parse ([f0ea8e9](https://github.com/sqlstack-io/sqlquack/commit/f0ea8e96ec1fa54055894998a7ab581fffec6d4f))


### Performance Improvements

* **convert:** list a long FLOAT variance operand once for its count and sums ([0db107f](https://github.com/sqlstack-io/sqlquack/commit/0db107f3bdc55ee130969158b7d0a66824f09ce4))
* **convert:** match a date format's groups once, drop the default frame, bind a long held cast ([c1eea2e](https://github.com/sqlstack-io/sqlquack/commit/c1eea2ebdc227b9fda3986d7f6e30425436d114a))
* **convert:** read a constant PARSE_JSON literal with json() when the fast check would ([ed93b7b](https://github.com/sqlstack-io/sqlquack/commit/ed93b7b945eb27534d8af1846a125e5e63551b18))
* **convert:** write UNION ALL branches once: names, parentheses and integer casts ([735e0fe](https://github.com/sqlstack-io/sqlquack/commit/735e0fe16dcb651e2331f5e9cc804f819a5e9004))

## [0.23.2](https://github.com/sqlstack-io/sqlquack/compare/v0.23.1...v0.23.2) (2026-11-25)


### Bug Fixes

* **convert:** keep doubling to round a quotient at result scale 37 ([7addacf](https://github.com/sqlstack-io/sqlquack/commit/7addacf97c6fc3cd8a2267f95801305d59d7df81))
* **convert:** pin variant_written_v1 by its definition statement's hash ([94eb19a](https://github.com/sqlstack-io/sqlquack/commit/94eb19a65d391cb922891ad4fcad44d67f611b74))

## [0.23.1](https://github.com/sqlstack-io/sqlquack/compare/v0.23.0...v0.23.1) (2026-11-24)


### Bug Fixes

* **syntax:** name VALUES and UNPIVOT items by their alias in query scopes ([e78032e](https://github.com/sqlstack-io/sqlquack/commit/e78032e1078597139a8f1ad4608f76f62a2a3a2e))

## [0.23.0](https://github.com/sqlstack-io/sqlquack/compare/v0.22.1...v0.23.0) (2026-11-23)


### ⚠ BREAKING CHANGES

* `sqlquack-lint` is removed; its Rules now live in SQLClean. The tokenizer moved to `sqlquack-tokenize`: `sqlquack_syntax::{LosslessToken, TokenKind, TokenizeFailure}` are now `sqlquack_tokenize::{LosslessToken, TokenKind, TokenizeFailure}`, `sqlquack_syntax::lossless::*` is `sqlquack_tokenize::lossless::*`, `sqlquack_syntax::tokens::models::{Tokenizer, TokenizerConfig}` are `sqlquack_tokenize::tokens::models::{Tokenizer, TokenizerConfig}` (also `sqlquack_tokenize::{Tokenizer, TokenizerConfig}`), and `sqlquack_syntax::tokens::main::parse_dollar_string_token` is `sqlquack_tokenize::tokens::main::parse_dollar_string_token`; depend on `sqlquack-tokenize` to use them. `sqlquack_parse::tokenize` and the token, span and dialect types (`Token`, `TokenType`, `Span`, `Dialect`, `CallMode`) keep their paths. `CallMode` gains `QueriesLocated` and `AsWrittenLocated`, `TriggerBody` gains `Statements`, and tree nodes gain position fields (for example `Lateral::alias_span`, `Select::select_span`, `BinaryOp::operator_span`), so an exhaustive match on `CallMode` or `TriggerBody`, or a struct literal of those nodes, no longer compiles. A WITH before VALUES now parses as a SELECT over the VALUES carrying the WITH.

### Features

* give located trees every position Rules need; move the tokenizer into sqlquack-tokenize; remove sqlquack-lint ([c4acfd1](https://github.com/sqlstack-io/sqlquack/commit/c4acfd1894deb8809900105b6b1b1c0c8b32e234))

## [0.22.1](https://github.com/sqlstack-io/sqlquack/compare/v0.22.0...v0.22.1) (2026-11-19)


### Bug Fixes

* **databricks:** keep a returned table's columns apart from their spacing ([b0b1471](https://github.com/sqlstack-io/sqlquack/commit/b0b1471cd342a61ce31ee39221a5932df98cf5bd))
* **databricks:** keep raw strings in path keys and resources ([f20770d](https://github.com/sqlstack-io/sqlquack/commit/f20770ddfcd0ede142dbcec49c78f3d3b6028f62))
* **duckdb:** a NATURAL join of sides sharing no column is an error ([f676760](https://github.com/sqlstack-io/sqlquack/commit/f676760ac2097ce38a628da231be13023113290d))
* **semantics:** a join condition reads only the items its engine lets it see ([59eee86](https://github.com/sqlstack-io/sqlquack/commit/59eee86478b87288875746b243cd3de651a75631))
* **semantics:** a merged join key is ambiguous beside another item's column on PostgreSQL and SQLite ([1491c39](https://github.com/sqlstack-io/sqlquack/commit/1491c39718a7bd4155d1d991ff801271be352358))
* **semantics:** check a named window's frame offsets as column references ([5226ec6](https://github.com/sqlstack-io/sqlquack/commit/5226ec6a91e26d401265159891a0b2859cbb3ec7))
* **semantics:** find a named window in any case where names fold ([922c838](https://github.com/sqlstack-io/sqlquack/commit/922c838542ca2d4df3fdde2ecf680422c9b42d75))
* **semantics:** hide a projection's own alias from the named window it reads ([069484f](https://github.com/sqlstack-io/sqlquack/commit/069484f0d506b74dc89d4effad6b9fc758fa897f))
* **semantics:** keep SQLite and BigQuery window keys from reading output aliases ([d83c2c0](https://github.com/sqlstack-io/sqlquack/commit/d83c2c06fedfde94fb3b6c3f9e1206753a50886d))
* **semantics:** report an unknown base of a window an OVER reads ([b04bf13](https://github.com/sqlstack-io/sqlquack/commit/b04bf13943f46ce61379ad0c50698eb464108489))
* **sqlite:** a projection cannot read another projection's alias ([ed2cee5](https://github.com/sqlstack-io/sqlquack/commit/ed2cee511d3016b5366bd6b432425b357b45e60e))
* **syntax:** keep a window name's quotes in OVER and WINDOW ([c418bc4](https://github.com/sqlstack-io/sqlquack/commit/c418bc4b819d494a6d3f7ce49836fe1c435825c7))


### Performance Improvements

* **semantics:** append a level's findings instead of extending with them ([2d5c958](https://github.com/sqlstack-io/sqlquack/commit/2d5c95888f367b673e773b873461f1121b51203a))
* **semantics:** append a node's findings instead of extending with them ([0a2ba35](https://github.com/sqlstack-io/sqlquack/commit/0a2ba35459fe5c2b80af000ce9db0be1dbd7522c))
* **semantics:** collect a SELECT's scope once for its structure rules ([2859752](https://github.com/sqlstack-io/sqlquack/commit/2859752543010e8bad8038e1e51c1f26cf581d74))
* **semantics:** hash scope-rule names with FxHash ([5ffe07e](https://github.com/sqlstack-io/sqlquack/commit/5ffe07edf3de849ffc002b5b46918a0ba81b3f82))
* **semantics:** look an aggregate's name up in a set ([d783426](https://github.com/sqlstack-io/sqlquack/commit/d78342663d5c086cb70aef906b5c94efa1f69c23))
* **semantics:** normalise a single-spaced ASCII type spelling in one step ([f9ad7cf](https://github.com/sqlstack-io/sqlquack/commit/f9ad7cf968378bae819a2fec1bfc93fb5af1c55c))
* **semantics:** order findings without a position map when none needs one ([d8a7ea5](https://github.com/sqlstack-io/sqlquack/commit/d8a7ea5fa420d73542d81d4d691790528f09392e))
* **semantics:** read the family of a bare type word without allocating ([8fbc3b7](https://github.com/sqlstack-io/sqlquack/commit/8fbc3b788f1220b99dcf0599982c3b6041fc7a9d))
* **semantics:** record the lower analysis instruction count ([3ce6c39](https://github.com/sqlstack-io/sqlquack/commit/3ce6c391429da21beafda478240b41c383a43606))
* **semantics:** upper-case a Snowflake identifier in place ([6395a1e](https://github.com/sqlstack-io/sqlquack/commit/6395a1e77f9f8ac611686c38592d933830a3cc79))
* **semantics:** walk a node's children without collecting them first ([edfdae5](https://github.com/sqlstack-io/sqlquack/commit/edfdae5593d6158fdd812da1349c7598136f71e5))

## [0.22.0](https://github.com/sqlstack-io/sqlquack/compare/v0.21.0...v0.22.0) (2026-11-19)


### ⚠ BREAKING CHANGES

* **lint:** lint now refuses a Postgres, BigQuery or Databricks statement whose tree nests past 512 levels, as Snowflake, DuckDB and SQLite already did, so 600 UNION ALL operands or 600 `::` casts no longer lint in those dialects.
* keep each comma item's joins with it in Postgres, Databricks, DuckDB and SQL Server
* **bigquery:** sqlquack-syntax adds Literal::RawByteString and Literal::JoinedBytes, and GrantPrincipal gains a written field, so an exhaustive match on Literal or a GrantPrincipal struct literal no longer compiles.

### Bug Fixes

* **bigquery:** keep a column's and a struct field's OPTIONS ([563407d](https://github.com/sqlstack-io/sqlquack/commit/563407d12b0b425e51296d81036a0b3597c445bf))
* **bigquery:** keep raw and adjacent pieces of bytes, typed literals and grantees ([42ce043](https://github.com/sqlstack-io/sqlquack/commit/42ce0431c5a9cb4f63a99697a824b4f5301b6a63))
* keep each comma item's joins with it in Postgres, Databricks, DuckDB and SQL Server ([65ef255](https://github.com/sqlstack-io/sqlquack/commit/65ef2556d22343d71607eab399f7d0417d4c6c95))
* **lint:** a join inside UPDATE FROM never limits the write ([2786df7](https://github.com/sqlstack-io/sqlquack/commit/2786df7d5edeba8c0e32fb205259eb2b39fb585d))
* **lint:** add the call-depth note only to refusals of SQL that calls something ([30b4ed0](https://github.com/sqlstack-io/sqlquack/commit/30b4ed0c8b84329cc22c62d5c122ca3c2feea459))
* **lint:** leave a fixture harness's empty relation out of the call depth ([f5af215](https://github.com/sqlstack-io/sqlquack/commit/f5af2154b72b2741d869751da0230a9c683154d0))
* **lint:** make max_function_call_depth the exact limit in every linted dialect ([22a2a4c](https://github.com/sqlstack-io/sqlquack/commit/22a2a4cdfd0832aebed01b85df42f2439189ed2b))
* **lint:** read cross joins and join keys inside FROM items ([9db4571](https://github.com/sqlstack-io/sqlquack/commit/9db457140547d1c387990e7c610ab108af7be3a5))
* **lint:** refuse native trees past the 512 depth ceiling, as the shared parser does ([5c7768c](https://github.com/sqlstack-io/sqlquack/commit/5c7768ccfa7a9c9625a31df4e95fb5049a5b83b0))
* list a nested USING or NATURAL key once in a star ([2ddd78a](https://github.com/sqlstack-io/sqlquack/commit/2ddd78a58436da41fea6884ca7dbb315606dc400))

## [0.21.0](https://github.com/sqlstack-io/sqlquack/compare/v0.20.0...v0.21.0) (2026-11-18)


### ⚠ BREAKING CHANGES

* **lint:** take the function-call depth limit in parse_sql
* **bigquery:** the BigQuery parser now gives Literal::JoinedString, serialized as literal_type "joined_string" with the joined value and its pieces, for adjacent string literals, where it gave one Literal::String of the joined text.

### Features

* **lint:** take the function-call depth limit in parse_sql ([4dd6098](https://github.com/sqlstack-io/sqlquack/commit/4dd60981c0fbce202fa36f6ce13eb3d040bfc0ae))


### Bug Fixes

* **bigquery:** accept a grantee written as adjacent strings ([714b00f](https://github.com/sqlstack-io/sqlquack/commit/714b00fc1a5cbc18c346a2c5e6cf297b055ca1d9))
* **bigquery:** keep the pieces of adjacent string literals ([2cd6428](https://github.com/sqlstack-io/sqlquack/commit/2cd6428f560002a5c46e186a0a69a38dcaf0e4b1))
* **databricks:** keep a function parameter's comment ([999f5ed](https://github.com/sqlstack-io/sqlquack/commit/999f5edf5c8ad0a9a249b8923b4a967eceefa593))
* **databricks:** keep only raw and adjacent credential values as written ([62fa162](https://github.com/sqlstack-io/sqlquack/commit/62fa162e8fb526bc9720e9065d1871c1c7069a19))
* **databricks:** keep raw and adjacent credential values and raw typed literals ([9d4a165](https://github.com/sqlstack-io/sqlquack/commit/9d4a1650267cf55113a9af596ae15140ac6c9c20))
* **databricks:** keep raw and adjacent spellings in plain-text string fields ([b05cd31](https://github.com/sqlstack-io/sqlquack/commit/b05cd31d40cbfadcb57fd32f830012685703fde8))
* **lint:** resolve a named window written after a parenthesized query ([20f2974](https://github.com/sqlstack-io/sqlquack/commit/20f297429a38f3c6de85195bc429f679fef8d319))
* **semantics:** check only the named windows an OVER reads ([6e472d2](https://github.com/sqlstack-io/sqlquack/commit/6e472d23657a5939ae51cb3bc7dfebc8dbbe3390))
* **semantics:** check the keys of a named window as column references ([ffbf6f8](https://github.com/sqlstack-io/sqlquack/commit/ffbf6f8db2fba15cacaa7ec32eb2c9a49e78662b))
* **semantics:** match a raw typed literal against its plain form when grouping ([3c700d5](https://github.com/sqlstack-io/sqlquack/commit/3c700d55e4b8a81e621dbd364e2069968982652d))
* **semantics:** match grouping expressions whose strings are spelled differently ([08c3ff8](https://github.com/sqlstack-io/sqlquack/commit/08c3ff804751356b26122780962183fd81f545c9))

## [0.20.0](https://github.com/sqlstack-io/sqlquack/compare/v0.19.0...v0.20.0) (2026-11-18)


### Features

* set the function-call depth limit at every parse entry point, up to 512 ([bf55b2b](https://github.com/sqlstack-io/sqlquack/commit/bf55b2b5268e9a569d1f51f7ed8bfb5f1a10f346))

## [0.19.0](https://github.com/sqlstack-io/sqlquack/compare/v0.18.2...v0.19.0) (2026-11-18)


### ⚠ BREAKING CHANGES

* **databricks:** sqlquack-syntax adds the Literal::JoinedString variant; code that matches Literal exhaustively must handle it.

### Bug Fixes

* **databricks:** keep a written SELECT * FROM ( query ) apart from a pipe step's input ([343c48a](https://github.com/sqlstack-io/sqlquack/commit/343c48aba8856511a0e7d43aeb7ea6be790dedff))
* **databricks:** keep an insert's parenthesised query apart from a bare one ([ca4766f](https://github.com/sqlstack-io/sqlquack/commit/ca4766f645a903b2abf754fe931c00dd11d24f99))
* **databricks:** keep raw and adjacent string spellings in trees ([75f899a](https://github.com/sqlstack-io/sqlquack/commit/75f899a236fc392a8704f662907f9b511fd9a30a))
* **databricks:** keep WINDOW after a parenthesized query apart from WINDOW inside it ([be79864](https://github.com/sqlstack-io/sqlquack/commit/be79864e9d7662cf3e1f93ce41eb935816d4d795))
* **databricks:** keep WINDOW after a parenthesized query apart from WINDOW over a star of it ([b5a5cbe](https://github.com/sqlstack-io/sqlquack/commit/b5a5cbe7287b546554ffbd91d741536320a04ef5))
* **databricks:** refuse a TIMESTAMP with both WITHOUT and WITH LOCAL TIME ZONE ([933d416](https://github.com/sqlstack-io/sqlquack/commit/933d416d38ea82297103b9b430f5c558a8b9a4d6))
* **databricks:** type a raw string literal as STRING ([b860d19](https://github.com/sqlstack-io/sqlquack/commit/b860d19a68ca181d45427ed56e1a9d95c9c4044a))
* **semantics:** check a join written in parentheses as a bare join ([c07b62c](https://github.com/sqlstack-io/sqlquack/commit/c07b62c39c4ccd957f5f694cb74c569a0cd28c60))
* **semantics:** check USING beside a parenthesised join against its own sides ([6225fbe](https://github.com/sqlstack-io/sqlquack/commit/6225fbe3cb7acaf90fa457e6b46573da57333f97))

## [0.18.2](https://github.com/sqlstack-io/sqlquack/compare/v0.18.1...v0.18.2) (2026-11-18)


### Bug Fixes

* **snowflake:** refuse a bare ALTER instead of panicking ([57e8722](https://github.com/sqlstack-io/sqlquack/commit/57e8722ab0466062c317d8405e9430e3c4fe25fd))

## [0.18.1](https://github.com/sqlstack-io/sqlquack/compare/v0.18.0...v0.18.1) (2026-11-18)


### Bug Fixes

* **databricks:** keep backquoted names, tag key spellings and credential value types in trees ([3d42bc3](https://github.com/sqlstack-io/sqlquack/commit/3d42bc38bfd6be89564ef3982c44c27888a39463))
* **databricks:** keep every pair of parentheses around a relation or a query ([95cfa5c](https://github.com/sqlstack-io/sqlquack/commit/95cfa5cd3923a614bfeb8e062947ab771a4e1978))
* **lint:** check CTEs of a parenthesized query or CTE body ([dc288d1](https://github.com/sqlstack-io/sqlquack/commit/dc288d1f5fc286d8ff9ddf200818dfa383f7f821))
* **semantics:** resolve clauses after a parenthesized query in its scope ([2744091](https://github.com/sqlstack-io/sqlquack/commit/27440910036908d93e3f7ea585cf3aca4804869b))

## [0.18.0](https://github.com/sqlstack-io/sqlquack/compare/v0.17.0...v0.18.0) (2026-11-18)


### ⚠ BREAKING CHANGES

* **syntax:** serialised call trees carry set_quantifier in place of distinct.

### Features

* **coverage:** add the Databricks receipt referee and obligation tooling ([d664bd7](https://github.com/sqlstack-io/sqlquack/commit/d664bd7aee12e7aa8a45ad3bcce6ab052d6ec540))
* **coverage:** catalogue Databricks SQL 2026.45 forms, obligations and cases ([afab7da](https://github.com/sqlstack-io/sqlquack/commit/afab7da213633ffe71b8b3252b3085555293eb71))
* **coverage:** screen approved forms for collapsed spellings and gate on it ([d3edd9d](https://github.com/sqlstack-io/sqlquack/commit/d3edd9da090ebeb811bdbd4bbb2db71a9e0bb517))
* **databricks:** parse documented Databricks SQL into trees that keep what was written ([80e56ab](https://github.com/sqlstack-io/sqlquack/commit/80e56ab50a373f4ecd83d3a109b173414435e380))
* **syntax:** record a call's set quantifier, keeping an explicit ALL ([9863126](https://github.com/sqlstack-io/sqlquack/commit/9863126fbe2c5952970729a489931ee343d7f3a8))


### Bug Fixes

* **convert:** count rows for COUNT(*) under any quantifier but DISTINCT ([5377ca7](https://github.com/sqlstack-io/sqlquack/commit/5377ca74c7ddf7116a0934d972d42e158b428624))
* **databricks:** keep the formatter and lint refusing forms whose trees are not reviewed ([fb82b0a](https://github.com/sqlstack-io/sqlquack/commit/fb82b0a4402255a6e35b1607de9081e05c61089c))
* **website:** show the current Snowflake syntax figure and fail CI when it drifts ([28dab7b](https://github.com/sqlstack-io/sqlquack/commit/28dab7bd05df7b2bb08c6c01ba672f5e8be87742))

## [0.17.0](https://github.com/sqlstack-io/sqlquack/compare/v0.16.1...v0.17.0) (2026-11-16)


### Features

* **syntax:** keep the FROM in Snowflake's DATE_PART ( part FROM value ) ([5df8007](https://github.com/sqlstack-io/sqlquack/commit/5df80079bfbba305eefe332f2df70b1a5c4929db))
* **syntax:** type Snowflake's TABLE ( ... ) arguments of data metric functions ([b31c298](https://github.com/sqlstack-io/sqlquack/commit/b31c298397c7ad61411d52a823bd092ebc2df084))


### Bug Fixes

* **snowflake:** keep a stage's PATTERN and DIRECTORY's stage as written ([776ddfd](https://github.com/sqlstack-io/sqlquack/commit/776ddfd8c9dfc2c2987b6be5b40431ef6bcab91c))
* **snowflake:** keep an external table column's NOT NULL and quoted stage ([a3821b0](https://github.com/sqlstack-io/sqlquack/commit/a3821b08b4744cc5e8b9dd777d3903f46f2f9797))
* **snowflake:** keep AT, BEFORE and CHANGES after a FROM call ([f250959](https://github.com/sqlstack-io/sqlquack/commit/f2509597338d3608e08240d76ca5fcc4a34bd181))
* **snowflake:** keep BY and = in ALTER SEQUENCE INCREMENT ([203c8dd](https://github.com/sqlstack-io/sqlquack/commit/203c8dd62a8e3fa5c0c6f40e2ba601fc506f67e6))
* **snowflake:** keep FORCE, ADD, WITH and OR REPLACE in table and index DDL ([f789c72](https://github.com/sqlstack-io/sqlquack/commit/f789c7248a93ecedbd975b4ed0e2f00c03d2a077))
* **snowflake:** keep NOT ENFORCED after a foreign key's reference ([0fec4c1](https://github.com/sqlstack-io/sqlquack/commit/0fec4c1d1119cdb3d5402effa0e2123edfba62f1))
* **snowflake:** keep the IDENTITY, START and GENERATED ALWAYS spellings of columns ([b62c6f3](https://github.com/sqlstack-io/sqlquack/commit/b62c6f3891aa9c1c154b8fafe6159ef11a1c552d))
* **snowflake:** keep the optional WITH before view policies and tags ([6b38506](https://github.com/sqlstack-io/sqlquack/commit/6b385067d51203dcb82e7d7394c63efd6ae194a9))
* **snowflake:** label an online feature table's PRIMARY KEY ([286d50c](https://github.com/sqlstack-io/sqlquack/commit/286d50cbde003f0084ad472a091f647a86743c9e))
* **snowflake:** read // straight after a stage as its path ([20f07ed](https://github.com/sqlstack-io/sqlquack/commit/20f07ed8e5186a13b8af2799ca00bf8dfd0e171b))
* **snowflake:** read a model's quoted stage source as a stage reference ([1b1edf9](https://github.com/sqlstack-io/sqlquack/commit/1b1edf9dc66749dcfa5796c8515123e02ab77710))
* **snowflake:** read event routing regions as strings and SHOW OPENFLOW DATA PLANE INTEGRATIONS LIKE ([0d76ab1](https://github.com/sqlstack-io/sqlquack/commit/0d76ab1e47e8c7406fba9d47767f4da3799b7ae6))
* **snowflake:** refuse a window name inside a semantic view metric window ([b364ec6](https://github.com/sqlstack-io/sqlquack/commit/b364ec6311c55a58ea812aedd49914a9413c3265))
* **snowflake:** refuse UNSET values, a nameless SHOW PARAMETERS IN TABLE, MODIFY CONSTRAINT columns and a bare UNIQUE ([83275ca](https://github.com/sqlstack-io/sqlquack/commit/83275ca61f3af329cd20d3cadeabbca3db52e629))
* **snowflake:** tell LAST and LATEST deployments and IMPORTS aliases apart ([8d69193](https://github.com/sqlstack-io/sqlquack/commit/8d691932d6ccd341fcabc143d895d5111266495a))
* **snowflake:** type a view's RECURSIVE and scope and a materialization's REFRESH_MODE ([ff178bc](https://github.com/sqlstack-io/sqlquack/commit/ff178bc62a1a117ea29a7d5d85f2d337b8ef5d85))
* **snowflake:** type ENABLE, DISABLE and VALIDATE constraint modifiers and keep inline ones ([dde64db](https://github.com/sqlstack-io/sqlquack/commit/dde64db6cbb753badb87e508a635ef063d1ac92b))
* **snowflake:** type SHOW SERVICES EXCLUDE JOBS and OF TYPE ([b88a9a2](https://github.com/sqlstack-io/sqlquack/commit/b88a9a20d8a82767a6005eb1a53ed8bbbbf0f536))
* **snowflake:** type the out-of-line constraints of DEFINE, ICEBERG and DYNAMIC tables ([fd26983](https://github.com/sqlstack-io/sqlquack/commit/fd26983a78e1bea2fe78b55f2a43b0ac10551ff1))
* **syntax-cases:** continue a list whose first item is optional ([a502766](https://github.com/sqlstack-io/sqlquack/commit/a502766274a36f969f647a691eef6f79cbf259f5))
* **syntax-receipts:** classify four errno 1003 post-parse messages as semantic errors ([303d389](https://github.com/sqlstack-io/sqlquack/commit/303d38950be1f2c469c22286da37c458273d8f47))
* **tokenizer:** end a stage at punctuation so a following // is still a comment ([fdb054a](https://github.com/sqlstack-io/sqlquack/commit/fdb054a61fd111a427f5f051f25338efe4fb0d24))

## [0.16.1](https://github.com/sqlstack-io/sqlquack/compare/v0.16.0...v0.16.1) (2026-11-15)


### Bug Fixes

* **ci:** pin DuckShop's SQLBuild to the published 0.231.0 ([d70daf2](https://github.com/sqlstack-io/sqlquack/commit/d70daf21f07163053f16322c78fe2f4388aab032))
* **format:** indent a call's arguments under a line comment before the first one ([30471a5](https://github.com/sqlstack-io/sqlquack/commit/30471a5e93ceb9a6d9fc2773719d29e4be579a53))
* **format:** keep a call's argument indent after a comment between arguments ([60450f9](https://github.com/sqlstack-io/sqlquack/commit/60450f92a8501d599a1c784cc809948b6b2387c4))
* **receipts:** classify 3 more reviewed post-parse Snowflake errnos as semantic ([b143f0e](https://github.com/sqlstack-io/sqlquack/commit/b143f0e8daca2221c995a1675145cccdbdbbb141))
* **snowflake:** give a dynamic table's secondary roles their own property ([e7d8135](https://github.com/sqlstack-io/sqlquack/commit/e7d813577b366f8e4754549888a23be2f3a89c5f))
* **snowflake:** keep a procedure's RETURNS TABLE columns as fields with their quoting ([b93ecd9](https://github.com/sqlstack-io/sqlquack/commit/b93ecd97028f011b783c980ade80476951fec722))
* **snowflake:** keep a table's policy, tag and column actions structured ([729ec8a](https://github.com/sqlstack-io/sqlquack/commit/729ec8ace02a3dd2d8ff46b3af553fa499cdef70))
* **snowflake:** keep an anonymous procedure's options as properties ([6f6611f](https://github.com/sqlstack-io/sqlquack/commit/6f6611f14b531ed92dc6c7a57b1d33ca75686b35))
* **snowflake:** read a MODIFY ( ... ) column list as typed column actions ([f380256](https://github.com/sqlstack-io/sqlquack/commit/f38025693036d78c834662839a629f7ad4476619))
* **snowflake:** refuse nameless ALTER SET and UNSET that Snowflake refuses ([737c81d](https://github.com/sqlstack-io/sqlquack/commit/737c81d3f50395b2f5676cdd552d02482f307869))
* **snowflake:** set a constraint's options on their typed fields ([6d8844c](https://github.com/sqlstack-io/sqlquack/commit/6d8844c1f22b54fb8a7f32e1e77d36ed08e48eee))
* **syntax-cases:** write a Snowflake keyword with a lower-case part instead of filling it ([16441f7](https://github.com/sqlstack-io/sqlquack/commit/16441f7c7ce908e6d852100e1a2b007181d6cc3d))
* **syntax-cases:** write an option of a branch that is one group of options ([22beb13](https://github.com/sqlstack-io/sqlquack/commit/22beb130f456490a3664b779b6bcdd9ee8d4b8c4))

## [0.16.0](https://github.com/sqlstack-io/sqlquack/compare/v0.15.2...v0.16.0) (2026-11-15)


### Features

* **syntax-cases:** give each option its own case when Snowflake refuses the case with every option ([dd29423](https://github.com/sqlstack-io/sqlquack/commit/dd2942359da58df2bdc91ebe89e5e0edcd0a4bd5))
* **syntax-cases:** settle a documented observe case from its agreeing receipts ([59523a0](https://github.com/sqlstack-io/sqlquack/commit/59523a034898c4d98c414893b9728650cb7b8313))
* **syntax-coverage:** accept receipts whose echoed locations were redacted, counted in locations_redacted ([0af6b9e](https://github.com/sqlstack-io/sqlquack/commit/0af6b9eb0ce6a6181ef9b87f34114fe6bccbc62e))


### Bug Fixes

* **receipts:** classify 11 more reviewed post-parse Snowflake errnos as semantic ([7a79c7e](https://github.com/sqlstack-io/sqlquack/commit/7a79c7ec51e1cb9340bd8c292fdf423db43e1e1e))
* **receipts:** classify 20 reviewed post-parse Snowflake errnos as semantic ([95bbd31](https://github.com/sqlstack-io/sqlquack/commit/95bbd31c5bffc13fa181c9353fc1f5f379370190))
* **snowflake:** accept UNSET COMMENT when modifying an external consumer's token ([e1b7851](https://github.com/sqlstack-io/sqlquack/commit/e1b78511e822c96651ee9de4a1712cad7c727854))
* **snowflake:** file a cortex search service's query under AS ([3c78fb2](https://github.com/sqlstack-io/sqlquack/commit/3c78fb201cb5d015a47c0c473c5995ac7814d22a))
* **snowflake:** keep a CREATE SNOWFLAKE class privilege's class path as a name ([fcb06a7](https://github.com/sqlstack-io/sqlquack/commit/fcb06a743d8048707d67a9de6598cfd43475b21c))
* **snowflake:** keep a routine's COPY = ( GRANTS ) as a keyword ([deeefb7](https://github.com/sqlstack-io/sqlquack/commit/deeefb7325827777c7d1d2d96bdb68cc14da87fb))
* **snowflake:** keep a storage location's option keys as keys ([9d597e4](https://github.com/sqlstack-io/sqlquack/commit/9d597e4c33f31d865d31b42431821ed6e9fb140f))
* **snowflake:** keep AS on a RESAMPLE metadata column alias ([62b46a3](https://github.com/sqlstack-io/sqlquack/commit/62b46a3bee11327e55cd383d71abadc6c0b7834b))
* **snowflake:** keep COLUMN and SET/DATA/TYPE words on an altered column ([18da974](https://github.com/sqlstack-io/sqlquack/commit/18da974850a342445ffc82aaf4237a57e2f6d68a))
* **snowflake:** keep how a temporary function was spelled ([06bc929](https://github.com/sqlstack-io/sqlquack/commit/06bc92934efec30501d9deb361fe36d882a9b5c5))
* **snowflake:** keep how a temporary procedure spells TEMP ([b76b514](https://github.com/sqlstack-io/sqlquack/commit/b76b51476f30437dae61e2464faddc08ceb823a9))
* **snowflake:** keep the account of a DCM database definition ([8a8918a](https://github.com/sqlstack-io/sqlquack/commit/8a8918addfa011101d8fc886ceeb4f0ea6581ce2))
* **snowflake:** key an ALTER option list's pairs by name ([3d4866a](https://github.com/sqlstack-io/sqlquack/commit/3d4866acfc795ca71ab20e074fa5b868c6e35dd1))
* **snowflake:** parse an unquoted s3:// URL in ALTER STAGE SET URL ([674b0ab](https://github.com/sqlstack-io/sqlquack/commit/674b0ab6aecd7b8edc97c86121de55b251f72442))
* **snowflake:** read a group's object types as keywords ([eec8538](https://github.com/sqlstack-io/sqlquack/commit/eec853887b9c1c5d7fe888c2da4317c0e25c45cc))
* **snowflake:** read a MATCH_RECOGNIZE pattern as a tree of variables, anchors, groups and quantifiers ([c46fa19](https://github.com/sqlstack-io/sqlquack/commit/c46fa19d21ee825d43ad092e3c02ee6c4da5f48f))
* **snowflake:** read documented property keywords as keywords ([bdaca33](https://github.com/sqlstack-io/sqlquack/commit/bdaca33d3f9fe183249a957bb379665ea36e42f0))
* **snowflake:** refuse EXECUTE AS USER after another property in ALTER TASK SET ([0e51c5b](https://github.com/sqlstack-io/sqlquack/commit/0e51c5be0a789db9d078e514d0edf85546305615))

## [0.15.2](https://github.com/sqlstack-io/sqlquack/compare/v0.15.1...v0.15.2) (2026-11-15)


### Bug Fixes

* **snowflake:** accept a replica source that leaves out its account ([e2627ac](https://github.com/sqlstack-io/sqlquack/commit/e2627acac8b3ad1da42c8e0d6b2fa10ecdda1575))
* **snowflake:** accept SET and UNSET on a named organization ([f4eb433](https://github.com/sqlstack-io/sqlquack/commit/f4eb4332a82cb636505edd8b1eebff8aa451e6a6))
* **snowflake:** accept UNDROP VOLUME ([19bd250](https://github.com/sqlstack-io/sqlquack/commit/19bd2501a875e08db8c116300f813ec00ba73d0b))
* **snowflake:** keep a compute pool's STOP ALL OF TYPE as a property ([6ee2777](https://github.com/sqlstack-io/sqlquack/commit/6ee277729dc84618b95be3efeb22f9f9511463dd))
* **snowflake:** keep a data metric function's left-out column ([d423774](https://github.com/sqlstack-io/sqlquack/commit/d423774656172a0c1774615114ec394a47cbd377))
* **snowflake:** keep a data movement rule's row-limit return type ([71f3d27](https://github.com/sqlstack-io/sqlquack/commit/71f3d27ff6c76cdb5bfeb3242544e00a7edb37b2))
* **snowflake:** keep a sequence's OR ALTER and optional words ([5303167](https://github.com/sqlstack-io/sqlquack/commit/53031674f2c4e176ffafef94d836904e57f7efa5))
* **snowflake:** keep an account policy's FOR ALL scope as a property ([e2d0cc9](https://github.com/sqlstack-io/sqlquack/commit/e2d0cc9478ccd62ef7ee24e365d016ee6199cd24))
* **snowflake:** keep an ALTER action's own IF [ NOT ] EXISTS as a flag ([025de45](https://github.com/sqlstack-io/sqlquack/commit/025de45c3f8bf62c428e545610b27fbb0d92efec))
* **snowflake:** keep an application upgrade's PATCH as a property ([01cac50](https://github.com/sqlstack-io/sqlquack/commit/01cac504ae9c14a38a20c119a2bcce654bfebaff))
* **snowflake:** keep FROM in model and specification create sources ([578fbc5](https://github.com/sqlstack-io/sqlquack/commit/578fbc5e4cc8b870f3486bad3490d07cb64eff6a))
* **snowflake:** keep how a temporary table was spelled ([eeee673](https://github.com/sqlstack-io/sqlquack/commit/eeee67311d1097333efa3872a773fd260003c481))
* **snowflake:** keep KEY = value pairs in ALTER lists keyed ([e9270c3](https://github.com/sqlstack-io/sqlquack/commit/e9270c387626224e754b5ca880aaa104630d3c39))
* **snowflake:** keep PAT as written in token actions ([527eb2f](https://github.com/sqlstack-io/sqlquack/commit/527eb2f2f19a6bc90163daa52808ecd2192aee4e))
* **snowflake:** keep quoting and AS in MATCH_RECOGNIZE names ([51b2bf4](https://github.com/sqlstack-io/sqlquack/commit/51b2bf4acce2b49c974ba62367f69e78f9e9d952))
* **snowflake:** keep whether RUNTIME_VERSION and PACKAGES were written as strings ([5b3b325](https://github.com/sqlstack-io/sqlquack/commit/5b3b325093db181782a4e622eb357177d93858ab))
* **snowflake:** label an Openflow connector's runtime and a runtime's deployment ([1fe132d](https://github.com/sqlstack-io/sqlquack/commit/1fe132daca1121ea2e92e7281665b9fa5ca26074))
* **snowflake:** read a multi-word property value as a keyword phrase ([1a0c294](https://github.com/sqlstack-io/sqlquack/commit/1a0c29463657b80c105c48ab09136078b1b4f44e))
* **snowflake:** read CREATE ORGANIZATION &lt;name&gt; as an organization with properties ([bbf26e5](https://github.com/sqlstack-io/sqlquack/commit/bbf26e514850ed39c23b73c99db5ff6a93a54d41))
* **snowflake:** refuse FORCE before an account policy's FOR ALL scope ([cff2aea](https://github.com/sqlstack-io/sqlquack/commit/cff2aea89505cf3050b038d6b7528d760e4f68a5))

## [0.15.1](https://github.com/sqlstack-io/sqlquack/compare/v0.15.0...v0.15.1) (2026-11-15)


### Bug Fixes

* **syntax:** accept function calls nested up to 256 deep ([a236172](https://github.com/sqlstack-io/sqlquack/commit/a2361727aaabb67c3007755c9999925dfe357b96))
* **syntax:** leave out a create's source when the statement names none ([847e266](https://github.com/sqlstack-io/sqlquack/commit/847e2660f365d40a4430942acf598066b491cb91))

## [0.15.0](https://github.com/sqlstack-io/sqlquack/compare/v0.14.0...v0.15.0) (2026-11-15)


### Features

* **syntax-coverage:** audit forms without obligations and settle generated cases by their receipts ([5df886e](https://github.com/sqlstack-io/sqlquack/commit/5df886e054902c1f1d86fe1ae247803b36b2f250))
* **syntax-coverage:** count a refusal after parsing from a statement run without privileges ([b7f1fad](https://github.com/sqlstack-io/sqlquack/commit/b7f1fad788c0fe20c008f066a9ffec2b0750b906))
* **syntax-coverage:** count Snowflake's syntax error from a statement run without privileges ([7a09b73](https://github.com/sqlstack-io/sqlquack/commit/7a09b736fb768df4c5e945782b6b47567f13a152))
* **syntax:** record Snowflake statements as structured trees that keep what was written ([d758b8c](https://github.com/sqlstack-io/sqlquack/commit/d758b8c76e941f133486e5edc7d998ae734d81f5))


### Bug Fixes

* **snowflake:** accept an online feature table without a warehouse ([71b1b61](https://github.com/sqlstack-io/sqlquack/commit/71b1b61f5cbefe278450ea22d0323c03ea4d48d1))
* **snowflake:** accept one string as a share's SET ACCOUNTS ([8444210](https://github.com/sqlstack-io/sqlquack/commit/8444210cac95cad9f4f35aa3f5e21b7af23bee2f))
* **snowflake:** read CREATE OPENFLOW CONNECTOR, DEPLOYMENT and RUNTIME as trees ([af5807f](https://github.com/sqlstack-io/sqlquack/commit/af5807f612dd643316f33235b5614bf084aefca8))
* **snowflake:** read DROP ACCOUNT with its grace period and DROP MANAGED ACCOUNT ([77c95de](https://github.com/sqlstack-io/sqlquack/commit/77c95de58b87c682accad5d8f9f10f8b7d5bc45a))
* **snowflake:** refuse a bare word where Snowflake takes only a string ([d2087cd](https://github.com/sqlstack-io/sqlquack/commit/d2087cd8491d989fefe414028adbf9a7e8713b9a))
* **snowflake:** refuse a trailing comma in more lists Snowflake rejects ([4b917b0](https://github.com/sqlstack-io/sqlquack/commit/4b917b0d78b7b2b6b380810f4b04e1cf0e0ff5dc))
* **snowflake:** refuse OR REPLACE with IF NOT EXISTS on warehouses ([a1a8273](https://github.com/sqlstack-io/sqlquack/commit/a1a82732167ecf64b3d4c4c02a980e6d787888c5))
* **snowflake:** refuse tags on CREATE OR ALTER SECRET and DEFINE SECRET ([e717f7c](https://github.com/sqlstack-io/sqlquack/commit/e717f7ca15554bff443638f0b55404337cb157e7))
* **snowflake:** reject a bare word in UNDROP IDENTIFIER( ) and stop emitting empty names ([e267ab5](https://github.com/sqlstack-io/sqlquack/commit/e267ab52dff92297c19869a4d607a1568cd0e2e2))
* **syntax-cases:** settle observed cases from probe-role receipts under any setup ([42222de](https://github.com/sqlstack-io/sqlquack/commit/42222de5a32b906e0f341a9e4325b4cab7711436))
* **syntax-catalogue:** type the committed catalogue file as tables of rows ([b6aa931](https://github.com/sqlstack-io/sqlquack/commit/b6aa9319441b1e3ba479ae4b118f52408a6adacc))
* **syntax-receipts:** read errno 90105, no current database, as a post-parse semantic error ([36ad01f](https://github.com/sqlstack-io/sqlquack/commit/36ad01f412f4684eccbf5c6134f8be2b0215fa75))

## [0.14.0](https://github.com/sqlstack-io/sqlquack/compare/v0.13.0...v0.14.0) (2026-11-14)


### Features

* **syntax-coverage:** give each repeated Snowflake list item its own missing, class and wrong-kind obligations ([c1bc963](https://github.com/sqlstack-io/sqlquack/commit/c1bc9637a4e3ea28e88b0390105d57c34b81be90))


### Bug Fixes

* **snowflake:** accept an extra semicolon after a block member ([6ab4b43](https://github.com/sqlstack-io/sqlquack/commit/6ab4b439fae9ad7b752482169e3f4b5ea4f5126c))
* **snowflake:** read a DEFINE TABLE reference without FOREIGN KEY ([4716d40](https://github.com/sqlstack-io/sqlquack/commit/4716d4079db4b7e1388abc643e44f9f48dae66bb))
* **snowflake:** refuse a COPY query column that starts with a dot ([019bcb7](https://github.com/sqlstack-io/sqlquack/commit/019bcb7896787c6030f0883edfee22aad8735a3b))
* **snowflake:** refuse a string after a comma in a group's name list ([de82372](https://github.com/sqlstack-io/sqlquack/commit/de823729a0eb6dcdb7580094628864402297d17a))
* **snowflake:** refuse a trailing comma in lists Snowflake rejects ([29076cb](https://github.com/sqlstack-io/sqlquack/commit/29076cb73186d3f32a229167c83e5f09b6787041))
* **snowflake:** refuse DEFAULT on an untyped column ([58306ca](https://github.com/sqlstack-io/sqlquack/commit/58306cac42788f426d47c2143d71cb8d74308884))
* **snowflake:** report ALTER WAREHOUSE parse errors instead of raw text ([1f5a47d](https://github.com/sqlstack-io/sqlquack/commit/1f5a47d15d75a85d7a243349b54ea729f3a00d7f))
* **snowflake:** require a string key after a comma in header and secret maps ([b51361d](https://github.com/sqlstack-io/sqlquack/commit/b51361d495baec78b9bbdd6a4182fe076d4e7da3))
* **statements:** keep the empty part when keying a call that leaves out its schema ([9688f25](https://github.com/sqlstack-io/sqlquack/commit/9688f2589baee26277aff4aac095d37411e5963f))
* **syntax-coverage:** fill the second list item after an optional first item, and drop the parenthesis of an elided group ([76df2b1](https://github.com/sqlstack-io/sqlquack/commit/76df2b1d4a72453319f58454b7769d6233db8af7))
* **syntax-coverage:** keep a documented Snowflake refusal when a positive renders the same SQL ([d839796](https://github.com/sqlstack-io/sqlquack/commit/d83979689f92a765b5db073d4d9c31938868d0c3))
* **syntax-snowflake:** parse a call that leaves out its schema as a qualified call ([478d767](https://github.com/sqlstack-io/sqlquack/commit/478d767fba452605295fe85ac09f4e0eab487b7e))
* **syntax-snowflake:** represent the part a..b leaves out as null with a flag on its owner ([9801e12](https://github.com/sqlstack-io/sqlquack/commit/9801e127e6c41f85a70cf39002c146261eb03d1f))

## [0.13.0](https://github.com/sqlstack-io/sqlquack/compare/v0.12.1...v0.13.0) (2026-11-14)


### Features

* **snowflake:** keep routine and task properties, WITH, FORCE, quoted UNSET names, contact purposes and grant targets as tree fields ([cc0d126](https://github.com/sqlstack-io/sqlquack/commit/cc0d1265c62bb7ab0109eac385a0da9800b9bdc2))
* **snowflake:** parse CREATE TASK, CREATE VARIABLE, CREATE TAG, CONNECTION, DCM PROJECT and ALTER WAREHOUSE into typed trees ([cc0d126](https://github.com/sqlstack-io/sqlquack/commit/cc0d1265c62bb7ab0109eac385a0da9800b9bdc2))
* **syntax-coverage:** classify Snowflake name, word, numeric and placeholder operands, each with a wrong-kind case ([cc0d126](https://github.com/sqlstack-io/sqlquack/commit/cc0d1265c62bb7ab0109eac385a0da9800b9bdc2))
* **syntax-coverage:** re-baseline Snowflake form coverage to 2,826 verified of 3,479 forms (from 2,730) ([cc0d126](https://github.com/sqlstack-io/sqlquack/commit/cc0d1265c62bb7ab0109eac385a0da9800b9bdc2))


### Bug Fixes

* **snowflake:** match recorded verdicts for typed operands ([63f2195](https://github.com/sqlstack-io/sqlquack/commit/63f21954e1b147a57edb92081819c4797b9fa558))
* **snowflake:** record no name for an unnamed ALTER USER or ON ACCOUNT grant, and flag an access token's own IF [NOT] EXISTS ([cc0d126](https://github.com/sqlstack-io/sqlquack/commit/cc0d1265c62bb7ab0109eac385a0da9800b9bdc2))
* **snowflake:** refuse reserved words as INSERT, MERGE, LIKE, SET and UNSET names ([cc0d126](https://github.com/sqlstack-io/sqlquack/commit/cc0d1265c62bb7ab0109eac385a0da9800b9bdc2))

## [0.12.1](https://github.com/sqlstack-io/sqlquack/compare/v0.12.0...v0.12.1) (2026-11-13)


### Bug Fixes

* **syntax-coverage:** give numeric Snowflake operands a wrong-kind case ([4c8dfa6](https://github.com/sqlstack-io/sqlquack/commit/4c8dfa65d803151df0a0e87274017284135d8d33))
* **syntax:** keep a SHOW DELEGATED AUTHORIZATIONS clause as its scope ([fb725bf](https://github.com/sqlstack-io/sqlquack/commit/fb725bfdc7a2f63da1359ad624970d3a3316b55b))
* **syntax:** match recorded Snowflake verdicts and keep SHOW, task and routine modifiers in the tree ([d4ca770](https://github.com/sqlstack-io/sqlquack/commit/d4ca77041337a9f30f5a15827684a8f4ac4aefb7))
* **syntax:** match recorded Snowflake verdicts for listing filters, routine maps, Iceberg versions and partial CREATE kinds ([3bd7322](https://github.com/sqlstack-io/sqlquack/commit/3bd73222493fae27e6095de30763a489f8b553bc))
* **syntax:** match recorded Snowflake verdicts for numeric operands given a string ([e8ab1f7](https://github.com/sqlstack-io/sqlquack/commit/e8ab1f7e10cd7bf55dbd6f369b9fe707a86bc990))
* **syntax:** refuse a malformed CREATE INTERACTIVE WAREHOUSE and read MAX_STALENESS as a string ([cfe1e85](https://github.com/sqlstack-io/sqlquack/commit/cfe1e85b00222b956414d07b1a80505cac051126))
* **syntax:** structure a lone Snowflake table contact and read SNAPSHOT POLICY before SET as a snapshot name ([721eb8f](https://github.com/sqlstack-io/sqlquack/commit/721eb8ffb3815cbce7b53a5f7f3620cbddd27b19))

## [0.12.0](https://github.com/sqlstack-io/sqlquack/compare/v0.11.3...v0.12.0) (2026-11-13)


### Features

* **syntax:** parse Snowflake EXECUTE IMMEDIATE into a structured node ([5cfcbe0](https://github.com/sqlstack-io/sqlquack/commit/5cfcbe0f053fb914c41e9c81c99c94bc95c7e581))


### Bug Fixes

* **syntax:** check a CREATE VARIABLE's type, default and comment ([66f09a0](https://github.com/sqlstack-io/sqlquack/commit/66f09a09390ca6dc60c878c3de7292cdbcf25694))
* **syntax:** check DESCRIBE FUNCTION TABLE arguments ([f556c0f](https://github.com/sqlstack-io/sqlquack/commit/f556c0fc4c368bbcb9185fb2f7ef2b1d1c24a99f))
* **syntax:** check warehouse table lists, ABORT, unnamed actions and connector PUSH ([f4ec2d3](https://github.com/sqlstack-io/sqlquack/commit/f4ec2d3ca597cb8ed7edcccc4a30db1d7130ba5f))
* **syntax:** follow the recorded RESULTSET, notebook name and SHOW SERVICES verdicts ([f0799e7](https://github.com/sqlstack-io/sqlquack/commit/f0799e716d8700e5c86c50d4b3e3a75d406d6546))
* **syntax:** read a lone APPLICATION, CLASS or grantee kind in Snowflake SHOW as a name ([1c7b1bf](https://github.com/sqlstack-io/sqlquack/commit/1c7b1bffe6854e7b6361adb1b8cb50bfe4095f0b))
* **syntax:** read a notebook named PROJECT with options and an unnamed ALTER WAREHOUSE IF EXISTS ([908ad5b](https://github.com/sqlstack-io/sqlquack/commit/908ad5b33c5d4f306c3df36503a8c487e5b2a818))
* **syntax:** read SHOW WITH PRIVILEGES as documented privileges ([8565bf4](https://github.com/sqlstack-io/sqlquack/commit/8565bf4c678097cbf698a6c3e7b9616550dfdaee))
* **syntax:** refuse CONTACT on an event table ([de1fa67](https://github.com/sqlstack-io/sqlquack/commit/de1fa67039eec211627d9cc6d3940067c915de89))

## [0.11.3](https://github.com/sqlstack-io/sqlquack/compare/v0.11.2...v0.11.3) (2026-11-13)


### Bug Fixes

* **syntax-receipts:** place errno 921 parse errors as syntax errors ([e770e16](https://github.com/sqlstack-io/sqlquack/commit/e770e16beac6afe81defd78e9399c49c444c8dfe))
* **syntax-receipts:** place errors that show Snowflake read an object statement as semantic ([ecce335](https://github.com/sqlstack-io/sqlquack/commit/ecce335b9df74428d744bc25ea78799e7ad66ef5))
* **syntax:** accept documented EXECUTE IMMEDIATE sources and keep DEFINE in DEFINE text ([4938fa7](https://github.com/sqlstack-io/sqlquack/commit/4938fa7b51f1e13c79f7f6345c4ee329ba28a741))
* **syntax:** check external agents, connector versions, root task lists and organization user groups ([55b02e6](https://github.com/sqlstack-io/sqlquack/commit/55b02e67f20e515d6a31d34dc7b92357236daba5))
* **syntax:** check SHOW SERVICES, service snapshots and DEFINE FILE FORMAT modifiers ([50f569d](https://github.com/sqlstack-io/sqlquack/commit/50f569d8e0bf0042f74ded8033b89f539163d179))
* **syntax:** keep INCREMENT = to Snowflake and read SET INCREMENT as an integer ([f2d1e81](https://github.com/sqlstack-io/sqlquack/commit/f2d1e81880d6c66c18d0d1d997b4910f88ee1c95))
* **syntax:** need a user name to rename, enroll MFA, or set a user policy or tag ([a2386f9](https://github.com/sqlstack-io/sqlquack/commit/a2386f97d6abf18d571e4597e21fe1ec5a3677e9))
* **syntax:** read ALTER DYNAMIC TABLE actions the way Snowflake records them ([6a6e004](https://github.com/sqlstack-io/sqlquack/commit/6a6e004333ebad90e6b94f3879892d7e7781d881))
* **syntax:** read ALTER SEQUENCE SET and the smallest sequence START ([5516f15](https://github.com/sqlstack-io/sqlquack/commit/5516f15c124b08cf9301aaed0ab6b3c5b0bb17da))
* **syntax:** read CREATE and ALTER POSTGRES INSTANCE as Snowflake does ([94cc0c8](https://github.com/sqlstack-io/sqlquack/commit/94cc0c8f59868dc3d8ab2850e2046c1725335694))
* **syntax:** read dynamic table clones, SET and event routing rules as the probe batch recorded ([5e87a78](https://github.com/sqlstack-io/sqlquack/commit/5e87a78bb61cb5d0e7d1e972b00a5c074e8b7707))
* **syntax:** read external consumers, AI functions, EXECUTE IMMEDIATE and DEFINE of tags, pipes and code bundles as Snowflake does ([81d9549](https://github.com/sqlstack-io/sqlquack/commit/81d954981779ad4567309fb7a0c62407d4121a86))
* **syntax:** read notebook projects, organization profiles and group name properties as Snowflake does ([6559625](https://github.com/sqlstack-io/sqlquack/commit/65596259af720e99f3531535b94346cd07c87074))
* **syntax:** read Snowflake GRANT, REVOKE and SHOW GRANTS as Snowflake does ([9e31fde](https://github.com/sqlstack-io/sqlquack/commit/9e31fde5144880b820abb7f2ed8dcbcbb9d708ef))
* **syntax:** read the object statement shapes the probe batch settled ([7d2a329](https://github.com/sqlstack-io/sqlquack/commit/7d2a329a10b0357c690068a57e61867a78b34f1f))

## [0.11.2](https://github.com/sqlstack-io/sqlquack/compare/v0.11.1...v0.11.2) (2026-11-13)


### Bug Fixes

* **syntax:** accept DEFINE DATABASE and SCHEMA with properties ([f8193fd](https://github.com/sqlstack-io/sqlquack/commit/f8193fd548594162300c580d23374ad4a96a14d0))
* **syntax:** read CREATE and ALTER DATA MOVEMENT RULE as Snowflake does ([71aaa30](https://github.com/sqlstack-io/sqlquack/commit/71aaa3075142e03a910bb16d3dcbe2daaa2c608c))
* **syntax:** read CREATE and DEFINE ALERT as Snowflake does ([16d0f5e](https://github.com/sqlstack-io/sqlquack/commit/16d0f5e694478d7e8e63357112fcb75b71a7d353))
* **syntax:** read CREATE and DEFINE STREAMLIT as Snowflake does ([02610d6](https://github.com/sqlstack-io/sqlquack/commit/02610d6756ecc608cbe6aacfa9b0a4524148b4c3))
* **syntax:** read CREATE BACKUP SET, SNAPSHOT SET and APPLICATION as Snowflake does ([3457610](https://github.com/sqlstack-io/sqlquack/commit/34576100b797b61a30bd6f78178625e1cb2ab650))
* **syntax:** read CREATE CORTEX SEARCH SERVICE as Snowflake does ([9f5f666](https://github.com/sqlstack-io/sqlquack/commit/9f5f666e5bcda9d1908b9f6f1e11e58e86149b9e))
* **syntax:** read CREATE EXTERNAL and ORGANIZATION LISTING as Snowflake does ([cf25d86](https://github.com/sqlstack-io/sqlquack/commit/cf25d864c2f1575e6d1449478a245707d4139515))
* **syntax:** read CREATE SERVICE and APPLICATION SERVICE as Snowflake does ([d8725e4](https://github.com/sqlstack-io/sqlquack/commit/d8725e40f090d9c508921671b8d136f615ac5069))
* **syntax:** read Snowflake procedure modes, handler words and scoped tables ([e4b5543](https://github.com/sqlstack-io/sqlquack/commit/e4b55437511e463d5c0eb636386913c373e1ac77))

## [0.11.1](https://github.com/sqlstack-io/sqlquack/compare/v0.11.0...v0.11.1) (2026-11-13)


### Bug Fixes

* **lint:** read a FIRST or LAST join alias as the relation's alias ([f009b82](https://github.com/sqlstack-io/sqlquack/commit/f009b828ab1ad8209186bd8ec5da8f0d6ec84b7e))
* **syntax:** accept an online feature table without TARGET_LAG or with IF NOT EXISTS ([98e46f8](https://github.com/sqlstack-io/sqlquack/commit/98e46f86dba50ceecaac286ef829e02f9ae1db62))
* **syntax:** read ALTER and DROP MODEL monitor as a model named or qualified by monitor ([8fe9485](https://github.com/sqlstack-io/sqlquack/commit/8fe9485dcd1dd0a4c10c3f0ca23dd10e87af4a46))
* **syntax:** record a SEMANTIC_VIEW that writes DIMENSIONS before METRICS or FACTS ([8343a82](https://github.com/sqlstack-io/sqlquack/commit/8343a82f5dd324084270562f92a47ed39d77a35d))
* **syntax:** record a stage path as a stage reference, not a string ([75399f2](https://github.com/sqlstack-io/sqlquack/commit/75399f21ce7e53f85e785b23f11864f7ab3969b1))

## [0.11.0](https://github.com/sqlstack-io/sqlquack/compare/v0.10.0...v0.11.0) (2026-11-12)


### Features

* **syntax:** give nine Snowflake property-list CREATE kinds structured trees ([351d2c2](https://github.com/sqlstack-io/sqlquack/commit/351d2c2dee619fb333cb45852bb2f887123725d4))


### Bug Fixes

* **syntax:** accept a COPY FILES stage written as a string literal ([1e89bad](https://github.com/sqlstack-io/sqlquack/commit/1e89bad985c0baa6d59af9a428a0af9771e67936))
* **syntax:** end compute pool and artifact repository CREATEs at their tag list, and name RSS ([dee1656](https://github.com/sqlstack-io/sqlquack/commit/dee1656a64822c8ef38c91153e00d2b601a2e832))
* **syntax:** keep a parenthesized property list flat ([a8db36d](https://github.com/sqlstack-io/sqlquack/commit/a8db36d0297e1a52cbb77cb54816d879e5b61a32))
* **syntax:** keep FOR APPLICATION to compute pools, read CREATE MODEL monitor as a model, and refuse a job service option given on both sides of FROM ([b2a8bc5](https://github.com/sqlstack-io/sqlquack/commit/b2a8bc5eb289f8eecbb0456b925cc18cede0da5e))
* **syntax:** read a bare FIRST or LAST as a join alias, refuse FIRST ( x ) after an aliased table, and take job service options before FROM ([925f2f7](https://github.com/sqlstack-io/sqlquack/commit/925f2f77185694782d470f05a2d94a03c77d9598))
* **syntax:** record AS on a table alias only when the SQL writes it ([6c2692b](https://github.com/sqlstack-io/sqlquack/commit/6c2692b000423d74ecf61d13895f460da7373593))

## [0.10.0](https://github.com/sqlstack-io/sqlquack/compare/v0.9.0...v0.10.0) (2026-11-12)


### Features

* **syntax:** check the head and option lists of Snowflake statements kept as text ([48f2895](https://github.com/sqlstack-io/sqlquack/commit/48f28959f3557753a065be8dd189eeb730dbd47e))


### Bug Fixes

* **syntax:** name DELETE USING subqueries, semantic views and FIRST/LAST join aliases as what they are ([506cc2e](https://github.com/sqlstack-io/sqlquack/commit/506cc2e7c54c828c2658743fe4e08bd3376b8f41))

## [0.9.0](https://github.com/sqlstack-io/sqlquack/compare/v0.8.0...v0.9.0) (2026-11-12)


### Features

* **syntax:** check Snowflake class instance commands against their grammar ([8ace8c0](https://github.com/sqlstack-io/sqlquack/commit/8ace8c06dbe307d3d912608319f5ba346a92c7c5))
* **syntax:** parse EXECUTE DCM PROJECT plan, deploy and preview ([23aae47](https://github.com/sqlstack-io/sqlquack/commit/23aae474a136ebdb6925c93a89691f4e8e10a201))


### Bug Fixes

* **syntax:** keep DCM names and anonymous procedure return nullability in the tree ([24c23aa](https://github.com/sqlstack-io/sqlquack/commit/24c23aa4a0d13794fa8c924b3f997084ba0886ab))
* **syntax:** refuse a null-input clause on a Snowflake handler-language anonymous procedure ([4e6a694](https://github.com/sqlstack-io/sqlquack/commit/4e6a69440317be5fd501b8af84d593440dff850b))

## [0.8.0](https://github.com/sqlstack-io/sqlquack/compare/v0.7.0...v0.8.0) (2026-11-12)


### Features

* **syntax:** build Snowflake semantic views as nodes ([45d2ebb](https://github.com/sqlstack-io/sqlquack/commit/45d2ebb54a69f9333dbf03fa49ed7c42a44d5937))
* **syntax:** check Snowflake ALTER VIEW properties against their grammar ([ad4c296](https://github.com/sqlstack-io/sqlquack/commit/ad4c296887137827db360364535f4f3823bdaa72))
* **syntax:** check Snowflake streams against their grammar ([2837449](https://github.com/sqlstack-io/sqlquack/commit/2837449b3f2979604a54a20b11ce6d5216cdcade))
* **syntax:** check Snowflake views and materialized views against their grammar ([bd58df1](https://github.com/sqlstack-io/sqlquack/commit/bd58df1d34aabc04fbac7c306479f67a0302ef1a))


### Bug Fixes

* **semantics:** give every select item its range through each dialect's strict parser ([8e7bfdc](https://github.com/sqlstack-io/sqlquack/commit/8e7bfdc157a119feab49b15437e0415710aa966a))
* **syntax:** read a Snowflake view's column list after its properties ([344755e](https://github.com/sqlstack-io/sqlquack/commit/344755e758f52cc5ad8cd3af4a6aa95a0d62b498))
* **syntax:** read every Snowflake stream option on every source ([4bd716c](https://github.com/sqlstack-io/sqlquack/commit/4bd716c195599cad928fe559b0f73754c90e20b4))
* **syntax:** refuse Snowflake view shapes its receipts reject ([6b23d65](https://github.com/sqlstack-io/sqlquack/commit/6b23d6570ff7c8ce273ce3296a15dab83f050096))

## [0.7.0](https://github.com/sqlstack-io/sqlquack/compare/v0.6.0...v0.7.0) (2026-11-12)


### Features

* **convert:** run FULL JOIN USING after a JOIN without ON left to right ([1cbc3bd](https://github.com/sqlstack-io/sqlquack/commit/1cbc3bde743bc1bcb7f9f9daafc5b6e44678d293))


### Bug Fixes

* **syntax-cases:** render external table options and DCM environments as documented ([dc5dd46](https://github.com/sqlstack-io/sqlquack/commit/dc5dd46df2b85854a89c16c27e5a9d1c32188d5a))

## [0.6.0](https://github.com/sqlstack-io/sqlquack/compare/v0.5.1...v0.6.0) (2026-11-12)


### Features

* **convert:** convert TOP and FETCH FIRST to LIMIT ([b593f7e](https://github.com/sqlstack-io/sqlquack/commit/b593f7e12da474961e85cbf24941d6832ca11eaa))
* **convert:** lay out DuckDB output and helper macros with the formatter ([be7425b](https://github.com/sqlstack-io/sqlquack/commit/be7425bf45ea9a23c6b4e51bcf9a6999fe0e1be2))
* **convert:** run JOIN without ON as a cross join and name more unaliased columns ([8f02569](https://github.com/sqlstack-io/sqlquack/commit/8f025691b82e42d83ec395c0c6cfe25c18b5335f))
* **format:** break wide window specifications and keep one-line clauses on their header ([61e7a4e](https://github.com/sqlstack-io/sqlquack/commit/61e7a4e49e9f44f2381579b72adf2a3df4e1f233))
* **format:** give parenthesised groups that span lines their own lines ([c8e14ba](https://github.com/sqlstack-io/sqlquack/commit/c8e14ba9dfc40d6848d3f7fa4566d66b058607ff))
* **format:** keep token case on request and lay out converted DuckDB SQL ([4637fde](https://github.com/sqlstack-io/sqlquack/commit/4637fdeec6f8fb77d8f8323643f816f929a6eecf))
* **semantics:** give each output column its expression's byte range ([f09d2e9](https://github.com/sqlstack-io/sqlquack/commit/f09d2e9027c161771bf744f3f1aee8a5ace4a1cc))
* **syntax:** accept Snowflake's scoped and volatile temporary tables ([8d5ba02](https://github.com/sqlstack-io/sqlquack/commit/8d5ba02a0b57911af27bb41e74eddb9d0e432beb))
* **syntax:** build Snowflake dynamic iceberg and interactive tables as nodes ([ad40a47](https://github.com/sqlstack-io/sqlquack/commit/ad40a47873dda561493ab5f3a0d8c087deea3257))
* **syntax:** build Snowflake table and view governance actions as nodes ([b98b9a0](https://github.com/sqlstack-io/sqlquack/commit/b98b9a0c98617c1b691356aedaea987a88c76e9d))
* **syntax:** build Snowflake tables with policies as structured nodes ([168431e](https://github.com/sqlstack-io/sqlquack/commit/168431e272e673a6397140b6be1d7d4502d65538))
* **syntax:** check Snowflake DCM table definitions against the table grammar ([d88fa2f](https://github.com/sqlstack-io/sqlquack/commit/d88fa2f55e9fdd7efd056b6552225b9b23697dca))
* **syntax:** check Snowflake event, event routing and online feature tables ([2e21de6](https://github.com/sqlstack-io/sqlquack/commit/2e21de6171ce763638cbcda8292061258da4cb89))
* **syntax:** check Snowflake hybrid table indexes ([38ae49e](https://github.com/sqlstack-io/sqlquack/commit/38ae49e3532c5dad7a6e391a524998b4a3be990c))
* **syntax:** check Snowflake iceberg and dynamic tables against their grammar ([498be3a](https://github.com/sqlstack-io/sqlquack/commit/498be3acd47f2fd1361567b88a05176bfa9e9c15))
* **syntax:** check Snowflake table search optimization targets ([5ac49ea](https://github.com/sqlstack-io/sqlquack/commit/5ac49eafc5b99d9882839a3d0911bcadc7756345))
* **syntax:** parse Snowflake external tables into structured nodes ([39207dd](https://github.com/sqlstack-io/sqlquack/commit/39207dd93c0483f0889d27d2d3e44ace200d8797))
* **syntax:** parse Snowflake iceberg column type and write default actions ([b2725c8](https://github.com/sqlstack-io/sqlquack/commit/b2725c8b2e96f6c71ddb078740dd20eabade6edb))
* **syntax:** parse Snowflake iceberg partition evolution ([1872de0](https://github.com/sqlstack-io/sqlquack/commit/1872de03bd426a3881b8f9fe3ca12ff89f306f64))
* **syntax:** parse Snowflake table reclustering and iceberg refresh ([7d42d8b](https://github.com/sqlstack-io/sqlquack/commit/7d42d8b0e88f4406b910db37fae20d0a3346926b))
* **syntax:** parse Snowflake's key and unnamed constraint drops and constraint renames ([066cf8d](https://github.com/sqlstack-io/sqlquack/commit/066cf8d85ee6be57f75c623ed904d82c807895e2))
* **syntax:** read several Snowflake table properties after one SET ([d3523c7](https://github.com/sqlstack-io/sqlquack/commit/d3523c7343bd74429d88f817eb551e30959f3a9d))
* **syntax:** read Snowflake ALTER TABLE ADD with the table grammar ([5939f22](https://github.com/sqlstack-io/sqlquack/commit/5939f22862a04d69a12bfdaa60757b95a39647f1))


### Bug Fixes

* **check:** refuse == as Snowflake does, with a new SQ0206 ([86e200b](https://github.com/sqlstack-io/sqlquack/commit/86e200b05f8769ae352a4c77e970f97c762705f2))
* **diagnostics:** keep node names and guessed aliases out of the unaliased column message ([cd64a37](https://github.com/sqlstack-io/sqlquack/commit/cd64a37109271ac06a842cc426ea190a8b89cced))
* **format:** keep a sign touching a modulo, which DuckDB reads as one operator ([50734ca](https://github.com/sqlstack-io/sqlquack/commit/50734ca952e0864bc16b81492372438312b32557))
* **format:** keep the extra indent of a tighter operator under a looser one ([bab166c](https://github.com/sqlstack-io/sqlquack/commit/bab166c50a5b714bb485f051b500f9a96cec752b))
* **semantics:** leave every set operation's outputs without a range ([23c1099](https://github.com/sqlstack-io/sqlquack/commit/23c10991e3994b0f612f5828664f58b0ddf06580))
* **syntax-coverage:** fail on a not-submitted entry whose case has a receipt ([cde63cd](https://github.com/sqlstack-io/sqlquack/commit/cde63cdf46fbcdaace287b6be8f208649e7c01f5))
* **syntax-receipts:** count a run-database packet's repeated template once ([ee638f5](https://github.com/sqlstack-io/sqlquack/commit/ee638f5a8f02ada8f4634293ffe13879447f764d))
* **syntax-receipts:** document the phase rules in order and keep timeouts refused ([c3a5599](https://github.com/sqlstack-io/sqlquack/commit/c3a5599d6aeb63a0b5dafba320cccb8eca81c9cd))
* **syntax-receipts:** import execute runs that have no run database ([01ace82](https://github.com/sqlstack-io/sqlquack/commit/01ace826bc50e5d04b5ab967cac3d74a106fa59b))
* **syntax-receipts:** place replication not enabled for the account as semantic ([e55255a](https://github.com/sqlstack-io/sqlquack/commit/e55255a01ce4c0b1c700244d33dec7a343cdd6ce))
* **syntax-receipts:** place reviewed errors whose message shows Snowflake parsed the statement ([bcc7178](https://github.com/sqlstack-io/sqlquack/commit/bcc71789d71f4c6a964efe674879ffe3d2b7daec))
* **syntax-receipts:** refuse a record whose connector failed before Snowflake answered ([9b67a8b](https://github.com/sqlstack-io/sqlquack/commit/9b67a8b2b7baf92896fb32fb2b039c5fe4596b19))
* **syntax-receipts:** refuse a run-database packet carrying another run's template ([722d8b8](https://github.com/sqlstack-io/sqlquack/commit/722d8b88862d75b5e43b9c46863b6acdb2479482))
* **syntax:** keep the parentheses of a set operation that starts a longer one ([20078f8](https://github.com/sqlstack-io/sqlquack/commit/20078f89f22f7e9dacb31ecff31b86501b5780b2))
* **syntax:** keep the parse in parse_strict so the instruction guard counts it ([4f5cdfa](https://github.com/sqlstack-io/sqlquack/commit/4f5cdfa75f77b224316daaf88009ce9dbce6b25f))
* **syntax:** keep the source range of Snowflake table column names ([451032f](https://github.com/sqlstack-io/sqlquack/commit/451032f051834263da892d1234b2b625ad47537d))
* **syntax:** name what is missing when a statement ends where an expression belongs ([b802ca9](https://github.com/sqlstack-io/sqlquack/commit/b802ca9c327ce3aebe6ba8d39c54e3d1b0a38009))
* **syntax:** parse keyword lambda parameters and nested parenthesised set operations ([9e2b9b6](https://github.com/sqlstack-io/sqlquack/commit/9e2b9b6ee95541ce7f0da29c31cf7c5ca1b9c996))
* **syntax:** parse Snowflake timestamp conversions with no arguments ([369de78](https://github.com/sqlstack-io/sqlquack/commit/369de78e41bf5757c5b04ecc5f43112f4242583c))
* **syntax:** point a dangling operator before GROUP BY or ORDER BY at the clause ([9d34f79](https://github.com/sqlstack-io/sqlquack/commit/9d34f7988293bfa224463f7cce4e64a666e26188))
* **syntax:** read every secondary role of a Snowflake dynamic table ([b18355d](https://github.com/sqlstack-io/sqlquack/commit/b18355d94d6fb460de7d9424b7ce896726783d08))
* **syntax:** record a star's source range in a located parse ([d91f489](https://github.com/sqlstack-io/sqlquack/commit/d91f4892347fe0eca8e953c913c79a3788308406))
* **syntax:** refuse misplaced Snowflake table actions ([fb4e866](https://github.com/sqlstack-io/sqlquack/commit/fb4e8668aefcf201ab7e836d04cc03c1ff423d9a))
* **syntax:** require a value for every Snowflake session setting ([32ddec1](https://github.com/sqlstack-io/sqlquack/commit/32ddec120cd45fe0b7d8dee9cf9bf2500e2004de))


### Performance Improvements

* **semantics:** map output ranges in ASCII SQL without scanning it ([8a2ea92](https://github.com/sqlstack-io/sqlquack/commit/8a2ea92ee9976d262ea52ee9f815ef3ee9c59e34))

## [0.5.1](https://github.com/sqlstack-io/sqlquack/compare/v0.5.0...v0.5.1) (2026-11-11)


### Bug Fixes

* **lint:** stop panicking on a query that ends in FROM ([4ae2e6a](https://github.com/sqlstack-io/sqlquack/commit/4ae2e6a2b8c21059251be6802f53f7f3cb489b29))
* **semantics:** accept LAG and LEAD with one or two arguments in DuckDB ([6ba17ca](https://github.com/sqlstack-io/sqlquack/commit/6ba17cab252488aa3543e1a9c334d961b5db7624))
* **semantics:** leave a DuckDB TIMESTAMP(p) cast untyped unless p is 6 ([4f7b865](https://github.com/sqlstack-io/sqlquack/commit/4f7b865ef3211ce2e620a06d191f8c844ae34ac2))
* **semantics:** let a derived table inside a LATERAL subquery read enclosing items ([f9bcb57](https://github.com/sqlstack-io/sqlquack/commit/f9bcb5750fa6541344820b52d0dd0ecbdca8ac74))
* **semantics:** let a FROM item read only the items before it, and accept a DuckDB derived table without LATERAL that reads an earlier FROM item as implicitly lateral, as DuckDB runs it ([aad9f8f](https://github.com/sqlstack-io/sqlquack/commit/aad9f8fb5ea6b15b30059f381786fda04f634465))
* **semantics:** stop reading a lambda parameter as an ungrouped column, so lambdas in grouped queries are no longer rejected ([226a3a8](https://github.com/sqlstack-io/sqlquack/commit/226a3a839637b68a3080dd6edf4f756e5a742d5d))
* **semantics:** type DuckDB JSON casts, to_json, AT TIME ZONE and TO_TIMESTAMP ([e509581](https://github.com/sqlstack-io/sqlquack/commit/e509581f562c88a8d6c53830979dce9301fd45fe))

## [0.5.0](https://github.com/sqlstack-io/sqlquack/compare/v0.4.1...v0.5.0) (2026-11-10)


### Features

* **verify:** reuse the judgements of unchanged cases and judge cases in parallel ([3a0068f](https://github.com/sqlstack-io/sqlquack/commit/3a0068f5ea0b90174c7fb6d08b006de3a4057a26))


### Bug Fixes

* **lineage:** give a merged column its strongest branch's transform kind ([0a86a2d](https://github.com/sqlstack-io/sqlquack/commit/0a86a2d4765ecfbf76783ba50110dc807068b467))

## [0.4.1](https://github.com/sqlstack-io/sqlquack/compare/v0.4.0...v0.4.1) (2026-11-10)


### Performance Improvements

* **convert:** divide wide decimals through one helper macro ([3d7ce28](https://github.com/sqlstack-io/sqlquack/commit/3d7ce28f80f9374b550d2f9fcfcd10d7bb8392ad))

## [0.4.0](https://github.com/sqlstack-io/sqlquack/compare/v0.3.0...v0.4.0) (2026-11-10)


### ⚠ BREAKING CHANGES

* **config:** split the user sqlquack.toml from the evidence policy

### Features

* **config:** split the user sqlquack.toml from the evidence policy ([017327d](https://github.com/sqlstack-io/sqlquack/commit/017327d3a465892c1dd03640adaf0492c767ac96))


### Bug Fixes

* **config:** keep refusing a fact under --strict when [allow] refuses it ([6fc9ba1](https://github.com/sqlstack-io/sqlquack/commit/6fc9ba19704f6180e93595af1f73d72fba8b769b))


### Performance Improvements

* **syntax:** cap the token list a tokenize run reserves up front ([f9f37b3](https://github.com/sqlstack-io/sqlquack/commit/f9f37b37acfd6e62f07b2643f214a850772f7389))

## [0.3.0](https://github.com/sqlstack-io/sqlquack/compare/v0.2.0...v0.3.0) (2026-11-10)


### ⚠ BREAKING CHANGES

* **census:** --names is removed. Use --detailed for a named report and --anonymous for a report without names.
* **census:** the previous anonymous counts report moves behind --detailed. --names and --assume-supported now require --detailed, and the slowest-files list appears only in detailed mode. Census now exits 1 when any file is blocked.

### Features

* **census:** add --anonymous for a shareable counts-only report ([f94b4d9](https://github.com/sqlstack-io/sqlquack/commit/f94b4d971f4cf63c731031929a0eaeb4b813670a))
* **census:** name the unknown relation in a name-resolution cause ([c587e72](https://github.com/sqlstack-io/sqlquack/commit/c587e72c464cbeb7403600c4e60253e84c6d2b9c))
* **census:** report a diagnostic for each blocked file ([fbadd64](https://github.com/sqlstack-io/sqlquack/commit/fbadd645020ed10f1cd8d9046447990f4ae37e15))
* **cli:** explain known differences and list them or every code ([9d62bf2](https://github.com/sqlstack-io/sqlquack/commit/9d62bf2224730e18db14b0911b909d96292122d1))
* **cli:** plain-English diagnostics and a one-line summary of known differences ([4758180](https://github.com/sqlstack-io/sqlquack/commit/47581809f3b59a7ee325378765187c6567fe2562))
* **cli:** quote the policy file's reason in --verbose known-difference warnings ([c1ed980](https://github.com/sqlstack-io/sqlquack/commit/c1ed980c27ba78ab8eb5dfcbcf048b9a029d26d8))
* **policy:** describe all 38 known differences in plain English ([101b811](https://github.com/sqlstack-io/sqlquack/commit/101b8111820aace201818c161db1106c250d1f5a))
* **policy:** describe known differences in plain English ([c93f4ee](https://github.com/sqlstack-io/sqlquack/commit/c93f4eec8eaaf98b8912d809ca9312efdfdbbb70))
* **sqlserver:** read FOR SYSTEM_TIME as the table's version, its bounds a constant or a variable as SQL Server requires ([0b68995](https://github.com/sqlstack-io/sqlquack/commit/0b68995104fc730dead340578810c52fc3bdfe8c))
* **sqlserver:** read FOR XML, FOR JSON and FOR BROWSE, and lay out FOR XML and FOR JSON as SQLClean does ([aa6da9b](https://github.com/sqlstack-io/sqlquack/commit/aa6da9ba9e07326bb474bc66cda743cf16c72796))
* **sqlserver:** read join hints, REMOTE only after INNER, and keep them in the join keyword when laying out ([fd8b0eb](https://github.com/sqlstack-io/sqlquack/commit/fd8b0eb118922ae1b759e36c7e2ce45db9d13765))
* **sqlserver:** read table hints, with SQL Server's conflict rules and the hints a write target refuses ([c24aa5d](https://github.com/sqlstack-io/sqlquack/commit/c24aa5d2be03c3f0a071492ac6db47fb7a65bf6e))
* **sqlserver:** read table variables as table sources and write targets, without the hints and samples SQL Server refuses them ([2c93771](https://github.com/sqlstack-io/sqlquack/commit/2c93771ea64d81daab371da242090cd26e16c31f))
* **sqlserver:** read TABLESAMPLE on a table name, after its alias and before its hints, as SQL Server does ([c6bbd9c](https://github.com/sqlstack-io/sqlquack/commit/c6bbd9cfa0d4ddcbe6c9ea58e7a86ecda75620ef))


### Bug Fixes

* **census:** keep model chaining opt-in ([8c25d57](https://github.com/sqlstack-io/sqlquack/commit/8c25d57dd9da846f3f335dfcf8fa73bfb14961d5))
* **cli:** mark differences the defaults allow only in part ([eab0a1d](https://github.com/sqlstack-io/sqlquack/commit/eab0a1d3bbbd5ef208e000a07c58c5f485490ced))
* **cli:** say known difference in census, review and verify output ([df82e77](https://github.com/sqlstack-io/sqlquack/commit/df82e77f2c3ed0f1bb62aaf7746c4c01a1f2ee2b))
* **convert:** name the project's sqlquack.toml by its name, not its folder's path ([affd49f](https://github.com/sqlstack-io/sqlquack/commit/affd49fe29b59442edbc6d059e466075079d7c40))
* **coverage:** refuse cited translation cases whose path has an empty, . or .. part ([4dfb5fc](https://github.com/sqlstack-io/sqlquack/commit/4dfb5fc972f1eca7c2628b602ed36cb00d664c6a))
* **diagnostics:** say what must be true of the session, not how to change the setting ([64166cc](https://github.com/sqlstack-io/sqlquack/commit/64166cc438cc549c54ef07f14be5f635dc74713c))
* **diagnostics:** show a known difference's fine and wrong cases as whole sentences ([e87b620](https://github.com/sqlstack-io/sqlquack/commit/e87b620ccfecb5074eac58b0566751a0a1b03100))
* **distributions:** compile DuckDB's ICU extension into every bundled build ([5b4ee0f](https://github.com/sqlstack-io/sqlquack/commit/5b4ee0fa3305011bf52de152627fccaa5bd2b3ab))
* **distributions:** compile DuckDB's json extension into every bundled build ([267e0b9](https://github.com/sqlstack-io/sqlquack/commit/267e0b97036d81da35698c27844d4cf11a125751))
* **distributions:** keep DuckDB's compiled-in extensions in Linux links ([f1cbdb6](https://github.com/sqlstack-io/sqlquack/commit/f1cbdb6e7a236c8cd18581606b6fcaacd9bd57a2))
* **distributions:** name DuckDB's release version in builds from its sources ([cae4c1c](https://github.com/sqlstack-io/sqlquack/commit/cae4c1cb49087d8a8b149e790f5dd9513b22f3da))


### Performance Improvements

* cast a widened number literal to its wider DECIMAL only once ([cfaa28d](https://github.com/sqlstack-io/sqlquack/commit/cfaa28d7bd6c4a0dfe83896998b8eccf89bde3af))
* **semantics:** analyse a statement's lineage and findings in one pass ([4da19b0](https://github.com/sqlstack-io/sqlquack/commit/4da19b07639dd58eb249786ad469ca24e8e03fc5))
* **semantics:** cheaper per-level findings bookkeeping ([5349692](https://github.com/sqlstack-io/sqlquack/commit/5349692072ef9e43861f2b8c761132cbcfe418a2))
* **semantics:** read a level's output aliases without copying them per projection ([6be2847](https://github.com/sqlstack-io/sqlquack/commit/6be2847cb7d0e3babcfcfac19906545823808fac))
* **semantics:** read each Snowflake type spelling once per thread ([7013ca5](https://github.com/sqlstack-io/sqlquack/commit/7013ca5d8154eecc26d84f1fd953746b8f34c9cd))
* **semantics:** reuse a level's FROM items between resolution and findings ([31c42e8](https://github.com/sqlstack-io/sqlquack/commit/31c42e80954e99aa2088461a2fe61daffbb399c7))
* **semantics:** share a level's alias-free traces between resolution and findings ([87786d3](https://github.com/sqlstack-io/sqlquack/commit/87786d3340acdeb52224ecb92d19e7faa9018db1))
* **semantics:** share a resolved query's column list instead of copying it ([6e0f593](https://github.com/sqlstack-io/sqlquack/commit/6e0f593194df8a938a879fb7a59f52048c9eaf09))
* **syntax:** box the star expression so every expression moves fewer bytes ([c786e78](https://github.com/sqlstack-io/sqlquack/commit/c786e78d8ddfac730bd20c7645a0f13579ab31d8))
* **syntax:** check strict coverage on the parser's own tokens ([73a6fff](https://github.com/sqlstack-io/sqlquack/commit/73a6fffe8dc9821c0c5b80fc7f28e32688f42608))
* **syntax:** let dialect parsers inline token text reads ([fe9a0ee](https://github.com/sqlstack-io/sqlquack/commit/fe9a0eeade936bd34229aa40d0125277ad15f72c))
* **syntax:** list children without tracking their paths ([6c054a0](https://github.com/sqlstack-io/sqlquack/commit/6c054a0be3a62fea24de1d8772e8ee77a16169fe))
* **syntax:** look Snowflake reserved words up instead of scanning the list ([503ecee](https://github.com/sqlstack-io/sqlquack/commit/503ecee7e056316c9b1c84129b4c387a162f0bb2))
* **syntax:** read ASCII single-character tokens and identifier quotes from a table ([754664d](https://github.com/sqlstack-io/sqlquack/commit/754664d594e273c75991a48900082eaed52523d7))
* **syntax:** reserve the token list once per tokenize run ([2096c69](https://github.com/sqlstack-io/sqlquack/commit/2096c698a967af5f10659224c09d8eb5a101b907))
