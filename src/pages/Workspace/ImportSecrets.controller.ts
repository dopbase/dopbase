import { computed, onUnmounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { importSecrets } from "~/services/secrets.api";
import type { ImportMode, ImportSecretsResponse } from "~/services/secrets.api";
import { errorMessage } from "~/services/api-errors";
import { useImportStore } from "~/stores/import.store";

/** Owns the review and confirmation flow for an in-memory .env import. */
export function useImportSecretsController() {
  const route = useRoute();
  const router = useRouter();
  const importStore = useImportStore();

  const environmentId = computed(() =>
    typeof route.params.environmentId === "string"
      ? route.params.environmentId
      : null,
  );
  const projectRef = computed(() =>
    typeof route.params.projectRef === "string" ? route.params.projectRef : null,
  );
  const valid = computed(
    () =>
      environmentId.value !== null &&
      importStore.pending?.environmentId === environmentId.value &&
      (importStore.pending?.entries.length ?? 0) > 0,
  );

  if (!valid.value) void router.replace({ name: "environment", params: route.params });
  onUnmounted(() => importStore.clear());

  const fileName = computed(() => importStore.pending?.fileName ?? "");
  const keys = computed(() => importStore.pending?.entries.map((entry) => entry.key) ?? []);
  const parseErrors = computed(() => importStore.pending?.errors ?? []);
  const stage = ref<"review" | "dry">("review");
  const mode = ref<ImportMode>("merge");
  const working = ref(false);
  const actionError = ref<string | null>(null);
  const dryResult = ref<ImportSecretsResponse | null>(null);

  const effectGroups = computed(() => {
    const result = dryResult.value;
    if (!result) return [];
    return [
      { label: "added", keys: result.addedKeys },
      { label: "updated", keys: result.updatedKeys },
      { label: "unchanged", keys: result.unchangedKeys },
      { label: "deleted", keys: result.deletedKeys },
    ].filter((group) => group.keys.length > 0);
  });

  function backToEnvironment(): void {
    void router.push({ name: "environment", params: route.params });
  }

  function cancel(): void {
    backToEnvironment();
  }

  async function validate(): Promise<void> {
    const pending = importStore.pending;
    const target = environmentId.value;
    if (!pending || !target || working.value) return;
    working.value = true;
    actionError.value = null;
    try {
      dryResult.value = await importSecrets(target, {
        mode: mode.value,
        dryRun: true,
        entries: pending.entries,
      });
      if (importStore.pending === pending && environmentId.value === target) {
        stage.value = "dry";
      }
    } catch (cause) {
      actionError.value = errorMessage(
        cause,
        "The import is not valid. Check the file and try again.",
      );
    } finally {
      working.value = false;
    }
  }

  async function apply(): Promise<void> {
    const pending = importStore.pending;
    const target = environmentId.value;
    const revision = dryResult.value?.revision;
    if (!pending || !target || working.value) return;
    working.value = true;
    actionError.value = null;
    try {
      await importSecrets(target, {
        mode: mode.value,
        dryRun: false,
        entries: pending.entries,
        expectedRevision: revision,
      });
      void router.push({
        name: "environment",
        params: route.params,
        query: { imported: String(Date.now()) },
      });
    } catch (cause) {
      actionError.value = errorMessage(
        cause,
        "The import failed on the server. Nothing may have changed. Run validation again to confirm.",
      );
    } finally {
      working.value = false;
    }
  }

  return {
    environmentId,
    projectRef,
    fileName,
    keys,
    parseErrors,
    stage,
    mode,
    working,
    actionError,
    effectGroups,
    backToEnvironment,
    cancel,
    validate,
    apply,
  };
}

export type ImportSecretsController = ReturnType<typeof useImportSecretsController>;
