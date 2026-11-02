<script setup lang="ts">
/**
 * AgentsEmptyArt — the illustration on AI agents when there are none yet:
 * an agent between the CI job it runs in and the token it signs in with.
 * Neutral greys only, so it stays quiet next to the page.
 */
</script>

<template>
  <svg
    viewBox="0 0 320 170"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    class="h-auto w-full"
    aria-hidden="true">
    <!-- CI job, behind on the left -->
    <rect
      x="24.5"
      y="52.5"
      width="104"
      height="76"
      rx="8"
      class="fill-editor stroke-line" />
    <path d="M25 68.5h103" class="stroke-line-soft" />
    <circle cx="36" cy="60.5" r="2.5" class="fill-line" />
    <circle cx="45" cy="60.5" r="2.5" class="fill-line" />
    <circle cx="54" cy="60.5" r="2.5" class="fill-line" />
    <path
      d="M36 80l5 4-5 4"
      class="stroke-ink-faint"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round" />
    <rect x="47" y="82" width="44" height="4" rx="2" class="fill-line-strong" />
    <rect x="36" y="96" width="62" height="4" rx="2" class="fill-line" />
    <rect x="36" y="108" width="34" height="4" rx="2" class="fill-line" />

    <!-- Token, behind on the right -->
    <rect
      x="191.5"
      y="62.5"
      width="104"
      height="58"
      rx="8"
      class="fill-editor stroke-line" />
    <circle
      cx="226"
      cy="91.5"
      r="7"
      class="stroke-ink-faint"
      stroke-width="1.5" />
    <path
      d="M233 91.5h14M243 91.5v5M238.5 91.5v3.5"
      class="stroke-ink-faint"
      stroke-width="1.5"
      stroke-linecap="round" />
    <rect
      x="254"
      y="85"
      width="30"
      height="4"
      rx="2"
      class="fill-line-strong" />
    <rect x="254" y="95" width="20" height="4" rx="2" class="fill-line" />

    <!-- Antenna -->
    <path d="M160 64V50" class="stroke-line-strong" stroke-width="1.5" />
    <circle cx="160" cy="46" r="4" class="fill-ink-faint" />

    <!-- Ears -->
    <rect
      x="113.5"
      y="83.5"
      width="9"
      height="22"
      rx="3"
      class="fill-raised stroke-line-strong" />
    <rect
      x="197.5"
      y="83.5"
      width="9"
      height="22"
      rx="3"
      class="fill-raised stroke-line-strong" />

    <!-- Head -->
    <rect
      x="122.75"
      y="64.75"
      width="74.5"
      height="60.5"
      rx="11.25"
      class="fill-raised stroke-ink-faint"
      stroke-width="1.5" />
    <rect
      x="134.5"
      y="79.5"
      width="51"
      height="25"
      rx="8"
      class="fill-canvas stroke-line-strong" />
    <rect x="146" y="86" width="8" height="12" rx="4" class="fill-ink-muted" />
    <rect x="166" y="86" width="8" height="12" rx="4" class="fill-ink-muted" />
    <rect x="150" y="112" width="20" height="3" rx="1.5" class="fill-line" />
  </svg>
</template>
