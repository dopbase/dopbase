<script setup lang="ts">
/**
 * ClockIcon — a clock face used for chat history affordances.
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
    <circle cx="12" cy="12" r="10" />
    <polyline points="12 6 12 12 16 14" />
  </svg>
</template>
