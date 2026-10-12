import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import mermaid from 'astro-mermaid';
import starlightAurora from 'starlight-aurora';
import apiSidebar from './src/content/docs/reference/api/_sidebar.mjs';
import hugoRedirects from './src/integrations/hugo-redirects.mjs';
import redirects from './src/redirects.json' with { type: 'json' };
import { entries, group } from './src/sidebar.mjs';

// The Pages workflow sets DOCS_SITE and DOCS_BASE. On a pull request and
// locally they are empty, and the site is at /.
const site = process.env.DOCS_SITE || undefined;
const base = process.env.DOCS_BASE || '/';

// The plissken API pages, with reference/api.md as their overview.
const api = {
	...apiSidebar,
	label: 'API Reference',
	collapsed: true,
	items: [{ slug: 'reference/api', label: 'Overview' }, ...apiSidebar.items],
};

export default defineConfig({
	site,
	base,
	// Mermaid is one large chunk, loaded only on a page with a diagram.
	vite: { build: { chunkSizeWarningLimit: 4096 } },
	integrations: [
		// Before Starlight. Light diagrams in the light theme; dark diagrams in
		// the dark theme and in hyper (the default theme applies to every theme
		// that astro-mermaid does not know).
		mermaid({ theme: 'dark', autoTheme: true }),
		starlight({
			title: 'Cloacina',
			description: 'One workflow orchestration engine, two ways to run it — embed it as a library or operate it as a service. For Rust and Python.',
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/colliery-io/cloacina' }],
			plugins: [starlightAurora({ hyper: true })],
			expressiveCode: { frames: { extractFileNameFromCode: false } },
			sidebar: [
				{
					label: 'Start Here',
					items: [
						{ slug: 'start' },
						{ slug: 'embed' },
						{ slug: 'service' },
						{ slug: 'engine' },
					],
				},
				{
					label: 'Tutorials',
					items: [group('tutorials/embed', 'Embed the Library'), group('tutorials/service', 'Run the Service')],
				},
				{
					label: 'How-to guides',
					items: [
						...entries('how-to').filter((i) => i.slug),
						group('how-to/embed', 'Embed the Library'),
						group('how-to/service', 'Run the Service'),
						group('how-to/workflows', 'Workflows'),
						group('how-to/computation-graphs', 'Computation Graphs'),
						group('how-to/constructors', 'Constructors & Providers'),
					],
				},
				{ label: 'Reference', items: [{ slug: 'reference', label: 'Overview' }, ...entries('reference', { 'reference/api': api })] },
				{
					label: 'Explanation',
					items: [
						...entries('explanation').filter((i) => i.slug),
						group('explanation/embed', 'Embed the Library'),
						group('explanation/service', 'Run the Service'),
						group('explanation/engine', 'Engine'),
						{ label: 'Primitives', collapsed: true, items: entries('explanation/primitives') },
					],
				},
				{ label: 'Contributing', items: [{ slug: 'contributing', label: 'Overview' }, ...entries('contributing')] },
			],
		}),
		// Hugo's empty tag list pages had addresses too.
		hugoRedirects({ ...redirects, 'tags/': '', 'tags/page/1/': '' }),
	],
});
