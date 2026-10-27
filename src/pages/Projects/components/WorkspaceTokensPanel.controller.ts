import { onUnmounted, ref, watch } from "vue";
import type { Ref } from "vue";
import * as tokensApi from "~/services/tokens.api";
import type {
  CreatedWorkspaceTokenResponse,
  WorkspaceToken,
} from "~/services/tokens.api";
import { ApiError } from "~/services/http.client";

export function useWorkspaceTokensPanelController(workspaceId: Ref<string>) {
  const tokens = ref<WorkspaceToken[] | null>(null);
  const loading = ref(false);
  const loadError = ref<string | null>(null);
  const actionError = ref<string | null>(null);
  const creating = ref(false);
  let loadRequest: AbortController | null = null;
  const created = ref<CreatedWorkspaceTokenResponse | null>(null);

  async function load(target = workspaceId.value): Promise<void> {
    loadRequest?.abort();
    const request = new AbortController();
    loadRequest = request;
    loading.value = true;
    loadError.value = null;
    try {
      const result = await tokensApi.listWorkspaceTokens(target, request.signal);
      if (!request.signal.aborted && workspaceId.value === target) {
        tokens.value = result;
      }
    } catch {
      if (request.signal.aborted || workspaceId.value !== target) return;
      loadError.value = "Could not load workspace tokens.";
      tokens.value = null;
    } finally {
      if (loadRequest === request) loading.value = false;
    }
  }

  watch(workspaceId, load, { immediate: true });
  onUnmounted(() => {
    loadRequest?.abort();
    created.value = null;
  });

  async function create(name: string, expiresIn = "never"): Promise<void> {
    const target = workspaceId.value;
    creating.value = true;
    actionError.value = null;
    try {
      const result = await tokensApi.createWorkspaceToken(target, {
        name,
        role: "workspace",
        expiresIn,
      });
      if (workspaceId.value !== target) return;
      created.value = result;
      await load(target);
    } catch (cause) {
      if (workspaceId.value !== target) return;
      if (cause instanceof ApiError && cause.status === 409) {
        actionError.value = "A token with this name already exists.";
      } else {
        actionError.value = "Could not create the token.";
      }
      throw cause;
    } finally {
      if (workspaceId.value === target) creating.value = false;
    }
  }

  function acknowledgeCreated(): void {
    created.value = null;
  }

  async function revoke(token: WorkspaceToken): Promise<void> {
    const target = workspaceId.value;
    actionError.value = null;
    try {
      await tokensApi.revokeToken(token.id);
      if (workspaceId.value === target) await load(target);
    } catch {
      if (workspaceId.value !== target) return;
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

export type WorkspaceTokensPanelController = ReturnType<
  typeof useWorkspaceTokensPanelController
>;
