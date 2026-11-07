import { readFileSync } from "node:fs";
import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import { assetFileNames } from "./config/asset-file-names.ts";

const pkg = JSON.parse(
  readFileSync(new URL("./package.json", import.meta.url), "utf8"),
) as { version: string };

// Dates render in the browser's locale, and tests assert US-style text such
// as "Nov 26, 2026". Node takes its default locale from LANG/LC_ALL when a
// process starts, so set them here, before Vitest forks its workers, to make
// test runs the same in every terminal.
if (process.env.VITEST) {
  process.env.LC_ALL = "en_US.UTF-8";
  process.env.LANG = "en_US.UTF-8";
}

// https://vite.dev/config/
export default defineConfig({
  // The backend serves the UI root-mounted from ../dist (rust-embed), so
  // assets must use absolute URLs — relative ones break on nested SPA routes.
  base: "/",
  plugins: [vue(), tailwindcss()],
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  resolve: {
    alias: {
      "~": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  build: {
    // The server's CSP allows images and fonts from 'self' only, so Vite must
    // not inline small assets as data: URIs; the browser would block them.
    assetsInlineLimit: 0,
    rolldownOptions: {
      checks: {
        pluginTimings: false, // turn off the PLUGIN_TIMINGS message
      },
      output: {
        assetFileNames,
      },
    },
  },
  server: {
    host: "0.0.0.0",
    port: 9000,
    strictPort: true,
    proxy: {
      "/api": {
        target: "http://127.0.0.1:8840",
        changeOrigin: true,
      },
    },
  },
  test: {
    // Simulate a browser environment for DOM testing
    environment: "happy-dom",
    // Enable global test APIs (describe, it, expect) without importing them in every file
    globals: true,
    // Install a working `localStorage` mock and reset it between tests.
    setupFiles: ["./src/tests/setup.ts"],
    // Exclude route definitions and story files from being collected as tests.
    // Routes are defined per-page in `*.route.ts` (or sometimes inline in the page),
    // and `*.story.vue` files are visual stories — neither are unit tests.
    exclude: [
      "**/node_modules/**",
      "**/dist/**",
      "**/cypress/**",
      "**/.{idea,git,cache,output,temp}/**",
      "**/{karma,rollup,webpack,vite,vitest,jest,ava,babel,nyc,cypress,tsup,build}.config.*",
      "src/**/*.route.ts",
      "src/**/*.story.vue",
      "src/assets/**",
      "scripts/**/*.test.mjs",
    ],
    coverage: {
      provider: "v8",
      reporter: ["text", "html", "lcov"],
      reportsDirectory: "./coverage",
      include: ["src/**/*.{ts,vue}"],
      exclude: [
        "src/**/*.test.ts",
        "src/**/*.test.tsx",
        "src/**/*.d.ts",
        "src/**/*.interface.ts",
        "src/App.vue",
        "src/router.ts",
        "src/main.ts",
        "src/**/index.ts",
        "src/tests/**",
        // Route definitions (per-page `*.route.ts`) and visual stories are not unit-testable units
        "src/**/*.route.ts",
        "src/**/*.story.vue",
        "src/assets/**",
      ],
    },
  },
});
