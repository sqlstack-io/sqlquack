// The SQLQuack playground: each edit runs the engine (SQLQuack compiled to WebAssembly, in a
// worker) and shows its stages; translated SQL runs in DuckDB-WASM, which downloads in the
// background once the page is idle.

import { DUCKDB_DIR, DUCKDB_EXTENSIONS, DUCKDB_WASM_BYTES } from './duckdb-version.js';
import { ENGINE_DIR, ENGINE_WASM_BYTES } from './engine-version.js';
import { EXAMPLE_GROUPS, EXAMPLES, ROWS, SOURCES, stopLabel } from './examples.js';
import { escape, previewLimit, rendered, resultTable, underlined } from './spans.js';

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
const sourcesBox = $('#pg-sources');
const rowsBox = $('#pg-rows');
const sessionBox = $('#pg-session');
const strictBox = $('#pg-strict');
const zoneBox = $('#pg-zone');
const allowList = $('#pg-allow-list');
const tomlBox = $('#pg-toml');

// Every known difference, with whether the defaults allow it, from the engine; `allowed` holds each
// one the reader has ticked or unticked.
let differences = [];
const allowed = new Map();
const isAllowed = (difference) => allowed.get(difference.name) ?? difference.allowed_by_default;
// The choices that differ from the defaults, each a line under [allow].
const allowChanges = () => differences.filter((difference) => isAllowed(difference) !== difference.allowed_by_default);

// The session as [project.session] holds it: TIMEZONE when it is not UTC, then the other parameters.
const PARAMETER = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*?)\s*$/;
function sessionParameters() {
	const zone = zoneBox.value.trim();
	const others = sessionBox.value
		.split('\n')
		.map((line) => line.match(PARAMETER))
		.filter(Boolean)
		.map(([, name, value]) => [name.toUpperCase(), value]);
	return zone && zone.toUpperCase() !== 'UTC' ? [['TIMEZONE', zone], ...others] : others;
}
const tomlValue = (value) =>
	/^(true|false)$/i.test(value) ? value.toLowerCase() : /^-?\d+$/.test(value) ? value : JSON.stringify(value);

// The sqlquack.toml these choices make: only what differs from the defaults.
function configText() {
	const parts = [];
	const session = sessionParameters();
	if (session.length) parts.push(`[project.session]\n${session.map(([name, value]) => `${name} = ${tomlValue(value)}`).join('\n')}`);
	const changes = allowChanges();
	if (changes.length) parts.push(`[allow]\n${changes.map((difference) => `${difference.name} = ${isAllowed(difference)}`).join('\n')}`);
	return parts.join('\n\n');
}

function renderChoices() {
	allowList.innerHTML = differences
		.map(
			(difference) =>
				`<label><input type="checkbox" data-difference="${difference.name}"${isAllowed(difference) ? ' checked' : ''}${strictBox.checked ? ' disabled' : ''} /> ` +
				`<code>${difference.name}</code> <span>${escape(difference.title)}</span></label>`,
		)
		.join('');
	tomlBox.textContent = configText() || '# Empty: the defaults, with the session in UTC.';
}

// The Translate stage's subtitle: what the choices allow beyond the defaults.
function choicesSummary() {
	if (strictBox.checked) return 'strict: no known differences allowed';
	const added = allowChanges().filter(isAllowed);
	return added.length ? `allows: ${added.map((difference) => difference.name).join(', ')}` : 'the defaults';
}

const settings = () =>
	JSON.stringify({
		session: sessionParameters()
			.map(([name, value]) => `${name}=${value}`)
			.join('\n'),
		strict: strictBox.checked,
	});
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

let latest = null;
// The helper macros the translated statement calls, created before it runs.
let latestSetup = [];
let latestRaises = {};

// The helper macro definitions a translation calls, folded away above its SQL.
function helperMacros(setup) {
	if (!setup.length) return '';
	const count = setup.length - 1;
	return (
		`<details class="pg-helpers"><summary>${count} SQLQuack helper macro${count === 1 ? '' : 's'}, created before the SQL runs</summary>` +
		`<pre class="terminal sql">${escape(setup.map((statement) => `${statement};`).join('\n'))}</pre></details>`
	);
}
// The editor's current text and underlines, so a run's error can add its own.
let shown = { chars: [], spans: [] };
let current = 0;

