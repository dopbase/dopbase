<script setup lang="ts">
/**
 * DbKbd — a key cap for shortcut hints, such as `E` or `⌘K`.
 * Presentation-only.
 */
</script>

<template>
  <kbd
    class="inline-flex shrink-0 items-center rounded-control border border-line bg-raised px-1.5 font-mono text-sm leading-[22px] text-ink-muted">
    <slot />
  </kbd>
</template>
