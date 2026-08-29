<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { DbAlert, DbBadge, DbButton, DbModal, DbSelect } from "~/components/ui";
import {
  parseEnvFile,
  summarizeEnvEntries,
  type ParsedEnvEntry,
} from "~/utils/env-file";
import * as secretsApi from "~/services/secrets.api";
import type { ImportMode, ImportSecretsResponse } from "~/services/secrets.api";
import { UploadIcon } from "~/assets/icons";

/**
 * ImportSecretsDialog — `.env` import with local parsing, key/count
 * preview (never rendering values), server-side dry-run validation, and
 * merge/replace modes. Values are submitted to the API but never rendered.
 */
const props = defineProps<{
  open: boolean;
  environmentId: string;
}>();

const emit = defineEmits<{ close: []; imported: [] }>();

type Stage = "file" | "review" | "dry" | "done";

const stage = ref<Stage>("file");
const entries = ref<ParsedEnvEntry[]>([]);
const parseErrors = ref<string[]>([]);
const fileError = ref<string | null>(null);
const actionError = ref<string | null>(null);
const mode = ref<ImportMode>("merge");
const dryResult = ref<ImportSecretsResponse | null>(null);
const appliedResult = ref<ImportSecretsResponse | null>(null);
const working = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);

watch(
  () => props.open,
  (open) => {
    if (open) {
      stage.value = "file";
      entries.value = [];
      parseErrors.value = [];
      fileError.value = null;
      actionError.value = null;
      mode.value = "merge";
      dryResult.value = null;
      appliedResult.value = null;
      working.value = false;
    }
  },
);

const summary = computed(() => summarizeEnvEntries(entries.value));

async function onFileChange(event: Event): Promise<void> {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file) return;
  fileError.value = null;
  parseErrors.value = [];
  try {
    const content = await file.text();
    const parsed = parseEnvFile(content);
    if (parsed.entries.length === 0 && parsed.errors.length > 0) {
      parseErrors.value = parsed.errors;
      return;
    }
    entries.value = parsed.entries;
    parseErrors.value = parsed.errors;
    stage.value = "review";
  } catch {
    fileError.value = "The file could not be read.";
  } finally {
    if (fileInput.value) fileInput.value.value = "";
  }
}

async function validate(): Promise<void> {
  working.value = true;
  actionError.value = null;
  try {
    dryResult.value = await secretsApi.importSecrets(props.environmentId, {
      mode: mode.value,
      dryRun: true,
      entries: entries.value,
    });
    stage.value = "dry";
  } catch {
    actionError.value =
      "The import is not valid. Check the file and try again.";
  } finally {
    working.value = false;
  }
}

async function apply(): Promise<void> {
  working.value = true;
  actionError.value = null;
  try {
    appliedResult.value = await secretsApi.importSecrets(props.environmentId, {
      mode: mode.value,
      dryRun: false,
      entries: entries.value,
    });
    stage.value = "done";
    emit("imported");
  } catch {
    actionError.value =
      "The import failed on the server. Nothing may have changed — re-run the dry run to confirm.";
  } finally {
    working.value = false;
  }
}

const effectLists = computed(() => {
  const result = stage.value === "done" ? appliedResult.value : dryResult.value;
  if (!result) return [];
  return [
    { label: "added", keys: result.addedKeys },
    { label: "updated", keys: result.updatedKeys },
    { label: "unchanged", keys: result.unchangedKeys },
    { label: "deleted", keys: result.deletedKeys },
  ].filter((group) => group.keys.length > 0);
});
</script>

