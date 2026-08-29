<script setup lang="ts">
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
    <path
      d="M4 12h16a1 1 0 0 1 1 1v1a5 5 0 0 1-5 5H8a5 5 0 0 1-5-5v-1a1 1 0 0 1 1-1Z" />
    <path d="M6 12V5a2 2 0 0 1 2-2h1.5A2.5 2.5 0 0 1 12 5.5" />
    <path d="M6 19v2M18 19v2" />
  </svg>
</template>
