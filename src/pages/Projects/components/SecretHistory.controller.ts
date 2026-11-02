import { computed, onUnmounted, ref, watch } from "vue";
import type { Ref } from "vue";
import { useInfiniteQuery } from "@pinia/colada";
import * as secretsApi from "~/services/secrets.api";
import type {
  RollbackSecretResponse,
  SecretVersion,
} from "~/services/secrets.api";
import { ApiError } from "~/services/http.client";
import { isAbortError } from "~/services/api-errors";
import { useReauthentication } from "~/composable";
import { useAuthStore } from "~/stores/auth.store";
import { loadFailure, onlyWhen } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { secretVersionsQuery } from "~/queries/secrets";
import { formatCountdown } from "~/utils/format";
import { REVEAL_SECONDS } from "./SecretsPanel.controller";

export const HISTORY_PAGE_SIZE = 25;

/** Human label for what produced a version. */
export function changeLabel(version: SecretVersion): string {
  switch (version.changeType) {
    case "created":
      return "Created";
    case "updated":
      return "Updated";
    case "imported":
      return "Imported";
    case "rolled_back":
      return version.sourceVersion === null
        ? "Rolled back"
        : `Rolled back from v${version.sourceVersion}`;
    case "legacy":
      return "Before history";
  }
}

/**
 * Version history of one secret: paged metadata, a reauth-gated reveal of
 * one earlier value with the same 30-second auto-hide as the table, and
 * rollback.
 *
 * A revealed value lives only in this controller, never in the query cache,
 * and is dropped when the key or environment changes, when the dialog
 * closes, and on unmount. The cached history holds metadata only.
 */
