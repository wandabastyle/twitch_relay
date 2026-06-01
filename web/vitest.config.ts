import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'jsdom',
    exclude: ['node_modules', 'build'],
    include: ['src/**/*.test.ts'],
  },
});
