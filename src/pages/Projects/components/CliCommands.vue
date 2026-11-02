<script setup lang="ts">
import { DbCopyButton } from "~/components/ui";

/**
 * CliCommands — shell commands in an editor-colored box, each with its
 * own copy button. Presentation-only.
 */
defineProps<{
  commands: string[];
  /** Dims the box for a step that isn't ready yet. */
  dim?: boolean;
}>();
</script>

<template>
  <div
    class="flex flex-col gap-1.5 rounded-card border border-line-soft bg-editor px-4 py-3"
    :class="dim ? 'opacity-60' : ''">
    <div
      v-for="command in commands"
      :key="command"
      class="flex items-center gap-2">
      <code
        class="min-w-0 flex-1 truncate font-mono text-sm leading-[22px] text-ink-strong"
        :title="command">
        <span class="select-none text-ink-muted">$ </span>{{ command }}
      </code>
      <DbCopyButton :value="command" label="Copy command" icon-only />
    </div>
  </div>
</template>
