<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { AlertTriangleIcon, TerminalIcon } from "~/assets/icons";
import { DbAlert, DbButton, DbCopyButton, DbModal } from "~/components/ui";

/**
 * OneTimeTokenDialog — shows a new token's plaintext once. It stays open
 * until the person says they've stored it; after that the value is gone.
 * Optional `usage` lines show how to use the token, and `scope` and
 * `expires` replace the token id in the summary.
 */
const props = defineProps<{
  open: boolean;
  title: string;
  token: string;
  name: string;
  id: string;
  detail?: string;
  scope?: string;
  expires?: string;
  usage?: string;
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
  <DbModal :open="open" :title="title" size="xl" persistent>
    <div class="flex flex-col gap-4">
      <div
        class="flex items-start gap-3 rounded-control bg-warn/15 px-4 py-3"
        role="note">
        <AlertTriangleIcon class="mt-0.5 h-4 w-4 shrink-0 text-warn" />
        <div class="text-sm leading-[22px]">
          <p class="font-medium text-ink-strong">
            You won't see this token again
          </p>
          <p class="text-ink">
            Store it in your deploy platform's secret settings now. If it's
            lost, revoke it and create a new one.
            <span v-if="detail">{{ detail }}</span>
          </p>
        </div>
      </div>

      <div
        class="flex items-center gap-2 rounded-control border border-line bg-canvas py-1.5 pl-3 pr-1.5">
        <input
          ref="tokenInput"
          :value="token"
          readonly
          aria-label="Generated token"
          class="min-w-0 flex-1 bg-transparent font-mono text-sm text-ink-strong outline-none selection:bg-selection"
          data-testid="one-time-token"
          @focus="selectToken"
          @click="selectToken" />
        <DbCopyButton
          :value="token"
          label="Copy"
          @copied="handleCopied"
          @copy-error="handleCopyError" />
      </div>

      <section
        v-if="usage"
        class="overflow-hidden rounded-control border border-line bg-canvas"
        aria-label="Use it">
        <div
          class="flex items-center gap-2 border-b border-line-soft py-1 pl-3 pr-1 text-sm text-ink-muted">
          <TerminalIcon class="h-4 w-4" />
          Use it
          <DbCopyButton
            class="ml-auto"
            :value="usage"
            label="Copy usage"
            icon-only />
        </div>
        <pre
          class="overflow-x-auto px-3 py-3 font-mono text-sm leading-[22px] text-ink-strong"
          data-testid="token-usage"><code>{{ usage }}</code></pre>
      </section>

      <!-- One row when there's room; each pair wraps whole on a phone. -->
      <dl class="flex flex-wrap gap-x-6 gap-y-3 text-sm">
        <div class="min-w-0">
          <dt class="text-ink-muted">Name</dt>
          <dd class="font-mono text-ink-strong [overflow-wrap:anywhere]">
            {{ name }}
          </dd>
        </div>
        <div class="min-w-0">
          <dt class="text-ink-muted">{{ scope ? "Scope" : "Token ID" }}</dt>
          <dd class="font-mono text-ink-strong [overflow-wrap:anywhere]">
            {{ scope ?? id }}
          </dd>
        </div>
        <div v-if="expires" class="min-w-0">
          <dt class="text-ink-muted">Expires</dt>
          <dd class="font-mono text-ink-strong">{{ expires }}</dd>
        </div>
      </dl>

      <DbAlert v-if="copyError">{{ copyError }}</DbAlert>
    </div>

    <template #footer>
      <DbButton variant="primary" @click="emit('acknowledge')">
        I've stored it
      </DbButton>
    </template>
  </DbModal>
</template>
