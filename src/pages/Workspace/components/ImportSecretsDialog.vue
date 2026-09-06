<script setup lang="ts">
import { ref, toRef } from "vue";
import { DbAlert, DbButton, DbModal } from "~/components/ui";
import { UploadIcon } from "~/assets/icons";
import { useImportSecretsDialogController } from "./ImportSecretsDialog.controller";

/**
 * ImportSecretsDialog — picks and parses a `.env` file locally, then hands
 * the parsed entries to the full-page review (`environment-import` route)
 * via the import store. Values are never rendered; the review happens on
 * the page, not in this popup, so large files scroll freely.
 */
const props = defineProps<{
  open: boolean;
  environmentId: string;
}>();

const emit = defineEmits<{ close: [] }>();

const fileInput = ref<HTMLInputElement | null>(null);
const { parseErrors, fileError, parsing, processFile } =
  useImportSecretsDialogController(
    toRef(props, "environmentId"),
    toRef(props, "open"),
  );

async function onFileChange(event: Event): Promise<void> {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file) return;
  if (await processFile(file)) emit("close");
  if (fileInput.value) fileInput.value.value = "";
}
</script>

<template>
  <DbModal
    :open="open"
    title="Import .env secrets"
    @close="!parsing && emit('close')">
    <div class="flex flex-col gap-4">
      <p class="text-sm text-ink">
        Select a <code class="font-mono text-xs">.env</code> file. It is parsed
        locally, the review opens on a full page where values are validated and
        stored encrypted, never rendered in the browser.
      </p>
      <label
        class="flex cursor-pointer flex-col items-center gap-2 rounded-lg border border-dashed border-line bg-canvas px-6 py-8 text-center transition-colors hover:border-accent/50">
        <UploadIcon class="h-5 w-5 text-ink-muted" />
        <span class="text-sm text-ink">
          {{ parsing ? "Parsing…" : "Choose a file" }}
        </span>
        <input
          ref="fileInput"
          type="file"
          accept=".env,.txt,text/plain"
          class="sr-only"
          @change="onFileChange" />
      </label>
      <DbAlert v-if="fileError">{{ fileError }}</DbAlert>
      <DbAlert v-if="parseErrors.length > 0" tone="info">
        {{ parseErrors.length }} line(s) were skipped:
        {{ parseErrors.join(" ") }}
      </DbAlert>
      <div class="flex items-center justify-end">
        <DbButton variant="ghost" :disabled="parsing" @click="emit('close')">
          Cancel
        </DbButton>
      </div>
    </div>
  </DbModal>
</template>
