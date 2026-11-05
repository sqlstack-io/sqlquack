// Prepares the playground's served files, ignored by Git and rebuilt by `npm run build`:
// DuckDB-WASM from node_modules into public/playground/duckdb/<version>/, so the playground serves
// it from the site rather than a CDN (its browser module bundled with its Apache Arrow import), with
// duckdb-version.js naming that directory; and the repository's policy file, which the page offers
// to load. The version in the path lets hosts cache DuckDB as immutable (public/_headers).
// Run from website/.
import { build } from 'esbuild';
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

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
writeFileSync('public/playground/duckdb-version.js', `export const DUCKDB_DIR = './duckdb/${version}/';\n`);
console.log(`duckdb-wasm ${version} vendored into ${out}`);
copyFileSync('../sqlquack.toml', 'public/playground/sqlquack.toml');
