<script setup lang="ts">
/**
 * DopbaseIcon — the Dopbase double-D mark in its primary Lavender colours.
 * Place it freestanding on dark surfaces, never inside a tile. Inherit
 * attrs so callers can set the size through class.
 */
defineOptions({ inheritAttrs: true });
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 54 64"
    aria-hidden="true"
    v-bind="$attrs">
    <path
      class="fill-brand-mark"
      d="M6 0h14a20 20 0 0 1 20 20v4a20 20 0 0 1-20 20H6a6 6 0 0 1-6-6V6a6 6 0 0 1 6-6Z" />
    <path
      class="fill-brand-mark"
      d="M20 20h14a20 20 0 0 1 20 20v4a20 20 0 0 1-20 20H20a6 6 0 0 1-6-6V26a6 6 0 0 1 6-6Z" />
    <path
      class="fill-brand-mark-overlap"
      d="M34 20a20 20 0 0 1 6 .92V24a20 20 0 0 1-20 20h-6V26a6 6 0 0 1 6-6Z" />
  </svg>
</template>
