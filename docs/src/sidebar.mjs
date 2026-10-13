// The sidebar, built from the pages: each folder is a group, labelled with the
// title of its index page; the pages are in the order of `sidebar.order` (the
// Hugo weight), then title — the order and labels of the Hugo menu.
// Pages with `sidebar.hidden` (the plissken API pages) are left out.
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const DOCS = fileURLToPath(new URL('./content/docs/', import.meta.url));

/** title, sidebar.order, sidebar.label and sidebar.hidden of a page. */
function meta(file) {
	const text = readFileSync(file, 'utf8');
	const fm = text.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? '';
	const value = (re) => {
		const m = fm.match(re);
		return m ? JSON.parse(m[1].startsWith('"') ? m[1] : JSON.stringify(m[1])) : undefined;
	};
	const order = fm.match(/^\s+order:\s*(-?\d+)/m);
	return {
		title: value(/^title:\s*(.+)$/m),
		label: value(/^\s+label:\s*(.+)$/m),
		hidden: /^\s+hidden:\s*true/m.test(fm),
		order: order ? Number(order[1]) : Infinity,
	};
}

const PAGE = /\.mdx?$/;

function indexOf(dir) {
	for (const name of ['index.md', 'index.mdx']) {
		const file = join(DOCS, dir, name);
		if (existsSync(file)) return file;
	}
}

const byOrder = (a, b) => a.order - b.order || a.label.localeCompare(b.label);

/** The pages and groups of a folder, without its index page. */
export function entries(dir, replace = {}) {
	const items = [];
	for (const name of readdirSync(join(DOCS, dir))) {
		const path = join(DOCS, dir, name);
		const slug = dir ? `${dir}/${name}` : name;
		if (statSync(path).isDirectory()) {
			if (replace[slug]) {
				items.push({ order: Infinity, ...replace[slug] });
				continue;
			}
			const g = folder(slug, undefined, replace);
			if (g) items.push(g);
		} else if (PAGE.test(name) && !/^index\.mdx?$/.test(name)) {
			const m = meta(path);
			const s = slug.replace(PAGE, '');
			if (m.hidden) continue;
			items.push(replace[s] ? { ...replace[s], order: m.order, label: replace[s].label } : { slug: s, label: m.label ?? m.title, order: m.order });
		}
	}
	return items.sort(byOrder).map(({ order, ...item }) => item);
}

/** A folder as a collapsed group; its index page is the first item, "Overview". */
export function folder(dir, label, replace = {}) {
	const index = indexOf(dir);
	const m = index ? meta(index) : { order: Infinity };
	const items = entries(dir, replace);
	if (index) items.unshift({ slug: dir, label: 'Overview' });
	if (!items.length) return undefined;
	return { label: label ?? m.label ?? m.title ?? dir.split('/').pop(), collapsed: true, items, order: m.order };
}

/** A group with no `order` key, for the sidebar config. */
export function group(dir, label, replace = {}) {
	const { order, ...g } = folder(dir, label, replace);
	return g;
}
