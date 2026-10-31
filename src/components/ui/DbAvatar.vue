<script setup lang="ts">
import { computed } from "vue";

/**
 * DbAvatar — the first letter of a name or email in a small tile, so a
 * list of people scans by eye. Square with the control radius, like every
 * other corner in the console. Presentation-only.
 */
const props = withDefaults(
  defineProps<{
    name: string;
    size?: "sm" | "md";
    /** Dimmed, for a disabled account. */
    muted?: boolean;
  }>(),
  { size: "md", muted: false },
);

const initial = computed(
  () => props.name.trim().charAt(0).toUpperCase() || "?",
);
</script>

<template>
  <span
    class="inline-flex shrink-0 items-center justify-center rounded-control bg-raised font-medium text-ink"
    :class="[
      size === 'sm' ? 'h-6 w-6 text-xs' : 'h-7 w-7 text-sm',
      muted ? 'opacity-50' : '',
    ]"
    aria-hidden="true">
    {{ initial }}
  </span>
</template>
