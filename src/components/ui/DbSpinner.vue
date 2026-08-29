<script setup lang="ts">
/** DbSpinner — small indeterminate spinner for loading states. */
</script>

<template>
  <svg
    class="animate-spin text-current"
    viewBox="0 0 24 24"
    fill="none"
    aria-hidden="true">
    <circle
      class="opacity-25"
      cx="12"
      cy="12"
      r="10"
      stroke="currentColor"
      stroke-width="4" />
    <path
      class="opacity-75"
      fill="currentColor"
      d="M4 12a8 8 0 0 1 8-8V0C5.373 0 0 5.373 0 12h4z" />
  </svg>
</template>
