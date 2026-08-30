import { computed, onUnmounted, ref, watch } from "vue";
import type { Ref } from "vue";
import * as secretsApi from "~/services/secrets.api";
import type {
  ImportSecretsResponse,
  SecretMetadata,
} from "~/services/secrets.api";
import { ApiError } from "~/services/http.client";
import { useReauthentication } from "~/composable";
import {
  mergeLayoutValues,
  parseEnvFileLines,
  stripLayoutValues,
  type EnvFileIssue,
} from "~/utils/env-file";

/** How long a revealed plaintext stays visible in component memory. */
export const REVEAL_SECONDS = 30;

/**
 * Secrets panel controller: metadata listing, secure set/delete, and the
 * recent-password reveal flow with its auto-hide countdown.
 *
 * Revealed plaintext lives only in this component's memory — never in
 * storage or global state — and is dropped on route change or unmount.
 * Copying does not extend the timer.
 *
 * The `.env` editor follows the same discipline: opening it reauthenticates
 * and loads all plaintext into the buffer (via the export endpoint), and
 * the buffer is wiped on close, environment change, or unmount. Saving runs
 * a server dry-run first and persists the value-free layout alongside the
 * entries so comments and ordering survive reloads.
 */
export function useSecretsPanelController(environmentId: Ref<string>) {
  const secrets = ref<SecretMetadata[] | null>(null);
  const loading = ref(false);
  const loadError = ref<string | null>(null);
  const actionError = ref<string | null>(null);
  const { runWithReauth } = useReauthentication();

  const revealedKey = ref<string | null>(null);
  const revealedValue = ref<string | null>(null);
  const revealSecondsLeft = ref(0);
  let hideTimer: ReturnType<typeof setInterval> | null = null;

  // .env editor state — declared before the environment watch below, which
  // wipes the buffer immediately on mount.
  /** Whether the editor view is active. Content lives only while it is. */
  const editorOpen = ref(false);
  const editorContent = ref<string | null>(null);
  /** The content as last loaded or saved; the dirty-check baseline. */
  const editorBaseline = ref<string | null>(null);
  const editorLoading = ref(false);
  const editorLoadError = ref<string | null>(null);
  const editorSaving = ref(false);
  const editorError = ref<string | null>(null);
  /** Dry-run result awaiting confirmation. */
  const editorDiff = ref<ImportSecretsResponse | null>(null);
  /** True when the initial load is parked behind the reauth dialog. */
  const editorAwaitingReauth = ref(false);

  function hideRevealed(): void {
    revealedKey.value = null;
    revealedValue.value = null;
    revealSecondsLeft.value = 0;
    if (hideTimer) {
      clearInterval(hideTimer);
      hideTimer = null;
    }
  }

  async function load(): Promise<void> {
    loading.value = true;
    loadError.value = null;
    try {
      secrets.value = await secretsApi.listSecrets(environmentId.value);
    } catch {
      loadError.value = "Could not load secrets.";
      secrets.value = null;
    } finally {
      loading.value = false;
    }
  }

  watch(
    environmentId,
    () => {
      hideRevealed();
      wipeEditor();
      load();
    },
    { immediate: true },
  );
  onUnmounted(() => {
    hideRevealed();
    wipeEditor();
  });

  const revealCountdown = computed(() => `${revealSecondsLeft.value}s`);

  function describeError(cause: unknown, fallback: string): string {
    if (cause instanceof ApiError && cause.firstCode === "REQUEST_INVALID") {
      return "Keys may use letters, numbers, '_', '-', or '.', and cannot start with a number.";
    }
    if (cause instanceof ApiError && cause.status === 0) {
      return "Cannot reach the Dopbase server.";
    }
    return fallback;
  }

  async function reveal(key: string): Promise<void> {
    actionError.value = null;
    try {
      await runWithReauth(async () => {
        const revealed = await secretsApi.revealSecret(
          environmentId.value,
          key,
        );
        hideRevealed();
        revealedKey.value = revealed.key;
        revealedValue.value = revealed.value;
        revealSecondsLeft.value = REVEAL_SECONDS;
        hideTimer = setInterval(() => {
          revealSecondsLeft.value -= 1;
          if (revealSecondsLeft.value <= 0) hideRevealed();
        }, 1000);
      });
    } catch (cause) {
      actionError.value = describeError(cause, "Could not reveal the secret.");
    }
  }

  async function setSecret(key: string, value: string): Promise<void> {
    actionError.value = null;
    try {
      await secretsApi.setSecret(environmentId.value, key, value);
      await load();
    } catch (cause) {
      actionError.value = describeError(cause, "Could not save the secret.");
      throw cause;
    }
  }

  async function deleteSecret(key: string): Promise<void> {
    actionError.value = null;
    try {
      await secretsApi.deleteSecret(environmentId.value, key);
      if (revealedKey.value === key) hideRevealed();
      await load();
    } catch (cause) {
      actionError.value = describeError(cause, "Could not delete the secret.");
      throw cause;
    }
  }

  // -------------------------------------------------------------------------
  // .env editor
  // -------------------------------------------------------------------------

  function wipeEditor(): void {
    editorOpen.value = false;
    editorContent.value = null;
    editorBaseline.value = null;
    editorLoading.value = false;
    editorLoadError.value = null;
    editorSaving.value = false;
    editorError.value = null;
    editorDiff.value = null;
    editorAwaitingReauth.value = false;
  }

  const editorIssues = computed<EnvFileIssue[]>(() =>
    editorContent.value === null
      ? []
      : parseEnvFileLines(editorContent.value).issues,
  );
  const editorEntries = computed(() =>
    editorContent.value === null
      ? []
      : parseEnvFileLines(editorContent.value).entries,
  );
  const editorDirty = computed(
    () =>
      editorContent.value !== null &&
      editorContent.value !== editorBaseline.value,
  );
  const editorCanSave = computed(
    () =>
      editorDirty.value &&
      editorIssues.value.length === 0 &&
      !editorLoading.value &&
      !editorSaving.value,
  );

  /**
   * Opens the editor: loads the stored layout (values-free) and, behind
   * reauthentication, the current plaintext values, then merges both into
   * the editable buffer. With no stored layout yet, a fresh `KEY=` slot
   * layout is generated from the current keys. When the value load is
   * parked behind the reauthentication dialog, the buffer is filled by the
   * parked retry once the password is confirmed.
   */
  async function openEditor(): Promise<void> {
    editorOpen.value = true;
    editorLoading.value = true;
    editorLoadError.value = null;
    editorDiff.value = null;
    editorError.value = null;
    editorAwaitingReauth.value = false;
    let loaded = false;
    try {
      const stored = await secretsApi.getEnvLayout(environmentId.value);
      await runWithReauth(async () => {
        // The merge happens inside the closure so a parked reauthentication
        // retry fills the buffer when the password is confirmed.
        if (!editorOpen.value) return;
        const exported = await secretsApi.exportSecrets(environmentId.value);
        const content = mergeLayoutValues(stored.layout, exported.entries);
        editorContent.value = content;
        editorBaseline.value = content;
        editorAwaitingReauth.value = false;
        loaded = true;
      });
    } catch (cause) {
      editorLoadError.value = describeError(
        cause,
        "Could not load the secrets for editing.",
      );
    } finally {
      editorLoading.value = false;
    }
    if (!loaded && editorLoadError.value === null) {
      editorAwaitingReauth.value = true;
    }
  }

  function closeEditor(): void {
    wipeEditor();
  }

  /** Runs the server dry-run and parks the result for confirmation. */
  async function saveDraft(): Promise<boolean> {
    if (!editorCanSave.value || editorContent.value === null) return false;
    editorSaving.value = true;
    editorError.value = null;
    try {
      editorDiff.value = await secretsApi.importSecrets(environmentId.value, {
        mode: "replace",
        dryRun: true,
        entries: editorEntries.value,
      });
      return true;
    } catch (cause) {
      editorError.value = describeError(
        cause,
        "The changes are not valid. Check the editor and try again.",
      );
      return false;
    } finally {
      editorSaving.value = false;
    }
  }

  /** Applies the confirmed diff and persists the value-free layout. */
  async function applyDraft(): Promise<boolean> {
    if (editorContent.value === null) return false;
    editorSaving.value = true;
    editorError.value = null;
    try {
      await secretsApi.importSecrets(environmentId.value, {
        mode: "replace",
        dryRun: false,
        entries: editorEntries.value,
        envLayout: stripLayoutValues(editorContent.value),
      });
      editorBaseline.value = editorContent.value;
      editorDiff.value = null;
      hideRevealed();
      await load();
      return true;
    } catch (cause) {
      editorError.value = describeError(
        cause,
        "The changes could not be saved. Nothing may have changed — review and try again.",
      );
      return false;
    } finally {
      editorSaving.value = false;
    }
  }

  /** Reverts the buffer to the last loaded or saved content. */
  function discardDraft(): void {
    if (editorBaseline.value !== null)
      editorContent.value = editorBaseline.value;
    editorDiff.value = null;
    editorError.value = null;
  }

  /** Returns from the confirmation summary to the buffer. */
  function backToEditing(): void {
    editorDiff.value = null;
  }

  return {
    secrets,
    loading,
    loadError,
    actionError,
    /** Refetches the metadata listing (e.g. after a full-page import). */
    reload: load,
    revealedKey,
    revealedValue,
    revealCountdown,
    hideRevealed,
    reveal,
    setSecret,
    deleteSecret,
    editorOpen,
    editorContent,
    editorLoading,
    editorLoadError,
    editorAwaitingReauth,
    editorSaving,
    editorError,
    editorDiff,
    editorIssues,
    editorEntries,
    editorDirty,
    editorCanSave,
    openEditor,
    closeEditor,
    saveDraft,
    applyDraft,
    discardDraft,
    backToEditing,
  };
}

export type SecretsPanelController = ReturnType<
  typeof useSecretsPanelController
>;
