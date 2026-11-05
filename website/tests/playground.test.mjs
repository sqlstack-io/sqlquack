// The playground engine and its span handling, under Node's test runner: `npm run test:engine`.
// Needs the engine built into public/playground/engine (scripts/build_engine.sh).
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { initSync, analyze } from '../public/playground/engine/sqlquack_wasm.js';
import { rendered, underlined } from '../public/playground/spans.js';

initSync({ module: readFileSync(new URL('../public/playground/engine/sqlquack_wasm_bg.wasm', import.meta.url)) });

const SCHEMA = JSON.stringify({ ORDERS: [['ORDER_ID', 'NUMBER(38,0)'], ['NOTE', 'VARCHAR']] });

const marked = (html) => [...html.matchAll(/<mark[^>]*>(.*?)<\/mark>/g)].map((match) => match[1]);

test('spans after non-ASCII text underline the characters the diagnostic names', () => {
	// 'é' is one UTF-16 unit, '😀' two and 'ß' one: three characters but four units before `nope`.
	const sql = "SELECT 'é😀ß' AS note, nope FROM orders";
	const report = JSON.parse(analyze(sql, SCHEMA));
	const [finding] = report.checks.findings;
	assert.equal(finding.code, 'E201');
	const chars = Array.from(sql);
	assert.equal(chars.slice(finding.span.start, finding.span.end).join(''), 'nope');
	// Slicing the string by UTF-16 units would land one unit early, on ` nop`.
	assert.notEqual(sql.slice(finding.span.start, finding.span.end), 'nope');
	const { html } = underlined(chars, [{ kind: 'error', span: finding.span }]);
	assert.deepEqual(marked(html), ['nope']);
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
	// SQL Snowflake rejects is not translated, though no rule refuses it: DuckDB's bind would.
	assert.deepEqual(stages('SELECT order_id, note FROM orders GROUP BY note'), ['pass', 'fail', 'skipped']);
	assert.deepEqual(stages('SELECT COUNT(*) AS n FROM orders WHERE n > 1'), ['pass', 'fail', 'skipped']);
});

test('translated SQL comes laid out over lines, with only whitespace changed', () => {
	const report = JSON.parse(analyze('SELECT order_id, note FROM orders WHERE order_id > 1', SCHEMA));
	const [statement] = report.translate.statements;
	assert.match(statement.pretty, /^SELECT\n {4}.+,\n {4}.+\nFROM .+\nWHERE /);
	assert.equal(statement.pretty.replace(/\s+/g, ''), statement.sql.replace(/\s+/g, ''));
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
	// The built-in default records no profiles, so any session parameter is refused.
	assert.deepEqual(translate({ session: 'TIMEZONE=UTC' }).diagnostics.map((d) => d.code), ['SQ0007']);
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

test('every example stops at the stage it is meant to, and the default translates cleanly', async () => {
	const { EXAMPLES, CAPTIONS, SCHEMA: PAGE_SCHEMA } = await import('../public/playground/examples.js');
	const expected = {
		valid: ['pass', 'pass', 'pass'],
		decimal: ['pass', 'pass', 'pass'],
		syntax: ['fail', 'skipped', 'skipped'],
		rejected: ['pass', 'fail', 'skipped'],
		unknown: ['pass', 'fail', 'skipped'],
		refused: ['pass', 'pass', 'fail'],
	};
	assert.deepEqual(Object.keys(EXAMPLES).sort(), Object.keys(expected).sort());
	for (const [name, sql] of Object.entries(EXAMPLES)) {
		const report = JSON.parse(analyze(sql, PAGE_SCHEMA));
		assert.deepEqual(['parse', 'checks', 'translate'].map((stage) => report[stage].status), expected[name], name);
	}
	// The one deliberate refusal is a known divergence, explained by its caption.
	const refused = JSON.parse(analyze(EXAMPLES.refused, PAGE_SCHEMA)).translate;
	assert.deepEqual(refused.diagnostics.map((d) => d.code), ['SQ0008']);
	assert.ok(CAPTIONS.refused);
});
