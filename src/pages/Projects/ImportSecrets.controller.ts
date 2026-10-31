import { computed, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { importSecrets } from "~/services/secrets.api";
import type { ImportMode, ImportSecretsResponse } from "~/services/secrets.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable";
import { ApiError } from "~/services/http.client";
import { useImportStore } from "~/stores/import.store";

/** Owns the review and confirmation flow for an in-memory .env import. */
export function useImportSecretsController() {
  const route = useRoute();
  const router = useRouter();
  const importStore = useImportStore();
  const { runWithReauth } = useReauthentication();
  let operationScope = new AbortController();

  const environmentName = computed(() =>
    typeof route.params.environmentName === "string"
      ? route.params.environmentName
      : null,
  );
  // The URL names the environment; the parsed file carries its id.
  const environmentId = computed(() =>
    importStore.pending?.environmentName === environmentName.value
      ? (importStore.pending?.environmentId ?? null)
      : null,
  );
  const projectRef = computed(() =>
    typeof route.params.projectRef === "string"
      ? route.params.projectRef
      : null,
  );
  const valid = computed(
    () =>
      environmentId.value !== null &&
      (importStore.pending?.entries.length ?? 0) > 0,
  );

  if (!valid.value)
    void router.replace({ name: "environment", params: route.params });
  onUnmounted(() => {
    operationScope.abort();
    importStore.clear();
  });

  const fileName = computed(() => importStore.pending?.fileName ?? "");
  const keys = computed(
    () => importStore.pending?.entries.map((entry) => entry.key) ?? [],
  );
  const parseErrors = computed(() => importStore.pending?.errors ?? []);
  const stage = ref<"review" | "dry">("review");
  const mode = ref<ImportMode>("merge");
  const working = ref(false);
  const actionError = ref<string | null>(null);
  const dryResult = ref<ImportSecretsResponse | null>(null);

  function invalidateOperation(): void {
    operationScope.abort();
    operationScope = new AbortController();
    working.value = false;
    dryResult.value = null;
    stage.value = "review";
    actionError.value = null;
  }

  watch([environmentId, mode, () => importStore.pending], invalidateOperation, {
    deep: true,
    flush: "sync",
  });

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
    invalidateOperation();
    void router.push({ name: "environment", params: route.params });
  }

  function cancel(): void {
    backToEnvironment();
  }

  async function validate(): Promise<void> {
    const pending = importStore.pending;
    const target = environmentId.value;
    if (!valid.value || !pending || !target || working.value) return;
    const signal = operationScope.signal;
    const request = {
      mode: mode.value,
      dryRun: true,
      entries: pending.entries.map(({ key, value }) => ({ key, value })),
    };
    working.value = true;
    actionError.value = null;
    try {
      await runWithReauth(async () => {
        const result = await importSecrets(target, request, signal);
        if (signal.aborted) return;
        dryResult.value = result;
        stage.value = "dry";
      }, signal);
    } catch (cause) {
      if (
        signal.aborted ||
        (cause instanceof DOMException && cause.name === "AbortError")
      )
        return;
      actionError.value = errorMessage(
        cause,
        "The import is not valid. Check the file and try again.",
      );
    } finally {
      if (!signal.aborted) working.value = false;
    }
  }

  async function apply(): Promise<void> {
    const pending = importStore.pending;
    const target = environmentId.value;
    const revision = dryResult.value?.revision;
    if (!valid.value || !pending || !target || !revision || working.value)
      return;
    const signal = operationScope.signal;
    const params = { ...route.params };
    const request = {
      mode: mode.value,
      dryRun: false,
      entries: pending.entries.map(({ key, value }) => ({ key, value })),
      expectedRevision: revision,
    };
    working.value = true;
    actionError.value = null;
    try {
      await runWithReauth(async () => {
        await importSecrets(target, request, signal);
      }, signal);
      if (signal.aborted) return;
      void router.push({
        name: "environment",
        params,
        query: { imported: String(Date.now()) },
      });
    } catch (cause) {
      if (
        signal.aborted ||
        (cause instanceof DOMException && cause.name === "AbortError")
      )
        return;
      if (cause instanceof ApiError && cause.hasCode("IMPORT_PREVIEW_STALE")) {
        dryResult.value = null;
        stage.value = "review";
        actionError.value = "The secrets changed. Preview the import again.";
      } else {
        actionError.value = errorMessage(
          cause,
          "The import failed on the server. Nothing may have changed. Run validation again to confirm.",
        );
      }
    } finally {
      if (!signal.aborted) working.value = false;
    }
  }

  return {
    environmentId,
    environmentName,
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

export type ImportSecretsController = ReturnType<
  typeof useImportSecretsController
>;
