// Fills {{NAME}} tokens from src/data/figures.json: in the static homepage at build time and
// while `astro dev` serves it, and in MDX pages through the exported `figures`.
import { readFile, readdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import data from './data/figures.json' with { type: 'json' };

export const figures = Object.fromEntries(Object.entries(data).filter(([name]) => !name.startsWith('_')));

export function fill(text) {
	return text.replace(/\{\{([A-Z_]+)\}\}/g, (token, name) => figures[name] ?? token);
}

const placeholders = () => Object.entries(figures).filter(([name, value]) => value === `{{${name}}}`).map(([name]) => name);

async function htmlFiles(dir) {
	const entries = await readdir(dir, { withFileTypes: true });
	const nested = await Promise.all(
		entries.map((entry) => {
			const full = path.join(dir, entry.name);
			return entry.isDirectory() ? htmlFiles(full) : entry.name.endsWith('.html') ? [full] : [];
		}),
	);
	return nested.flat();
}

export function figuresIntegration() {
	return {
		name: 'sqlquack-figures',
		hooks: {
			'astro:server:setup': ({ server }) => {
				server.middlewares.use(async (req, res, next) => {
					if (req.url !== '/' && req.url !== '/index.html') return next();
					const html = await readFile(new URL('../public/index.html', import.meta.url), 'utf8');
					res.setHeader('content-type', 'text/html; charset=utf-8');
					res.end(fill(html));
				});
			},
			'astro:build:done': async ({ dir, logger }) => {
				for (const file of await htmlFiles(fileURLToPath(dir))) {
					const html = await readFile(file, 'utf8');
					const filled = fill(html);
					if (filled !== html) await writeFile(file, filled);
				}
				const left = placeholders();
				if (left.length) logger.warn(`placeholder figures still unset in src/data/figures.json: ${left.join(', ')}`);
			},
		},
	};
}
