import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import { assetFileNames } from "./config/asset-file-names.ts";

// https://vite.dev/config/
export default defineConfig({
  base: "./",
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: {
      "~": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  build: {
    rolldownOptions: {
      output: {
        assetFileNames,
      },
    },
  },
  server: {
    host: "0.0.0.0",
    port: 8080,
    strictPort: true,
  },
  test: {
    // Simulate a browser environment for DOM testing
    environment: "happy-dom",
    // Enable global test APIs (describe, it, expect) without importing them in every file
    globals: true,
    // Install a working `localStorage` mock and reset it between tests.
    setupFiles: ["./tests/setup.ts"],
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
        // Route definitions (per-page `*.route.ts`) and visual stories are not unit-testable units
        "src/**/*.route.ts",
        "src/**/*.story.vue",
        "src/assets/**",
      ],
    },
  },
});
