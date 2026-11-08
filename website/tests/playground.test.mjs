// The playground engine and its span handling, under Node's test runner: `npm run test:engine`.
// Needs the engine built into wasm/pkg (scripts/build_engine.sh); the script prepares the playground first.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

import { initSync, analyze, fixture_tables, run_failure } from '../wasm/pkg/sqlquack_wasm.js';
import { previewLimit, rendered, resultTable, underlined } from '../public/playground/spans.js';

initSync({ module: readFileSync(new URL('../wasm/pkg/sqlquack_wasm_bg.wasm', import.meta.url)) });

const SOURCES = 'sources:\n  - name: orders\n    columns:\n      - name: order_id\n        type: NUMBER(38,0)\n      - name: note\n        type: VARCHAR\n';

// A DuckDB as the page starts one: the DuckDB-WASM build it serves, the extensions it loads from the
// site, then the sample sources' tables and rows. A fresh one per case, so no earlier case has
// already loaded an extension a later case needs.
async function pageDuckDB() {
	const { SOURCES: PAGE_SOURCES, ROWS } = await import('../public/playground/examples.js');
	const { DUCKDB_DIR, DUCKDB_EXTENSIONS } = await import('../public/playground/duckdb-version.js');
	const require = createRequire(import.meta.url);
	const duckdb = require('@duckdb/duckdb-wasm/dist/duckdb-node-blocking.cjs');
	const dist = dirname(require.resolve('@duckdb/duckdb-wasm/dist/duckdb-node-blocking.cjs'));
	const bundles = { base: { mainModule: join(dist, 'duckdb-base.wasm'), mainWorker: join(dist, 'duckdb-node-base.worker.cjs') } };
	const db = await duckdb.createDuckDB(bundles, new duckdb.VoidLogger(), duckdb.NODE_RUNTIME);
	await db.instantiate();
	const conn = db.connect();
	conn.query(`SET custom_extension_repository = '${fileURLToPath(new URL(`../public/playground/${DUCKDB_DIR}extensions`, import.meta.url))}'`);
	for (const extension of DUCKDB_EXTENSIONS) conn.query(`LOAD ${extension}`);
	for (const table of JSON.parse(fixture_tables(PAGE_SOURCES)).tables) conn.query(table);
	conn.query(ROWS);
	// Every value as text, as the page reads them.
	return (sql) => conn.query(`SELECT COLUMNS(*)::VARCHAR FROM (${sql}) AS result`).toArray().map((row) => Object.values(row.toJSON()));
}

const marked = (html) => [...html.matchAll(/<mark[^>]*>(.*?)<\/mark>/g)].map((match) => match[1]);

test('spans after non-ASCII text underline the characters the diagnostic names', () => {
	// 'é' is one UTF-16 unit, '😀' two and 'ß' one: three characters but four units before `nope`.
	const sql = "SELECT 'é😀ß' AS note, nope FROM orders";
	const report = JSON.parse(analyze(sql, SOURCES));
	const [refusal] = report.translate.diagnostics;
	assert.equal(refusal.code, 'SQ0004');
	const chars = Array.from(sql);
	assert.equal(chars.slice(refusal.span.start, refusal.span.end).join(''), 'nope');
	// Slicing the string by UTF-16 units would land one unit early, on ` nop`.
	assert.notEqual(sql.slice(refusal.span.start, refusal.span.end), 'nope');
	const { html } = underlined(chars, [{ kind: 'error', span: refusal.span }]);
	assert.deepEqual(marked(html), ['nope']);
	const [finding] = report.checks.findings;
	assert.equal(chars.slice(finding.span.start, finding.span.end).join(''), 'nope');
});

test('a parse failure on a later line renders at its line and column', () => {
	const sql = "SELECT '日本' AS note\nFROM orders\nWHERE";
	const report = JSON.parse(analyze(sql, SOURCES));
	assert.equal(report.parse.status, 'fail');
	const [failure] = report.parse.diagnostics;
	assert.equal(failure.code, 'SQ0001');
	assert.equal(Array.from(sql).slice(failure.span.start, failure.span.end).join(''), 'WHERE');
	assert.match(rendered(failure, Array.from(sql)), /--> query\.sql:3:1\n/);
	assert.deepEqual(marked(underlined(Array.from(sql), [{ kind: 'error', span: failure.span }]).html), ['WHERE']);
});

