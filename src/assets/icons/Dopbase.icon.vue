<script setup lang="ts">
/**
 * DopbaseIcon — the Dopbase brand mark.
 * Uses currentColor so each surface can control its color via classes.
 */
defineOptions({ inheritAttrs: true });
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 48 48"
    fill="currentColor"
    aria-hidden="true"
    v-bind="$attrs">
    <path
      fill-rule="evenodd"
      d="M24 2C11.85 2 2 11.85 2 24s9.85 22 22 22 22-9.85 22-22S36.15 2 24 2Zm0 4c9.94 0 18 8.06 18 18s-8.06 18-18 18S6 33.94 6 24 14.06 6 24 6Zm-7.5 6.75h8.05C32.1 12.75 36.5 16.95 36.5 24s-4.4 11.25-11.95 11.25H16.5v-22.5Zm5.25 4.5v13.5h2.6c4.6 0 6.9-2.25 6.9-6.75s-2.3-6.75-6.9-6.75h-2.6Z"
      clip-rule="evenodd" />
  </svg>
</template>
