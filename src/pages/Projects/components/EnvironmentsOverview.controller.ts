import { computed, onUnmounted, ref, watch } from "vue";
import type { Ref } from "vue";
import type { Environment } from "~/services";
import * as secretsApi from "~/services/secrets.api";
import type { SecretMetadata } from "~/services/secrets.api";
import { useReauthentication } from "~/composable";
import {
  mergeLayoutValues,
  parseEnvFileLines,
  stripLayoutValues,
  type EnvFileIssue,
} from "~/utils/env-file";

export interface EnvSecretRow {
  environment: Environment;
  secrets: SecretMetadata[] | null;
  error: string | null;
}

export interface EnvEditorState {
  environmentId: string;
  environmentName: string;
  content: string;
  baseline: string;
  loading: boolean;
  error: string | null;
  issues: EnvFileIssue[];
}

/**
 * Loads secret metadata for every environment in a project and supports
 * cross-environment editing (table + synchronized .env columns).
 */
export function useEnvironmentsOverviewController(
  projectRef: Ref<string | null>,
  environments: Ref<Environment[] | null>,
) {
  const rows = ref<EnvSecretRow[]>([]);
  const loading = ref(false);
  const loadError = ref<string | null>(null);
  const actionError = ref<string | null>(null);
  const expandedKey = ref<string | null>(null);
  const { runWithReauth } = useReauthentication();
  let loadRequest: AbortController | null = null;

  const editors = ref<EnvEditorState[]>([]);
  const editorLoading = ref(false);
  const editorLoadError = ref<string | null>(null);
  const editorAwaitingReauth = ref(false);
  const editorSaving = ref(false);
  const editorError = ref<string | null>(null);
  const sharedCaretLine = ref(1);
  let editorScope = new AbortController();

  const allKeys = computed(() => {
    const keys = new Set<string>();
    for (const row of rows.value) {
      for (const secret of row.secrets ?? []) keys.add(secret.key);
    }
    return [...keys].sort((a, b) => a.localeCompare(b));
  });

  const editorDirty = computed(() =>
    editors.value.some((editor) => editor.content !== editor.baseline),
  );

  async function load(): Promise<void> {
    const reference = projectRef.value;
    const list = environments.value;
    if (!reference || !list) {
      rows.value = [];
      return;
    }
    loadRequest?.abort();
    const request = new AbortController();
    loadRequest = request;
    loading.value = true;
    loadError.value = null;
    rows.value = list.map((environment) => ({
      environment,
      secrets: null,
      error: null,
    }));
    try {
      await Promise.all(
        rows.value.map(async (row) => {
          try {
            const secrets = await secretsApi.listSecrets(
              row.environment.id,
              request.signal,
            );
            if (!request.signal.aborted && projectRef.value === reference) {
              row.secrets = secrets;
            }
          } catch {
            if (!request.signal.aborted && projectRef.value === reference) {
              row.error = "Could not load secrets.";
              row.secrets = [];
            }
          }
        }),
      );
    } finally {
      if (loadRequest === request) loading.value = false;
    }
  }

  watch([projectRef, environments], () => void load(), { immediate: true });
  onUnmounted(() => {
    loadRequest?.abort();
    editorScope.abort();
  });

  function hasKey(key: string, environmentId: string): boolean {
    const row = rows.value.find((item) => item.environment.id === environmentId);
    return row?.secrets?.some((secret) => secret.key === key) ?? false;
  }

  async function setSecret(
    environmentId: string,
    key: string,
    value: string,
  ): Promise<void> {
    actionError.value = null;
    try {
      await secretsApi.setSecret(environmentId, key, value);
      await load();
    } catch {
      actionError.value = "The secret could not be saved.";
    }
  }

  async function deleteSecret(
    environmentId: string,
    key: string,
  ): Promise<void> {
    actionError.value = null;
    try {
      await secretsApi.deleteSecret(environmentId, key);
      await load();
    } catch {
      actionError.value = "The secret could not be deleted.";
    }
  }

  function wipeEditors(): void {
    editors.value = [];
    editorLoadError.value = null;
    editorError.value = null;
    editorAwaitingReauth.value = false;
    sharedCaretLine.value = 1;
  }

  async function openEditors(): Promise<void> {
    const list = environments.value;
    if (!list?.length) return;
    editorScope.abort();
    editorScope = new AbortController();
    const signal = editorScope.signal;
    editorLoading.value = true;
    editorLoadError.value = null;
    editorError.value = null;
    editorAwaitingReauth.value = false;
    editors.value = list.map((environment) => ({
      environmentId: environment.id,
      environmentName: environment.name,
      content: "",
      baseline: "",
      loading: true,
      error: null,
      issues: [],
    }));
    let loaded = false;
    try {
      await runWithReauth(
        async () => {
          for (const editor of editors.value) {
            if (signal.aborted) return;
            const stored = await secretsApi.getEnvLayout(editor.environmentId);
            const exported = await secretsApi.exportSecrets(editor.environmentId);
            const content = mergeLayoutValues(stored.layout, exported.entries);
            editor.content = content;
            editor.baseline = content;
            editor.issues = parseEnvFileLines(content).issues;
            editor.loading = false;
          }
          loaded = true;
          editorAwaitingReauth.value = false;
        },
        signal,
        () => {
          if (!signal.aborted) {
            editorLoading.value = false;
            editorAwaitingReauth.value = true;
          }
        },
      );
    } catch {
      if (!signal.aborted) {
        editorLoadError.value = "Could not load secrets for editing.";
      }
    } finally {
      if (!signal.aborted) editorLoading.value = false;
    }
    if (!signal.aborted && !loaded && !editorLoadError.value) {
      editorAwaitingReauth.value = true;
    }
  }

  function closeEditors(): void {
    editorScope.abort();
    editorScope = new AbortController();
    wipeEditors();
  }

  function updateEditorContent(environmentId: string, content: string): void {
    const editor = editors.value.find(
      (item) => item.environmentId === environmentId,
    );
    if (!editor) return;
    editor.content = content;
    editor.issues = parseEnvFileLines(content).issues;
  }

  async function saveEditors(): Promise<boolean> {
    if (!editorDirty.value) return true;
    editorSaving.value = true;
    editorError.value = null;
    try {
      for (const editor of editors.value) {
        if (editor.content === editor.baseline || editor.issues.length > 0) {
          continue;
        }
        const entries = parseEnvFileLines(editor.content).entries;
        const preview = await secretsApi.importSecrets(editor.environmentId, {
          mode: "replace",
          dryRun: true,
          entries,
        });
        await secretsApi.importSecrets(editor.environmentId, {
          mode: "replace",
          dryRun: false,
          entries,
          envLayout: stripLayoutValues(editor.content),
          expectedRevision: preview.revision,
        });
        editor.baseline = editor.content;
      }
      await load();
      return true;
    } catch {
      editorError.value = "One or more environments could not be saved.";
      return false;
    } finally {
      editorSaving.value = false;
    }
  }

  return {
    rows,
    loading,
    loadError,
    actionError,
    allKeys,
    expandedKey,
    hasKey,
    setSecret,
    deleteSecret,
    reload: load,
    editors,
    editorLoading,
    editorLoadError,
    editorAwaitingReauth,
    editorSaving,
    editorError,
    editorDirty,
    sharedCaretLine,
    openEditors,
    closeEditors,
    updateEditorContent,
    saveEditors,
  };
}

export type EnvironmentsOverviewController = ReturnType<
  typeof useEnvironmentsOverviewController
>;
