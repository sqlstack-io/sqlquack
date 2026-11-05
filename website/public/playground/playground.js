// The SQLQuack playground: each edit runs the engine (SQLQuack compiled to WebAssembly, in a
// worker) and shows its stages; translated SQL runs in DuckDB-WASM.

import { CAPTIONS, EXAMPLES, ROWS, SCHEMA } from './examples.js';
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
const caption = $('#pg-caption');
const verdict = $('#pg-verdict');
const marks = $('#pg-marks');
const tip = $('#pg-tip');

// `label` overrides the pill's text, which is otherwise the status.
function setStage(id, status, body, sub, label) {
	const stage = $(`#stage-${id}`);
	const pill = stage.querySelector('.pg-status');
	pill.className = `pg-status ${status}`;
	pill.textContent = label ?? status;
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

function showCaption(example) {
	caption.textContent = CAPTIONS[example] ?? '';
	caption.hidden = !CAPTIONS[example];
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
	verdict.hidden = true;
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
	// Valid SQL that SQLQuack declines to translate is not an error in the SQL: shown in amber, not red.
	const refused = parse.status === 'pass' && checks.status === 'pass' && translate.status === 'fail';
	const tone = (html) => (refused ? html.replace('class="bad"', 'class="warn"') : html);
	for (const diagnostic of translate.diagnostics ?? [])
		spans.push({ kind: refused ? 'warn' : 'error', span: diagnostic.span, text: tone(rendered(diagnostic, chars)) });
	let body = '';
	if (translate.status !== 'skipped') {
		const sql = (translate.statements ?? []).map((s) => s.pretty).join(';\n\n');
		if (sql) body = `<div class="pg-sql">${terminal(escape(sql))}<button class="copy" type="button" data-copy-sql>Copy</button></div>`;
		body += (translate.diagnostics ?? []).map((d) => terminal(tone(rendered(d, chars)))).join('');
	}
	const policy = translate.profile ? `policy: ${translate.profile}` : '';
	if (refused) {
		const reason = translate.diagnostics?.[0]?.message ?? 'no supported conversion';
		verdict.innerHTML = `<strong>Valid Snowflake SQL.</strong> Not translated to DuckDB: ${escape(reason)}`;
		verdict.hidden = false;
		setStage('translate', 'refused', body, policy, 'not translated');
	} else setStage('translate', translate.status, body, policy);
	underline(chars, spans);

	const statements = translate.statements ?? [];
	if (translate.status === 'pass' && statements.length === 1) {
		latest = statements[0].sql;
		if (duckdb) execute();
		else setStage('run', 'ready', '<button class="button primary" type="button" id="pg-run">Run in DuckDB-WASM</button>', 'loads DuckDB-WASM (about 8 MB compressed)');
	} else if (translate.status === 'pass') {
		setStage('run', 'skipped', terminal('<span class="dim">Runs one statement at a time.</span>'));
	} else setStage('run', 'skipped', '', refused ? 'nothing was translated' : '');
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
	const copy = event.target.closest('[data-copy-sql]');
	if (copy) {
		navigator.clipboard.writeText(copy.previousElementSibling.textContent).then(() => {
			copy.textContent = 'Copied';
			setTimeout(() => (copy.textContent = 'Copy'), 1500);
		});
	}
});
sqlBox.addEventListener('input', () => {
	showCaption(null);
	schedule();
});
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
	showCaption(event.target.value);
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
if (!params.get('sql')) showCaption(params.get('example'));
schemaBox.value = SCHEMA;
rowsBox.value = ROWS;
setStage('parse', 'loading', '', 'loading the engine');
await engineReady;
await run();
if (params.has('run')) execute();
