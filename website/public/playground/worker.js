// Runs the SQLQuack engine off the page's thread, so typing never waits on a long statement.
// Each request carries an id; the reply echoes it, so the page can drop replies it no longer needs.
import init, { analyze, fixture_tables } from './engine/sqlquack_wasm.js';

const ready = init();

self.onmessage = async ({ data }) => {
	await ready;
	const started = performance.now();
	const result = data.op === 'fixture' ? fixture_tables(data.schema) : analyze(data.sql, data.schema, data.settings);
	self.postMessage({ id: data.id, result, elapsed: performance.now() - started });
};

ready.then(() => self.postMessage({ ready: true }));
