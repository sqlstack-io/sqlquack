// The playground engine and its span handling, under Node's test runner: `npm run test:engine`.
// Needs the engine built into wasm/pkg (scripts/build_engine.sh).
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { test } from 'node:test';

import { initSync, analyze, fixture_tables, run_failure } from '../wasm/pkg/sqlquack_wasm.js';
import { rendered, resultTable, underlined } from '../public/playground/spans.js';

initSync({ module: readFileSync(new URL('../wasm/pkg/sqlquack_wasm_bg.wasm', import.meta.url)) });

const SCHEMA = JSON.stringify({ ORDERS: [['ORDER_ID', 'NUMBER(38,0)'], ['NOTE', 'VARCHAR']] });

const marked = (html) => [...html.matchAll(/<mark[^>]*>(.*?)<\/mark>/g)].map((match) => match[1]);

test('spans after non-ASCII text underline the characters the diagnostic names', () => {
	// 'é' is one UTF-16 unit, '😀' two and 'ß' one: three characters but four units before `nope`.
	const sql = "SELECT 'é😀ß' AS note, nope FROM orders";
	const report = JSON.parse(analyze(sql, SCHEMA));
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
	const report = JSON.parse(analyze(sql, SCHEMA));
	assert.equal(report.parse.status, 'fail');
	const [failure] = report.parse.diagnostics;
	assert.equal(failure.code, 'SQ0001');
	assert.equal(Array.from(sql).slice(failure.span.start, failure.span.end).join(''), 'WHERE');
	assert.match(rendered(failure, Array.from(sql)), /--> query\.sql:3:1\n/);
	assert.deepEqual(marked(underlined(Array.from(sql), [{ kind: 'error', span: failure.span }]).html), ['WHERE']);
});

