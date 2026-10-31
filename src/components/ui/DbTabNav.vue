<script setup lang="ts">
import type { RouteLocationRaw } from "vue-router";

/**
 * DbTabNav — tabs that are links, so each one has its own address and
 * opens in a new browser tab too. The page decides which tabs show.
 * Presentation-only.
 */
defineProps<{
  tabs: readonly { key: string; label: string; to: RouteLocationRaw }[];
  active: string;
  /** Names the tab list for screen readers. */
  label: string;
}>();
</script>

<template>
  <nav class="border-b border-line-soft" :aria-label="label">
    <div
      class="flex gap-4 overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
      <RouterLink
        v-for="tab in tabs"
        :key="tab.key"
        :to="tab.to"
        class="relative shrink-0 px-1 py-2.5 text-sm font-medium leading-5 transition-colors pointer-coarse:py-3"
        :class="
          tab.key === active
            ? 'text-ink-strong'
            : 'text-ink-muted hover:text-ink'
        "
        :aria-current="tab.key === active ? 'page' : undefined">
        {{ tab.label }}
        <span
          v-if="tab.key === active"
          class="absolute inset-x-0 bottom-0 h-0.5 bg-accent"
          aria-hidden="true" />
      </RouterLink>
    </div>
  </nav>
</template>
