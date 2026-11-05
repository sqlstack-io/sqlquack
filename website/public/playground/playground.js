// The SQLQuack playground: each edit runs the engine (SQLQuack compiled to WebAssembly, in a
// worker) and shows its stages; translated SQL runs in DuckDB-WASM.

import { escape, rendered, underlined } from './spans.js';

// The engine's worker; `engine(op, fields)` resolves with its reply.
const worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });
const pending = new Map();
let nextId = 0;
const engineReady = new Promise((resolve) => {
	worker.addEventListener('message', ({ data }) => {
		if (data.ready) return resolve();
		pending.get(data.id)?.(data);
		pending.delete(data.id);
	});
});
function engine(op, fields) {
	const id = ++nextId;
	return new Promise((resolve) => {
		pending.set(id, resolve);
		worker.postMessage({ id, op, ...fields });
	});
}


const SCHEMA = `{
  "ORDERS": [
    ["ORDER_ID", "NUMBER(38,0)"],
    ["CUSTOMER_ID", "NUMBER(38,0)"],
    ["STATUS", "VARCHAR"],
    ["TOTAL", "NUMBER(12,2)"],
    ["ORDERED_AT", "TIMESTAMP_NTZ"]
  ]
}`;

const ROWS = `INSERT INTO "ORDERS" VALUES
  (1, 10, 'shipped', 120.50, TIMESTAMP '2026-10-01 09:30:00'),
  (2, 10, 'shipped', 80.00, TIMESTAMP '2026-10-03 14:00:00'),
  (3, 11, 'pending', 45.25, TIMESTAMP '2026-10-04 11:15:00'),
  (4, 12, 'shipped', 310.00, TIMESTAMP '2026-10-05 16:45:00'),
  (5, 12, NULL, 12.99, TIMESTAMP '2026-10-06 08:05:00');`;

const EXAMPLES = {
	valid: `SELECT customer_id,
       SUM(total) AS revenue,
       IFF(COUNT(*) > 1, 'repeat', 'new') AS segment
FROM orders
WHERE status = 'shipped'
GROUP BY customer_id
ORDER BY revenue DESC`,
	decimal: `SELECT order_id, total / 3 AS third
FROM orders
ORDER BY order_id`,
	syntax: `SELECT customer_id, SUM(total) AS revenue
FROM orders
WHERE status = 'shipped' AND
GROUP BY customer_id`,
	rejected: `SELECT customer_id, SUM(total) AS revenue
FROM orders
GROUP BY customer_id
QUALIFY revenue > 100`,
	unknown: `SELECT order_id, discount
FROM orders`,
	refused: `SELECT order_id,
       HASH(customer_id) AS bucket
FROM orders`,
};

const $ = (selector) => document.querySelector(selector);
const sqlBox = $('#pg-sql');
const schemaBox = $('#pg-schema');
const rowsBox = $('#pg-rows');
const policyBox = $('#pg-policy');
const sessionBox = $('#pg-session');
const strictBox = $('#pg-strict');
const utcBox = $('#pg-utc');
const settings = () =>
	JSON.stringify({ policy: policyBox.value, session: sessionBox.value, strict: strictBox.checked, utc_session: utcBox.checked });
const marks = $('#pg-marks');
const tip = $('#pg-tip');

function setStage(id, status, body, sub) {
	const stage = $(`#stage-${id}`);
	const pill = stage.querySelector('.pg-status');
	pill.className = `pg-status ${status}`;
	pill.textContent = status;
	stage.querySelector('.pg-body').innerHTML = body ?? '';
	if (sub !== undefined) stage.querySelector('.pg-sub').textContent = sub;
}

const terminal = (html) => `<pre class="terminal">${html}</pre>`;

// Underlines every span in the editor; overlapping spans keep the first.
function underline(chars, items) {
	const { html, items: kept } = underlined(chars, items);
	marks.innerHTML = html;
	marks.items = kept;
	marks.scrollTop = sqlBox.scrollTop;
}

let latest = null;
let current = 0;

