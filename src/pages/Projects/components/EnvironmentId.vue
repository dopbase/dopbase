<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import { CheckIcon, CopyIcon } from "~/assets/icons";
import { copyText } from "~/utils/clipboard";

/**
 * EnvironmentId — an environment's ID that copies itself when clicked, so
 * the CLI can take `env_…` instead of a typed `project/environment`. The
 * chip sits in the context bar; inline sits in a table row and shows its
 * copy icon when the row (a `group`) is hovered, or always on touch. When
 * the clipboard is blocked, the ID is selected for a manual copy instead.
 */
const props = withDefaults(
  defineProps<{
    id: string;
    variant?: "chip" | "inline";
  }>(),
  { variant: "inline" },
);

const state = ref<"copied" | "failed" | null>(null);
const text = ref<HTMLSpanElement | null>(null);
let timer: ReturnType<typeof setTimeout> | null = null;

async function copy(): Promise<void> {
  const copied = await copyText(props.id);
  state.value = copied ? "copied" : "failed";
  if (!copied && text.value) {
    window.getSelection()?.selectAllChildren(text.value);
  }
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => (state.value = null), 2000);
}

onBeforeUnmount(() => {
  if (timer) clearTimeout(timer);
});
</script>

<template>
  <button
    type="button"
    class="inline-flex max-w-full shrink-0 cursor-pointer items-center gap-1.5 rounded-control font-mono text-[13px] leading-[22px] transition-colors"
    :class="
      variant === 'chip'
        ? 'h-8 border border-line pl-2 pr-1.5 text-ink-muted hover:border-line-strong hover:bg-raised hover:text-ink-strong'
        : 'text-ink-faint hover:text-ink-strong focus-visible:text-ink-strong group-hover:text-ink-muted'
    "
    :aria-label="`Copy environment ID ${id}`"
    :title="
      state === 'failed'
        ? 'Couldn’t copy. Press ⌘C or Ctrl+C.'
        : 'Copy environment ID'
    "
    data-testid="environment-id"
    @click="copy">
    <span ref="text" class="truncate select-all">{{ id }}</span>
    <CheckIcon
      v-if="state === 'copied'"
      class="h-3.5 w-3.5 shrink-0 text-ok"
      aria-hidden="true" />
    <CopyIcon
      v-else
      class="h-3.5 w-3.5 shrink-0"
      :class="
        variant === 'inline'
          ? 'opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 pointer-coarse:opacity-100'
          : 'text-ink-faint'
      "
      aria-hidden="true" />
    <span class="sr-only" aria-live="polite">{{
      state === "copied"
        ? "Copied"
        : state === "failed"
          ? "Couldn’t copy. The ID is selected."
          : ""
    }}</span>
  </button>
</template>
