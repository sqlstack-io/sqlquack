# Changelog

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