export function useSecretHistoryController(
  environmentId: Ref<string>,
  secretKey: Ref<string | null>,
) {
  const { runWithReauth } = useReauthentication();
  const auth = useAuthStore();
  const invalidate = useInvalidate();
  /** Reveals and rollbacks for the key on screen; reset when it changes. */
  let scope = new AbortController();

  // Keyed by environment and secret, so another one starts empty and a page
  // still loading for the previous one never lands here.
  const history = useInfiniteQuery(() =>
    onlyWhen(
      secretKey.value !== null,
      secretVersionsQuery({
        environmentId: environmentId.value,
        key: secretKey.value ?? "",
        limit: HISTORY_PAGE_SIZE,
      }),
    ),
  );
  const loadingMore = ref(false);
  const loadError = loadFailure(
    history,
    (cause) => describeError(cause, "Could not load the history."),
    { withData: true },
  );
  const pages = computed(() =>
    loadError.value ? [] : (history.data.value?.pages ?? []),
  );
  const versions = computed<SecretVersion[] | null>(() =>
    history.data.value && !loadError.value
      ? pages.value.flatMap((page) => page?.items ?? [])
      : null,
  );
  const retainedVersions = computed(
    () => pages.value[0]?.retainedVersions ?? null,
  );
  const nextBefore = computed(() => pages.value.at(-1)?.nextBefore ?? null);
  const loading = computed(() => history.isLoading.value && !loadingMore.value);
  const actionError = ref<string | null>(null);
  const notice = ref<string | null>(null);

  const revealedVersion = ref<number | null>(null);
  const revealedValue = ref<string | null>(null);
  const revealSecondsLeft = ref(0);
  let hideTimer: ReturnType<typeof setInterval> | null = null;

  const rollbackTarget = ref<SecretVersion | null>(null);
  const rollingBack = ref(false);
  const rollbackError = ref<string | null>(null);

  const current = computed(
    () => versions.value?.find((version) => version.current) ?? null,
  );
  const revealCountdown = computed(() =>
    formatCountdown(revealSecondsLeft.value),
  );

  function hideRevealed(): void {
    revealedVersion.value = null;
    revealedValue.value = null;
    revealSecondsLeft.value = 0;
    if (hideTimer) {
      clearInterval(hideTimer);
      hideTimer = null;
    }
  }

  function reset(): void {
    scope.abort();
    scope = new AbortController();
    hideRevealed();
    loadingMore.value = false;
    actionError.value = null;
    notice.value = null;
    rollbackTarget.value = null;
    rollbackError.value = null;
  }

  /** Tries again after a failed load. */
  async function load(): Promise<void> {
    if (!secretKey.value) return;
    await history.refetch();
  }

  async function loadMore(): Promise<void> {
    if (!secretKey.value || nextBefore.value === null || loadingMore.value)
      return;
    const signal = scope.signal;
    loadingMore.value = true;
    try {
      await history.loadNextPage({ throwOnError: true });
    } catch (cause) {
      if (signal.aborted || isAbortError(cause) || auth.sessionExpired) return;
      actionError.value = describeError(
        cause,
        "Could not load older versions.",
      );
    } finally {
      if (!signal.aborted) loadingMore.value = false;
    }
  }

  // Another key or environment: drop what was revealed or chosen for the last.
  watch([environmentId, secretKey], reset, { immediate: true });
  onUnmounted(() => {
    scope.abort();
    hideRevealed();
  });

  function describeError(cause: unknown, fallback: string): string {
    if (!(cause instanceof ApiError)) return fallback;
    if (cause.status === 0) return "Cannot reach the Dopbase server.";
    switch (cause.firstCode) {
      case "SECRET_VERSION_CONFLICT":
        return "This secret changed while you were looking. The history is up to date now, so check it and try again.";
      case "SECRET_VERSION_EXPIRED":
        return "That version is past the retention limit, so its value is no longer stored.";
      case "SECRET_VERSION_IS_CURRENT":
        return "That version is already the current value.";
      case "SECRET_NOT_FOUND":
        return "This secret no longer exists.";
      default:
        return fallback;
    }
  }

  async function reveal(version: number): Promise<void> {
    const key = secretKey.value;
    if (!key) return;
    const target = environmentId.value;
    const signal = scope.signal;
    actionError.value = null;
    try {
      await runWithReauth(async () => {
        const revealed = await secretsApi.revealSecretVersion(
          target,
          key,
          version,
        );
        if (signal.aborted) return;
        hideRevealed();
        revealedVersion.value = version;
        revealedValue.value = revealed.value;
        revealSecondsLeft.value = REVEAL_SECONDS;
        hideTimer = setInterval(() => {
          revealSecondsLeft.value -= 1;
          if (revealSecondsLeft.value <= 0) hideRevealed();
        }, 1000);
      }, signal);
    } catch (cause) {
      if (signal.aborted || isAbortError(cause) || auth.sessionExpired) return;
      actionError.value = describeError(
        cause,
        "Could not reveal that version.",
      );
    }
  }

  function askRollback(version: SecretVersion): void {
    rollbackError.value = null;
    notice.value = null;
    rollbackTarget.value = version;
  }

  function cancelRollback(): void {
    if (rollingBack.value) return;
    rollbackTarget.value = null;
    rollbackError.value = null;
  }

  /** Rolls back to the chosen version. Resolves with the result, or null. */
  async function confirmRollback(): Promise<RollbackSecretResponse | null> {
    const key = secretKey.value;
    const chosen = rollbackTarget.value;
    const expected = current.value;
    if (!key || !chosen || !expected) return null;
    const target = environmentId.value;
    const signal = scope.signal;
    rollingBack.value = true;
    rollbackError.value = null;
    let result: RollbackSecretResponse | undefined;
    try {
      await runWithReauth(async () => {
        result = await secretsApi.rollbackSecret(
          target,
          key,
          chosen.version,
          expected.version,
        );
      }, signal);
      if (signal.aborted || result === undefined) return null;
      rollbackTarget.value = null;
      hideRevealed();
      notice.value = `Rolled back to v${result.sourceVersion}. Now at v${result.version}.`;
      await invalidate.secretsChanged(target);
      return result;
    } catch (cause) {
      if (signal.aborted || isAbortError(cause) || auth.sessionExpired)
        return null;
      const conflict =
        cause instanceof ApiError &&
        cause.firstCode === "SECRET_VERSION_CONFLICT";
      rollbackError.value = describeError(cause, "Could not roll back.");
      // It changed elsewhere: show the latest before another try.
      if (conflict) await invalidate.secretsChanged(target);
      return null;
    } finally {
      rollingBack.value = false;
    }
  }

  return {
    versions,
    current,
    retainedVersions,
    nextBefore,
    loading,
    loadingMore,
    loadError,
    actionError,
    notice,
    revealedVersion,
    revealedValue,
    revealCountdown,
    rollbackTarget,
    rollingBack,
    rollbackError,
    load,
    loadMore,
    reveal,
    hideRevealed,
    askRollback,
    cancelRollback,
    confirmRollback,
  };
}