<template>
  <DbModal
    :open="open"
    title="Import .env secrets"
    @close="!working && emit('close')">
    <!-- Stage: pick file -->
    <div v-if="stage === 'file'" class="flex flex-col gap-4">
      <p class="text-sm text-ink">
        Select a <code class="font-mono text-xs">.env</code> file. It is parsed
        locally; values are validated and stored encrypted, never rendered in
        the browser.
      </p>
      <label
        class="flex cursor-pointer flex-col items-center gap-2 rounded-lg border border-dashed border-line bg-canvas px-6 py-8 text-center transition-colors hover:border-accent/50">
        <UploadIcon class="h-5 w-5 text-ink-muted" />
        <span class="text-sm text-ink">Choose a file</span>
        <input
          ref="fileInput"
          type="file"
          accept=".env,.txt,text/plain"
          class="sr-only"
          @change="onFileChange" />
      </label>
      <DbAlert v-if="fileError">{{ fileError }}</DbAlert>
    </div>

    <!-- Stage: review parsed keys -->
    <div v-else-if="stage === 'review'" class="flex flex-col gap-4">
      <p class="text-sm text-ink">
        <span class="font-mono text-ink-strong">{{ summary.count }}</span>
        keys parsed. Values are hidden.
      </p>
      <div
        class="max-h-40 overflow-y-auto rounded-md border border-line-soft bg-canvas px-3 py-2 font-mono text-xs text-ink">
        <p v-for="key in summary.keys" :key="key">{{ key }}</p>
      </div>
      <DbSelect
        v-model="mode"
        label="Mode"
        :options="[
          {
            label: 'Merge — add new keys, update existing',
            value: 'merge',
          },
          {
            label: 'Replace — also remove keys not in the file',
            value: 'replace',
          },
        ]" />
      <DbAlert v-if="parseErrors.length > 0" tone="info">
        {{ parseErrors.length }} line(s) were skipped:
        {{ parseErrors.join(" ") }}
      </DbAlert>
      <DbAlert v-if="actionError">{{ actionError }}</DbAlert>
      <div class="flex items-center justify-end gap-2">
        <DbButton variant="ghost" :disabled="working" @click="emit('close')">
          Cancel
        </DbButton>
        <DbButton variant="primary" :loading="working" @click="validate">
          Validate
        </DbButton>
      </div>
    </div>

    <!-- Stage: dry-run result -->
    <div v-else-if="stage === 'dry'" class="flex flex-col gap-4">
      <div class="flex items-center gap-2">
        <DbBadge tone="accent">dry-run ok</DbBadge>
        <span class="text-xs text-ink-muted">
          mode: {{ mode }} · nothing stored yet
        </span>
      </div>
      <div
        v-for="group in effectLists"
        :key="group.label"
        class="rounded-md border border-line-soft bg-canvas px-3 py-2">
        <p
          class="mb-1 font-mono text-xs uppercase tracking-wide text-ink-faint">
          {{ group.label }} ({{ group.keys.length }})
        </p>
        <p class="font-mono text-xs text-ink">
          {{ group.keys.join(", ") }}
        </p>
      </div>
      <DbAlert v-if="actionError">{{ actionError }}</DbAlert>
      <div class="flex items-center justify-end gap-2">
        <DbButton variant="ghost" :disabled="working" @click="stage = 'review'">
          Back
        </DbButton>
        <DbButton variant="primary" :loading="working" @click="apply">
          Apply import
        </DbButton>
      </div>
    </div>

    <!-- Stage: done -->
    <div v-else class="flex flex-col gap-4">
      <div class="flex items-center gap-2">
        <DbBadge tone="ok">imported</DbBadge>
        <span class="text-xs text-ink-muted">
          {{ summary.count }} keys processed
        </span>
      </div>
      <div
        v-for="group in effectLists"
        :key="group.label"
        class="rounded-md border border-line-soft bg-canvas px-3 py-2">
        <p
          class="mb-1 font-mono text-xs uppercase tracking-wide text-ink-faint">
          {{ group.label }} ({{ group.keys.length }})
        </p>
        <p class="font-mono text-xs text-ink">
          {{ group.keys.join(", ") }}
        </p>
      </div>
      <DbAlert v-if="actionError">{{ actionError }}</DbAlert>
      <div class="flex items-center justify-end">
        <DbButton variant="primary" @click="emit('close')">Done</DbButton>
      </div>
    </div>
  </DbModal>
</template>
