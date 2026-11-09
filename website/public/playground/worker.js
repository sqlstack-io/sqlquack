// Runs the SQLQuack engine off the page's thread, so typing never waits on a long statement.
// The page fetches and compiles the engine's wasm (from its cache when it can) and sends the
// compiled module first. Each later request carries an id; the reply echoes it, so the page can
// drop replies it no longer needs.
import { ENGINE_DIR } from './engine-version.js';

const bindings = import(`${ENGINE_DIR}sqlquack_wasm.js`);
let ready;

self.onmessage = async ({ data }) => {
	if (data.module) {
		ready = bindings.then(async (engine) => {
			await engine.default({ module_or_path: data.module });
			return engine;
		});
		ready.then(() => self.postMessage({ ready: true }));
		return;
	}
	const { analyze, fixture_tables, run_failure } = await ready;
	const started = performance.now();
	const result =
		data.op === 'fixture' ? fixture_tables(data.sources) : data.op === 'failure' ? run_failure(data.message) : analyze(data.sql, data.sources, data.settings);
	self.postMessage({ id: data.id, result, elapsed: performance.now() - started });
};
