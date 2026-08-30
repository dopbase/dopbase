<script setup lang="ts">
import { ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { DbAlert, DbButton, DbModal } from "~/components/ui";
import { parseEnvFile } from "~/utils/env-file";
import { useImportStore } from "~/stores/import.store";
import { UploadIcon } from "~/assets/icons";

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

const route = useRoute();
const router = useRouter();
const importStore = useImportStore();

const parseErrors = ref<string[]>([]);
const fileError = ref<string | null>(null);
const parsing = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);

watch(
  () => props.open,
  (open) => {
    if (open) {
      parseErrors.value = [];
      fileError.value = null;
    }
  },
);

async function onFileChange(event: Event): Promise<void> {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file) return;
  fileError.value = null;
  parseErrors.value = [];
  parsing.value = true;
  try {
    const content = await file.text();
    const parsed = parseEnvFile(content);
    if (parsed.entries.length === 0 && parsed.errors.length > 0) {
      parseErrors.value = parsed.errors;
      return;
    }
    importStore.begin({
      environmentId: props.environmentId,
      fileName: file.name,
      entries: parsed.entries,
      errors: parsed.errors,
    });
    emit("close");
    await router.push({
      name: "environment-import",
      params: {
        projectRef: route.params.projectRef,
        environmentId: props.environmentId,
      },
    });
  } catch {
    fileError.value = "The file could not be read.";
  } finally {
    parsing.value = false;
    if (fileInput.value) fileInput.value.value = "";
  }
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
        locally; the review opens on a full page where values are validated and
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