test('each stage reports pass, fail or skipped', () => {
	const stages = (sql) => {
		const report = JSON.parse(analyze(sql, SOURCES));
		return ['parse', 'checks', 'translate'].map((stage) => report[stage].status);
	};
	assert.deepEqual(stages('SELECT order_id FROM orders'), ['pass', 'pass', 'pass']);
	assert.deepEqual(stages('SELECT order_id FROM orders WHERE'), ['fail', 'skipped', 'skipped']);
	assert.deepEqual(stages('SELECT 1 QUALIFY TRUE'), ['pass', 'fail', 'skipped']);
	assert.deepEqual(stages('SELECT HASH(order_id) AS h FROM orders'), ['pass', 'pass', 'fail']);
});

test('the policy and session parameters choose the profile, or refuse, as on the command line', () => {
	const policy = readFileSync(new URL('../../sqlquack.toml', import.meta.url), 'utf8');
	const translate = (settings) => {
		const report = JSON.parse(analyze('SELECT order_id FROM orders', SOURCES, JSON.stringify(settings)));
		return report.settings_error ? { error: report.settings_error } : report.translate;
	};
	assert.equal(translate({}).profile, 'the built-in default');
	assert.equal(translate({ policy }).profile, 'utc-v1');
	// Only utc-qiic-v1 records quoted identifiers resolving to upper case.
	assert.equal(translate({ policy, session: 'quoted_identifiers_ignore_case = TRUE\n' }).profile, 'utc-qiic-v1');
	// No recorded profile starts the week on Monday: refused with SQ0007, before any conversion.
	const monday = translate({ policy, session: 'WEEK_START=1' });
	assert.equal(monday.status, 'fail');
	assert.deepEqual(monday.diagnostics.map((d) => d.code), ['SQ0007']);
	assert.equal(monday.statements, undefined);
	// The built-in default records no profiles: it takes TIMEZONE and result-neutral parameters only.
	assert.equal(translate({ session: 'TIMEZONE=UTC' }).profile, 'the built-in default');
	assert.deepEqual(translate({ session: 'WEEK_START=1' }).diagnostics.map((d) => d.code), ['SQ0007']);
	assert.match(translate({ policy: 'schema_version = 1\nnot_a_key = 2' }).error, /^policy: /);
	assert.match(translate({ session: 'TIMEZONE' }).error, /NAME=VALUE/);
});

