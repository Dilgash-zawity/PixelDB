/**
 * The small Markdown subset text widgets understand, parsed into plain data
 * that components render as elements (never as HTML): headings, paragraphs,
 * bullet and numbered lists, quotes, rules, and **bold**, *italic* and
 * `code` inside text.
 */

export interface Span {
	text: string;
	bold?: boolean;
	italic?: boolean;
	code?: boolean;
}

export type Block =
	| { kind: 'heading'; level: 1 | 2 | 3; spans: Span[] }
	| { kind: 'paragraph'; spans: Span[] }
	| { kind: 'list'; ordered: boolean; items: Span[][] }
	| { kind: 'quote'; spans: Span[] }
	| { kind: 'rule' };

/** Splits text into styled spans. Markers without a closing pair stay as text. */
export function parseInline(text: string): Span[] {
	const spans: Span[] = [];
	const re = /`([^`]+)`|\*\*([^*]+)\*\*|\*([^*]+)\*|_([^_]+)_/g;
	let last = 0;
	for (let m = re.exec(text); m; m = re.exec(text)) {
		if (m.index > last) spans.push({ text: text.slice(last, m.index) });
		if (m[1] !== undefined) spans.push({ text: m[1], code: true });
		else if (m[2] !== undefined) spans.push({ text: m[2], bold: true });
		else spans.push({ text: m[3] ?? m[4], italic: true });
		last = m.index + m[0].length;
	}
	if (last < text.length) spans.push({ text: text.slice(last) });
	return spans;
}

export function parseMarkdown(source: string): Block[] {
	const blocks: Block[] = [];
	let paragraph: string[] = [];
	let list: { ordered: boolean; items: Span[][] } | null = null;
	const flush = () => {
		if (paragraph.length) blocks.push({ kind: 'paragraph', spans: parseInline(paragraph.join(' ')) });
		paragraph = [];
		if (list) blocks.push({ kind: 'list', ...list });
		list = null;
	};
	for (const raw of source.split(/\r?\n/)) {
		const line = raw.trim();
		const heading = /^(#{1,3})\s+(.*)$/.exec(line);
		const bullet = /^[-*+]\s+(.*)$/.exec(line);
		const numbered = /^\d+[.)]\s+(.*)$/.exec(line);
		if (!line) {
			flush();
		} else if (/^(-{3,}|\*{3,})$/.test(line)) {
			flush();
			blocks.push({ kind: 'rule' });
		} else if (heading) {
			flush();
			blocks.push({ kind: 'heading', level: heading[1].length as 1 | 2 | 3, spans: parseInline(heading[2]) });
		} else if (bullet || numbered) {
			const ordered = !!numbered;
			if (paragraph.length || (list && list.ordered !== ordered)) flush();
			list ??= { ordered, items: [] };
			list.items.push(parseInline((bullet ?? numbered)![1]));
		} else if (line.startsWith('>')) {
			flush();
			blocks.push({ kind: 'quote', spans: parseInline(line.replace(/^>\s?/, '')) });
		} else {
			if (list) flush();
			paragraph.push(line);
		}
	}
	flush();
	return blocks;
}
