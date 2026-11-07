// Diagnostic spans and result tables in the playground. The engine counts Unicode characters (code points), and
// JavaScript strings count UTF-16 units, so every span is applied to `Array.from(sql)`.

export const escape = (text) => text.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

// Spans count Unicode characters; JavaScript strings count UTF-16 units, so index by code point.
export function position(chars, offset) {
	let line = 1;
	let column = 1;
	for (let i = 0; i < offset && i < chars.length; i++) {
		if (chars[i] === '\n') {
			line += 1;
			column = 1;
		} else column += 1;
	}
	return { line, column };
}

// One diagnostic as the command line renders it, rustc style.
export function rendered(diagnostic, chars) {
	const head = `<span class="bad">error[${escape(diagnostic.code)}]</span>: ${escape(diagnostic.message)}`;
	const span = diagnostic.span;
	if (!span) return head + (diagnostic.label ? `\n  = ${escape(diagnostic.label)}` : '');
	const { line, column } = span.line ? span : position(chars, span.start);
	const lines = chars.join('').split('\n');
	const text = lines[line - 1] ?? '';
	const endOnLine = span.end_line && span.end_line !== line ? text.length + 1 : column + Math.max(1, span.end - span.start);
	const width = Math.max(1, endOnLine - column);
	const gutter = ' '.repeat(String(line).length);
	let out = `${head}\n${gutter}--> query.sql:${line}:${column}\n${gutter} |\n${line} | ${escape(text)}\n${gutter} | ${' '.repeat(column - 1)}<span class="bad">${'^'.repeat(width)}</span>`;
	if (diagnostic.label) out += ` ${escape(diagnostic.label)}`;
	for (const note of diagnostic.notes ?? []) out += `\n${gutter} = <span class="dim">note:</span> ${escape(note)}`;
	if (diagnostic.help) out += `\n${gutter} = <span class="dim">help:</span> ${escape(diagnostic.help)}`;
	return out;
}

// The editor's underline layer as HTML: every span marked; overlapping spans keep the first.
// `chars` is the SQL split into Unicode characters, the unit the engine's spans count.
export function underlined(chars, items) {
	const sorted = items.filter((item) => item.span).sort((a, b) => a.span.start - b.span.start);
	let html = '';
	let at = 0;
	sorted.forEach((item, index) => {
		const start = Math.min(item.span.start, chars.length);
		let end = Math.min(Math.max(item.span.end, start + 1), chars.length);
		if (start < at) return;
		if (end === start) {
			// A span at the very end, such as a missing token: underline the last character.
			if (start === 0) return;
			html += escape(chars.slice(at, start - 1).join(''));
			html += `<mark data-i="${index}" class="${item.kind}">${escape(chars[start - 1])}</mark>`;
			at = start;
			return;
		}
		html += escape(chars.slice(at, start).join(''));
		html += `<mark data-i="${index}" class="${item.kind}">${escape(chars.slice(start, end).join(''))}</mark>`;
		at = end;
	});
	html += escape(chars.slice(at).join('')) + '\n';
	return { html, items: sorted };
}

// The Run stage's table: `names` as the header and `columns` (one array of text or null per name)
// read by position, so two columns of the same name keep their own values.
export function resultTable(names, columns) {
	const rows = columns[0]?.length ?? 0;
	const cell = (value) => (value === null ? '<span class="dim">NULL</span>' : escape(String(value)));
	const body = Array.from({ length: rows }, (_, row) => `<tr>${columns.map((column) => `<td>${cell(column[row])}</td>`).join('')}</tr>`);
	const head = names.map((name) => `<th>${escape(name)}</th>`).join('');
	return { html: `<div class="pg-rows"><table><thead><tr>${head}</tr></thead><tbody>${body.join('')}</tbody></table></div>`, rows };
}
