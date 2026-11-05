// The SQLQuack playground: each keystroke runs the engine (SQLQuack compiled to WebAssembly) and
// shows its stages; translated SQL runs in DuckDB-WASM, loaded from jsDelivr on first use.
import init, { analyze, fixture_tables } from './engine/sqlquack_wasm.js';

const DUCKDB_WASM = 'https://cdn.jsdelivr.net/npm/@duckdb/duckdb-wasm@1.32.0/+esm';

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
const marks = $('#pg-marks');
const tip = $('#pg-tip');

const escape = (text) => text.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

// Spans count Unicode characters; JavaScript strings count UTF-16 units, so index by code point.
function position(chars, offset) {
	let line = 1;
	let column = 1;
	for (let i = 0; i < offset && i < chars.length; i++) {
		if (chars[i] === '\n') {
			line += 1;
			column = 1;
		} else column += 1;
	}
	return { line, column };
}

// One diagnostic as the command line renders it, rustc style.
function rendered(diagnostic, chars) {
	const head = `<span class="bad">error[${escape(diagnostic.code)}]</span>: ${escape(diagnostic.message)}`;
	const span = diagnostic.span;
	if (!span) return head + (diagnostic.label ? `\n  = ${escape(diagnostic.label)}` : '');
	const { line, column } = span.line ? span : position(chars, span.start);
	const lines = chars.join('').split('\n');
	const text = lines[line - 1] ?? '';
	const endOnLine = span.end_line && span.end_line !== line ? text.length + 1 : column + Math.max(1, span.end - span.start);
	const width = Math.max(1, endOnLine - column);
	const gutter = ' '.repeat(String(line).length);
	let out = `${head}\n${gutter}--> query.sql:${line}:${column}\n${gutter} |\n${line} | ${escape(text)}\n${gutter} | ${' '.repeat(column - 1)}<span class="bad">${'^'.repeat(width)}</span>`;
	if (diagnostic.label) out += ` ${escape(diagnostic.label)}`;
	for (const note of diagnostic.notes ?? []) out += `\n${gutter} = <span class="dim">note:</span> ${escape(note)}`;
	if (diagnostic.help) out += `\n${gutter} = <span class="dim">help:</span> ${escape(diagnostic.help)}`;
	return out;
}

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
	const sorted = items.filter((item) => item.span).sort((a, b) => a.span.start - b.span.start);
	let html = '';
	let at = 0;
	sorted.forEach((item, index) => {
		const start = Math.min(item.span.start, chars.length);
		let end = Math.min(Math.max(item.span.end, start + 1), chars.length);
		if (start < at) return;
		if (end === start) {
			// A span at the very end, such as a missing token: underline the last character.
			if (start === 0) return;
			html += escape(chars.slice(at, start - 1).join(''));
			html += `<mark data-i="${index}" class="${item.kind}">${escape(chars[start - 1])}</mark>`;
			at = start;
			return;
		}
		html += escape(chars.slice(at, start).join(''));
		html += `<mark data-i="${index}" class="${item.kind}">${escape(chars.slice(start, end).join(''))}</mark>`;
		at = end;
	});
	html += escape(chars.slice(at).join('')) + '\n';
	marks.innerHTML = html;
	marks.items = sorted;
	marks.scrollTop = sqlBox.scrollTop;
}

let latest = null;

function run() {
	const sql = sqlBox.value;
	const chars = Array.from(sql);
	const report = JSON.parse(analyze(sql, schemaBox.value));
	latest = null;
	if (report.schema_error) {
		underline(chars, []);
		setStage('parse', 'skipped', terminal(`<span class="bad">The schema is not valid:</span> ${escape(report.schema_error)}`));
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
	setStage('translate', translate.status, body);
	underline(chars, spans);

	const statements = translate.statements ?? [];
	if (translate.status === 'pass' && statements.length === 1) {
		latest = statements[0].sql;
		if (duckdb) execute();
		else setStage('run', 'ready', '<button class="button primary" type="button" id="pg-run">Run in DuckDB-WASM</button>', 'loads DuckDB from jsDelivr');
	} else if (translate.status === 'pass') {
		setStage('run', 'skipped', terminal('<span class="dim">Runs one statement at a time.</span>'));
	} else setStage('run', 'skipped', '');
}

// DuckDB-WASM, loaded on first use.
let duckdb = null;
let running = 0;

async function loadDuckDB() {
	const module = await import(DUCKDB_WASM);
	const bundle = await module.selectBundle(module.getJsDelivrBundles());
	const workerUrl = URL.createObjectURL(new Blob([`importScripts("${bundle.mainWorker}");`], { type: 'text/javascript' }));
	const db = new module.AsyncDuckDB(new module.VoidLogger(), new Worker(workerUrl));
	await db.instantiate(bundle.mainModule, bundle.pthreadWorker);
	URL.revokeObjectURL(workerUrl);
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
			const tables = JSON.parse(fixture_tables(schemaBox.value));
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
await init();
run();
if (params.has('run')) execute();
