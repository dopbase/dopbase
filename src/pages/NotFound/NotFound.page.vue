<script setup lang="ts">
import { TerminalIcon } from "~/assets/icons";
import { DbButton } from "~/components/ui";
</script>

<template>
  <div
    class="flex min-h-svh flex-col items-center justify-center gap-4 px-6 text-center">
    <div
      class="flex h-12 w-12 items-center justify-center rounded-xl border border-line bg-panel text-ink-muted">
      <TerminalIcon class="h-6 w-6" />
    </div>
    <p class="font-mono text-sm text-ink-faint">404 · not found</p>
    <h1 class="text-lg font-semibold">This route does not exist</h1>
    <p class="max-w-md text-sm text-ink-muted">
      The page you requested is not part of the Dopbase admin console.
    </p>
    <RouterLink to="/workspace">
      <DbButton variant="primary">Back to workspace</DbButton>
    </RouterLink>
  </div>
</template>
