import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {
    ignorePatterns: ["static/hls.js"],
  },
  lint: {
    ignorePatterns: ["static/hls.js"],
  },
  plugins: [sveltekit()],
  server: {
    proxy: {
      "/auth": "http://127.0.0.1:8080",
      "/api": "http://127.0.0.1:8080",
      "/watch": "http://127.0.0.1:8080",
      "/healthz": "http://127.0.0.1:8080",
      "/readyz": "http://127.0.0.1:8080",
    },
  },
});
