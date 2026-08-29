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
      return "border-ok/30 bg-ok/10 text-ok";
    case "warn":
      return "border-warn/30 bg-warn/10 text-warn";
    case "crit":
      return "border-crit/30 bg-crit/10 text-crit";
    case "accent":
      return "border-accent/30 bg-accent-soft text-accent-strong";
    default:
      return "border-line bg-raised text-ink";
  }
});
</script>

<template>
  <span
    class="inline-flex items-center rounded border px-1.5 py-0.5 font-mono text-xs leading-none"
    :class="toneClasses">
    <slot />
  </span>
</template>
