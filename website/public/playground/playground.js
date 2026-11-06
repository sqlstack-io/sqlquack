// The SQLQuack playground: each edit runs the engine (SQLQuack compiled to WebAssembly, in a
// worker) and shows its stages; translated SQL runs in DuckDB-WASM, which downloads in the
// background once the page is idle.

import { DUCKDB_DIR, DUCKDB_WASM_BYTES } from './duckdb-version.js';
import { ENGINE_DIR, ENGINE_WASM_BYTES } from './engine-version.js';
import { CAPTIONS, EXAMPLES, ROWS, SCHEMA } from './examples.js';
import { escape, rendered, underlined } from './spans.js';

const here = (path) => new URL(path, import.meta.url).href;
const ENGINE_WASM = here(`${ENGINE_DIR}sqlquack_wasm_bg.wasm`);
const DUCKDB_FILES = ['duckdb-browser.mjs', 'duckdb-browser-base.worker.js', 'duckdb-base.wasm'].map((file) => here(DUCKDB_DIR + file));

// The engine and DuckDB live in directories named for their content or version, so a URL never
// changes meaning: the page keeps them in Cache Storage, and a second visit loads them without the
// network whatever the host's cache headers (GitHub Pages sends ten minutes). Earlier versions'
// files are dropped.
const cache = (self.caches ? caches.open('sqlquack-playground') : Promise.reject()).catch(() => null);
cache.then(async (store) => {
	for (const request of (await store?.keys()) ?? []) if (![ENGINE_WASM, ...DUCKDB_FILES].includes(request.url)) store.delete(request);
});

// `url`'s response, from the cache or the network, reporting its decoded bytes as they arrive.
async function cached(url, onProgress) {
	const store = await cache;
	let response = await store?.match(url).catch(() => undefined);
	if (!response) {
		response = await fetch(url);
		if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
		store?.put(url, response.clone()).catch(() => {});
	}
	if (!onProgress) return response;
	let received = 0;
	const counted = response.body.pipeThrough(
		new TransformStream({
			transform(chunk, controller) {
				received += chunk.byteLength;
				onProgress(received);
				controller.enqueue(chunk);
			},
		}),
	);
	return new Response(counted, { headers: response.headers });
}

// The engine's worker; `engine(op, fields)` resolves with its reply. The page compiles the wasm
// and hands the worker the module, so the download starts with this script, not after the worker
// and its imports load.
const worker = new Worker(here('./worker.js'), { type: 'module' });
const pending = new Map();
let nextId = 0;
const engineReady = new Promise((resolve) => {
	worker.addEventListener('message', ({ data }) => {
		if (data.ready) {
			performance.mark('engine-ready');
			return resolve();
		}
		pending.get(data.id)?.(data);
		pending.delete(data.id);
	});
});
const percent = (received, total) => Math.min(99, Math.floor((received / total) * 100));
cached(ENGINE_WASM, (received) => progress('parse', `loading the engine… ${percent(received, ENGINE_WASM_BYTES)}%`))
	.then((response) => {
		const copy = response.clone();
		// A host that serves .wasm with another type rules out streaming compilation.
		return WebAssembly.compileStreaming(response).catch(async () => WebAssembly.compile(await copy.arrayBuffer()));
	})
	.then((module) => worker.postMessage({ module }))
	.catch((error) => setStage('parse', 'fail', terminal(`<span class="bad">The engine did not load:</span> ${escape(String(error.message ?? error))}`), ''));
function engine(op, fields) {
	const id = ++nextId;
	return new Promise((resolve) => {
		pending.set(id, resolve);
		worker.postMessage({ id, op, ...fields });
	});
}

