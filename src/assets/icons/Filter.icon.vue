<script setup lang="ts">
/**
 * FilterIcon — a funnel for filter controls.
 * Inherit attrs so callers can override size/color via class or style.
 */
defineOptions({ inheritAttrs: true });
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width="18"
    height="18"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    v-bind="$attrs">
    <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3" />
  </svg>
</template>
