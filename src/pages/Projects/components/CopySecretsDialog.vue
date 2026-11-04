<script setup lang="ts">
import { computed, toRef } from "vue";
import { useCopySecretsDialogController } from "./CopySecretsDialog.controller";
import type { CopySource } from "./EmptyEnvironment.controller";
import {
  DbAlert,
  DbButton,
  DbCheckbox,
  DbModal,
  DbSegment,
  DbSkeleton,
} from "~/components/ui";
import type { CopySecretsResponse } from "~/services/secrets.api";

/**
 * CopySecretsDialog — review the keys another environment would bring
 * into this empty one, untick what doesn't belong, and copy the rest in
 * one request. Values never reach the browser: they're copied on the
 * server, or left empty.
 */
const props = defineProps<{
  open: boolean;
  environmentId: string;
  environmentName: string;
  source: CopySource | null;
  /** What the start panel had chosen: keys and values, or keys only. */
  withValues: boolean;
}>();

const emit = defineEmits<{
  close: [];
  copied: [result: CopySecretsResponse];
}>();

const controller = useCopySecretsDialogController(
  toRef(props, "environmentId"),
  toRef(props, "source"),
  toRef(props, "open"),
  toRef(props, "withValues"),
);
const {
  keys,
  loadError,
  selected,
  selectedCount,
  allSelected,
  someSelected,
  values,
  submitting,
  submitError,
} = controller;

const copyOptions = [
  { value: "keys", label: "Keys only" },
  { value: "values", label: "Keys and values" },
];
const copyWhat = computed({
  get: () => (values.value ? "values" : "keys"),
  set: (next: string) => (values.value = next === "values"),
});
const title = computed(
  () =>
    `Copy keys from ${props.source?.name ?? ""} to ${props.environmentName}`,
);

function keysLabel(count: number): string {
  return count === 1 ? "1 key" : `${count} keys`;
}

async function submit(): Promise<void> {
  const result = await controller.submit();
  if (result) emit("copied", result);
}
</script>

<template>
  <DbModal
    :open="open"
    :title="title"
    :persistent="submitting"
    size="md"
    @close="emit('close')">
    <div class="flex flex-col gap-4" data-testid="copy-secrets-dialog">
      <p class="text-sm leading-[22px] text-ink-muted">
        Pick the keys <span class="font-mono">{{ environmentName }}</span>
        needs. Nothing in
        <span class="font-mono">{{ source?.name }}</span> changes.
      </p>
      <DbSegment
        v-if="source?.canCopyValues"
        v-model="copyWhat"
        class="self-start"
        :options="copyOptions"
        label="What to copy" />

      <div
        v-if="!keys && !loadError"
        class="flex flex-col gap-2"
        data-testid="copy-secrets-loading">
        <DbSkeleton v-for="i in 4" :key="i" class="h-8 w-full" />
      </div>
      <div v-else-if="loadError" class="flex flex-col items-start gap-3">
        <DbAlert>{{ loadError }}</DbAlert>
        <DbButton size="sm" @click="controller.load">Try again</DbButton>
      </div>
      <div
        v-else-if="keys"
        class="overflow-hidden rounded-card border border-line">
        <div class="border-b border-line-soft bg-raised/50 px-3 py-2">
          <DbCheckbox
            :model-value="allSelected"
            :indeterminate="someSelected"
            :label="`${selectedCount} of ${keysLabel(keys.length)}`"
            @update:model-value="controller.toggleAll" />
        </div>
        <ul class="max-h-[320px] overflow-y-auto" aria-label="Keys to copy">
          <li
            v-for="secret in keys"
            :key="secret.key"
            class="flex items-center gap-3 border-b border-line-soft px-3 py-2 last:border-b-0">
            <DbCheckbox
              class="min-w-0 flex-1 font-mono [overflow-wrap:anywhere]"
              :model-value="selected.has(secret.key)"
              :label="secret.key"
              @update:model-value="(on) => controller.toggle(secret.key, on)" />
            <span class="shrink-0 text-sm text-ink-faint">
              v{{ secret.version }} in {{ source?.name }}
            </span>
          </li>
        </ul>
      </div>

      <p class="text-sm leading-[22px] text-ink-muted">
        {{
          values
            ? "Values are copied on the server and never shown here."
            : "Each key starts with an empty value. Fill them in on Secrets."
        }}
      </p>
      <DbAlert v-if="submitError">{{ submitError }}</DbAlert>
    </div>

    <template #footer>
      <DbButton :disabled="submitting" @click="emit('close')">Cancel</DbButton>
      <DbButton
        variant="primary"
        :disabled="selectedCount === 0 || !keys"
        :loading="submitting"
        data-testid="copy-secrets-submit"
        @click="submit">
        Copy {{ keysLabel(selectedCount) }}
      </DbButton>
    </template>
  </DbModal>
</template>