// Updates a loading stage's subtitle, at most once a frame.
const progressText = {};
function progress(id, text) {
	if (progressText[id] === undefined) {
		requestAnimationFrame(() => {
			const stage = $(`#stage-${id}`);
			if (stage.querySelector('.pg-status').classList.contains('loading')) stage.querySelector('.pg-sub').textContent = progressText[id];
			if (id === 'run') renderDuckDBProgress();
			delete progressText[id];
		});
	}
	progressText[id] = text;
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
let latestRaises = {};
// The editor's current text and underlines, so a run's error can add its own.
let shown = { chars: [], spans: [] };
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
		// Translated SQL keeps its line layout and scrolls sideways; diagnostics still wrap.
		body = (translate.statements ?? []).map((s) => `<pre class="terminal sql">${escape(s.sql)}</pre>`).join('');
		body = unknownRelationHints(translate.diagnostics) + body + conversionNotes(translate);
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
	shown = { chars, spans };

	const statements = translate.statements ?? [];
	if (translate.status === 'pass' && statements.length === 1) {
		latest = statements[0].sql;
		latestRaises = statements[0].raises ?? {};
		if (duckdb) execute();
		else showDuckDBLoading();
	} else if (translate.status === 'pass') {
		setStage('run', 'skipped', terminal('<span class="dim">Runs one statement at a time.</span>'));
	} else setStage('run', 'skipped', '', refused ? 'nothing was translated' : '');
}

// What the translation relies on that a reader should know: each known divergence the policy
// allows (as `sqlquack convert` prints "relies on known divergence …"), and any other notes.
function conversionNotes(translate) {
	const notes = [...(translate.notes ?? [])].map((note) => escape(note));
	for (const statement of translate.statements ?? [])
		for (const d of statement.divergences ?? []) {
			const at = d.constructs.map((construct) => `<code>${escape(construct)}</code>`).join(', ');
			notes.push(`Relies on known divergence <code>${escape(d.id)}</code> at ${at}${d.reason ? `: ${escape(d.reason)}` : ''}`);
		}
	return [...new Set(notes)].map((note) => `<p class="pg-conversion-note"><strong>Note.</strong> ${note}</p>`).join('');
}

// A relation the schema does not declare: SQLQuack needs its columns and Snowflake types, and
// does not guess them. Each one gets a line saying so, with a button that adds a stub to fill in.
const UNKNOWN_RELATION = /^relation `(.+)` has no known columns$/;
const STUB_COLUMN = ['COLUMN_NAME', 'SNOWFLAKE_TYPE'];

function declaredRelations() {
	try {
		return Object.keys(JSON.parse(schemaBox.value));
	} catch {
		return [];
	}
}

function unknownRelationHints(diagnostics) {
	const relations = new Set();
	for (const d of diagnostics ?? []) {
		const match = d.code === 'SQ0004' && UNKNOWN_RELATION.exec(d.label ?? '');
		if (match) relations.add(match[1]);
	}
	const declared = declaredRelations();
	return [...relations]
		.map((name) => {
			const code = `<code>${escape(name)}</code>`;
			return declared.includes(name)
				? `<p class="pg-hint"><span>SQLQuack needs each table's columns and Snowflake types. Fill in ${code}'s in the Schema box.</span></p>`
				: `<p class="pg-hint"><span>SQLQuack needs each table's columns and Snowflake types. Add ${code} to the Schema box.</span>` +
						`<button class="button" type="button" data-add-relation="${escape(name)}">Add ${escape(name)} to schema</button></p>`;
		})
		.join('');
}

// Adds `name` with one placeholder column to the end of the schema's JSON, keeping the rest of the
// text as typed, and selects the placeholder so typing replaces it.
function addRelation(name) {
	const text = schemaBox.value;
	const close = text.lastIndexOf('}');
	if (close < 0 || declaredRelations().includes(name)) return;
	const before = text.slice(0, close).trimEnd();
	const column = `[${STUB_COLUMN.map((part) => JSON.stringify(part)).join(', ')}]`;
	const entry = `  ${JSON.stringify(name)}: [\n    ${column}\n  ]`;
	const separator = before.endsWith('{') ? '\n' : ',\n';
	schemaBox.value = `${before}${separator}${entry}\n${text.slice(close)}`;
	schemaBox.closest('details').open = true;
	const placeholder = `${before}${separator}  ${JSON.stringify(name)}: [\n    ["`.length;
	schemaBox.focus();
	schemaBox.setSelectionRange(placeholder, placeholder + STUB_COLUMN[0].length);
	schedule();
}

// DuckDB-WASM, downloaded in the background once the page is idle (or on the first run).
let duckdb = null;
let duckdbLoad = null;
let duckdbReceived = 0;
let duckdbError = null;
let running = 0;

function startDuckDB() {
	duckdbLoad ??= loadDuckDB().then(
		(loaded) => {
			duckdb = loaded;
			performance.mark('duckdb-ready');
			if (latest) execute();
			return loaded;
		},
		(error) => {
			duckdbError = error;
			duckdbLoad = null;
			if (latest) showDuckDBLoading();
			throw error;
		},
	);
	return duckdbLoad;
}

