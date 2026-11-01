<script setup lang="ts">
/**
 * GroupsEmptyArt — the illustration on Groups when there are none yet:
 * a person joining a group, and the group reaching a project. Neutral
 * greys only, so it stays quiet next to the page.
 */

// Members in the stack, left to right.
const members = [122, 146, 170];
</script>

<template>
  <svg
    viewBox="0 0 320 170"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    class="h-auto w-full"
    aria-hidden="true">
    <!-- Stacked groups behind -->
    <rect
      x="104.5"
      y="30.5"
      width="111"
      height="90"
      rx="10"
      class="fill-canvas stroke-line-soft" />
    <rect
      x="88.5"
      y="41.5"
      width="143"
      height="96"
      rx="10"
      class="fill-editor stroke-line" />

    <!-- A person joining, on the left -->
    <path
      d="M50 104h22"
      class="stroke-line-strong"
      stroke-width="1.5"
      stroke-dasharray="3 4"
      stroke-linecap="round" />
    <circle cx="38" cy="104" r="12.5" class="fill-editor stroke-line" />
    <circle cx="38" cy="100" r="3.5" class="fill-ink-faint" />
    <path d="M32 112.5a6 6 0 0 1 12 0z" class="fill-ink-faint" />

    <!-- The group reaching a project, on the right -->
    <path
      d="M249 104h17"
      class="stroke-line-strong"
      stroke-width="1.5"
      stroke-dasharray="3 4"
      stroke-linecap="round" />
    <rect
      x="266.5"
      y="87.5"
      width="36"
      height="33"
      rx="6"
      class="fill-editor stroke-line" />
    <path
      d="M275 98.5a2 2 0 0 1 2-2h4.5l2 2.5h7.5a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2h-14a2 2 0 0 1-2-2z"
      class="stroke-ink-faint"
      stroke-width="1.5"
      stroke-linejoin="round" />

    <!-- Front group -->
    <rect
      x="72.75"
      y="54.75"
      width="174.5"
      height="96.5"
      rx="9.25"
      class="fill-raised stroke-ink-faint"
      stroke-width="1.5" />
    <rect
      x="88"
      y="69"
      width="58"
      height="5"
      rx="2.5"
      class="fill-line-strong" />
    <rect
      x="194.75"
      y="64.75"
      width="38.5"
      height="14.5"
      rx="7.25"
      class="stroke-line-strong"
      stroke-width="1.5" />
    <path
      d="M203 72l2.5 2.5 4.5-5"
      class="stroke-ink-muted"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round" />
    <rect
      x="213"
      y="70.5"
      width="14"
      height="3"
      rx="1.5"
      class="fill-line-strong" />

    <!-- Members -->
    <g v-for="cx in members" :key="cx">
      <circle
        :cx="cx"
        cy="106"
        r="15"
        stroke-width="3"
        class="fill-line stroke-raised" />
      <circle :cx="cx" cy="101" r="4.5" class="fill-ink-muted" />
      <path :d="`M${cx - 8} 116.5a8 7.5 0 0 1 16 0z`" class="fill-ink-muted" />
    </g>

    <!-- Room for one more -->
    <circle
      cx="200"
      cy="106"
      r="13.5"
      class="stroke-line-strong"
      stroke-width="1.5"
      stroke-dasharray="3 3" />
    <path
      d="M200 100v12M194 106h12"
      class="stroke-ink-faint"
      stroke-width="1.5"
      stroke-linecap="round" />

    <rect x="88" y="134" width="70" height="4" rx="2" class="fill-line" />
    <rect x="166" y="134" width="40" height="4" rx="2" class="fill-line" />
  </svg>
</template>
