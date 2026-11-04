<script setup lang="ts">
import type { RouteLocationRaw } from "vue-router";

/**
 * PageHeader — the top of a page: an optional breadcrumb back to the list,
 * the title, a line under it, and the page's main actions on the right.
 */
defineProps<{
  title: string;
  subtitle?: string;
  /** "Users", linking back to the list this page belongs to. */
  parent?: { label: string; to: RouteLocationRaw };
}>();
</script>

<template>
  <header class="flex flex-wrap items-start justify-between gap-x-4 gap-y-3">
    <div class="min-w-0">
      <p v-if="parent" class="mb-1 text-[13px] text-ink-muted">
        <RouterLink :to="parent.to" class="hover:text-ink hover:underline">
          {{ parent.label }}
        </RouterLink>
        <span aria-hidden="true"> / </span>
        <span class="text-ink">{{ title }}</span>
      </p>
      <h1
        class="text-lg font-semibold leading-7 text-ink-strong [overflow-wrap:anywhere]">
        {{ title }}
      </h1>
      <p v-if="subtitle" class="mt-0.5 text-sm text-ink-muted">
        {{ subtitle }}
      </p>
      <slot name="meta" />
    </div>
    <div v-if="$slots.actions" class="flex shrink-0 items-center gap-2">
      <slot name="actions" />
    </div>
  </header>
</template>