async function loadDuckDB() {
	duckdbError = null;
	// Each file as a same-origin blob URL, so DuckDB's module, worker and wasm all come from the cache.
	const blob = async (url, type, onProgress) => {
		const contents = await (await cached(url, onProgress)).blob();
		return URL.createObjectURL(contents.slice(0, contents.size, type));
	};
	const [moduleUrl, workerUrl, wasmUrl] = await Promise.all([
		blob(DUCKDB_FILES[0], 'text/javascript'),
		blob(DUCKDB_FILES[1], 'text/javascript'),
		blob(DUCKDB_FILES[2], 'application/wasm', (received) => {
			duckdbReceived = received;
			progress('run', '');
		}),
	]);
	duckdbReceived = DUCKDB_WASM_BYTES;
	progress('run', '');
	const module = await import(moduleUrl);
	const db = new module.AsyncDuckDB(new module.VoidLogger(), new Worker(workerUrl));
	await db.instantiate(wasmUrl);
	const version = await (await db.connect()).query('SELECT version() AS v');
	return { db, version: String(version.toArray()[0].v) };
}

// The run stage while DuckDB downloads: a progress bar, or the error with a retry.
function showDuckDBLoading() {
	if (duckdbError) {
		setStage(
			'run',
			'fail',
			terminal(`<span class="bad">DuckDB did not load:</span> ${escape(String(duckdbError.message ?? duckdbError))}`) +
				'<button class="button" type="button" id="pg-run">Try again</button>',
			'',
		);
		return;
	}
	setStage('run', 'loading', '<div class="pg-progress"><span></span></div>', '', 'loading');
	renderDuckDBProgress();
}

function renderDuckDBProgress() {
	const bar = $('#stage-run .pg-progress span');
	if (!bar) return;
	const done = percent(duckdbReceived, DUCKDB_WASM_BYTES);
	bar.style.width = `${done}%`;
	$('#stage-run .pg-sub').textContent =
		duckdbReceived >= DUCKDB_WASM_BYTES ? 'Starting DuckDB…' : `Loading DuckDB… ${done}%`;
}

async function execute() {
	const sql = latest;
	if (!sql) return;
	const ticket = ++running;
	setStage('run', 'running', '');
	try {
		if (!duckdb) await startDuckDB();
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
		const message = String(error.message ?? error);
		// An error a building block raised as Snowflake would: shown as Snowflake's, where it comes from.
		const failure = JSON.parse((await engine('failure', { message })).result);
		if (ticket !== running) return;
		const version = duckdb ? `DuckDB ${duckdb.version}` : '';
		if (!failure.errno) {
			setStage('run', 'fail', terminal(`<span class="bad">DuckDB:</span> ${escape(message)}`), version);
			return;
		}
		const { chars, spans } = shown;
		const raised = (latestRaises[failure.raise_id] ?? []).map((location) => {
			const diagnostic = { code: String(failure.errno), message: failure.message, span: { start: location.start, end: location.end }, label: 'raised here' };
			return { kind: 'error', span: diagnostic.span, text: rendered(diagnostic, chars) };
		});
		if (raised.length) underline(chars, [...spans, ...raised]);
		const head = `<span class="bad">error ${escape(String(failure.errno))} (${escape(failure.sqlstate)})</span>: ${escape(failure.message)}`;
		setStage(
			'run',
			'fail',
			terminal(raised[0]?.text ?? head) +
				'<p class="pg-conversion-note">Snowflake raises this error for these rows, and so does the translated SQL in DuckDB.</p>',
			version,
			'error',
		);
	}
}

let timer = 0;
const schedule = () => {
	clearTimeout(timer);
	timer = setTimeout(run, 120);
};

document.addEventListener('click', (event) => {
	if (event.target.id === 'pg-run') execute();
	if (event.target.dataset.addRelation) addRelation(event.target.dataset.addRelation);
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
setStage('parse', 'loading', '', 'loading the engine…', 'loading');
await engineReady;
await run();
// DuckDB starts once the page has settled; `run` above already shows its progress.
(self.requestIdleCallback ?? ((callback) => setTimeout(callback, 200)))(() => startDuckDB().catch(() => {}), { timeout: 2000 });
