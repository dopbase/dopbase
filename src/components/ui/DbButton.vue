<script setup lang="ts">
import { computed } from "vue";
import DbSpinner from "./DbSpinner.vue";

/**
 * DbButton — the single button primitive for every interactive surface.
 * Presentation-only; carries no business logic.
 */
const props = withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "sm" | "md";
    type?: "button" | "submit";
    disabled?: boolean;
    loading?: boolean;
  }>(),
  { variant: "secondary", size: "md", type: "button" },
);

const variantClasses = computed(() => {
  switch (props.variant) {
    case "primary":
      return "bg-accent text-white hover:bg-accent-strong border border-transparent";
    case "ghost":
      return "bg-transparent text-ink hover:bg-raised hover:text-ink-strong border border-transparent";
    case "danger":
      return "bg-crit/10 text-crit border border-crit/40 hover:bg-crit/20";
    default:
      return "bg-raised text-ink-strong border border-line hover:border-ink-faint";
  }
});

const sizeClasses = computed(() =>
  props.size === "sm" ? "h-8 px-3 text-xs gap-1.5" : "h-9 px-4 text-sm gap-2",
);
</script>

<template>
  <button
    :type="type"
    :disabled="disabled || loading"
    class="inline-flex cursor-pointer items-center justify-center rounded-md font-medium transition-colors disabled:pointer-events-none disabled:opacity-50"
    :class="[variantClasses, sizeClasses]">
    <DbSpinner v-if="loading" class="h-3.5 w-3.5" />
    <slot />
  </button>
</template>
