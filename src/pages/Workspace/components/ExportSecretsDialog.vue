<script setup lang="ts">
import { ref, watch } from "vue";
import { DbAlert, DbButton, DbModal } from "~/components/ui";
import * as secretsApi from "~/services/secrets.api";
import { serializeEnvFile } from "~/utils/env-file";
import { useReauthentication } from "~/composable";
import { DownloadIcon } from "~/assets/icons";

/**
 * ExportSecretsDialog — downloads the environment's secrets as a `.env`
 * file. Requires recent password authentication; the 403 reauth challenge
 * is handled by the global reauthentication dialog.
 */
const props = defineProps<{
  open: boolean;
  environmentId: string;
  environmentName: string;
  projectName: string;
}>();

const emit = defineEmits<{ close: []; exported: [] }>();

const working = ref(false);
const error = ref<string | null>(null);
const { runWithReauth } = useReauthentication();

watch(
  () => props.open,
  (open) => {
    if (open) {
      working.value = false;
      error.value = null;
    }
  },
);

function download(content: string, fileName: string): void {
  const blob = new Blob([content], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = fileName;
  anchor.click();
  URL.revokeObjectURL(url);
}

async function confirmExport(): Promise<void> {
  working.value = true;
  error.value = null;
  try {
    await runWithReauth(async () => {
      const result = await secretsApi.exportSecrets(props.environmentId);
      download(
        serializeEnvFile(result.entries),
        `${props.projectName}_${props.environmentName}.env`,
      );
    });
    emit("exported");
    emit("close");
  } catch {
    error.value = "The export failed. Try again.";
  } finally {
    working.value = false;
  }
}
</script>

<template>
  <DbModal
    :open="open"
    title="Export secrets"
    size="sm"
    @close="!working && emit('close')">
    <div class="flex flex-col gap-4">
      <div class="flex items-start gap-3">
        <div
          class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-warn/30 bg-warn/10 text-warn">
          <DownloadIcon class="h-4 w-4" />
        </div>
        <p class="text-sm text-ink">
          Downloads every secret in
          <span class="font-mono text-xs text-ink-strong">
            {{ projectName }}/{{ environmentName }}
          </span>
          as plaintext <code class="font-mono text-xs">.env</code>. The download
          is not encrypted — handle the file carefully.
        </p>
      </div>
      <DbAlert v-if="error">{{ error }}</DbAlert>
      <div class="flex items-center justify-end gap-2">
        <DbButton variant="ghost" :disabled="working" @click="emit('close')">
          Cancel
        </DbButton>
        <DbButton variant="primary" :loading="working" @click="confirmExport">
          Download .env
        </DbButton>
      </div>
    </div>
  </DbModal>
</template>
