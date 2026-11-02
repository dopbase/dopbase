import { computed, onUnmounted, ref, watch } from "vue";
import type { Ref } from "vue";
import { useQuery } from "@pinia/colada";
import * as tokensApi from "~/services/tokens.api";
import type { CreatedTokenResponse, RunnerToken } from "~/services/tokens.api";
import { ApiError } from "~/services/http.client";
import { loadFailure, onlyWhen } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { runnerTokensQuery } from "~/queries/tokens";
import { formatDate, formatRelativeTime } from "~/utils/format";
import { copyText } from "~/utils/clipboard";

const DAY = 86_400_000;

export interface TokenRow {
  token: RunnerToken;
  lastUsed: string;
  expires: string;
  /** `warn` within a week of expiry, `crit` once revoked or expired. */
  expiresTone: "default" | "warn" | "crit";
  active: boolean;
}

/** How a token's expiry reads in the table. */
export function describeExpiry(
  token: RunnerToken,
  now: number,
): Pick<TokenRow, "expires" | "expiresTone" | "active"> {
  if (token.revokedAt)
    return { expires: "Revoked", expiresTone: "crit", active: false };
  if (!token.expiresAt)
    return { expires: "Never", expiresTone: "default", active: true };
  const left = new Date(token.expiresAt).getTime() - now;
  if (left <= 0)
    return { expires: "Expired", expiresTone: "crit", active: false };
  if (left < 7 * DAY) {
    const days = Math.ceil(left / DAY);
    return {
      expires: days === 1 ? "Expires in 1 day" : `Expires in ${days} days`,
      expiresTone: "warn",
      active: true,
    };
  }
  return {
    expires: formatDate(token.expiresAt),
    expiresTone: "default",
    active: true,
  };
}

/**
 * Runner tokens panel controller: metadata listing, creation with the
 * single-use plaintext response, and revocation.
 *
 * The plaintext token is held in component memory only, until the creation
 * dialog is explicitly acknowledged — after that it is unrecoverable.
 *
 * The table rows carry ready-to-show text: last use, and expiry with a warn
 * tone inside the last week. A minute timer keeps those current.
 */
export function useTokensPanelController(environmentId: Ref<string>) {
  const invalidate = useInvalidate();
  // Keyed by environment, so another one starts empty and a late answer for
  // the previous one never lands here. The Run dialog reads the same entry.
  const tokenList = useQuery(() =>
    onlyWhen(
      environmentId.value !== "",
      runnerTokensQuery(environmentId.value),
    ),
  );
  // A failed reload clears the rows, so nothing acts on tokens that may be gone.
  const loadError = loadFailure(tokenList, "Could not load runner tokens.", {
    withData: true,
  });
  const tokens = computed<RunnerToken[] | null>(() =>
    loadError.value ? null : (tokenList.data.value ?? null),
  );
  // Only a first load shows the skeleton; a background refresh keeps the rows.
  const loading = computed(
    () => tokenList.isLoading.value && tokenList.data.value === undefined,
  );
  const actionError = ref<string | null>(null);
  const creating = ref(false);

  /** Shown exactly once inside the creation dialog. */
  const created = ref<CreatedTokenResponse | null>(null);

  const filter = ref("");
  const now = ref(Date.now());
  const clock = setInterval(() => (now.value = Date.now()), 60_000);

  const rows = computed<TokenRow[] | null>(() => {
    if (!tokens.value) return null;
    const query = filter.value.trim().toLowerCase();
    return tokens.value
      .filter((token) => token.name.toLowerCase().includes(query))
      .map((token) => ({
        token,
        lastUsed: token.lastUsedAt
          ? formatRelativeTime(token.lastUsedAt)
          : "Never",
        ...describeExpiry(token, now.value),
      }));
  });

  /** The id of the token whose name was just copied. */
  const copiedId = ref<string | null>(null);
  async function copyName(token: RunnerToken): Promise<void> {
    if (!(await copyText(token.name))) return;
    copiedId.value = token.id;
    setTimeout(() => {
      if (copiedId.value === token.id) copiedId.value = null;
    }, 2000);
  }

  // A new environment starts clean: no error or busy state from the last one.
  watch(environmentId, () => {
    actionError.value = null;
    creating.value = false;
  });
  onUnmounted(() => {
    clearInterval(clock);
    // Closing the panel discards the plaintext permanently.
    created.value = null;
  });

  async function create(name: string, expiresIn = "never"): Promise<void> {
    const target = environmentId.value;
    creating.value = true;
    actionError.value = null;
    try {
      const result = await tokensApi.createToken(target, {
        name,
        role: "runner",
        expiresIn,
      });
      // Show the plaintext even if the environment changed meanwhile: the
      // token exists, and this is the only time it can be seen.
      created.value = result;
      await invalidate.runnerTokensChanged(target);
    } catch (cause) {
      if (environmentId.value !== target) return;
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
    const target = environmentId.value;
    actionError.value = null;
    try {
      await tokensApi.revokeToken(token.id);
      await invalidate.runnerTokensChanged(target);
    } catch {
      if (environmentId.value !== target) return;
      actionError.value = "Could not revoke the token.";
      throw new Error("revoke-failed");
    }
  }

  return {
    tokens,
    rows,
    filter,
    copiedId,
    copyName,
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