async function run() {
	const ticket = ++current;
	const sql = sqlBox.value;
	const reply = await engine('analyze', { sql, schema: schemaBox.value, settings: settings() });
	// A newer edit has already been sent; its reply will render instead.
	if (ticket !== current) return;
	const chars = Array.from(sql);
	const report = JSON.parse(reply.result);
	latest = null;
	if (report.schema_error || report.settings_error) {
		underline(chars, []);
		const what = report.schema_error ? 'The schema is not valid' : 'The policy or session is not valid';
		setStage('parse', 'skipped', terminal(`<span class="bad">${what}:</span> ${escape(report.schema_error ?? report.settings_error)}`));
		for (const id of ['checks', 'translate', 'run']) setStage(id, 'skipped');
		return;
	}
	if (report.empty) {
		underline(chars, []);
		for (const id of ['parse', 'checks', 'translate', 'run']) setStage(id, 'skipped');
		return;
	}
	const spans = [];
	const add = (diagnostics, kind) => {
		for (const diagnostic of diagnostics ?? []) spans.push({ kind, span: diagnostic.span, text: rendered(diagnostic, chars) });
	};

	const parse = report.parse;
	add(parse.diagnostics, 'error');
	setStage(
		'parse',
		parse.status,
		parse.diagnostics ? parse.diagnostics.map((d) => terminal(rendered(d, chars))).join('') : '',
		parse.statements ? `${parse.statements} statement${parse.statements === 1 ? '' : 's'}` : '',
	);

	const checks = report.checks;
	add(checks.diagnostics, 'error');
	const findings = (checks.findings ?? []).map((finding) => {
		const text = rendered({ code: finding.code, message: finding.message, span: finding.span }, chars).replace('class="bad"', 'class="warn"');
		spans.push({ kind: 'warn', span: finding.span, text });
		return terminal(text);
	});
	setStage(
		'checks',
		checks.status,
		checks.status === 'skipped' ? '' : (checks.diagnostics ?? []).map((d) => terminal(rendered(d, chars))).join('') + findings.join(''),
		checks.status === 'skipped' ? '' : checks.names_checked ? 'rejections, names and scopes' : 'rejections',
	);

	const translate = report.translate;
	add(translate.diagnostics, 'error');
	let body = '';
	if (translate.status !== 'skipped') {
		body = (translate.statements ?? []).map((s) => terminal(escape(s.sql))).join('');
		body += (translate.diagnostics ?? []).map((d) => terminal(rendered(d, chars))).join('');
	}
	setStage('translate', translate.status, body, translate.profile ? `policy: ${translate.profile}` : '');
	underline(chars, spans);

	const statements = translate.statements ?? [];
	if (translate.status === 'pass' && statements.length === 1) {
		latest = statements[0].sql;
		if (duckdb) execute();
		else setStage('run', 'ready', '<button class="button primary" type="button" id="pg-run">Run in DuckDB-WASM</button>', 'loads DuckDB-WASM (about 8 MB compressed)');
	} else if (translate.status === 'pass') {
		setStage('run', 'skipped', terminal('<span class="dim">Runs one statement at a time.</span>'));
	} else setStage('run', 'skipped', '');
}

// DuckDB-WASM, loaded on first use.
let duckdb = null;
let running = 0;

async function loadDuckDB() {
	// Served from this site, in a directory named for its version (scripts/prepare_playground.mjs).
	const { DUCKDB_DIR } = await import('./duckdb-version.js');
	const DUCKDB_WASM = new URL(DUCKDB_DIR, import.meta.url);
	const module = await import(new URL('duckdb-browser.mjs', DUCKDB_WASM).href);
	const worker = new Worker(new URL('duckdb-browser-base.worker.js', DUCKDB_WASM));
	const db = new module.AsyncDuckDB(new module.VoidLogger(), worker);
	await db.instantiate(new URL('duckdb-base.wasm', DUCKDB_WASM).href);
	const version = await (await db.connect()).query('SELECT version() AS v');
	return { db, version: String(version.toArray()[0].v) };
}

