// Inline code keeps each word on one line, so `--verbose` never splits after its dashes, yet a long
// piece of inline code still wraps at its spaces: sqlquack.css sets inline code to nowrap and the
// span this puts around each run of spaces back to normal.
export const inlineCodeSpaces = {
	name: 'inline-code-spaces',
	element: {
		filter: ['code'],
		visit(node, ctx) {
			if (ctx.parent(node)?.tagName === 'pre') return;
			for (const child of node.children) {
				if (child.type === 'text' && child.value.includes(' ')) ctx.replaceNode(child, spaced(child.value));
			}
		},
	},
};

function spaced(text) {
	return text
		.split(/( +)/)
		.filter(Boolean)
		.map((part) =>
			part.startsWith(' ')
				? { type: 'element', tagName: 'span', properties: { className: ['sqq-space'] }, children: [{ type: 'text', value: part }] }
				: { type: 'text', value: part },
		);
}
