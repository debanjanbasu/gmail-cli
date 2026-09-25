import type { APIRoute } from 'astro';

export const prerender = true;

const pages = ['', 'install/', 'privacy/', 'terms/'];

export const GET: APIRoute = ({ site }) => {
  const origin = site ?? new URL('https://debanjanbasu.github.io');
  const configuredBase = import.meta.env.BASE_URL;
  const root = configuredBase.endsWith('/') ? configuredBase : `${configuredBase}/`;
  const body = pages
    .map((path) => `  <url><loc>${new URL(`${root}${path}`, origin).toString()}</loc></url>`)
    .join('\n');
  const sitemap = `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${body}\n</urlset>\n`;
  return new Response(sitemap, {
    headers: { 'Content-Type': 'application/xml; charset=utf-8' },
  });
};