async function run() {
	const ticket = ++current;
	const sql = sqlBox.value;
	const reply = await engine('analyze', { sql, sources: sourcesBox.value, settings: settings() });
	// A newer edit has already been sent; its reply will render instead.
	if (ticket !== current) return;
	const chars = Array.from(sql);
	const report = JSON.parse(reply.result);
	latest = null;
	verdict.hidden = true;
	if (report.sources_error || report.settings_error) {
		underline(chars, []);
		const what = report.sources_error ? 'The sources are not valid' : 'The policy or session is not valid';
		setStage('parse', 'skipped', terminal(`<span class="bad">${what}:</span> ${escape(report.sources_error ?? report.settings_error)}`));
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

	// A source left out, as `sqlquack convert` warns about it; the other sources still declare.
	const warnings = (report.warnings ?? []).map((w) => terminal(`<span class="warn">warning:</span> ${escape(w)}`)).join('');
	const parse = report.parse;
	add(parse.diagnostics, 'error');
	setStage(
		'parse',
		parse.status,
		warnings + (parse.diagnostics ? parse.diagnostics.map((d) => terminal(rendered(d, chars))).join('') : ''),
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
		body = (translate.statements ?? [])
			.map((s) => helperMacros(s.setup ?? []) + `<pre class="terminal sql">${escape(s.sql)}</pre>`)
			.join('');
		body = unknownRelationHints(translate.diagnostics) + body + conversionNotes(translate);
		body += (translate.diagnostics ?? []).map((d) => terminal(tone(rendered(d, chars)))).join('');
		body += allowButtons(translate.diagnostics);
	}
	const policy = translate.status === 'skipped' ? '' : choicesSummary();
	if (refused) {
		const reason = translate.diagnostics?.[0]?.message ?? 'no supported conversion';
		verdict.innerHTML = `<strong>Valid Snowflake SQL, not translated:</strong> ${escape(reason)}`;
		verdict.hidden = false;
		setStage('translate', 'refused', body, policy, 'not translated');
	} else setStage('translate', translate.status, body, policy);
	underline(chars, spans);
	shown = { chars, spans };

	const statements = translate.statements ?? [];
	if (translate.status === 'pass' && statements.length === 1) {
		latest = statements[0].sql;
		latestSetup = statements[0].setup ?? [];
		latestRaises = statements[0].raises ?? {};
		if (duckdb) execute();
		else showDuckDBLoading();
	} else if (translate.status === 'pass') {
		setStage('run', 'skipped', terminal('<span class="dim">Runs one statement at a time.</span>'));
	} else setStage('run', 'skipped', '', refused ? 'nothing was translated' : '');
}

// A button to allow each refused known difference, once each; strict mode refuses them whatever
// [allow] says, so it offers none.
function allowButtons(diagnostics) {
	if (strictBox.checked) return '';
	const names = [...new Set((diagnostics ?? []).map((d) => d.known_difference).filter(Boolean))];
	return names
		.map((name) => `<p class="pg-hint"><button class="pg-link" type="button" data-allow="${escape(name)}">Allow ${escape(name)}</button></p>`)
		.join('');
}

function allowDifference(name) {
	allowed.set(name, true);
	renderChoices();
	$('#pg-allow-panel').open = true;
	// The list scrolls on its own; bring the newly ticked row into it without moving the page.
	const row = allowList.querySelector(`[data-difference="${name}"]`)?.parentElement;
	if (row) allowList.scrollTop = row.offsetTop - allowList.offsetTop - allowList.clientHeight / 2;
	schedule();
}

// What the translation relies on that a reader should know, as `sqlquack convert` prints it:
// each known difference the policy allows, and each assumption, which gives a wrong value
// silently where it does not hold.
function conversionNotes(translate) {
	const notes = [];
	for (const statement of translate.statements ?? [])
		for (const d of statement.divergences ?? []) {
			const at = d.constructs.map((construct) => `<code>${escape(construct)}</code>`).join(', ');
			const kind = d.assumption ? 'assumption' : 'known difference';
			notes.push(`Relies on ${kind} <code>${escape(d.id)}</code> at ${at}${d.reason ? `: ${escape(d.reason)}` : ''}`);
		}
	return [...new Set(notes)].map((note) => `<p class="pg-conversion-note"><strong>Note.</strong> ${note}</p>`).join('');
}

// A relation the sources do not declare: SQLQuack needs its columns and Snowflake types, and
// does not guess them. Each one gets a line saying so, with a button that adds a stub to fill in.
const UNKNOWN_RELATION = /^relation `(.+)` has no known columns$/;
const SOURCE_NAME = /^ {2}- name: *(.+?) *$/gm;
const STUB_COLUMN = ['column_name', 'SNOWFLAKE_TYPE'];

// A name as a source file writes it: unquoted when Snowflake folds it to itself, else quoted.
function sourceName(name) {
	return name === name.toUpperCase() ? name.toLowerCase() : `'"${name.replaceAll('"', '""')}"'`;
}

// Each source's name as Snowflake resolves it: quoted names keep their case, others upper case.
function declaredRelations() {
	return [...sourcesBox.value.matchAll(SOURCE_NAME)].map(([, written]) => {
		const unquoted = written.replace(/^'(.*)'$/, '$1');
		return unquoted.startsWith('"') ? unquoted.slice(1, -1).replaceAll('""', '"') : unquoted.toUpperCase();
	});
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
				? `<p class="pg-hint"><span>SQLQuack needs each table's columns and Snowflake types. Fill in ${code}'s in the Sources box.</span></p>`
				: `<p class="pg-hint"><span>SQLQuack needs each table's columns and Snowflake types. Add ${code} to the Sources box.</span>` +
						`<button class="button" type="button" data-add-relation="${escape(name)}">Add ${escape(name)} to sources</button></p>`;
		})
		.join('');
}

