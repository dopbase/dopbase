import { onUnmounted, ref, watch } from "vue";
import type { Ref } from "vue";
import * as tokensApi from "~/services/tokens.api";
import type { CreatedTokenResponse, RunnerToken } from "~/services/tokens.api";
import { ApiError } from "~/services/http.client";

/**
 * Runner tokens panel controller: metadata listing, creation with the
 * single-use plaintext response, and revocation.
 *
 * The plaintext token is held in component memory only, until the creation
 * dialog is explicitly acknowledged — after that it is unrecoverable.
 */
export function useTokensPanelController(environmentId: Ref<string>) {
  const tokens = ref<RunnerToken[] | null>(null);
  const loading = ref(false);
  const loadError = ref<string | null>(null);
  const actionError = ref<string | null>(null);
  const creating = ref(false);
  let loadRequest: AbortController | null = null;

  /** Shown exactly once inside the creation dialog. */
  const created = ref<CreatedTokenResponse | null>(null);

  async function load(target = environmentId.value): Promise<void> {
    loadRequest?.abort();
    const request = new AbortController();
    loadRequest = request;
    loading.value = true;
    loadError.value = null;
    try {
      const result = await tokensApi.listTokens(target, request.signal);
      if (!request.signal.aborted && environmentId.value === target) {
        tokens.value = result;
      }
    } catch {
      if (request.signal.aborted || environmentId.value !== target) return;
      loadError.value = "Could not load runner tokens.";
      tokens.value = null;
    } finally {
      if (loadRequest === request) loading.value = false;
    }
  }

  watch(environmentId, load, { immediate: true });
  onUnmounted(() => {
    loadRequest?.abort();
    // Closing the panel discards the plaintext permanently.
    created.value = null;
  });

  async function create(name: string): Promise<void> {
    const target = environmentId.value;
    creating.value = true;
    actionError.value = null;
    try {
      const result = await tokensApi.createToken(target, {
        name,
        role: "runner",
      });
      if (environmentId.value !== target) return;
      created.value = result;
      await load(target);
    } catch (cause) {
      if (environmentId.value !== target) return;
      if (cause instanceof ApiError && cause.status === 409) {
        actionError.value = "A token with this name already exists.";
      } else {
        actionError.value = "Could not create the token.";
      }
      throw cause;
    } finally {
      if (environmentId.value === target) creating.value = false;
    }
  }

  function acknowledgeCreated(): void {
    created.value = null;
  }

  async function revoke(token: RunnerToken): Promise<void> {
    const target = environmentId.value;
    actionError.value = null;
    try {
      await tokensApi.revokeToken(token.id);
      if (environmentId.value === target) await load(target);
    } catch {
      if (environmentId.value !== target) return;
      actionError.value = "Could not revoke the token.";
      throw new Error("revoke-failed");
    }
  }

  return {
    tokens,
    loading,
    loadError,
    actionError,
    creating,
    created,
    create,
    acknowledgeCreated,
    revoke,
  };
}

export type TokensPanelController = ReturnType<typeof useTokensPanelController>;
