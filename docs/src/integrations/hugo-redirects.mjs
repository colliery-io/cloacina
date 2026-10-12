// After the build, a small page at each old address of the Hugo site (pages
// that moved, Hugo aliases and the old API pages) sends the reader to the new
// page. GitHub Pages is static, so a server redirect is not possible.
//
// The map is src/redirects.json (old path -> new path, both from the site
// root). The targets are relative, so the pages work under any base path. A
// page of the site is never written over.
import { existsSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, posix } from 'node:path';
import { fileURLToPath } from 'node:url';
import { redirectPage } from 'starlight-aurora/mdbook-redirects';

/** "a/b/", "x/" -> "../../x/" */
export function relativeTarget(from, to) {
	const rel = posix.relative(`/${from}`, `/${to}`);
	return rel ? `${rel}/` : './';
}

/** @param {Record<string, string>} map @returns {import('astro').AstroIntegration} */
export default function hugoRedirects(map) {
	return {
		name: 'hugo-redirects',
		hooks: {
			'astro:build:done': async ({ dir, logger }) => {
				const out = fileURLToPath(dir);
				let written = 0;
				let pages = 0;
				for (const [from, to] of Object.entries(map)) {
					const file = join(out, from, 'index.html');
					if (existsSync(file)) {
						pages++;
						continue;
					}
					await mkdir(dirname(file), { recursive: true });
					await writeFile(file, redirectPage(relativeTarget(from, to)));
					written++;
				}
				logger.info(`${written} redirects from the old Hugo addresses (${pages} are pages of the site)`);
			},
		},
	};
}
