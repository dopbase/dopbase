<script setup lang="ts">
/**
 * WashingMachineIcon — a washing machine facility indicator.
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
    <path d="M3 6h18" />
    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
    <path d="M8 2v4" />
    <path d="M16 2v4" />
    <path d="M12 12v3" />
    <path d="M12 18a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" />
  </svg>
</template>
