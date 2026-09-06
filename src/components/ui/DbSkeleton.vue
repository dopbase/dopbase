<script setup lang="ts">
/**
 * DbSkeleton — pulsating placeholder element for loading states to prevent layout shift.
 */
defineOptions({ inheritAttrs: true });
</script>

<template>
  <div
    class="animate-pulse rounded bg-line-soft/80"
    aria-hidden="true" />
</template>