// Adds `name` with one placeholder column to the end of the sources, keeping the rest of the text
// as typed, and selects the placeholder so typing replaces it.
function addRelation(name) {
	if (declaredRelations().includes(name)) return;
	const before = sourcesBox.value.trimEnd();
	const head = `${before ? `${before}\n` : 'sources:\n'}  - name: ${sourceName(name)}\n    columns:\n      - name: `;
	sourcesBox.value = `${head}${STUB_COLUMN[0]}\n        type: ${STUB_COLUMN[1]}\n`;
	sourcesBox.closest('details').open = true;
	sourcesBox.focus();
	sourcesBox.setSelectionRange(head.length, head.length + STUB_COLUMN[0].length);
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
	const conn = await db.connect();
	// The extensions this build lacks, from the site, before any SQL: DuckDB autoloads ICU while
	// binding the first statement that needs it, which corrupts that statement's function lookups.
	await conn.query(`SET custom_extension_repository = '${here(`${DUCKDB_DIR}extensions`)}'`);
	for (const extension of DUCKDB_EXTENSIONS) {
		await conn.query(`LOAD ${extension}`).catch((error) => {
			throw new Error(`the ${extension} extension did not load: ${error.message ?? error}`);
		});
	}
	const version = await conn.query('SELECT version() AS v');
	await conn.close();
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
	// Whether the error came from the translated statement itself, after the fixture loaded.
	let ranStatement = false;
	try {
		if (!duckdb) await startDuckDB();
		const conn = await duckdb.db.connect();
		try {
			const tables = JSON.parse((await engine('fixture', { sources: sourcesBox.value })).result);
			if (tables.error) throw new Error(`sources: ${tables.error}`);
			for (const statement of latestSetup) await conn.query(statement);
			for (const table of tables.tables) await conn.query(table.replace(/^CREATE TABLE/, 'CREATE OR REPLACE TABLE'));
			if (rowsBox.value.trim()) await conn.query(rowsBox.value);
			const started = performance.now();
			ranStatement = true;
			// Every value as DuckDB's own text, so decimals and timestamps show exactly.
			const result = await conn.query(`SELECT COLUMNS(*)::VARCHAR FROM (${sql}) AS result`);
			const elapsed = performance.now() - started;
			// The wrapper renames a repeated output name (ORDER_ID_1); the statement's own names come
			// from DESCRIBE, which keeps them as DuckDB and Snowflake return them.
			const fields = result.schema.fields.map((field) => field.name);
			const described = await conn.query(`DESCRIBE ${sql}`).catch(() => null);
			const own = described ? Array.from(described.getChild('column_name').toArray(), String) : [];
			if (ticket !== running) return;
			const columns = fields.map((_, index) => Array.from(result.getChildAt(index).toArray()));
			const { html: table, rows } = resultTable(own.length === fields.length ? own : fields, columns);
			setStage('run', 'pass', table, `${rows} row${rows === 1 ? '' : 's'} · ${elapsed.toFixed(0)} ms · DuckDB ${duckdb.version}`);
		} finally {
			await conn.close();
		}
	} catch (error) {
		const message = String(error.message ?? error);
		// An error a building block raised as Snowflake would: shown as Snowflake's, where it comes from.
		const failure = JSON.parse((await engine('failure', { message })).result);
		if (ticket !== running) return;
		const version = duckdb ? `DuckDB ${duckdb.version}` : '';
		if (!failure.errno && ranStatement && previewLimit(message)) {
			setStage(
				'run',
				'refused',
				'<p class="pg-conversion-note"><strong>A limit of the in-browser DuckDB, not of the translation.</strong> ' +
					'This page runs a preview build of DuckDB-WASM, which could not run this translated SQL. SQLQuack translates for ' +
					'native DuckDB 2.0: <code>sqlquack convert</code> binds every translated statement in native DuckDB before ' +
					'returning it, so run it there to see the result.</p>' +
					terminal(`<span class="dim">DuckDB-WASM:</span> ${escape(message)}`),
				version,
				'not run here',
			);
			return;
		}
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
	if (event.target.dataset.allow) allowDifference(event.target.dataset.allow);
});
sqlBox.addEventListener('input', () => {
	// Edited SQL is no longer the example, so neither the chip nor the link names it any more.
	if (activeExample) {
		setActiveExample(null);
		const url = new URL(location.href);
		url.searchParams.delete('example');
		history.replaceState(null, '', url);
	}
	schedule();
});
sourcesBox.addEventListener('input', schedule);
for (const box of [sessionBox, zoneBox, strictBox])
	box.addEventListener('input', () => {
		renderChoices();
		schedule();
	});
