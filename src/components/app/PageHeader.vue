<script setup lang="ts">
import { computed } from "vue";
import type { Breadcrumb } from "~/navigation/breadcrumb";

/**
 * PageHeader — the top of a page: an optional breadcrumb back up the pages
 * above it, the title, a line under it, and the page's main actions on the
 * right.
 */
const props = defineProps<{
  title: string;
  subtitle?: string;
  /** "Users", linking back to the list this page belongs to. */
  parent?: Breadcrumb;
  /**
   * Every page above this one, outermost first ("Users", then the user).
   * Takes the place of `parent`.
   */
  trail?: Breadcrumb[];
}>();

const crumbs = computed<Breadcrumb[]>(
  () => props.trail ?? (props.parent ? [props.parent] : []),
);
</script>

<template>
  <header class="flex flex-wrap items-start justify-between gap-x-4 gap-y-3">
    <div class="min-w-0">
      <nav
        v-if="crumbs.length > 0"
        aria-label="Breadcrumb"
        class="mb-1 text-[13px] text-ink-muted [overflow-wrap:anywhere]"
        data-testid="breadcrumb">
        <template v-for="(crumb, index) in crumbs" :key="index">
          <RouterLink :to="crumb.to" class="hover:text-ink hover:underline">
            {{ crumb.label }}
          </RouterLink>
          <span aria-hidden="true"> / </span>
        </template>
        <span class="text-ink" aria-current="page">{{ title }}</span>
      </nav>
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
