<script setup lang="ts">
/**
 * DbLevelBadge — a five-bar meter and a label, so access depth scans
 * quickly down a table. It draws what it's given; `levelMeter()` in
 * `~/utils/level` turns an access level into these props.
 */
withDefaults(
  defineProps<{
    label: string;
    /** Bars lit, from 0 to 5. */
    filled: number;
    /** Light the bars in the accent colour. */
    strong?: boolean;
  }>(),
  { strong: false },
);
</script>

<template>
  <span
    class="inline-flex shrink-0 items-center gap-2 whitespace-nowrap rounded-control bg-raised py-0.5 pl-2 pr-2.5 text-sm font-medium leading-5 text-ink">
    <span class="flex items-center gap-0.5" aria-hidden="true">
      <span
        v-for="bar in 5"
        :key="bar"
        class="h-2.5 w-1 rounded-[1px]"
        :class="
          bar <= filled
            ? strong
              ? 'bg-accent'
              : 'bg-ink-strong'
            : 'bg-line-strong'
        "
        data-testid="level-bar" />
    </span>
    {{ label }}
  </span>
</template>
