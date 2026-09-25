import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://grr-cli.pages.dev',
  base: '/',
  output: 'static',
  trailingSlash: 'always',
  build: {
    format: 'directory',
  },
  compressHTML: true,
});
