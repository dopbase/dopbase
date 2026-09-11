<script setup lang="ts">
import { computed } from "vue";
import DbSpinner from "./DbSpinner.vue";

/**
 * DbButton — the single button primitive for every interactive surface.
 * Presentation-only and carries no business logic.
 */
const props = withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "sm" | "md" | "lg";
    type?: "button" | "submit";
    disabled?: boolean;
    loading?: boolean;
  }>(),
  { variant: "secondary", size: "md", type: "button" },
);

const variantClasses = computed(() => {
  switch (props.variant) {
    case "primary":
      // Solid brand fill. Hover and active states lighten the base (PocketBase's
      // alt-color convention) instead of swapping to a different hue.
      return "bg-accent text-white hover:bg-accent-alt1 active:bg-accent-alt2";
    case "ghost":
      return "bg-transparent text-ink hover:bg-raised hover:text-ink-strong active:bg-line-strong";
    case "danger":
      // Solid destructive fill (PocketBase's danger button).
      return "bg-crit text-white hover:bg-crit-alt1 active:bg-crit-alt2";
    default:
      // Secondary: filled surface, no border (PocketBase's secondary).
      return "bg-raised text-ink-strong hover:bg-line active:bg-line-strong";
  }
});

const sizeClasses = computed(() => {
  switch (props.size) {
    case "sm":
      return "h-8 gap-1.5 px-3 text-xs";
    case "lg":
      return "h-11 gap-2 px-6 text-sm";
    default:
      return "h-10 gap-2 px-5 text-sm";
  }
});
</script>

<template>
  <button
    :type="type"
    :disabled="disabled || loading"
    class="inline-flex cursor-pointer select-none items-center justify-center rounded-control font-semibold transition-colors duration-150 active:duration-75 disabled:pointer-events-none disabled:opacity-50"
    :class="[variantClasses, sizeClasses]">
    <DbSpinner v-if="loading" class="h-3.5 w-3.5" />
    <slot />
  </button>
</template>
