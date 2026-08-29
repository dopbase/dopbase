<script setup lang="ts">
import { ref } from "vue";
import { CheckIcon, CopyIcon } from "~/assets/icons";

/**
 * DbCopyButton — copies the given value to the clipboard and flips to a
 * short "Copied" confirmation. Presentation-only.
 */
const props = withDefaults(
  defineProps<{
    value: string;
    label?: string;
  }>(),
  { label: "Copy" },
);

const copied = ref(false);
let timer: ReturnType<typeof setTimeout> | null = null;

async function copy(): Promise<void> {
  try {
    await navigator.clipboard.writeText(props.value);
    copied.value = true;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => (copied.value = false), 2000);
  } catch {
    // Clipboard unavailable (permissions/insecure context); stay silent.
  }
}
</script>

<template>
  <button
    type="button"
    class="inline-flex cursor-pointer items-center gap-1.5 rounded border border-line bg-raised px-2 py-1 font-mono text-xs text-ink transition-colors hover:border-ink-faint hover:text-ink-strong"
    @click="copy">
    <CheckIcon v-if="copied" class="h-3.5 w-3.5 text-ok" />
    <CopyIcon v-else class="h-3.5 w-3.5" />
    <span>{{ copied ? "Copied" : label }}</span>
  </button>
</template>
