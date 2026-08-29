<script setup lang="ts">
/**
 * WardrobeIcon — a wardrobe cabinet facility indicator.
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
    <rect x="5" y="2" width="14" height="20" rx="2" />
    <path d="M12 2v20" />
    <path d="M9.5 11v2" />
    <path d="M14.5 11v2" />
  </svg>
</template>