async function execute() {
	const sql = latest;
	if (!sql) return;
	const ticket = ++running;
	setStage('run', 'running', '');
	try {
		if (!duckdb) duckdb = await loadDuckDB();
		const conn = await duckdb.db.connect();
		try {
			const tables = JSON.parse((await engine('fixture', { schema: schemaBox.value })).result);
			if (tables.error) throw new Error(`schema: ${tables.error}`);
			for (const table of tables.tables) await conn.query(table.replace(/^CREATE TABLE/, 'CREATE OR REPLACE TABLE'));
			if (rowsBox.value.trim()) await conn.query(rowsBox.value);
			const started = performance.now();
			// Every value as DuckDB's own text, so decimals and timestamps show exactly.
			const result = await conn.query(`SELECT COLUMNS(*)::VARCHAR FROM (${sql}) AS result`);
			const elapsed = performance.now() - started;
			if (ticket !== running) return;
			const names = result.schema.fields.map((field) => field.name);
			const rows = result.toArray().map((row) => row.toJSON());
			const table = `<div class="pg-rows"><table><thead><tr>${names.map((n) => `<th>${escape(n)}</th>`).join('')}</tr></thead><tbody>${rows
				.map((row) => `<tr>${names.map((n) => `<td>${row[n] === null ? '<span class="dim">NULL</span>' : escape(String(row[n]))}</td>`).join('')}</tr>`)
				.join('')}</tbody></table></div>`;
			setStage('run', 'pass', table, `${rows.length} row${rows.length === 1 ? '' : 's'} · ${elapsed.toFixed(0)} ms · DuckDB ${duckdb.version}`);
		} finally {
			await conn.close();
		}
	} catch (error) {
		if (ticket !== running) return;
		setStage('run', 'fail', terminal(`<span class="bad">DuckDB:</span> ${escape(String(error.message ?? error))}`), duckdb ? `DuckDB ${duckdb.version}` : '');
	}
}

let timer = 0;
const schedule = () => {
	clearTimeout(timer);
	timer = setTimeout(run, 120);
};

document.addEventListener('click', (event) => {
	if (event.target.id === 'pg-run') execute();
});
sqlBox.addEventListener('input', schedule);
schemaBox.addEventListener('input', schedule);
for (const box of [policyBox, sessionBox, strictBox, utcBox]) box.addEventListener('input', schedule);
$('#pg-load-policy').addEventListener('click', async () => {
	policyBox.value = await (await fetch(new URL('./sqlquack.toml', import.meta.url))).text();
	schedule();
});
rowsBox.addEventListener('input', () => latest && duckdb && schedule());
sqlBox.addEventListener('scroll', () => (marks.scrollTop = sqlBox.scrollTop));
$('#pg-example').addEventListener('change', (event) => {
	if (!EXAMPLES[event.target.value]) return;
	sqlBox.value = EXAMPLES[event.target.value];
	event.target.value = '';
	run();
});

// The underline layer sits under the textarea, so hovering is tracked on the textarea itself.
sqlBox.addEventListener('mousemove', (event) => {
	const hit = [...marks.querySelectorAll('mark')].find((mark) =>
		[...mark.getClientRects()].some((r) => event.clientX >= r.left && event.clientX <= r.right && event.clientY >= r.top && event.clientY <= r.bottom),
	);
	if (!hit) {
		tip.hidden = true;
		return;
	}
	tip.innerHTML = marks.items[Number(hit.dataset.i)].text;
	tip.hidden = false;
	const x = Math.min(event.clientX + 14, innerWidth - tip.offsetWidth - 8);
	tip.style.left = `${Math.max(8, x)}px`;
	tip.style.top = `${event.clientY + 18}px`;
});
sqlBox.addEventListener('mouseleave', () => (tip.hidden = true));

const params = new URLSearchParams(location.search);
sqlBox.value = params.get('sql') ?? EXAMPLES[params.get('example')] ?? EXAMPLES.valid;
schemaBox.value = SCHEMA;
rowsBox.value = ROWS;
setStage('parse', 'loading', '', 'loading the engine');
await engineReady;
await run();
if (params.has('run')) execute();