test('the conversion budget is timed in the browser and refuses with SQ0006 when exceeded', () => {
	const policy = readFileSync(new URL('../../sqlquack.toml', import.meta.url), 'utf8');
	// A zero-millisecond budget: any measured conversion time exceeds it. `Instant` would panic here.
	const tight = policy.replace(/\[conversion_budget\][^[]*/, '[conversion_budget]\nmilliseconds = 0\nsize_ratio = 2000\nminimum_bytes = 4194304\n\n');
	const report = JSON.parse(analyze('SELECT order_id FROM orders', SOURCES, JSON.stringify({ policy: tight })));
	assert.equal(report.translate.status, 'fail');
	assert.equal(report.translate.diagnostics[0].code, 'SQ0006');
	assert.match(report.translate.diagnostics[0].label, /beyond the budget of 0 ms/);
});

test('every example chip stops at the stage its label names, the Run stage included', async () => {
	const { EXAMPLE_GROUPS, EXAMPLES, CAPTIONS, SOURCES: PAGE_SOURCES } = await import('../public/playground/examples.js');
	const chips = EXAMPLE_GROUPS.flatMap((group) => group.examples);
	assert.deepEqual(chips.map((chip) => chip.key).sort(), Object.keys(EXAMPLES).sort());
	assert.equal(new Set(chips.map((chip) => chip.slug)).size, chips.length);
	// Translated examples run as the page runs them, each in a fresh DuckDB.
	const stops = async (sql) => {
		const report = JSON.parse(analyze(sql, PAGE_SOURCES));
		const failed = ['parse', 'checks', 'translate'].find((stage) => report[stage].status !== 'pass');
		if (failed) return failed;
		const run = await pageDuckDB();
		try {
			run(report.translate.statements[0].sql);
			return null;
		} catch {
			return 'run';
		}
	};
	for (const chip of chips) assert.equal(await stops(EXAMPLES[chip.key]), chip.stops, chip.name);
	// The one deliberate refusal is a known divergence, explained by its caption.
	const refused = JSON.parse(analyze(EXAMPLES.refused, PAGE_SOURCES)).translate;
	assert.deepEqual(refused.diagnostics.map((d) => d.code), ['SQ0008']);
	assert.ok(CAPTIONS.refused);
});

test('date arithmetic against CURRENT_DATE runs in the in-browser DuckDB, on its first statement', async () => {
	const { SOURCES: PAGE_SOURCES } = await import('../public/playground/examples.js');
	const cases = [
		// Last 30 days: the fixture's orders are older, and stay so.
		['SELECT order_id FROM orders WHERE ordered_at >= DATEADD(day, -30, CURRENT_DATE)', []],
		// Every sample customer signed up more than 30 days ago.
		['SELECT customer_id FROM customers WHERE DATEDIFF(day, signup_date, CURRENT_DATE) > 30 ORDER BY customer_id', [['10'], ['11'], ['12']]],
		// A month later than a month's last day is the next month's last day when it is shorter.
		[
			"SELECT DATEADD(month, 1, DATE '2024-01-31') AS leap, DATEADD(month, -1, DATE '2026-03-31') AS feb FROM orders " +
				'WHERE ordered_at < DATEADD(year, 1, CURRENT_DATE) AND order_id = 1',
			[['2024-02-29', '2026-02-28']],
		],
	];
	for (const [sql, rows] of cases) {
		const report = JSON.parse(analyze(sql, PAGE_SOURCES));
		assert.equal(report.translate.status, 'pass', sql);
		const run = await pageDuckDB();
		assert.deepEqual(run(report.translate.statements[0].sql), rows, sql);
	}
});

test('a run error from the in-browser DuckDB is told apart from an error about the rows', () => {
	// What the preview build of DuckDB-WASM reported for a translated DATEADD that native DuckDB 2.0 runs.
	assert.ok(previewLimit("Binder Error: No function matches the given name and argument types '+(DATE, INTERVAL)'."));
	assert.ok(previewLimit('memory access out of bounds'));
	// Errors about values would come from native DuckDB too.
	assert.ok(!previewLimit("Conversion Error: Could not convert string 'x' to INT32"));
	assert.ok(!previewLimit('Invalid Input Error: Attempting to fetch from an unsuccessful query result'));
});

test('translated SQL is laid out one clause per line unless the compact setting asks for one line', () => {
	const sql = "SELECT order_id, note, 'a longer literal' AS label FROM orders WHERE order_id > 1";
	const translated = (settings) => JSON.parse(analyze(sql, SOURCES, JSON.stringify(settings))).translate.statements[0].sql;
	const laidOut = translated({});
	assert.equal(
		laidOut,
		'SELECT\n    "ORDERS"."ORDER_ID" AS "ORDER_ID",\n    "ORDERS"."NOTE" AS "NOTE",\n    ' +
			'\'a longer literal\' AS "LABEL"\nFROM "ORDERS" AS "ORDERS"\nWHERE ("ORDERS"."ORDER_ID" > CAST(1 AS DECIMAL(1,0)))',
	);
	const compact = translated({ compact: true });
	assert.equal(compact, laidOut.replace(/\n */g, ' '));
	assert.ok(!compact.includes('\n'));
});

test('an undeclared relation is refused by name, and the stub the page adds stays refused until filled in', async () => {
	const { SOURCES: PAGE_SOURCES } = await import('../public/playground/examples.js');
	// The page reads the relation's name from this label to offer "Add T to sources".
	const unknown = JSON.parse(analyze('SELECT * FROM t', PAGE_SOURCES)).translate;
	assert.equal(unknown.status, 'fail');
	assert.deepEqual(
		unknown.diagnostics.map((d) => [d.code, d.label]),
		[['SQ0004', 'relation `T` has no known columns']],
	);
	// The stub's placeholder type is not a Snowflake type: no column is assumed, so it still refuses.
	const stub = `${PAGE_SOURCES}  - name: t\n    columns:\n      - name: column_name\n        type: SNOWFLAKE_TYPE\n`;
	const stubbed = JSON.parse(analyze('SELECT * FROM t', stub));
	assert.deepEqual(stubbed.translate.diagnostics.map((d) => d.code), ['SQ0004']);
	// The page says why the stub is left out, as `sqlquack convert` warns, and analysis goes on.
	assert.deepEqual(stubbed.warnings, [
		'the source t is left out, because its column column_name has no type SQLQuack reads: unknown Snowflake type `SNOWFLAKE_TYPE`',
	]);
	// The other sources still declare beside the one left out.
	assert.equal(JSON.parse(analyze('SELECT * FROM orders', stub)).translate.status, 'pass');
	// Every table of the sample sources without semi-structured columns translates as declared.
	for (const relation of ['ORDERS', 'CUSTOMERS', 'PRODUCTS', 'ORDER_ITEMS']) {
		assert.equal(JSON.parse(analyze(`SELECT * FROM ${relation.toLowerCase()}`, PAGE_SOURCES)).translate.status, 'pass', relation);
	}
});

test('each known divergence a translation relies on is reported with its constructs and the policy reason', () => {
	const repository = readFileSync(new URL('../../sqlquack.toml', import.meta.url), 'utf8');
	const policy = repository.replace('constructs = ["fn:HASH"]\ndefault = "refuse"', 'constructs = ["fn:HASH"]\ndefault = "warn"');
	assert.notEqual(policy, repository);
	const translate = (settings) => JSON.parse(analyze('SELECT HASH(order_id) AS h FROM orders', SOURCES, JSON.stringify(settings))).translate;
	const [statement] = translate({ policy }).statements;
	assert.deepEqual(
		statement.divergences.map((d) => [d.id, d.constructs]),
		[['hash', ['fn:HASH']]],
	);
	assert.match(statement.divergences[0].reason, /never Snowflake's value/);
	assert.equal(statement.divergences[0].assumption, false);
	// An assumption gives a wrong value silently where it does not hold, so it is reported as one.
	const today = JSON.parse(analyze('SELECT CURRENT_DATE AS d', SOURCES, JSON.stringify({ policy: repository, utc_session: true })));
	const assumed = today.translate.statements[0].divergences.find((d) => d.id === 'session_timezone_utc');
	assert.equal(assumed?.assumption, true);
	assert.ok(assumed.reason);
	// Without a divergence, none is reported.
	assert.deepEqual(JSON.parse(analyze('SELECT order_id FROM orders', SOURCES)).translate.statements[0].divergences, []);
});

test('a run error a building block raised reads back as the Snowflake error, at the expression that raised it', () => {
	const sql = 'SELECT CAST(note AS NUMBER(10,2)) AS v FROM orders';
	const [statement] = JSON.parse(analyze(sql, SOURCES)).translate.statements;
	const tag = /sqlquack:100038#[0-9a-f]{12}: [^']*/.exec(statement.sql)[0];
	// As DuckDB reports a failed cast of the raised text.
	const failure = JSON.parse(run_failure(`Conversion Error: Could not convert string '${tag}' to INT32\n\nLINE 1: ...`));
	assert.equal(failure.errno, 100038);
	assert.equal(failure.message, 'Numeric value is not recognized');
	const [location] = statement.raises[failure.raise_id];
	assert.equal(Array.from(sql).slice(location.start, location.end).join(''), 'CAST(note AS NUMBER(10,2))');
	// DuckDB's own errors are not read as Snowflake's.
	assert.deepEqual(JSON.parse(run_failure('Binder Error: Referenced column "x" not found')), {});
});

test('a raised message holding a quote reads back whole, as DuckDB quotes it unescaped', () => {
	const sql = "SELECT TO_DATE(note, 'YYYY-MM-DD') AS d FROM orders";
	// The repository's policy allows the date-text divergence the built-in one refuses.
	const policy = readFileSync(new URL('../../sqlquack.toml', import.meta.url), 'utf8');
	const [statement] = JSON.parse(analyze(sql, SOURCES, JSON.stringify({ policy }))).translate.statements;
	const [, id] = /sqlquack:100097#([0-9a-f]{12}): /.exec(statement.sql);
	const text = "Can't parse the text as date with the format";
	// As DuckDB 2.0.0 reports the failed cast of the raised text.
	const failure = JSON.parse(run_failure(`Conversion Error: Could not convert string 'sqlquack:100097#${id}: ${text}' to INT32\n\nLINE 1: ...`));
	assert.equal(failure.errno, 100097);
	assert.equal(failure.message, text);
	assert.equal(failure.raise_id, id);
	const [location] = statement.raises[id];
	assert.equal(Array.from(sql).slice(location.start, location.end).join(''), "TO_DATE(note, 'YYYY-MM-DD')");
});

test('a result with a repeated column name shows each column under its own name, with its own values', () => {
	// DESCRIBE's names for `SELECT order_id, order_id`, and each column's values by position.
	const { html, rows } = resultTable(['ORDER_ID', 'ORDER_ID'], [['1', '2'], ['1', null]]);
	assert.equal(rows, 2);
	assert.deepEqual([...html.matchAll(/<th>(.*?)<\/th>/g)].map((match) => match[1]), ['ORDER_ID', 'ORDER_ID']);
	const cells = [...html.matchAll(/<tr>(<td>.*?)<\/tr>/g)].map((row) => [...row[1].matchAll(/<td>(.*?)<\/td>/g)].map((cell) => cell[1]));
	assert.deepEqual(cells, [['1', '1'], ['2', '<span class="dim">NULL</span>']]);
});
