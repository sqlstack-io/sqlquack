// Fails while any headline figure in src/data/figures.json is still its {{NAME}} placeholder, or
// while a built page in dist/ still shows one. Run before publishing: `npm run check:figures`.
import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import { placeholders } from '../src/figures.mjs';

const problems = placeholders().map((name) => `src/data/figures.json: ${name} is unset`);
async function scan(dir) {
	for (const entry of await readdir(dir, { withFileTypes: true }).catch(() => [])) {
		const full = path.join(dir, entry.name);
		if (entry.isDirectory()) await scan(full);
		else if (/\.(html|txt)$/.test(entry.name)) {
			const left = (await readFile(full, 'utf8')).match(/\{\{[A-Z\\_]+\}\}/g);
			if (left) problems.push(`${full}: ${[...new Set(left)].join(', ')}`);
		}
	}
}
await scan('dist');
if (problems.length) {
	console.error(`placeholder figures remain:\n  ${problems.join('\n  ')}`);
	process.exit(1);
}
console.log('every figure is set');
