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
    viewBox="0 0 1024 1024"
    fill="currentColor"
    aria-hidden="true"
    v-bind="$attrs">
    <circle
      cx="512"
      cy="512"
      r="397"
      fill="none"
      stroke="currentColor"
      stroke-width="96" />
    <path
      fill-rule="evenodd"
      d="M334 298h198c146 0 226 81 226 214s-80 214-226 214H334V298Zm122 102v224h70c72 0 110-39 110-112s-38-112-110-112h-70Z"
      clip-rule="evenodd" />
  </svg>
</template>
