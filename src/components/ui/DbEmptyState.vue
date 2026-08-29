<script setup lang="ts">
/**
 * DbEmptyState — centered placeholder for empty lists and workspaces,
 * with an icon area and an actions slot.
 */
defineProps<{
  title: string;
  description?: string;
}>();
</script>

<template>
  <div
    class="flex flex-col items-center justify-center gap-3 rounded-[var(--radius-card)] border border-dashed border-line bg-panel/50 px-8 py-14 text-center">
    <div
      v-if="$slots.icon"
      class="flex h-11 w-11 items-center justify-center rounded-lg border border-line bg-raised text-ink-muted">
      <slot name="icon" />
    </div>
    <h3 class="text-base font-semibold text-ink-strong">{{ title }}</h3>
    <p v-if="description" class="max-w-md text-sm text-ink-muted">
      {{ description }}
    </p>
    <div v-if="$slots.actions" class="mt-1 flex items-center gap-2">
      <slot name="actions" />
    </div>
  </div>
</template>
