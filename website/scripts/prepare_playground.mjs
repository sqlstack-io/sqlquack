// Prepares the playground's served files, ignored by Git and rebuilt by `npm run build`: the engine
// from wasm/pkg into public/playground/engine/<hash>/, with engine-version.js naming that directory;
// DuckDB-WASM from node_modules into public/playground/duckdb/<version>/, so the playground serves
// it from the site rather than a CDN (its browser module bundled with its Apache Arrow import, and
// the extensions the page loads), with
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
// The extensions the page loads before any SQL runs, served from the site in DuckDB's repository
// layout so the page never fetches from extensions.duckdb.org. ICU must load before the first query:
// this build lacks it, and autoloading it while a statement binds (CURRENT_DATE does) corrupts the
// binder's function lookups. The sample schema's JSON column needs json. Pinned per DuckDB-WASM
// version, since an extension only loads into the DuckDB build it was compiled for.
const EXTENSIONS = {
	'1.33.1-dev114.0': {
		path: 'v2.0.0/wasm_eh',
		sha256: {
			icu: '0e379b9d688f0df82192f15f9405f60861f478956ad0282ce831572695458308',
			json: '0d4e36ac68c53809c27a4f1e7801142409b2735ad2f7cb7f8b09b8db8d558984',
		},
	},
};
const pinned = EXTENSIONS[version];
if (!pinned) {
	console.error(`prepare_playground: no extensions pinned for duckdb-wasm ${version}; add them to EXTENSIONS`);
	process.exit(1);
}
const downloads = 'node_modules/.cache/duckdb-extensions';
mkdirSync(downloads, { recursive: true });
mkdirSync(join(out, 'extensions', pinned.path), { recursive: true });
for (const [name, sha256] of Object.entries(pinned.sha256)) {
	const file = `${name}.duckdb_extension.wasm`;
	const download = join(downloads, `${sha256}.wasm`);
	if (!existsSync(download)) {
		const url = `https://extensions.duckdb.org/${pinned.path}/${file}`;
		const response = await fetch(url);
		if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
		const bytes = Buffer.from(await response.arrayBuffer());
		const actual = createHash('sha256').update(bytes).digest('hex');
		if (actual !== sha256) throw new Error(`${url}: sha256 ${actual}, expected ${sha256}`);
		writeFileSync(download, bytes);
	}
	copyFileSync(download, join(out, 'extensions', pinned.path, file));
}
// The wasm's decoded size lets the page show download progress whatever the transfer encoding.
const duckdbBytes = statSync(join(out, 'duckdb-base.wasm')).size;
writeFileSync(
	'public/playground/duckdb-version.js',
	`export const DUCKDB_DIR = './duckdb/${version}/';\nexport const DUCKDB_WASM_BYTES = ${duckdbBytes};\n` +
		`export const DUCKDB_EXTENSIONS = ${JSON.stringify(Object.keys(pinned.sha256))};\n`,
);
console.log(`duckdb-wasm ${version} vendored into ${out}, with ${Object.keys(pinned.sha256).join(', ')}`);
