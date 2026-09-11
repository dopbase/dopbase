<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { KeyIcon } from "~/assets/icons";
import { DbAlert, DbButton, DbCopyButton, DbModal } from "~/components/ui";

const props = defineProps<{
  open: boolean;
  title: string;
  token: string;
  name: string;
  id: string;
  detail?: string;
}>();

const emit = defineEmits<{ acknowledge: [] }>();
const tokenInput = ref<HTMLInputElement | null>(null);
const copyError = ref<string | null>(null);

watch(
  () => props.open,
  () => {
    copyError.value = null;
  },
);

function selectToken(): void {
  tokenInput.value?.select();
}

function handleCopied(): void {
  copyError.value = null;
}

function handleCopyError(): void {
  copyError.value =
    "Copy failed. Press Ctrl+C or Command+C to copy the selected token.";
  void nextTick(() => {
    tokenInput.value?.focus();
    selectToken();
  });
}
</script>

<template>
  <DbModal :open="open" :title="title" size="md" persistent>
    <div class="flex flex-col gap-4">
      <div
        class="flex items-start gap-3 rounded-card border border-warn/30 bg-warn/10 p-3.5">
        <div
          class="flex h-8 w-8 shrink-0 items-center justify-center rounded-control border border-warn/30 bg-canvas/40 text-warn">
          <KeyIcon class="h-4 w-4" />
        </div>
        <p class="text-sm text-ink">
          Copy this token now. It is shown once and cannot be recovered.
          <span v-if="detail">{{ detail }}</span>
        </p>
      </div>

      <section
        class="overflow-hidden rounded-card border border-accent/35 bg-canvas shadow-[inset_3px_0_0_0_var(--color-accent)]"
        aria-label="One-time token">
        <div
          class="flex items-center justify-between border-b border-line-soft px-4 py-2.5">
          <span class="font-mono text-xs font-semibold uppercase tracking-[0.12em] text-accent-strong">
            One-time token
          </span>
          <span class="text-xs text-ink-faint">plaintext</span>
        </div>
        <div class="flex flex-col gap-3 p-4 sm:flex-row sm:items-center">
          <input
            ref="tokenInput"
            :value="token"
            readonly
            aria-label="Generated token"
            class="min-w-0 flex-1 rounded-control border border-line bg-raised px-3 py-2.5 font-mono text-xs text-ink-strong outline-none selection:bg-selection"
            data-testid="one-time-token"
            @focus="selectToken"
            @click="selectToken" />
          <DbCopyButton
            class="justify-center sm:self-stretch"
            :value="token"
            label="Copy token"
            @copied="handleCopied"
            @copy-error="handleCopyError" />
        </div>
      </section>

      <div class="grid gap-2 rounded-control bg-raised px-3.5 py-3 text-sm sm:grid-cols-2">
        <div class="min-w-0">
          <p class="text-xs text-ink-muted">Name</p>
          <p class="truncate font-mono text-xs text-ink-strong">{{ name }}</p>
        </div>
        <div class="min-w-0">
          <p class="text-xs text-ink-muted">Token ID</p>
          <p class="truncate font-mono text-xs text-ink-strong">{{ id }}</p>
        </div>
      </div>

      <DbAlert v-if="copyError">{{ copyError }}</DbAlert>
    </div>

    <template #footer>
      <DbButton variant="primary" @click="emit('acknowledge')">
        I've stored it safely
      </DbButton>
    </template>
  </DbModal>
</template>
