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

  /** Shown exactly once inside the creation dialog. */
  const created = ref<CreatedTokenResponse | null>(null);

  async function load(): Promise<void> {
    loading.value = true;
    loadError.value = null;
    try {
      tokens.value = await tokensApi.listTokens(environmentId.value);
    } catch {
      loadError.value = "Could not load runner tokens.";
      tokens.value = null;
    } finally {
      loading.value = false;
    }
  }

  watch(environmentId, load, { immediate: true });
  onUnmounted(() => {
    // Closing the panel discards the plaintext permanently.
    created.value = null;
  });

  async function create(name: string): Promise<void> {
    creating.value = true;
    actionError.value = null;
    try {
      created.value = await tokensApi.createToken(environmentId.value, {
        name,
        role: "runner",
      });
      await load();
    } catch (cause) {
      if (cause instanceof ApiError && cause.status === 409) {
        actionError.value = "A token with this name already exists.";
      } else {
        actionError.value = "Could not create the token.";
      }
      throw cause;
    } finally {
      creating.value = false;
    }
  }

  function acknowledgeCreated(): void {
    created.value = null;
  }

  async function revoke(token: RunnerToken): Promise<void> {
    actionError.value = null;
    try {
      await tokensApi.revokeToken(token.id);
      await load();
    } catch {
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
