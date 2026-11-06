// Fills {{NAME}} tokens from src/data/figures.json: in the built HTML and llms.txt files and
// while `astro dev` serves it, and in MDX pages through the exported `figures`.
import { readFile, readdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import data from './data/figures.json' with { type: 'json' };

const entries = Object.entries(data).filter(([name]) => !name.startsWith('_'));
// A null figure is hidden. It keeps its {{NAME}} token, so any use that shown() does not guard
// still shows as a placeholder that `npm run check:figures` reports.
const hidden = new Set(entries.filter(([, value]) => value === null).map(([name]) => name));
export const figures = Object.fromEntries(entries.map(([name, value]) => [name, value ?? `{{${name}}}`]));
export const shown = (name) => !hidden.has(name);

// Markdown output, such as llms.txt, escapes the underscores: {{SYNTAX\_PCT}}. A hidden figure's
// marked block, <!-- figure:NAME --> to <!-- /figure:NAME -->, is dropped; a shown one's markers go.
export function fill(text) {
	return text
		.replace(/<!-- figure:([A-Z_]+) -->([\s\S]*?)<!-- \/figure:\1 -->/g, (_, name, block) => (hidden.has(name) ? '' : block))
		.replace(/\{\{([A-Z\\_]+)\}\}/g, (token, name) => figures[name.replaceAll('\\', '')] ?? token);
}

export const placeholders = () =>
	Object.entries(figures).filter(([name, value]) => !hidden.has(name) && value === `{{${name}}}`).map(([name]) => name);

async function builtFiles(dir) {
	const entries = await readdir(dir, { withFileTypes: true });
	const nested = await Promise.all(
		entries.map((entry) => {
			const full = path.join(dir, entry.name);
			return entry.isDirectory() ? builtFiles(full) : /\.(html|txt)$/.test(entry.name) ? [full] : [];
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
				for (const file of await builtFiles(fileURLToPath(dir))) {
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
