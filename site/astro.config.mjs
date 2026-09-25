import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://debanjanbasu.github.io',
  base: '/grr-cli',
  output: 'static',
  trailingSlash: 'always',
  build: {
    format: 'directory',
  },
  compressHTML: true,
});
