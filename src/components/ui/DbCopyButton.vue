<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
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
const emit = defineEmits<{ copied: []; "copy-error": [] }>();

const copied = ref(false);
let timer: ReturnType<typeof setTimeout> | null = null;

function copyWithSelection(value: string): boolean {
  const textarea = document.createElement("textarea");
  textarea.value = value;
  textarea.readOnly = true;
  textarea.setAttribute("aria-hidden", "true");
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  document.body.append(textarea);
  textarea.select();
  try {
    return document.execCommand("copy");
  } catch {
    return false;
  } finally {
    textarea.remove();
  }
}

async function copy(): Promise<void> {
  let succeeded = false;
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(props.value);
      succeeded = true;
    }
  } catch {
    // Plain HTTP and denied browser permissions can block the Clipboard API.
  }
  if (!succeeded) succeeded = copyWithSelection(props.value);
  if (succeeded) {
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
