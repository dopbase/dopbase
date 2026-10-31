<script setup lang="ts">
import type { RouteLocationRaw } from "vue-router";

/**
 * ProjectTabs — the project page's tabs. Each is a link, so a tab opens in
 * a new browser tab too. The controller decides which tabs the viewer gets.
 */
defineProps<{
  tabs: { key: string; label: string; to: RouteLocationRaw }[];
  active: string;
}>();
</script>

<template>
  <nav class="border-b border-line-soft" aria-label="Project sections">
    <div
      class="page-width flex gap-4 overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
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