test('each stage reports pass, fail or skipped', () => {
	const stages = (sql) => {
		const report = JSON.parse(analyze(sql, SCHEMA));
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
		const report = JSON.parse(analyze('SELECT order_id FROM orders', SCHEMA, JSON.stringify(settings)));
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
	const report = JSON.parse(analyze('SELECT order_id FROM orders', SCHEMA, JSON.stringify({ policy: tight })));
	assert.equal(report.translate.status, 'fail');
	assert.equal(report.translate.diagnostics[0].code, 'SQ0006');
	assert.match(report.translate.diagnostics[0].label, /beyond the budget of 0 ms/);
});

test('every example chip stops at the stage its label names, the Run stage included', async () => {
	const { EXAMPLE_GROUPS, EXAMPLES, CAPTIONS, ROWS, SCHEMA: PAGE_SCHEMA } = await import('../public/playground/examples.js');
	const chips = EXAMPLE_GROUPS.flatMap((group) => group.examples);
	assert.deepEqual(chips.map((chip) => chip.key).sort(), Object.keys(EXAMPLES).sort());
	assert.equal(new Set(chips.map((chip) => chip.slug)).size, chips.length);
	// Translated examples run as the page runs them: the page's fixture tables and rows, in the
	// DuckDB-WASM build the page serves, with every value read as text.
	const require = createRequire(import.meta.url);
	const duckdb = require('@duckdb/duckdb-wasm/dist/duckdb-node-blocking.cjs');
	const dist = dirname(require.resolve('@duckdb/duckdb-wasm/dist/duckdb-node-blocking.cjs'));
	const bundles = { base: { mainModule: join(dist, 'duckdb-base.wasm'), mainWorker: join(dist, 'duckdb-node-base.worker.cjs') } };
	const db = await duckdb.createDuckDB(bundles, new duckdb.VoidLogger(), duckdb.NODE_RUNTIME);
	await db.instantiate();
	const conn = db.connect();
	for (const table of JSON.parse(fixture_tables(PAGE_SCHEMA)).tables) conn.query(table);
	conn.query(ROWS);
	const stops = (sql) => {
		const report = JSON.parse(analyze(sql, PAGE_SCHEMA));
		const failed = ['parse', 'checks', 'translate'].find((stage) => report[stage].status !== 'pass');
		if (failed) return failed;
		try {
			conn.query(`SELECT COLUMNS(*)::VARCHAR FROM (${report.translate.statements[0].sql}) AS result`);
			return null;
		} catch {
			return 'run';
		}
	};
	for (const chip of chips) assert.equal(stops(EXAMPLES[chip.key]), chip.stops, chip.name);
	// The one deliberate refusal is a known divergence, explained by its caption.
	const refused = JSON.parse(analyze(EXAMPLES.refused, PAGE_SCHEMA)).translate;
	assert.deepEqual(refused.diagnostics.map((d) => d.code), ['SQ0008']);
	assert.ok(CAPTIONS.refused);
});

test('translated SQL is laid out one clause per line unless the compact setting asks for one line', () => {
	const sql = "SELECT order_id, note, 'a longer literal' AS label FROM orders WHERE order_id > 1";
	const translated = (settings) => JSON.parse(analyze(sql, SCHEMA, JSON.stringify(settings))).translate.statements[0].sql;
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
	const { SCHEMA: PAGE_SCHEMA } = await import('../public/playground/examples.js');
	// The page reads the relation's name from this label to offer "Add T to schema".
	const unknown = JSON.parse(analyze('SELECT * FROM t', PAGE_SCHEMA)).translate;
	assert.equal(unknown.status, 'fail');
	assert.deepEqual(
		unknown.diagnostics.map((d) => [d.code, d.label]),
		[['SQ0004', 'relation `T` has no known columns']],
	);
	// The stub's placeholder type is not a Snowflake type: no column is assumed, so it still refuses.
	const stub = JSON.stringify({ ...JSON.parse(PAGE_SCHEMA), T: [['COLUMN_NAME', 'SNOWFLAKE_TYPE']] });
	assert.deepEqual(JSON.parse(analyze('SELECT * FROM t', stub)).translate.diagnostics.map((d) => d.code), ['SQ0004']);
	// Every relation of the sample schema without semi-structured columns translates as declared.
	for (const relation of ['ORDERS', 'CUSTOMERS', 'PRODUCTS', 'ORDER_ITEMS']) {
		assert.equal(JSON.parse(analyze(`SELECT * FROM ${relation.toLowerCase()}`, PAGE_SCHEMA)).translate.status, 'pass', relation);
	}
});

test('each known divergence a translation relies on is reported with its constructs and the policy reason', () => {
	const repository = readFileSync(new URL('../../sqlquack.toml', import.meta.url), 'utf8');
	const policy = repository.replace('constructs = ["fn:HASH"]\ndefault = "refuse"', 'constructs = ["fn:HASH"]\ndefault = "warn"');
	assert.notEqual(policy, repository);
	const translate = (settings) => JSON.parse(analyze('SELECT HASH(order_id) AS h FROM orders', SCHEMA, JSON.stringify(settings))).translate;
	const [statement] = translate({ policy }).statements;
	assert.deepEqual(
		statement.divergences.map((d) => [d.id, d.constructs]),
		[['hash', ['fn:HASH']]],
	);
	assert.match(statement.divergences[0].reason, /never Snowflake's value/);
	assert.equal(statement.divergences[0].assumption, false);
	// An assumption gives a wrong value silently where it does not hold, so it is reported as one.
	const today = JSON.parse(analyze('SELECT CURRENT_DATE AS d', SCHEMA, JSON.stringify({ policy: repository, utc_session: true })));
	const assumed = today.translate.statements[0].divergences.find((d) => d.id === 'session_timezone_utc');
	assert.equal(assumed?.assumption, true);
	assert.ok(assumed.reason);
	// Without a divergence, none is reported.
	assert.deepEqual(JSON.parse(analyze('SELECT order_id FROM orders', SCHEMA)).translate.statements[0].divergences, []);
});

test('a run error a building block raised reads back as the Snowflake error, at the expression that raised it', () => {
	const sql = 'SELECT CAST(note AS NUMBER(10,2)) AS v FROM orders';
	const [statement] = JSON.parse(analyze(sql, SCHEMA)).translate.statements;
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
	const [statement] = JSON.parse(analyze(sql, SCHEMA, JSON.stringify({ policy }))).translate.statements;
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
