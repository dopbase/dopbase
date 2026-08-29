<script setup lang="ts">
/**
 * ElevatorIcon — up/down arrows used as an elevator facility indicator.
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
    <rect x="4" y="3" width="16" height="18" rx="2" />
    <path d="M12 3v18" />
    <path d="m7.5 10 1.5-2 1.5 2" />
    <path d="m13.5 14 1.5 2 1.5-2" />
  </svg>
</template>
