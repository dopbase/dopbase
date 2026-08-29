<script setup lang="ts">
import { computed } from "vue";

/**
 * DbAlert — inline banner for form-level and load errors.
 * Presentation-only.
 */
const props = withDefaults(
  defineProps<{
    tone?: "error" | "info" | "success";
  }>(),
  { tone: "error" },
);

const toneClasses = computed(() => {
  switch (props.tone) {
    case "info":
      return "border-accent/30 bg-accent-soft text-accent-strong";
    case "success":
      return "border-ok/30 bg-ok/10 text-ok";
    default:
      return "border-crit/30 bg-crit/10 text-crit";
  }
});
</script>

<template>
  <div
    class="rounded-md border px-3 py-2 text-xs leading-relaxed"
    :class="toneClasses"
    role="alert">
    <slot />
  </div>
</template>