allowList.addEventListener('change', (event) => {
	const name = event.target.dataset.difference;
	if (!name) return;
	allowed.set(name, event.target.checked);
	renderChoices();
	schedule();
});
rowsBox.addEventListener('input', () => latest && duckdb && schedule());
sqlBox.addEventListener('scroll', () => (marks.scrollTop = sqlBox.scrollTop));

// The example chips: one click loads an example and runs it, and the link names it
// (`?example=<slug>`) so it can be shared.
const examples = EXAMPLE_GROUPS.flatMap((group) => group.examples);
const examplesBox = $('#pg-examples');
examplesBox.innerHTML = EXAMPLE_GROUPS.map(
	(group) =>
		`<div class="pg-group"><p class="pg-group-head"><strong>${escape(group.title)}</strong> <span>${escape(group.tagline)}</span></p>` +
		'<div class="pg-chips">' +
		group.examples
			.map(
				(example) =>
					`<button class="pg-chip" type="button" aria-pressed="false" data-example="${example.key}" data-stops="${example.stops ?? 'none'}">` +
					`<span class="pg-chip-name">${escape(example.name)}</span><span class="pg-chip-stop">${escape(stopLabel(example.stops))}</span></button>`,
			)
			.join('') +
		'</div></div>',
).join('');

let activeExample = null;
function setActiveExample(key) {
	activeExample = key;
	for (const chip of examplesBox.querySelectorAll('.pg-chip')) chip.setAttribute('aria-pressed', String(chip.dataset.example === key));
}

function loadExample(key) {
	sqlBox.value = EXAMPLES[key];
	setActiveExample(key);
}

examplesBox.addEventListener('click', (event) => {
	const key = event.target.closest('.pg-chip')?.dataset.example;
	if (!key) return;
	loadExample(key);
	const url = new URL(location.href);
	url.searchParams.delete('sql');
	url.searchParams.set('example', examples.find((example) => example.key === key).slug);
	history.replaceState(null, '', url);
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
if (params.get('sql') !== null) sqlBox.value = params.get('sql');
else {
	// A slug, or an example's key as earlier links used; anything else opens the default quietly.
	const requested = params.get('example');
	const key = examples.find((example) => example.slug === requested)?.key ?? (Object.hasOwn(EXAMPLES, requested ?? '') ? requested : 'valid');
	loadExample(key);
}
sourcesBox.value = SOURCES;
rowsBox.value = ROWS;
setStage('parse', 'loading', '', 'loading the engine…', 'loading');
await engineReady;
differences = JSON.parse((await engine('differences', {})).result);
renderChoices();
await run();
// DuckDB starts once the page has settled; `run` above already shows its progress.
(self.requestIdleCallback ?? ((callback) => setTimeout(callback, 200)))(() => startDuckDB().catch(() => {}), { timeout: 2000 });
