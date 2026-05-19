import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite-plus';
import { fileURLToPath, URL } from 'node:url';

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
    },
  },
  build: {
    outDir: 'build',
    emptyOutDir: true,
  },
  server: {
    proxy: {
      '/auth': 'http://127.0.0.1:8080',
      '/api': 'http://127.0.0.1:8080',
      '/healthz': 'http://127.0.0.1:8080',
      '/readyz': 'http://127.0.0.1:8080',
      '/static': 'http://127.0.0.1:8080',
      '/stream': 'http://127.0.0.1:8080',
    },
  },
  lint: {
    ignorePatterns: ['build/**', 'static/hls.js'],
    options: {
      typeAware: true,
      typeCheck: false,
    },
  },
  fmt: {
    ignorePatterns: ['build/**', 'static/hls.js'],
    singleQuote: true,
    semi: true,
  },
});
