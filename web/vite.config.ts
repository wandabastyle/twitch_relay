import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [react()],
  build: {
    emptyOutDir: true,
    outDir: 'build',
  },
  fmt: {
    ignorePatterns: ['build/**', 'src/lib/**', 'static/hls.js'],
    semi: true,
    singleQuote: true,
    sortImports: {
      enabled: true,
      partitionByNewline: true,
      newlinesBetween: false,
    },
  },
  lint: {
    ignorePatterns: ['build/**', 'src/lib/**', 'static/hls.js'],
    options: {
      typeAware: true,
      typeCheck: true,
    },
  },
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:8080',
      '/auth': 'http://127.0.0.1:8080',
      '/healthz': 'http://127.0.0.1:8080',
      '/readyz': 'http://127.0.0.1:8080',
      '/static': 'http://127.0.0.1:8080',
      '/stream': 'http://127.0.0.1:8080',
    },
  },
});
