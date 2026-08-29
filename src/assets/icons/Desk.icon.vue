<script setup lang="ts">
/**
 * DeskIcon — a desk used as a study desk facility indicator.
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
    <path d="M2 9h20" />
    <path d="M5 9v11" />
    <path d="M19 9v11" />
    <path d="M10 13h4" />
    <path d="M14 3c-1.5 1-2 2-2 3.5" />
    <path d="m12 5-3 1.5" />
    <path d="m12 5 3 1.5" />
  </svg>
</template>
