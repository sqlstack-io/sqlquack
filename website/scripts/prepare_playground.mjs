// Prepares the playground's served files, ignored by Git and rebuilt by `npm run build`: the engine
// from wasm/pkg into public/playground/engine/<hash>/, with engine-version.js naming that directory;
// DuckDB-WASM from node_modules into public/playground/duckdb/<version>/, so the playground serves
// it from the site rather than a CDN (its browser module bundled with its Apache Arrow import), with
// duckdb-version.js naming that directory; and the repository's policy file, which the page offers
// to load. The version in the path lets hosts cache DuckDB as immutable (public/_headers).
// Run from website/.
import { build } from 'esbuild';
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

// The engine is built separately by scripts/build_engine.sh and ignored by Git; without it the
// playground never leaves "loading the engine", so refuse to build a site that lacks it.
const engineFiles = ['sqlquack_wasm.js', 'sqlquack_wasm_bg.wasm'];
const missing = engineFiles.map((file) => `wasm/pkg/${file}`).filter((path) => !existsSync(path));
if (missing.length > 0) {
	console.error(`prepare_playground: missing ${missing.join(', ')}; run ./scripts/build_engine.sh first`);
	process.exit(1);
}
// A directory named for the engine's content, so the page can cache it for good (playground.js).
const hash = createHash('sha256');
for (const file of engineFiles) hash.update(readFileSync(join('wasm/pkg', file)));
const engineDir = `engine/${hash.digest('hex').slice(0, 12)}`;
rmSync('public/playground/engine', { recursive: true, force: true });
mkdirSync(`public/playground/${engineDir}`, { recursive: true });
for (const file of engineFiles) copyFileSync(join('wasm/pkg', file), `public/playground/${engineDir}/${file}`);
const engineBytes = statSync('wasm/pkg/sqlquack_wasm_bg.wasm').size;
writeFileSync(
	'public/playground/engine-version.js',
	`export const ENGINE_DIR = './${engineDir}/';\nexport const ENGINE_WASM_BYTES = ${engineBytes};\n`,
);

const root = 'node_modules/@duckdb/duckdb-wasm';
const { version } = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
const out = `public/playground/duckdb/${version}`;

rmSync('public/playground/duckdb', { recursive: true, force: true });
mkdirSync(out, { recursive: true });
await build({
	entryPoints: [join(root, 'dist/duckdb-browser.mjs')],
	bundle: true,
	format: 'esm',
	minify: true,
	outfile: join(out, 'duckdb-browser.mjs'),
	logLevel: 'warning',
});
for (const file of ['duckdb-base.wasm', 'duckdb-browser-base.worker.js']) {
	copyFileSync(join(root, 'dist', file), join(out, file));
}
// The wasm's decoded size lets the page show download progress whatever the transfer encoding.
const duckdbBytes = statSync(join(out, 'duckdb-base.wasm')).size;
writeFileSync(
	'public/playground/duckdb-version.js',
	`export const DUCKDB_DIR = './duckdb/${version}/';\nexport const DUCKDB_WASM_BYTES = ${duckdbBytes};\n`,
);
console.log(`duckdb-wasm ${version} vendored into ${out}`);
copyFileSync('../sqlquack.toml', 'public/playground/sqlquack.toml');
