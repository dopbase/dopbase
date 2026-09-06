<script setup lang="ts">
import { computed } from "vue";

/**
 * DbBadge — small monospace status chip used for actions, states, and IDs.
 * Presentation-only.
 */
const props = withDefaults(
  defineProps<{
    tone?: "neutral" | "ok" | "warn" | "crit" | "accent";
  }>(),
  { tone: "neutral" },
);

const toneClasses = computed(() => {
  switch (props.tone) {
    case "ok":
      return "bg-ok/20 text-ok";
    case "warn":
      return "bg-warn/20 text-warn";
    case "crit":
      return "bg-crit/20 text-crit";
    case "accent":
      return "bg-accent/20 text-accent-strong";
    default:
      return "bg-line text-ink";
  }
});
</script>

<template>
  <span
    class="inline-flex min-h-6 items-center rounded-control px-2 py-0.5 font-mono text-xs font-medium leading-none"
    :class="toneClasses">
    <slot />
  </span>
</template>
