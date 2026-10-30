<script setup lang="ts">
import { useServerStatus } from "~/composable";
import { DopbaseIcon } from "~/assets/icons";

/**
 * FocusLayout — one centered column and nothing else, for pages that do a
 * single job outside the Admin UI's navigation: approving a terminal,
 * finishing a single sign-on. The header names the server, so the person
 * can check they are approving on the instance they expect.
 */
const { endpoint } = useServerStatus();
</script>

<template>
  <div class="flex min-h-svh flex-col bg-canvas">
    <header
      class="flex items-center justify-between gap-4 border-b border-line-soft px-5 py-4 sm:px-8">
      <div class="flex items-center gap-2.5">
        <DopbaseIcon class="h-7 w-7 shrink-0" />
        <span class="font-mono text-sm font-semibold text-ink-strong">
          Dopbase
        </span>
      </div>
      <span
        class="truncate font-mono text-xs text-ink-muted"
        data-testid="focus-endpoint">
        {{ endpoint }}
      </span>
    </header>
    <main
      class="flex flex-1 items-start justify-center px-5 py-12 sm:items-center sm:py-16">
      <div class="w-full max-w-md">
        <slot />
      </div>
    </main>
  </div>
</template>
