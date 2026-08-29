<script setup lang="ts">
/**
 * ZapIcon — a lightning bolt, used for the electricity biller tile.
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
    <path d="M13 2 3 14h9l-1 8 10-12h-9l1-8z" />
  </svg>
</template>
