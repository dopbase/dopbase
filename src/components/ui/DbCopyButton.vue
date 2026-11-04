<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import { CheckIcon, CopyIcon } from "~/assets/icons";
import { copyText } from "~/utils/clipboard";

/**
 * DbCopyButton — copies the given value to the clipboard and flips to a
 * short "Copied" confirmation. `iconOnly` draws a 32px icon button for
 * tight spaces, with the label kept for screen readers and the tooltip.
 * Presentation-only.
 */
const props = withDefaults(
  defineProps<{
    value: string;
    label?: string;
    iconOnly?: boolean;
  }>(),
  { label: "Copy", iconOnly: false },
);
const emit = defineEmits<{ copied: []; "copy-error": [] }>();

const copied = ref(false);
let timer: ReturnType<typeof setTimeout> | null = null;

async function copy(): Promise<void> {
  if (await copyText(props.value)) {
    copied.value = true;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => (copied.value = false), 2000);
    emit("copied");
  } else {
    copied.value = false;
    emit("copy-error");
  }
}

onBeforeUnmount(() => {
  if (timer) clearTimeout(timer);
});
</script>

<template>
  <button
    v-if="iconOnly"
    type="button"
    class="flex h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-control text-ink-muted transition-colors hover:bg-raised hover:text-ink-strong"
    :aria-label="copied ? 'Copied' : label"
    :title="copied ? 'Copied' : label"
    @click="copy">
    <CheckIcon v-if="copied" class="h-4 w-4 text-ok" />
    <CopyIcon v-else class="h-4 w-4" />
    <span class="sr-only" aria-live="polite">{{ copied ? "Copied" : "" }}</span>
  </button>
  <button
    v-else
    type="button"
    class="inline-flex cursor-pointer items-center gap-1.5 rounded-control bg-raised px-2.5 py-1 font-mono text-xs text-ink transition-colors hover:bg-line hover:text-ink-strong"
    :aria-label="copied ? 'Copied' : label"
    @click="copy">
    <CheckIcon v-if="copied" class="h-3.5 w-3.5 text-ok" />
    <CopyIcon v-else class="h-3.5 w-3.5" />
    <span>{{ copied ? "Copied" : label }}</span>
    <span class="sr-only" aria-live="polite">{{ copied ? "Copied" : "" }}</span>
  </button>
</template>
